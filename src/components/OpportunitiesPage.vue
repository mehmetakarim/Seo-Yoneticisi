<script setup lang="ts">
/**
 * Fırsatlar — "önce hangi ürüne bakmalıyım?"
 *
 * 262 ürün arasında elle seçim yapmak yerine, Google Search Console verisiyle emeğin en çok
 * getiri sağlayacağı ürünleri sıralar. Sıralama ölçütü soyut bir puan değil **kaçırılan
 * tıklama**: "bu sayfa, konumunun getirmesi gereken tıklamanın kaçını alamıyor".
 */
import { computed, onMounted, ref } from "vue";
import { useStore } from "../store";
import { workState, type Opportunity, type OpportunityReason, type WorkState } from "../types";
import Icon from "./Icon.vue";
import ModalShell from "./ModalShell.vue";
import AnalysisSection from "./AnalysisSection.vue";

const store = useStore();
const report = computed(() => store.opportunity);

// --- Filtreler ---
// Tamamen istemcide: 60 satır için sunucuya gitmek gereksiz gecikme olurdu.
const workFilter = ref<WorkState | "all">("all");
const reasonFilter = ref<OpportunityReason | "all">("all");
/** Kategori veya marka kesiti — "cat:Notebook" / "brand:Lenovo" biçiminde. */
const sliceFilter = ref<string>("");

/** EOL listesi uzun olabilir (bu sitede ~970 sayfa) — önce en değerlileri göster. */
const eolLimit = ref(25);
/** Sorgu listesi de uzun olabilir — önce en değerlileri. */
const sdLimit = ref(20);
const decayLimit = ref(15);

const all = computed<Opportunity[]>(() => report.value?.opportunities ?? []);
// Not: eski önbellekte olmayan alanlara karşı şablonda da `?.` kullanılıyor —
// asıl koruma backend tarafında (serde default), bu ikinci savunma hattı.

const filtered = computed(() =>
  all.value.filter((o) => {
    if (workFilter.value !== "all" && workState(o) !== workFilter.value) return false;
    if (reasonFilter.value !== "all" && o.reason !== reasonFilter.value) return false;
    if (sliceFilter.value) {
      const [kind, val] = sliceFilter.value.split(/:(.*)/s);
      if (kind === "cat" && o.category !== val) return false;
      if (kind === "brand" && o.brand !== val) return false;
    }
    return true;
  }),
);

const anyFilter = computed(
  () => workFilter.value !== "all" || reasonFilter.value !== "all" || !!sliceFilter.value,
);
function clearFilters() {
  workFilter.value = "all";
  reasonFilter.value = "all";
  sliceFilter.value = "";
}

/** Sayaçlar tüm listeden hesaplanır — filtre uygulanınca sayılar değişip kafa karıştırmasın. */
const workCounts = computed(() => {
  const c = { untouched: 0, partial: 0, worked: 0 };
  for (const o of all.value) c[workState(o)]++;
  return c;
});
const reasonCounts = computed(() => {
  const c: Record<string, number> = { second_page: 0, no_clicks: 0, low_ctr: 0 };
  for (const o of all.value) c[o.reason]++;
  return c;
});

/** Kaçırılan tıklamaya göre en büyük kesitler — nereye toplu odaklanılacağını gösterir. */
function topSlices(key: "category" | "brand", n: number) {
  const m = new Map<string, { n: number; missed: number }>();
  for (const o of all.value) {
    const k = o[key] || "(belirtilmemiş)";
    const cur = m.get(k) ?? { n: 0, missed: 0 };
    cur.n++;
    cur.missed += o.missed_clicks;
    m.set(k, cur);
  }
  return [...m.entries()]
    .sort((a, b) => b[1].missed - a[1].missed)
    .slice(0, n)
    .map(([name, v]) => ({ name, ...v }));
}
const topCats = computed(() => topSlices("category", 5));
const topBrands = computed(() => topSlices("brand", 5));

const WORK: Record<WorkState, { label: string; badge: string; tip: string }> = {
  untouched: {
    label: "Dokunulmamış",
    badge: "eksik",
    tip: "Bu ürün için henüz meta veya açıklama üretilmemiş — en hızlı kazanç burada.",
  },
  partial: {
    label: "Kısmen",
    badge: "hatali",
    tip: "Meta veya açıklamadan yalnızca biri üretilmiş.",
  },
  worked: {
    label: "Çalışıldı",
    badge: "tamamlandi",
    tip: "Meta ve açıklama üretilmiş ama sayfa hâlâ fırsat listesinde — yani üretim sonuç vermemiş. Hedef kelimeyi gözden geçirmek gerekebilir.",
  },
};

onMounted(() => {
  // Önbellekten yükle — sayfa her açıldığında GSC'ye gitmeye gerek yok.
  if (!store.opportunity) void store.loadOpportunityCache();
});

