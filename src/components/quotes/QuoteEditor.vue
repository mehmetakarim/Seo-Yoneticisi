<script setup lang="ts">
/**
 * Teklif düzenleyici (Faz T).
 *
 * 🔴 **Marj yalnızca burada.** Satır ve toplam marjı ekranda görünüyor; müşteriye giden
 * çıktıda (Faz T2) maliyet **alanı bile yok**. Ölçüm bu uyarının gerekçesi: katalogda 7 ürün
 * zararına, 31 ürün %10'un altında.
 *
 * ⚠️ Para birimi değiştirilirse satır fiyatları dokunulmadan kalıyor — 100 USD'lik satır
 * TRY'ye geçince 100 TL olmaz. Ekran bunu söylüyor; sessizce yanlış fiyat üretmek en kötüsü.
 */
import { computed, ref, watch } from "vue";
import { api } from "../../api";
import { useStore } from "../../store";
import type { CatalogMatch, Quote, QuoteItem } from "../../types";
import Icon from "../Icon.vue";
import QuoteDocModal from "./QuoteDocModal.vue";

const store = useStore();
const q = computed(() => store.quote);

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

/** Başlık düzenleme kopyası — kaydetmeden store değişmesin. */
const f = ref(formdan(null));

/** Kayıtlı teklifin başlık alanlarını forma çevirir — hem ilk yükleme hem "Vazgeç" bunu kullanıyor. */
function formdan(v: Quote | null) {
  return {
    contactId: v?.contact_id ?? null,
    currency: v?.currency ?? "USD",
    fxRate: v?.fx_rate ? String(v.fx_rate) : "",
    validUntil: v?.valid_until?.slice(0, 10) ?? "",
    note: v?.note ?? "",
  };
}

watch(
  () => store.quote,
  (v) => {
    f.value = formdan(v);
    urunArama.value = "";
    urunSonuc.value = [];
  },
  { immediate: true },
);

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

/** Kaydedilmemiş başlık değişikliklerini geri alır. */
function vazgec() {
  f.value = formdan(store.quote);
}

function kaydet() {
  const v = q.value;
  if (!v) return;
  const kur = parseFloat(f.value.fxRate.replace(",", "."));
  void store.saveQuote({
    id: v.id,
    contactId: f.value.contactId,
    currency: f.value.currency,
    fxRate: Number.isFinite(kur) && kur > 0 ? kur : null,
    fxDate: Number.isFinite(kur) && kur > 0 ? new Date().toISOString().slice(0, 10) : null,
    validUntil: f.value.validUntil || null,
    note: f.value.note,
  });
}

// --- Satır ekleme: katalog araması EOL/CRM ile aynı uçtan ---
const elleAd = ref("");
let zaman: ReturnType<typeof setTimeout> | null = null;

function urunAra(v: string) {
  urunArama.value = v;
  if (zaman) clearTimeout(zaman);
  if (v.trim().length < 3) {
    urunSonuc.value = [];
    return;
  }
  zaman = setTimeout(async () => {
    urunSonuc.value = await api.searchLiveProducts(v).catch(() => []);
  }, 220);
}

async function katalogdanEkle(m: CatalogMatch) {
  // ⚠️ `slug` DEĞİL `sku`: slug adresin son parçası. Karıştırılınca arka uç ürünü bulamıyor
  // ("Ürün bulunamadı: Query returned no rows" — saha hatası, 2026-08-12).
  await store.addQuoteLine(m.sku, m.name);
  urunArama.value = "";
  urunSonuc.value = [];
}

