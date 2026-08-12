<script setup lang="ts">
/**
 * İçerik açığı — sıralanan sayfanın tipi sorgunun niyetini karşılıyor mu? (Faz İ)
 *
 * **Neden bu ekran var.** Diğer altı araç ekranının hepsi ürün-merkezliydi: sorgu analizi
 * katalogda olmayan sayfayı düşürüyor, GSC çağrısı bile ürün yoluyla filtreleniyordu.
 * Ölçüldü (2026-08-12): ürün dışı **5.700 sorgu / 101.118 gösterim** hiçbir ekranda
 * görünmüyordu. Kategori ve marka sayfalarının çöküşü tam olarak o kör noktadaydı.
 *
 * **Ne göstermiyor, bilerek.** "İçerik yok" listesi değil bu. Kullanıcının elindeki 50
 * maddelik backlog öyle diyordu; ölçtük, 48 maddenin 40'ında gerçekten yazı yoktu ama
 * *"yazı yok" ile "yazı yaz" aynı şey değil.* TeamViewer'da 157 bin gösterim vardı ve
 * arayan teamviewer.com'a gidiyordu. Bu yüzden navigasyonel sorgular ayrı bölümde,
 * gerekçesiyle duruyor — gizlenmiyor ama kuyruğa da girmiyor.
 */
import { computed, onMounted, ref, watch } from "vue";
import { useRowFocus } from "../../useRowFocus";
import { api } from "../../api";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useStore } from "../../store";
import ToolShell from "./ToolShell.vue";
import SeoTable, { type TableCol, type TableRow } from "./SeoTable.vue";
import StorePageModal from "./StorePageModal.vue";
import type { ContentGap, GapKind, StorePageDetail } from "../../types";

const store = useStore();
const veri = computed<ContentGap[]>(() => store.opportunity?.content_gaps ?? []);
const nav = computed(() => store.opportunity?.navigational ?? []);
const limit = ref(30);
const suzgec = ref<GapKind | "hepsi">("hepsi");
const navAcik = ref(false);

/** Kovaların ekrandaki adı ve tek cümlelik gerekçesi — arka uçtaki `GapKind` ile eş. */
const KOVA: Record<GapKind, { ad: string; aciklama: string }> = {
  intent_mismatch: {
    ad: "Niyet uyuşmuyor",
    aciklama:
      "İlk sayfadasınız ama tıklanmıyorsunuz: sorgu bilgi arıyor, karşısına vitrin sayfası çıkıyor.",
  },
  no_page: {
    ad: "Sayfa yok",
    aciklama: "Sorguyu anasayfa karşılıyor — o konuya ait bir sayfanız yok.",
  },
  wrong_match: {
    ad: "Yanlış eşleşme",
    aciklama: "Sıralanan ürün sayfasının sorguyla ortak kelimesi yok; Google yanlış sayfayı seçmiş.",
  },
};

const sayim = computed(() => {
  const m = { intent_mismatch: 0, no_page: 0, wrong_match: 0 } as Record<GapKind, number>;
  for (const g of veri.value) m[g.kind]++;
  return m;
});

const suzulmus = computed(() =>
  suzgec.value === "hepsi" ? veri.value : veri.value.filter((g) => g.kind === suzgec.value),
);

// Kuyruktan gelen odak (Faz K mekanizması, ortak `useRowFocus`).
// ⚠️ Kimlik `sorgu + sayfa` — arka uçtaki `focus_id` ile birebir aynı olmak zorunda,
// ayrışırsa odaklama sessizce çalışmaz (0b3'te yaşanan kaydırma hatasının sınıfı).
const focusId = useRowFocus(
  "contentgap",
  () => suzulmus.value.map((g) => g.query + g.page),
  limit,
);

// 🔴 Süzgeç tuzağı: odaklanan madde açık olmayan bir kovadaysa satır listede HİÇ olmaz ve
// odak sessizce başarısız olur. Kuyruktan gelindiğinde süzgeç sıfırlanıyor.
watch(focusId, (id) => {
  if (!id) return;
  const g = veri.value.find((x) => x.query + x.page === id);
  if (g && suzgec.value !== "hepsi" && suzgec.value !== g.kind) suzgec.value = "hepsi";
});

const cols: TableCol[] = [
  { key: "name", label: "Sorgu / Sıralanan sayfa", type: "text" },
  { key: "kind", label: "Durum", type: "text" },
  { key: "imp", label: "Gösterim", type: "num" },
  { key: "pos", label: "Konum", type: "num" },
  { key: "miss", label: "Kaçırılan", type: "num", emphasis: "down" },
  { key: "act", label: "İşlem", type: "actions" },
];