/** Sebep → mevcut rozet token'ı. Yeni renk uydurulmuyor; anlam eşlemesi bilinçli:
 *  kırmızı = hiç tıklanmıyor (en kötü), amber = meta çalışması, mavi = konum işi. */
const REASON: Record<OpportunityReason, { label: string; badge: string; tip: string }> = {
  no_clicks: {
    label: "Tıklama yok",
    badge: "eksik",
    tip: "Gösterim alıyor ama hiç tıklanmıyor — başlık ve açıklama ilgi çekmiyor.",
  },
  low_ctr: {
    label: "Düşük CTR",
    badge: "hatali",
    tip: "İlk sayfada ama tıklama oranı konumunun beklenenin çok altında — meta çalışması gerekiyor.",
  },
  second_page: {
    label: "İkinci sayfa",
    badge: "bekliyor",
    tip: "11–20. sırada. İlk sayfaya çıkmak için küçük bir iyileştirme yeterli olabilir.",
  },
};

const totalMissed = computed(() => Math.round(filtered.value.reduce((s, o) => s + o.missed_clicks, 0)));

/** Tam URL'den son yol parçası — canonical hedefi olarak gönderilir. */
const slugOf = (u: string) => u.trim().replace(/\/$/, "").split("/").pop() ?? "";

/** Bu EOL sayfası için önerilen hedef slug; öneri yoksa boş (kullanıcı kendisi seçer). */
const targetOf = (eolUrl: string) => {
  const s = store.successors[eolUrl];
  return s?.sku && s.url ? slugOf(s.url) : "";
};

const fmtDate = (s: string) => s.replace("T", " ").slice(0, 16);
const pct = (n: number) => (n * 100).toFixed(1).replace(".", ",");
</script>

