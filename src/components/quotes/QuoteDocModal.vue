<script setup lang="ts">
/**
 * Teklif belgesi — önizleme, panoya kopyalama ve yazdırma (Faz T2).
 *
 * 🔴 **Maliyet burada YOK ve olamaz.** Belge arka uçta `QuoteOut`tan üretiliyor; o yapıda
 * maliyet alanı bulunmuyor (bkz. `quote_html.rs`). Bu bileşen yalnızca gelen HTML'i
 * gösteriyor, kendi hesabını yapmıyor.
 *
 * ## ⚠️ Kopyalama neden `writeText` değil
 *
 * Uygulamadaki diğer "Kopyala" düğmeleri `writeText` kullanıyor — düz metin. HTML'i öyle
 * kopyalarsak mail'e **kaynak kod** yapışır (`<div style=...`), tablo değil. Bu yüzden
 * `navigator.clipboard.write` + `ClipboardItem` ile **iki biçim birden** konuyor:
 * `text/html` (Gmail, Outlook biçimli alır) ve `text/plain` (düz metin istemciler).
 *
 * Tarayıcı bunu reddederse (izin, güvenli bağlam) düz metne düşülüyor — sessiz kalmak yerine
 * kullanıcıya hangi biçimin kopyalandığı söyleniyor.
 */
import { ref, watch } from "vue";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { api } from "../../api";
import { useStore } from "../../store";
import Icon from "../Icon.vue";
import ModalShell from "../ModalShell.vue";

const props = defineProps<{ open: boolean; quoteId: number | null }>();
const emit = defineEmits<{ close: [] }>();
const store = useStore();

const html = ref("");
const metin = ref("");
const yukleniyor = ref(false);

watch(
  () => [props.open, props.quoteId],
  async () => {
    if (!props.open || !props.quoteId) return;
    yukleniyor.value = true;
    try {
      const d = await api.renderQuote(props.quoteId);
      html.value = d.html;
      metin.value = d.text;
    } catch (e) {
      store.toast(String(e), "error");
    } finally {
      yukleniyor.value = false;
    }
  },
  { immediate: true },
);

async function kopyala() {
  try {
    // Biçimli kopyalama: mail'e tablo olarak yapışsın.
    const item = new ClipboardItem({
      "text/html": new Blob([html.value], { type: "text/html" }),
      "text/plain": new Blob([metin.value], { type: "text/plain" }),
    });
    await navigator.clipboard.write([item]);
    store.toast("Teklif panoya kopyalandı — mail'e biçimli yapışır.", "ok");
  } catch {
    // Yedek yol: en azından okunur düz metin.
    await writeText(metin.value);
    store.toast("Biçimli kopyalama desteklenmedi; düz metin kopyalandı.", "ok");
  }
}

/**
 * PDF / yazdırma — belge varsayılan tarayıcıda açılıyor.
 *
 * 🔴 **Üçüncü deneme; ilk ikisi çalışmadı.**
 * 1. `window.open` + yazdır → Tauri'de açılır pencere yok, `null` döndü.
 * 2. Uygulama içinden `window.print()` → macOS'ta WKWebView bu çağrıyı **uygulamıyor**;
 *    düğme sessizce hiçbir şey yapardı ki bu en kötüsü.
 * 3. ✅ Belge geçici dosyaya yazılıp tarayıcıda açılıyor. Yazdırma penceresi her platformda
 *    **PDF olarak kaydet** seçeneğini veriyor — kullanıcının istediği buydu.
 *
 * ⚠️ Dosyayı **Rust açıyor**, buradan `openPath` çağrılmıyor: opener eklentisinin JS kapsamı
 * geçici klasördeki yolu reddediyordu (*"Not allowed to open path /var/folders/…"*).
 *
 * ⚠️ İki adım (aç → Cmd/Ctrl+P) ama çalışması garanti. Uygulama içinde gerçek PDF üretmek
 * font gömme ve sayfa düzeni demek; yol haritasında bilinçli olarak kapsam dışı.
 */
async function pdfAc() {
  if (!props.quoteId) return;
  try {
    await api.exportQuoteHtml(props.quoteId);
    store.toast("Belge tarayıcıda açıldı — yazdırma penceresinden PDF olarak kaydedin.", "ok");
  } catch (e) {
    store.toast(String(e), "error");
  }
}
</script>

<template>
  <ModalShell
    :open="props.open"
    label="Teklif belgesi"
    title="Teklif belgesi"
    icon="fileEdit"
    :width="652"
    scroll
    @close="emit('close')"
  >
    <template #sub>Müşteriye gidecek hâli — maliyet ve marj bu belgede yer almaz.</template>

    <div v-if="yukleniyor" class="bekle">Hazırlanıyor…</div>
    <!-- Arka uçtan gelen belge; içerik kaçışı Rust tarafında yapıldı (quote_html::esc). -->
    <div v-else class="onizleme" v-html="html"></div>

    <!-- ⚠️ Işınlanan yazdırma kopyası ve `@media print` kuralları KALDIRILDI. Yazdırma
         tarayıcıda yapılıyor; kurallar dursaydı uygulamada Cmd+P **bomboş sayfa** basardı
         (kural `#app`i gizliyor, gösterilecek kopya ise artık yok). -->

    <template #footer>
      <button class="ghost" @click="emit('close')">Kapat</button>
      <button class="ghost" @click="pdfAc">
        <Icon name="external" :size="14" :stroke-width="2" /> PDF olarak kaydet
      </button>
      <button class="solid" @click="kopyala">
        <Icon name="copy" :size="14" :stroke-width="2" /> Panoya kopyala
      </button>
    </template>
  </ModalShell>
</template>

<style scoped>
.bekle {
  padding: 40px;
  text-align: center;
  color: var(--c-faint);
  font-size: 13px;
}
/* Belge kendi (açık) rengini taşıyor: müşteriye gidecek hâli neyse önizleme de o.
   Koyu temada da beyaz kâğıt görünüyor — kasıtlı.

   ⚠️ Modal genişliği belgeye göre (560 + 2×22 dolgu): sağda boş bir şerit kalıyordu
   (saha geri bildirimi). Belge ayrıca ortalanıyor — kap belgeden geniş kalırsa bile
   sola yapışmasın. */
.onizleme {
  background: #fff;
  border: 1px solid var(--c-border);
  border-radius: 10px;
  padding: 22px;
  display: flex;
  justify-content: center;
}
.solid,
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
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.solid {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}
</style>
