<script setup lang="ts">
/**
 * Bakım kontrolü — açılışta ve Ayarlar'da **aynı** bileşen.
 *
 * **Neden var.** Aynı bakım işleri dört ayrı ekrana dağılmıştı: feed senkronu Ürünler'de,
 * GSC yenileme Genel Bakış'ta, model listesi ve token testi Ayarlar'da. Her açılışta dört
 * ekran gezmek gerekiyordu.
 *
 * 🔴 **Kontrol var, çalıştırma yok.** Ucuz kontroller gerçekten koşuyor (token, model
 * listesi); pahalı olanların yalnızca tazeliği okunuyor ve düğmeye SİZ basıyorsunuz.
 * Gerekçe `commands/health.rs` başlığında: GSC analizi bir ölçüm olayı, her açılışta
 * koşarsa ölçtüğümüz şeyi ölçüm aracıyla değiştirmiş oluruz.
 *
 * ⚠️ Tek bileşen, iki yer: kopyalasaydık biri güncellenmeyip sapardı — bu projede
 * kopyalanan tablo/geometri dört kez saptı.
 */
import { computed, ref, watch } from "vue";
import { api } from "../api";
import { syncTeamStoreInventory } from "../teamStorePages";
import { teamAccessIssue, teamReachable, teamRestoring, teamUser, teamWorkspace } from "../teamConnection";
import { readTeamDataHealth } from "../teamHealth";
import { testTeamGemini } from "../teamGemini";
import { testTeamIdeasoft } from "../teamIdeasoft";
import { useStore } from "../store";
import Icon from "./Icon.vue";
import type { HealthCheck } from "../types";

const props = withDefaults(defineProps<{ compact?: boolean }>(), { compact: false });
const emit = defineEmits<{ done: [temiz: boolean] }>();
const store = useStore();

const dataChecks = ref<HealthCheck[]>([]);
const connectionChecks = ref<HealthCheck[]>([]);
const checking = ref(true);
const calisan = ref("");
let checkRequest = 0;

const inWorkspace = computed(() => Boolean(teamWorkspace.value || teamUser.value));
const hepsi = computed(() => [...dataChecks.value, ...connectionChecks.value]);
const ilerleme = computed(() => Math.round(hepsi.value.length / 5 * 100));
const sorunlu = computed(() =>
  hepsi.value.filter((c) => c.state === "stale" || c.state === "error"),
);

const TON: Record<string, { ikon: string; renk: string }> = {
  ok: { ikon: "check", renk: "var(--green)" },
  stale: { ikon: "clock", renk: "var(--amber)" },
  error: { ikon: "alert", renk: "var(--red)" },
  // ⚠️ Kapalı modül HATA DEĞİL: kullanmadığı bir şey için kullanıcıyı endişelendirmeyelim.
  off: { ikon: "info", renk: "var(--c-faint)" },
};

async function sharedConnectionChecks(): Promise<HealthCheck[]> {
  const providers = [
    { key: "gemini", label: "Ortak Gemini", test: testTeamGemini },
    { key: "ideasoft", label: "Ortak IdeaSoft", test: testTeamIdeasoft },
  ];
  return Promise.all(providers.map(async ({ key, label, test }) => {
    if (teamReachable.value !== true || !navigator.onLine || teamAccessIssue.value) {
      return { key, label, state: "off", detail: "Bağlantı kurulunca denetlenecek", action: "" };
    }
    try {
      return { key, label, state: "ok", detail: await test(), action: "" };
    } catch (e) {
      return { key, label, state: "error", detail: e instanceof Error ? e.message : String(e), action: "open_settings" };
    }
  }));
}

async function kontrolEt() {
  const request = ++checkRequest;
  dataChecks.value = [];
  connectionChecks.value = [];
  checking.value = true;
  if (teamRestoring.value) return;
  const results = [
    { slot: dataChecks, result: inWorkspace.value ? readTeamDataHealth() : api.localHealth() },
    { slot: connectionChecks, result: inWorkspace.value ? sharedConnectionChecks() : api.remoteHealth() },
  ];
  await Promise.all(results.map(async ({ slot, result }) => {
    try {
      const checks = await result;
      if (request === checkRequest) slot.value = checks;
    } catch (e) {
      if (request === checkRequest) store.toast(String(e), "error");
    }
  }));
  if (request !== checkRequest) return;
  checking.value = false;
  emit("done", hepsi.value.length === 5 && sorunlu.value.length === 0);
}
watch([teamUser, teamWorkspace, teamRestoring, teamReachable, teamAccessIssue], kontrolEt, { immediate: true });
defineExpose({ kontrolEt });

