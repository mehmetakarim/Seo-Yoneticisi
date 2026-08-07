<script setup lang="ts">
/**
 * Yükselmeye yakın sorgular — pozisyon 4–20, gösterim eşiğin üstünde.
 *
 * Bu ekran "sorun var" ile "şunu yaz" arasındaki farkı kapatıyor: sayfa düzeyinde
 * "bu sayfa az tıklanıyor" demek yerine, HANGİ ARAMADA kaçıncı sırada olduğunuzu söylüyor.
 * Satırdaki sorgu o ürün için doğrudan **hedef kelime adayı** — döngü burada kapanıyor:
 * GSC fırsatı buluyor, operatör seçiyor, mevcut üretim yazıyor.
 *
 * Ölçüm (2026-07-29): 120 sorgu / 69 ürün.
 */
import { computed, ref } from "vue";
import { useStore } from "../../store";
import ToolShell from "./ToolShell.vue";
import SeoTable, { type TableCol, type TableRow } from "./SeoTable.vue";

const store = useStore();
const veri = computed(() => store.opportunity?.striking ?? []);
const limit = ref(40);

const cols: TableCol[] = [
  { key: "name", label: "Sorgu / Ürün", type: "text" },
  { key: "imp", label: "Gösterim", type: "num" },
  { key: "clk", label: "Tıklama", type: "num" },
  { key: "pos", label: "Konum", type: "num" },
  { key: "miss", label: "Kaçırılan", type: "num", emphasis: "down" },
  { key: "act", label: "İşlem", type: "actions" },
];

const n = (x: number) => Math.round(x).toString();
const rows = computed<TableRow[]>(() =>
  veri.value.slice(0, limit.value).map((q) => ({
    id: q.sku + q.query,
    name: q.query,
    sub: q.name,
    values: { imp: n(q.impressions), clk: n(q.clicks), pos: q.position.toFixed(1), miss: n(q.missed_clicks) },
    actions: [{ key: "open" }],
  })),
);

/** Satırın kimliği sku+sorgu; eylem için sku'ya geri dönmek gerekiyor. */
function skuOf(id: string): string | undefined {
  return veri.value.find((q) => q.sku + q.query === id)?.sku;
}
function ac(id: string) {
  const sku = skuOf(id);
  if (sku) store.openProduct(sku);
}
</script>

<template>
  <ToolShell
    :empty="!veri.length"
    empty-text="4–20. sırada, iyileştirilmeye değer bir arama bulunamadı."
  >
    <p class="note">
      Bu aramalarda 4–20. sıradasınız — küçük bir iyileştirme ilk sıralara taşıyabilir.
      Sorgu, o ürün için <b>hedef kelime adayıdır</b>.
    </p>

    <SeoTable
      :cols="cols"
      :rows="rows"
      :summary="`${veri.length} sorgu · 4–20. sırada`"
      :count-label="`${rows.length} / ${veri.length} satır`"
      :more-label="veri.length > limit ? `Sonraki 40'ı göster (${veri.length - limit} kaldı)` : ''"
      @row="ac"
      @action="ac($event.id)"
      @more="limit += 40"
    />
  </ToolShell>
</template>

<style scoped>
/* ⚠️ Tablo geometrisi burada YOK — hepsi SeoTable'da. Kalan tek şey ekranın kendi açıklaması. */
.note {
  margin: 0 0 12px;
  max-width: 640px;
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--c-soft);
}
</style>
