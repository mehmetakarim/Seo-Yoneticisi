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
 * Yazdırma / PDF.
 *
 * 🔴 **Ayrı pencere KULLANILMIYOR.** İlk sürüm `window.open` ile yeni pencere açıyordu;
 * Tauri penceresinde açılır pencere desteği kapalı olduğu için `null` dönüyor ve kullanıcı
 * *"Yazdırma penceresi açılamadı"* uyarısını alıyordu (saha hatası, 2026-08-11).
 *
 * Şimdi belge `body`ye ışınlanmış hâlde duruyor; `@media print` uygulamayı gizleyip yalnızca
 * onu bastırıyor. Yazıcı olmasa da işletim sisteminin yazdırma penceresinden **PDF olarak
 * kaydet** seçilebiliyor — asıl ihtiyaç buydu.
 */
function yazdir() {
  window.print();
}
</script>

<template>
  <ModalShell
    :open="props.open"
    label="Teklif belgesi"
    title="Teklif belgesi"
    icon="fileEdit"
    :width="760"
    scroll
    @close="emit('close')"
  >
    <template #sub>Müşteriye gidecek hâli — maliyet ve marj bu belgede yer almaz.</template>

    <div v-if="yukleniyor" class="bekle">Hazırlanıyor…</div>
    <!-- Arka uçtan gelen belge; içerik kaçışı Rust tarafında yapıldı (quote_html::esc). -->
    <div v-else class="onizleme" v-html="html"></div>

    <!-- Yazdırılan kopya: `body` altında duruyor ki `@media print` uygulamayı gizleyip
         yalnızca bunu bastırabilsin. Ekranda görünmüyor (`.yazdir-kap { display:none }`). -->
    <Teleport to="body">
      <div v-if="props.open" class="yazdir-kap" v-html="html"></div>
    </Teleport>

    <template #footer>
      <button class="ghost" @click="emit('close')">Kapat</button>
      <button class="ghost" @click="yazdir">
        <Icon name="external" :size="14" :stroke-width="2" /> Yazdır / PDF
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
   Koyu temada da beyaz kâğıt görünüyor — kasıtlı. */
.onizleme {
  background: #fff;
  border: 1px solid var(--c-border);
  border-radius: 10px;
  padding: 22px;
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