<template>
  <div class="page om-scroll">
    <!-- Üst şerit: özet + yenileme -->
    <div class="top">
      <div class="sum">
        <template v-if="report">
          <b>{{ all.length }}</b> fırsat ·
          yaklaşık <b>{{ totalMissed }}</b> tıklama kaçırılıyor
          <span class="dim">
            · son {{ report.days }} gün · {{ report.matched }}/{{ report.total_products }} ürün
            Google'da bulundu · {{ fmtDate(report.analyzed_at) }}
          </span>
        </template>
        <span v-else class="dim">Henüz analiz çalıştırılmadı.</span>
      </div>
      <button class="run" :disabled="store.opportunityBusy" @click="store.runOpportunityAnalysis()">
        <Icon
          :name="store.opportunityBusy ? 'loader' : 'refresh'"
          :size="15"
          :class="{ spin: store.opportunityBusy }"
        />
        {{ store.opportunityBusy ? "Analiz ediliyor…" : report ? "Yenile" : "Analizi çalıştır" }}
      </button>
    </div>

    <!-- Hata kalıcı gösterilir: toast kaybolur, kullanıcı sebebi göremez -->
    <div v-if="store.opportunityError" class="err">
      <Icon name="alert" :size="14" />
      <span>{{ store.opportunityError }}</span>
    </div>

    <!-- Hiç analiz yoksa: ne olduğunu anlat -->
    <div v-if="!report && !store.opportunityBusy" class="empty">
      <Icon name="search" :size="30" :stroke-width="1.5" />
      <p class="e-title">Emeğinizi nereye harcayacağınızı Google söylesin</p>
      <p class="e-sub">
        Search Console verisiyle hangi ürünlerin gösterim alıp tıklanmadığını, hangilerinin
        ikinci sayfada takıldığını sıralar. Tek bir sorgu ile tüm katalog taranır.
      </p>
    </div>

    <!-- Kesitler: kayıp nerede toplanıyor? Tek tek yerine grup halinde çalışma imkânı. -->
    <div v-if="all.length" class="slices">
      <div class="slice-group">
        <span class="slice-label">Kategori</span>
        <button
          v-for="c in topCats"
          :key="'c' + c.name"
          class="slice"
          :class="{ on: sliceFilter === 'cat:' + c.name }"
          @click="sliceFilter = sliceFilter === 'cat:' + c.name ? '' : 'cat:' + c.name"
        >
          {{ c.name }}<b>{{ Math.round(c.missed) }}</b>
        </button>
      </div>
      <div class="slice-group">
        <span class="slice-label">Marka</span>
        <button
          v-for="b in topBrands"
          :key="'b' + b.name"
          class="slice"
          :class="{ on: sliceFilter === 'brand:' + b.name }"
          @click="sliceFilter = sliceFilter === 'brand:' + b.name ? '' : 'brand:' + b.name"
        >
          {{ b.name }}<b>{{ Math.round(b.missed) }}</b>
        </button>
      </div>
    </div>

    <!-- Filtreler: iş durumu ve sebep. ProductList'teki çip deseninin aynısı. -->
    <div v-if="all.length" class="filters">
      <button class="filter" :class="{ on: workFilter === 'all' }" @click="workFilter = 'all'">
        <span>Hepsi</span><span class="count">{{ all.length }}</span>
      </button>
      <button
        v-for="w in (['untouched', 'partial', 'worked'] as const)"
        :key="w"
        class="filter"
        :class="{ on: workFilter === w }"
        :data-tip="WORK[w].tip"
        @click="workFilter = w"
      >
        <span>{{ WORK[w].label }}</span><span class="count">{{ workCounts[w] }}</span>
      </button>

      <span class="fsep"></span>

      <button
        v-for="r in (['low_ctr', 'no_clicks', 'second_page'] as const)"
        :key="r"
        class="filter"
        :class="{ on: reasonFilter === r }"
        @click="reasonFilter = reasonFilter === r ? 'all' : r"
      >
        <span>{{ REASON[r].label }}</span><span class="count">{{ reasonCounts[r] }}</span>
      </button>

      <button v-if="anyFilter" class="filter clear" @click="clearFilters()">
        <Icon name="x" :size="12" :stroke-width="2.4" /> Filtreyi temizle
      </button>
    </div>

    <!-- Fırsat tablosu -->
    <div v-if="filtered.length" class="card">
      <table class="tbl">
        <thead>
          <tr>
            <th class="c-name">Ürün</th>
            <th class="c-work">Durum</th>
            <th class="c-num">Gösterim</th>
            <th class="c-num">Tıklama</th>
            <th class="c-num">CTR</th>
            <th class="c-num">Konum</th>
            <th class="c-num">Kaçırılan</th>
            <th class="c-reason">Sebep</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="o in filtered"
            :key="o.sku"
            class="row"
            :title="`${o.name} — ürüne git`"
            @click="store.openProduct(o.sku)"
          >
            <td class="c-name">
              <div class="nm">{{ o.name }}</div>
              <div class="sku">{{ o.sku }}</div>
            </td>
            <td class="c-work">
              <span
                class="status tip-below"
                :style="{
                  background: `var(--badge-${WORK[workState(o)].badge}-bg)`,
                  color: `var(--badge-${WORK[workState(o)].badge}-c)`,
                }"
                :data-tip="WORK[workState(o)].tip"
              >
                {{ WORK[workState(o)].label }}
              </span>
            </td>
            <td class="c-num">{{ Math.round(o.impressions) }}</td>
            <td class="c-num">{{ Math.round(o.clicks) }}</td>
            <td class="c-num">%{{ pct(o.ctr) }}</td>
            <td class="c-num">{{ o.position.toFixed(1) }}</td>
            <td class="c-num miss">{{ Math.round(o.missed_clicks) }}</td>
            <td class="c-reason">
              <span
                class="status tip-below"
                :style="{
                  background: `var(--badge-${REASON[o.reason].badge}-bg)`,
                  color: `var(--badge-${REASON[o.reason].badge}-c)`,
                }"
                :data-tip="REASON[o.reason].tip"
              >
                {{ REASON[o.reason].label }}
              </span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Filtre hiçbir satır bırakmadı: sessiz boş tablo bırakma -->
    <div v-else-if="report && all.length" class="clean">
      <Icon name="search" :size="16" :stroke-width="2.2" style="color: var(--c-soft)" />
      Bu filtrede fırsat yok.
      <a class="link" @click="clearFilters()">Filtreyi temizle</a>
    </div>

    <!-- Analiz yapıldı ama hiç fırsat yok -->
    <div v-else-if="report" class="clean">
      <Icon name="check" :size="16" :stroke-width="2.4" style="color: var(--green)" />
      Fırsat bulunamadı — Google'da bulunan ürünlerin tamamı konumuna göre beklenen performansta.
    </div>

    <!-- Gerileme: düşüşteki sayfa, hiç yükselmemiş olandan daha acil -->
    <AnalysisSection
      v-if="report?.decay?.length"
      title="Düşüşte olanlar"
      :count="report.decay.length"
      :summary="`${Math.round(report.decay.reduce((a, d) => a + d.clicks_lost, 0))} tıklama kaybı`"
    >
      <template #note>
        Önceki {{ report.days }} güne göre gerileyen sayfalar. Burada bir şey bozulmuş —
        müdahale edilmezse kayıp büyür. Konum değişmeden tıklama düştüyse sorun sıralamada
        değil, arama sonucundaki görünümde olabilir.
      </template>
      <table class="tbl">
        <thead>
          <tr>
            <th class="c-name">Ürün</th>
            <th class="c-num">Tıklama</th>
            <th class="c-num">Konum</th>
            <th class="c-num">Kayıp</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="d in report.decay.slice(0, decayLimit)"
            :key="d.sku"
            class="row"
            @click="store.openProduct(d.sku)"
          >
            <td class="c-name"><div class="nm">{{ d.name }}</div><div class="sku">{{ d.sku }}</div></td>
            <td class="c-num">
              <span class="was">{{ Math.round(d.clicks_before) }}</span>
              <span class="arrow">→</span>{{ Math.round(d.clicks_now) }}
            </td>
            <td class="c-num">
              <span class="was">{{ d.position_before.toFixed(1) }}</span>
              <span class="arrow">→</span>{{ d.position_now.toFixed(1) }}
            </td>
            <td class="c-num miss">−{{ Math.round(d.clicks_lost) }}</td>
          </tr>
        </tbody>
      </table>
      <div v-if="report.decay.length > decayLimit" class="eol-more">
        <a class="link" @click="decayLimit += 30">
          Sonraki 30'u göster ({{ report.decay.length - decayLimit }} kaldı)
        </a>
      </div>
    </AnalysisSection>

    <!-- Striking distance: hangi SORGUDA kaçıncı sıradasınız — "ne yazmalıyım" katmanı -->
    <AnalysisSection
      v-if="report?.striking?.length"
      title="Yükselmeye yakın sorgular"
      :count="report.striking.length"
    >
      <template #note>
        Bu aramalarda 4–20. sıradasınız — küçük bir iyileştirme ilk sıralara taşıyabilir.
        Sorgu, o ürün için <b>hedef kelime adayıdır</b>: satıra tıklayıp ürüne gidin.
      </template>
      <table class="tbl">
        <thead>
          <tr>
            <th class="c-name">Sorgu / Ürün</th>
            <th class="c-num">Gösterim</th>
            <th class="c-num">Tıklama</th>
            <th class="c-num">Konum</th>
            <th class="c-num">Kaçırılan</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="q in report.striking.slice(0, sdLimit)"
            :key="q.sku + q.query"
            class="row"
            @click="store.openProduct(q.sku)"
          >
            <td class="c-name">
              <div class="nm">{{ q.query }}</div>
              <div class="sku">{{ q.name }}</div>
            </td>
            <td class="c-num">{{ Math.round(q.impressions) }}</td>
            <td class="c-num">{{ Math.round(q.clicks) }}</td>
            <td class="c-num">{{ q.position.toFixed(1) }}</td>
            <td class="c-num miss">{{ Math.round(q.missed_clicks) }}</td>
          </tr>
        </tbody>
      </table>
      <div v-if="report.striking.length > sdLimit" class="eol-more">
        <a class="link" @click="sdLimit += 40">
          Sonraki 40'ı göster ({{ report.striking.length - sdLimit }} kaldı)
        </a>
      </div>
    </AnalysisSection>

    <!-- Kanibalizasyon: kendi sayfalarımız birbiriyle yarışıyor -->
    <AnalysisSection
      v-if="report?.cannibalization?.length"
      title="Birbiriyle yarışan sayfalar"
      :count="report.cannibalization.length"
    >
      <template #note>
        Aynı aramada birden çok ürün sayfanız görünüyor ve hiçbiri öne çıkamıyor.
        <b>Otomatik birleştirme önerilmez</b> — önce hangi sayfanın o aramayı sahiplenmesi
        gerektiğine karar verin, diğerlerini farklılaştırın.
      </template>
      <div class="cann-list">
        <div v-for="c in report.cannibalization" :key="c.query" class="cann">
          <div class="cann-q">
            <span class="q">{{ c.query }}</span>
            <span class="cann-m">
              {{ Math.round(c.impressions) }} gösterim · {{ Math.round(c.clicks) }} tıklama
            </span>
          </div>
          <div
            v-for="pg in c.pages"
            :key="pg.sku"
            class="cann-p"
            @click="store.openProduct(pg.sku)"
          >
            <span class="cann-pos">{{ pg.position.toFixed(1) }}.</span>
            <span class="nm">{{ pg.name }}</span>
            <span class="cann-c">{{ Math.round(pg.clicks) }} tık</span>
          </div>
        </div>
      </div>
    </AnalysisSection>

    <!-- Satışta olmayan ama trafik alan sayfalar. Ölçüm: ürün trafiğinin %69'u burada. -->
    <AnalysisSection
      v-if="report?.eol?.length"
      title="Satışta olmayan ama trafik alan sayfalar"
      :count="report.eol.length"
      :summary="`${Math.round(report.eol_clicks)} tıklama`"
    >
      <template #note>
        Bu adresler kataloğunuzda yok ama Google'da hâlâ sıralanıyor — ziyaretçi geliyor,
        ürünü satın alamıyor. Çözüm genelde güncel nesle <b>301 yönlendirme</b>; yönlendirmeyi
        IdeaSoft panelinden tanımlamanız gerekir, uygulama bunu yapamaz.
        Bazı sayfaları bilinçli tutuyor olabilirsiniz — liste öneridir, karar sizin.
      </template>
      <template #action>
        <button
          class="succ-btn cat-sync"
          :disabled="store.catalogBusy"
          data-tip="Tüm kataloğu bir kez çeker (~7 dk). Canonical için GEREKMEZ — uygulama tek satırı anında bulur. Bu yalnızca listenin tamamını hızlandırır."
          @click="store.syncCatalog()"
        >
          <Icon
            :name="store.catalogBusy ? 'loader' : 'refresh'"
            :size="11"
            :class="{ spin: store.catalogBusy }"
          />
          {{ store.catalogBusy ? "Katalog alınıyor…" : "Katalogla eşleştir" }}
        </button>
      </template>
      <table class="tbl">
        <thead>
          <tr>
            <th class="c-name">Sayfa</th>
            <th class="c-num">Tıklama</th>
            <th class="c-num">Gösterim</th>
            <th class="c-num">Konum</th>
          </tr>
        </thead>
        <tbody>
          <template v-for="e in report.eol.slice(0, eolLimit)" :key="e.url">
            <tr>
              <td class="c-name">
                <div class="nm">{{ e.slug }}</div>
                <!-- Halef önerisi: istek üzerine, tek sayfa için. Toplu çalıştırmak
                     günlük model kotasını anında tüketirdi. -->
                <div class="succ">
                  <button
                    v-if="!store.successors[e.url]"
                    class="succ-btn"
                    :disabled="!!store.successorBusy"
                    @click="store.suggestSuccessor(e.url)"
                  >
                    <Icon
                      :name="store.successorBusy === e.url ? 'loader' : 'sparkles'"
                      :size="11"
                      :class="{ spin: store.successorBusy === e.url }"
                    />
                    {{ store.successorBusy === e.url ? "Bakılıyor…" : "Halef öner" }}
                  </button>
                  <template v-else>
                    <span v-if="store.successors[e.url].sku" class="succ-ok">
                      <Icon name="check" :size="11" :stroke-width="2.6" />
                      {{ store.successors[e.url].name }}
                    </span>
                    <span v-else class="succ-none">Uygun halef bulunamadı</span>
                    <!-- Canonical yazma: her satır için ayrı, onaylı. Toplu işlem YOK.
                         Halef bulunmasa da buton görünür — hedefi kullanıcı seçebilir. -->
                    <button
                      class="succ-btn"
                      :disabled="store.canonicalBusy"
                      @click="store.startCanonical(e.slug, targetOf(e.url))"
                    >
                      <Icon name="upload" :size="11" />
                      {{ targetOf(e.url) ? "Canonical ayarla" : "Hedef seç ve ayarla" }}
                    </button>
                    <span class="succ-why">{{ store.successors[e.url].reason }}</span>
                  </template>
                </div>
              </td>
              <td class="c-num miss">{{ Math.round(e.clicks) }}</td>
              <td class="c-num">{{ Math.round(e.impressions) }}</td>
              <td class="c-num">{{ e.position.toFixed(1) }}</td>
            </tr>
          </template>
        </tbody>
      </table>
      <div v-if="report.eol.length > eolLimit" class="eol-more">
        <a class="link" @click="eolLimit += 50">
          Sonraki 50'yi göster ({{ report.eol.length - eolLimit }} kaldı)
        </a>
      </div>
    </AnalysisSection>

    <!-- Google'da hiç görünmeyenler: farklı bir iş, bilinçli olarak ayrı -->
    <AnalysisSection
      v-if="report?.invisible?.length"
      title="Google'da görünmeyenler"
      :count="report.invisible.length"
    >
      <template #note>
        Son {{ report.days }} günde hiç gösterim almamışlar. Bu bir meta sorunu değil —
        indeksleme veya görünürlük işi; içerik üretmek tek başına çözmez.
      </template>
      <div class="inv-list">
        <div
          v-for="p in report.invisible"
          :key="p.sku"
          class="inv-row"
          @click="store.openProduct(p.sku)"
        >
          <span class="nm">{{ p.name }}</span>
          <span class="sku">{{ p.sku }}</span>
        </div>
      </div>
    </AnalysisSection>

    <!-- Hedef seçme modali: halef önerisi boş çıktığında ya da öneri değiştirilmek
         istendiğinde. Yine TEK satır için — toplu seçim yok. -->
    <ModalShell
      :open="!!store.canonicalPicker"
      label="Canonical hedefi seç"
      title="Canonical hedefini seçin"
      :sub="store.canonicalPicker?.eolSlug"
      :closable="!store.canonicalSearching"
      @close="store.cancelCanonicalPicker()"
    >
      <div class="pick-row">
        <input
          v-model="store.canonicalQuery"
          class="pick-in"
          type="text"
          placeholder="Ürün adı yazın (en az 3 harf)"
          @keyup.enter="store.searchCanonicalTarget()"
        />
        <button
          class="succ-btn"
          :disabled="store.canonicalSearching || store.canonicalQuery.trim().length < 3"
          @click="store.searchCanonicalTarget()"
        >
          <Icon
            :name="store.canonicalSearching ? 'loader' : 'search'"
            :size="11"
            :class="{ spin: store.canonicalSearching }"
          />
          {{ store.canonicalSearching ? "Aranıyor…" : "Ara" }}
        </button>
      </div>
      <!-- Arama IdeaSoft'ta ÜRÜN ADINA göre çalışıyor (ölçüldü), slug'a göre değil. -->
      <div class="pick-hint">
        Arama mağazanızdaki ürün adlarında yapılır — feed'de olmayan ürünler de bulunur.
      </div>
      <div v-if="store.canonicalResults.length" class="pick-list">
        <div
          v-for="r in store.canonicalResults"
          :key="r.slug"
          class="pick-item"
          @click="store.pickCanonicalTarget(r.slug)"
        >
          <span class="pi-name">{{ r.name }}</span>
          <span class="pi-slug">{{ r.slug }}</span>
        </div>
      </div>
      <div
        v-else-if="!store.canonicalSearching && store.canonicalQuery.trim().length >= 3"
        class="pick-hint"
      >
        Sonuç yok. Daha az sözcükle deneyin — arama tüm sözcüklerin geçmesini ister.
      </div>

      <template #footer>
        <button class="ghost" :disabled="store.canonicalSearching" @click="store.cancelCanonicalPicker()">
          Vazgeç
        </button>
      </template>
    </ModalShell>

    <!-- Canonical onay modali. Faz 9 gönderim modalinin deseni: önce fark, sonra onay. -->
    <ModalShell
      :open="!!store.canonicalPending"
      label="Canonical onayı"
      title="Canonical ayarlanacak"
      :sub="store.canonicalPending?.product_name"
      :closable="!store.canonicalBusy"
      @close="store.cancelCanonical()"
    >
      <div class="diff">
        <div class="d-row">
          <span class="d-lab">Şu an</span>
          <span class="d-val muted">{{ store.canonicalPending?.current || "tanımlı değil" }}</span>
        </div>
        <div class="d-row hi">
          <span class="d-lab">Olacak</span>
          <span class="d-val">{{ store.canonicalPending?.proposed }}</span>
        </div>
        <!-- Hedefin ADI: slug'a bakarak yanlış ürünü onaylamak kolay. Bu satır
             yazmadan önce doğru sayfayı seçtiğinizi görmenizi sağlıyor. -->
        <div class="d-row">
          <span class="d-lab">Hedef</span>
          <span class="d-val">
            {{ store.canonicalPending?.target_name }}
            <a class="link chg" @click="store.changeCanonicalTarget()">değiştir</a>
          </span>
        </div>
      </div>
      <div class="warn">
        <Icon name="info" :size="13" />
        <span>
          <b>Bu bir yönlendirme değildir.</b> Ziyaretçi yine eski sayfaya düşer; yalnızca
          Google'a "asıl sayfa şu" sinyali gider. Gerçek 301 için IdeaSoft panelini
          kullanmanız gerekir.
          <template v-if="store.canonicalPending?.will_create">
            Bu ürünün SEO kaydı yok, oluşturulacak.
          </template>
        </span>
      </div>

      <template #footer>
        <button class="ghost" :disabled="store.canonicalBusy" @click="store.cancelCanonical()">
          Vazgeç
        </button>
        <div style="flex:1"></div>
        <button class="run" :disabled="store.canonicalBusy" @click="store.confirmCanonical()">
          <Icon
            :name="store.canonicalBusy ? 'loader' : 'check'"
            :size="14"
            :class="{ spin: store.canonicalBusy }"
          />
          {{ store.canonicalBusy ? "Yazılıyor…" : "Onaylıyorum, yaz" }}
        </button>
      </template>
    </ModalShell>
  </div>
