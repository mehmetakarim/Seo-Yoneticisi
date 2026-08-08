<script setup lang="ts">
/**
 * Bugün — sabah açan kişinin ilk gördüğü ekran (Faz K).
 *
 * Diğer dokuz ekran "nerede ne var" sorusunu cevaplıyor; burası **"bugün ne yapayım"**
 * sorusunu cevaplıyor. Yeni veri üretmiyor: beş analiz, ölçüm omurgası ve feed bayrakları
 * zaten vardı — eksik olan tek şey **seçimdi** (2.226 adaydan 10 madde).
 *
 * Tasarım kaynağı: `design/faz-k-mevcut/E-ticaret SEO yönetim aracı-handoff.zip`
 * ("Bugun Ekrani"). Ölçüler oradan birebir: madde dolgusu 14/18px, ayırıcı 1px, başlık
 * 14px/620, skor 17px/660, skor çubuğu 72×4, ilerleme çubuğu 132×6, geçiş 160ms.
 *
 * ⚠️ **Kart çerçevesi YOK.** İlk sürümde her madde ayrı bir karttı; tasarım turunda maddeler
 * tek yüzeye alınıp aralarına ince çizgi kondu — göz önce başlığı, sonra nedeni görüyor.
 *
 * 🔴 **"Yapıldı" maddeyi listeden DÜŞÜRMÜYOR** (saha geri bildirimi, 2026-08-08).
 * İlk sürümde düşürüyordu ve yerine 11. aday geliyordu: sayaç hep 10'da kalıyor, gün hiç
 * bitmiyordu — *"bu mantık ile günlük iş hiç bitmez"*. Artık madde yerinde kalıyor, üstü
 * çiziliyor ve "4 / 10 bitti" ilerlemesi sayılabiliyor.
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
const bitti = computed(() => q.value?.done_count ?? 0);

/** Kova rozeti — mevcut rozet token'ları; tasarım da bu paletten seçmiş. */
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

/** Skor çubuğu = skor ÷ listenin en yükseği. Sayı tek başına anlamsız (309 mu iyi, 37 mi?). */
const enYuksek = computed(() => Math.max(1, ...items.value.map((i) => i.score)));
const cubukYuzde = (it: QueueItem) => Math.max(6, Math.round((it.score / enYuksek.value) * 100));

/** Skorun nasıl çıktığı — yol haritasının şartı: formül kullanıcıya gösterilir. */
function skorMetni(it: QueueItem): string {
  if (it.bucket === "urgent")
    return `40 (acil tabanı) + ${Math.round(it.clicks)} tıklama = ${Math.round(it.score)}`;
  if (it.bucket === "review")
    return `sabit ${Math.round(it.score)} (kayıp değil, zamanı gelmiş kontrol)`;
  const w = it.bucket === "leverage" ? "1,0" : it.bucket === "upkeep" ? "0,8" : "0,6";
  return `${Math.round(it.clicks)} tıklama × ${w} = ${Math.round(it.score)}`;
}

