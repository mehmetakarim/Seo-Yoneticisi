<script setup lang="ts">
/**
 * Bugün — sabah açan kişinin ilk gördüğü ekran (Faz K).
 *
 * Diğer dokuz ekran "nerede ne var" sorusunu cevaplıyor; burası **"bugün ne yapayım"**
 * sorusunu cevaplıyor. Yeni veri üretmiyor: beş analiz, ölçüm omurgası ve feed bayrakları
 * zaten vardı — eksik olan tek şey **seçimdi** (2.226 adaydan 10 madde).
 *
 * ⚠️ **Tablo değil liste.** Araç ekranlarında satırlar birer veri noktası; burada birer iş.
 * Ama dil ortak: `SeoTable`'ın kabuk geometrisi (12px köşe, `--c-border-soft`, `--c-list`
 * şerit) ve global `.chip` aynen kullanılıyor — kopyalanmıyor.
 */
import { computed, onMounted, ref } from "vue";
import { useStore } from "../../store";
import type { Bucket, QueueItem } from "../../types";
import Icon from "../Icon.vue";
import ModalShell from "../ModalShell.vue";

const store = useStore();
const skorAcik = ref(false);

onMounted(() => {
  // Kuyruk mevcut önbellekten hesaplanıyor; önbellek yoksa önce onu yükle.
  if (!store.opportunity) void store.loadOpportunityCache();
  void store.loadToday();
});

const q = computed(() => store.today);
const items = computed(() => q.value?.items ?? []);

/** Kova rozetinin rengi — mevcut rozet token'ları, yeni renk uydurulmuyor. */
const TONE: Record<Bucket, string> = {
  urgent: "eksik",
  leverage: "uygun",
  leak: "bekliyor",
  review: "tamamlandi",
  upkeep: "hatali",
};
const LABEL: Record<Bucket, string> = {
  urgent: "Acil",
  leverage: "Yüksek kaldıraç",
  leak: "Kaçak trafik",
  review: "Sonuç kontrolü",
  upkeep: "Bakım",
};

/** Skorun nasıl çıktığı — yol haritasının şartı: formül kullanıcıya gösterilir. */
function skorMetni(it: QueueItem): string {
  if (it.bucket === "urgent")
    return `40 (acil tabanı) + ${Math.round(it.clicks)} tıklama = ${Math.round(it.score)}`;
  if (it.bucket === "review") return `sabit ${Math.round(it.score)} (kayıp değil, zamanı gelmiş kontrol)`;
  const w = it.bucket === "leverage" ? "1,0" : it.bucket === "upkeep" ? "0,8" : "0,6";
  return `${Math.round(it.clicks)} tıklama × ${w} = ${Math.round(it.score)}`;
}

const AGIRLIKLAR: { kova: string; agirlik: string; gerekce: string }[] = [
  { kova: "Yüksek kaldıraç", agirlik: "1,0", gerekce: "kaçırılan tıklama doğrudan hedef; meta/açıklama uygulamada üretiliyor" },
  { kova: "Bakım", agirlik: "0,8", gerekce: "kayıp gerçek ama sebebi dışsal olabilir (rakip, mevsim)" },
  { kova: "Kaçak trafik", agirlik: "0,6", gerekce: "tıklama gerçek, ama asıl çözüm 301 ve onu uygulama yapamıyor" },
  { kova: "Acil", agirlik: "40 + tıklama", gerekce: "tıklaması düşük olsa da canlıda yanlış içerik duruyor" },
  { kova: "Sonuç kontrolü", agirlik: "sabit 30", gerekce: "kayıp değil, zamanı gelmiş bir kontrol" },
];

/** Yarının tarihi (YYYY-AA-GG) — erteleme bu tarihe kadar. */
function yarin(): string {
  const d = new Date();
  d.setDate(d.getDate() + 1);
  return d.toISOString().slice(0, 10);
}

const bosKova = computed(() =>
  (q.value?.bucket_counts ?? []).filter((b) => b.candidates === 0),
);
const gunFarki = computed(() => {
  const a = q.value?.analyzed_at;
  if (!a) return -1;
  const d = new Date(a.slice(0, 10));
  return Math.round((Date.now() - d.getTime()) / 86400000);
});
</script>