</template>

<style scoped>
.page {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 18px 22px 28px;
}
.top {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 14px;
}
.sum {
  font-size: 12.5px;
  color: var(--c-text);
  line-height: 1.5;
}
.sum b {
  font-weight: 660;
  font-variant-numeric: tabular-nums;
}
.dim {
  color: var(--c-soft);
}
.run {
  margin-left: auto;
  flex: none;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 36px;
  padding: 0 16px;
  border: none;
  border-radius: 9px;
  background: var(--accent);
  color: #fff;
  font-size: 13px;
  font-weight: 590;
  cursor: pointer;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
}
.run:hover:not(:disabled) {
  filter: brightness(1.06);
}
.run:disabled {
  opacity: 0.8;
  cursor: default;
}
.err {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 10px 12px;
  margin-bottom: 14px;
  font-size: 12px;
  line-height: 1.5;
  color: var(--warn-text);
  background: var(--warn-bg);
  border: 1px solid var(--warn-border);
  border-radius: 9px;
}
.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 4px;
  padding: 60px 20px;
  color: var(--c-faint);
}
.e-title {
  margin: 8px 0 0;
  font-size: 14px;
  font-weight: 620;
  color: var(--c-mid);
}
.e-sub {
  margin: 0;
  max-width: 460px;
  font-size: 12.5px;
  line-height: 1.6;
  color: var(--c-soft);
}
.clean {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 16px 18px;
  font-size: 12.5px;
  color: var(--c-mid);
  background: var(--ok-soft-bg);
  border-radius: 11px;
}

