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
});

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
    },
    {
      page: "eol",
      icon: "archive",
      title: "Satışta olmayanlar",
      desc: "Google'da sıralanan ama katalogda olmayan sayfalar.",
      count: r.eol?.length ?? 0,
      cost: `${Math.round(r.eol_clicks ?? 0)} tıklama ölü sayfaya gidiyor`,
    },
    {
      page: "striking",
      icon: "trendUp",
      title: "Yükselmeye yakın",
      desc: "4–20. sıradaki aramalar; hedef kelime adayı.",
      count: r.striking?.length ?? 0,
      cost: "sorgu düzeyinde fırsat",
    },
    {
      page: "decay",
      icon: "trendDown",
      title: "Düşüşte olanlar",
      desc: "Önceki döneme göre gerileyen sayfalar.",
      count: r.decay?.length ?? 0,
      cost: decayLost ? `${decayLost} tıklama kaybedildi` : "kayıp yok",
    },
    {
      page: "cannibal",
      icon: "split",
      title: "Yarışan sayfalar",
      desc: "Aynı aramada birden çok sayfanız görünüyor.",
      count: r.cannibalization?.length ?? 0,
      cost: "elle inceleme gerekir",
    },
  ];
});

const fmtDate = (s: string) => s.replace("T", " ").slice(0, 16);
</script>

<template>
  <div class="page om-scroll">
    <div class="top">
      <div class="sum">
        <template v-if="report">
          <b>{{ missed }}</b> tıklama kaçırılıyor · <b>{{ report.eol?.length ?? 0 }}</b> sayfa
          satışta değil
          <span class="dim">
            · son {{ report.days }} gün · {{ report.matched }}/{{ report.total_products }} ürün
            Google'da bulundu · {{ fmtDate(report.analyzed_at) }}
          </span>
        </template>
        <span v-else class="dim">Henüz analiz çalıştırılmadı.</span>
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
      <button v-for="c in cards" :key="c.page" class="tool" @click="store.page = c.page">
        <div class="t-head">
          <div class="icon-badge"><Icon :name="c.icon" :size="15" /></div>
          <span class="t-count">{{ c.count }}</span>
        </div>
        <div class="t-title">{{ c.title }}</div>
        <div class="t-desc">{{ c.desc }}</div>
        <div class="t-cost">{{ c.cost }}</div>
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
.top {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 14px;
}
.sum {
  font-size: 12.5px;
  color: var(--c-text);
  line-height: 1.5;
}
.sum b {
  font-weight: 660;
  font-variant-numeric: tabular-nums;
}
.dim {
  color: var(--c-soft);
}
.run {
  margin-left: auto;
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
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
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

/* Beş kart. `max-width` bilinçli: sınırsız bırakılınca geniş ekranda 4+1 gibi dengesiz bir
   satır oluşuyordu; 3 sütunda 3+2 kasıtlı görünüyor. Ayrıca kartlar okunamayacak kadar
   genişlemiyor — açıklama satırı kısa kalıyor. */
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 12px;
  max-width: 900px;
}
.tool {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 14px 15px 13px;
  text-align: left;
  background: var(--c-card);
  border: 1px solid var(--c-border);
  border-radius: 13px;
  cursor: pointer;
  font-family: inherit;
  transition:
    border-color 0.16s cubic-bezier(0.32, 0.72, 0, 1),
    transform 0.16s cubic-bezier(0.32, 0.72, 0, 1);
}
.tool:hover {
  border-color: var(--accent);
  transform: translateY(-1px);
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
  color: var(--warn-text);
}
</style>
