<script setup lang="ts">
import { computed, ref } from "vue";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { density, wordCount } from "../validation";
import { useStore } from "../store";
import Icon from "./Icon.vue";
import SeoCard from "./SeoCard.vue";
import VersionHistory from "./VersionHistory.vue";

import { BADGE_LABEL } from "../validation";
import type { MetaBadge, DetailsVersionMeta } from "../types";

const props = defineProps<{
  detailsHtml: string;
  keyword: string;
  detailsDone: boolean;
  badge: MetaBadge;
  imageCount: number;
  /** İçeriği üreten Gemini modeli (null = henüz üretilmedi). */
  model: string | null;
  /** Yeniden üretimden önceki hâller (en yeni başta). */
  history: DetailsVersionMeta[];
}>();

const histOpen = ref(false);

const store = useStore();

// Faz 7: en az 3 galeri görseli yoksa üretim engellenir.
const imageGateOk = computed(() => props.imageCount >= 3);

// Üret butonunun tooltip'i: engelliyken sebebi, normalde maliyet uyarısı.
const genTip = computed(() =>
  imageGateOk.value
    ? "Uzun içerik üretir · daha fazla kredi harcar"
    : `En az 3 görsel gerekli — şu an ${props.imageCount}/4`,
);

const badgeStyle = computed(() => ({
  background: `var(--badge-${props.badge}-bg)`,
  color: `var(--badge-${props.badge}-c)`,
}));

const words = computed(() => wordCount(props.detailsHtml));
const hasContent = computed(() => !!props.detailsHtml && props.detailsHtml.trim().length > 0);

const dens = computed(() => density(props.detailsHtml, props.keyword));
const densOk = computed(() => dens.value >= 1.5 && dens.value <= 3.5);
const densText = computed(() => (words.value === 0 ? "—" : "%" + dens.value.toFixed(1)));
const densNote = computed(() => {
  if (words.value === 0) return "içerik yok";
  if (dens.value < 1.5) return "düşük";
  return densOk.value ? "ideal" : "yüksek";
});
const densColor = computed(() => {
  if (words.value === 0) return "var(--c-faint)";
  if (densOk.value) return "var(--green)";
  return dens.value < 1.5 ? "var(--amber)" : "var(--red)";
});
const densMark = computed(() => Math.min(dens.value / 5, 1) * 100 + "%");

const wordBar = computed(() => Math.min(words.value / 50, 1) * 100 + "%");
const wordColor = computed(() =>
  words.value === 0 ? "var(--c-faint)" : words.value >= 50 ? "var(--green)" : "var(--red)",
);

// Feed HTML'ini temizle: script ve inline event handler'ları at.
const safeHtml = computed(() =>
  (props.detailsHtml || "")
    .replace(/<script[\s\S]*?<\/script>/gi, "")
    .replace(/\son\w+\s*=\s*"[^"]*"/gi, "")
    .replace(/\son\w+\s*=\s*'[^']*'/gi, ""),
);

const copied = ref(false);
async function copyHtml() {
  try {
    await writeText(props.detailsHtml || "");
    copied.value = true;
    setTimeout(() => (copied.value = false), 1600);
  } catch {
    store.toast("Panoya kopyalanamadı", "error");
  }
}

function genDetails() {
  store.generateDetails();
}
</script>

