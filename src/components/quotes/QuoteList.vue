<script setup lang="ts">
/**
 * Teklif listesi (Faz T).
 *
 * Sıralama teklifin son işlemine göre yapılır. Durum süzgeci aynı sıraya uygulanır.
 *
 * Kroma (`.list-panel`, `.search`, `.filters`) `styles.css`ten.
 */
import { computed, onMounted, ref, watch } from "vue";
import { useStore } from "../../store";
import { quoteActivity } from "../../quoteOrder";
import { teamClient, teamSession, teamUser, teamWorkspace } from "../../teamConnection";
import { teamQuoteCanEdit, teamQuoteQueue, teamQuoteSaving, teamQuotesOffline } from "../../teamQuotes";
import Icon from "../Icon.vue";

const store = useStore();
// Teklif listesinde arama kutusu yok (sayı az). ⌘F sözleşmesi boş geçiliyor.
defineExpose({ focusSearch: () => {} });

onMounted(() => void store.loadQuotes());
watch([teamClient, teamSession, teamWorkspace], ([client, session, workspace], _, onCleanup) => {
  if (!client || !session || !workspace) return;
  const channel = client.channel(`quotes:${workspace}`).on("postgres_changes", {
    event: "*", schema: "public", table: "team_quotes", filter: `workspace_id=eq.${workspace}`,
  }, () => { void store.loadQuotes(); }).subscribe();
  const trashChannel = client.channel(`quotes-trash:${workspace}`).on("postgres_changes", {
    event: "*", schema: "public", table: "team_quote_trash_state", filter: `workspace_id=eq.${workspace}`,
  }, ({ new: state }) => { void store.observeQuoteTrash(state); }).subscribe();
  onCleanup(() => {
    void client.removeChannel(channel);
    void client.removeChannel(trashChannel);
  });
}, { immediate: true });

const DURUMLAR = [
  { key: "", label: "Tümü" },
  { key: "draft", label: "Taslak" },
  { key: "sent", label: "Gönderildi" },
  { key: "won", label: "Kazanıldı" },
  { key: "lost", label: "Kaybedildi" },
];

/** Rozet tonu — mevcut durum token'ları, yeni renk icat edilmiyor. */
const TON: Record<string, string> = {
  draft: "bekliyor",
  sent: "uygun",
  won: "tamamlandi",
  lost: "eksik",
  expired: "hatali",
};

const bugun = new Date().toISOString().slice(0, 10);
/** Gönderilmiş ve süresi geçmiş teklif — kapanmayı bekliyor. */
const suresiGecti = (q: { status: string; valid_until: string | null }) =>
  q.status === "sent" && !!q.valid_until && q.valid_until.slice(0, 10) < bugun;

const para = (v: number, c: string) =>
  new Intl.NumberFormat("tr-TR", { style: "currency", currency: c, maximumFractionDigits: 0 })
    .format(v === 0 ? 0 : v);

const acikTeklifler = computed(() => store.quotes.filter((quote) => quote.status === "sent"));
const acikToplamlar = computed(() => {
  const totals = new Map<string, number>();
  for (const quote of acikTeklifler.value) {
    totals.set(quote.currency, (totals.get(quote.currency) ?? 0) + quote.grand_total);
  }
  return [...totals].map(([currency, total]) => para(total, currency)).join(" · ");
});
const ozet = computed(() => store.quoteSummary);
const trashOpen = computed(() => !!teamUser.value && teamQuoteCanEdit.value && store.quoteTrashOpen);
const trashBusy = computed(() => store.quoteLifecycleBusy || teamQuotesOffline.value || teamQuoteSaving.value);
const dateFormat = new Intl.DateTimeFormat("tr-TR", { dateStyle: "medium", timeStyle: "short" });
const rowsPane = ref<HTMLElement | null>(null);

watch(() => store.quotes, (quotes, previous) => {
  const selected = store.quote;
  const newest = quotes[0];
  if (trashOpen.value || !selected || newest?.id !== selected.id || previous[0]?.id === selected.id) return;
  const previousRow = previous.find(({ id }) => id === selected.id);
  if (previousRow && quoteActivity(newest) > quoteActivity(previousRow)
    && quoteActivity(selected) === quoteActivity(newest) && rowsPane.value?.scrollTop) {
    rowsPane.value.scrollTop = 0;
  }
}, { flush: "post" });

function deletionDate(value: string) {
  const date = new Date(value);
  return Number.isFinite(date.getTime()) ? dateFormat.format(date) : "";
}
</script>

