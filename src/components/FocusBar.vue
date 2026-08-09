<script setup lang="ts">
/**
 * Odak seansı çubuğu — üst şeridin altında, **tüm ekranlarda** (Faz S).
 *
 * ⚠️ Kabukta (App.vue) yaşıyor çünkü **iş başka ekranlarda yapılıyor**: meta üretimi Ürünler'de,
 * canonical Satışta olmayanlar'da. Seans bir ekrana bağlı olsaydı kullanıcı işi yapmak için
 * oraya geçtiği anda sayacı ve "Bitti" düğmesini kaybederdi.
 *
 * # 🚫 Oyunlaştırma yok
 *
 * Yol haritasının açık yasağı: XP yok · lig yok · seri cezası yok · konfeti yok ·
 * "Harikasın! 🔥" yok. Sayaç sakin, özet bir bilanço. Ertesi sabah yine açılacak bir ekranda
 * coşku yorar.
 */
import { computed, onMounted, onUnmounted } from "vue";
import { useStore } from "../store";
import { BUCKET_LABEL as LABEL, BUCKET_TONE as TONE } from "../buckets";
import Icon from "./Icon.vue";

const store = useStore();
let sayac: ReturnType<typeof setInterval> | null = null;

onMounted(() => {
  void store.loadSession();
  // Saniyede bir yalnızca GÖSTERİMİ tazeler; ölçüm arka uçtaki damgalardan çıkıyor.
  sayac = setInterval(() => store.tickSession(), 1000);
});
onUnmounted(() => {
  if (sayac) clearInterval(sayac);
});

const s = computed(() => store.session);
const it = computed(() => s.value?.locked ?? null);

// ♻️ Kova etiketleri `buckets.ts`te — Bugün ekranıyla ortak (iki kopya sapmaya bir adımdı).

/** Kalan süre — dolduğunda negatife düşmez, "süre doldu" hâline geçer. */
const kalanSn = computed(() => {
  if (!s.value?.session_id) return 0;
  return s.value.planned_minutes * 60 - store.sessionElapsed;
});
const sureDoldu = computed(() => kalanSn.value <= 0);
const sayacMetni = computed(() => {
  const t = Math.abs(kalanSn.value);
  const d = Math.floor(t / 60);
  const sn = t % 60;
  return `${d}:${String(sn).padStart(2, "0")}`;
});

/** Bu işte ne kadar süredir çalışılıyor — ölçülen şeyin ta kendisi. */
const isSuresi = computed(() => {
  if (!it.value) return "";
  const dk = Math.floor((Date.now() - new Date(it.value.started_at).getTime()) / 60000);
  return dk >= 1 ? `${dk} dk` : "";
});

function yarin(): string {
  const d = new Date();
  d.setDate(d.getDate() + 1);
  return d.toISOString().slice(0, 10);
}

async function ertele() {
  const x = it.value;
  if (!x) return;
  await store.dismissQueueItem(x.kind, x.reference, yarin());
  await store.resolveSessionItem("dismissed");
}
</script>

