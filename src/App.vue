<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, type Component } from "vue";
import { useStore } from "./store";
import { titleOf, type Page } from "./navigation";
import Sidebar from "./components/Sidebar.vue";
import ProductsPage from "./components/ProductsPage.vue";
import SettingsPage from "./components/SettingsPage.vue";
import UpdateModal from "./components/UpdateModal.vue";
import Icon from "./components/Icon.vue";

const store = useStore();
/**
 * O an gösterilen sayfanın örneği. Kabuk sayfanın ne olduğunu bilmez; yalnızca
 * `focusSearch` gibi bir yeteneği varsa kullanır (ör. ⌘F). Yeni sayfalar da
 * isterlerse aynı adı dışa açarak kısayoldan yararlanabilir.
 */
const pageRef = ref<{ focusSearch?: () => void } | null>(null);

/** navigation.ts'teki kayda karşılık gelen bileşenler. Yeni sayfa → buraya bir satır. */
const PAGES: Record<Page, Component> = {
  products: ProductsPage,
  settings: SettingsPage,
};

const isProducts = computed(() => store.page === "products");
const pageTitle = computed(() => titleOf(store.page));
const pageSub = computed(() => {
  if (!isProducts.value) return "Kaynaklar ve yedekleme";
  const done = store.counts.tamamlandi;
  const active = store.counts.tumu;
  return `${active} ürün bekliyor · ${done} tamamlandı`;
});

// İş takibi: tamamlanan ürün oranı (meta + açıklama + teknik tablo hepsi işaretli).
const donePct = computed(() => {
  const total = store.allRows.length;
  return total ? Math.round((store.counts.tamamlandi / total) * 100) : 0;
});

function fmtSync(): string {
  if (!store.lastSync) return "henüz senkron yok";
  return store.lastSync.run_at.replace("T", " ").slice(0, 16);
}

function onKey(e: KeyboardEvent) {
  const tag = (e.target as HTMLElement)?.tagName?.toLowerCase();
  const typing = tag === "input" || tag === "textarea";

  // ⌘/Ctrl + F → arama
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "f") {
    if (store.page === "products") {
      e.preventDefault();
      pageRef.value?.focusSearch?.();
    }
    return;
  }
  if (store.page !== "products") return;

  if (e.key === "ArrowDown") {
    e.preventDefault();
    store.navigate(1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    store.navigate(-1);
  } else if (!typing && (e.key === "g" || e.key === "G")) {
    e.preventDefault();
    if (!store.selectedSku) return;
    // ⇧G = açıklama üret, G = meta üret
    if (e.shiftKey) store.generateDetails();
    else store.generateMeta();
  } else if (!typing && (e.key === "d" || e.key === "D")) {
    e.preventDefault();
    if (store.selectedSku) store.toggleMetaDone();
  }
}

onMounted(() => {
  store.init();
  window.addEventListener("keydown", onKey);
});
onUnmounted(() => window.removeEventListener("keydown", onKey));
</script>

<template>
  <div class="app">
    <Sidebar />

    <main class="main">
      <header class="topbar">
        <div>
          <div class="pg-title">{{ pageTitle }}</div>
          <div class="pg-sub">{{ pageSub }}</div>
        </div>
        <div v-if="isProducts" class="topbar-right">
          <div class="sync-info">
            <span class="last">Son güncelleme: {{ fmtSync() }}</span>
            <div v-if="store.allRows.length" class="progress" :title="`${store.counts.tamamlandi} ürün tamamlandı`">
              <div class="pbar">
                <div class="pfill" :style="{ width: donePct + '%' }"></div>
              </div>
              <span class="ptext">{{ store.counts.tamamlandi }}/{{ store.allRows.length }} tamamlandı</span>
            </div>
          </div>
          <button class="sync" :disabled="store.syncing" @click="store.sync()">
            <Icon name="refresh" :size="16" :class="{ spin: store.syncing }" />
            <span>{{ store.syncing ? "Güncelleniyor…" : "Manuel Güncelle" }}</span>
          </button>
        </div>
      </header>

      <UpdateModal />

      <component :is="PAGES[store.page]" ref="pageRef" />
    </main>

    <div class="toasts">
      <div v-for="t in store.toasts" :key="t.id" class="toast" :class="t.kind">
        {{ t.text }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.app {
  display: flex;
  height: 100vh;
  width: 100vw;
  min-width: 1200px;
  background: var(--c-bg);
  color: var(--c-text);
  overflow: hidden;
}
.main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.topbar {
  flex: none;
  height: 64px;
  border-bottom: 1px solid var(--c-border);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 22px;
}
.pg-title {
  font-size: 18px;
  font-weight: 640;
  letter-spacing: -0.02em;
  color: var(--c-text);
}
.pg-sub {
  font-size: 12.5px;
  color: var(--c-soft);
  margin-top: 1px;
}
.topbar-right {
  display: flex;
  align-items: center;
  gap: 16px;
}
.sync-info {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 4px;
}
.last {
  font-size: 12.5px;
  color: var(--c-soft);
}
.progress {
  display: flex;
  align-items: center;
  gap: 8px;
}
.pbar {
  width: 140px;
  height: 5px;
  border-radius: 999px;
  background: var(--c-track);
  overflow: hidden;
}
.pfill {
  height: 100%;
  background: var(--green);
  border-radius: 999px;
  transition: width 0.35s cubic-bezier(0.32, 0.72, 0, 1);
}
.ptext {
  font-size: 11px;
  color: var(--c-faint);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}
.sync {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 38px;
  padding: 0 18px;
  border: none;
  border-radius: 9px;
  background: var(--accent);
  color: #fff;
  font-size: 13.5px;
  font-weight: 590;
  cursor: pointer;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
}
.sync:hover {
  filter: brightness(1.06);
}
.sync:active {
  transform: scale(0.98);
}
.sync:disabled {
  cursor: default;
  opacity: 0.8;
}
.spin {
  animation: spin 0.8s linear infinite;
}
.toasts {
  position: fixed;
  bottom: 20px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  flex-direction: column;
  gap: 8px;
  z-index: 100;
  align-items: center;
}
.toast {
  padding: 10px 16px;
  border-radius: 10px;
  font-size: 12.5px;
  font-weight: 560;
  color: #fff;
  background: #333;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
  animation: popIn 0.2s ease;
}
.toast.error {
  background: #c0392b;
}
.toast.ok {
  background: #1a8a4a;
}
.toast.info {
  background: var(--accent);
}
</style>
