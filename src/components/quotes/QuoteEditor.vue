<script setup lang="ts">
/**
 * Teklif düzenleyici (Faz T).
 *
 * 🔴 **Marj yalnızca burada.** Satır ve toplam marjı ekranda görünüyor; müşteriye giden
 * çıktıda (Faz T2) maliyet **alanı bile yok**. Ölçüm bu uyarının gerekçesi: katalogda 7 ürün
 * zararına, 31 ürün %10'un altında.
 *
 * Teklifin para birimi ilk kalem eklenmeden seçilir. Kalemlerin fiyatı ekleme anında hesaplanır.
 */
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { api } from "../../api";
import { useStore } from "../../store";
import { teamAccessIssue, teamProject, teamUser, teamWorkspace } from "../../teamConnection";
import { searchTeamCatalog } from "../../teamCatalog";
import { teamQuoteCanEdit, teamQuoteConflicts, teamQuotesOffline, teamQuoteSaving } from "../../teamQuotes";
import type { CatalogMatch, Contact, ContactId, Quote, QuoteItem } from "../../types";
import Icon from "../Icon.vue";
import QuoteDocModal from "./QuoteDocModal.vue";
import QuoteHistoryModal from "./QuoteHistoryModal.vue";
import QuoteLifecycleDialog from "./QuoteLifecycleDialog.vue";
import QuoteConflictModal from "./QuoteConflictModal.vue";

const store = useStore();
const quoteContacts = ref<Contact[]>([]);
watch([teamUser, teamWorkspace], async ([user, workspace], _, onCleanup) => {
  let canceled = false;
  onCleanup(() => { canceled = true; });
  quoteContacts.value = [];
  if (user && !workspace) return;
  try {
    const contacts = user ? await api.listContacts("", false, "", "") : await api.listLocalContacts();
    if (!canceled) quoteContacts.value = contacts;
  } catch (error) {
    store.toast(String(error), "error");
  }
}, { immediate: true });
const q = computed(() => store.quote);
const central = computed(() => typeof q.value?.id === "string");
const conflict = computed(() => q.value && central.value && teamQuoteConflicts.value.has(q.value.id));
const locked = computed(() => central.value && (!teamQuoteCanEdit.value || q.value?.status !== "draft" || !!conflict.value));
const statusLocked = computed(() => satirEkleniyor.value || central.value && (teamQuotesOffline.value || !teamQuoteCanEdit.value
  || !!conflict.value || teamQuoteSaving.value || store.quoteLifecycleBusy));

/**
 * ⚠️ Bu üç ref aşağıdaki `watch`tan ÖNCE tanımlı olmak ZORUNDA: `immediate: true` olan
 * izleyici kurulum sırasında hemen çalışıp hepsine yazıyor. Aşağıda kalsalardı geçici ölüm
 * bölgesine (TDZ) düşerlerdi.
 *
 * 🔴 Bu hata bu oturumda **İKİ KEZ** yapıldı (önce `ContactCard`, sonra burada) ve ikisini de
 * yalnızca **konsol** yakaladı — ekran her iki durumda da doğru çiziliyordu. Ekran görüntüsü
 * tek başına yeterli bir doğrulama değil (brain.md 0b3).
 */
const urunArama = ref("");
const urunSonuc = ref<CatalogMatch[]>([]);
const urunAramaHatasi = ref("");
const urunSecimi = ref<CatalogMatch | null>(null);
const kurAlani = ref<HTMLInputElement | null>(null);
const satirEkleniyor = ref(false);
const satirKilidi = computed(() => locked.value || satirEkleniyor.value || (central.value && teamQuoteSaving.value));
let aramaIstegi = 0;
let zaman: ReturnType<typeof setTimeout> | null = null;

/** Başlık düzenleme kopyası — kaydetmeden store değişmesin. */
const f = ref(formdan(null));

/** Kayıtlı teklifin başlık alanlarını forma çevirir — hem ilk yükleme hem "Vazgeç" bunu kullanıyor. */
function formdan(v: Quote | null) {
  return {
    contactId: v?.contact_id ?? null,
    currency: v?.currency ?? "TRY",
    fxRate: v?.fx_rate ? String(v.fx_rate) : "",
    validUntil: v?.valid_until?.slice(0, 10) ?? "",
    note: v?.note ?? "",
  };
}

watch(
  () => store.quote,
  (v, previous) => {
    const incoming = formdan(v);
    if (v?.id !== previous?.id) {
      f.value = incoming;
      return;
    }
    const saved = formdan(previous ?? null);
    // Keep edits to unrelated fields when adding a line also saves currency or FX.
    const keepEdit = <K extends keyof typeof saved>(field: K) =>
      f.value[field] === saved[field] ? incoming[field] : f.value[field];
    const currency = v?.items.length ? incoming.currency : keepEdit("currency");
    f.value = {
      contactId: keepEdit("contactId"), currency,
      fxRate: currency === f.value.currency ? keepEdit("fxRate") : incoming.fxRate,
      validUntil: keepEdit("validUntil"), note: keepEdit("note"),
    };
  },
  { immediate: true },
);

watch([() => q.value?.id, teamProject, teamUser, teamWorkspace, teamAccessIssue], () => {
  ++aramaIstegi;
  if (zaman) clearTimeout(zaman);
  urunArama.value = "";
  urunSonuc.value = [];
  urunAramaHatasi.value = "";
  urunSecimi.value = null;
});
onBeforeUnmount(() => { if (zaman) clearTimeout(zaman); });

