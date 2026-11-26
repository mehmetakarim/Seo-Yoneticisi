<script setup lang="ts">
import { computed } from "vue";
import type { Quote } from "../../types";

const props = defineProps<{
  local: Quote;
  remote: Quote | null;
  expandedLines?: boolean;
}>();

const currencyFormat = new Intl.NumberFormat("tr-TR", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
const numberFormat = new Intl.NumberFormat("tr-TR", { maximumFractionDigits: 6 });
const dateFormat = new Intl.DateTimeFormat("tr-TR", { dateStyle: "medium", timeZone: "UTC" });
const price = (value: number, currency: string) => Number.isFinite(value) ? `${currencyFormat.format(value)} ${currency}`.trim() : "—";
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
  { label: "Kur tarihi", get: (quote) => date(quote.fx_date) },
  { label: "Geçerlilik", get: (quote) => date(quote.valid_until) },
  { label: "Kalem sayısı", get: (quote) => numberFormat.format(quote.items.length) },
  { label: "Genel toplam", get: (quote) => price(quote.grand_total, quote.currency) },
  { label: "Not", get: (quote) => quote.note },
];
const comparison = computed(() => facts.map(({ label, get }) => ({
  label, local: get(props.local) || "—", remote: props.remote ? get(props.remote) || "—" : null,
})));
const sides = computed(() => {
  const versions = [{ label: "Bu cihaz", quote: props.local }];
  if (props.remote) versions.push({ label: "Ortak sürüm", quote: props.remote });
  return versions;
});
</script>

<template>
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
  <details v-if="sides.some(({ quote }) => quote.items.length)" class="line-details" :open="expandedLines">
    <summary>Teklif kalemleri</summary>
    <div class="line-sides">
      <section v-for="{ label, quote } in sides" :key="label">
        <h3>{{ label }}</h3>
        <ul v-if="quote.items.length">
          <li v-for="item in quote.items" :key="item.id">
            <strong>{{ item.name }}<span v-if="item.sku"> · {{ item.sku }}</span></strong>
            <span>{{ numberFormat.format(item.qty) }} × {{ price(item.unit_price, quote.currency) }} · KDV %{{ numberFormat.format(item.tax_rate) }}</span>
            <span v-if="item.cost != null"> · Maliyet {{ price(item.cost, quote.currency) }}</span>
          </li>
        </ul>
        <p v-else>Kalem yok.</p>
      </section>
    </div>
  </details>
</template>

<style scoped>
.comparison-wrap { overflow: hidden; border: 1px solid var(--c-border); border-radius: 9px; }
.comparison { width: 100%; table-layout: fixed; border-collapse: collapse; color: var(--c-text); font-size: 13px; }
.comparison :is(th, td) { padding: 8px 10px; text-align: start; overflow-wrap: anywhere; }
.comparison thead { background: var(--c-hover); }
.comparison thead th:first-child { width: clamp(85px, 26%, 150px); }
.comparison small { display: block; color: var(--c-mid); font-size: 11px; font-weight: 500; }
.comparison tbody :is(th, td) { border-top: 1px solid var(--c-border); }
.comparison tbody th { color: var(--c-mid); font-size: 12px; font-weight: 550; }
.comparison tbody td { white-space: pre-wrap; }
.comparison .changed { background: var(--accent-tint); }
.line-details { padding: 9px 10px; border: 1px solid var(--c-border); border-radius: 9px; color: var(--c-text); font-size: 13px; }
.line-details summary { cursor: pointer; }
.line-details summary:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; border-radius: 2px; }
.line-sides { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 200px), 1fr)); gap: 8px 16px; }
.line-sides h3 { margin: 10px 0 3px; font-size: 12px; }
.line-sides ul { display: grid; gap: 4px; margin: 0; padding: 0; list-style: none; }
.line-sides li { padding: 5px 0; border-top: 1px solid var(--c-border-soft); overflow-wrap: anywhere; }
.line-sides li strong { display: block; }
.line-sides li span { color: var(--c-mid); font-size: 12px; }
@media (max-width: 520px) { .comparison :is(th, td) { padding: 6px; } }
</style>
