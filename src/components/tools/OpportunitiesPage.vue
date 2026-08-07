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
import AnalysisSection from "../AnalysisSection.vue";
import ToolShell from "./ToolShell.vue";
import SeoTable, { type Chip, type TableCol, type TableRow } from "./SeoTable.vue";

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

const cols: TableCol[] = [
  { key: "name", label: "Ürün", type: "text" },
  { key: "durum", label: "Durum", type: "badge" },
  { key: "imp", label: "Gösterim", type: "num" },
  { key: "clk", label: "Tıklama", type: "num" },
  { key: "ctr", label: "CTR", type: "pct" },
  { key: "pos", label: "Konum", type: "num" },
  { key: "miss", label: "Kaçırılan", type: "num", emphasis: "down" },
  { key: "sebep", label: "Sebep", type: "badge" },
  { key: "act", label: "İşlem", type: "actions" },
];

const rows = computed<TableRow[]>(() =>
  filtered.value.map((o) => {
    const w = WORK[workState(o)];
    const r = REASON[o.reason];
    return {
      id: o.sku,
      name: o.name,
      sub: o.sku,
      badges: {
        durum: { label: w.label, tone: w.badge, tip: w.tip },
        sebep: { label: r.label, tone: r.badge, tip: r.tip },
      },
      values: {
        imp: Math.round(o.impressions),
        clk: Math.round(o.clicks),
        ctr: `%${pct(o.ctr)}`,
        pos: o.position.toFixed(1),
        miss: Math.round(o.missed_clicks),
      },
      actions: [{ key: "open" as const }],
    };
  }),
);

/** Kesit çipi: aynı kesite ikinci tıklama filtreyi kaldırır. */
function sliceChip(kind: "cat" | "brand", name: string, missed: number): Chip {
  const key = `${kind}:${name}`;
  return {
    label: name,
    count: Math.round(missed),
    active: sliceFilter.value === key,
    onClick: () => (sliceFilter.value = sliceFilter.value === key ? "" : key),
  };
}

/**
 * Üst şeritteki çip satırları. Eskiden iş durumu ve sebep tek satırda, aralarında görünmez
 * bir ayraçla duruyordu; ayrı ve ETİKETLİ satırlar hem daha okunur hem kesitlerle tutarlı.
 */
const chipRows = computed(() => [
  { label: "Kategori", items: topCats.value.map((c) => sliceChip("cat", c.name, c.missed)) },
  { label: "Marka", items: topBrands.value.map((b) => sliceChip("brand", b.name, b.missed)) },
  {
    label: "İş durumu",
    items: [
      { label: "Hepsi", count: all.value.length, active: workFilter.value === "all",
        onClick: () => (workFilter.value = "all") },
      ...(["untouched", "partial", "worked"] as const).map((w) => ({
        label: WORK[w].label,
        count: workCounts.value[w],
        active: workFilter.value === w,
        onClick: () => (workFilter.value = workFilter.value === w ? "all" : w),
      })),
    ] as Chip[],
  },
  {
    label: "Sebep",
    items: [
      ...(["low_ctr", "no_clicks", "second_page"] as const).map((r) => ({
        label: REASON[r].label,
        count: reasonCounts.value[r],
        active: reasonFilter.value === r,
        onClick: () => (reasonFilter.value = reasonFilter.value === r ? "all" : r),
      })),
      ...(anyFilter.value
        ? [{ label: "Filtreyi temizle", onClick: () => clearFilters() } as Chip]
        : []),
    ] as Chip[],
  },
]);

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
    <SeoTable
      v-if="all.length"
      :cols="cols"
      :rows="rows"
      :chip-rows="chipRows"
      :state="!rows.length ? 'empty' : 'normal'"
      :summary="`${all.length} fırsat · yaklaşık ${totalMissed} tıklama kaçırılıyor`"
      :count-label="`${rows.length} / ${all.length} satır`"
      empty-label="Bu filtrede fırsat yok"
      empty-hint="Filtreleri temizleyin veya başka bir kesit seçin."
      @row="store.openProduct($event)"
      @action="store.openProduct($event.id)"
    />

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

<style scoped>.head b {
  font-weight: 660;
  font-variant-numeric: tabular-nums;
}.slice-group {
  display: flex;
  align-items: center;
  gap: 5px;
  flex-wrap: wrap;
}.slice {
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
}.slice.on {
  border-color: var(--accent);
  background: var(--accent-tint);
  color: var(--accent);
}.slice.on b {
  color: var(--accent);
}.filter {
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
}.filter.on {
  border-color: var(--accent);
  background: var(--accent-tint);
  color: var(--accent);
  font-weight: 580;
}.filter.on .count {
  background: rgba(255, 255, 255, 0.55);
  color: var(--accent);
}.status {
  display: inline-flex;
  align-items: center;
  padding: 3px 9px;
  border-radius: 999px;
  font-size: 11.5px;
  font-weight: 600;
}.c-reason {
  text-align: right;
  white-space: nowrap;
}.row:hover {
  background: var(--c-hover);
}link {
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
