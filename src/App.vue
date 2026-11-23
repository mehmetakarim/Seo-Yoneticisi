<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch, type Component } from "vue";
import { useStore } from "./store";
import { markTeamDisconnected, probeTeamConnection, resetTeamConnectionStatus, resolveTeamWorkspace, restoreTeamSession, teamAccessIssue, teamClient, teamConnectionRetry, teamProject, teamReachable, teamRestoring, teamSession, teamUser, teamWorkspace } from "./teamConnection";
import { contactSync, flushTeamContacts, teamContacts } from "./teamContacts";
import { flushTeamQuotes, resetTeamQuoteIndicators, teamQuoteQueue, teamQuotes } from "./teamQuotes";
import { refreshTeamCatalog } from "./teamCatalog";
import { flushTeamProductSeo, teamSeoSync } from "./teamProductSeo";
import { flushTeamGsc, flushTeamGscSnapshots, teamGscMode, teamMeasurementScope, teamOutcomeInputs } from "./teamGsc";
import { flushGscReports } from "./teamGscReport";
import { flushTeamIdeasoftReceipts } from "./teamIdeasoft";
import { flushTeamStorePages } from "./teamStorePages";
import { flushTeamToday, teamTodayMode, teamTodayScope, teamTodaySync } from "./teamToday";
import { KIND_BY_PAGE } from "./catalog";
import { NAV, subOf, titleOf, TOOL_PAGES, type Page } from "./navigation";
import Sidebar from "./components/Sidebar.vue";
import ProductsPage from "./components/ProductsPage.vue";
import ContactsPage from "./components/contacts/ContactsPage.vue";
// Faz İ2: kategori · marka · içerik ekranlarının ÜÇÜ de bu bileşen. Kayıt üç satır,
// bileşen bir tane — üç kopya ekran bu projede beş kez sapmış bir desendi.
import CatalogPage from "./components/catalog/CatalogPage.vue";
import QuotesPage from "./components/quotes/QuotesPage.vue";
import SettingsPage from "./components/SettingsPage.vue";
// SEO araçları: her analiz kendi ekranında (v0.7.0). Hepsi TEK `opportunity_json`
// önbelleğinin dilimlerini okur — ekran başına GSC çağrısı yok.
import TodayPage from "./components/tools/TodayPage.vue";
import OverviewPage from "./components/tools/OverviewPage.vue";
import OpportunitiesPage from "./components/tools/OpportunitiesPage.vue";
import StrikingPage from "./components/tools/StrikingPage.vue";
import CannibalPage from "./components/tools/CannibalPage.vue";
import DecayPage from "./components/tools/DecayPage.vue";
import EolPage from "./components/tools/EolPage.vue";
import ContentGapPage from "./components/tools/ContentGapPage.vue";
import AssistantPage from "./components/tools/AssistantPage.vue";
import FocusBar from "./components/FocusBar.vue";
import FocusSummaryModal from "./components/FocusSummaryModal.vue";
import UpdateModal from "./components/UpdateModal.vue";
import SetupWizard from "./components/SetupWizard.vue";
import StartupCheck from "./components/StartupCheck.vue";
import TeamSignIn from "./components/TeamSignIn.vue";
import TeamChatPanel from "./components/TeamChatPanel.vue";
import TeamPage from "./components/TeamPage.vue";
import Icon from "./components/Icon.vue";

const store = useStore();
const startupReady = ref(false);
const workspaceOpen = computed(() => startupReady.value && !!teamUser.value && !!teamWorkspace.value && !teamAccessIssue.value);
watch(workspaceOpen, (open) => { if (open) void store.init(); });
const teamPanelOpen = ref(false);
const teamPanelPinned = ref(false);
const teamPanelButton = ref<HTMLButtonElement | null>(null);

function setTeamPanelOpen(open: boolean) {
  teamPanelOpen.value = open;
  if (!open) teamPanelButton.value?.focus();
}
/**
 * O an gösterilen sayfanın örneği. Kabuk sayfanın ne olduğunu bilmez; yalnızca
 * `focusSearch` gibi bir yeteneği varsa kullanır (ör. ⌘F). Yeni sayfalar da
 * isterlerse aynı adı dışa açarak kısayoldan yararlanabilir.
 */
const pageRef = ref<{ focusSearch?: () => void } | null>(null);
const pageSlidesBack = ref(false);

function removePreviousPage(_page: Element, done: () => void) {
  done();
}

