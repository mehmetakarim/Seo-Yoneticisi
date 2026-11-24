<script setup lang="ts">
import { computed, ref, onBeforeUnmount, onMounted, watch } from "vue";
import { teamQuotes, type TeamQuoteVersion } from "../../teamQuotes";
import Icon from "../Icon.vue";
import ModalShell from "../ModalShell.vue";

const props = defineProps<{ quoteId: string; quoteNumber: string }>();
const emit = defineEmits<{ close: [] }>();
const surface = ref<HTMLElement | null>(null);
const records = ref<{ versions: TeamQuoteVersion[]; cached: boolean } | null>(null);
const versionNumber = ref<number | null>(null);
const loading = ref(false);
const message = ref("");
const selected = computed(() => records.value?.versions.find((version) => version.version === versionNumber.value));
let request: AbortController | undefined;
let app: HTMLElement | null;
let wasInert = false;

const dialog = () => surface.value?.querySelector<HTMLElement>('[role="dialog"]');

onMounted(() => {
  app = document.getElementById("app");
  if (app) {
    wasInert = app.inert;
    app.inert = true;
  }
  const element = dialog();
  element?.setAttribute("aria-modal", "true");
  element?.setAttribute("tabindex", "-1");
  element?.focus({ preventScroll: true });
});

onBeforeUnmount(() => {
  request?.abort();
  if (app) app.inert = wasInert;
});

async function loadHistory() {
  request?.abort();
  const controller = new AbortController();
  const quoteId = props.quoteId;
  request = controller;
  records.value = null;
  loading.value = true;
  message.value = "";
  const current = () => request === controller && !controller.signal.aborted && props.quoteId === quoteId;
  try {
    const result = await teamQuotes.history(quoteId, controller.signal);
    if (!current()) return;
    if (!result.versions.some(({ version }) => version === versionNumber.value)) {
      versionNumber.value = result.versions[0]?.version ?? null;
    }
    records.value = result;
  } catch (error) {
    if (current() && !(error instanceof Error && error.name === "AbortError")) {
      message.value = error instanceof Error ? error.message : "Gönderim geçmişi alınamadı.";
    }
  } finally {
    if (current()) loading.value = false;
  }
}

watch(() => props.quoteId, () => {
  versionNumber.value = null;
  void loadHistory();
}, { immediate: true });

function refresh() {
  dialog()?.focus({ preventScroll: true });
  void loadHistory();
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    emit("close");
    return;
  }
  if (event.key !== "Tab") return;
  const focusable = Array.from(surface.value?.querySelectorAll<HTMLElement>('button:not(:disabled), [tabindex="0"]') ?? []);
  const index = focusable.indexOf(document.activeElement as HTMLElement);
  if (index >= 0 && (event.shiftKey ? index > 0 : index < focusable.length - 1)) return;
  event.preventDefault();
  (event.shiftKey ? focusable[focusable.length - 1] : focusable[0])?.focus();
}

