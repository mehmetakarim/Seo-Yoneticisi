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

const store = useStore();
const rows = computed(() => store.opportunity?.striking ?? []);
const limit = ref(40);
</script>

<template>
  <ToolShell
    :empty="!rows.length"
    empty-text="4–20. sırada, iyileştirilmeye değer bir arama bulunamadı."
  >
    <div class="head">
      <b>{{ rows.length }}</b> sorgu
      <p class="note">
        Bu aramalarda 4–20. sıradasınız — küçük bir iyileştirme ilk sıralara taşıyabilir.
        Sorgu, o ürün için <b>hedef kelime adayıdır</b>: satıra tıklayıp ürüne gidin.
      </p>
    </div>

    <div class="card">
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
            v-for="q in rows.slice(0, limit)"
            :key="q.sku + q.query"
            class="row"
            :title="`${q.name} — ürüne git`"
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
      <div v-if="rows.length > limit" class="more">
        <a class="link" @click="limit += 40">
          Sonraki 40'ı göster ({{ rows.length - limit }} kaldı)
        </a>
      </div>
    </div>
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
.note {
  margin: 6px 0 0;
  max-width: 640px;
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--c-soft);
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
  margin-top: 2px;
}
.more {
  padding: 10px 16px;
  border-top: 1px solid var(--c-border-soft);
  font-size: 12px;
}
</style>