<template>
  <section class="list-panel">
    <div class="search-wrap">
      <div class="baslik">
        <b>{{ trashOpen ? "Silinenler" : `${store.quotes.length} teklif` }}</b>
        <span v-if="!trashOpen && acikTeklifler.length" class="acik">
          {{ acikTeklifler.length }} açık · {{ acikToplamlar }}
        </span>
      </div>
      <button v-if="teamUser && teamQuoteCanEdit" class="secondary-action trash-toggle" type="button"
        :disabled="trashOpen ? store.quoteLifecycleBusy : trashBusy" @click="store.setQuoteTrashOpen(!trashOpen)">
        <Icon :name="trashOpen ? 'fileEdit' : 'archive'" :size="14" aria-hidden="true" />
        {{ trashOpen ? "Tekliflere dön" : "Silinenler" }}
      </button>
      <button v-if="!trashOpen" class="eylem birincil" title="Yeni teklif" :disabled="!!teamUser && (!teamQuoteCanEdit || teamQuoteSaving || store.quoteLifecycleBusy)" @click="store.createQuote()">
        <Icon name="plus" :size="15" :stroke-width="2.4" />
      </button>
    </div>

    <div v-if="teamUser && (teamQuotesOffline || teamQuoteQueue.pending || teamQuoteQueue.conflicts)" class="ozet" role="status">
      <span v-if="teamQuotesOffline" class="o-sat">Çevrimdışı · Bu cihazdaki son kayıtlar</span>
      <span v-if="teamQuoteQueue.pending" class="o-sat">{{ teamQuoteQueue.pending }} işlem eşitlenmeyi bekliyor</span>
      <span v-if="teamQuoteQueue.conflicts" class="o-sat o-kay">{{ teamQuoteQueue.conflicts }} işlem eşitlenemedi</span>
    </div>

    <div v-if="!trashOpen" class="filters">
      <button
        v-for="d in DURUMLAR"
        :key="d.key"
        class="filter"
        :class="{ on: store.quoteStatusFilter === d.key }"
        @click="
          store.quoteStatusFilter = d.key;
          store.loadQuotes();
        "
      >
        {{ d.label }}
      </button>
    </div>

    <!-- Kazanma/kaybetme özeti: fazın bitiş şartı "kayıp nedeni raporlanabiliyor".
         Hiç kapanmış teklif yokken çizilmiyor — boş rapor gürültüdür. -->
    <div v-if="!trashOpen && ozet && (ozet.won_count || ozet.lost_count)" class="ozet">
      <div class="o-sat">
        <span class="o-kaz">{{ ozet.won_count }} kazanıldı</span>
        <span v-for="[c, v] in ozet.won_totals" :key="c" class="o-tut">{{ para(v, c) }}</span>
      </div>
      <div class="o-sat">
        <span class="o-kay">{{ ozet.lost_count }} kaybedildi</span>
      </div>
      <div v-if="ozet.lost_reasons.length" class="o-neden">
        <span v-for="[neden, n] in ozet.lost_reasons.slice(0, 4)" :key="neden" class="o-n">
          {{ neden }} <b>{{ n }}</b>
        </span>
      </div>
    </div>

    <div ref="rowsPane" class="rows om-scroll">
      <template v-if="trashOpen">
        <p v-if="store.quotesBusy" class="trash-state" role="status">Silinen teklifler yükleniyor…</p>
        <div v-else-if="store.quoteTrashError" class="trash-state" role="alert">
          <p>{{ store.quoteTrashError }}</p>
          <button class="secondary-action" type="button" :disabled="trashBusy" @click="store.loadQuotes()">Tekrar dene</button>
        </div>
        <ul v-else-if="store.quoteTrash.length" class="trash-list" aria-label="Silinen teklifler">
          <li v-for="quote in store.quoteTrash" :key="quote.id" class="trash-row">
            <div class="row-main">
              <div class="row-name" :title="quote.no">{{ quote.no }}</div>
              <time class="row-sub" :datetime="quote.deleted_at">{{ deletionDate(quote.deleted_at) }}</time>
            </div>
            <button class="secondary-action trash-restore" type="button" :aria-label="`${quote.no} teklifini geri yükle`"
              :disabled="trashBusy" @click="store.restoreQuote(quote.id)">Geri yükle</button>
          </li>
        </ul>
        <div v-else class="empty">Silinmiş teklif yok.</div>
      </template>
      <TransitionGroup v-else-if="store.quotes.length" name="quote-row" tag="div">
        <div
          v-for="q in store.quotes"
          :key="q.id"
          class="row"
          :class="{ sel: q.id === store.quote?.id }"
          @click="store.openQuote(q.id)"
        >
          <div class="row-main">
            <div class="row-name">
              {{ q.no }}
              <span class="badge" :class="`b-${TON[q.status] ?? 'bekliyor'}`">
                {{ q.status_label }}
              </span>
              <!-- Süresi geçmiş açık teklif: kapatılmayı bekleyen bir iş. -->
              <span v-if="suresiGecti(q)" class="gecti">süresi geçti</span>
            </div>
            <div class="row-sub">
              {{ q.contact_name || "kişi seçilmedi" }} · {{ q.items.length }} kalem
              <template v-if="q.version_count"> · v{{ q.version_count + (q.status === 'draft' ? 1 : 0) }}</template>
            </div>
          </div>
          <div class="tutar">
            <div class="t-ana">{{ para(q.grand_total, q.currency) }}</div>
            <!-- 🔴 Marj yalnızca burada; müşteriye giden çıktıda yok. -->
            <div v-if="q.margin" class="t-marj" :class="q.margin.state">
              %{{ q.margin.pct.toFixed(0) }}
            </div>
          </div>
        </div>
      </TransitionGroup>

      <div v-else-if="!store.quotesBusy" class="empty">
        <Icon name="fileEdit" :size="30" :stroke-width="1.6" style="margin-bottom: 10px" />
        <div>{{ store.quoteStatusFilter ? "Bu durumda teklif yok" : "Henüz teklif yok" }}</div>
        <p v-if="!store.quoteStatusFilter" class="e-alt">
          <b>+</b> ile teklif açın, satırları kataloğunuzdan ekleyin. Fiyat ve KDV ürünün
          kendi verisinden geliyor.
        </p>
      </div>
    </div>
  </section>
