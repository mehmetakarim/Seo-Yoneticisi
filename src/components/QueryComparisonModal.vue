<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { compareTeamProductQuery } from "../teamGsc";
import type { QueryComparison } from "../types";
import Icon from "./Icon.vue";
import ModalShell from "./ModalShell.vue";

const props = defineProps<{ sku: string; product: string; keyword: string }>();
const emit = defineEmits<{ close: [] }>();
const query = ref(props.keyword);
const queryInput = ref<HTMLInputElement | null>(null);
const comparison = ref<QueryComparison | null>(null);
const queryBusy = ref(false);
const queryError = ref("");

let app: HTMLElement | null = null;
onMounted(() => {
  app = document.getElementById("app");
  if (app) app.inert = true;
  queryInput.value?.focus();
});
onBeforeUnmount(() => {
  if (app) app.inert = false;
});

async function compareQuery() {
  const term = query.value.trim();
  if (!term || queryBusy.value) return;
  comparison.value = null;
  queryError.value = "";
  queryBusy.value = true;
  try {
    comparison.value = await compareTeamProductQuery(props.sku, term);
  } catch (error) {
    queryError.value = String(error);
  } finally {
    queryBusy.value = false;
  }
}

const number = new Intl.NumberFormat("tr-TR", { maximumFractionDigits: 1 });
const percent = new Intl.NumberFormat("tr-TR", { style: "percent", maximumFractionDigits: 1 });
</script>

<template>
  <Teleport to="body">
    <div @keydown.stop @keydown.esc.stop="emit('close')">
      <ModalShell
        open
        scroll
        icon="chartLine"
        label="7 günlük sorgu karşılaştırması"
        title="7 günlük sorgu karşılaştırması"
        :sub="product"
        :width="620"
        @close="emit('close')"
      >
        <p class="query-note intro">Son mağaza işleminin öncesini ve sonrasını karşılaştırır. İşlem günü hariç tutulur.</p>
        <form class="query-form" @submit.prevent="compareQuery">
          <input
            ref="queryInput"
            v-model="query"
            class="fx"
            aria-label="Karşılaştırılacak arama sorgusu"
            placeholder="Karşılaştırılacak arama sorgusu"
            :readonly="queryBusy"
            @input="comparison = null; queryError = ''"
          />
          <button type="submit" class="primary-action" :disabled="queryBusy || !query.trim()">
            <Icon :name="queryBusy ? 'loader' : 'chartLine'" :size="15" :class="{ spin: queryBusy }" />
            {{ queryBusy ? "Ölçüm alınıyor…" : "Karşılaştır" }}
          </button>
        </form>
        <p v-if="queryError" class="query-note" role="alert">{{ queryError }}</p>
        <div v-if="comparison" aria-live="polite">
          <p v-if="!comparison.before || !comparison.after" class="query-note">
            İşlem tarihi {{ comparison.event_day }}. Takip penceresinin tamamlanması ve GSC verisinin kesinleşmesi bekleniyor.
          </p>
          <template v-else>
            <div class="query-periods">
              <div v-for="(period, index) in [comparison.before, comparison.after]" :key="period.start" class="query-period">
                <strong>{{ index === 0 ? "Önce" : "Sonra" }}</strong>
                <span>{{ period.start }} – {{ period.end }}</span>
                <dl v-if="period.position !== null && period.ctr !== null">
                  <dt>Ortalama konum</dt><dd>{{ number.format(period.position) }}</dd>
                  <dt>Gösterim</dt><dd>{{ number.format(period.impressions) }}</dd>
                  <dt>Tıklama</dt><dd>{{ number.format(period.clicks) }}</dd>
                  <dt>CTR</dt><dd>{{ percent.format(period.ctr) }}</dd>
                </dl>
                <span>{{ period.days ? `${period.days}/7 gün veri var` : "Bu dönem için veri yok" }}</span>
              </div>
            </div>
            <p class="query-note">GSC bazı sorgu günlerini döndürmeyebilir. Bu karşılaştırma erken bir sinyal gösterir.</p>
          </template>
        </div>
      </ModalShell>
    </div>
  </Teleport>
</template>

<style scoped>
.query-note {
  color: var(--c-soft);
  font-size: 12px;
  line-height: 1.55;
}
.intro { margin-top: 0; }
.query-form, .query-periods {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.query-form input, .query-period {
  flex: 1 1 180px;
  padding: 8px 10px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-card);
  color: var(--c-text);
}
.query-periods {
  margin-top: 14px;
}
.query-period {
  display: grid;
  align-content: start;
  background: var(--c-panel);
  gap: 4px;
  font-size: 12px;
}
.query-period span {
  color: var(--c-soft);
}
.query-period dl {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 4px;
  margin: 6px 0;
}
.query-period dd {
  margin: 0;
  font-variant-numeric: tabular-nums;
}
</style>