const AGIRLIKLAR = [
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

const bosKova = computed(() => (q.value?.bucket_counts ?? []).filter((b) => b.candidates === 0));
const gunFarki = computed(() => {
  const a = q.value?.analyzed_at;
  if (!a) return -1;
  return Math.round((Date.now() - new Date(a.slice(0, 10)).getTime()) / 86400000);
});
</script>

<template>
  <div class="page om-scroll">
    <!-- Analiz hiç çalışmamışsa kuyruk kurulamaz; kullanıcıyı boş listeyle baş başa bırakma. -->
    <div v-if="!store.opportunity" class="empty">
      <Icon name="sun" :size="24" :stroke-width="1.9" />
      <p class="e-title">Analiz henüz çalışmadı</p>
      <p class="e-sub">
        Kuyruk, Search Console verisi ile ürün feed'inin karşılaştırılmasından çıkıyor. İlk
        analizi çalıştırdığınızda bugünün işleri burada listelenecek.
      </p>
      <button class="e-btn" @click="store.page = 'overview'">Analizi çalıştır</button>
    </div>

    <template v-else>
      <!-- Üst şerit: kaç iş, ne kadar sürer, ne kadarı bitti. -->
      <div class="strip">
        <div class="s-main">
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
            <!-- Günün BİTEBİLDİĞİNİ gösteren tek işaret. -->
            <div v-if="items.length" class="prog">
              <span class="p-lab">{{ bitti }} / {{ items.length }} bitti</span>
              <span class="p-bar"
                ><span class="p-fill" :style="{ width: (bitti / items.length) * 100 + '%' }"></span
              ></span>
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
        <div class="s-btns">
          <!-- Seans çubuğu kabukta yaşıyor; buradaki düğme yalnızca başlatıyor. -->
          <button
            v-if="items.length && !store.session?.session_id"
            class="seans-btn"
            data-tip="Kuyruktan tek iş kilitlenir, süre ölçülür. Oyunlaştırma yok."
            @click="store.startSession()"
          >
            <Icon name="clock" :size="13" /> Odak seansı başlat
          </button>
          <button class="skor-btn" @click="skorAcik = true">
            <Icon name="info" :size="13" /> Skor nasıl hesaplanıyor?
          </button>
        </div>
      </div>

      <!-- Kuyruk: tek yüzey, aralarında ince çizgi (tasarım kararı — kart çerçevesi yok). -->
      <div v-if="items.length" class="list">
        <div
          v-for="it in items"
          :key="it.reference.kind + it.reference.ref"
          class="item"
          :class="{ done: it.done }"
        >
          <div class="i-main">
            <div class="i-head">
              <span class="badge" :class="`b-${TONE[it.bucket]}`">{{ LABEL[it.bucket] }}</span>
              <span class="i-title" :title="it.title">{{ it.title }}</span>
            </div>
            <div class="i-why">{{ it.reason }}</div>
            <!-- Aynı ürünün diğer kovalardaki sebepleri. Ölçüm: 12 ürün 2+ kovada. -->
            <div v-if="it.also.length" class="i-also">
              <span v-for="(a, i) in it.also" :key="i">ayrıca: {{ a }}</span>
            </div>

            <div class="i-act">
              <button
                class="go"
                :disabled="it.done"
                @click="store.openQueueItem(it.page, it.focus_id)"
              >
                <Icon name="external" :size="12" /> Aç
              </button>

              <button
                v-if="!it.done"
                class="mark"
                :data-tip="it.reference.kind === 'product'
                  ? 'Bu işi yaptım. Sonucunu ölçmeye başla — mağazada gerçekten bir değişiklik yaptıysanız işaretleyin.'
                  : 'Bu işi yaptım. Madde sonraki analize kadar kapalı kalır.'"
                @click="store.completeQueueItem(it.reference.kind, it.reference.ref)"
              >
                <Icon name="check" :size="12" :stroke-width="2.6" /> Yapıldı
              </button>
              <template v-else>
                <span class="mark on">
                  <Icon name="check" :size="12" :stroke-width="2.6" /> Yapıldı
                </span>
                <a
                  class="link geri"
                  @click="store.restoreQueueItem(it.reference.kind, it.reference.ref)"
                >
                  geri al
                </a>
              </template>

              <!-- ⚠️ Ertele/gizle üstüne gelince beliriyor ama yeri BAŞTAN ayrılmış:
                   satır yüksekliği değişirse liste zıplar. -->
              <span v-if="!it.done" class="gec">
                <button
                  class="ghost-sm"
                  data-tip="Yarın tekrar göster"
                  @click="store.dismissQueueItem(it.reference.kind, it.reference.ref, yarin())"
                >
                  Bugünlük ertele
                </button>
                <button
                  class="ghost-sm"
                  data-tip="Bu bir iş değil, bir daha gösterme. Veri silinmez, ürün kendi ekranında durur."
                  @click="store.dismissQueueItem(it.reference.kind, it.reference.ref, null)"
                >
                  Gizle
                </button>
              </span>
            </div>
          </div>

          <div class="i-score">
            <div class="sc-top">
              <span class="sc-val" :data-tip="skorMetni(it)">{{ Math.round(it.score) }}</span>
              <!-- ⚠️ "≈" yalnızca TAHMİNDE. Ölçülmüş süre kesin yazılıyor ve baloncuk
                   bunu söylüyor — kullanıcı neye güveneceğini bilmeli (Faz S). -->
              <span
                class="sc-min"
                :class="{ olculdu: it.minutes_measured }"
                :data-tip="it.minutes_measured
                  ? 'Odak seanslarında ölçüldü (kova medyanı)'
                  : 'Tahmin — bu kovada henüz yeterli ölçüm yok'"
              >{{ it.minutes_measured ? "" : "≈" }}{{ it.minutes }} dk</span>
            </div>
            <!-- Sayı tek başına anlamsız; çubuk görece büyüklüğü veriyor. -->
            <span class="sc-bar">
              <span
                class="sc-fill"
                :class="`f-${TONE[it.bucket]}`"
                :style="{ width: cubukYuzde(it) + '%' }"
              ></span>
            </span>
          </div>
        </div>
      </div>

      <div v-else-if="!store.todayBusy" class="empty">
        <Icon name="check" :size="24" :stroke-width="1.9" />
        <p class="e-title">Bugün için seçilecek iş yok</p>
        <p class="e-sub">
          Kuyruktaki işlerin hepsi kapandı. Yeni adaylar bir sonraki analizde çıkacak.
          <template v-if="q && q.hidden">
            {{ q.hidden }} madde kuyruktan çıkarılmış —
            <a class="link" @click="store.restoreQueueItems()">geri getirin</a>.
          </template>
        </p>
        <button class="e-btn" @click="store.page = 'opportunities'">Tüm fırsatlara git</button>
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
  padding: 18px 24px 28px;
}

