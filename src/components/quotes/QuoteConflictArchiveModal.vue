<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import type { TeamQuoteConflictArchive } from "../../teamQuotes";
import Icon from "../Icon.vue";
import ModalShell from "../ModalShell.vue";
import QuoteVersionComparison from "./QuoteVersionComparison.vue";

const props = defineProps<{ number: string; entries: TeamQuoteConflictArchive[] }>();
const emit = defineEmits<{ close: [] }>();
const root = ref<HTMLElement | null>(null);
const selectedId = ref(props.entries[0]?.id);
const selected = computed(() => props.entries.find(({ id }) => id === selectedId.value) ?? props.entries[0]);
let app: HTMLElement | null = null;
let previousInert = false;

onMounted(() => {
  app = document.getElementById("app");
  if (app) {
    previousInert = app.inert;
    app.inert = true;
  }
  const dialog = root.value?.querySelector<HTMLElement>('[role="dialog"]');
  dialog?.setAttribute("aria-modal", "true");
  dialog?.setAttribute("tabindex", "-1");
  dialog?.focus({ preventScroll: true });
});

onBeforeUnmount(() => {
  if (app) app.inert = previousInert;
});

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    emit("close");
    return;
  }
  if (event.key !== "Tab") return;
  const controls = [...(root.value?.querySelectorAll<HTMLElement>("button:not(:disabled), select, summary") ?? [])];
  const focused = controls.indexOf(document.activeElement as HTMLElement);
  if (focused >= 0 && (event.shiftKey ? focused > 0 : focused < controls.length - 1)) return;
  event.preventDefault();
  (event.shiftKey ? controls[controls.length - 1] : controls[0])?.focus();
}

const dateFormat = new Intl.DateTimeFormat("tr-TR", { dateStyle: "medium", timeStyle: "short" });
function savedAt(value: string | null) {
  const date = new Date(value ?? "");
  return Number.isFinite(date.getTime()) ? dateFormat.format(date) : "";
}
</script>

<template>
  <Teleport to="body">
    <div ref="root" class="quote-archive" @keydown.stop="onKeydown">
      <ModalShell open scroll icon="archive" label="Saklanan teklif düzenlemeleri" title="Saklanan teklif düzenlemeleri"
        :sub="number" :width="760" @close="emit('close')">
        <p class="archive-info"><Icon name="archive" :size="14" aria-hidden="true" /> Bu taslaklar bu cihazda saklanıyor.</p>
        <label v-if="entries.length > 1" class="archive-picker">
          Saklanan düzenleme
          <select v-model="selectedId" class="fx">
            <option v-for="(entry, index) in entries" :key="entry.id" :value="entry.id">
              {{ savedAt(entry.saved_at) || `Düzenleme ${entries.length - index} · Tarih kaydı yok` }}
              <template v-if="entry.local.revision != null"> · Sürüm {{ entry.local.revision }}</template>
            </option>
          </select>
        </label>
        <template v-if="selected">
          <p v-if="entries.length === 1" class="archive-date">
            <template v-if="selected.saved_at && savedAt(selected.saved_at)">Saklama tarihi · <time :datetime="selected.saved_at">{{ savedAt(selected.saved_at) }}</time></template>
            <template v-else>Saklama tarihi kaydı yok.</template>
          </p>
          <QuoteVersionComparison :key="selected.id" :local="selected.local" :remote="selected.remote" expanded-lines />
        </template>
        <p v-else class="archive-date" role="status">Bu teklif için saklanan düzenleme yok.</p>
        <template #footer>
          <button type="button" class="secondary-action" @click="emit('close')">Kapat</button>
        </template>
      </ModalShell>
    </div>
  </Teleport>
</template>

<style scoped>
.quote-archive { color-scheme: light; }
:global([data-theme="dark"] .quote-archive) { color-scheme: dark; }
.quote-archive :deep(.modal:focus) { outline: none; }
.quote-archive :deep(.modal.scroll::-webkit-scrollbar) { width: 10px; }
.quote-archive :deep(.modal.scroll::-webkit-scrollbar-track) { background: transparent; }
.quote-archive :deep(.modal.scroll::-webkit-scrollbar-thumb) {
  border: 3px solid transparent;
  border-radius: 6px;
  background: rgba(140, 140, 150, 0.35);
  background-clip: padding-box;
}
.quote-archive :deep(.modal.scroll::-webkit-scrollbar-thumb:hover) {
  background-color: rgba(140, 140, 150, 0.55);
}
.quote-archive :is(button, select):focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.archive-info { display: flex; align-items: center; gap: 8px; margin: 0; color: var(--c-mid); font-size: 13px; }
.archive-info svg { flex: none; }
.archive-picker { display: flex; align-items: center; flex-wrap: wrap; gap: 6px 10px; color: var(--c-mid); font-size: 13px; }
.archive-picker select {
  max-width: 100%;
  min-height: 34px;
  padding: 6px 10px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-input);
  color: var(--c-text);
  font: inherit;
}
.archive-date { margin: 0; color: var(--c-mid); font-size: 12px; }
</style>
