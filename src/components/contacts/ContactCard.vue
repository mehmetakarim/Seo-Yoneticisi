<script setup lang="ts">
/**
 * Kişi kartı (Faz C) — kimlik, sonraki adım ve temas geçmişi.
 *
 * 🔑 Ekranın kalbi **sonraki adım**: yol haritasının deyimiyle "CRM'in %80'i". Tarih
 * verildiği gün kişi Bugün kuyruğunda çıkıyor, dönüş yapılınca temizleniyor.
 *
 * ⚠️ Temas ekleme formu **randevuyu da aynı adımda** alıyor ("aradım, iki hafta sonra
 * tekrar"). İkiye bölünseydi kullanıcı ikincisini unutur, kişi sessizce düşerdi.
 */
import { computed, ref, watch } from "vue";
import { useStore } from "../../store";
import { api } from "../../api";
import type { CatalogMatch, Quote } from "../../types";
import Icon from "../Icon.vue";

const store = useStore();

const KANALLAR = ["", "mail", "telefon", "instagram", "fuar", "referans", "diğer"];
const TEMAS_TURLERI = [
  { key: "call", label: "Telefon" },
  { key: "email", label: "E-posta" },
  { key: "meeting", label: "Görüşme" },
  { key: "note", label: "Not" },
];
const TEMAS_ETIKET: Record<string, string> = {
  call: "Telefon",
  email: "E-posta",
  meeting: "Görüşme",
  note: "Not",
  followup_done: "Dönüş yapıldı",
};

/** Düzenleme kopyası — kullanıcı kaydetmeden store'daki kayıt değişmesin. */
const f = ref(bos());

/**
 * Temas formu.
 *
 * 🔴 **Tanım aşağıdaki `watch`tan ÖNCE olmak ZORUNDA.** `immediate: true` olan izleyici
 * kurulum sırasında hemen çalışıp bu değişkene yazıyor; aşağıda kalsaydı geçici ölüm
 * bölgesine (TDZ) düşer ve *"Cannot access 'temas' before initialization"* ile **tüm ekran
 * boş açılırdı**. Saha hatası, 2026-08-10 — bkz. brain.md.
 */
const temas = ref({ kind: "call", note: "", nextStepAt: "", nextStepNote: "" });
function bos() {
  return {
    name: "",
    company: "",
    email: "",
    phone: "",
    channel: "",
    note: "",
    nextStepAt: "",
    nextStepNote: "",
  };
}

watch(
  () => store.contact,
  (c) => {
    f.value = c
      ? {
          name: c.name,
          company: c.company,
          email: c.email,
          phone: c.phone,
          channel: c.channel,
          note: c.note,
          nextStepAt: c.next_step_at?.slice(0, 10) ?? "",
          nextStepNote: c.next_step_note,
        }
      : bos();
    temas.value = { kind: "call", note: "", nextStepAt: "", nextStepNote: "" };
  },
  { immediate: true },
);

const yeni = computed(() => !store.contact);
const kaydedilebilir = computed(() => f.value.name.trim().length > 0);

async function kaydet() {
  if (!kaydedilebilir.value) return;
  await store.saveContact({
    id: store.contact?.id ?? null,
    name: f.value.name,
    company: f.value.company,
    email: f.value.email,
    phone: f.value.phone,
    channel: f.value.channel,
    note: f.value.note,
    nextStepAt: f.value.nextStepAt || null,
    nextStepNote: f.value.nextStepNote,
  });
}

// --- İlgi etiketleri (Faz C2) ---
// ⚠️ Sabit liste yok: bir mağazanın etiketleri ("sunucu") diğerininkine benzemez. Öneriler
// kullanıcının kendi verisinden geliyor, yenisini yazmak engellenmiyor.
const yeniEtiket = ref("");
const etiketOnerileri = computed(() =>
  store.contactTags.filter((t) => !(store.contact?.tags ?? []).includes(t)).slice(0, 8),
);

async function etiketEkle(t: string) {
  const c = store.contact;
  const ad = t.trim();
  if (!c || !ad || c.tags.includes(ad)) return;
  await store.setContactTags(c.id, [...c.tags, ad]);
  yeniEtiket.value = "";
}

async function etiketCikar(t: string) {
  const c = store.contact;
  if (!c) return;
  await store.setContactTags(
    c.id,
    c.tags.filter((x) => x !== t),
  );
}

