<script setup lang="ts" generic="Version extends { revision?: number }, Review extends { local: Version; remote: Version | null; remoteState: string; remoteAccessBlocked?: boolean }">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef } from "vue";
import { stripHtml } from "../validation";
import Icon from "./Icon.vue";
import ModalShell from "./ModalShell.vue";

type Field<T> = { key: string; label: string; value: (version: T) => unknown; format?: "status" | "table" | "html" };

const props = defineProps<{
  name: string;
  mode: "review" | "history";
  history: { saved_at: string; local: Version; remote: Version }[];
  fields: Field<Version>[];
  load: (signal: AbortSignal) => Promise<Review>;
  accept: (review: Review, signal: AbortSignal) => Promise<Version | null>;
}>();
const emit = defineEmits<{ close: []; accepted: [remote: Version] }>();
const root = ref<HTMLElement | null>(null);
const review = shallowRef<Review | null>(null);
const selectedArchive = ref(0);
const loading = ref(false);
const applying = ref(false);
const error = ref("");
let request: AbortController | undefined;
let app: HTMLElement | null = null;
let previousInert = false;

const isHistory = computed(() => props.mode === "history");
const title = computed(() => isHistory.value ? "Saklanan SEO düzenlemeleri" : "SEO sürümlerini karşılaştır");
const archive = computed(() => props.history[selectedArchive.value] ?? props.history[0]);
const record = computed(() => isHistory.value ? archive.value : review.value);
const dialog = () => root.value?.querySelector<HTMLElement>('[role="dialog"]');

onMounted(() => {
  app = document.getElementById("app");
  if (app) {
    previousInert = app.inert;
    app.inert = true;
  }
  const element = dialog();
  element?.setAttribute("aria-modal", "true");
  element?.setAttribute("tabindex", "-1");
  element?.focus({ preventScroll: true });
  if (!isHistory.value) void refresh();
});

onBeforeUnmount(() => {
  request?.abort();
  if (app) app.inert = previousInert;
});

function close() {
  if (!applying.value) emit("close");
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    close();
    return;
  }
  if (event.key !== "Tab") return;
  const controls = [...(root.value?.querySelectorAll<HTMLElement>("button:not(:disabled), select:not(:disabled), summary") ?? [])];
  const focused = controls.indexOf(document.activeElement as HTMLElement);
  if (focused >= 0 && (event.shiftKey ? focused > 0 : focused < controls.length - 1)) return;
  event.preventDefault();
  (event.shiftKey ? controls[controls.length - 1] : controls[0])?.focus();
}

async function refresh() {
  request?.abort();
  const controller = new AbortController();
  request = controller;
  error.value = "";
  review.value = null;
  loading.value = true;
  dialog()?.focus({ preventScroll: true });
  try {
    const result = await props.load(controller.signal);
    if (!controller.signal.aborted) review.value = result;
  } catch (cause) {
    if (!controller.signal.aborted) error.value = cause instanceof Error ? cause.message : "SEO sürümleri okunamadı.";
  } finally {
    if (!controller.signal.aborted) loading.value = false;
  }
}

async function accept() {
  const snapshot = review.value;
  if (!snapshot?.remote || loading.value || applying.value || error.value) return;
  request?.abort();
  const controller = new AbortController();
  request = controller;
  applying.value = true;
  dialog()?.focus({ preventScroll: true });
  try {
    const applied = await props.accept(snapshot, controller.signal);
    if (controller.signal.aborted) return;
    if (applied) {
      emit("accepted", applied);
      return;
    }
    error.value = "Bu cihazdaki düzenlemeler değişti. Sürümleri yeniden karşılaştırın.";
  } catch (cause) {
    if (!controller.signal.aborted) error.value = cause instanceof Error ? cause.message : "Ortak SEO sürümü doğrulanamadı.";
  }
  if (!controller.signal.aborted) applying.value = false;
}