/**
 * Satırdaki eylemi çalıştırır.
 *
 * ⚠️ Bunlar PAHALI işler ve bilerek elle tetikleniyor. Bittiğinde kontrol yenileniyor ki
 * satır "bugün" desin — yoksa kullanıcı işi yaptı ama ekran hâlâ "bayat" gösterirdi.
 */
async function calistir(c: HealthCheck) {
  if (!c.action || calisan.value) return;
  calisan.value = c.key;
  try {
    // ⚠️ `store.sync()` (feed senkronu) — `api.syncFeed` doğrudan çağrılmıyor: store
    // eylemi ayrıca listeyi yeniliyor ve özet şeridini gösteriyor, ikisi de gerekli.
    if (c.action === "sync_feed") await store.sync();
    else if (c.action === "run_analysis") await store.runOpportunityAnalysis();
    else if (c.action === "sync_store_pages") await syncTeamStoreInventory();
    else if (c.action === "open_settings") {
      store.page = "settings";
      emit("done", true);
      return;
    }
    await kontrolEt();
  } catch (e) {
    store.toast(String(e), "error");
  } finally {
    calisan.value = "";
  }
}

const EYLEM_AD: Record<string, string> = {
  sync_feed: "Senkronla",
  run_analysis: "Analiz et",
  sync_store_pages: "Envanteri çek",
  open_settings: "Ayarlar",
};

function actionLabel(action: string) {
  if (inWorkspace.value && action === "sync_feed") return "Kataloğu yenile";
  return EYLEM_AD[action] || "Çalıştır";
}
</script>

<template>
  <div class="hp" :class="{ compact: props.compact }">
    <div v-if="checking" class="bar">
      <div class="dolgu" :style="{ width: ilerleme + '%' }"></div>
    </div>

    <div v-for="c in hepsi" :key="c.key" class="sat">
      <Icon :name="TON[c.state].ikon" :size="14" :style="{ color: TON[c.state].renk }" />
      <span class="ad">{{ c.label }}</span>
      <span class="det">{{ c.detail }}</span>
      <button
        v-if="c.action"
        class="eyl"
        :disabled="!!calisan"
        @click="calistir(c)"
      >
        <Icon v-if="calisan === c.key" name="refresh" :size="12" class="spin" />
        {{ calisan === c.key ? "…" : actionLabel(c.action) }}
      </button>
    </div>

    <div v-if="checking" class="sat bekle">
      <Icon name="refresh" :size="14" class="spin" />
      <span class="ad">Bağlantılar denetleniyor…</span>
    </div>
  </div>
</template>

<style scoped>
.hp {
  display: flex;
  flex-direction: column;
}
.bar {
  height: 3px;
  border-radius: 2px;
  background: var(--c-border);
  overflow: hidden;
  margin-bottom: 10px;
}
.dolgu {
  height: 100%;
  background: var(--accent);
  transition: width 0.3s cubic-bezier(0.32, 0.72, 0, 1);
}
.sat {
  display: grid;
  grid-template-columns: 18px 1fr auto auto;
  gap: 10px;
  align-items: center;
  padding: 7px 0;
  border-bottom: 1px solid var(--c-border);
  font-size: 12.5px;
}
.sat:last-child {
  border-bottom: 0;
}
.ad {
  font-weight: 560;
}
.det {
  color: var(--c-faint);
  font-size: 11.5px;
  text-align: right;
}
.eyl {
  height: 25px;
  padding: 0 10px;
  border-radius: 7px;
  border: 1px solid var(--c-border);
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 11.5px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 5px;
}
.eyl:disabled {
  opacity: 0.45;
  cursor: default;
}
.bekle .ad {
  font-weight: 400;
  color: var(--c-faint);
}
.compact .sat {
  grid-template-columns: 18px minmax(0, 1fr) auto;
  gap: 3px 8px;
  padding: 7px 0;
  font-size: 12px;
}
.compact .det {
  grid-column: 2 / -1;
  grid-row: 2;
  text-align: left;
}
.compact .sat:has(.eyl) .det { grid-column-end: 3; }
.compact .eyl {
  grid-column: 3;
  grid-row: 1 / span 2;
}
</style>