<template>
  <SeoCard
    icon="fileText"
    title="Açıklama SEO"
    sub="Ürün detay sayfası uzun içeriği"
    :badge-label="BADGE_LABEL[badge]"
    :badge-style="badgeStyle"
    :model="model"
  >
    <div class="info-row">
      <span class="info">
        <Icon name="code" :size="13" />
        HTML yapısı korunur, yalnızca metin içeriği yenilenir.
      </span>
      <button class="copy" :class="{ ok: copied }" @click="copyHtml">
        <Icon name="copy" :size="12" />
        {{ copied ? "Kopyalandı" : "HTML kopyala" }}
      </button>
    </div>

    <div class="preview om-scroll">
      <div v-if="hasContent" class="seo-html-preview" v-html="safeHtml"></div>
      <div v-else class="preview-empty">
        <Icon name="clipboardList" :size="26" :stroke-width="1.6" />
        <span>Henüz açıklama içeriği yok — <b>Açıklamayı Üret</b> ile oluşturun.</span>
      </div>
      <div v-if="store.generatingDetails" class="gen-overlay">
        <Icon name="loader" :size="22" :stroke-width="2.4" class="spin" style="color:var(--accent)" />
        <span>Uzun içerik üretiliyor…</span>
      </div>
    </div>

    <div class="metrics">
      <div class="metric">
        <div class="metric-head">
          <span>Kelime sayısı</span>
          <span class="metric-min">min 50</span>
        </div>
        <div class="metric-val">
          <span class="big" :style="{ color: wordColor }">{{ words }}</span>
          <span class="unit">kelime</span>
        </div>
        <div class="track">
          <div class="fill" :style="{ width: wordBar, background: wordColor }"></div>
        </div>
      </div>
      <div class="metric">
        <div class="metric-head">
          <span>Hedef kelime yoğunluğu</span>
          <span class="metric-min">hedef %2–3</span>
        </div>
        <div class="metric-val">
          <span class="big" :style="{ color: densColor }">{{ densText }}</span>
          <span class="unit">{{ densNote }}</span>
        </div>
        <div class="dens-track">
          <div class="dens-zone"></div>
          <div class="dens-mark" :style="{ background: densColor, left: densMark }"></div>
        </div>
      </div>
    </div>

    <div class="empty-check" :style="{ color: hasContent ? 'var(--c-mid)' : 'var(--red)' }">
      <Icon :name="hasContent ? 'check' : 'x'" :size="13" :stroke-width="2.6" :style="{ color: hasContent ? 'var(--green)' : 'var(--red)' }" />
      İçerik boş değil
    </div>

    <div v-if="history.length" class="hist-line">
      <a class="link" @click="histOpen = !histOpen">önceki sürümler ({{ history.length }})</a>
    </div>
    <VersionHistory
      v-if="histOpen && history.length"
      :items="history.map((v) => ({ at: v.at, summary: `${v.words} kelime`, model: v.model }))"
      noun="açıklama"
      @restore="store.restoreDetailsVersion($event)"
    />

    <template #actions>
      <!-- data-tip butonun kendisinde değil sarmalayıcıda: disabled buton fare olayı üretmez -->
      <div class="gen-wrap" :data-tip="genTip">
        <button
          class="gen"
          :class="{ busy: store.generatingDetails }"
          :disabled="store.generatingDetails || !imageGateOk"
          @click="genDetails"
        >
          <Icon :name="store.generatingDetails ? 'loader' : 'sparkles'" :size="15" :stroke-width="store.generatingDetails ? 2.2 : 1.9" :class="{ spin: store.generatingDetails }" />
          {{ store.generatingDetails ? "Üretiliyor…" : "Açıklamayı Üret" }}
        </button>
      </div>
      <button
        v-if="store.settings?.ideasoft_active"
        class="is-push"
        :disabled="store.ideasoftBusy"
        title="Bu kartın içeriğini IdeaSoft'a gönder"
        @click="store.openIdeasoftPreview(['details'])"
      >
        <Icon name="upload" :size="14" />
        IdeaSoft'a Gönder
      </button>
      <div style="flex:1"></div>
      <button class="done" :class="{ active: detailsDone }" @click="store.toggleDetailsDone()">
        <Icon name="badgeCheck" :size="15" :stroke-width="2.2" />
        {{ detailsDone ? "Açıklama tamamlandı ✓" : "Açıklamayı Tamamlandı işaretle" }}
      </button>
    </template>
  </SeoCard>
</template>

<style scoped>
.hist-line {
  margin-top: 10px;
  font-size: 11.5px;
}
.link {
  color: var(--accent);
  cursor: pointer;
  font-weight: 560;
}

