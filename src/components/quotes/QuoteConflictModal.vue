<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef } from "vue";
import { teamQuotes, type TeamQuoteConflict } from "../../teamQuotes";
import type { Quote } from "../../types";
import Icon from "../Icon.vue";
import ModalShell from "../ModalShell.vue";

const props = defineProps<{ id: string; number: string }>();
const emit = defineEmits<{ close: []; accepted: [remote: Quote] }>();
const root = ref<HTMLElement | null>(null);
const review = shallowRef<TeamQuoteConflict | null>(null);
const loading = ref(false);
const applying = ref(false);
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
    const controls = [...(root.value?.querySelectorAll<HTMLElement>('button:not(:disabled), summary, [tabindex="0"]') ?? [])];
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

async function accept() {
  const snapshot = review.value;
  if (!snapshot?.remote || applying.value || loading.value || error.value) return;
  request?.abort();
  const controller = new AbortController();
  request = controller;
  applying.value = true;
  dialog()?.focus({ preventScroll: true });
  try {
    const remote = await teamQuotes.acceptConflict(snapshot, controller.signal);
    if (!controller.signal.aborted) emit("accepted", remote);
  } catch (cause) {
    if (!controller.signal.aborted && !(cause instanceof DOMException && cause.name === "AbortError")) {
      error.value = cause instanceof Error ? cause.message : "Ortak sürüm seçilemedi. Tekrar deneyin.";
    }
  } finally {
    if (!controller.signal.aborted) applying.value = false;
  }
}

const currencyFormat = new Intl.NumberFormat("tr-TR", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
const numberFormat = new Intl.NumberFormat("tr-TR", { maximumFractionDigits: 6 });
const dateFormat = new Intl.DateTimeFormat("tr-TR", { dateStyle: "medium", timeZone: "UTC" });
const price = (value: number, currency: string) => `${currencyFormat.format(value)} ${currency}`;
function date(text: string | null) {
  const value = new Date(`${text?.slice(0, 10) ?? ""}T00:00:00Z`);
  return Number.isFinite(value.getTime()) ? dateFormat.format(value) : "—";
}

const facts: { label: string; get: (quote: Quote) => string }[] = [
  { label: "Teklif numarası", get: (quote) => quote.no },
  { label: "Durum", get: (quote) => quote.status_label },
  { label: "Müşteri", get: (quote) => quote.contact_name },
  { label: "Para birimi", get: (quote) => quote.currency },
  { label: "Kur", get: (quote) => quote.fx_rate == null ? "—" : numberFormat.format(quote.fx_rate) },
  { label: "Geçerlilik", get: (quote) => date(quote.valid_until) },
  { label: "Kalem sayısı", get: (quote) => numberFormat.format(quote.items.length) },
  { label: "Genel toplam", get: (quote) => price(quote.grand_total, quote.currency) },
  { label: "Not", get: (quote) => quote.note },
];
const comparison = computed(() => {
  const data = review.value;
  if (!data) return [];
  return facts.map(({ label, get }) => ({ label, local: get(data.local) || "—", remote: data.remote ? get(data.remote) || "—" : null }));
});
const sides = computed(() => {
  const data = review.value;
  if (!data) return [];
  const rows = [{ label: "Bu cihaz", quote: data.local }];
  if (data.remote) rows.push({ label: "Ortak sürüm", quote: data.remote });
  return rows;
});
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
          <div class="comparison-wrap">
            <table class="comparison" aria-label="Teklif sürümleri karşılaştırması">
              <thead>
                <tr>
                  <th scope="col">Alan</th>
                  <th v-for="{ label, quote } in sides" :key="label" scope="col">
                    {{ label }} <small v-if="quote.revision != null">Sürüm {{ quote.revision }}</small>
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in comparison" :key="row.label" :class="{ changed: row.remote !== null && row.local !== row.remote }">
                  <th scope="row">{{ row.label }}</th>
                  <td>{{ row.local }}</td>
                  <td v-if="row.remote !== null">{{ row.remote }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <details v-if="sides.some(({ quote }) => quote.items.length)" class="line-details">
            <summary>Teklif kalemleri</summary>
            <div class="line-sides">
              <section v-for="{ label, quote } in sides" :key="label">
                <h3>{{ label }}</h3>
                <ul v-if="quote.items.length">
                  <li v-for="item in quote.items" :key="item.id">
                    <strong>{{ item.name }}<span v-if="item.sku"> · {{ item.sku }}</span></strong>
                    <span>{{ numberFormat.format(item.qty) }} × {{ price(item.unit_price, quote.currency) }} · KDV %{{ numberFormat.format(item.tax_rate) }}</span>
                  </li>
                </ul>
                <p v-else>Kalem yok.</p>
              </section>
            </div>
          </details>
          <p v-if="review.remote && !error" class="review-state">
            Ortak sürümü seçerseniz bu teklife ait cihaz değişiklikleri eşitleme kuyruğundan çıkarılır.
          </p>
        </template>
        <template #footer>
          <button type="button" class="secondary-action" :disabled="applying" @click="close">Kapat</button>
          <button v-if="!loading && (!review?.remote || error)" type="button" class="primary-action" :disabled="applying" @click="refresh">Karşılaştırmayı yenile</button>
          <button v-if="review?.remote && !error" type="button" class="primary-action" :class="{ busy: applying }"
            :disabled="applying" @click="accept">
            {{ applying ? "Ortak sürüm doğrulanıyor…" : "Ortak sürümü kullan" }}
          </button>
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
.comparison-wrap {
  overflow: hidden;
  border: 1px solid var(--c-border);
  border-radius: 9px;
}
.comparison { width: 100%; table-layout: fixed; border-collapse: collapse; font-size: 13px; }
.comparison :is(th, td) { padding: 8px 10px; text-align: start; overflow-wrap: anywhere; }
.comparison thead { background: var(--c-hover); }
.comparison thead th:first-child { width: clamp(85px, 26%, 150px); }
.comparison small { display: block; color: var(--c-mid); font-size: 11px; font-weight: 500; }
.comparison tbody :is(th, td) { border-top: 1px solid var(--c-border); }
.comparison tbody th { color: var(--c-mid); font-size: 12px; font-weight: 550; }
.comparison tbody td { white-space: pre-wrap; }
.comparison .changed { background: var(--accent-tint); }
.line-details { padding: 9px 10px; border: 1px solid var(--c-border); border-radius: 9px; font-size: 13px; }
.line-details summary { color: var(--c-text); cursor: pointer; }
.line-sides { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 200px), 1fr)); gap: 8px 16px; }
.line-sides h3 { margin: 10px 0 3px; font-size: 12px; }
.line-sides ul { display: grid; gap: 4px; margin: 0; padding: 0; list-style: none; }
.line-sides li { padding: 5px 0; border-top: 1px solid var(--c-border-soft); overflow-wrap: anywhere; }
.line-sides li strong { display: block; }
.line-sides li span { color: var(--c-mid); font-size: 12px; }
@media (max-width: 520px) {
  .comparison :is(th, td) { padding: 6px; }
}
</style>