const timestampFormat = new Intl.DateTimeFormat("tr-TR", { dateStyle: "short", timeStyle: "short" });
const validUntilFormat = new Intl.DateTimeFormat("tr-TR", { dateStyle: "medium", timeZone: "UTC" });
const quantityFormat = new Intl.NumberFormat("tr-TR", { maximumFractionDigits: 6 });
const priceFormat = new Intl.NumberFormat("tr-TR", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
const taxFormat = new Intl.NumberFormat("tr-TR", { style: "percent", maximumFractionDigits: 2 });

function timestamp(value: string) {
  const date = new Date(value);
  return Number.isFinite(date.getTime()) ? timestampFormat.format(date) : "";
}

function validUntil(value: string | null) {
  const date = new Date(`${value?.slice(0, 10) ?? ""}T00:00:00Z`);
  return Number.isFinite(date.getTime()) ? validUntilFormat.format(date) : "—";
}

function price(value: number | null, currency: string) {
  return value === null ? "—" : `${priceFormat.format(value)}${currency ? ` ${currency}` : ""}`;
}
</script>

<template>
  <Teleport to="body">
    <div ref="surface" class="quote-history-surface" @keydown.stop="onKeydown">
      <ModalShell
        open
        scroll
        icon="clock"
        label="Gönderim geçmişi"
        title="Gönderim geçmişi"
        :sub="quoteNumber"
        :width="720"
        @close="emit('close')"
      >
        <p v-if="loading" class="history-state" role="status">
          <Icon name="loader" :size="16" class="spin" aria-hidden="true" /> Gönderim geçmişi yükleniyor…
        </p>
        <p v-else-if="message" class="history-state history-error" role="alert">
          <Icon name="alert" :size="16" aria-hidden="true" /> {{ message }}
        </p>
        <template v-else-if="records">
          <p v-if="records.cached" class="history-cache" role="status">
            <Icon name="clock" :size="14" aria-hidden="true" /> Bu cihazda saklanan gönderim geçmişi gösteriliyor.
          </p>
          <div v-if="selected" class="history-layout" :class="{ 'with-versions': records.versions.length > 1 }">
            <nav v-if="records.versions.length > 1" class="version-list om-scroll" aria-label="Gönderim sürümleri">
              <button
                v-for="version in records.versions"
                :key="version.version"
                class="version-action"
                type="button"
                :aria-current="versionNumber === version.version ? 'true' : undefined"
                @click="versionNumber = version.version"
              >
                <strong>Sürüm {{ version.version }}</strong>
                <time v-if="timestamp(version.at)" :datetime="version.at">{{ timestamp(version.at) }}</time>
              </button>
            </nav>
            <article class="version-detail" :aria-label="`Sürüm ${selected.version}`">
              <div class="version-heading">
                <h3>Sürüm {{ selected.version }}</h3>
                <time v-if="timestamp(selected.at)" :datetime="selected.at">{{ timestamp(selected.at) }}</time>
              </div>
              <dl class="version-facts">
                <div><dt>Teklif numarası</dt><dd>{{ selected.snapshot_json.quote.no || "—" }}</dd></div>
                <div><dt>Para birimi</dt><dd>{{ selected.snapshot_json.quote.currency || "—" }}</dd></div>
                <div><dt>Geçerlilik</dt><dd>{{ validUntil(selected.snapshot_json.quote.valid_until) }}</dd></div>
              </dl>
              <div v-if="selected.snapshot_json.items.length" class="version-lines om-scroll-x" tabindex="0">
                <table aria-label="Gönderilen teklifin kalemleri">
                  <thead>
                    <tr><th scope="col">Kalem</th><th scope="col" class="numeric">Adet</th><th scope="col" class="numeric">Birim fiyat</th><th scope="col" class="numeric">KDV</th></tr>
                  </thead>
                  <tbody>
                    <tr v-for="(item, index) in selected.snapshot_json.items" :key="index">
                      <th scope="row">{{ item.name || "—" }}</th>
                      <td class="numeric">{{ item.qty === null ? "—" : quantityFormat.format(item.qty) }}</td>
                      <td class="numeric">{{ price(item.unit_price, selected.snapshot_json.quote.currency) }}</td>
                      <td class="numeric">{{ item.tax_rate === null ? "—" : taxFormat.format(item.tax_rate / 100) }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <p v-else class="history-state no-lines">Bu sürümün kayıtlı teklif kalemi yok.</p>
              <div v-if="selected.snapshot_json.quote.note.trim()" class="version-note">
                <h4>Teklif notu</h4>
                <p>{{ selected.snapshot_json.quote.note }}</p>
              </div>
            </article>
          </div>
          <p v-else class="history-state" role="status">
            <Icon name="clock" :size="16" aria-hidden="true" />
            {{ records.cached ? "Bu teklif için cihazda saklanan bir gönderim sürümü yok." : "Bu teklif için henüz bir gönderim sürümü yok." }}
          </p>
        </template>

        <template #footer>
          <button type="button" class="secondary-action" @click="emit('close')">Kapat</button>
          <button v-if="message || records?.cached" type="button" class="secondary-action" @click="refresh">
            <Icon name="refresh" :size="14" aria-hidden="true" /> Yenile
          </button>
        </template>
      </ModalShell>
    </div>
  </Teleport>
</template>

<style scoped>
.quote-history-surface :deep(.modal:focus) { outline: 0; }
.quote-history-surface button:focus-visible, .version-lines:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.history-state, .history-cache {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 46px;
  margin: 0;
  padding: 10px 12px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-list);
  color: var(--c-mid);
  font-size: 12.5px;
  line-height: 1.45;
}
.history-state > svg, .history-cache > svg { flex: none; }
.history-cache { min-height: 0; color: var(--warn-text); background: var(--warn-bg); border-color: var(--warn-border); }
.history-error { color: var(--badge-eksik-c); background: var(--badge-eksik-bg); border-color: var(--badge-eksik-bg); }
.history-layout { display: grid; gap: 12px; grid-template-columns: minmax(0, 1fr); align-items: start; }
.history-layout.with-versions { grid-template-columns: minmax(132px, 1fr) minmax(0, 3fr); }
.version-list { display: grid; gap: 4px; max-height: 52vh; overflow-y: auto; padding: 4px; }
.version-action {
  display: grid;
  gap: 3px;
  min-height: 50px;
  padding: 7px 9px;
  border: 1px solid transparent;
  border-radius: 9px;
  background: var(--c-list);
  color: var(--c-text);
  text-align: left;
  cursor: pointer;
}
.version-action:hover { background: var(--c-hover); }
.version-action[aria-current="true"] { background: var(--accent-tint); border-color: var(--accent-tint); }
.version-action strong { font-size: 12px; font-weight: 620; }
.version-action time, .version-heading time { color: var(--c-mid); font-size: 11.5px; font-variant-numeric: tabular-nums; }
.version-detail { display: grid; gap: 12px; min-width: 0; padding: 12px; border: 1px solid var(--c-border); border-radius: 11px; }
.version-heading { display: flex; justify-content: space-between; flex-wrap: wrap; align-items: center; gap: 5px 10px; }
.version-heading h3 { margin: 0; color: var(--c-text); font-size: 14px; font-weight: 640; }
.version-facts { display: grid; margin: 0; padding: 10px; gap: 9px; grid-template-columns: repeat(auto-fit, minmax(min(100%, 105px), 1fr)); background: var(--c-list); border-radius: 8px; }
.version-facts dt { color: var(--c-mid); font-size: 11px; }
.version-facts dd { margin: 2px 0 0; color: var(--c-text); font-size: 12.5px; font-weight: 590; overflow-wrap: anywhere; }
.version-lines { border: 1px solid var(--c-border); border-radius: 8px; overflow: auto; }
.version-lines table { width: 100%; border-collapse: collapse; color: var(--c-text); font-size: 12px; }
.version-lines th, .version-lines td { padding: 8px; text-align: left; vertical-align: top; }
.version-lines thead { background: var(--c-list); color: var(--c-mid); }
.version-lines tbody tr { border-top: 1px solid var(--c-border-soft); }
.version-lines tbody th { min-width: 88px; overflow-wrap: anywhere; font-weight: 540; }
.version-lines .numeric { text-align: right; white-space: nowrap; font-variant-numeric: tabular-nums; }
.no-lines { min-height: 0; padding: 7px 9px; }
.version-note h4 { margin: 0 0 4px; color: var(--c-text); font-size: 12px; font-weight: 620; }
.version-note p { margin: 0; color: var(--c-mid); font-size: 12.5px; line-height: 1.5; white-space: pre-wrap; overflow-wrap: anywhere; }
@media (max-width: 650px) {
  .history-layout.with-versions { grid-template-columns: minmax(0, 1fr); }
  .version-list { grid-template-columns: repeat(auto-fit, minmax(min(100%, 120px), 1fr)); max-height: 28vh; }
}
</style>
