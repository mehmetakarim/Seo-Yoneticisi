<script setup lang="ts">
/**
 * Teklif listesi (Faz T).
 *
 * Sıralama: **taslaklar önce, sonra gönderilenler**, sonra kapananlar — liste "hangi teklif
 * benden bir şey bekliyor" sorusuna göre dizili (Kişiler ekranındaki sonraki-adım sıralaması
 * ile aynı fikir).
 *
 * Kroma (`.list-panel`, `.search`, `.filters`) `styles.css`ten.
 */
import { computed, onMounted } from "vue";
import { useStore } from "../../store";
import Icon from "../Icon.vue";

const store = useStore();
// Teklif listesinde arama kutusu yok (sayı az). ⌘F sözleşmesi boş geçiliyor.
defineExpose({ focusSearch: () => {} });

onMounted(() => void store.loadQuotes());

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
    .format(v);

const acikToplam = computed(() =>
  store.quotes.filter((q) => q.status === "sent").reduce((t, q) => t + q.grand_total, 0),
);
const acikSayi = computed(() => store.quotes.filter((q) => q.status === "sent").length);
</script>

<template>
  <section class="list-panel">
    <div class="search-wrap">
      <div class="baslik">
        <b>{{ store.quotes.length }} teklif</b>
        <span v-if="acikSayi" class="acik">
          {{ acikSayi }} açık ·
          {{ para(acikToplam, store.quotes.find((q) => q.status === "sent")?.currency ?? "USD") }}
        </span>
      </div>
      <button class="eylem birincil" title="Yeni teklif" @click="store.createQuote(null, 'USD')">
        <Icon name="plus" :size="15" :stroke-width="2.4" />
      </button>
    </div>

    <div class="filters">
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

    <div class="rows om-scroll">
      <template v-if="store.quotes.length">
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
              <template v-if="q.version_count"> · v{{ q.version_count + 1 }}</template>
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
      </template>

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