// --- "Bu ürünle ilgilendi" (Faz C2) ---
// ♻️ Arama EOL halef seçicisiyle aynı uçtan: satıştaki ürünlerde arıyor.
const urunArama = ref("");
const urunSonuc = ref<CatalogMatch[]>([]);
/** Kişinin teklifleri (Faz T2) — ürün bağıyla aynı fikir: tek kaynaktan sorgu. */
const teklifler = ref<Quote[]>([]);
let urunZaman: ReturnType<typeof setTimeout> | null = null;

function urunAra(v: string) {
  urunArama.value = v;
  if (urunZaman) clearTimeout(urunZaman);
  // 3 karakterin altında arka uç zaten boş dönüyor; gereksiz çağrı yapmıyoruz.
  if (v.trim().length < 3) {
    urunSonuc.value = [];
    return;
  }
  urunZaman = setTimeout(async () => {
    urunSonuc.value = await api.searchLiveProducts(v).catch(() => []);
  }, 220);
}

watch(
  () => store.contact?.id,
  async (id) => {
    teklifler.value = id ? await api.quotesOfContact(id).catch(() => []) : [];
  },
  { immediate: true },
);

const paraB = (v: number, c: string) =>
  new Intl.NumberFormat("tr-TR", { style: "currency", currency: c, maximumFractionDigits: 0 })
    .format(v);

function teklifAc(id: number) {
  store.page = "quotes";
  void store.openQuote(id);
}

async function urunBagla(m: CatalogMatch) {
  const c = store.contact;
  if (!c) return;
  await store.linkContactProduct(c.id, m.slug);
  urunArama.value = "";
  urunSonuc.value = [];
}

async function temasEkle() {
  const c = store.contact;
  if (!c) return;
  await store.addContactEvent(
    c.id,
    temas.value.kind,
    temas.value.note,
    temas.value.nextStepAt || null,
    temas.value.nextStepNote || null,
  );
  temas.value = { kind: "call", note: "", nextStepAt: "", nextStepNote: "" };
}

const tarih = (s: string) => s.slice(0, 10).split("-").reverse().join(".");
</script>