/* ---- üst şerit ---- */
.strip {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px 18px;
  margin-bottom: 14px;
  border: 1px solid var(--c-border-soft);
  border-radius: 12px;
  background: var(--c-list);
}
.s-main {
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
.prog {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-left: auto;
}
.p-lab {
  font-size: 11.5px;
  color: var(--c-soft);
  font-variant-numeric: tabular-nums;
}
.p-bar {
  display: block;
  width: 132px;
  height: 6px;
  border-radius: 4px;
  background: var(--c-chip);
  overflow: hidden;
}
.p-fill {
  display: block;
  height: 100%;
  border-radius: 4px;
  background: var(--badge-uygun-c);
  transition: width 0.16s cubic-bezier(0.32, 0.72, 0, 1);
}
.meta {
  margin-top: 4px;
  font-size: 11px;
  color: var(--c-faint);
}
.s-btns {
  display: flex;
  gap: 8px;
  flex: none;
}
.seans-btn {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  height: 32px;
  padding: 0 13px;
  border: none;
  border-radius: 9px;
  background: var(--accent);
  color: #fff;
  font-size: 12px;
  font-weight: 580;
  cursor: pointer;
  font-family: inherit;
}
.seans-btn:hover {
  filter: brightness(1.06);
}
.skor-btn {
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
.skor-btn:hover {
  background: var(--c-hover);
}

/* ---- kuyruk: tek yüzey, kart çerçevesi yok ---- */
.list {
  border: 1px solid var(--c-border-soft);
  border-radius: 12px;
  background: var(--c-card);
  overflow: hidden;
}
.item {
  display: flex;
  align-items: flex-start;
  gap: 16px;
  padding: 14px 18px;
  border-bottom: 1px solid var(--c-border-soft);
  transition: background 0.16s cubic-bezier(0.32, 0.72, 0, 1);
}
.item:last-child {
  border-bottom: 0;
}
.item:hover {
  background: var(--c-hover);
}
.i-main {
  flex: 1;
  min-width: 0;
}
.i-head {
  display: flex;
  align-items: center;
  gap: 9px;
}
.badge {
  flex: none;
  padding: 2px 8px;
  border-radius: 20px;
  font-size: 10.5px;
  font-weight: 620;
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
  font-size: 14px;
  font-weight: 620;
  letter-spacing: -0.01em;
  line-height: 1.3;
  color: var(--c-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.i-why {
  margin-top: 5px;
  font-size: 12px;
  line-height: 1.45;
  color: var(--c-soft);
}
.i-also {
  display: flex;
  flex-direction: column;
  gap: 3px;
  margin-top: 3px;
  font-size: 11.5px;
  line-height: 1.4;
  color: var(--c-faint);
}

/* ---- eylemler ---- */
.i-act {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 10px;
}
.go {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 30px;
  padding: 0 14px;
  border: none;
  border-radius: 8px;
  background: var(--accent);
  color: #fff;
  font-size: 12.5px;
  font-weight: 590;
  cursor: pointer;
  font-family: inherit;
}
.go:hover:not(:disabled) {
  filter: brightness(1.06);
}
.go:disabled {
  opacity: 0.4;
  cursor: default;
}
.mark {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 30px;
  padding: 0 12px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-card);
  color: var(--c-mid);
  font-size: 12.5px;
  font-weight: 560;
  cursor: pointer;
  font-family: inherit;
}
.mark:hover {
  background: var(--c-hover);
}
/* İşaretlenmiş hâl düğme değil DURUM — tıklanacak bir şey kalmadı. */
.mark.on {
  border-color: transparent;
  background: var(--badge-uygun-bg);
  color: var(--badge-uygun-c);
  cursor: default;
}
.gec {
  display: inline-flex;
  gap: 8px;
  opacity: 0;
  transition: opacity 0.16s cubic-bezier(0.32, 0.72, 0, 1);
}
.item:hover .gec,
.gec:focus-within {
  opacity: 1;
}
.ghost-sm {
  height: 30px;
  padding: 0 11px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--c-faint);
  font-size: 12px;
  font-weight: 540;
  cursor: pointer;
  font-family: inherit;
}
.ghost-sm:hover {
  background: var(--c-chip);
  color: var(--c-mid);
}
.geri {
  font-size: 11.5px;
}

/* ---- skor bloğu ---- */
.i-score {
  flex: none;
  width: 96px;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 6px;
}
.sc-top {
  display: flex;
  align-items: baseline;
  gap: 6px;
}
.sc-val {
  font-size: 17px;
  font-weight: 660;
  color: var(--c-text);
  font-variant-numeric: tabular-nums;
  cursor: help;
}
.sc-min {
  font-size: 11px;
  color: var(--c-faint);
  font-variant-numeric: tabular-nums;
  cursor: help;
}
/* Ölçülmüş süre biraz daha belirgin: tahminle aynı görünmemeli. */
.sc-min.olculdu {
  color: var(--c-soft);
  font-weight: 560;
}
.sc-bar {
  display: block;
  width: 72px;
  height: 4px;
  border-radius: 3px;
  background: var(--c-chip);
  overflow: hidden;
}
.sc-fill {
  display: block;
  height: 100%;
  border-radius: 3px;
}
.f-eksik { background: var(--badge-eksik-c); }
.f-uygun { background: var(--badge-uygun-c); }
.f-bekliyor { background: var(--badge-bekliyor-c); }
.f-hatali { background: var(--badge-hatali-c); }
.f-tamamlandi { background: var(--badge-tamamlandi-c); }

/* ---- yapıldı hâli: madde YERİNDE kalıyor, sadece soluyor ---- */
.item.done {
  background: color-mix(in srgb, var(--badge-uygun-bg) 55%, transparent);
}
.item.done .i-title {
  color: var(--c-faint);
  text-decoration: line-through;
}
.item.done .sc-val {
  color: var(--c-faint);
}
.item.done .sc-fill {
  background: var(--c-border);
}

/* ---- boş durumlar ---- */
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
  max-width: 440px;
  font-size: 12.5px;
  line-height: 1.6;
  color: var(--c-soft);
}
.e-btn {
  margin-top: 16px;
  height: 32px;
  padding: 0 16px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-card);
  color: var(--c-mid);
  font-size: 12.5px;
  font-weight: 560;
  cursor: pointer;
  font-family: inherit;
}
.e-btn:hover {
  background: var(--c-hover);
}
.link {
  color: var(--accent);
  cursor: pointer;
  font-weight: 560;
}

/* ---- skor modali ---- */
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