/* Kart iskeleti SeoCard ile aynı dil (gölgesiz, aynı kenar/yarıçap) */
.card {
  border: 1px solid var(--c-border-soft);
  border-radius: 13px;
  background: var(--c-card);
  overflow: hidden;
}
.tbl tr:last-child td {
  border-bottom: 0;
}
.row {
  cursor: pointer;
}
.row:hover td {
  background: var(--c-hover);
}
.c-work {
  white-space: nowrap;
}
.c-reason {
  text-align: right;
  white-space: nowrap;
}
.sku {
  font-size: 10.5px;
  color: var(--c-faint);
  margin-top: 1px;
}
/* Kaçırılan tıklama sıralama ölçütü — vurgulu */
.status {
  display: inline-flex;
  align-items: center;
  padding: 3px 9px;
  border-radius: 999px;
  font-size: 11.5px;
  font-weight: 600;
}

/* Kesitler — kayıp nerede toplanıyor */
.slices {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 12px;
}
.slice-group {
  display: flex;
  align-items: center;
  gap: 5px;
  flex-wrap: wrap;
}
.slice-label {
  font-size: 11px;
  color: var(--c-faint);
  width: 54px;
  flex: none;
}
.slice {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 9px;
  border: 1px solid var(--c-border-soft);
  border-radius: 7px;
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 11.5px;
  cursor: pointer;
  white-space: nowrap;
}
.slice b {
  font-variant-numeric: tabular-nums;
  color: var(--c-soft);
  font-weight: 640;
}
.slice.on {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-tint);
}
.slice.on b {
  color: var(--accent);
}

