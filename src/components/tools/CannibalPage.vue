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
 */
import { computed } from "vue";
import { useStore } from "../../store";
import ToolShell from "./ToolShell.vue";

const store = useStore();
const rows = computed(() => store.opportunity?.cannibalization ?? []);
</script>

<template>
  <ToolShell
    :empty="!rows.length"
    empty-text="Kendi sayfalarınız birbiriyle yarışmıyor — her arama için tek bir sayfanız öne çıkıyor."
  >
    <div class="head">
      <b>{{ rows.length }}</b> arama
      <p class="note">
        Aynı aramada birden çok ürün sayfanız görünüyor ve hiçbiri öne çıkamıyor.
        <b>Otomatik birleştirme önerilmez</b> — önce hangi sayfanın o aramayı sahiplenmesi
        gerektiğine karar verin, diğerlerini farklılaştırın.
      </p>
    </div>

    <div class="card">
      <div class="cann-list">
        <div v-for="c in rows" :key="c.query" class="cann">
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
            :title="`${pg.name} — ürüne git`"
            @click="store.openProduct(pg.sku)"
          >
            <span class="cann-pos">{{ pg.position.toFixed(1) }}.</span>
            <span class="nm">{{ pg.name }}</span>
            <span class="cann-c">{{ Math.round(pg.clicks) }} tık</span>
          </div>
        </div>
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
</style>
