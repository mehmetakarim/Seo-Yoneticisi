<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useStore } from "../store";
import { groupedNav } from "../navigation";
import { teamAccessIssue, teamCheckedAt, teamReachable, teamRestoring, teamServiceStatus, teamUser, teamWorkspace } from "../teamConnection";
import Icon from "./Icon.vue";

/**
 * Menü 3 öğeden 9'a çıktı (her SEO aracı kendi ekranında). Düz liste hâlinde bu bir
 * "araç kutusu" değil "uzun liste" gibi görünüyordu; grup başlıkları hiyerarşi kuruyor.
 * Kayıt `navigation.ts`'te — buraya yeni öğe eklenmez.
 */
const groups = groupedNav();

const store = useStore();
const openGroup = ref<(typeof groups)[number]["group"] | null>(null);

watch(
  () => store.page,
  (page) => {
    openGroup.value = groups.find(({ items }) => items.some((item) => item.key === page))?.group ?? null;
  },
  { immediate: true },
);

function fmtSync(): string {
  if (!store.lastSync) return "henüz senkron yok";
  return store.lastSync.run_at.replace("T", " ").slice(0, 16);
}

const connection = computed(() => {
  if (!teamUser.value) return teamRestoring.value
    ? { tone: "checking", label: "Ekip oturumu kontrol ediliyor", detail: "" }
    : { tone: "local", label: "Yerel kayıtlar", detail: `Son katalog senkronu · ${fmtSync()}` };
  if (!teamWorkspace.value) return { tone: "checking", label: "Çalışma alanı bekleniyor", detail: "" };
  const detail = teamCheckedAt.value
    ? `Son kontrol · ${teamCheckedAt.value.toLocaleTimeString("tr-TR")}`
    : "";
  if (teamAccessIssue.value) return { tone: "offline", label: "Supabase erişim hatası", detail };
  if (teamServiceStatus.value !== null) return {
    tone: "offline", label: `Supabase servis hatası (${teamServiceStatus.value})`, detail: "Yerel kayıtlar kullanılıyor",
  };
  if (teamReachable.value === false) return { tone: "offline", label: "Çevrimdışı · Yerel kayıtlar", detail };
  if (teamReachable.value === true) return { tone: "online", label: "Çevrimiçi · Supabase", detail };
  return { tone: "checking", label: "Supabase kontrol ediliyor", detail };
});
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

    <nav class="nav om-scroll" aria-label="Ana menü">
      <div v-for="(g, index) in groups" :key="g.group ?? ''" class="nav-group">
        <button
          v-if="g.group"
          type="button"
          class="nav-cap"
          :aria-expanded="openGroup === g.group"
          :aria-controls="'nav-group-' + index"
          @click="openGroup = openGroup === g.group ? null : g.group"
        >
          {{ g.group }}
          <span class="nav-chevron" aria-hidden="true"></span>
        </button>
        <div v-show="!g.group || openGroup === g.group" :id="'nav-group-' + index">
          <button
            v-for="item in g.items"
            :key="item.key"
            type="button"
            class="nav-item"
            :class="{ active: store.page === item.key }"
            :aria-current="store.page === item.key ? 'page' : undefined"
            :data-nav="item.key"
            @click="store.page = item.key"
          >
            <Icon :name="item.icon" :size="16" :stroke-width="1.9" />
            <span>{{ item.label }}</span>
          </button>
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

    <div class="sync-status" :data-tone="connection.tone">
      <span class="dot" aria-hidden="true"></span>
      <div>
        <div class="s1" role="status">{{ connection.label }}</div>
        <div v-if="connection.detail" class="s2">{{ connection.detail }}</div>
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
  margin-top: 6px;
}
/* Grup başlığı: küçük, soluk, harf aralığı açık — okunmak için değil, ayırmak için. */
.nav-cap {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 9px 10px;
  font-size: 10.5px;
  font-weight: 640;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--c-mid);
}
.nav-cap, .nav-item {
  width: 100%;
  border: 0;
  border-radius: 8px;
  background: transparent;
  text-align: left;
  cursor: pointer;
}
.nav-cap:hover {
  background: var(--c-hover);
}
.nav-chevron {
  width: 6px;
  height: 6px;
  border-right: 1.5px solid currentColor;
  border-bottom: 1.5px solid currentColor;
  transform: rotate(-45deg);
}
.nav-cap[aria-expanded="true"] .nav-chevron {
  transform: rotate(45deg);
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
  flex: none;
  border-radius: 50%;
  background: var(--c-faint);
}
.sync-status[data-tone="online"] .dot {
  background: #2fa84f;
  box-shadow: 0 0 0 3px rgba(47, 168, 79, 0.15);
}
.sync-status[data-tone="offline"] .dot {
  background: var(--warn-text);
}
.sync-status[data-tone="checking"] .dot {
  background: var(--accent);
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
