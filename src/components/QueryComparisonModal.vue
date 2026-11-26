<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { compareTeamProductQuery, teamMeasurementScope } from "../teamGsc";
import type { QueryComparison, QueryDayState } from "../types";
import Icon from "./Icon.vue";
import ModalShell from "./ModalShell.vue";

const props = defineProps<{ sku: string; product: string; keyword: string }>();
const emit = defineEmits<{ close: [] }>();
const query = ref(props.keyword);
const modal = ref<HTMLElement | null>(null);
const queryInput = ref<HTMLInputElement | null>(null);
const comparison = ref<QueryComparison | null>(null);
const busy = ref<"compare" | "start" | "pause" | null>(null);
const queryError = ref("");
const trackToggle = ref<HTMLButtonElement | null>(null);
const dayStates: { state: QueryDayState; label: string }[] = [
  { state: "final", label: "Kesinleşti" },
  { state: "provisional", label: "Geçici" },
  { state: "missing", label: "Eksik" },
  { state: "pending", label: "Bekliyor" },
];
const stateLabel = Object.fromEntries(dayStates.map(({ state, label }) => [state, label]));
const periods = computed(() => {
  const result = comparison.value;
  if (!result?.before || !result.after) return [];
  return [{ label: "Önce", period: result.before }, { label: "Sonra", period: result.after }].map(({ label, period }) => {
    const observations = result.daily?.observations.filter(({ day }) => day >= period.start && day <= period.end) ?? [];
    const coverage = dayStates.map(({ state, label }) => ({ state, label, count: observations.filter((day) => day.state === state).length }));
    return { label, period, observations, coverage };
  });
});

let app: HTMLElement | null = null;
let wasInert = false;
let generation = 0;
onMounted(() => {
  app = document.getElementById("app");
  if (app) {
    wasInert = app.inert;
    app.inert = true;
  }
  modal.value?.querySelector('[role="dialog"]')?.setAttribute("aria-modal", "true");
  queryInput.value?.focus({ preventScroll: true });
});
onBeforeUnmount(() => {
  generation++;
  if (app) app.inert = wasInert;
});
watch([() => props.sku, teamMeasurementScope], () => {
  generation++;
  busy.value = null;
  clearResult();
}, { flush: "sync" });

function clearResult() {
  comparison.value = null;
  queryError.value = "";
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    emit("close");
    return;
  }
  if (event.key !== "Tab") return;
  const controls = Array.from(modal.value?.querySelectorAll<HTMLElement>("button:not(:disabled), input, summary") ?? []);
  const active = controls.indexOf(document.activeElement as HTMLElement);
  if (active >= 0 && (event.shiftKey ? active > 0 : active < controls.length - 1)) return;
  event.preventDefault();
  const target = event.shiftKey ? controls[controls.length - 1] : controls[0];
  target?.focus();
}

async function compareQuery(track?: boolean) {
  const term = query.value.trim();
  if (!term || busy.value) return;
  const request = ++generation;
  queryError.value = "";
  busy.value = track === undefined ? "compare" : track ? "start" : "pause";
  let changedTracking = false;
  try {
    const result = await compareTeamProductQuery(props.sku, term, track);
    if (request !== generation) return;
    comparison.value = result;
    changedTracking = track !== undefined;
  } catch (error) {
    if (request === generation) queryError.value = error instanceof Error ? error.message : String(error);
  } finally {
    if (request === generation) {
      busy.value = null;
      if (changedTracking) {
        await nextTick();
        if (request === generation) trackToggle.value?.focus({ preventScroll: true });
      }
    }
  }
}

const number = new Intl.NumberFormat("tr-TR", { maximumFractionDigits: 1 });
const percent = new Intl.NumberFormat("tr-TR", { style: "percent", maximumFractionDigits: 1 });
const dayLabel = new Intl.DateTimeFormat("tr-TR", { timeZone: "UTC", day: "numeric", month: "short" });
const formatDay = (value: string) => dayLabel.format(new Date(`${value}T00:00:00Z`));
</script>

