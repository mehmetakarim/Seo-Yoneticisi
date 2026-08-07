<script setup lang="ts">
/**
 * Birbiriyle yarışan sayfalar (kanibalizasyon) — bir aramada ≥2 ürün sayfamız görünüyor
 * ve hiçbirinin baskın payı yok (%70 eşiği; tıklama yoksa gösterim payına düşer).
 *
 * ⚠️ **Otomatik birleştirme bilinçli olarak ÖNERİLMİYOR** — QueryLoom araştırması da elle
 * inceleme diyor. Yanlış birleştirme geri alınması zor bir SEO hasarı; karar operatörde.
 * Bu ekran tespit eder, uygulamaz.
 *
 * Ölçüm (2026-07-29): 21.634 sorgu içinden yalnızca 3 tanesi — gürültüsüz bir liste.
 *
 * ⚠️ Bu ekran **gruplu varyantı** kullanıyor: veri gerçekten hiyerarşik (bir sorgu → o
 * sorguda yarışan N sayfa). Düz tabloya zorlamak veriyi yanlış anlatırdı (Faz B kararı).
 */
import { computed } from "vue";
import { useStore } from "../../store";
import ToolShell from "./ToolShell.vue";
import { useRowFocus } from "../../useRowFocus";
import SeoTable, { type TableCol, type TableGroup } from "./SeoTable.vue";

const store = useStore();
const veri = computed(() => store.opportunity?.cannibalization ?? []);

const cols: TableCol[] = [
  { key: "name", label: "Sayfa", type: "text" },
  { key: "pos", label: "Konum", type: "num" },
  { key: "clk", label: "Tıklama", type: "num" },
  { key: "act", label: "İşlem", type: "actions" },
];

const n = (x: number) => Math.round(x).toString();
/** ⚠️ Burada odak bir SATIRA değil GRUBA gidiyor: kuyruk maddesi bir sorgu, satırlar ise
 *  o sorguda yarışan sayfalar. SeoTable grup başlıklarını da `data-row-id` ile işaretliyor. */
const focusId = useRowFocus("cannibal", () => []);

const groups = computed<TableGroup[]>(() =>
  veri.value.map((c) => ({
    label: c.query,
    count: `${c.pages.length} sayfa`,
    meta: `${n(c.impressions)} gösterim · ${n(c.clicks)} tıklama`,
    rows: c.pages.map((pg) => ({
      id: pg.sku,
      name: pg.name,
      values: { pos: pg.position.toFixed(1), clk: n(pg.clicks) },
      actions: [{ key: "open" as const }],
    })),
  })),
);
</script>

<template>
  <ToolShell
    :empty="!veri.length"
    empty-text="Kendi sayfalarınız birbiriyle yarışmıyor — her arama için tek bir sayfanız öne çıkıyor."
  >
    <p class="note">
      Aynı aramada birden çok ürün sayfanız görünüyor ve hiçbiri öne çıkamıyor.
      <b>Otomatik birleştirme önerilmez</b> — önce hangi sayfanın o aramayı sahiplenmesi
      gerektiğine karar verin, diğerlerini farklılaştırın.
    </p>

    <SeoTable
      :cols="cols"
      :groups="groups"
      :focus-id="focusId"
      :summary="`${veri.length} aramada ${groups.reduce((a, g) => a + g.rows.length, 0)} sayfa yarışıyor`"
      @row="store.openProduct($event)"
      @action="store.openProduct($event.id)"
    />
  </ToolShell>
</template>

<style scoped>
.note {
  margin: 0 0 12px;
  max-width: 640px;
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--c-soft);
}
</style>