async function elleEkle() {
  if (!elleAd.value.trim()) return;
  await store.addQuoteLine(null, elleAd.value);
  elleAd.value = "";
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
const geriAlYedegi = ref<Record<number, Pick<QuoteItem, "name" | "qty" | "unit_price" | "tax_rate">>>({});

/** Satır alanı düzenlendiğinde kaydet — her tuşta değil, odak çıkışında. */
function satirKaydet(it: QuoteItem, alan: Partial<QuoteItem>) {
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
  await store.setQuoteStatus("sent");
  if (!v.contact_id) return;

  const gun = 7;
  const t = new Date();
  t.setDate(t.getDate() + gun);
  const tarih = t.toISOString().slice(0, 10);
  const kisi = store.contacts.find((c) => c.id === v.contact_id);
  if (kisi?.next_step_at) {
    // Zaten bir sözü var: üstüne yazmıyoruz, yalnızca hatırlatıyoruz.
    store.toast(`Gönderildi. ${kisi.name} için zaten ${kisi.next_step_at.slice(0, 10)} tarihli bir adım var.`, "ok");
    return;
  }
  takipOnerisi.value = { contactId: v.contact_id, tarih, ad: kisi?.name ?? "" };
}

const takipOnerisi = ref<{ contactId: number; tarih: string; ad: string } | null>(null);

async function takipKur() {
  const t = takipOnerisi.value;
  const v = q.value;
  if (!t || !v) return;
  await store.addContactEvent(t.contactId, "note", `${v.no} takibi`, t.tarih, `${v.no} teklifini hatırlat`);
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
const eylemler = computed(() => DURUM_EYLEM[q.value?.status ?? ""] ?? []);
</script>

<template>
  <section v-if="q" class="ed om-scroll">
    <div class="head">
      <div>
        <h2>{{ q.no }}</h2>
        <div class="h-alt">
          {{ q.status_label }}
          <template v-if="q.sent_at"> · gönderim {{ q.sent_at.slice(0, 10) }}</template>
          <template v-if="q.close_reason"> · {{ q.close_reason }}</template>
        </div>
      </div>
      <div class="head-btns">
        <button class="ghost" @click="belgeAcik = true">
          <Icon name="eye" :size="14" :stroke-width="2" /> Belge
        </button>
        <button v-if="q.status === 'draft'" class="ghost" @click="gonderildi">
          Gönderildi olarak işaretle
        </button>
        <button
          v-for="e in eylemler"
          :key="e.to"
          class="ghost"
          @click="store.setQuoteStatus(e.to, e.neden ? kapatmaNedeni : '')"
        >
          {{ e.label }}
        </button>
        <!-- Vazgeç yalnızca değişiklik VARKEN görünüyor: hep duran bir düğme, hiçbir şey
             yapmadığı anlarda gürültüdür. -->
        <button v-if="kirli" class="ghost" @click="vazgec">Vazgeç</button>
        <button class="primary" :disabled="!kirli" @click="kaydet">Kaydet</button>
      </div>
    </div>

    <p v-if="kirli" class="hint kirli-not">
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
        <select v-model.number="f.contactId" class="fx">
          <option :value="null">— seçilmedi —</option>
          <option v-for="c in store.contacts" :key="c.id" :value="c.id">
            {{ c.name }}<template v-if="c.company"> · {{ c.company }}</template>
          </option>
        </select>
      </label>
      <label>
        Para birimi
        <select v-model="f.currency" class="fx">
          <option value="USD">USD</option>
          <option value="TRY">TRY</option>
        </select>
      </label>
      <label>
        USD/TRY kuru
        <input v-model="f.fxRate" class="fx" placeholder="ör. 47,59" />
      </label>
      <label>Geçerlilik <input v-model="f.validUntil" class="fx" type="date" /></label>
    </div>

    <!-- Kur yalnızca bu durumda gerekiyor; her zaman uyarı göstermek gürültü olurdu. -->
    <p v-if="f.currency === 'USD'" class="hint">
      <Icon name="info" :size="13" />
      Kataloğunuzda USD dışı ürünler de var (EUR, TL). USD teklifte onların fiyatı ancak
      <b>kur girilirse</b> hesaplanabiliyor; TRY teklifte kur gerekmiyor.
    </p>

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
                :value="it.name"
                @change="satirKaydet(it, { name: ($event.target as HTMLInputElement).value })"
              />
              <span v-if="!it.sku" class="elle">elle</span>
            </td>
            <td class="sag">
              <input
                class="hucre num"
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
            <td class="eylem-h">
              <!-- Geri alma yalnızca o satırda değişiklik yapıldıysa görünüyor; yer
                   ayrılmış durumda ki düğme belirince satırlar kaymasın. -->
              <button
                v-if="geriAlYedegi[it.id]"
                class="sil geri"
                title="Bu satırdaki son değişikliği geri al"
                @click="satirGeriAl(it)"
              >
                <Icon name="undo" :size="12" :stroke-width="2.2" />
              </button>
              <span v-else class="yer"></span>
              <button class="sil" title="Satırı sil" @click="store.deleteQuoteLine(it.id)">
                <Icon name="x" :size="12" :stroke-width="2.4" />
              </button>
            </td>
          </tr>
          <tr v-if="!q.items.length">
            <td colspan="7" class="bos">Aşağıdan katalogdan ürün ekleyin veya elle satır yazın.</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Satır ekleme -->
    <div class="ekle">
      <div class="ek-alan">
        <input
          class="fx"
          :value="urunArama"
          placeholder="Katalogdan ürün ara (en az 3 harf)"
          @input="urunAra(($event.target as HTMLInputElement).value)"
        />
        <div v-if="urunSonuc.length" class="sonuc om-scroll">
          <button v-for="m in urunSonuc" :key="m.slug" class="s-sat" @click="katalogdanEkle(m)">
            {{ m.name }}
          </button>
        </div>
      </div>
      <div class="ek-alan">
        <input
          v-model="elleAd"
          class="fx"
          placeholder="Elle satır (montaj, nakliye…)"
          @keyup.enter="elleEkle"
        />
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

    <label class="tam">Not <textarea v-model="f.note" class="fx" rows="2"></textarea></label>

    <div class="alt">
      <button class="ghost sil-teklif" @click="store.deleteQuote(q.id)">Teklifi sil</button>
    </div>

    <QuoteDocModal :open="belgeAcik" :quote-id="q.id" @close="belgeAcik = false" />
  </section>

  <section v-else class="bos-sec">
    <Icon name="fileEdit" :size="30" :stroke-width="1.6" />
    <div>Soldan bir teklif seçin</div>
  </section>
</template>

<style scoped>
.ed {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  padding: 20px 24px 40px;
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
  flex: none;
  flex-wrap: wrap;
  justify-content: flex-end;
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
.ek-alan {
  position: relative;
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
