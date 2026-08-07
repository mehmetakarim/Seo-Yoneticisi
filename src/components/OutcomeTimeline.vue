<script setup lang="ts">
/**
 * Ürün detayındaki olay/sonuç zaman çizelgesi (Faz Ö).
 *
 * "Ne yaptık, ne zaman" ve —mağazaya ulaşan işler için— "sonra ne oldu".
 *
 * ⚠️ Sonuç rozeti yalnızca **en yeni** mağaza olayında gösteriliyor: eski gönderimin etkisi
 * yenisi tarafından zaten ezilmiş olur, her satıra rozet basmak çift sayım izlenimi verirdi.
 *
 * ⚠️ Gönderim kaydı hiç yoksa bu açıkça söyleniyor. İçeriği elle kopyalayıp mağazaya
 * yapıştıran kullanıcı için olay oluşmuyor; sessizce boş bırakmak "ölçtük, bir şey çıkmadı"
 * gibi okunurdu — oysa hiç ölçülmedi.
 */
import { ref, watch } from "vue";
import { api } from "../api";
import { useStore } from "../store";
import type { ProductTimeline } from "../types";
import Icon from "./Icon.vue";
import SeoCard from "./SeoCard.vue";

const store = useStore();
const data = ref<ProductTimeline | null>(null);
const busy = ref(false);

async function yukle(sku: string | null) {
  data.value = null;
  if (!sku) return;
  busy.value = true;
  try {
    data.value = await api.getProductTimeline(sku);
  } catch {
    // Sessiz: zaman çizelgesi yardımcı bir katman, ürün ekranını düşürmemeli.
  } finally {
    busy.value = false;
  }
}
watch(() => store.selectedSku, yukle, { immediate: true });

/** `2026-08-01T13:20:00` → `01 Ağu 13:20` */
const AYLAR = ["Oca", "Şub", "Mar", "Nis", "May", "Haz", "Tem", "Ağu", "Eyl", "Eki", "Kas", "Ara"];
function tarih(s: string) {
  const g = s.slice(8, 10);
  const a = AYLAR[Number(s.slice(5, 7)) - 1] ?? "";
  return `${g} ${a} ${s.slice(11, 16)}`;
}
</script>

<template>
  <SeoCard
    v-if="busy || (data && data.items.length)"
    icon="chartLine"
    title="Geçmiş ve sonuç"
    sub="Ne yaptık, sonra ne oldu"
    stack
  >
    <div v-if="busy" class="info">
      <Icon name="loader" :size="13" class="spin" />
      Yükleniyor…
    </div>

    <template v-else-if="data">
      <!-- Ölçüm için gönderim şart: yerel "tamamlandı" Google'ın gördüğünü değiştirmiyor. -->
      <div v-if="!data.has_store_event" class="info">
        <Icon name="info" :size="13" />
        Bu ürün <b>ölçülemiyor</b> — mağazaya gönderim kaydı yok. İçeriği kopyalayıp elle
        yapıştırdıysanız uygulama bunu göremez; <b>Gönder</b> düğmesini kullanırsanız sonraki
        değişikliğin etkisi ölçülür.
      </div>

      <ol class="tl">
        <li v-for="(it, i) in data.items" :key="i" class="ev">
          <span class="dot" :class="{ store: !!it.outcome_label }"></span>
          <div class="ev-body">
            <div class="ev-top">
              <span class="ev-label">{{ it.label }}</span>
              <span class="ev-at">{{ tarih(it.at) }}</span>
            </div>
            <span
              v-if="it.outcome_label"
              class="badge tip-below"
              :data-tip="it.outcome_tip"
              :style="{
                background: `var(--badge-${it.outcome_tone}-bg)`,
                color: `var(--badge-${it.outcome_tone}-c)`,
              }"
            >
              <span class="bdot"></span>{{ it.outcome_label }}
            </span>
          </div>
        </li>
      </ol>
    </template>
  </SeoCard>
</template>

<style scoped>
.info {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--c-soft);
}
.tl {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
}
.ev {
  display: flex;
  gap: 10px;
  padding: 7px 0;
  position: relative;
}
/* Olaylar arası çizgi — sonuncuda çizilmiyor. */
.ev:not(:last-child)::before {
  content: "";
  position: absolute;
  left: 3.5px;
  top: 18px;
  bottom: -2px;
  width: 1px;
  background: var(--c-border-soft);
}
.dot {
  width: 8px;
  height: 8px;
  flex: none;
  margin-top: 5px;
  border-radius: 50%;
  background: var(--c-border);
  z-index: 1;
}
/* Mağazaya ulaşan olay dolgulu: ölçülen olay budur. */
.dot.store {
  background: var(--accent);
}
.ev-body {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.ev-top {
  display: flex;
  align-items: baseline;
  gap: 8px;
  min-width: 0;
}
.ev-label {
  font-size: 12.5px;
  color: var(--c-text);
}
.ev-at {
  font-size: 10.5px;
  color: var(--c-faint);
  white-space: nowrap;
  font-variant-numeric: tabular-nums;
}
.badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 2px 8px;
  border-radius: 20px;
  font-size: 10.5px;
  font-weight: 600;
  white-space: nowrap;
}
.bdot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: currentColor;
  flex: none;
}
</style>
