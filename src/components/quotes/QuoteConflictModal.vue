<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef } from "vue";
import {
  quoteConflictFields, teamQuoteCanEdit, teamQuotes, type TeamQuoteConflict,
  type TeamQuoteConflictField, type TeamQuoteConflictRetry,
} from "../../teamQuotes";
import type { Quote } from "../../types";
import Icon from "../Icon.vue";
import ModalShell from "../ModalShell.vue";
import QuoteVersionComparison from "./QuoteVersionComparison.vue";

const props = defineProps<{ id: string; number: string }>();
const emit = defineEmits<{ close: []; accepted: [remote: Quote]; reapplied: [retry: TeamQuoteConflictRetry] }>();
const root = ref<HTMLElement | null>(null);
const review = shallowRef<TeamQuoteConflict | null>(null);
const loading = ref(false);
const applying = ref<"shared" | "local" | null>(null);
const selected = ref<TeamQuoteConflictField[]>([]);
const error = ref("");
let request: AbortController | undefined;
let app: HTMLElement | null = null;
let previousInert = false;

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
  void refresh();
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
  } else if (event.key === "Tab") {
    const controls = [...(root.value?.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), summary, [tabindex="0"]') ?? [])];
    const focused = controls.indexOf(document.activeElement as HTMLElement);
    if (focused >= 0 && (event.shiftKey ? focused > 0 : focused < controls.length - 1)) return;
    event.preventDefault();
    (event.shiftKey ? controls[controls.length - 1] : controls[0])?.focus();
  }
}

async function refresh() {
  request?.abort();
  const controller = new AbortController();
  request = controller;
  review.value = null;
  selected.value = [];
  error.value = "";
  loading.value = true;
  dialog()?.focus({ preventScroll: true });
  try {
    const result = await teamQuotes.inspectConflict(props.id, controller.signal);
    if (!controller.signal.aborted) review.value = result;
  } catch (cause) {
    if (!controller.signal.aborted && !(cause instanceof DOMException && cause.name === "AbortError")) {
      error.value = cause instanceof Error ? cause.message : "Teklif sürümleri görüntülenemedi.";
    }
  } finally {
    if (!controller.signal.aborted) loading.value = false;
  }
}

async function apply(choice: "shared" | "local") {
  const snapshot = review.value;
  if (!snapshot?.remote || applying.value || loading.value || error.value) return;
  const fields = [...selected.value];
  if (choice === "local" && (!canReapply.value || !fields.length)) return;
  request?.abort();
  const controller = new AbortController();
  request = controller;
  applying.value = choice;
  dialog()?.focus({ preventScroll: true });
  try {
    if (choice === "local") {
      const retry = await teamQuotes.reapplyConflict(snapshot, fields, controller.signal);
      if (!controller.signal.aborted) emit("reapplied", retry);
    } else {
      const remote = await teamQuotes.acceptConflict(snapshot, controller.signal);
      if (!controller.signal.aborted) emit("accepted", remote);
    }
  } catch (cause) {
    if (!controller.signal.aborted && !(cause instanceof DOMException && cause.name === "AbortError")) {
      error.value = cause instanceof Error ? cause.message : "Teklif kaydedilemedi. Karşılaştırmayı yenileyin.";
    }
  } finally {
    if (!controller.signal.aborted) applying.value = null;
  }
}

const fieldLabels: Record<TeamQuoteConflictField, string> = {
  number: "Teklif numarası",
  customer: "Müşteri",
  pricing: "Para birimi, kur ve kalemler",
  validity: "Geçerlilik",
  note: "Not",
};
const changedFields = computed(() => {
  const data = review.value;
  return data?.remote ? quoteConflictFields(data.local, data.remote) : [];
});
const canReapply = computed(() => teamQuoteCanEdit.value && review.value?.remote?.status === "draft"
  && changedFields.value.length > 0 && !error.value);
const remoteMessages = {
  offline: "Ortak sürümü görmek için Supabase bağlantısı gerekiyor.",
  missing: "Ortak teklif aktif listede bulunamadı. Cihazdaki taslak korunuyor.",
  unavailable: "Ortak teklif şu anda okunamıyor. Cihazdaki taslak korunuyor.",
};
</script>

<template>
  <Teleport to="body">
    <div ref="root" @keydown.stop="onKeydown">
      <ModalShell open scroll icon="alert" label="Teklif çakışması" title="Teklif çakışması" :sub="number" :width="760"
        :closable="!applying" @close="close">
        <p v-if="loading" class="review-state" role="status">
          <Icon name="loader" :size="16" class="spin" aria-hidden="true" /> Teklif sürümleri okunuyor…
        </p>
        <p v-if="error" class="review-state review-error" role="alert">{{ error }}</p>
        <template v-if="review">
          <p v-if="review.remoteState !== 'ready'" class="review-state" role="status">{{ remoteMessages[review.remoteState] }}</p>
          <QuoteVersionComparison :local="review.local" :remote="review.remote" />
          <fieldset v-if="canReapply" class="retry-fields" :disabled="!!applying">
            <legend>Uygulanacak yerel alanlar</legend>
            <p>Seçtiğiniz alanlar ortak taslağa kaydedilir. Diğer alanların ortak sürümü korunur.</p>
            <label v-for="field in changedFields" :key="field">
              <input v-model="selected" type="checkbox" :value="field" />
              {{ fieldLabels[field] }}
            </label>
          </fieldset>
          <p v-if="review.remote && !error" class="review-state">
            Ortak sürümü seçtiğinizde cihaz taslağı yerelde saklanır ve eşitleme kuyruğundan çıkarılır.
          </p>
        </template>
        <template #footer>
          <div class="review-actions">
            <button type="button" class="secondary-action" :disabled="!!applying" @click="close">Kapat</button>
            <button v-if="!loading && (!review?.remote || error)" type="button" class="primary-action" :disabled="!!applying" @click="refresh">Karşılaştırmayı yenile</button>
            <button v-if="review?.remote && !error" type="button" :class="canReapply ? 'secondary-action' : 'primary-action'"
              :disabled="!!applying" @click="apply('shared')">
              {{ applying === 'shared' ? "Ortak sürüm doğrulanıyor…" : "Ortak sürümü kullan" }}
            </button>
            <button v-if="canReapply" type="button" class="primary-action"
              :disabled="!!applying || !selected.length" @click="apply('local')">
              {{ applying === 'local' ? "Yerel alanlar uygulanıyor…" : "Yerel alanları uygula" }}
            </button>
          </div>
        </template>
      </ModalShell>
    </div>
  </Teleport>
</template>

<style scoped>
.review-state {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  color: var(--c-mid);
  font-size: 13px;
  line-height: 1.5;
}
.review-error { color: var(--badge-eksik-c); }
.retry-fields { margin: 0; min-width: 0; padding: 10px; border: 1px solid var(--c-border); border-radius: 9px; }
.retry-fields legend { padding: 0 4px; color: var(--c-text); font-size: 13px; font-weight: 600; }
.retry-fields p { margin: 0 0 6px; color: var(--c-mid); font-size: 13px; line-height: 1.5; }
.retry-fields label { display: inline-flex; align-items: center; gap: 6px; min-height: 32px; padding-inline-end: 14px; font-size: 13px; }
.retry-fields input { accent-color: var(--accent); }
.review-actions { display: flex; flex-wrap: wrap; gap: 8px; }
</style>