/* Filtre çipleri — ProductList.vue ile aynı dil */
.filters {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}
.filter {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: none;
  padding: 6px 11px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 12.5px;
  font-weight: 500;
  cursor: pointer;
  white-space: nowrap;
}
.filter.on {
  background: var(--accent);
  color: #fff;
  border-color: var(--accent);
  font-weight: 600;
}
.filter .count {
  font-size: 11px;
  padding: 0 6px;
  border-radius: 20px;
  background: var(--c-chip);
  color: var(--c-soft);
  font-weight: 600;
}
.filter.on .count {
  background: rgba(255, 255, 255, 0.22);
  color: #fff;
}
.filter.clear {
  border-style: dashed;
  color: var(--c-soft);
}
.fsep {
  width: 1px;
  height: 20px;
  background: var(--c-border);
  margin: 0 4px;
}
.link {
  color: var(--accent);
  cursor: pointer;
  font-weight: 560;
}

/* Kanibalizasyon — sorgu başlığı + altında yarışan sayfalar */
.cann-list {
  padding: 4px 0;
}
.cann {
  padding: 10px 16px;
  border-bottom: 1px solid var(--c-border-soft);
}
.cann:last-child {
  border-bottom: 0;
}
.cann-q {
  display: flex;
  align-items: baseline;
  gap: 10px;
  margin-bottom: 6px;
}
.cann-q .q {
  font-size: 13px;
  font-weight: 640;
  color: var(--c-text);
}
.cann-m {
  font-size: 11px;
  color: var(--c-soft);
  font-variant-numeric: tabular-nums;
}
.cann-p {
  display: flex;
  align-items: baseline;
  gap: 9px;
  padding: 4px 0 4px 12px;
  font-size: 12px;
  cursor: pointer;
  border-radius: 6px;
}
.cann-p:hover {
  background: var(--c-hover);
}
.cann-pos {
  width: 34px;
  flex: none;
  text-align: right;
  color: var(--c-soft);
  font-variant-numeric: tabular-nums;
  font-weight: 600;
}
.cann-c {
  margin-left: auto;
  flex: none;
  color: var(--c-faint);
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}