/** navigation.ts'teki kayda karşılık gelen bileşenler. Yeni sayfa → buraya bir satır. */
const PAGES: Record<Page, Component> = {
  today: TodayPage,
  products: ProductsPage,
  categories: CatalogPage,
  brands: CatalogPage,
  blogs: CatalogPage,
  contacts: ContactsPage,
  quotes: QuotesPage,
  overview: OverviewPage,
  opportunities: OpportunitiesPage,
  striking: StrikingPage,
  cannibal: CannibalPage,
  decay: DecayPage,
  eol: EolPage,
  contentgap: ContentGapPage,
  assistant: AssistantPage,
  settings: SettingsPage,
  team: TeamPage,
};

const isProducts = computed(() => store.page === "products");
const pageTitle = computed(() => titleOf(store.page));
/**
 * Üst şerit alt metni. Sabit olanlar `navigation.ts`'te (kayıt tek yerde kalsın);
 * yalnızca ürün sayacı gibi DURUMA bağlı olanlar burada hesaplanıyor.
 */
const pageSub = computed(() => {
  if (isProducts.value) {
    return `${store.counts.tumu} ürün bekliyor · ${store.counts.tamamlandi} tamamlandı`;
  }
  if (store.page === "settings") return "Kaynaklar ve yedekleme";
  return subOf(store.page);
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
  if (!workspaceOpen.value) return;
  const target = e.target instanceof HTMLElement ? e.target : null;
  if (target?.closest("#team-chat-panel")) return;
  const tag = target?.tagName.toLowerCase();
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

/**
 * Asistan hangi ekranın verisiyle konuşacağını bilmeli; `store.page` asistana geçince
 * değiştiği için son ARAÇ ekranı ayrıca tutuluyor. Buradan izlenmesi bilinçli: `store.page`
 * birçok yerden doğrudan atanıyor, tek bir aksiyona bağlamak hepsini elden geçirmeyi
 * gerektirirdi.
 */
watch(
  () => store.page,
  (page, previous) => {
    if (TOOL_PAGES.includes(page)) store.lastToolPage = page;
    pageSlidesBack.value = NAV.findIndex((item) => item.key === page) < NAV.findIndex((item) => item.key === previous);
  },
  { immediate: true },
);

function refreshTeamScreen(reuseTodayInputs = false, refreshTodayDecisions = false) {
  if (store.page === "today") return store.loadToday(reuseTodayInputs, refreshTodayDecisions);
  if (store.page === "contacts") return store.loadContacts();
  if (store.page === "quotes") return store.loadQuotes();
  if (store.page === "products") return store.refreshProductsSource().catch((error) => store.toast(String(error), "error"));
}

watch([teamTodayMode, teamTodayScope], () => {
  const scope = teamTodayScope();
  const team = teamTodayMode();
  store.resetTodaySource();
  if (workspaceOpen.value && (store.page === "today" || scope)) {
    void store.loadToday().then(() => {
      if (scope === teamTodayScope() && team === teamTodayMode()) return store.loadSession();
    });
  }
});

let todayRealtimeAvailable = false;
watch([teamClient, teamTodayScope, () => teamSession.value?.user.id], ([client, scope, user], _, cleanup) => {
  if (!client || !scope || !user) return;
  const workspace = teamWorkspace.value;
  let refreshTimer: number | undefined;
  const refresh = () => {
    if (scope !== teamTodayScope() || (store.page !== "today" && !store.session?.session_id)) return;
    window.clearTimeout(refreshTimer);
    refreshTimer = window.setTimeout(() => void store.loadToday(), 250);
  };
  const filter = `workspace_id=eq.${workspace}`;
  let active = true;
  const channel = client.channel(`today:${workspace}`)
    .on("postgres_changes", { event: "*", schema: "public", table: "team_today_decisions", filter }, refresh)
    .on("postgres_changes", { event: "*", schema: "public", table: "team_contacts", filter }, refresh)
    .subscribe((status) => {
      if (!active || scope !== teamTodayScope()) return;
      todayRealtimeAvailable = status === "SUBSCRIBED";
      if (todayRealtimeAvailable) refresh();
    });
  cleanup(() => {
    active = false;
    todayRealtimeAvailable = false;
    window.clearTimeout(refreshTimer);
    void client.removeChannel(channel);
  });
});

watch([teamProject, teamUser, teamWorkspace], () => {
  teamPanelOpen.value = false;
  store.resetContactsSource();
  store.resetQuoteSource();
  store.resetProductsSource();
  store.resetStorePagesSource();
  resetTeamQuoteIndicators();
  if (startupReady.value && teamSession.value && !teamRestoring.value) resetTeamConnectionStatus();
  if (!workspaceOpen.value) return;
  if (store.page !== "today") void refreshTeamScreen();
  const pageKind = KIND_BY_PAGE[store.page];
  if (pageKind) void store.loadStorePages(pageKind);
  if (store.page !== "products") void store.reload().catch((error) => store.toast(String(error), "error"));
});

async function refreshGscOutcomes() {
  if (TOOL_PAGES.includes(store.page) || store.outcomeSummary) await store.loadOutcomes();
  else await teamOutcomeInputs();
}

watch([teamGscMode, teamMeasurementScope], () => {
  store.opportunity = null;
  store.outcomeSummary = null;
  store.outcomeBadges = {};
  store.opportunityError = "";
  if (!workspaceOpen.value) return;
  void store.loadOpportunityCache();
  void refreshGscOutcomes().catch(console.warn);
});

let checkingConnection = false;
let checkAgain = false;
let nextCatalogCheck = 0;
async function checkTeamConnection(force = false) {
  if (!startupReady.value || !teamUser.value) return;
  if (checkingConnection) {
    checkAgain ||= force;
    return;
  }
  checkingConnection = true;
  try {
    if (!teamSession.value && navigator.onLine) await restoreTeamSession();
    if (force && !teamWorkspace.value) await resolveTeamWorkspace();
    if (!teamUser.value || !teamWorkspace.value) return;
    const wasReachable = teamReachable.value;
    const hadAccessIssue = teamAccessIssue.value;
    const reachable = await probeTeamConnection();
    if (!reachable) {
      if (workspaceOpen.value && (force || wasReachable !== false)) await refreshTeamScreen();
      return;
    }
    if (teamAccessIssue.value) return;
    void flushTeamGsc().catch(console.warn);
    const measurements = flushTeamGscSnapshots().catch(console.warn);
    const reports = flushGscReports().catch(console.warn);
    void flushTeamStorePages().catch(console.warn);
    void flushTeamIdeasoftReceipts().catch(console.warn);
    const pendingContacts = contactSync.value.pending;
    const pendingSeo = teamSeoSync.value.pending;
    const queued = pendingContacts || teamQuoteQueue.value.pending || pendingSeo
      || (teamTodaySync.value.scope === teamTodayScope() && teamTodaySync.value.pending);
    if (!force && wasReachable === true && !hadAccessIssue && !queued && store.page === "today" && !todayRealtimeAvailable) {
      await store.loadToday(true, true);
      return;
    }
    if (force || wasReachable !== true || hadAccessIssue || queued) {
      // A queued quote can refer to a customer that must be sent first.
      for (const flush of [flushTeamContacts, flushTeamQuotes, flushTeamProductSeo]) {
        try {
          await flush();
        } catch (error) {
          store.toast(String(error), "error");
        }
      }
      const todayChanged = await flushTeamToday().catch((error) => { console.warn(error); return false; });
      const reuseTodayInputs = wasReachable === true && !hadAccessIssue && !todayChanged &&
        pendingContacts === contactSync.value.pending && pendingSeo === teamSeoSync.value.pending;
      await refreshTeamScreen(reuseTodayInputs, force || !todayRealtimeAvailable);
      if (store.page !== "today" && store.session?.session_id && !reuseTodayInputs) await store.loadToday();
      if (wasReachable !== true || hadAccessIssue) {
        const pageKind = KIND_BY_PAGE[store.page];
        await Promise.allSettled([
          store.page !== "contacts" ? teamContacts.list("", true, "", "") : undefined,
          store.page !== "quotes" ? teamQuotes.list() : undefined,
          refreshTeamCatalog(),
          measurements.then(refreshGscOutcomes),
          reports.then(() => store.loadOpportunityCache()),
          pageKind ? flushTeamStorePages().then(() => store.loadStorePages(pageKind)) : undefined,
        ]);
      }
    } else if (store.page === "products" && Date.now() >= nextCatalogCheck) {
      nextCatalogCheck = Date.now() + 60_000;
      await store.refreshOpenCatalog().catch(console.warn);
    }
  } finally {
    checkingConnection = false;
    if (checkAgain) {
      checkAgain = false;
      void checkTeamConnection(true);
    }
  }
}

watch([() => teamSession.value?.user.id, teamWorkspace], () => { void checkTeamConnection(true); });
watch(teamConnectionRetry, () => { void checkTeamConnection(true); });
let syncTimer: number | undefined;
function connected() {
  void checkTeamConnection(true);
}
function disconnected() {
  markTeamDisconnected();
  if (workspaceOpen.value) void refreshTeamScreen();
}

onMounted(() => {
  void Promise.allSettled([store.loadAppearance(), restoreTeamSession()]).then(() => {
    startupReady.value = true;
    if (navigator.onLine && teamUser.value) connected();
  });
  window.addEventListener("online", connected);
  window.addEventListener("offline", disconnected);
  window.addEventListener("focus", connected);
  syncTimer = window.setInterval(() => {
    if (teamUser.value) void checkTeamConnection();
  }, 12_000);
  window.addEventListener("keydown", onKey);
});
onUnmounted(() => {
  window.removeEventListener("online", connected);
  window.removeEventListener("offline", disconnected);
  window.removeEventListener("focus", connected);
  window.clearInterval(syncTimer);
  window.removeEventListener("keydown", onKey);
});
</script>

<template>
  <TeamSignIn v-if="!workspaceOpen" :checking="!startupReady" />
  <div v-else class="app">
    <Sidebar />

    <main class="main">
      <header class="topbar">
        <div>
          <div class="pg-title">{{ pageTitle }}</div>
          <div class="pg-sub">{{ pageSub }}</div>
        </div>
        <div class="topbar-right">
          <template v-if="isProducts">
            <div class="sync-info">
              <span v-if="!teamUser || !teamWorkspace" class="last">Son güncelleme: {{ fmtSync() }}</span>
              <div v-if="store.allRows.length" class="progress" :title="`${store.counts.tamamlandi} ürün tamamlandı`">
                <div class="pbar">
                  <div class="pfill" :style="{ width: donePct + '%' }"></div>
                </div>
                <span class="ptext">{{ store.counts.tamamlandi }}/{{ store.allRows.length }} tamamlandı</span>
              </div>
            </div>
            <button class="sync" :disabled="store.syncing" @click="store.sync()">
              <Icon name="refresh" :size="16" :class="{ spin: store.syncing }" />
              <span>{{ store.syncing ? "Güncelleniyor…" : teamUser && teamWorkspace ? "Kataloğu Yenile" : "Manuel Güncelle" }}</span>
            </button>
          </template>
          <button
            ref="teamPanelButton"
            class="team-panel-trigger"
            type="button"
            aria-label="Ekip paneli"
            aria-controls="team-chat-panel"
            :aria-expanded="teamPanelOpen"
            @click="setTeamPanelOpen(!teamPanelOpen)"
          >
            <Icon name="users" :size="16" aria-hidden="true" />
            <span>Ekip</span>
          </button>
        </div>
      </header>

      <UpdateModal />
      <SetupWizard />
      <StartupCheck />
      <FocusSummaryModal />

      <div class="workspace" :class="{ 'workspace-pinned': teamPanelOpen && teamPanelPinned }">
        <div class="workspace-content">
          <!-- Odak seansı ekran değiştiğinde de görünür kalır. -->
          <FocusBar />

          <div class="workspace-viewport">
            <Transition name="surface-slide" @leave="removePreviousPage">
              <!-- Katalog ekranları aynı bileşeni kullandığı için geçişte seçim ve arama sıfırlanmalı. -->
              <div :key="store.page" class="workspace-page" :class="{ 'workspace-page-back': pageSlidesBack }">
                <component :is="PAGES[store.page]" ref="pageRef" />
              </div>
            </Transition>
          </div>
        </div>
        <TeamChatPanel v-model:pinned="teamPanelPinned" :open="teamPanelOpen" @update:open="setTeamPanelOpen" />
      </div>
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
.workspace {
  --team-chat-width: min(100%, clamp(284px, 24vw, 380px));
  position: relative;
  flex: 1;
  display: grid;
  grid-template-columns: minmax(0, 1fr) 0px;
  grid-template-rows: minmax(0, 1fr);
  min-height: 0;
  transition: grid-template-columns var(--slide-duration) var(--slide-easing);
}
.workspace-content, .workspace-viewport, .workspace-page {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
}
.workspace-viewport, .workspace-page { flex: 1; }
.workspace-viewport { overflow: clip; }
.workspace-page-back { --slide-enter-offset: calc(-1 * var(--slide-distance)); }
.workspace-page.surface-slide-enter-active:focus-within {
  transition: none;
  transform: none;
  opacity: 1;
}
.workspace-pinned { grid-template-columns: minmax(0, 1fr) var(--team-chat-width); }
.workspace-pinned :deep(.list-panel) { width: clamp(220px, 34%, 392px); }
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
.team-panel-trigger {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: 38px;
  padding: 0 13px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-bg);
  color: var(--c-text);
  font: inherit;
  font-size: 13px;
  font-weight: 590;
  white-space: nowrap;
  cursor: pointer;
}
.team-panel-trigger:hover { background: var(--c-hover); }
.team-panel-trigger[aria-expanded="true"] {
  border-color: transparent;
  background: var(--accent-tint);
  color: var(--accent);
}
.team-panel-trigger:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
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