/**
 * Geri alma yedekleri **yalnızca başka teklife geçilince** siliniyor.
 *
 * 🔴 Önce yukarıdaki izleyicinin içindeydi ve hiç çalışmıyordu: her satır kaydından sonra
 * teklif yeniden okunuyor, `store.quote` yeni bir nesne oluyor, izleyici tetikleniyor ve
 * yedeği **daha görünmeden** siliyordu. Kimliğe bağlı ayrı bir izleyici gerekiyor.
 */
watch(
  () => store.quote?.id,
  () => {
    geriAlYedegi.value = {};
  },
);

/**
 * Kaydedilmemiş başlık değişikliği var mı?
 *
 * ⚠️ Yalnızca **başlık** alanlarını kapsıyor (müşteri, para birimi, kur, geçerlilik, not).
 * Satırlar anında kaydediliyor — bir hücreden çıktığınızda yazılıyorlar. Bu yüzden "Vazgeç"
 * satırları geri almıyor ve düğmenin yanındaki not bunu açıkça söylüyor: sessizce yarım bir
 * söz vermektense sınırı yazmak doğru.
 */
const kirli = computed(
  () => JSON.stringify(f.value) !== JSON.stringify(formdan(store.quote)),
);

const yazilanKur = computed(() => {
  const text = f.value.fxRate.trim();
  if (!/^(?:\d+(?:[.,]\d+)?|[.,]\d+)$/.test(text)) return null;
  const rate = Number(text.replace(",", "."));
  return Number.isFinite(rate) && rate > 0 ? rate : null;
});
const gecersizKur = computed(() => f.value.currency === "USD" && Boolean(f.value.fxRate.trim()) && yazilanKur.value === null);
const paraBirimiDegisti = computed(() => Boolean(q.value && f.value.currency !== q.value.currency));
const urunKurIstiyor = computed(() => {
  const currency = urunSecimi.value?.currency_abbr?.trim().toUpperCase();
  return f.value.currency === "USD" && Boolean(currency && currency !== "USD");
});

function teklifFiyati(v: Quote) {
  const currency = f.value.currency;
  const fxRate = currency === "USD" ? yazilanKur.value : null;
  const fxDate = fxRate === null ? null : fxRate === v.fx_rate ? v.fx_date : new Date().toISOString().slice(0, 10);
  return { currency, fxRate, fxDate };
}

/** Kaydedilmemiş başlık değişikliklerini geri alır. */
function vazgec() {
  f.value = formdan(store.quote);
}

async function kaydet() {
  const v = q.value;
  if (!v || gecersizKur.value || satirEkleniyor.value) return;
  const form = JSON.stringify(f.value);
  const saved = await store.saveQuote({
    id: v.id,
    contactId: f.value.contactId,
    contactName: quoteContacts.value.find((contact) => contact.id === f.value.contactId)?.name,
    ...teklifFiyati(v),
    validUntil: f.value.validUntil || null,
    note: f.value.note,
  });
  if (saved && q.value?.id === v.id && JSON.stringify(f.value) === form) f.value = formdan(q.value);
}

// --- Satır ekleme: katalog araması EOL/CRM ile aynı uçtan ---
const elleAd = ref("");

function urunAra(v: string) {
  const request = ++aramaIstegi;
  urunArama.value = v;
  urunAramaHatasi.value = "";
  urunSecimi.value = null;
  urunSonuc.value = [];
  if (zaman) clearTimeout(zaman);
  if (v.trim().length < 3) return;
  const id = q.value?.id;
  const current = () => request === aramaIstegi && id === q.value?.id;
  zaman = setTimeout(async () => {
    try {
      const results = typeof id === "string" ? await searchTeamCatalog(v) : await api.searchLiveProducts(v);
      if (current()) urunSonuc.value = results;
    } catch (error) {
      if (current()) {
        urunSonuc.value = [];
        urunAramaHatasi.value = String(error);
      }
    }
  }, 220);
}

async function urunSec(product: CatalogMatch) {
  ++aramaIstegi;
  urunSonuc.value = [];
  urunSecimi.value = product;
  if (urunKurIstiyor.value && yazilanKur.value === null) {
    await nextTick();
    kurAlani.value?.focus();
  } else if (!gecersizKur.value) {
    await katalogdanEkle();
  }
}

async function katalogdanEkle() {
  const product = urunSecimi.value;
  const quote = q.value;
  if (!product || !quote || satirKilidi.value || gecersizKur.value
    || (urunKurIstiyor.value && yazilanKur.value === null)) return;
  const pricing = teklifFiyati(quote);
  satirEkleniyor.value = true;
  try {
    const added = await store.addQuoteLine(product.sku, product.name, pricing);
    if (!added || urunSecimi.value?.sku !== product.sku || q.value?.id !== quote.id) return;
    f.value.currency = pricing.currency;
    f.value.fxRate = pricing.fxRate === null ? "" : String(pricing.fxRate);
    urunArama.value = "";
    urunSecimi.value = null;
  } finally {
    satirEkleniyor.value = false;
  }
}