/* Satışta olmayan ama trafik alan sayfalar */
.eol {
  margin-top: 18px;
}
.succ {
  margin-top: 5px;
  display: flex;
  align-items: baseline;
  flex-wrap: wrap;
  gap: 6px;
  font-size: 11px;
}
.succ-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 2px 8px;
  border: 1px dashed var(--c-border);
  border-radius: 7px;
  background: transparent;
  color: var(--c-soft);
  font-size: 11px;
  cursor: pointer;
}
.succ-btn:hover:not(:disabled) {
  background: var(--c-hover);
  color: var(--c-mid);
}
.succ-btn:disabled {
  opacity: 0.5;
  cursor: default;
}
.succ-ok {
  display: inline-flex;
  /* Metin sarınca ikon ortada kalmasın — üste hizalı dursun. */
  align-items: flex-start;
  gap: 5px;
  color: var(--green);
  font-weight: 580;
  line-height: 1.4;
}
.succ-ok svg {
  flex: none;
  margin-top: 2px;
}
/* "Halef yok" bir başarısızlık değil, geçerli bir cevap — kırmızı DEĞİL, nötr. */
.succ-none {
  color: var(--c-soft);
  font-weight: 560;
}
.succ-why {
  color: var(--c-faint);
  line-height: 1.4;
}
/* Gerilemede "önce → sonra": eski değer soluk, yeni değer vurgulu */
.was {
  color: var(--c-faint);
}
.arrow {
  color: var(--c-faint);
  margin: 0 4px;
}
.cat-sync {
  flex: none;
  align-self: flex-start;
}
/* Onay modali — UpdateModal ile aynı dil */
.diff {
  border: 1px solid var(--c-border-soft);
  border-radius: 9px;
  overflow: hidden;
}
.d-row {
  display: flex;
  gap: 10px;
  padding: 9px 12px;
  font-size: 12px;
  border-bottom: 1px solid var(--c-border-soft);
}
.d-row:last-child {
  border-bottom: 0;
}
/* Vurgu YAZILACAK satırda — son satırda değil. Hedef adı satırı eklendiğinde
   `:last-child` vurguyu ona kaydırıyordu; asıl değişen değer "Olacak". */