<template>
  <section class="card om-scroll">
    <div class="head">
      <h2>{{ yeni ? "Yeni kişi" : store.contact?.name }}</h2>
      <div class="head-btns">
        <button
          v-if="store.contact"
          class="ghost"
          @click="store.archiveContact(store.contact.id, !store.contact.archived)"
        >
          {{ store.contact.archived ? "Arşivden çıkar" : "Arşivle" }}
        </button>
        <button class="primary" :disabled="!kaydedilebilir" @click="kaydet">Kaydet</button>
      </div>
    </div>

    <!-- ⚠️ Arşiv SİLME değil: geçmiş temaslar bir kayıt, kişi listeden çıksa da kalmalı. -->
    <p v-if="store.contact?.archived" class="hint">
      Bu kişi arşivde: listede soluk görünüyor ve Bugün kuyruğuna iş düşürmüyor. Temas geçmişi
      olduğu gibi duruyor.
    </p>

    <div class="grid">
      <label>Ad soyad <input v-model="f.name" class="fx" placeholder="Zorunlu" /></label>
      <label>Firma <input v-model="f.company" class="fx" /></label>
      <label>E-posta <input v-model="f.email" class="fx" type="email" /></label>
      <label>Telefon <input v-model="f.phone" class="fx" /></label>
      <label>
        Kanal
        <select v-model="f.channel" class="fx">
          <option v-for="k in KANALLAR" :key="k" :value="k">{{ k || "—" }}</option>
        </select>
      </label>
    </div>

    <!-- Fazın kalbi: bu tarih Bugün kuyruğunu besliyor. -->
    <div class="adim">
      <div class="adim-bas">
        <Icon name="calendarClock" :size="14" :stroke-width="2" />
        <b>Sonraki adım</b>
        <span class="adim-not">tarih verirseniz o gün Bugün listesinde çıkar</span>
      </div>
      <div class="adim-alan">
        <input v-model="f.nextStepAt" class="fx" type="date" />
        <input v-model="f.nextStepNote" class="fx" placeholder="Ne yapılacak? (ör. fiyat ver)" />
      </div>
    </div>

    <label class="tam">Not <textarea v-model="f.note" class="fx" rows="2"></textarea></label>

    <template v-if="store.contact">
      <!-- İlgi etiketleri: kanal "nereden geldi", etiket "neyle ilgileniyor". -->
      <div class="etiket">
        <label class="lbl">İlgi etiketleri</label>
        <div class="et-satir">
          <span v-for="t in store.contact.tags" :key="t" class="chip on">
            {{ t }}
            <button class="et-x" title="Etiketi kaldır" @click="etiketCikar(t)">
              <Icon name="x" :size="11" :stroke-width="2.6" />
            </button>
          </span>
          <input
            v-model="yeniEtiket"
            class="fx et-giris"
            placeholder="Etiket ekle…"
            @keyup.enter="etiketEkle(yeniEtiket)"
          />
        </div>
        <div v-if="etiketOnerileri.length" class="et-oneri">
          <span class="ono">Kullandıklarınız:</span>
          <button v-for="t in etiketOnerileri" :key="t" class="chip" @click="etiketEkle(t)">
            {{ t }}
          </button>
        </div>
      </div>

      <!-- SEO tarafıyla CRM'i birleştiren tek yer. -->
      <div class="etiket">
        <label class="lbl">İlgilendiği ürünler</label>
        <div v-for="p in store.contactProducts" :key="p.sku" class="urun-sat">
          <Icon name="box" :size="13" :stroke-width="1.9" />
          <span class="urun-ad">{{ p.name }}</span>
          <button class="et-x" title="Bağı kaldır" @click="store.unlinkContactProduct(store.contact!.id, p.sku)">
            <Icon name="x" :size="11" :stroke-width="2.6" />
          </button>
        </div>
        <input
          class="fx"
          :value="urunArama"
          placeholder="Ürün ara ve bağla (en az 3 harf)"
          @input="urunAra(($event.target as HTMLInputElement).value)"
        />
        <div v-if="urunSonuc.length" class="urun-sonuc om-scroll">
          <button v-for="m in urunSonuc" :key="m.slug" class="us-sat" @click="urunBagla(m)">
            {{ m.name }}
          </button>
        </div>
      </div>
    </template>

    <template v-if="store.contact">
      <div class="ayrac"></div>

      <div class="temas">
        <b>Temas ekle</b>
        <div class="tur">
          <button
            v-for="t in TEMAS_TURLERI"
            :key="t.key"
            class="chip"
            :class="{ on: temas.kind === t.key }"
            @click="temas.kind = t.key"
          >
            {{ t.label }}
          </button>
        </div>
        <input v-model="temas.note" class="fx" placeholder="Ne konuşuldu?" />
        <!-- Temas + yeni randevu TEK adımda — ayrılırsa ikincisi unutulur. -->
        <div class="adim-alan">
          <input v-model="temas.nextStepAt" class="fx" type="date" />
          <input v-model="temas.nextStepNote" class="fx" placeholder="Sonraki adım (isteğe bağlı)" />
        </div>
        <button class="primary" @click="temasEkle">Temas kaydet</button>
      </div>

      <!-- Teklifler: kişinin ne aldığı/almadığı temas geçmişinin bir parçası. -->
      <div v-if="teklifler.length" class="gecmis">
        <b>Teklifler</b>
        <button v-for="t in teklifler" :key="t.id" class="teklif-sat" @click="teklifAc(t.id)">
          <span class="ts-no">{{ t.no }}</span>
          <span class="ts-durum">{{ t.status_label }}</span>
          <span class="ts-tut">{{ paraB(t.grand_total, t.currency) }}</span>
        </button>
      </div>

      <div class="gecmis">
        <b>Temas geçmişi</b>
        <div v-if="!store.contactEvents.length" class="bos">
          Henüz temas kaydı yok. İlk aramayı veya maili buraya yazarsanız, kişinin ne zaman
          hangi konuyla geldiğini sonra hatırlarsınız.
        </div>
        <div v-for="e in store.contactEvents" :key="e.id" class="olay">
          <span class="olay-tarih">{{ tarih(e.at) }}</span>
          <span class="olay-tur">{{ TEMAS_ETIKET[e.kind] ?? e.kind }}</span>
          <span class="olay-not">{{ e.note || "—" }}</span>
        </div>
      </div>
    </template>

    <p v-else class="hint">
      Kişiyi kaydettikten sonra temas geçmişi ve sonraki adım takibi açılır.
    </p>
  </section>
</template>

