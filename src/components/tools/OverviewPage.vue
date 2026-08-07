<script setup lang="ts">
/**
 * SEO araçlarının giriş noktası: analizi buradan çalıştırırsınız, sonucun özetini burada
 * görürsünüz, hangi araca gideceğinize buradan karar verirsiniz.
 *
 * Neden ayrı bir ekran: beş analiz tek sayfada alt alta duruyordu ve kullanıcı hepsine tek
 * bir uzun kaydırmayla ulaşıyordu ("inanılmaz uzun ve dağınık"). Araçlar kendi ekranlarına
 * taşınınca ortak bir başlangıç noktası gerekti — yoksa "analizi nereden çalıştırıyordum?"
 * sorusu doğardı.
 *
 * ⚠️ Tek `analyze_opportunities` çağrısı ve tek `opportunity_json` önbelleği. Araç ekranları
 * aynı raporun dilimlerini okur; ekran başına GSC çağrısı YOK.
 */
import { computed, onMounted } from "vue";
import { useStore } from "../../store";
import type { Page } from "../../navigation";
import Icon from "../Icon.vue";

const store = useStore();
const report = computed(() => store.opportunity);

onMounted(() => {
  if (!store.opportunity) void store.loadOpportunityCache();
  void store.loadOutcomes();
});

const sonuc = computed(() => store.outcomeSummary);
/** Henüz hiç dönem yoksa kullanıcıya "geçmişi getir" düğmesi gösteriliyor. */
const gecmisVar = computed(() => (sonuc.value?.snapshots ?? 0) > 0);

const missed = computed(() =>
  Math.round((report.value?.opportunities ?? []).reduce((s, o) => s + o.missed_clicks, 0)),
);

/**
 * Araç kartları. `value` bir SAYIM değil, o aracın **bedeli** — kaç tıklama söz konusu.
 * Kullanıcı "hangi araca önce bakayım" sorusunu sayıya değil kayba bakarak cevaplasın.
 */
interface Card {
  page: Page;
  icon: string;
  title: string;
  desc: string;
  count: number;
  cost: string;
  /**
   * ⚠️ Bedel her zaman kayıp değil. Eskiden beşinin de rengi kehribardı; "elle inceleme
   * gerekir" bir uyarı değil, yalnızca bir not. Renk anlam taşımalı, süs olmamalı.
   */
  tone: "loss" | "note";
}

const cards = computed<Card[]>(() => {
  const r = report.value;
  if (!r) return [];
  const decayLost = Math.round((r.decay ?? []).reduce((a, d) => a + d.clicks_lost, 0));
  return [
    {
      page: "opportunities",
      icon: "search",
      title: "Fırsatlar",
      desc: "Konumunun getirmesi gereken tıklamayı alamayan ürünler.",
      count: r.opportunities?.length ?? 0,
      cost: `${missed.value} tıklama kaçıyor`,
      tone: "loss",
    },
    {
      page: "eol",
      icon: "archive",
      title: "Satışta olmayanlar",
      desc: "Google'da sıralanan ama katalogda olmayan sayfalar.",
      count: r.eol?.length ?? 0,
      cost: `${Math.round(r.eol_clicks ?? 0)} tıklama ölü sayfaya gidiyor`,
      tone: "loss",
    },
    {
      page: "striking",
      icon: "trendUp",
      title: "Yükselmeye yakın",
      desc: "4–20. sıradaki aramalar; hedef kelime adayı.",
      count: r.striking?.length ?? 0,
      cost: "sorgu düzeyinde fırsat",
      tone: "note",
    },
    {
      page: "decay",
      icon: "trendDown",
      title: "Düşüşte olanlar",
      desc: "Önceki döneme göre gerileyen sayfalar.",
      count: r.decay?.length ?? 0,
      cost: decayLost ? `${decayLost} tıklama kaybedildi` : "kayıp yok",
      tone: decayLost ? "loss" : "note",
    },
    {
      page: "cannibal",
      icon: "split",
      title: "Yarışan sayfalar",
      desc: "Aynı aramada birden çok sayfanız görünüyor.",
      count: r.cannibalization?.length ?? 0,
      cost: "elle inceleme gerekir",
      tone: "note",
    },
  ];
});

const fmtDate = (s: string) => s.replace("T", " ").slice(0, 16);
</script>