/** Sayfa tipinin ekrandaki adı — arka uçtaki `PageKind::label` ile eş tutulmalı. */
const PK: Record<string, string> = {
  product: "Ürün",
  category: "Kategori",
  brand: "Marka",
  blog: "İçerik",
  home: "Anasayfa",
  other: "Bilinmiyor",
};

const n = (x: number) => Math.round(x).toString();
/** Uzun adresi okunur kısalt: gövde değil son parça anlamlı. */
const kisaUrl = (u: string) => u.replace(/^https?:\/\/[^/]+/, "") || "/";

const rows = computed<TableRow[]>(() =>
  suzulmus.value.slice(0, limit.value).map((g) => ({
    id: g.query + g.page,
    selected: g.query + g.page === focusId.value,
    name: g.query,
    // ⚠️ Alt satırda kova adı TEKRARLANMIYOR: "Durum" sütununda zaten var ve iki kez
    // yazmak satırı gürültüye boğuyordu. Burada yalnızca sıralanan sayfa — asıl bağlam o.
    sub: `${PK[g.page_kind]} · ${kisaUrl(g.page)}`,
    values: {
      kind: KOVA[g.kind].ad,
      imp: n(g.impressions),
      pos: g.position.toFixed(1),
      miss: n(g.missed_clicks),
    },
    // ⚠️ İyileştirme eylemi YALNIZCA envanterde karşılığı olan satırlarda. Anasayfa ya da
    // envanterde bulunmayan bir adres için panel açmak boş ekran demekti.
    actions: kayitOf(g) ? [{ key: "edit" }, { key: "open" }] : [{ key: "open" }],
  })),
);

// ===== Sayfa iyileştirme paneli =====
// Envanterden slug → (tip, id) eşlemesi: satırdaki adresin hangi mağaza kaydı olduğunu
// bilmeden panel açılamaz. ⚠️ Envanter çekilmemişse eşleme boş kalır ve satırda yalnızca
// "aç" eylemi görünür — panel açıp boş ekran göstermektense eylemi hiç sunmamak doğru.
const envanter = ref<Map<string, StorePageDetail>>(new Map());
const panelAcik = ref(false);
const panelTip = ref("");
const panelId = ref<number | null>(null);

async function envanteriYukle() {
  const m = new Map<string, StorePageDetail>();
  for (const tip of ["category", "brand", "blog"]) {
    try {
      for (const p of await api.listStorePages(tip)) m.set(p.slug.toLowerCase(), p);
    } catch {
      // IdeaSoft kapalı ya da envanter yok — eylem sunulmaz, ekran çalışmaya devam eder.
    }
  }
  envanter.value = m;
}
onMounted(envanteriYukle);