.info-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 9px;
}
.info {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  font-size: 11.5px;
  color: var(--c-soft);
}
.copy {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 3px 9px;
  border: 1px solid var(--c-border);
  border-radius: 7px;
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 11px;
  font-weight: 560;
  cursor: pointer;
}
.copy:hover {
  background: var(--c-hover);
}
.copy.ok {
  color: var(--green);
}
.preview {
  border: 1px solid var(--c-border);
  border-radius: 11px;
  background: var(--c-input);
  min-height: 190px;
  max-height: 320px;
  overflow-y: auto;
  padding: 16px 18px;
  color: var(--c-text);
}
.preview-empty {
  height: 158px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--c-faint);
  gap: 8px;
  font-size: 12.5px;
  text-align: center;
}
.preview-empty b {
  color: var(--c-soft);
}
.metrics {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  margin-top: 14px;
}
.metric {
  padding: 11px 13px;
  border: 1px solid var(--c-border-soft);
  border-radius: 10px;
  background: var(--c-input);
}
.metric-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  font-size: 11.5px;
  color: var(--c-soft);
}
.metric-min {
  font-size: 11px;
  color: var(--c-faint);
}
.metric-val {
  display: flex;
  align-items: baseline;
  gap: 6px;
  margin: 5px 0 8px;
}
.big {
  font-size: 20px;
  font-weight: 680;
}
.unit {
  font-size: 11.5px;
  color: var(--c-faint);
}
.track {
  height: 5px;
  border-radius: 4px;
  background: var(--c-track);
  overflow: hidden;
}
.fill {
  height: 100%;
  border-radius: 4px;
  transition: width 0.25s ease;
}
.dens-track {
  position: relative;
  height: 5px;
  border-radius: 4px;
  background: var(--c-track);
  overflow: hidden;
}
.dens-zone {
  position: absolute;
  left: 40%;
  width: 20%;
  top: 0;
  bottom: 0;
  background: var(--dens-zone);
}
.dens-mark {
  position: absolute;
  top: -2px;
  bottom: -2px;
  width: 2px;
  border-radius: 2px;
}
.empty-check {
  display: flex;
  align-items: center;
  gap: 7px;
  margin-top: 11px;
  font-size: 11.5px;
}
/* Yalnızca tooltip çıpası (data-tip) — konumlandırma [data-tip]'ten gelir. */
.gen-wrap {
  display: inline-flex;
}
/* MetaSeoCard'daki birincil buton ile birebir aynı ölçüler. */
.gen {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 38px;
  padding: 0 16px;
  border: none;
  border-radius: 9px;
  background: var(--accent);
  color: #fff;
  font-size: 13px;
  font-weight: 590;
  cursor: pointer;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
}
.gen:hover {
  filter: brightness(1.05);
}
.gen.busy {
  opacity: 0.75;
  cursor: default;
}
.gen:disabled:not(.busy) {
  opacity: 0.45;
  cursor: not-allowed;
}
.is-push {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  height: 38px;
  padding: 0 14px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 12.5px;
  font-weight: 560;
  cursor: pointer;
}
.is-push:hover {
  border-color: var(--accent);
  color: var(--accent);
}
.is-push:disabled {
  opacity: 0.5;
  cursor: default;
}
.spin {
  animation: spin 0.8s linear infinite;
}
.preview {
  position: relative;
}
.gen-overlay {
  position: absolute;
  inset: 0;
  background: var(--overlay-bg);
  backdrop-filter: blur(2px);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  border-radius: 11px;
  font-size: 12.5px;
  font-weight: 560;
  color: var(--c-mid);
}
.done {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  height: 38px;
  padding: 0 14px;
  border: 1px solid var(--badge-uygun-bg);
  border-radius: 9px;
  background: var(--badge-uygun-bg);
  color: var(--green);
  font-size: 12.5px;
  font-weight: 580;
  cursor: pointer;
}
.done:hover {
  filter: brightness(0.98);
}
</style>
