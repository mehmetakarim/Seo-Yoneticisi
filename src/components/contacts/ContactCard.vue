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
import { computed, nextTick, onBeforeUnmount, ref, shallowRef, watch } from "vue";
import { useStore } from "../../store";
import { api } from "../../api";
import { contactSync, teamContactConflictHistory, teamContactProgress, teamContactSource, type TeamContactConflictArchive } from "../../teamContacts";
import { teamAccessIssue, teamProject, teamUser, teamWorkspace } from "../../teamConnection";
import { searchTeamCatalog } from "../../teamCatalog";
import type { CatalogMatch, Contact, Quote } from "../../types";
import Icon from "../Icon.vue";
import ContactConflictModal from "./ContactConflictModal.vue";

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
  quote_sent: "Teklif gönderildi",
  quote_won: "Teklif kazanıldı",
  quote_lost: "Teklif kaybedildi",
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
const savingEvent = ref(false);
const savingAction = ref(false);
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

function contactForm(c: Contact | null | undefined) {
  return c ? {
    name: c.name, company: c.company, email: c.email, phone: c.phone, channel: c.channel,
    note: c.note, nextStepAt: c.next_step_at?.slice(0, 10) ?? "", nextStepNote: c.next_step_note,
  } : bos();
}

watch(() => store.contact, (current, previous) => {
  const next = contactForm(current);
  if (current?.id !== previous?.id || JSON.stringify(next) !== JSON.stringify(contactForm(previous))) f.value = next;
}, { immediate: true });
watch(() => store.contact?.id, () => {
  temas.value = { kind: "call", note: "", nextStepAt: "", nextStepNote: "" };
});

const yeni = computed(() => !store.contact);
const kaydedilebilir = computed(() => f.value.name.trim().length > 0);
const savingContact = ref(false);
const hasConflict = ref(false);
const contactHistory = shallowRef<TeamContactConflictArchive[]>([]);
const conflictModal = ref<"review" | "history" | null>(null);
const contactHeading = ref<HTMLHeadingElement | null>(null);
const historyButton = ref<HTMLButtonElement | null>(null);
let conflictTrigger: HTMLElement | null = null;
const writing = computed(() => savingContact.value || savingEvent.value || savingAction.value);
const editBlocked = computed(() => writing.value || hasConflict.value);
const sharedContactId = computed(() => typeof store.contact?.id === "string" ? store.contact.id : undefined);

watch([sharedContactId, teamProject, teamUser, teamWorkspace, teamAccessIssue], ([id, , , , issue]) => {
  conflictModal.value = null;
  conflictTrigger = null;
  contactHistory.value = [];
  if (id && !issue) void loadContactHistory(id);
}, { immediate: true });

watch([() => store.contact, () => contactSync.value.conflicts, teamProject, teamUser, teamWorkspace, teamAccessIssue], async ([person, , , , , issue], previous, onCleanup) => {
  const id = person?.id;
  if (typeof id !== "string" || issue) {
    hasConflict.value = false;
    return;
  }
  if (previous[0]?.id !== id) hasConflict.value = false;
  let current = true;
  onCleanup(() => { current = false; });
  const progress = await teamContactProgress(id).catch(() => "synced");
  if (current && store.contact === person) hasConflict.value = progress === "conflict";
}, { immediate: true });

async function loadContactHistory(id: string) {
  const source = teamContactSource();
  const history = await teamContactConflictHistory(id).catch(() => null);
  if (history && sharedContactId.value === id && teamContactSource() === source) contactHistory.value = history;
}

function showConflictModal(mode: "review" | "history", event: MouseEvent) {
  if (!sharedContactId.value || writing.value || teamAccessIssue.value) return;
  conflictTrigger = event.currentTarget instanceof HTMLElement ? event.currentTarget : null;
  conflictModal.value = mode;
}

async function closeConflictModal() {
  conflictModal.value = null;
  const trigger = conflictTrigger;
  conflictTrigger = null;
  await nextTick();
  if (trigger?.isConnected) trigger.focus();
}

async function contactResolved(remote: Contact) {
  const id = sharedContactId.value;
  if (!id || remote.id !== id) return;
  const source = teamContactSource();
  conflictModal.value = null;
  conflictTrigger = null;
  hasConflict.value = false;
  await Promise.all([store.refreshResolvedContact(remote), loadContactHistory(id)]);
  if (sharedContactId.value === id && teamContactSource() === source) {
    await nextTick();
    (historyButton.value ?? contactHeading.value)?.focus({ preventScroll: true });
  }
}

function canRefreshContact() {
  return !writing.value && JSON.stringify(f.value) === JSON.stringify(contactForm(store.contact));
}
defineExpose({ canRefreshContact });

async function kaydet() {
  if (!kaydedilebilir.value || editBlocked.value) return;
  savingContact.value = true;
  try {
    await store.saveContact({
    id: store.contact?.id ?? null,
    revision: store.contact?.revision,
    name: f.value.name,
    company: f.value.company,
    email: f.value.email,
    phone: f.value.phone,
    channel: f.value.channel,
    note: f.value.note,
    nextStepAt: f.value.nextStepAt || null,
    nextStepNote: f.value.nextStepNote,
    });
  } finally {
    savingContact.value = false;
  }
}

