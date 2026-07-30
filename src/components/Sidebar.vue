<script setup lang="ts">
import { useStore } from "../store";
import { groupedNav } from "../navigation";
import Icon from "./Icon.vue";

/**
 * Menü 3 öğeden 9'a çıktı (her SEO aracı kendi ekranında). Düz liste hâlinde bu bir
 * "araç kutusu" değil "uzun liste" gibi görünüyordu; grup başlıkları hiyerarşi kuruyor.
 * Kayıt `navigation.ts`'te — buraya yeni öğe eklenmez.
 */
const groups = groupedNav();

const store = useStore();

function fmtSync(): string {
  if (!store.lastSync) return "henüz senkron yok";
  return store.lastSync.run_at.replace("T", " ").slice(0, 16);
}
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <div class="brand-logo">
        <Icon name="search" :size="17" :stroke-width="2.2" style="color:#fff" />
      </div>
      <div>
        <div class="brand-title">SEO Yöneticisi</div>
        <div class="brand-sub">Katalog Feed</div>
      </div>
    </div>

    <nav class="nav om-scroll">
      <div v-for="g in groups" :key="g.group" class="nav-group">
        <div class="nav-cap">{{ g.group }}</div>
        <div
          v-for="item in g.items"
          :key="item.key"
          class="nav-item"
          :class="{ active: store.page === item.key }"
          @click="store.page = item.key"
        >
          <Icon :name="item.icon" :size="16" :stroke-width="1.9" />
          <span>{{ item.label }}</span>
        </div>
      </div>
    </nav>

    <div class="theme-switch">
      <button
        :class="{ on: store.theme === 'light' }"
        @click="store.setTheme('light')"
      >
        <Icon name="sun" :size="14" /> Açık
      </button>
      <button
        :class="{ on: store.theme === 'dark' }"
        @click="store.setTheme('dark')"
      >
        <Icon name="moon" :size="14" /> Koyu
      </button>
    </div>

    <div class="sync-status">
      <span class="dot"></span>
      <div>
        <div class="s1">Senkron aktif</div>
        <div class="s2">Son: {{ fmtSync() }}</div>
      </div>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: clamp(190px, 16vw, 238px);
  flex: none;
  background: var(--c-panel);
  border-right: 1px solid var(--c-border);
  display: flex;
  flex-direction: column;
  padding: 20px 14px;
}
.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 4px 8px 22px;
}
.brand-logo {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  background: var(--accent);
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
}
.brand-title {
  font-size: 14px;
  font-weight: 600;
  letter-spacing: -0.01em;
  color: var(--c-text);
}
.brand-sub {
  font-size: 11px;
  color: var(--c-soft);
  margin-top: 1px;
}
.nav {
  display: flex;
  flex-direction: column;
  /* 9 öğe kısa pencerede taşabilir; tema anahtarı ve senkron durumu hep görünür kalsın. */
  overflow-y: auto;
  min-height: 0;
}
.nav-group + .nav-group {
  margin-top: 16px;
}
/* Grup başlığı: küçük, soluk, harf aralığı açık — okunmak için değil, ayırmak için. */
.nav-cap {
  padding: 0 10px 6px;
  font-size: 10.5px;
  font-weight: 640;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--c-faint);
}
.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 10px;
  border-radius: 8px;
  color: var(--c-mid);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
}
.nav-item + .nav-item {
  margin-top: 2px;
}
.nav-item:hover {
  background: var(--c-hover);
}
.nav-item.active {
  background: var(--accent-tint);
  color: var(--accent);
  font-weight: 590;
}
.nav-item.active:hover {
  background: var(--accent-tint);
}
.theme-switch {
  margin-top: 14px;
  display: flex;
  padding: 3px;
  background: var(--c-chip);
  border-radius: 9px;
  gap: 2px;
}
.theme-switch button {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  height: 30px;
  border: none;
  border-radius: 7px;
  background: transparent;
  color: var(--c-soft);
  font-size: 12px;
  font-weight: 560;
  cursor: pointer;
}
.theme-switch button.on {
  background: var(--c-card);
  color: var(--accent);
}
:root[data-theme="dark"] .theme-switch button.on {
  background: #3a3a3c;
  color: #fff;
}
.sync-status {
  margin-top: auto;
  padding: 12px 10px 4px;
  border-top: 1px solid var(--c-border);
  display: flex;
  align-items: center;
  gap: 9px;
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #2fa84f;
  box-shadow: 0 0 0 3px rgba(47, 168, 79, 0.15);
}
.s1 {
  font-size: 12px;
  font-weight: 560;
  color: var(--c-mid);
}
.s2 {
  font-size: 11px;
  color: var(--c-soft);
  margin-top: 1px;
}
</style>