<template>
  <div v-if="s?.session_id" class="bar" :class="{ bitti: sureDoldu }">
    <!-- Sayaç: sakin, tek renk, uyarı kırmızısı yok. -->
    <div class="sayac" :title="`Planlanan ${s.planned_minutes} dakika`">
      <Icon :name="sureDoldu ? 'check' : 'clock'" :size="14" />
      <span class="sn">{{ sayacMetni }}</span>
      <span v-if="sureDoldu" class="etiket">süre doldu</span>
    </div>

    <span class="ayrac"></span>

    <template v-if="it">
      <span class="badge" :class="`b-${TONE[it.bucket]}`">{{ LABEL[it.bucket] }}</span>
      <span class="baslik" :title="it.reason">{{ it.title }}</span>
      <span v-if="isSuresi" class="gecen">{{ isSuresi }}</span>

      <div class="eylem">
        <button class="ac" @click="store.openQueueItem(it.page, it.focus_id)">
          <Icon name="external" :size="12" /> Aç
        </button>
        <button class="ok" @click="store.resolveSessionItem('done')">
          <Icon name="check" :size="12" :stroke-width="2.6" /> Bitti
        </button>
        <!-- ⚠️ "Atla" YALNIZCA bu seans için: iş kuyrukta kalır, kalıcı karar yazılmaz.
             Kalıcı kararlar (Gizle / Bugünlük ertele) Bugün ekranında ve burada erteleme. -->
        <button class="gh" data-tip="Bu seansta atla — iş kuyrukta kalır" @click="store.resolveSessionItem('skipped')">
          Atla
        </button>
        <button class="gh" data-tip="Yarın tekrar göster" @click="ertele()">Ertele</button>
      </div>
    </template>
    <span v-else class="baslik bos">Kuyrukta kilitlenecek iş kalmadı.</span>

    <div class="sag">
      <span class="ilerleme">{{ s.done_count }} bitti</span>
      <!-- Mola ÖNERİLİR, otomatik başlamaz. -->
      <span v-if="store.sessionBreakOffered" class="mola">
        {{ s.break_minutes }} dakika mola?
      </span>
      <button class="gh" @click="store.endSession('stopped')">Seansı bitir</button>
    </div>
  </div>
</template>

<style scoped>
.bar {
  flex: none;
  display: flex;
  align-items: center;
  gap: 10px;
  height: 44px;
  padding: 0 22px;
  border-bottom: 1px solid var(--c-border);
  background: var(--c-list);
  font-size: 12px;
}
/* Süre dolunca renk değil, YALNIZCA vurgu değişiyor — uyarı kırmızısı bu ekranın dili değil. */
.bar.bitti .sn {
  color: var(--c-soft);
}
.sayac {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--c-mid);
  flex: none;
}
.sn {
  font-size: 13px;
  font-weight: 620;
  color: var(--c-text);
  font-variant-numeric: tabular-nums;
}
.etiket {
  font-size: 11px;
  color: var(--c-faint);
}
.ayrac {
  width: 1px;
  height: 16px;
  background: var(--c-border);
  flex: none;
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
/* Müşteri: SEO durumu değil, insan işi — nötr yüzey onu listede ayırt ediyor. */
.b-notr { background: var(--c-chip); color: var(--c-mid); }
.baslik {
  flex: 1;
  min-width: 0;
  font-weight: 560;
  color: var(--c-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.baslik.bos {
  font-weight: 400;
  color: var(--c-soft);
}
.gecen {
  flex: none;
  font-size: 11px;
  color: var(--c-faint);
  font-variant-numeric: tabular-nums;
}
.eylem {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: none;
}
.ac,
.ok {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 26px;
  padding: 0 10px;
  border: none;
  border-radius: 7px;
  font-size: 11.5px;
  font-weight: 580;
  cursor: pointer;
  font-family: inherit;
}
.ac {
  background: var(--accent);
  color: #fff;
}
.ok {
  background: var(--badge-uygun-bg);
  color: var(--badge-uygun-c);
}
.ac:hover,
.ok:hover {
  filter: brightness(0.97);
}
.gh {
  height: 26px;
  padding: 0 9px;
  border: none;
  border-radius: 7px;
  background: transparent;
  color: var(--c-faint);
  font-size: 11.5px;
  cursor: pointer;
  font-family: inherit;
}
.gh:hover {
  background: var(--c-chip);
  color: var(--c-mid);
}
.sag {
  display: flex;
  align-items: center;
  gap: 10px;
  flex: none;
  margin-left: auto;
}
.ilerleme {
  font-size: 11.5px;
  color: var(--c-soft);
  font-variant-numeric: tabular-nums;
}
.mola {
  font-size: 11.5px;
  color: var(--badge-uygun-c);
}
</style>
