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
import type { CatalogMatch, QuoteItem } from "../../types";
import Icon from "../Icon.vue";

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
const f = ref({ contactId: null as number | null, currency: "USD", fxRate: "", validUntil: "", note: "" });
watch(
  () => store.quote,
  (v) => {
    f.value = {
      contactId: v?.contact_id ?? null,
      currency: v?.currency ?? "USD",
      fxRate: v?.fx_rate ? String(v.fx_rate) : "",
      validUntil: v?.valid_until?.slice(0, 10) ?? "",
      note: v?.note ?? "",
    };
    urunArama.value = "";
    urunSonuc.value = [];
  },
  { immediate: true },
);

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
  await store.addQuoteLine(m.slug, m.name);
  urunArama.value = "";
  urunSonuc.value = [];
}

async function elleEkle() {
  if (!elleAd.value.trim()) return;
  await store.addQuoteLine(null, elleAd.value);
  elleAd.value = "";
}

/** Satır alanı düzenlendiğinde kaydet — her tuşta değil, odak çıkışında. */
function satirKaydet(it: QuoteItem, alan: Partial<QuoteItem>) {
  void store.updateQuoteLine(
    it.id,
    (alan.name ?? it.name) as string,
    Number(alan.qty ?? it.qty),
    Number(alan.unit_price ?? it.unit_price),
    Number(alan.tax_rate ?? it.tax_rate),
  );
}

const bicim = (v: number) =>
  new Intl.NumberFormat("tr-TR", { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(v);

const kapatmaNedeni = ref("");
const DURUM_EYLEM: Record<string, { to: string; label: string; neden?: boolean }[]> = {
  draft: [{ to: "sent", label: "Gönderildi olarak işaretle" }],
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
        <button
          v-for="e in eylemler"
          :key="e.to"
          class="ghost"
          @click="store.setQuoteStatus(e.to, e.neden ? kapatmaNedeni : '')"
        >
          {{ e.label }}
        </button>
        <button class="primary" @click="kaydet">Kaydet</button>
      </div>
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
                class="hucre num kisa"
                type="number"
                min="0"
                step="1"
                :value="it.tax_rate"
                @change="satirKaydet(it, { tax_rate: Number(($event.target as HTMLInputElement).value) })"
              />
            </td>
            <td class="sag tut">{{ bicim(it.net) }}</td>
            <!-- 🔴 Yalnızca ekranda. -->
            <td class="sag">
              <span v-if="it.margin" class="marj" :class="it.margin.state">
                %{{ it.margin.pct.toFixed(0) }}
              </span>
              <span v-else class="marj yok">—</span>
            </td>
            <td>
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
table {
  width: 100%;
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
.w-ad {
  min-width: 200px;
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
.num {
  text-align: right;
  font-variant-numeric: tabular-nums;
  width: 84px;
}
.kisa {
  width: 60px;
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