<template>
  <div class="page om-scroll">
    <!-- Özet şeridi: tablo bileşenindeki `.strip` ile aynı dil (kart kabuğu + --c-list zemin).
         Faz B'de tablolar bu dile geçti, bu ekran atlanmıştı. -->
    <div class="sum-card">
      <div class="strip">
        <div class="strip-main">
          <template v-if="report">
            <div class="metrics">
              <div class="metric">
                <span class="m-val loss">{{ missed }}</span>
                <span class="m-lab">tıklama kaçırılıyor</span>
              </div>
              <span class="m-sep"></span>
              <div class="metric">
                <span class="m-val loss">{{ report.eol?.length ?? 0 }}</span>
                <span class="m-lab">sayfa satışta değil</span>
              </div>
            </div>
            <div class="meta">
              son {{ report.days }} gün · {{ report.matched }}/{{ report.total_products }} ürün
              Google'da bulundu · {{ fmtDate(report.analyzed_at) }}
            </div>
          </template>
          <span v-else class="meta">Henüz analiz çalıştırılmadı.</span>
        </div>
        <button class="run" :disabled="store.opportunityBusy" @click="store.runOpportunityAnalysis()">
          <Icon
            :name="store.opportunityBusy ? 'loader' : 'refresh'"
            :size="15"
            :class="{ spin: store.opportunityBusy }"
          />
          {{ store.opportunityBusy ? "Analiz ediliyor…" : report ? "Yenile" : "Analizi çalıştır" }}
        </button>
      </div>
    </div>

    <!-- Sonuçlar: "yaptığımız iş işe yaradı mı?" — ölçüm omurgası (Faz Ö) -->
    <div class="sum-card sonuc-card">
      <div class="strip">
        <div class="strip-main">
          <template v-if="gecmisVar && sonuc">
            <div class="metrics">
              <div class="metric">
                <span class="m-val">{{ sonuc.measured_events }}</span>
                <span class="m-lab">gönderim izleniyor</span>
              </div>
              <span class="m-sep"></span>
              <div class="metric">
                <span class="m-val" :class="{ ok: sonuc.improved > 0 }">{{ sonuc.improved }}</span>
                <span class="m-lab">iyileşti</span>
              </div>
              <div class="metric">
                <span class="m-val">{{ sonuc.flat }}</span>
                <span class="m-lab">değişmedi</span>
              </div>
              <div class="metric">
                <span class="m-val" :class="{ loss: sonuc.worse > 0 }">{{ sonuc.worse }}</span>
                <span class="m-lab">geriledi</span>
              </div>
              <span class="m-sep"></span>
              <div class="metric">
                <span class="m-val">{{ sonuc.measuring }}</span>
                <span class="m-lab">ölçülüyor</span>
              </div>
            </div>
            <div class="meta">
              <!-- ⚠️ Nedensellik iddia edilmiyor: "sayesinde" değil, "sonrasında". -->
              {{ sonuc.snapshots }} dönem kayıtlı ({{ sonuc.oldest_window }} tarihinden beri) ·
              gönderim sonrası net
              <b :class="sonuc.net_delta_clicks >= 0 ? 'ok' : 'loss'">
                {{ sonuc.net_delta_clicks >= 0 ? "+" : "" }}{{ Math.round(sonuc.net_delta_clicks) }}
              </b>
              tıklama · etkinin görünmesi gönderimden sonra en az 21 gün alır
            </div>
          </template>
          <template v-else>
            <div class="metrics">
              <div class="metric"><span class="m-lab strong">Sonuç takibi kapalı</span></div>
            </div>
            <div class="meta">
              Search Console geçmişini bir kez getirin — yaptığınız gönderimlerin öncesi ve
              sonrası karşılaştırılabilsin. Son 12 ay çekilir, yaklaşık yarım dakika sürer.
            </div>
          </template>
        </div>
        <button
          v-if="!gecmisVar"
          class="run"
          :disabled="store.seedBusy"
          @click="store.seedMetricHistory()"
        >
          <Icon
            :name="store.seedBusy ? 'loader' : 'chartLine'"
            :size="15"
            :class="{ spin: store.seedBusy }"
          />
          {{ store.seedBusy ? "Geçmiş getiriliyor…" : "Geçmişi getir" }}
        </button>
      </div>
    </div>

    <div v-if="store.opportunityError" class="err">
      <Icon name="alert" :size="14" />
      <span>{{ store.opportunityError }}</span>
    </div>

    <!-- Hiç analiz yoksa: aracın ne yaptığını anlat, sonra çalıştırsın. -->
    <div v-if="!report && !store.opportunityBusy" class="empty">
      <Icon name="search" :size="30" :stroke-width="1.5" />
      <p class="e-title">Emeğinizi nereye harcayacağınızı Google söylesin</p>
      <p class="e-sub">
        Search Console verisiyle hangi ürünlerin gösterim alıp tıklanmadığını, hangilerinin
        ikinci sayfada takıldığını sıralar. Tek bir sorgu ile tüm katalog taranır ve beş
        aracın tamamı bu tek analizden beslenir.
      </p>
    </div>

    <!-- Araç kartları: hangi araca önce bakılacağı KAYBA göre okunur, sayıya göre değil. -->
    <div v-else-if="report" class="grid">
      <button
        v-for="(c, i) in cards"
        :key="c.page"
        class="tool"
        :class="{ wide: i < 2 }"
        @click="store.page = c.page"
      >
        <div class="t-head">
          <div class="icon-badge"><Icon :name="c.icon" :size="15" /></div>
          <span class="t-count">{{ c.count }}</span>
        </div>
        <div class="t-title">{{ c.title }}</div>
        <div class="t-desc">{{ c.desc }}</div>
        <div class="t-cost" :class="c.tone">{{ c.cost }}</div>
      </button>
    </div>
  </div>