<template>
  <div class="page om-scroll">
    <!-- Analiz hiç çalışmamışsa kuyruk kurulamaz; kullanıcıyı boş listeyle baş başa bırakma. -->
    <div v-if="!store.opportunity" class="empty">
      <Icon name="sun" :size="24" :stroke-width="1.9" />
      <p class="e-title">Kuyruk için önce analiz gerekiyor</p>
      <p class="e-sub">
        Bugün listesi Search Console analizinden besleniyor.
        <a class="link" @click="store.page = 'overview'">Genel Bakış</a>'tan çalıştırın.
      </p>
    </div>

    <template v-else>
      <!-- Üst şerit: kaç iş, veri ne kadar taze, skor nasıl hesaplanıyor. -->
      <div class="sum-card">
        <div class="strip">
          <div class="strip-main">
            <div class="metrics">
              <div class="metric">
                <span class="m-val">{{ items.length }}</span>
                <span class="m-lab">iş bugün için seçildi</span>
              </div>
              <span class="m-sep"></span>
              <div class="metric">
                <span class="m-val">{{ items.reduce((s, i) => s + i.minutes, 0) }}</span>
                <span class="m-lab">dakika (tahmini)</span>
              </div>
            </div>
            <div class="meta">
              Analiz {{ gunFarki <= 0 ? "bugün" : `${gunFarki} gün önce` }} çalıştı ·
              her kovadan en fazla 3 madde
              <!-- "gizli" değil "kuyruktan çıkarıldı": sayı hem kalıcı gizlenenleri hem
                   bugünlük ertelenenleri kapsıyor, ikisine birden "gizli" demek yanıltırdı. -->
              <template v-if="q && q.hidden">
                · {{ q.hidden }} madde kuyruktan çıkarıldı
                <a class="link" @click="store.restoreQueueItems()">geri getir</a>
              </template>
            </div>
          </div>
          <button class="run" @click="skorAcik = true">
            <Icon name="info" :size="13" /> Skor nasıl hesaplanıyor?
          </button>
        </div>
      </div>

      <!-- Kuyruk -->
      <div v-if="items.length" class="list">
        <div v-for="it in items" :key="it.reference.kind + it.reference.ref" class="item">
          <div class="i-head">
            <span class="badge" :class="`b-${TONE[it.bucket]}`">{{ LABEL[it.bucket] }}</span>
            <span class="i-title" :title="it.title">{{ it.title }}</span>
            <span class="i-score" :data-tip="skorMetni(it)">{{ Math.round(it.score) }}</span>
            <span class="i-min">≈{{ it.minutes }} dk</span>
          </div>
          <div class="i-why">{{ it.reason }}</div>
          <!-- Aynı ürünün diğer kovalardaki sebepleri. Ölçüm: 12 ürün 2+ kovada. -->
          <div v-if="it.also.length" class="i-also">
            <span v-for="(a, i) in it.also" :key="i" class="also-line">ayrıca: {{ a }}</span>
          </div>
          <div class="i-act">
            <button class="go" @click="store.openQueueItem(it.page, it.focus_id)">
              <Icon name="external" :size="12" /> Aç
            </button>
            <button
              class="ghost-sm"
              data-tip="Yarın tekrar göster"
              @click="store.dismissQueueItem(it.reference.kind, it.reference.ref, yarin())"
            >
              Bugünlük ertele
            </button>
            <button
              class="ghost-sm"
              data-tip="Bu maddeyi bir daha gösterme. Veri silinmez, ürün kendi ekranında durur."
              @click="store.dismissQueueItem(it.reference.kind, it.reference.ref, null)"
            >
              Gizle
            </button>
          </div>
        </div>
      </div>

      <div v-else-if="!store.todayBusy" class="empty">
        <Icon name="check" :size="24" :stroke-width="1.9" />
        <p class="e-title">Bugün için seçilecek iş yok</p>
        <p class="e-sub">
          Analizdeki tüm satırlar ya işlenmiş ya da gizlenmiş.
          <template v-if="q && q.hidden">
            {{ q.hidden }} madde kuyruktan çıkarılmış —
            <a class="link" @click="store.restoreQueueItems()">geri getirin</a>.
          </template>
        </p>
      </div>

      <!-- Boş kovalar: sessizce yok saymak yerine NEDEN boş olduğunu söylüyoruz. -->
      <div v-if="bosKova.length" class="bos">
        <Icon name="info" :size="12" />
        <span>
          Şu an aday çıkarmayan kova:
          <b v-for="(b, i) in bosKova" :key="b.bucket">{{ i ? " · " : "" }}{{ b.label }}</b>.
          <template v-if="q && q.review_ready_at">
            Sonuç kontrolü için gönderimlerin üstünden 28 gün geçmesi gerekiyor — en erken
            <b>{{ q.review_ready_at }}</b>.
          </template>
        </span>
      </div>
    </template>

    <!-- Skor formülü: yol haritasının şartı, kullanıcıya açıkça gösteriliyor. -->
    <ModalShell
      :open="skorAcik"
      label="Skor açıklaması"
      icon="info"
      title="Skor nasıl hesaplanıyor?"
      sub="Sıralama tek bir soruya dayanıyor"
      :width="560"
      @close="skorAcik = false"
    >
      <p class="mp">
        Skor <b>tıklama × kova ağırlığı</b>. Ağırlık keyfi değil; tek bir sorunun cevabı:
        <b>bu kaybın ne kadarı geri kazanılabilir ve uygulama bunu yapabilir mi?</b>
      </p>
      <div class="tbl">
        <div v-for="a in AGIRLIKLAR" :key="a.kova" class="t-row">
          <span class="t-k">{{ a.kova }}</span>
          <span class="t-a">{{ a.agirlik }}</span>
          <span class="t-g">{{ a.gerekce }}</span>
        </div>
      </div>
      <p class="mp">
        Ham skor sırası tek başına yeterli değil: ölçüldüğünde ilk 10 satışta olmayan
        sayfalara saplanıyor ve GSC fırsatları hiç görünmüyordu (en büyük kaçak sayfa 515
        tıklama, en büyük fırsat 37 kaçırılan tıklama). Bu yüzden <b>her kovadan en fazla
        3 madde</b> alınıyor.
      </p>
      <p class="mp faint">
        Süreler <b>tahmindir</b>, ölçüm değil — üretim süreleri henüz kaydedilmiyor.
      </p>
    </ModalShell>
  </div>