</template>

<style scoped>
.search-wrap {
  display: flex;
  align-items: center;
  gap: 8px;
}
.baslik {
  flex: 1;
  min-width: 0;
  font-size: 13px;
  color: var(--c-text);
}
.acik {
  display: block;
  font-size: 11.5px;
  color: var(--c-faint);
  margin-top: 2px;
}
.eylem {
  flex: none;
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--accent);
  border-radius: 9px;
  background: var(--accent);
  color: #fff;
  cursor: pointer;
}
.eylem:hover {
  opacity: 0.9;
}
.trash-toggle, .trash-restore { flex: none; padding: 7px 9px; font-size: 11.5px; }
.trash-list { padding: 0; margin: 0; list-style: none; }
.trash-row { display: flex; gap: 8px; align-items: center; padding: 11px 14px; border-bottom: 1px solid var(--c-border); }
.trash-state { margin: 0; padding: 14px; color: var(--c-mid); font-size: 12px; }
.trash-state p { margin: 0 0 8px; }
.ozet {
  padding: 10px 16px 12px;
  border-bottom: 1px solid var(--c-border);
  background: var(--c-list);
}
.o-sat {
  display: flex;
  align-items: baseline;
  gap: 8px;
  font-size: 12px;
  color: var(--c-faint);
}
.o-kaz {
  color: var(--badge-tamamlandi-c);
  font-weight: 560;
}
.o-kay {
  color: var(--badge-eksik-c);
  font-weight: 560;
}
.o-tut {
  font-variant-numeric: tabular-nums;
  color: var(--c-text);
}
/* Kayıp nedenleri: hangi sebebin kaç kez tekrarladığı tek bakışta görünsün. */
.o-neden {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin-top: 6px;
}
.o-n {
  font-size: 11px;
  color: var(--c-mid);
  background: var(--c-chip);
  border-radius: 20px;
  padding: 1px 8px;
}
.rows {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
}
.row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 11px 16px;
  border-bottom: 1px solid var(--c-border);
  cursor: pointer;
  transition: background 0.14s cubic-bezier(0.32, 0.72, 0, 1);
}
.quote-row-move {
  transition: transform var(--slide-duration) var(--slide-easing), background 0.14s cubic-bezier(0.32, 0.72, 0, 1);
}
@media (prefers-reduced-motion: reduce) {
  .quote-row-move { transition: none; }
}
.row:hover {
  background: var(--c-hover);
}
.row.sel {
  background: var(--accent-tint);
}
.row-main {
  flex: 1;
  min-width: 0;
}
.row-name {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13.5px;
  font-weight: 560;
  color: var(--c-text);
}
.row-sub {
  font-size: 11.5px;
  color: var(--c-faint);
  margin-top: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.badge {
  flex: none;
  padding: 1px 7px;
  border-radius: 20px;
  font-size: 10px;
  font-weight: 620;
}
.b-eksik { background: var(--badge-eksik-bg); color: var(--badge-eksik-c); }
.b-uygun { background: var(--badge-uygun-bg); color: var(--badge-uygun-c); }
.b-bekliyor { background: var(--badge-bekliyor-bg); color: var(--badge-bekliyor-c); }
.b-hatali { background: var(--badge-hatali-bg); color: var(--badge-hatali-c); }
.b-tamamlandi { background: var(--badge-tamamlandi-bg); color: var(--badge-tamamlandi-c); }
.gecti {
  font-size: 10px;
  color: var(--warn-text);
  font-weight: 560;
}
.tutar {
  flex: none;
  text-align: right;
}
.t-ana {
  font-size: 13px;
  font-weight: 620;
  color: var(--c-text);
  font-variant-numeric: tabular-nums;
}
.t-marj {
  font-size: 11px;
  font-weight: 560;
  color: var(--c-faint);
  font-variant-numeric: tabular-nums;
}
.t-marj.low {
  color: var(--warn-text);
}
.t-marj.negative {
  color: var(--badge-eksik-c);
}
.empty {
  padding: 44px 24px;
  text-align: center;
  color: var(--c-faint);
  font-size: 13px;
}
.e-alt {
  margin: 8px auto 0;
  max-width: 250px;
  font-size: 12px;
  line-height: 1.55;
}
</style>
