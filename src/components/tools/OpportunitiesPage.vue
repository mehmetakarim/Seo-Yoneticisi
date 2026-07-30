<script setup lang="ts">
/**
 * Fırsatlar — "önce hangi ürüne bakmalıyım?"
 *
 * Katalogdaki ürünler arasında elle seçim yapmak yerine, Google Search Console verisiyle
 * emeğin en çok getiri sağlayacağı ürünleri sıralar. Sıralama ölçütü soyut bir puan değil
 * **kaçırılan tıklama**: "bu sayfa, konumunun getirmesi gereken tıklamanın kaçını alamıyor".
 *
 * "Google'da görünmeyenler" de burada duruyor (ayrı bir araç ekranı değil): aynı soruyu
 * cevaplıyor — hangi üründe emek harcayayım — ve tek haneli bir liste; kendi ekranını hak
 * edecek kadar büyük değil. Yine de ayrı bir bölüm, çünkü çözümü farklı: bu bir meta işi
 * değil, indeksleme işi.
 */
import { computed, ref } from "vue";
import { useStore } from "../../store";
import { workState, type Opportunity, type OpportunityReason, type WorkState } from "../../types";
import Icon from "../Icon.vue";
import AnalysisSection from "../AnalysisSection.vue";
import ToolShell from "./ToolShell.vue";

const store = useStore();
const report = computed(() => store.opportunity);
const all = computed<Opportunity[]>(() => report.value?.opportunities ?? []);

// --- Filtreler ---
// Tamamen istemcide: 60 satır için sunucuya gitmek gereksiz gecikme olurdu.
const workFilter = ref<WorkState | "all">("all");
const reasonFilter = ref<OpportunityReason | "all">("all");
/** Kategori veya marka kesiti — "cat:Notebook" / "brand:Lenovo" biçiminde. */
const sliceFilter = ref<string>("");

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

const totalMissed = computed(() =>
  Math.round(filtered.value.reduce((s, o) => s + o.missed_clicks, 0)),
);
const pct = (n: number) => (n * 100).toFixed(1).replace(".", ",");
</script>

<template>
  <ToolShell
    :empty="!all.length && !report?.invisible?.length"
    empty-text="Fırsat bulunamadı — Google'da bulunan ürünlerin tamamı konumuna göre beklenen performansta."
  >
    <div v-if="all.length" class="head">
      <b>{{ all.length }}</b> fırsat · yaklaşık <b>{{ totalMissed }}</b> tıklama kaçırılıyor
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
    <div v-else-if="all.length" class="clean">
      <Icon name="search" :size="16" :stroke-width="2.2" style="color: var(--c-soft)" />
      Bu filtrede fırsat yok.
      <a class="link" @click="clearFilters()">Filtreyi temizle</a>
    </div>

    <!-- Google'da hiç görünmeyenler: farklı bir iş, bilinçli olarak ayrı bölüm -->
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
  </ToolShell>
</template>

<style scoped>
.head {
  margin-bottom: 12px;
  font-size: 12.5px;
  color: var(--c-text);
}
.head b {
  font-weight: 660;
  font-variant-numeric: tabular-nums;
}
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
  font-family: inherit;
}
.slice:hover {
  background: var(--c-hover);
}
.slice.on {
  border-color: var(--accent);
  background: var(--accent-tint);
  color: var(--accent);
}
.slice b {
  font-variant-numeric: tabular-nums;
  color: var(--c-soft);
  font-weight: 640;
}
.slice.on b {
  color: var(--accent);
}
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
  font-family: inherit;
}
.filter:hover {
  background: var(--c-hover);
}
.filter.on {
  border-color: var(--accent);
  background: var(--accent-tint);
  color: var(--accent);
  font-weight: 580;
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
  background: rgba(255, 255, 255, 0.55);
  color: var(--accent);
}
.fsep {
  width: 1px;
  height: 20px;
  background: var(--c-border);
  margin: 0 4px;
}
.status {
  display: inline-flex;
  align-items: center;
  padding: 3px 9px;
  border-radius: 999px;
  font-size: 11.5px;
  font-weight: 600;
}
.c-work {
  white-space: nowrap;
}
.c-reason {
  text-align: right;
  white-space: nowrap;
}
.row {
  cursor: pointer;
}
.row:hover {
  background: var(--c-hover);
}
.sku {
  font-size: 10.5px;
  color: var(--c-faint);
  margin-top: 1px;
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
.link {
  color: var(--accent);
  cursor: pointer;
  font-weight: 560;
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