<style scoped>
.card {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  padding: 20px 24px 40px;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 14px;
}
h2 {
  font-size: 17px;
  font-weight: 640;
  letter-spacing: -0.015em;
  margin: 0;
  color: var(--c-text);
}
.head-btns {
  display: flex;
  gap: 8px;
  flex: none;
}
.primary,
.ghost {
  height: 32px;
  padding: 0 14px;
  border-radius: 8px;
  font-size: 12.5px;
  font-weight: 560;
  cursor: pointer;
  border: 1px solid var(--c-border);
  background: var(--c-input);
  color: var(--c-mid);
}
.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}
.primary:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
  gap: 12px;
}
label,
.tam {
  display: block;
  font-size: 11.5px;
  font-weight: 560;
  color: var(--c-faint);
}
.tam {
  margin-top: 12px;
}
input,
select,
textarea {
  display: block;
  width: 100%;
  margin-top: 5px;
  padding: 8px 10px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-input);
  color: var(--c-text);
  font-size: 13px;
  font-family: inherit;
  outline: none;
}
textarea {
  resize: vertical;
}
.adim {
  margin-top: 16px;
  padding: 13px 14px;
  border: 1px solid var(--c-border);
  border-radius: 10px;
  background: var(--c-list);
}
.adim-bas {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 13px;
  color: var(--c-text);
}
.adim-not {
  font-size: 11.5px;
  color: var(--c-faint);
  font-weight: 400;
}
.adim-alan {
  display: flex;
  gap: 8px;
  margin-top: 9px;
}
.adim-alan input:first-child {
  flex: none;
  width: 150px;
}
.etiket {
  margin-top: 16px;
}
.lbl {
  display: block;
  font-size: 11.5px;
  font-weight: 560;
  color: var(--c-faint);
  margin-bottom: 6px;
}
.et-satir {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}
.et-giris {
  width: 150px;
  margin-top: 0;
  padding: 5px 9px;
  font-size: 12.5px;
}
.et-x {
  display: inline-flex;
  align-items: center;
  border: none;
  background: none;
  color: inherit;
  cursor: pointer;
  padding: 0 0 0 4px;
  opacity: 0.6;
}
.et-x:hover {
  opacity: 1;
}
.et-oneri {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  margin-top: 8px;
}
.ono {
  font-size: 11.5px;
  color: var(--c-faint);
}
.urun-sat {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 6px 0;
  font-size: 12.5px;
  color: var(--c-text);
  border-bottom: 1px solid var(--c-border);
}
.urun-ad {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.urun-sonuc {
  max-height: 148px;
  overflow-y: auto;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  margin-top: 6px;
}
.us-sat {
  display: block;
  width: 100%;
  text-align: left;
  padding: 7px 10px;
  border: none;
  background: none;
  color: var(--c-text);
  font-size: 12.5px;
  cursor: pointer;
}
.us-sat:hover {
  background: var(--c-hover);
}
.ayrac {
  height: 1px;
  background: var(--c-border);
  margin: 22px 0 18px;
}
.temas b,
.gecmis b {
  display: block;
  font-size: 13px;
  font-weight: 620;
  color: var(--c-text);
  margin-bottom: 9px;
}
.tur {
  display: flex;
  gap: 6px;
  margin-bottom: 9px;
}
.temas .primary {
  margin-top: 10px;
}
.gecmis {
  margin-top: 24px;
}
.bos {
  font-size: 12.5px;
  line-height: 1.6;
  color: var(--c-faint);
  max-width: 460px;
}
.teklif-sat {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  padding: 8px 0;
  border: none;
  border-bottom: 1px solid var(--c-border);
  background: none;
  cursor: pointer;
  font-size: 12.5px;
  text-align: left;
}
.teklif-sat:hover .ts-no {
  color: var(--accent);
}
.ts-no {
  flex: none;
  width: 96px;
  font-weight: 560;
  color: var(--c-text);
}
.ts-durum {
  flex: 1;
  color: var(--c-faint);
}
.ts-tut {
  flex: none;
  font-variant-numeric: tabular-nums;
  color: var(--c-text);
  font-weight: 560;
}
.olay {
  display: flex;
  gap: 12px;
  padding: 9px 0;
  border-bottom: 1px solid var(--c-border);
  font-size: 12.5px;
}
.olay-tarih {
  flex: none;
  width: 76px;
  color: var(--c-faint);
  font-variant-numeric: tabular-nums;
}
.olay-tur {
  flex: none;
  width: 82px;
  font-weight: 560;
  color: var(--c-mid);
}
.olay-not {
  color: var(--c-text);
  min-width: 0;
}
</style>