/** Adresin son parçası — arka uçtaki `page_kind::last_segment` ile aynı kural. */
function sonParca(u: string): string {
  const yol = u.replace(/^https?:\/\/[^/]+/, "").split(/[?#]/)[0].replace(/\/+$/, "");
  return (yol.split("/").pop() || "").toLowerCase();
}

function kayitOf(g: ContentGap): StorePageDetail | undefined {
  return envanter.value.get(sonParca(g.page));
}

function iyilestir(id: string) {
  const g = veri.value.find((x) => x.query + x.page === id);
  const k = g && kayitOf(g);
  if (!k) return;
  panelTip.value = k.kind;
  panelId.value = k.remote_id;
  panelAcik.value = true;
}

async function panelKapandi() {
  panelAcik.value = false;
  // Gönderim yapıldıysa envanterdeki meta değişti; listeyi tazele ki rozet doğru olsun.
  await envanteriYukle();
}

const toplamKacan = computed(() =>
  Math.round(veri.value.reduce((t, g) => t + g.missed_clicks, 0)),
);

/**
 * Satır eylemi: sıralanan sayfayı tarayıcıda aç.
 *
 * ⚠️ `store.openProduct` DEĞİL — bu ekrandaki satırlar ürün olmak zorunda değil (kategori,
 * marka, blog, anasayfa). Ürün ekranına gitmeye çalışmak çoğu satırda boş sonuç verirdi.
 */
async function ac(id: string) {
  const g = veri.value.find((x) => x.query + x.page === id);
  if (!g) return;
  try {
    await openUrl(g.page);
  } catch (e) {
    store.toast(String(e), "error");
  }
}
</script>

<template>
  <ToolShell
    :empty="!veri.length && !nav.length"
    empty-text="Sıralandığınız ama sayfası uymayan bir arama bulunamadı."
  >
    <p class="note">
      Google sizi bu aramalarda sıralıyor, ama çıkan sayfa aramanın niyetine uymuyor.
      Toplam <b>{{ toplamKacan }}</b> kaçırılan tıklama.
      <b>Yeni içerik yazmak tek çözüm değil</b> — bazen doğru cevap mevcut sayfayı düzeltmektir.
    </p>

    <div class="suz">
      <button :class="{ akt: suzgec === 'hepsi' }" @click="suzgec = 'hepsi'">
        Tümü <span class="rk">{{ veri.length }}</span>
      </button>
      <button
        v-for="(k, key) in KOVA"
        :key="key"
        :class="{ akt: suzgec === key }"
        :disabled="!sayim[key as GapKind]"
        @click="suzgec = key as GapKind"
      >
        {{ k.ad }} <span class="rk">{{ sayim[key as GapKind] }}</span>
      </button>
    </div>
    <p v-if="suzgec !== 'hepsi'" class="kova-not">{{ KOVA[suzgec].aciklama }}</p>

    <SeoTable
      :cols="cols"
      :rows="rows"
      :focus-id="focusId"
      @action="(p) => (p.key === 'edit' ? iyilestir(p.id) : ac(p.id))"
    />

    <StorePageModal
      :open="panelAcik"
      :kind="panelTip"
      :page-id="panelId"
      @close="panelKapandi"
    />

    <button v-if="suzulmus.length > limit" class="more" @click="limit += 30">
      {{ suzulmus.length - limit }} tane daha
    </button>

    <!-- 🔴 Navigasyonel bölüm GİZLENMİYOR. Kullanıcı "en büyük hacim neden listede yok?"
         diye sorabilmeli ve cevabı burada bulmalı. Kapalı başlıyor çünkü cevap değil,
         cevabın gerekçesi. -->
    <div v-if="nav.length" class="nav-bolum">
      <button class="nav-bas" @click="navAcik = !navAcik">
        <span>{{ nav.length }} arama bilerek listeye alınmadı</span>
        <span class="ok">{{ navAcik ? "gizle" : "neden?" }}</span>
      </button>
      <div v-if="navAcik" class="nav-govde">
        <p class="note">
          Bunlar <b>navigasyonel</b> aramalar: arayan kişi doğrudan bir markanın kendi sitesine
          ya da size ulaşmak istiyor. Sıralamayı yükseltmek bu oranı değiştirmez, içerik de
          kurtarmaz.
        </p>
        <div v-for="x in nav" :key="x.query" class="nav-sat">
          <span class="nav-sorgu">{{ x.query }}</span>
          <span class="nav-say">{{ n(x.impressions) }} gösterim · %{{ (x.ctr * 100).toFixed(2) }}</span>
          <span class="nav-neden">{{ x.reason }}</span>
        </div>
      </div>
    </div>
  </ToolShell>
</template>

<style scoped>
.note {
  font-size: 12.5px;
  color: var(--c-mid);
  line-height: 1.6;
  margin: 0 0 12px;
}
.suz {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  margin-bottom: 8px;
}
.suz button {
  height: 28px;
  padding: 0 11px;
  border-radius: 8px;
  border: 1px solid var(--c-border);
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 12px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  transition: background 0.18s cubic-bezier(0.32, 0.72, 0, 1);
}
.suz button.akt {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}
.suz button:disabled {
  opacity: 0.4;
  cursor: default;
}
.rk {
  font-variant-numeric: tabular-nums;
  opacity: 0.75;
  font-size: 11px;
}
.kova-not {
  font-size: 11.5px;
  color: var(--c-faint);
  margin: 0 0 10px;
  line-height: 1.5;
}
.more {
  margin-top: 10px;
  height: 30px;
  padding: 0 14px;
  border-radius: 8px;
  border: 1px solid var(--c-border);
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 12px;
  cursor: pointer;
}
.nav-bolum {
  margin-top: 18px;
  border-top: 1px solid var(--c-border);
  padding-top: 12px;
}
.nav-bas {
  width: 100%;
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: none;
  border: 0;
  padding: 4px 0;
  cursor: pointer;
  color: var(--c-faint);
  font-size: 12px;
}
.ok {
  color: var(--accent);
}
.nav-govde {
  margin-top: 8px;
}
.nav-sat {
  display: grid;
  grid-template-columns: 160px 170px 1fr;
  gap: 10px;
  padding: 6px 0;
  border-bottom: 1px solid var(--c-border);
  font-size: 12px;
  align-items: baseline;
}
.nav-sorgu {
  font-weight: 560;
}
.nav-say {
  color: var(--c-faint);
  font-variant-numeric: tabular-nums;
}
.nav-neden {
  color: var(--c-faint);
  font-size: 11.5px;
  line-height: 1.5;
}
</style>