<template>
  <Teleport to="body">
    <div ref="modal" @keydown.stop="handleKeydown">
      <ModalShell
        open
        scroll
        icon="chartLine"
        label="7 günlük sorgu karşılaştırması"
        title="7 günlük sorgu karşılaştırması"
        :sub="product"
        :width="730"
        @close="emit('close')"
      >
        <p class="query-note intro">Son mağaza işleminin öncesini ve sonrasını karşılaştırır. İşlem günü hariç tutulur.</p>
        <form class="query-form" :aria-busy="busy === 'compare'" @submit.prevent="compareQuery()">
          <input
            ref="queryInput"
            v-model="query"
            class="fx"
            aria-label="Karşılaştırılacak arama sorgusu"
            placeholder="Karşılaştırılacak arama sorgusu"
            maxlength="1024"
            :readonly="!!busy"
            @input="clearResult"
          />
          <button type="submit" class="primary-action" :disabled="!!busy || !query.trim()" :class="{ busy: busy === 'compare' }">
            <Icon :name="busy === 'compare' ? 'loader' : 'chartLine'" :size="15" :class="{ spin: busy === 'compare' }" aria-hidden="true" />
            {{ busy === 'compare' ? "Ölçüm alınıyor…" : "Karşılaştır" }}
          </button>
        </form>
        <p v-if="queryError" class="query-note query-error" role="alert"><Icon name="alert" :size="15" aria-hidden="true" />{{ queryError }}</p>
        <div v-if="comparison" class="query-results" aria-live="polite">
          <div v-if="comparison.daily" class="query-tracking">
            <div>
              <p class="query-note">Mağaza işlemi <time :datetime="comparison.event_day">{{ comparison.event_day }}</time> · Pasifik saati</p>
              <p v-if="comparison.daily.cached" class="query-note">Bu cihazda saklanan ölçüm gösteriliyor.</p>
              <p v-else-if="comparison.daily.refreshFailed" class="query-note">
                {{ comparison.daily.observations.some(({ state }) => state !== "pending")
                  ? "GSC şu anda yanıt vermiyor. Saklanan ölçümler gösteriliyor."
                  : "GSC şu anda yanıt vermiyor. Ölçümler bekleniyor." }}
              </p>
            </div>
            <div class="tracking-actions">
              <span v-if="comparison.daily.tracked" class="tracking-on">
                <Icon name="check" :size="15" aria-hidden="true" /> Günlük takip açık
              </span>
              <button v-if="comparison.daily.canTrack" ref="trackToggle" type="button" class="secondary-action" :disabled="!!busy" @click="compareQuery(!comparison.daily.tracked)">
                <Icon v-if="busy === 'start' || busy === 'pause'" name="loader" :size="15" class="spin" aria-hidden="true" />
                {{ busy === 'start' ? "Takibe alınıyor…" : busy === 'pause' ? "Takip durduruluyor…" : comparison.daily.tracked ? "Takibi durdur" : "Günlük takibe al" }}
              </button>
            </div>
          </div>
          <p v-if="!comparison.before || !comparison.after" class="query-note">
            İşlem tarihi {{ comparison.event_day }}. Takip penceresinin tamamlanması ve GSC verisinin kesinleşmesi bekleniyor.
          </p>
          <template v-else>
            <div class="query-periods">
              <section v-for="{ label, period, observations, coverage } in periods" :key="period.start" class="query-period" :aria-label="label">
                <header class="period-heading">
                  <h3>{{ label }}</h3>
                  <span>{{ formatDay(period.start) }} – {{ formatDay(period.end) }}</span>
                </header>
                <dl v-if="comparison.daily ? observations.some(({ state }) => state === 'final') : period.days">
                  <dt>Gösterim</dt><dd>{{ number.format(period.impressions) }}</dd>
                  <dt>Tıklama</dt><dd>{{ number.format(period.clicks) }}</dd>
                  <dt>CTR</dt><dd>{{ period.ctr === null ? "—" : percent.format(period.ctr) }}</dd>
                  <dt>Ortalama konum</dt><dd>{{ period.position === null ? "—" : number.format(period.position) }}</dd>
                </dl>
                <p v-else class="query-note">{{ comparison.daily ? "Bu dönem için kesinleşmiş ölçüm yok." : "Bu dönem için veri yok." }}</p>
                <template v-if="comparison.daily">
                  <ul class="coverage" aria-label="Günlük ölçüm durumları">
                    <li v-for="{ state, label: stateTitle, count } in coverage" :key="state" :class="state">
                      <i aria-hidden="true"></i><strong>{{ count }}</strong> {{ stateTitle }}
                    </li>
                  </ul>
                  <details class="daily-details">
                    <summary>Günlük ayrıntılar</summary>
                    <ul class="daily-observations">
                      <li v-for="sample in observations" :key="sample.day">
                        <div class="sample-heading">
                          <time :datetime="sample.day">{{ formatDay(sample.day) }}</time>
                          <span class="sample-state" :class="sample.state"><i aria-hidden="true"></i>{{ stateLabel[sample.state] }}</span>
                        </div>
                        <small v-if="sample.clicks !== null && sample.impressions !== null">
                          {{ number.format(sample.clicks) }} tıklama · {{ number.format(sample.impressions) }} gösterim
                          <template v-if="sample.position !== null"> · Konum {{ number.format(sample.position) }}</template>
                        </small>
                      </li>
                    </ul>
                  </details>
                </template>
                <span v-else-if="period.days">{{ period.days }}/7 gün veri var</span>
              </section>
            </div>
            <p class="query-note">
              {{ comparison.daily
                ? "Toplamlar kesinleşmiş ölçümleri içerir. Eksik günlerde Google sorgu satırı döndürmedi. Karşılaştırma erken bir sinyaldir."
                : "GSC bazı sorgu günlerini döndürmeyebilir. Bu karşılaştırma erken bir sinyal gösterir." }}
            </p>
          </template>
        </div>
      </ModalShell>
    </div>
  </Teleport>