.d-row.hi {
  background: var(--ok-soft-bg);
}
.chg {
  margin-left: 8px;
  font-size: 11px;
}

/* Hedef seçme: arama satırı + sonuç listesi */
.pick-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
.pick-in {
  flex: 1;
  min-width: 0;
  padding: 7px 10px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-input);
  color: var(--c-text);
  font-size: 12px;
  font-family: inherit;
}
.pick-in:focus {
  outline: none;
  border-color: var(--accent);
}
.pick-hint {
  margin-top: 8px;
  color: var(--c-soft);
  font-size: 11px;
  line-height: 1.45;
}
.pick-list {
  margin-top: 10px;
  max-height: 260px;
  overflow-y: auto;
  border: 1px solid var(--c-border-soft);
  border-radius: 9px;
}
.pick-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 11px;
  border-bottom: 1px solid var(--c-border-soft);
  cursor: pointer;
  transition: background 0.12s cubic-bezier(0.32, 0.72, 0, 1);
}
.pick-item:last-child {
  border-bottom: 0;
}
.pick-item:hover {
  background: var(--c-hover);
}
.pi-name {
  color: var(--c-text);
  font-size: 12px;
  font-weight: 560;
}
.pi-slug {
  color: var(--c-soft);
  font-size: 11px;
  overflow-wrap: anywhere;
}
.d-lab {
  width: 52px;
  flex: none;
  color: var(--c-soft);
}
.d-val {
  color: var(--c-text);
  overflow-wrap: anywhere;
}
.d-val.muted {
  color: var(--c-faint);
}
/* Uyarı kutusu — UpdateModal'daki ile aynı dil. Canonical'ın yönlendirme
   OLMADIĞINI söylüyor; gözden kaçmaması için vurgulu. */
/* Yalnızca yerel farklar; gerisi styles.css'teki `.warn` temelinden gelir.
   `flex-start`: metin birkaç satıra sarıyor, ikon ortada değil ÜSTTE durmalı. */
.warn {
  align-items: flex-start;
  padding: 9px 11px;
  line-height: 1.5;
  border-radius: 9px;
}
.warn svg {
  flex: none;
  margin-top: 2px;
}
.ghost {
  height: 36px;
  padding: 0 14px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 12.5px;
  font-weight: 560;
  cursor: pointer;
}

.eol-more {
  padding: 10px 16px;
  border-top: 1px solid var(--c-border-soft);
  font-size: 12px;
}

/* Görünmeyenler */
.inv {
  margin-top: 18px;
}
.inv-list {
  max-height: 260px;
  overflow-y: auto;
}
.inv-row {
  display: flex;
  align-items: baseline;
  gap: 10px;
  padding: 8px 16px;
  border-bottom: 1px solid var(--c-border-soft);
  cursor: pointer;
  font-size: 12.5px;
}
.inv-row:last-child {
  border-bottom: 0;
}
.inv-row:hover {
  background: var(--c-hover);
}
.inv-row .nm {
  -webkit-line-clamp: 1;
  line-clamp: 1;
}
</style>