</template>

<style scoped>
.page {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 18px 22px 28px;
}

/* Üst şerit — Genel Bakış'taki `sum-card`/`strip` dilinin aynısı. */
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
  gap: 14px;
  padding: 13px 16px;
  background: var(--c-list);
}
.strip-main {
  flex: 1;
  min-width: 0;
}
.metrics {
  display: flex;
  align-items: baseline;
  gap: 10px;
}
.metric {
  display: flex;
  align-items: baseline;
  gap: 5px;
}
.m-val {
  font-size: 19px;
  font-weight: 640;
  color: var(--c-text);
  font-variant-numeric: tabular-nums;
}
.m-lab {
  font-size: 11.5px;
  color: var(--c-soft);
}
.m-sep {
  width: 1px;
  height: 14px;
  background: var(--c-border);
}
.meta {
  margin-top: 4px;
  font-size: 11px;
  color: var(--c-faint);
}
.run {
  flex: none;
  display: inline-flex;
  align-items: center;
  gap: 7px;
  height: 32px;
  padding: 0 13px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-card);
  color: var(--c-mid);
  font-size: 12px;
  font-weight: 560;
  cursor: pointer;
  font-family: inherit;
}
.run:hover {
  background: var(--c-hover);
}

/* Kuyruk maddeleri */
.list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.item {
  border: 1px solid var(--c-border-soft);
  border-radius: 12px;
  background: var(--c-card);
  padding: 12px 14px;
  transition: border-color 0.18s cubic-bezier(0.32, 0.72, 0, 1);
}
.item:hover {
  border-color: var(--c-border);
}
.i-head {
  display: flex;
  align-items: center;
  gap: 9px;
}
.badge {
  flex: none;
  padding: 2px 8px;
  border-radius: 6px;
  font-size: 10.5px;
  font-weight: 600;
  white-space: nowrap;
}
.b-eksik { background: var(--badge-eksik-bg); color: var(--badge-eksik-c); }
.b-uygun { background: var(--badge-uygun-bg); color: var(--badge-uygun-c); }
.b-bekliyor { background: var(--badge-bekliyor-bg); color: var(--badge-bekliyor-c); }
.b-hatali { background: var(--badge-hatali-bg); color: var(--badge-hatali-c); }
.b-tamamlandi { background: var(--badge-tamamlandi-bg); color: var(--badge-tamamlandi-c); }
.i-title {
  flex: 1;
  min-width: 0;
  font-size: 13px;
  font-weight: 560;
  color: var(--c-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.i-score {
  flex: none;
  font-size: 12.5px;
  font-weight: 640;
  color: var(--c-mid);
  font-variant-numeric: tabular-nums;
  cursor: help;
}
.i-min {
  flex: none;
  font-size: 11px;
  color: var(--c-faint);
  font-variant-numeric: tabular-nums;
}
.i-why {
  margin-top: 5px;
  font-size: 11.5px;
  color: var(--c-soft);
  line-height: 1.5;
}
.i-also {
  display: flex;
  flex-direction: column;
  gap: 1px;
  margin-top: 3px;
}
.also-line {
  font-size: 10.5px;
  color: var(--c-faint);
  line-height: 1.45;
}
.i-act {
  display: flex;
  gap: 7px;
  margin-top: 9px;
}
.go {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 28px;
  padding: 0 12px;
  border: none;
  border-radius: 8px;
  background: var(--accent);
  color: #fff;
  font-size: 11.5px;
  font-weight: 580;
  cursor: pointer;
  font-family: inherit;
}
.go:hover {
  filter: brightness(1.06);
}
.ghost-sm {
  height: 28px;
  padding: 0 11px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-input);
  color: var(--c-soft);
  font-size: 11.5px;
  font-weight: 540;
  cursor: pointer;
  font-family: inherit;
}
.ghost-sm:hover {
  background: var(--c-hover);
  color: var(--c-mid);
}

.bos {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin-top: 14px;
  padding: 10px 13px;
  border-radius: 9px;
  background: var(--c-list);
  border: 1px solid var(--c-border-soft);
  font-size: 11.5px;
  line-height: 1.55;
  color: var(--c-soft);
}

.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 56px 20px 24px;
  color: var(--c-faint);
}
.e-title {
  margin: 12px 0 0;
  font-size: 14.5px;
  font-weight: 600;
  color: var(--c-text);
}
.e-sub {
  margin: 6px 0 0;
  max-width: 420px;
  font-size: 12.5px;
  line-height: 1.6;
  color: var(--c-soft);
}
.link {
  color: var(--accent);
  cursor: pointer;
  font-weight: 560;
}

/* Skor modali */
.mp {
  margin: 0 0 12px;
  font-size: 12.5px;
  line-height: 1.6;
  color: var(--c-mid);
}
.mp.faint {
  margin-bottom: 0;
  font-size: 11.5px;
  color: var(--c-faint);
}
.tbl {
  margin-bottom: 12px;
  border: 1px solid var(--c-border-soft);
  border-radius: 9px;
  overflow: hidden;
}
.t-row {
  display: grid;
  /* Sabit izler: her satır kendi grid'i, içeriğe bağlı iz hizayı bozar (bkz. SeoTable). */
  grid-template-columns: 118px 84px 1fr;
  gap: 10px;
  padding: 8px 12px;
  font-size: 11.5px;
  border-bottom: 1px solid var(--c-border-soft);
}
.t-row:last-child {
  border-bottom: 0;
}
.t-k {
  font-weight: 600;
  color: var(--c-text);
}
.t-a {
  color: var(--c-mid);
  font-variant-numeric: tabular-nums;
}
.t-g {
  color: var(--c-soft);
  line-height: 1.45;
}
</style>
