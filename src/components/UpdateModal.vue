<script setup lang="ts">
import { computed } from "vue";
import { useStore } from "../store";
import Icon from "./Icon.vue";
import ModalShell from "./ModalShell.vue";

const store = useStore();
const info = computed(() => store.updateInfo);

/** İndirme yüzdesi (toplam boyut bilinmiyorsa belirsiz çubuk). */
const pct = computed(() => {
  const t = store.updateTotal;
  return t > 0 ? Math.min(100, Math.round((store.updateDownloaded / t) * 100)) : 0;
});
const mb = (n: number) => (n / 1024 / 1024).toFixed(1);

/**
 * Sürüm notlarını satırlara ayırır: `-` / `*` / `•` ile başlayanlar madde, diğerleri paragraf.
 *
 * ⚠️ Bilinçli olarak `v-html` KULLANILMIYOR. Bu metin uzak sunucudan (release JSON) geliyor;
 * HTML olarak basmak, sürüm notu yazabilen herkese uygulama içinde kod çalıştırma imkânı verirdi.
 * Biçimlendirme yalnızca CSS ile yapılıyor, enjeksiyon yüzeyi yok.
 */
type NoteLine = { text: string; bullet: boolean };
const noteLines = computed<NoteLine[]>(() => {
  const out: NoteLine[] = [];
  for (const raw of (info.value?.notes ?? "").split(/\r?\n/)) {
    if (!raw.trim()) continue;
    const m = /^\s*[-*•]\s+(.*)$/.exec(raw);
    if (m) {
      out.push({ text: m[1].trim(), bullet: true });
    } else if (/^\s/.test(raw) && out.length) {
      // Girintili satır = önceki maddenin devamı (CHANGELOG'da uzun madde sarmışsa).
      // Ayrı paragraf olarak basılsa madde ortadan kopmuş görünürdü.
      out[out.length - 1].text += " " + raw.trim();
    } else {
      out.push({ text: raw.trim(), bullet: false });
    }
  }
  return out;
});
</script>

<template>
  <!-- `closable`: indirme sürerken kapanmamalı — kilit iskelette değil BURADA, çünkü
       "ne zaman kapanamaz" her modalde farklı bir koşul. -->
  <ModalShell
    :open="!!info"
    label="Güncelleme"
    icon="download"
    title="Yeni sürüm hazır"
    :closable="!store.updating"
    :width="460"
    @close="store.dismissUpdate()"
  >
    <template #sub>v{{ store.appVersion }} → <b>v{{ info?.version }}</b></template>

    <div v-if="noteLines.length" class="notes om-scroll">
      <p v-for="(l, i) in noteLines" :key="i" :class="{ bullet: l.bullet }">{{ l.text }}</p>
    </div>
    <div v-else class="notes muted">Bu sürüm için not girilmemiş.</div>

    <!-- İndirme durumu -->
    <div v-if="store.updating" class="dl">
      <div class="bar">
        <div class="fill" :class="{ indet: !store.updateTotal }" :style="store.updateTotal ? { width: pct + '%' } : {}"></div>
      </div>
      <span class="dl-text">
        {{ store.updateTotal
          ? `İndiriliyor… %${pct} (${mb(store.updateDownloaded)} / ${mb(store.updateTotal)} MB)`
          : "İndiriliyor…" }}
      </span>
    </div>

    <div class="warn">
      <Icon name="info" :size="13" />
      Güncelleme kurulduktan sonra uygulama yeniden başlatılır. Verileriniz korunur.
    </div>

    <template #footer>
      <button class="ghost" :disabled="store.updating" @click="store.dismissUpdate()">
        Sonra
      </button>
      <div style="flex:1"></div>
      <button class="gen" :class="{ busy: store.updating }" :disabled="store.updating" @click="store.runUpdate()">
        <Icon :name="store.updating ? 'loader' : 'download'" :size="15" :class="{ spin: store.updating }" />
        {{ store.updating ? "Güncelleniyor…" : "Şimdi güncelle" }}
      </button>
    </template>
  </ModalShell>
</template>

<style scoped>
.notes {
  max-height: 200px;
  overflow-y: auto;
  font-size: 12.5px;
  line-height: 1.6;
  color: var(--c-text);
  background: var(--c-list);
  border: 1px solid var(--c-border-soft);
  border-radius: 9px;
  padding: 11px 13px;
}
.notes p {
  margin: 0 0 5px;
}
.notes p:last-child {
  margin-bottom: 0;
}
/* Madde işareti ::before ile — metin içeriğinden gelmiyor, dolayısıyla
   uzaktan gelen notun görünümü etkilemesi mümkün değil. */
.notes p.bullet {
  padding-left: 15px;
  position: relative;
}
.notes p.bullet::before {
  content: "";
  position: absolute;
  left: 3px;
  top: 8px;
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: var(--c-soft);
}
.notes.muted {
  color: var(--c-faint);
}
.dl {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.bar {
  height: 6px;
  border-radius: 999px;
  background: var(--c-track);
  overflow: hidden;
}
.fill {
  height: 100%;
  background: var(--accent);
  border-radius: 999px;
  transition: width 0.25s ease;
}
.fill.indet {
  width: 40%;
  animation: indet 1.1s ease-in-out infinite;
}
@keyframes indet {
  0% { margin-left: -40%; }
  100% { margin-left: 100%; }
}
.dl-text {
  font-size: 11.5px;
  color: var(--c-soft);
  font-variant-numeric: tabular-nums;
}
.ghost {
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
.ghost:hover:not(:disabled) {
  background: var(--c-hover);
}
.ghost:disabled {
  opacity: 0.5;
  cursor: default;
}
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
/* Modal iskeleti ve animasyonu artık ModalShell.vue'da — burada yalnızca bu modale
   özgü içerik stilleri kalıyor. */
</style>