// --- İlgi etiketleri (Faz C2) ---
// ⚠️ Sabit liste yok: bir mağazanın etiketleri ("sunucu") diğerininkine benzemez. Öneriler
// kullanıcının kendi verisinden geliyor, yenisini yazmak engellenmiyor.
const yeniEtiket = ref("");
const etiketOnerileri = computed(() =>
  store.contactTags.filter((t) => !(store.contact?.tags ?? []).includes(t)).slice(0, 8),
);

async function changeContact(write: () => Promise<boolean>) {
  if (editBlocked.value) return false;
  savingAction.value = true;
  try {
    return await write();
  } finally {
    savingAction.value = false;
  }
}

async function etiketEkle(t: string) {
  const c = store.contact;
  const ad = t.trim();
  if (!c || !ad || c.tags.includes(ad)) return;
  const input = yeniEtiket.value;
  const saved = await changeContact(() => store.setContactTags(c.id, [...c.tags, ad]));
  if (saved && store.contact?.id === c.id && yeniEtiket.value === input && input.trim() === ad) yeniEtiket.value = "";
}

async function etiketCikar(t: string) {
  const c = store.contact;
  if (!c) return;
  await changeContact(() => store.setContactTags(c.id, c.tags.filter((x) => x !== t)));
}

const urunArama = ref("");
const urunSonuc = ref<CatalogMatch[]>([]);
const urunHatasi = ref("");
const arananUrunAdlari = ref(new Map<string, string>());
const bagliUrunAdlari = computed(() => new Map([
  ...store.allRows.map(({ sku, name }) => [sku, name] as const),
  ...arananUrunAdlari.value,
]));
/** Kişinin teklifleri (Faz T2) — ürün bağıyla aynı fikir: tek kaynaktan sorgu. */
const teklifler = ref<Quote[]>([]);
let urunZaman: ReturnType<typeof setTimeout> | null = null;
let aramaSayaci = 0;
onBeforeUnmount(() => {
  aramaSayaci++;
  if (urunZaman) clearTimeout(urunZaman);
});

function urunAra(v: string) {
  urunArama.value = v;
  urunSonuc.value = [];
  urunHatasi.value = "";
  const arama = ++aramaSayaci;
  if (urunZaman) clearTimeout(urunZaman);
  const id = store.contact?.id;
  if (id == null || v.trim().length < 3) return;
  const source = teamContactSource();
  const guncel = () => arama === aramaSayaci && store.contact?.id === id && source === teamContactSource();
  urunZaman = setTimeout(async () => {
    try {
      const results = typeof id === "string" ? await searchTeamCatalog(v) : await api.searchLiveProducts(v);
      if (!guncel()) return;
      const linked = new Set(store.contactProducts.map(({ sku }) => sku));
      urunSonuc.value = results.filter(({ sku }) => sku && !linked.has(sku));
      for (const { sku, name } of results) arananUrunAdlari.value.set(sku, name);
    } catch (error) {
      if (guncel()) urunHatasi.value = error instanceof Error ? error.message : "Ürünler alınamadı. Tekrar deneyin.";
    }
  }, 220);
}

watch(
  () => store.contact?.id,
  async (id, _previous, onCleanup) => {
    urunAra("");
    yeniEtiket.value = "";
    teklifler.value = [];
    if (id == null) return;
    let current = true;
    onCleanup(() => { current = false; });
    const result = await api.quotesOfContact(id).catch(() => []);
    if (current) teklifler.value = result;
  },
  { immediate: true },
);

const paraB = (v: number, c: string) =>
  new Intl.NumberFormat("tr-TR", { style: "currency", currency: c, maximumFractionDigits: 0 })
    .format(v);

function teklifAc(id: Quote["id"]) {
  store.page = "quotes";
  void store.openQuote(id);
}

async function urunBagla(m: CatalogMatch) {
  const c = store.contact;
  if (!c) return;
  const term = urunArama.value;
  const saved = await changeContact(() => store.linkContactProduct(c.id, m.sku));
  if (saved && store.contact?.id === c.id && term === urunArama.value) urunAra("");
}

function urunBaginiKaldir(sku: string) {
  const c = store.contact;
  if (c) void changeContact(() => store.unlinkContactProduct(c.id, sku));
}

function arsivDurumunuDegistir() {
  const c = store.contact;
  if (c) void changeContact(() => store.archiveContact(c.id, !c.archived));
}

