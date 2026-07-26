<script setup lang="ts">
import { computed } from "vue";
import { useStore } from "../store";
import Icon from "./Icon.vue";

const store = useStore();
const info = computed(() => store.updateInfo);

/** İndirme yüzdesi (toplam boyut bilinmiyorsa belirsiz çubuk). */
const pct = computed(() => {
  const t = store.updateTotal;
  return t > 0 ? Math.min(100, Math.round((store.updateDownloaded / t) * 100)) : 0;
});
const mb = (n: number) => (n / 1024 / 1024).toFixed(1);
</script>

<template>
  <Transition name="upd">
    <div v-if="info" class="overlay" @click.self="!store.updating && store.dismissUpdate()">
      <div class="modal" role="dialog" aria-label="Güncelleme">
        <header class="head">
          <div class="h-left">
            <div class="icon-badge"><Icon name="download" :size="15" /></div>
            <div>
              <div class="h-title">Yeni sürüm hazır</div>
              <div class="h-sub">
                v{{ store.appVersion }} → <b>v{{ info.version }}</b>
              </div>
            </div>
          </div>
          <button
            v-if="!store.updating"
            class="close"
            title="Kapat"
            @click="store.dismissUpdate()"
          >
            <Icon name="x" :size="16" :stroke-width="2.2" />
          </button>
        </header>

        <div class="body">
          <div v-if="info.notes" class="notes om-scroll">{{ info.notes }}</div>
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
        </div>

        <footer class="foot">
          <button class="ghost" :disabled="store.updating" @click="store.dismissUpdate()">
            Sonra
          </button>
          <div style="flex:1"></div>
          <button class="gen" :class="{ busy: store.updating }" :disabled="store.updating" @click="store.runUpdate()">
            <Icon :name="store.updating ? 'loader' : 'download'" :size="15" :class="{ spin: store.updating }" />
            {{ store.updating ? "Güncelleniyor…" : "Şimdi güncelle" }}
          </button>
        </footer>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: var(--overlay-bg);
  backdrop-filter: saturate(1.1) blur(3px);
}
.modal {
  width: 460px;
  max-width: 100%;
  background: var(--c-card);
  border: 1px solid var(--c-border);
  border-radius: 16px;
  box-shadow: 0 24px 60px var(--heavy-shadow);
  display: flex;
  flex-direction: column;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 15px 18px;
  border-bottom: 1px solid var(--c-border-soft);
}
.h-left {
  display: flex;
  align-items: center;
  gap: 11px;
}
.icon-badge {
  width: 30px;
  height: 30px;
  border-radius: 9px;
  background: var(--accent-tint);
  color: var(--accent);
  display: flex;
  align-items: center;
  justify-content: center;
}
.h-title {
  font-size: 14.5px;
  font-weight: 650;
  color: var(--c-text);
  letter-spacing: -0.01em;
}
.h-sub {
  font-size: 11.5px;
  color: var(--c-soft);
  margin-top: 1px;
}
.h-sub b {
  color: var(--accent);
}
.close {
  width: 30px;
  height: 30px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--c-soft);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}
.close:hover {
  background: var(--c-hover);
  color: var(--c-text);
}
.body {
  padding: 16px 18px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.notes {
  max-height: 200px;
  overflow-y: auto;
  font-size: 12.5px;
  line-height: 1.6;
  color: var(--c-text);
  background: var(--c-list);
  border: 1px solid var(--c-border-soft);
  border-radius: 9px;
  padding: 10px 12px;
  white-space: pre-wrap;
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
.warn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  font-size: 11.5px;
  color: var(--warn-text);
  background: var(--warn-bg);
  border: 1px solid var(--warn-border);
  border-radius: 8px;
}
.foot {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 18px;
  border-top: 1px solid var(--c-border-soft);
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
.spin {
  animation: spin 0.8s linear infinite;
}

/* Diğer modallarla aynı animasyon dili */
.upd-enter-active,
.upd-leave-active {
  transition: opacity 0.24s ease;
}
.upd-enter-from,
.upd-leave-to {
  opacity: 0;
}
.upd-enter-active .modal,
.upd-leave-active .modal {
  transition: transform 0.26s cubic-bezier(0.32, 0.72, 0, 1);
}
.upd-enter-from .modal,
.upd-leave-to .modal {
  transform: scale(0.96) translateY(8px);
}
@media (prefers-reduced-motion: reduce) {
  .upd-enter-active .modal,
  .upd-leave-active .modal {
    transition: none;
  }
  .upd-enter-from .modal,
  .upd-leave-to .modal {
    transform: none;
  }
  .fill.indet {
    animation: none;
  }
}
</style>