</template>

<style scoped>
.page {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 18px 22px 28px;
}

/* ---- özet şeridi: SeoTable'ın kabuk + strip dilinin aynısı ---- */
.sum-card {
  border: 1px solid var(--c-border-soft);
  border-radius: 12px;
  background: var(--c-card);
  overflow: hidden;
  margin-bottom: 14px;
}
.strip {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 12px 14px;
  background: var(--c-list);
}
.strip-main {
  flex: 1;
  min-width: 0;
}
.metrics {
  display: flex;
  align-items: baseline;
  flex-wrap: wrap;
  gap: 10px;
}
.metric {
  display: flex;
  align-items: baseline;
  gap: 6px;
}
.m-val {
  font-size: 19px;
  font-weight: 680;
  letter-spacing: -0.02em;
  font-variant-numeric: tabular-nums;
  color: var(--c-text);
}
.m-val.loss {
  color: var(--red);
}
.m-val.ok {
  color: var(--green);
}
.meta b.ok {
  color: var(--green);
}
.meta b.loss {
  color: var(--red);
}
.m-lab.strong {
  font-weight: 620;
  color: var(--c-text);
}
/* Sonuç şeridi özet şeridinin kardeşi; ayrı kart olması bilinçli — biri "bugün ne var",
   diğeri "dün ne yaptık, ne oldu" sorusunu cevaplıyor. */
.sonuc-card {
  margin-bottom: 14px;
}
.m-lab {
  font-size: 12.5px;
  color: var(--c-mid);
}
.m-sep {
  width: 1px;
  height: 14px;
  background: var(--c-border);
  align-self: center;
}
.meta {
  margin-top: 4px;
  font-size: 11.5px;
  color: var(--c-soft);
  line-height: 1.5;
}
.run {
  flex: none;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 36px;
  padding: 0 16px;
  border: none;
  border-radius: 9px;
  background: var(--accent);
  color: #fff;
  font-size: 13px;
  font-weight: 590;
  cursor: pointer;
  font-family: inherit;
}
.run:hover:not(:disabled) {
  filter: brightness(1.06);
}
.run:disabled {
  opacity: 0.8;
  cursor: default;
}

.err {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 10px 12px;
  margin-bottom: 14px;
  font-size: 12px;
  line-height: 1.5;
  color: var(--warn-text);
  background: var(--warn-bg);
  border: 1px solid var(--warn-border);
  border-radius: 9px;
}
.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 60px 20px;
  color: var(--c-soft);
  text-align: center;
}
.e-title {
  margin: 10px 0 0;
  font-size: 15px;
  font-weight: 620;
  color: var(--c-text);
}
.e-sub {
  margin: 0;
  max-width: 460px;
  font-size: 12.5px;
  line-height: 1.6;
}

/* ---- araç kartları ----
   Düzen 2+3: ilk iki araç (Fırsatlar · Satışta olmayanlar) gerçek tıklama kaybı taşıyor,
   geniş yuvayı onlar alıyor. 6 sütunluk ızgara boşluk bırakmadan 2+3'e bölünüyor —
   `auto-fill` ile 4+1 gibi dengesiz satırlar oluşuyordu.
   Kart kabuğu tablo kabuğuyla aynı: 12px yarıçap, --c-border-soft, --c-card. */
.grid {
  display: grid;
  grid-template-columns: repeat(6, minmax(0, 1fr));
  gap: 12px;
}
.tool {
  grid-column: span 2;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 14px 15px 13px;
  text-align: left;
  background: var(--c-card);
  border: 1px solid var(--c-border-soft);
  border-radius: 12px;
  cursor: pointer;
  font-family: inherit;
  transition:
    border-color 0.18s cubic-bezier(0.32, 0.72, 0, 1),
    background 0.18s cubic-bezier(0.32, 0.72, 0, 1);
}
.tool.wide {
  grid-column: span 3;
}
.tool:hover {
  border-color: var(--accent);
  background: var(--c-hover);
}
/* Dar pencerede 2+3 anlamını yitiriyor; tek sütuna iniyor. */
@media (max-width: 900px) {
  .grid {
    grid-template-columns: 1fr;
  }
  .tool,
  .tool.wide {
    grid-column: span 1;
  }
}
.t-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
}
.t-count {
  font-size: 19px;
  font-weight: 680;
  color: var(--c-text);
  font-variant-numeric: tabular-nums;
  letter-spacing: -0.02em;
}
.t-title {
  font-size: 13.5px;
  font-weight: 620;
  color: var(--c-text);
}
.t-desc {
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--c-soft);
}
.t-cost {
  margin-top: 6px;
  font-size: 11px;
  font-weight: 600;
}
/* Renk anlam taşır: gerçek kayıp kırmızı, bilgi notu sessiz. */
.t-cost.loss {
  color: var(--red);
}
.t-cost.note {
  color: var(--c-soft);
}
</style>
