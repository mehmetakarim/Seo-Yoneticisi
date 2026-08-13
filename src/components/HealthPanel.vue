<script setup lang="ts">
/**
 * Bakım kontrolü — açılışta ve Ayarlar'da **aynı** bileşen.
 *
 * **Neden var.** Aynı bakım işleri dört ayrı ekrana dağılmıştı: feed senkronu Ürünler'de,
 * GSC yenileme Genel Bakış'ta, model listesi ve token testi Ayarlar'da. Her açılışta dört
 * ekran gezmek gerekiyordu.
 *
 * 🔴 **Kontrol var, çalıştırma yok.** Ucuz kontroller gerçekten koşuyor (token, model
 * listesi); pahalı olanların yalnızca tazeliği okunuyor ve düğmeye SİZ basıyorsunuz.
 * Gerekçe `commands/health.rs` başlığında: GSC analizi bir ölçüm olayı, her açılışta
 * koşarsa ölçtüğümüz şeyi ölçüm aracıyla değiştirmiş oluruz.
 *
 * ⚠️ Tek bileşen, iki yer: kopyalasaydık biri güncellenmeyip sapardı — bu projede
 * kopyalanan tablo/geometri dört kez saptı.
 */
import { computed, onMounted, ref } from "vue";
import { api } from "../api";
import { useStore } from "../store";
import Icon from "./Icon.vue";
import type { HealthCheck } from "../types";

const props = withDefaults(defineProps<{ compact?: boolean }>(), { compact: false });
const emit = defineEmits<{ done: [temiz: boolean] }>();
const store = useStore();

const yerel = ref<HealthCheck[]>([]);
const uzak = ref<HealthCheck[]>([]);
const uzakBekliyor = ref(true);
const calisan = ref("");

const hepsi = computed(() => [...yerel.value, ...uzak.value]);
/** İlerleme: yerel kontroller anında biter, uzak olanlar ağ bekler. */
const ilerleme = computed(() => {
  const toplam = 5; // 3 yerel + 2 uzak — sabit, çünkü liste sabit
  return Math.round(((yerel.value.length + uzak.value.length) / toplam) * 100);
});
const sorunlu = computed(() =>
  hepsi.value.filter((c) => c.state === "stale" || c.state === "error"),
);

const TON: Record<string, { ikon: string; renk: string }> = {
  ok: { ikon: "check", renk: "var(--green)" },
  stale: { ikon: "clock", renk: "var(--amber)" },
  error: { ikon: "alert", renk: "var(--red)" },
  // ⚠️ Kapalı modül HATA DEĞİL: kullanmadığı bir şey için kullanıcıyı endişelendirmeyelim.
  off: { ikon: "info", renk: "var(--c-faint)" },
};

async function kontrolEt() {
  yerel.value = [];
  uzak.value = [];
  uzakBekliyor.value = true;
  try {
    yerel.value = await api.localHealth();
  } catch (e) {
    store.toast(String(e), "error");
  }
  try {
    uzak.value = await api.remoteHealth();
  } catch (e) {
    store.toast(String(e), "error");
  } finally {
    uzakBekliyor.value = false;
    emit("done", sorunlu.value.length === 0);
  }
}
onMounted(kontrolEt);
defineExpose({ kontrolEt });

/**
 * Satırdaki eylemi çalıştırır.
 *
 * ⚠️ Bunlar PAHALI işler ve bilerek elle tetikleniyor. Bittiğinde kontrol yenileniyor ki
 * satır "bugün" desin — yoksa kullanıcı işi yaptı ama ekran hâlâ "bayat" gösterirdi.
 */
async function calistir(c: HealthCheck) {
  if (!c.action || calisan.value) return;
  calisan.value = c.key;
  try {
    // ⚠️ `store.sync()` (feed senkronu) — `api.syncFeed` doğrudan çağrılmıyor: store
    // eylemi ayrıca listeyi yeniliyor ve özet şeridini gösteriyor, ikisi de gerekli.
    if (c.action === "sync_feed") await store.sync();
    else if (c.action === "run_analysis") await store.runOpportunityAnalysis();
    else if (c.action === "sync_store_pages") await api.syncStorePages();
    else if (c.action === "open_settings") {
      store.page = "settings";
      emit("done", true);
      return;
    }
    await kontrolEt();
  } catch (e) {
    store.toast(String(e), "error");
  } finally {
    calisan.value = "";
  }
}

const EYLEM_AD: Record<string, string> = {
  sync_feed: "Senkronla",
  run_analysis: "Analiz et",
  sync_store_pages: "Envanteri çek",
  open_settings: "Ayarlar",
};
</script>

<template>
  <div class="hp" :class="{ compact: props.compact }">
    <div v-if="uzakBekliyor" class="bar">
      <div class="dolgu" :style="{ width: ilerleme + '%' }"></div>
    </div>

    <div v-for="c in hepsi" :key="c.key" class="sat">
      <Icon :name="TON[c.state].ikon" :size="14" :style="{ color: TON[c.state].renk }" />
      <span class="ad">{{ c.label }}</span>
      <span class="det">{{ c.detail }}</span>
      <button
        v-if="c.action"
        class="eyl"
        :disabled="!!calisan"
        @click="calistir(c)"
      >
        <Icon v-if="calisan === c.key" name="refresh" :size="12" class="spin" />
        {{ calisan === c.key ? "…" : EYLEM_AD[c.action] || "Çalıştır" }}
      </button>
    </div>

    <div v-if="uzakBekliyor" class="sat bekle">
      <Icon name="refresh" :size="14" class="spin" />
      <span class="ad">Bağlantılar denetleniyor…</span>
    </div>
  </div>
</template>

<style scoped>
.hp {
  display: flex;
  flex-direction: column;
}
.bar {
  height: 3px;
  border-radius: 2px;
  background: var(--c-border);
  overflow: hidden;
  margin-bottom: 10px;
}
.dolgu {
  height: 100%;
  background: var(--accent);
  transition: width 0.3s cubic-bezier(0.32, 0.72, 0, 1);
}
.sat {
  display: grid;
  grid-template-columns: 18px 1fr auto auto;
  gap: 10px;
  align-items: center;
  padding: 7px 0;
  border-bottom: 1px solid var(--c-border);
  font-size: 12.5px;
}
.sat:last-child {
  border-bottom: 0;
}
.ad {
  font-weight: 560;
}
.det {
  color: var(--c-faint);
  font-size: 11.5px;
  text-align: right;
}
.eyl {
  height: 25px;
  padding: 0 10px;
  border-radius: 7px;
  border: 1px solid var(--c-border);
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 11.5px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 5px;
}
.eyl:disabled {
  opacity: 0.45;
  cursor: default;
}
.bekle .ad {
  font-weight: 400;
  color: var(--c-faint);
}
.compact .sat {
  padding: 5px 0;
  font-size: 12px;
}
</style>