async function temasEkle() {
  const c = store.contact;
  if (!c || editBlocked.value) return;
  const { kind, note, nextStepAt, nextStepNote } = temas.value;
  savingEvent.value = true;
  try {
    const saved = await store.addContactEvent(c.id, kind, note, nextStepAt || null, nextStepNote || null);
    if (saved && store.contact?.id === c.id && temas.value.kind === kind && temas.value.note === note
      && temas.value.nextStepAt === nextStepAt && temas.value.nextStepNote === nextStepNote) {
      temas.value = { kind: "call", note: "", nextStepAt: "", nextStepNote: "" };
    }
  } finally {
    savingEvent.value = false;
  }
}

const tarih = (s: string) => s.slice(0, 10).split("-").reverse().join(".");
</script>

<template>
  <section class="card om-scroll">
    <div class="head">
      <h2 ref="contactHeading" tabindex="-1">{{ yeni ? "Yeni kişi" : store.contact?.name }}</h2>
      <div class="head-btns">
        <button
          v-if="store.contact"
          class="ghost"
          :disabled="editBlocked"
          @click="arsivDurumunuDegistir"
        >
          {{ store.contact.archived ? "Arşivden çıkar" : "Arşivle" }}
        </button>
        <button class="primary" :disabled="!kaydedilebilir || editBlocked" @click="kaydet">Kaydet</button>
      </div>
    </div>
    <div v-if="hasConflict || contactHistory.length" class="conflict-actions">
      <div v-if="hasConflict" class="contact-conflict">
        <p role="status">Kişi kaydında eşitleme çakışması var. Bu cihazdaki düzenlemeler eşitleme kuyruğunda.</p>
        <button type="button" class="secondary-action" :disabled="writing" @click="showConflictModal('review', $event)">
          Eşitleme çakışmasını incele
        </button>
      </div>
      <button v-if="contactHistory.length" ref="historyButton" type="button" class="secondary-action" :disabled="writing"
        @click="showConflictModal('history', $event)">
        <Icon name="clock" :size="14" aria-hidden="true" /> Saklanan düzenlemeler
      </button>
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
            <button class="et-x" :disabled="editBlocked" :aria-label="`${t} etiketini kaldır`" @click="etiketCikar(t)">
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
          <button v-for="t in etiketOnerileri" :key="t" class="chip" :disabled="editBlocked" @click="etiketEkle(t)">
            {{ t }}
          </button>
        </div>
      </div>

      <!-- SEO tarafıyla CRM'i birleştiren tek yer. -->
      <div class="etiket">
        <label class="lbl">İlgilendiği ürünler</label>
        <div v-for="p in store.contactProducts" :key="p.sku" class="urun-sat">
          <Icon name="box" :size="13" :stroke-width="1.9" />
          <span class="urun-ad">{{ p.name === p.sku ? bagliUrunAdlari.get(p.sku) ?? p.sku : p.name }}</span>
          <button class="et-x" :disabled="editBlocked" :aria-label="`${bagliUrunAdlari.get(p.sku) ?? p.name} bağlantısını kaldır`" @click="urunBaginiKaldir(p.sku)">
            <Icon name="x" :size="11" :stroke-width="2.6" />
          </button>
        </div>
        <input
          class="fx"
          :value="urunArama"
          :placeholder="typeof store.contact.id === 'string' ? 'Ortak katalogda ürün ara' : 'Ürün ara ve bağla (en az 3 harf)'"
          aria-label="Bağlanacak ürünü ara"
          @input="urunAra(($event.target as HTMLInputElement).value)"
        />
        <p v-if="urunHatasi" class="hint" role="status">{{ urunHatasi }}</p>
        <div v-if="urunSonuc.length" class="urun-sonuc om-scroll">
          <button v-for="m in urunSonuc" :key="m.sku" class="us-sat" :disabled="editBlocked" @click="urunBagla(m)">
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
        <button class="primary" :disabled="editBlocked" @click="temasEkle">
          {{ savingEvent ? "Kaydediliyor…" : "Temas kaydet" }}
        </button>
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
    <ContactConflictModal v-if="sharedContactId && conflictModal" :key="`${sharedContactId}:${conflictModal}`"
      :id="sharedContactId" :name="store.contact?.name || ''" :mode="conflictModal" :history="contactHistory"
      @close="closeConflictModal" @accepted="contactResolved" />
  </section>
</template>

<style scoped>
.conflict-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin-bottom: 14px;
}
.contact-conflict {
  display: flex;
  flex: 1 1 100%;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 8px 12px;
  padding: 12px;
  border: 1px solid var(--warn-border);
  border-radius: 8px;
  background: var(--warn-bg);
  color: var(--warn-text);
  font-size: 12px;
}
.contact-conflict p { margin: 0; }
.card {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  padding: 20px 24px 40px;
  container-type: inline-size;
  container-name: contact-detail;
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
@container contact-detail (max-width: 420px) {
  .head { flex-wrap: wrap; }
  .head h2 { min-width: 0; overflow-wrap: anywhere; }
  .head-btns { max-width: 100%; flex-wrap: wrap; justify-content: flex-end; margin-inline-start: auto; }
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
@container contact-detail (max-width: 360px) {
  .adim-alan { flex-direction: column; }
  .adim-alan input:first-child { width: 100%; }
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