const formatting = new Intl.DateTimeFormat("tr-TR", { dateStyle: "medium", timeStyle: "short" });
function savedDate(value: string) {
  const parsed = new Date(value);
  return Number.isFinite(parsed.getTime()) ? formatting.format(parsed) : "Tarih bilinmiyor";
}

function showField(value: unknown, format: Field<Version>["format"]) {
  const raw = typeof value === "string" ? value : value == null ? "" : JSON.stringify(value, null, 2);
  if (!raw) return { raw, preview: "—", openLabel: "" };
  if (format === "status") return { raw, preview: raw === "done" ? "Tamamlandı" : raw === "pending" ? "Bekliyor" : raw, openLabel: "" };
  if (format === "table" && Array.isArray(value)) {
    const groups = value.map((group) => {
      if (!group || typeof group.group !== "string" || !Array.isArray(group.rows)) return "";
      const rows = group.rows.map((row: { label?: string; value?: string } | null) =>
        row?.label ? `${row.label} · ${row.value ?? ""}` : "");
      return [group.group, ...rows.filter(Boolean)].join("\n");
    });
    return { raw: groups.every(Boolean) ? groups.join("\n\n") : raw, preview: `${value.length} özellik grubu`, openLabel: "Tabloyu göster" };
  }
  const text = format === "html" ? stripHtml(raw) : raw;
  const preview = (text || "HTML içeriği").slice(0, 230);
  return {
    raw,
    preview: preview + (text.length > 230 ? "…" : ""),
    openLabel: format === "html" ? "HTML içeriğini göster" : text.length > 230 ? "Metnin tamamını göster" : "",
  };
}

const rows = computed(() => {
  const data = record.value;
  if (!data) return [];
  return props.fields.flatMap(({ key, label, value, format }) => {
    const local = showField(value(data.local), format);
    const shared = data.remote ? showField(value(data.remote), format) : null;
    return (shared ? local.raw !== shared.raw : !!local.raw) ? [{ key, label, local, shared }] : [];
  });
});

const remoteMessage = computed(() => {
  const data = review.value;
  if (!data || data.remoteState === "ready") return "";
  if (data.remoteState === "offline") return "Ortak SEO sürümünü görmek için Supabase bağlantısı gerekiyor.";
  if (data.remoteState === "missing") return "Supabase’te bu kayda ait bir SEO taslağı bulunamadı. Bu cihazdaki düzenlemeler gösteriliyor.";
  return data.remoteAccessBlocked ? "Ortak sürümü görmek için çalışma alanı erişimini doğrulayın."
    : "Ortak SEO sürümü şu anda okunamıyor. Karşılaştırmayı yenileyin.";
});
</script>