</template>

<style scoped>
.query-note {
  margin: 0;
  color: var(--c-mid);
  font-size: 12px;
  line-height: 1.55;
}
.query-form, .query-tracking {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}
.query-form input {
  flex: 1 1 180px;
  min-height: 36px;
  min-width: 0;
  padding: 8px 12px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-input);
  color: var(--c-text);
}
.query-error, .tracking-on { display: flex; align-items: center; gap: 6px; }
.query-error { color: var(--red); }
.query-results { display: grid; gap: 12px; color: var(--c-text); }
.query-tracking { justify-content: space-between; align-items: center; }
.tracking-actions { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
.query-tracking time { font-weight: 650; color: var(--c-text); }
.tracking-on { color: var(--green); font-size: 12px; font-weight: 650; }
.query-periods {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(250px, 100%), 1fr));
  gap: 10px;
}
.query-period {
  display: grid;
  align-content: start;
  min-width: 0;
  padding: 12px;
  border: 1px solid var(--c-border);
  border-radius: 12px;
  background: var(--c-panel);
  gap: 10px;
  font-size: 12px;
}
.period-heading, .sample-heading { display: flex; align-items: center; justify-content: space-between; gap: 6px; flex-wrap: wrap; }
.period-heading h3 { margin: 0; font-size: 14px; }
.period-heading span, .query-period > span { color: var(--c-mid); }
.coverage, .daily-observations { list-style: none; padding: 0; margin: 0; }
.coverage { display: grid; grid-template-columns: repeat(2, 1fr); gap: 5px; }
.coverage li {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 5px 6px;
  border-radius: 6px;
  background: var(--c-card);
  color: var(--c-mid);
}
.coverage li strong { color: var(--c-text); font-variant-numeric: tabular-nums; }
.coverage i, .sample-state i { flex: 0 0 6px; width: 6px; height: 6px; border-radius: 50%; background: var(--c-faint); }
.final i { background: var(--green); }
.provisional i { background: var(--amber); }
.missing i { background: var(--red); }
.pending i { background: var(--accent); }
.daily-details { border-top: 1px solid var(--c-border); }
.daily-details summary { padding-top: 8px; font-weight: 650; cursor: pointer; }
.daily-observations { display: grid; gap: 7px; margin-top: 9px; }
.daily-observations li + li { border-top: 1px solid var(--c-border); padding-top: 7px; }
.sample-state { display: flex; gap: 5px; align-items: center; }
.daily-observations small { color: var(--c-mid); font-size: 11px; }
.query-period dl {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 4px;
  margin: 0;
}
.query-period dd {
  margin: 0;
  font-variant-numeric: tabular-nums;
}
:deep(.modal.scroll) { scrollbar-color: var(--c-faint) transparent; scrollbar-width: thin; }
</style>