async function elleEkle() {
  if (!elleAd.value.trim() || satirKilidi.value || paraBirimiDegisti.value) return;
  satirEkleniyor.value = true;
  try {
    if (await store.addQuoteLine(null, elleAd.value)) elleAd.value = "";
  } finally {
    satirEkleniyor.value = false;
  }
}

/**
 * Satır düzenlemesinin **geri alma** yedeği.
 *
 * ⚠️ Satırlar anında kaydediliyor (hücreden çıkınca) — hız için doğru, ama yanlış yazılan
 * bir fiyatın eski hâli kaybolurdu. Değişiklikten ÖNCEKİ değerler burada duruyor ve satırın
 * sonundaki mini düğme onları geri yazıyor.
 *
 * Yalnızca **son** değişiklik tutuluyor: derin bir geçmiş yığını teklif düzenleyicide
 * abartı olurdu; ihtiyaç "yanlış tuşa bastım" anını kurtarmak.
 */
const geriAlYedegi = ref<Record<QuoteItem["id"], Pick<QuoteItem, "name" | "qty" | "unit_price" | "tax_rate">>>({});

/** Satır alanı düzenlendiğinde kaydet — her tuşta değil, odak çıkışında. */
function satirKaydet(it: QuoteItem, alan: Partial<QuoteItem>) {
  if (satirKilidi.value) return;
  const yeni = {
    name: (alan.name ?? it.name) as string,
    qty: Number(alan.qty ?? it.qty),
    unit_price: Number(alan.unit_price ?? it.unit_price),
    tax_rate: Number(alan.tax_rate ?? it.tax_rate),
  };
  // Değer gerçekten değiştiyse yedekle; aynıysa eski yedeği ezme.
  const degisti =
    yeni.name !== it.name ||
    yeni.qty !== it.qty ||
    yeni.unit_price !== it.unit_price ||
    yeni.tax_rate !== it.tax_rate;
  if (degisti) {
    geriAlYedegi.value = {
      ...geriAlYedegi.value,
      [it.id]: { name: it.name, qty: it.qty, unit_price: it.unit_price, tax_rate: it.tax_rate },
    };
  }
  void store.updateQuoteLine(it.id, yeni.name, yeni.qty, yeni.unit_price, yeni.tax_rate);
}

async function satirGeriAl(it: QuoteItem) {
  if (satirKilidi.value) return;
  const y = geriAlYedegi.value[it.id];
  if (!y) return;
  await store.updateQuoteLine(it.id, y.name, y.qty, y.unit_price, y.tax_rate);
  const kalan = { ...geriAlYedegi.value };
  delete kalan[it.id];
  geriAlYedegi.value = kalan;
}

