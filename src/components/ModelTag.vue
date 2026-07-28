<script setup lang="ts">
/**
 * "Bu içeriği hangi model üretti" rozeti.
 *
 * Neden gerekli: model zinciri kota dolduğunda alt modellere düşüyor ve ücretsiz katmanda
 * günlük limitler 25× fark ediyor (normal flash 20/gün, flash-lite 500/gün, Gemma 14.400/gün).
 * Kullanıcı hangi modelde olduğunu görmeden "devam mı edeyim yoksa limitler yenilensin mi
 * bekleyeyim" kararını veremez.
 */
import { computed } from "vue";

const props = defineProps<{ model: string | null }>();

/** Son çare modeli farklı bir aile (Gemma) — üslup Gemini'lerden sapabilir, belirtilmeli. */
const isFallback = computed(() => !!props.model && props.model.startsWith("gemma"));

/** `gemini-3.6-flash` → `3.6 Flash` — rozet dar, önek her satırda tekrar etmesin. */
const short = computed(() => {
  const m = props.model ?? "";
  if (!m) return "";
  if (m.startsWith("gemma")) return m.replace(/^gemma-/, "Gemma ").replace(/-it$/, "");
  return m
    .replace(/^gemini-/, "")
    .replace(/-/g, " ")
    .replace(/\b\w/g, (c) => c.toUpperCase());
});

/**
 * Kısa tutuluyor: baloncuk kartın İÇİNDE açılıyor (`.card { overflow: hidden }`) ve
 * uzun metin kart alt kenarından taşıp kırpılıyor — denendi, kırpıldı. İki satırı geçmemeli.
 */
const tip = computed(() =>
  isFallback.value
    ? `${props.model} ile üretildi — son çare model, diğerlerinin günlük limiti dolmuştu.`
    : `${props.model} ile üretildi.`,
);
</script>

<template>
  <span v-if="model" class="model-tag tip-below" :class="{ fallback: isFallback }" :data-tip="tip">
    <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor"
         stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path d="M12 3l1.9 5.8L20 10l-5.2 3.1L16 19l-4-3.2L8 19l1.2-5.9L4 10l6.1-1.2z" />
    </svg>
    <b>{{ short }}</b>
  </span>
</template>