<template>
  <Teleport to="body">
    <div ref="root" @keydown.stop="onKeydown">
      <ModalShell open scroll :label="title" :title="title" :sub="name" :icon="isHistory ? 'clock' : 'alert'"
        :width="780" :closable="!applying" @close="close">
        <p v-if="loading" class="review-state" role="status">
          <Icon name="loader" :size="16" class="spin" aria-hidden="true" /> SEO sürümleri okunuyor…
        </p>
        <p v-if="error" class="review-state failure" role="alert">{{ error }}</p>
        <p v-if="remoteMessage && !error" class="review-state" role="status">{{ remoteMessage }}</p>

        <label v-if="isHistory && history.length > 1" class="history-select">
          Saklama tarihi
          <select v-model.number="selectedArchive" class="fx">
            <option v-for="(entry, index) in history" :key="index" :value="index">{{ savedDate(entry.saved_at) }}</option>
          </select>
        </label>
        <p v-else-if="isHistory && archive" class="review-state">Saklama tarihi · {{ savedDate(archive.saved_at) }}</p>

        <template v-if="record">
          <div v-if="rows.length" class="comparison-wrap">
            <table class="comparison" aria-label="SEO taslak alanları">
              <thead>
                <tr>
                  <th scope="col">Alan</th>
                  <th scope="col">Bu cihaz</th>
                  <th v-if="record.remote" scope="col">Ortak sürüm <small v-if="record.remote.revision != null">v{{ record.remote.revision }}</small></th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in rows" :key="row.key">
                  <th scope="row">{{ row.label }}</th>
                  <td>
                    {{ row.local.preview }}
                    <details v-if="row.local.openLabel">
                      <summary>{{ row.local.openLabel }}</summary>
                      <pre>{{ row.local.raw }}</pre>
                    </details>
                  </td>
                  <td v-if="row.shared">
                    {{ row.shared.preview }}
                    <details v-if="row.shared.openLabel">
                      <summary>{{ row.shared.openLabel }}</summary>
                      <pre>{{ row.shared.raw }}</pre>
                    </details>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <p v-else class="review-state">{{ record.remote ? "SEO alanlarında fark yok." : "Bu cihazdaki taslakta dolu SEO alanı yok." }}</p>
          <p v-if="review?.remote && !error" class="notice">
            Ortak sürümü seçerseniz bu cihazdaki SEO değişiklikleri Saklanan düzenlemeler bölümünde tutulur.
          </p>
        </template>
        <p v-else-if="isHistory" class="review-state">Bu kayıt için saklanan düzenleme bulunamadı.</p>

        <template #footer>
          <div class="review-actions">
            <button type="button" class="secondary-action" :disabled="applying" @click="close">Kapat</button>
            <button v-if="!isHistory && !loading && (!review?.remote || error)" type="button" class="primary-action"
              :disabled="applying" @click="refresh">Karşılaştırmayı yenile</button>
            <button v-if="review?.remote && !error" type="button" class="primary-action" :disabled="applying"
              :class="{ busy: applying }" @click="accept">
              <Icon :name="applying ? 'loader' : 'check'" :size="15" :class="{ spin: applying }" aria-hidden="true" />
              {{ applying ? "Doğrulanıyor…" : "Ortak sürümü kullan" }}
            </button>
          </div>
        </template>
      </ModalShell>
    </div>
  </Teleport>
</template>

<style scoped>
:deep(.modal:focus) { outline: none; }
.review-state, .notice { margin: 0; color: var(--c-mid); font-size: 13px; line-height: 1.5; }
.review-state { display: flex; align-items: center; gap: 8px; }
.failure { color: var(--badge-eksik-c); }
.notice { padding: 9px 12px; border: 1px solid var(--c-border); border-radius: 9px; background: var(--c-hover); }
.comparison-wrap { overflow: hidden; border: 1px solid var(--c-border); border-radius: 9px; }
.comparison { width: 100%; table-layout: fixed; border-collapse: collapse; color: var(--c-text); font-size: 13px; }
.comparison :is(th, td) { padding: 9px 11px; text-align: start; vertical-align: top; overflow-wrap: anywhere; }
.comparison thead { background: var(--c-hover); }
.comparison thead th:first-child { width: 25%; }
.comparison thead small { display: inline-block; margin-inline-start: 4px; color: var(--c-mid); font-size: 11px; font-weight: 500; }
.comparison tbody :is(th, td) { border-top: 1px solid var(--c-border); }
.comparison tbody th { color: var(--c-mid); font-size: 12px; font-weight: 560; }
.comparison tbody td { white-space: pre-wrap; }
.comparison details { margin-top: 6px; color: var(--c-mid); font-size: 12px; }
.comparison summary { color: var(--accent); cursor: pointer; }
.comparison pre { margin: 6px 0 0; white-space: pre-wrap; overflow-wrap: anywhere; font-size: 11px; }
.history-select { display: flex; align-items: center; flex-wrap: wrap; gap: 6px 10px; color: var(--c-mid); font-size: 13px; }
.history-select select { max-width: 100%; padding: 6px 8px; border: 1px solid var(--c-border); border-radius: 8px; background: var(--c-input); color: var(--c-text); }
.review-actions { display: flex; flex-wrap: wrap; gap: 8px; }
@media (max-width: 520px) {
  .comparison :is(th, td) { padding: 6px; }
}
</style>