const bicim = (v: number) =>
  new Intl.NumberFormat("tr-TR", { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(v);

const belgeAcik = ref(false);
const kapatmaNedeni = ref("");
const historyTrigger = ref<HTMLButtonElement | null>(null);
const historyQuote = ref<{ id: string; number: string } | null>(null);
const canViewHistory = computed(() => typeof q.value?.id === "string" && !!teamUser.value && !!teamWorkspace.value && !teamAccessIssue.value);
const lifecycle = ref<{ action: "revise" | "trash"; id: string; number: string } | null>(null);
const conflictQuote = ref<{ id: string; number: string } | null>(null);
const conflictTrigger = ref<HTMLButtonElement | null>(null);
const heading = ref<HTMLHeadingElement | null>(null);
const reviseTrigger = ref<HTMLButtonElement | null>(null);
const trashTrigger = ref<HTMLButtonElement | null>(null);
const canManageQuote = computed(() => canViewHistory.value && teamQuoteCanEdit.value);

watch([() => q.value?.id, teamProject, teamUser, teamWorkspace, teamAccessIssue], () => {
  historyQuote.value = null;
  lifecycle.value = null;
  conflictQuote.value = null;
}, { flush: "sync" });

function openConflict() {
  const quote = q.value;
  if (conflict.value && typeof quote?.id === "string") conflictQuote.value = { id: quote.id, number: quote.no };
}

async function closeConflict() {
  const current = conflictQuote.value;
  conflictQuote.value = null;
  await nextTick();
  if (current?.id === q.value?.id) (conflictTrigger.value ?? heading.value)?.focus();
}

function useRemoteQuote(remote: Quote) {
  if (conflictQuote.value?.id !== remote.id || q.value?.id !== remote.id) return;
  store.useResolvedQuote(remote);
  store.toast("Ortak teklif sürümü kullanılıyor.", "ok");
  void closeConflict();
}

function openHistory() {
  const quote = q.value;
  if (canViewHistory.value && typeof quote?.id === "string") historyQuote.value = { id: quote.id, number: quote.no };
}

async function closeHistory() {
  const current = historyQuote.value;
  historyQuote.value = null;
  await nextTick();
  if (!historyQuote.value && current?.id === q.value?.id && canViewHistory.value) historyTrigger.value?.focus();
}

function openLifecycle(action: "revise" | "trash") {
  const quote = q.value;
  if (canManageQuote.value && !statusLocked.value && typeof quote?.id === "string") {
    lifecycle.value = { action, id: quote.id, number: quote.no };
  }
}

async function closeLifecycle() {
  const current = lifecycle.value;
  if (store.quoteLifecycleBusy || !current) return;
  lifecycle.value = null;
  await nextTick();
  if (q.value?.id !== current.id || lifecycle.value) return;
  const trigger = current.action === "revise" ? reviseTrigger.value : trashTrigger.value;
  (trigger ?? historyTrigger.value)?.focus();
}

async function confirmLifecycle() {
  const current = lifecycle.value;
  if (!current || !canManageQuote.value || statusLocked.value || q.value?.id !== current.id) return;
  const completed = await (current.action === "revise" ? store.reviseQuote() : store.deleteQuote(current.id));
  if (completed && lifecycle.value === current) await closeLifecycle();
}

/**
 * Gönderim akışı: önce durum yazılıyor, sonra **takip tarihi öneriliyor** (Faz C bağı).
 *
 * ⚠️ Tarih sessizce atanmıyor — kişinin mevcut randevusu ezilebilirdi. Kullanıcı onaylarsa
 * kişinin sonraki adımı yazılıyor ve madde Bugün kuyruğunun **Müşteri** kovasında çıkıyor.
 * Böylece ikinci bir hatırlatma sistemi kurulmuyor; tek yer Faz C'nin kendisi.
 */
async function gonderildi() {
  const v = q.value;
  if (!v) return;
  if (!(await store.setQuoteStatus("sent"))) return;
  if (!v.contact_id) return;

  const gun = 7;
  const t = new Date();
  t.setDate(t.getDate() + gun);
  const tarih = t.toISOString().slice(0, 10);
  const kisi = quoteContacts.value.find((c) => c.id === v.contact_id);
  if (kisi?.next_step_at) {
    // Zaten bir sözü var: üstüne yazmıyoruz, yalnızca hatırlatıyoruz.
    store.toast(`Gönderildi. ${kisi.name} için zaten ${kisi.next_step_at.slice(0, 10)} tarihli bir adım var.`, "ok");
    return;
  }
  takipOnerisi.value = { contactId: v.contact_id, tarih, ad: kisi?.name ?? "" };
}

const takipOnerisi = ref<{ contactId: ContactId; tarih: string; ad: string } | null>(null);

async function takipKur() {
  const t = takipOnerisi.value;
  const v = q.value;
  if (!t || !v) return;
  if (typeof t.contactId === "string") {
    try {
      const contact = await api.getContact(t.contactId);
      if (contact.next_step_at) {
        store.toast(`${contact.name} için zaten bir sonraki adım var.`, "ok");
      } else {
        await store.addContactEvent(t.contactId, "note", `${v.no} takibi`, t.tarih, `${v.no} teklifini hatırlat`, contact.revision);
      }
    } catch (error) {
      store.toast(String(error), "error");
      return;
    }
  } else {
    await store.addContactEvent(t.contactId, "note", `${v.no} takibi`, t.tarih, `${v.no} teklifini hatırlat`);
  }
  takipOnerisi.value = null;
}
const DURUM_EYLEM: Record<string, { to: string; label: string; neden?: boolean }[]> = {
  // ⚠️ "draft → sent" özel bir akış (takip önerisi) — aşağıdaki genel düğmede değil.
  draft: [],
  sent: [
    { to: "won", label: "Kazanıldı" },
    { to: "lost", label: "Kaybedildi", neden: true },
    { to: "draft", label: "Taslağa al" },
  ],
  expired: [{ to: "sent", label: "Yeniden gönderildi" }],
};
const eylemler = computed(() => (DURUM_EYLEM[q.value?.status ?? ""] ?? [])
  .filter(({ to }) => !central.value || to !== "draft"));
</script>

<template>
  <section v-if="q" class="ed om-scroll">
    <div class="head">
      <div>
        <h2 ref="heading" tabindex="-1">{{ q.no }}</h2>
        <div class="h-alt">
          {{ q.status_label }}
          <template v-if="q.sent_at"> · gönderim {{ q.sent_at.slice(0, 10) }}</template>
          <template v-if="q.close_reason"> · {{ q.close_reason }}</template>
        </div>
      </div>
      <div class="head-btns">
        <button class="ghost with-icon" @click="belgeAcik = true">
          <Icon name="eye" :size="14" :stroke-width="2" aria-hidden="true" /> Belge
        </button>
        <button v-if="canViewHistory" ref="historyTrigger" type="button" class="ghost with-icon" aria-haspopup="dialog" @click="openHistory">
          <Icon name="clock" :size="14" aria-hidden="true" /> Gönderim geçmişi
        </button>
        <button v-if="canManageQuote && q.status !== 'draft'" ref="reviseTrigger" type="button" class="ghost with-icon"
          :disabled="statusLocked" aria-haspopup="dialog" @click="openLifecycle('revise')">
          <Icon name="fileEdit" :size="14" aria-hidden="true" /> Düzenle
        </button>
        <button v-if="q.status === 'draft'" class="ghost" :disabled="statusLocked || kirli" @click="gonderildi">
          Gönderildi olarak işaretle
        </button>
        <button
          v-for="e in eylemler"
          :key="e.to"
          class="ghost"
          :disabled="statusLocked"
          @click="store.setQuoteStatus(e.to, e.neden ? kapatmaNedeni : '')"
        >
          {{ e.label }}
        </button>
        <!-- Vazgeç yalnızca değişiklik VARKEN görünüyor: hep duran bir düğme, hiçbir şey
             yapmadığı anlarda gürültüdür. -->
        <button v-if="kirli" class="ghost" @click="vazgec">Vazgeç</button>
        <button v-if="!locked" class="primary" :disabled="!kirli || gecersizKur || satirEkleniyor || (central && teamQuoteSaving)" @click="kaydet">Kaydet</button>
        <button v-if="canManageQuote" ref="trashTrigger" type="button" class="ghost with-icon quote-trash-action"
          :disabled="statusLocked" aria-haspopup="dialog" @click="openLifecycle('trash')">
          <Icon name="archive" :size="14" aria-hidden="true" /> Sil
        </button>
      </div>
    </div>

    <div v-if="conflict" class="quote-conflict">
      <p class="hint" role="status">Bu teklifin cihazdaki değişiklikleri ortak sürümle çakıştı.</p>
      <button ref="conflictTrigger" class="secondary-action" type="button" aria-haspopup="dialog" @click="openConflict">
        Çakışmayı incele
      </button>
    </div>
    <p v-if="kirli && !locked" class="hint kirli-not">
      <Icon name="info" :size="13" />
      Başlık alanlarında kaydedilmemiş değişiklik var. <b>Satır değişiklikleri anında
      kaydediliyor</b>, "Vazgeç" onları geri almaz.
    </p>

    <!-- Takip önerisi: kabul edilirse kişinin sonraki adımı yazılıyor, madde Bugün'e düşüyor. -->
    <div v-if="takipOnerisi" class="takip">
      <Icon name="calendarClock" :size="14" :stroke-width="2" />
      <span>
        <b>{{ takipOnerisi.ad }}</b> için {{ takipOnerisi.tarih }} tarihine takip koyulsun mu?
        O gün Bugün listenizde çıkar.
      </span>
      <button class="t-evet" @click="takipKur">Koy</button>
      <button class="t-hayir" @click="takipOnerisi = null">Gerek yok</button>
    </div>

    <!-- Kaybetme nedeni: raporlanabilmesi fazın bitiş şartı, o yüzden kapatmadan önce isteniyor. -->
    <input
      v-if="q.status === 'sent'"
      v-model="kapatmaNedeni"
      class="fx neden"
      placeholder="Kaybedildiyse nedeni (fiyat, termin, rakip…)"
    />

    <div class="grid">
      <label>
        Müşteri
        <select v-model="f.contactId" class="fx" :disabled="locked">
          <option :value="null">— seçilmedi —</option>
          <option v-if="central && q.contact_id && !quoteContacts.some((c) => c.id === q?.contact_id)" :value="q.contact_id">
            {{ q.contact_name }}
          </option>
          <option v-for="c in quoteContacts" :key="c.id" :value="c.id">
            {{ c.name }}<template v-if="c.company"> · {{ c.company }}</template>
          </option>
        </select>
      </label>
      <label>
        Para birimi
        <select
          v-model="f.currency"
          class="fx"
          :disabled="locked || q.items.length > 0 || satirEkleniyor"
          :title="q.items.length ? 'Para birimi ilk kalem eklenmeden seçilir.' : undefined"
        >
          <option value="TRY">TRY</option>
          <option value="USD">USD</option>
        </select>
      </label>
      <label v-if="f.currency === 'USD' && !urunSecimi">
        USD/TRY kuru
        <input
          v-model="f.fxRate"
          class="fx"
          inputmode="decimal"
          :readonly="locked || satirEkleniyor"
          :aria-invalid="gecersizKur || undefined"
          placeholder="Örn. 47,59"
        />
      </label>
      <label>Geçerlilik <input v-model="f.validUntil" class="fx" type="date" :readonly="locked" /></label>
    </div>

    <p v-if="gecersizKur && !urunSecimi" class="hint" role="status">Geçerli bir pozitif USD/TRY kuru girin.</p>

    <!-- Satırlar -->
    <div class="tablo om-scroll-x">
      <table>
        <!-- Genişlikler tek yerde: başlık ve hücreler ayrı ayrı ayarlanmıyor. -->
        <colgroup>
          <col />
          <col style="width: 78px" />
          <col style="width: 104px" />
          <col style="width: 72px" />
          <col style="width: 108px" />
          <col style="width: 72px" />
          <col style="width: 58px" />
        </colgroup>
        <thead>
          <tr>
            <th class="w-ad">Kalem</th>
            <th class="sag">Adet</th>
            <th class="sag">Birim</th>
            <th class="sag">KDV</th>
            <th class="sag">Tutar</th>
            <th class="sag">Marj</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="it in q.items" :key="it.id">
            <td>
              <input
                class="hucre"
                :readonly="satirKilidi"
                :value="it.name"
                @change="satirKaydet(it, { name: ($event.target as HTMLInputElement).value })"
              />
              <span v-if="!it.sku" class="elle">elle</span>
            </td>
            <td class="sag">
              <input
                class="hucre num"
                :readonly="satirKilidi"
                type="number"
                min="0"
                step="1"
                :value="it.qty"
                @change="satirKaydet(it, { qty: Number(($event.target as HTMLInputElement).value) })"
              />
            </td>
            <td class="sag">
              <input
                class="hucre num"
                :readonly="satirKilidi"
                type="number"
                min="0"
                step="0.01"
                :value="it.unit_price"
                @change="satirKaydet(it, { unit_price: Number(($event.target as HTMLInputElement).value) })"
              />
            </td>
            <td class="sag">
              <input
                class="hucre num"
                :readonly="satirKilidi"
                type="number"
                min="0"
                step="1"
                :value="it.tax_rate"
                @change="satirKaydet(it, { tax_rate: Number(($event.target as HTMLInputElement).value) })"
              />
            </td>
            <td class="sag metin tut">{{ bicim(it.net) }}</td>
            <!-- 🔴 Yalnızca ekranda. -->
            <td class="sag metin">
              <span v-if="it.margin" class="marj" :class="it.margin.state">
                %{{ it.margin.pct.toFixed(0) }}
              </span>
              <span v-else class="marj yok">—</span>
            </td>
            <td v-if="!locked" class="eylem-h">
              <!-- Geri alma yalnızca o satırda değişiklik yapıldıysa görünüyor; yer
                   ayrılmış durumda ki düğme belirince satırlar kaymasın. -->
              <button
                v-if="geriAlYedegi[it.id]"
                class="sil geri"
                title="Bu satırdaki son değişikliği geri al"
                :disabled="satirKilidi"
                @click="satirGeriAl(it)"
              >
                <Icon name="undo" :size="12" :stroke-width="2.2" />
              </button>
              <span v-else class="yer"></span>
              <button class="sil" title="Satırı sil" :disabled="satirKilidi" @click="store.deleteQuoteLine(it.id)">
                <Icon name="x" :size="12" :stroke-width="2.4" />
              </button>
            </td>
            <td v-else></td>
          </tr>
          <tr v-if="!q.items.length">
            <td colspan="7" class="bos">{{ locked ? "Teklifte kalem yok." : "Aşağıdan katalogdan ürün ekleyin veya elle satır yazın." }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Satır ekleme -->
    <div v-if="!locked" class="ekle">
      <div class="ek-alan">
        <input
          class="fx"
          :value="urunArama"
          :readonly="satirKilidi"
          aria-label="Teklife ürün eklemek için katalogda ara"
          :placeholder="central ? 'Ortak katalogda ürün ara' : 'Katalogdan ürün ara (en az 3 harf)'"
          @input="urunAra(($event.target as HTMLInputElement).value)"
        />
        <p v-if="urunAramaHatasi" class="hint" role="status">{{ urunAramaHatasi }}</p>
        <div v-if="urunSonuc.length" class="sonuc om-scroll">
          <button v-for="m in urunSonuc" :key="m.sku" type="button" class="s-sat" :disabled="satirKilidi" @click="urunSec(m)">
            {{ m.name }}
          </button>
        </div>
        <form v-if="urunSecimi" class="kur-ekle" @submit.prevent="katalogdanEkle">
          <strong>{{ urunSecimi.name }}</strong>
          <p v-if="urunKurIstiyor" class="hint">Mağazanın TL fiyatı girdiğiniz USD/TRY kuruyla çevrilir.</p>
          <label v-if="f.currency === 'USD'">
            USD/TRY kuru
            <input
              ref="kurAlani"
              v-model="f.fxRate"
              class="fx"
              inputmode="decimal"
              placeholder="Örn. 47,59"
              :readonly="satirKilidi"
              :aria-invalid="gecersizKur || undefined"
            />
          </label>
          <p v-if="gecersizKur" class="hint" role="status">Geçerli bir pozitif USD/TRY kuru girin.</p>
          <div class="kur-ekle-islemleri">
            <button type="button" class="ghost" :disabled="satirEkleniyor" @click="urunSecimi = null">Vazgeç</button>
            <button
              type="submit"
              class="primary"
              :disabled="satirKilidi || gecersizKur || (urunKurIstiyor && yazilanKur === null)"
            >{{ satirEkleniyor ? 'Ekleniyor…' : 'Ürünü ekle' }}</button>
          </div>
        </form>
      </div>
      <div class="ek-alan">
        <input
          v-model="elleAd"
          class="fx"
          :readonly="satirKilidi || paraBirimiDegisti"
          placeholder="Elle satır (montaj, nakliye…)"
          @keyup.enter="elleEkle"
        />
        <p v-if="paraBirimiDegisti" class="hint">Elle kalem eklemeden önce para birimini kaydedin.</p>
      </div>
    </div>

    <!-- Toplamlar: KDV orana göre kırılıyor (katalogda %20 ve %10 var). -->
    <div class="toplam">
      <div class="t-sat"><span>Ara toplam</span><b>{{ bicim(q.subtotal) }} {{ q.currency }}</b></div>
      <div v-for="t in q.taxes" :key="t.rate" class="t-sat kdv">
        <span>KDV %{{ t.rate.toFixed(0) }} ({{ bicim(t.base) }} üzerinden)</span>
        <b>{{ bicim(t.amount) }}</b>
      </div>
      <div class="t-sat genel"><span>Genel toplam</span><b>{{ bicim(q.grand_total) }} {{ q.currency }}</b></div>
      <div v-if="q.margin" class="t-sat marj-sat" :class="q.margin.state">
        <span>Tahmini marj <i>· yalnızca sizin görüyorsunuz</i></span>
        <b>{{ bicim(q.margin.amount) }} · %{{ q.margin.pct.toFixed(1) }}</b>
      </div>
    </div>

    <p v-if="q.margin?.state === 'negative'" class="warn">
      Bu teklif <b>zararına</b> görünüyor: maliyet, verdiğiniz fiyatın üzerinde.
    </p>
    <p v-else-if="q.margin?.state === 'low'" class="warn">
      Marj %10'un altında. Katalogda 31 ürün bu aralıkta — fiyatı bilerek mi verdiniz?
    </p>

    <label class="tam">Not <textarea v-model="f.note" class="fx" rows="2" :readonly="locked"></textarea></label>

    <div v-if="typeof q.id === 'number'" class="alt">
      <button class="ghost sil-teklif" @click="store.deleteQuote(q.id)">Teklifi sil</button>
    </div>

    <Teleport to="body">
      <QuoteDocModal :open="belgeAcik" :quote-id="q.id" @close="belgeAcik = false" />
    </Teleport>
    <QuoteHistoryModal
      v-if="historyQuote"
      :key="historyQuote.id"
      :quote-id="historyQuote.id"
      :quote-number="historyQuote.number"
      @close="closeHistory"
    />
    <QuoteLifecycleDialog v-if="lifecycle" :action="lifecycle.action" :number="lifecycle.number" :busy="store.quoteLifecycleBusy"
      @close="closeLifecycle" @confirm="confirmLifecycle" />
    <QuoteConflictModal v-if="conflictQuote" :key="conflictQuote.id" :id="conflictQuote.id" :number="conflictQuote.number"
      @close="closeConflict" @accepted="useRemoteQuote" />
  </section>

  <section v-else class="bos-sec">
    <Icon name="fileEdit" :size="30" :stroke-width="1.6" />
    <div>{{ store.quoteTrashOpen ? "Silinen teklifler" : "Soldan bir teklif seçin" }}</div>
  </section>
</template>

<style scoped>
.ed {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  padding: 20px 24px 40px;
  container-type: inline-size;
  container-name: quote-editor;
}
.head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}
h2 {
  font-size: 17px;
  font-weight: 640;
  letter-spacing: -0.015em;
  margin: 0;
  color: var(--c-text);
}
.h-alt {
  font-size: 11.5px;
  color: var(--c-faint);
  margin-top: 2px;
}
.head-btns {
  display: flex;
  gap: 8px;
  min-width: 0;
  flex-wrap: wrap;
  justify-content: flex-end;
}
.head > div:first-child { min-width: 0; overflow-wrap: anywhere; }
.with-icon { display: inline-flex; align-items: center; gap: 6px; }
.with-icon:hover { background: var(--c-hover); }
.with-icon:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
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
.takip {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  margin-bottom: 12px;
  border: 1px solid var(--accent);
  border-radius: 10px;
  background: var(--accent-tint);
  font-size: 12.5px;
  color: var(--c-text);
}
.takip span {
  flex: 1;
}
.t-evet,
.t-hayir {
  flex: none;
  height: 28px;
  padding: 0 12px;
  border-radius: 7px;
  font-size: 12px;
  font-weight: 560;
  cursor: pointer;
  border: 1px solid var(--accent);
  background: var(--accent);
  color: #fff;
}
.t-hayir {
  background: transparent;
  border-color: var(--c-border);
  color: var(--c-mid);
}
.kirli-not {
  margin-bottom: 12px;
}
.quote-conflict {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 12px;
  padding: 9px 12px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-card);
}
.quote-conflict .hint { margin: 0; }
.primary:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.neden {
  width: 100%;
  margin-bottom: 12px;
  padding: 7px 10px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-input);
  color: var(--c-text);
  font-size: 12.5px;
  outline: none;
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
  gap: 12px;
  margin-bottom: 12px;
}
label,
.tam {
  display: block;
  font-size: 11.5px;
  font-weight: 560;
  color: var(--c-faint);
}
.tam {
  margin-top: 14px;
}
input,
select,
textarea {
  display: block;
  width: 100%;
  margin-top: 5px;
  padding: 7px 9px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-input);
  color: var(--c-text);
  font-size: 12.5px;
  font-family: inherit;
  outline: none;
}
.tablo {
  overflow-x: auto;
  border: 1px solid var(--c-border);
  border-radius: 10px;
  margin-top: 14px;
}
/* 🔴 `table-layout: fixed` ZORUNLU (saha hatası, 2026-08-11).
   Otomatik yerleşimde pencere genişledikçe fazla alan sayısal sütunlara da dağılıyor;
   giriş kutusu sabit genişlikte olduğu için hücrenin SOLUNA yapışıyor, başlık ise sağda
   kalıyordu. Dar pencerede hücre zaten giriş kadar olduğu için sorun görünmüyordu — bu
   yüzden ilk düzeltme yetersiz kaldı ve yalnızca tam ekranda ortaya çıktı.
   Sabit yerleşimde genişlikleri aşağıdaki `col`lar belirliyor; fazlalık ilk sütuna gidiyor. */
