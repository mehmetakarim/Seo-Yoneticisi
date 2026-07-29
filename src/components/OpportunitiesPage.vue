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

    <!-- Striking distance: hangi SORGUDA kaçıncı sıradasınız — "ne yazmalıyım" katmanı -->
    <div v-if="report?.striking?.length" class="card sec">
      <div class="inv-head">
        <div>
          <div class="inv-title">
            Yükselmeye yakın sorgular ({{ report.striking.length }})
          </div>
          <div class="inv-sub">
            Bu aramalarda 4–20. sıradasınız — küçük bir iyileştirme ilk sıralara taşıyabilir.
            Sorgu, o ürün için <b>hedef kelime adayıdır</b>: satıra tıklayıp ürüne gidin.
          </div>
        </div>
      </div>
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
    </div>

    <!-- Kanibalizasyon: kendi sayfalarımız birbiriyle yarışıyor -->
    <div v-if="report?.cannibalization?.length" class="card sec">
      <div class="inv-head">
        <div>
          <div class="inv-title">
            Birbiriyle yarışan sayfalar ({{ report.cannibalization.length }})
          </div>
          <div class="inv-sub">
            Aynı aramada birden çok ürün sayfanız görünüyor ve hiçbiri öne çıkamıyor.
            <b>Otomatik birleştirme önerilmez</b> — önce hangi sayfanın o aramayı sahiplenmesi
            gerektiğine karar verin, diğerlerini farklılaştırın.
          </div>
        </div>
      </div>
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
    </div>

    <!-- Satışta olmayan ama trafik alan sayfalar. Ölçüm: ürün trafiğinin %69'u burada. -->
    <div v-if="report?.eol?.length" class="card eol">
      <div class="inv-head">
        <div>
          <div class="inv-title">
            Satışta olmayan ama trafik alan sayfalar ({{ report.eol.length }})
            <span class="eol-sum">{{ Math.round(report.eol_clicks) }} tıklama</span>
          </div>
          <div class="inv-sub">
            Bu adresler kataloğunuzda yok ama Google'da hâlâ sıralanıyor — ziyaretçi geliyor,
            ürünü satın alamıyor. Çözüm genelde güncel nesle <b>301 yönlendirme</b>; yönlendirmeyi
            IdeaSoft panelinden tanımlamanız gerekir, uygulama bunu yapamaz.
            Bazı sayfaları bilinçli tutuyor olabilirsiniz — liste öneridir, karar sizin.
          </div>
        </div>
      </div>
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
          <tr v-for="e in report.eol.slice(0, eolLimit)" :key="e.url">
            <td class="c-name"><div class="nm">{{ e.slug }}</div></td>
            <td class="c-num miss">{{ Math.round(e.clicks) }}</td>
            <td class="c-num">{{ Math.round(e.impressions) }}</td>
            <td class="c-num">{{ e.position.toFixed(1) }}</td>
          </tr>
        </tbody>
      </table>
      <div v-if="report.eol.length > eolLimit" class="eol-more">
        <a class="link" @click="eolLimit += 50">
          Sonraki 50'yi göster ({{ report.eol.length - eolLimit }} kaldı)
        </a>
      </div>
    </div>

    <!-- Google'da hiç görünmeyenler: farklı bir iş, bilinçli olarak ayrı -->
    <div v-if="report?.invisible?.length" class="card inv">
      <div class="inv-head">
        <div>
          <div class="inv-title">Google'da görünmeyenler ({{ report.invisible.length }})</div>
          <div class="inv-sub">
            Son {{ report.days }} günde hiç gösterim almamışlar. Bu bir meta sorunu değil —
            indeksleme veya görünürlük işi; içerik üretmek tek başına çözmez.
          </div>
        </div>
      </div>
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
    </div>
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
.tbl {
  width: 100%;
  border-collapse: collapse;
}
.tbl th {
  position: sticky;
  top: 0;
  z-index: 1;
  background: var(--c-input);
  border-bottom: 1px solid var(--c-border-soft);
  padding: 9px 12px;
  font-size: 11px;
  font-weight: 620;
  color: var(--c-soft);
  text-align: left;
  white-space: nowrap;
}
.tbl td {
  padding: 9px 12px;
  border-bottom: 1px solid var(--c-border-soft);
  font-size: 12.5px;
  color: var(--c-text);
  vertical-align: middle;
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
.c-name {
  width: 36%;
}
.c-work {
  white-space: nowrap;
}
.c-num {
  text-align: right;
  white-space: nowrap;
  font-variant-numeric: tabular-nums;
}
.c-reason {
  text-align: right;
  white-space: nowrap;
}
.nm {
  font-weight: 560;
  line-height: 1.35;
  /* uzun ürün adları satırı şişirmesin */
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.sku {
  font-size: 10.5px;
  color: var(--c-faint);
  margin-top: 1px;
}
/* Kaçırılan tıklama sıralama ölçütü — vurgulu */
.miss {
  font-weight: 660;
  color: var(--c-text);
}
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

.sec {
  margin-top: 18px;
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
.eol-sum {
  margin-left: 8px;
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--warn-bg);
  color: var(--warn-text);
  font-size: 11px;
  font-weight: 640;
  font-variant-numeric: tabular-nums;
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
.inv-head {
  padding: 13px 16px;
  border-bottom: 1px solid var(--c-border-soft);
}
.inv-title {
  font-size: 13.5px;
  font-weight: 640;
  color: var(--c-text);
}
.inv-sub {
  font-size: 11.5px;
  color: var(--c-soft);
  margin-top: 2px;
  line-height: 1.5;
  max-width: 640px;
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
.spin {
  animation: spin 0.8s linear infinite;
}
</style>
