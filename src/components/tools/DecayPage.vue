<script setup lang="ts">
/**
 * Düşüşte olanlar — önceki döneme göre trafik veya sıra kaybeden sayfalar.
 *
 * Neden ayrı bir araç: düşüşteki bir sayfa, hiç yükselmemiş olandan daha aciltir. Orada bir
 * şey BOZULMUŞ; müdahale edilmezse kayıp büyür. Ölçüm (2026-07-29): 49 sayfa, 907 tıklama
 * kaybı — ilk 10'un 6'sı 3D yazıcı, yani kayıp rastgele değil kategori kümeleniyor.
 */
import { computed, ref } from "vue";
import { useStore } from "../../store";
import ToolShell from "./ToolShell.vue";
import SeoTable, { type TableCol, type TableRow } from "./SeoTable.vue";

const store = useStore();
const veri = computed(() => store.opportunity?.decay ?? []);
const days = computed(() => store.opportunity?.days ?? 90);
const lost = computed(() => Math.round(veri.value.reduce((a, d) => a + d.clicks_lost, 0)));

/** Uzun listede önce en çok kaybedeni göster. */
const limit = ref(30);

const cols: TableCol[] = [
  { key: "name", label: "Ürün", type: "text" },
  { key: "clk", label: "Tıklama", type: "change" },
  { key: "pos", label: "Konum", type: "change" },
  { key: "loss", label: "Kayıp", type: "num", emphasis: "down" },
  { key: "act", label: "İşlem", type: "actions" },
];

const n = (x: number) => Math.round(x).toString();
const rows = computed<TableRow[]>(() =>
  veri.value.slice(0, limit.value).map((d) => ({
    id: d.sku,
    name: d.name,
    sub: d.sku,
    // Konum düşüşü sayı olarak ARTIŞ demek (3.4 → 9.8 kötüleşme) — ikisi de "down".
    changes: {
      clk: { from: n(d.clicks_before), to: n(d.clicks_now), tone: "down" },
      pos: { from: d.position_before.toFixed(1), to: d.position_now.toFixed(1), tone: "down" },
    },
    values: { loss: `−${n(d.clicks_lost)}` },
    actions: [{ key: "open" }],
  })),
);
</script>

<template>
  <ToolShell
    :empty="!veri.length"
    empty-text="Gerileyen sayfa yok — önceki döneme göre kayıp yaşayan ürün bulunamadı."
  >
    <p class="note">
      Önceki {{ days }} güne göre gerileyen sayfalar. Konum değişmeden tıklama düştüyse sorun
      sıralamada değil, arama sonucundaki görünümde olabilir — başlık ve açıklamaya bakın.
    </p>

    <SeoTable
      :cols="cols"
      :rows="rows"
      :summary="`${veri.length} sayfa geriledi · ${lost} tıklama kaybı`"
      :count-label="`${rows.length} / ${veri.length} satır`"
      :more-label="veri.length > limit ? `Sonraki 30'u göster (${veri.length - limit} kaldı)` : ''"
      @row="store.openProduct($event)"
      @action="store.openProduct($event.id)"
      @more="limit += 30"
    />
  </ToolShell>
</template>

<style scoped>
/* Tablo geometrisi SeoTable'da; burada yalnızca ekranın açıklaması kalıyor. */
.note {
  margin: 0 0 12px;
  max-width: 640px;
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--c-soft);
}
</style>