table {
  width: 100%;
  min-width: 692px; /* Kalem 200px, diğer sütunlar toplam 492px. */
  table-layout: fixed;
  border-collapse: collapse;
  font-size: 12.5px;
}
th,
td {
  padding: 6px 9px;
  border-bottom: 1px solid var(--c-border);
  text-align: left;
}
th {
  font-size: 11px;
  font-weight: 600;
  color: var(--c-faint);
  background: var(--c-list);
}
.sag {
  text-align: right;
}
/* 🔴 Sütun hizası: başlık metni hücrenin sağ kenarında, sayı ise GİRİŞİN iç boşluğu kadar
   içeride duruyordu — üstelik tarayıcının artırma okları sağdan ~18px daha yiyordu, sayı
   gözle görülür biçimde sola kayıyordu (saha geri bildirimi, 2026-08-11).
   Çözüm: okları kaldır, başlık ile girişin sağ iç boşluğunu eşitle. */
th.sag {
  padding-right: 15px;
}
/* Girişli hücrede sağ boşluk ikiye bölünüyor: hücrenin 9px'i + girişin 6px'i = 15px,
   yani başlıkla birebir. */
.hucre.num {
  padding-right: 6px;
}
/* Girişi olmayan (düz metin) sağ hücrelerde o 6px yok — eksiği burada tamamlanıyor,
   yoksa Tutar ve Marj sütunları başlıklarından 6px kayıyor. */
td.sag.metin {
  padding-right: 15px;
}
/* Artırma/azaltma okları gizli: hem hizayı bozuyorlardı hem de bu ekranın diline
   (sakin, az çizgi) yabancıydılar. Değer yazılarak giriliyor. */
.hucre[type="number"] {
  -moz-appearance: textfield;
  appearance: textfield;
}
.hucre[type="number"]::-webkit-outer-spin-button,
.hucre[type="number"]::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
.w-ad {
  min-width: 200px;
}
/* Sabit yerleşimde hücre uzayamıyor; uzun ürün adı üç noktayla kesiliyor. */
td:first-child .hucre {
  text-overflow: ellipsis;
}
tbody tr:last-child td {
  border-bottom: none;
}
.hucre {
  width: 100%;
  margin: 0;
  padding: 3px 6px;
  border: 1px solid transparent;
  background: transparent;
  font-size: 12.5px;
}
.hucre:hover,
.hucre:focus {
  border-color: var(--c-border);
  background: var(--c-input);
}
/* ⚠️ Sabit piksel genişlik YOK: giriş hücreyi dolduruyor ve metni sağa yaslı. Sabit
   kalsaydı geniş hücrede sola yapışır, hiza yine bozulurdu. */
.num {
  text-align: right;
  font-variant-numeric: tabular-nums;
  width: 100%;
}
.tut {
  font-variant-numeric: tabular-nums;
  font-weight: 560;
}
.elle {
  font-size: 10px;
  color: var(--c-faint);
  border: 1px solid var(--c-border);
  border-radius: 5px;
  padding: 0 4px;
  margin-left: 4px;
}
.marj {
  font-variant-numeric: tabular-nums;
  font-weight: 560;
  color: var(--c-mid);
}
.marj.low {
  color: var(--warn-text);
}
.marj.negative {
  color: var(--badge-eksik-c);
}
.marj.yok {
  color: var(--c-faint);
}
.eylem-h {
  white-space: nowrap;
  text-align: right;
}
.sil {
  border: none;
  background: none;
  color: var(--c-faint);
  cursor: pointer;
  padding: 2px;
}
.sil:hover {
  color: var(--badge-eksik-c);
}
.geri:hover {
  color: var(--accent);
}
/* Düğme belirdiğinde satır kaymasın diye yer önceden ayrılıyor. */
.yer {
  display: inline-block;
  width: 16px;
}
.bos {
  text-align: center;
  color: var(--c-faint);
  padding: 18px;
}
.ekle {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  margin-top: 12px;
}
@container quote-editor (max-width: 560px) {
  .head { flex-direction: column; align-items: stretch; overflow-wrap: anywhere; }
  .ekle { grid-template-columns: minmax(0, 1fr); }
}
.ek-alan {
  position: relative;
  min-width: 0;
}
.kur-ekle {
  display: grid;
  gap: 8px;
  margin-top: 8px;
  padding: 12px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-card);
  font-size: 12.5px;
}
.kur-ekle .hint {
  margin: 0;
}
.kur-ekle-islemleri {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: flex-end;
}
.sonuc {
  position: absolute;
  z-index: 5;
  left: 0;
  right: 0;
  max-height: 170px;
  overflow-y: auto;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-card);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
}
.s-sat {
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
.quote-trash-action { color: var(--badge-eksik-c); }
.s-sat:hover {
  background: var(--c-hover);
}
.toplam {
  margin-top: 16px;
  margin-left: auto;
  max-width: 380px;
}
.t-sat {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  padding: 5px 0;
  font-size: 12.5px;
  color: var(--c-mid);
}
.t-sat b {
  font-variant-numeric: tabular-nums;
  color: var(--c-text);
}
.t-sat.kdv {
  color: var(--c-faint);
  font-size: 12px;
}
.t-sat.genel {
  border-top: 1px solid var(--c-border);
  margin-top: 4px;
  padding-top: 8px;
  font-size: 14px;
  font-weight: 620;
  color: var(--c-text);
}
.marj-sat {
  border-top: 1px dashed var(--c-border);
  margin-top: 6px;
  padding-top: 8px;
  color: var(--c-faint);
}
.marj-sat i {
  font-style: normal;
  font-size: 11px;
}
.marj-sat.low b {
  color: var(--warn-text);
}
.marj-sat.negative b {
  color: var(--badge-eksik-c);
}
.alt {
  margin-top: 22px;
}
.sil-teklif {
  color: var(--badge-eksik-c);
}
.bos-sec {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  color: var(--c-faint);
  font-size: 13px;
}
</style>
