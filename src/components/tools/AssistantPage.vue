<script setup lang="ts">
/**
 * Yapay Zekâ Asistanı — analiz verisini konuşarak sorgulama.
 *
 * ⚠️ **Asistan hiçbir şey YAZMAZ.** Canonical, meta, IdeaSoft gönderimi — hepsi kendi açık
 * onaylı akışlarında kalıyor. Kullanıcının "toplu değil, tek tek, onayla" kuralı bir sohbet
 * arayüzüyle delinmez. Asistan okur, yorumlar, önceliklendirir; uygular kullanıcı.
 *
 * ⚠️ Bağlam **bulunulan ekranın** verisi (store.assistantContext). Bu yüzden üstte hangi
 * ekranın verisiyle konuşulduğu açıkça yazıyor — kullanıcı asistanın neyi "gördüğünü"
 * bilmeden sorduğu soruya güvenemez.
 */
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useStore } from "../../store";
import { NAV } from "../../navigation";
import Icon from "../Icon.vue";
import ModalShell from "../ModalShell.vue";
import MarkdownText from "../MarkdownText.vue";

const store = useStore();
const draft = ref("");
const scroller = ref<HTMLElement | null>(null);

// --- Geçmiş sohbetler ---
const histOpen = ref(false);
/** Silme onayı bekleyen sohbet; `confirmAll` ise tüm geçmiş. */
const pendingDelete = ref<number | null>(null);
const confirmAll = ref(false);

const deletingTitle = computed(
  () => store.chatSessions.find((s) => s.id === pendingDelete.value)?.title ?? "",
);

const toolLabel = (key: string) => NAV.find((n) => n.key === key)?.label ?? "";
const fmtDate = (s: string) => s.replace("T", " ").slice(0, 16);

async function openSession(id: number) {
  await store.openChatSession(id);
  histOpen.value = false;
  await nextTick();
  scrollDown();
}

function askDelete(id: number) {
  pendingDelete.value = id;
}
function cancelDelete() {
  pendingDelete.value = null;
  confirmAll.value = false;
}
async function doDelete() {
  if (confirmAll.value) await store.deleteAllChatSessions();
  else if (pendingDelete.value !== null) await store.deleteChatSession(pendingDelete.value);
  cancelDelete();
  if (!store.chatSessions.length) histOpen.value = false;
}

/** Bağlamın hangi ekrandan geldiği. Araç ekranı değilse asistanın eli boş. */
const source = computed(() => {
  const item = NAV.find((n) => n.key === store.lastToolPage);
  return item?.title ?? "";
});
const hasReport = computed(() => !!store.opportunity);

/**
 * ⚠️ Önbelleği bu ekran da yüklemeli. Aksi halde uygulamayı açıp doğrudan asistana geçen
 * kullanıcı "veri yok" görüyor — oysa analiz veritabanında duruyor. (Doğrulama sırasında
 * yakalandı: araç ekranlarına uğramadan gelince asistan boş kalıyordu.)
 */
onMounted(() => {
  if (!store.opportunity) void store.loadOpportunityCache();
  // Geçmiş her açılışta tazelenir — uygulama yeniden başlatıldığında sohbetler burada olmalı.
  void store.loadChatSessions();
});

const SUGGESTIONS = [
  "Bu listede en çok tıklama kaçıran 5 sayfa hangileri ve neden?",
  "Hangi kategoriye odaklanırsam en çok kazancım olur?",
  "Bu verideki en acil üç işi sırala.",
  "Bir sayfanın konumu iyi ama tıklaması düşükse ne yapmalıyım?",
];

async function send(text?: string) {
  const q = text ?? draft.value;
  if (!q.trim() || store.chatBusy) return;
  draft.value = "";
  const p = store.askAssistant(q);
  await nextTick();
  scrollDown();
  await p;
  await nextTick();
  scrollDown();
}

function scrollDown() {
  const el = scroller.value;
  if (el) el.scrollTop = el.scrollHeight;
}

// Akış sürerken de alta yapış: cevap büyüdükçe kullanıcı elle kaydırmasın.
watch(
  // `at(-1)` değil: tsconfig hedefi ES2022'nin altında.
  () => store.chat[store.chat.length - 1]?.text,
  async () => {
    await nextTick();
    scrollDown();
  },
);
</script>

<template>
  <div class="page">
    <div class="ctx">
      <Icon name="info" :size="12" />
      <span v-if="!hasReport">
        Henüz analiz çalıştırılmadı — asistanın elinde veri yok.
        <a class="link" @click="store.page = 'overview'">Genel Bakış</a>'tan çalıştırın.
      </span>
      <span v-else-if="source">
        <b>{{ source }}</b> ekranının verisiyle konuşuyorsunuz. Başka bir aracın verisini
        sormak için önce o ekrana gidin.
      </span>
      <span v-else>Genel özet verisiyle konuşuyorsunuz.</span>
      <div style="flex: 1"></div>
      <span v-if="store.chatModel" class="model-tag">{{ store.chatModel }}</span>
      <a
        v-if="store.chatSessions.length"
        class="link"
        @click="histOpen = !histOpen"
      >
        geçmiş sohbetler ({{ store.chatSessions.length }})
      </a>
      <button v-if="store.chat.length" class="clear" :disabled="store.chatBusy" @click="store.newChat()">
        Yeni sohbet
      </button>
    </div>

    <!-- Geçmiş listesi: kart geçmişindeki `hist-line` deseninin aynısı — açılır, yer kaplamaz. -->
    <div v-if="histOpen && store.chatSessions.length" class="hist">
      <div
        v-for="s in store.chatSessions"
        :key="s.id"
        class="h-row"
        :class="{ on: store.chatId === s.id }"
        @click="openSession(s.id)"
      >
        <div class="h-main">
          <div class="h-title">{{ s.title }}</div>
          <div class="h-meta">
            {{ fmtDate(s.updated_at) }} · {{ s.messages }} mesaj
            <template v-if="toolLabel(s.tool_page)"> · {{ toolLabel(s.tool_page) }}</template>
            <template v-if="s.model"> · {{ s.model }}</template>
          </div>
        </div>
        <!-- Silme kullanıcı insiyatifinde; satırı açmasın diye tıklama durduruluyor. -->
        <button class="h-del" title="Bu sohbeti sil" @click.stop="askDelete(s.id)">
          <Icon name="x" :size="13" :stroke-width="2.2" />
        </button>
      </div>
      <div class="h-foot">
        <a class="link danger" @click="confirmAll = true">Tüm geçmişi sil</a>
      </div>
    </div>

    <!-- Silme onayı: geçmiş geri getirilemez, sessizce silinmemeli. -->
    <ModalShell
      :open="pendingDelete !== null || confirmAll"
      label="Sohbet silme onayı"
      icon="alert"
      :title="confirmAll ? 'Tüm sohbet geçmişi silinecek' : 'Bu sohbet silinecek'"
      :sub="confirmAll ? `${store.chatSessions.length} sohbet` : deletingTitle"
      :width="420"
      @close="cancelDelete()"
    >
      <div class="warn">
        <Icon name="info" :size="13" />
        <span>Silinen sohbet geri getirilemez. Analiz verileriniz etkilenmez.</span>
      </div>
      <template #footer>
        <button class="ghost" @click="cancelDelete()">Vazgeç</button>
        <div style="flex: 1"></div>
        <button class="danger-btn" @click="doDelete()">
          <Icon name="x" :size="14" :stroke-width="2.4" /> Sil
        </button>
      </template>
    </ModalShell>

    <div ref="scroller" class="thread om-scroll">
      <!-- Boş durum: asistanın ne işe yaradığını anlatmanın en kısa yolu örnek sorular. -->
      <div v-if="!store.chat.length" class="intro">
        <div class="icon-badge"><Icon name="message" :size="15" /></div>
        <p class="i-title">Verinizi konuşarak sorgulayın</p>
        <p class="i-sub">
          Asistan yalnızca ekrandaki Search Console verisine bakar; veride olmayan bir şey
          sorulursa bilmediğini söyler. Hiçbir değişiklik yapmaz — öneri verir, uygulamak size
          kalır.
        </p>
        <div class="sugg">
          <button
            v-for="s in SUGGESTIONS"
            :key="s"
            class="s-item"
            :disabled="!hasReport || store.chatBusy"
            @click="send(s)"
          >
            {{ s }}
          </button>
        </div>
      </div>

      <!-- ⚠️ `:class="m.role"` KULLANMA: `role` değeri "model" ve bu, üstteki model
           rozetinin `.model` kuralına takılıyordu (koyu temada satır tamamen çip
           arka planına bürünüyordu). Sınıf adı veriden türetilmemeli. -->
      <div v-for="(m, i) in store.chat" :key="i" class="msg" :class="`from-${m.role}`">
        <div class="bubble">
          <MarkdownText v-if="m.text" :text="m.text" />
          <!-- Düşünce parçaları filtrelendiği için ilk saniyelerde balon boş kalır;
               burada kör bir döner ikon değil, modelin GERÇEKTEN çalıştığı sinyali var. -->
          <div v-else-if="store.chatThinking" class="think">
            <span class="d"></span><span class="d"></span><span class="d"></span>
            düşünüyor…
          </div>
        </div>
      </div>
    </div>

    <div class="composer">
      <textarea
        v-model="draft"
        class="input"
        rows="1"
        placeholder="Veriye dair bir şey sorun… (⌘↵ ile gönder)"
        :disabled="!hasReport"
        @keydown.enter.exact.prevent="send()"
        @keydown.meta.enter.prevent="send()"
        @keydown.ctrl.enter.prevent="send()"
      ></textarea>
      <button class="send" :disabled="!draft.trim() || store.chatBusy || !hasReport" @click="send()">
        <Icon :name="store.chatBusy ? 'loader' : 'sparkles'" :size="15" :class="{ spin: store.chatBusy }" />
        {{ store.chatBusy ? "Yanıtlanıyor…" : "Sor" }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.page {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  padding: 14px 22px 18px;
}
.ctx {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: none;
  margin-bottom: 12px;
  padding: 8px 12px;
  border-radius: 9px;
  background: var(--c-list);
  border: 1px solid var(--c-border-soft);
  font-size: 11.5px;
  color: var(--c-soft);
}
.model-tag {
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--c-chip);
  color: var(--c-soft);
  font-size: 10.5px;
  font-weight: 600;
}
.clear {
  border: none;
  background: transparent;
  color: var(--accent);
  font-size: 11.5px;
  font-weight: 560;
  cursor: pointer;
  font-family: inherit;
}
.clear:disabled {
  opacity: 0.5;
  cursor: default;
}

/* Geçmiş sohbet listesi */
.hist {
  flex: none;
  margin-bottom: 12px;
  max-height: 240px;
  overflow-y: auto;
  border: 1px solid var(--c-border-soft);
  border-radius: 10px;
  background: var(--c-card);
}
.h-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 12px;
  border-bottom: 1px solid var(--c-border-soft);
  cursor: pointer;
}
.h-row:last-of-type {
  border-bottom: 0;
}
.h-row:hover {
  background: var(--c-hover);
}
/* Açık olan sohbet: hangi konuşmanın ekranda olduğu listeden de görünsün. */
.h-row.on {
  background: var(--accent-tint);
}
.h-main {
  flex: 1;
  min-width: 0;
}
.h-title {
  font-size: 12.5px;
  font-weight: 560;
  color: var(--c-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.h-meta {
  margin-top: 2px;
  font-size: 10.5px;
  color: var(--c-faint);
}
.h-del {
  flex: none;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--c-faint);
  cursor: pointer;
}
.h-del:hover {
  background: var(--badge-eksik-bg);
  color: var(--badge-eksik-c);
}
.h-foot {
  padding: 8px 12px;
  border-top: 1px solid var(--c-border-soft);
  font-size: 11.5px;
}
.link.danger {
  color: var(--badge-eksik-c);
}
.ghost {
  height: 38px;
  padding: 0 14px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 12.5px;
  font-weight: 560;
  cursor: pointer;
  font-family: inherit;
}
.ghost:hover {
  background: var(--c-hover);
}
.danger-btn {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  height: 38px;
  padding: 0 16px;
  border: none;
  border-radius: 9px;
  background: var(--badge-eksik-c);
  color: #fff;
  font-size: 13px;
  font-weight: 590;
  cursor: pointer;
  font-family: inherit;
}
.danger-btn:hover {
  filter: brightness(1.06);
}

.thread {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding-right: 4px;
}
.intro {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 40px 20px 20px;
}
.i-title {
  margin: 12px 0 0;
  font-size: 15px;
  font-weight: 620;
  color: var(--c-text);
}
.i-sub {
  margin: 6px 0 0;
  max-width: 480px;
  font-size: 12.5px;
  line-height: 1.6;
  color: var(--c-soft);
}
.sugg {
  display: flex;
  flex-direction: column;
  gap: 7px;
  margin-top: 20px;
  width: 100%;
  max-width: 480px;
}
.s-item {
  padding: 9px 12px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-card);
  color: var(--c-mid);
  font-size: 12.5px;
  text-align: left;
  cursor: pointer;
  font-family: inherit;
  transition: border-color 0.14s cubic-bezier(0.32, 0.72, 0, 1);
}
.s-item:hover:not(:disabled) {
  border-color: var(--accent);
  color: var(--c-text);
}
.s-item:disabled {
  opacity: 0.45;
  cursor: default;
}

.msg {
  display: flex;
  margin-bottom: 10px;
}
.msg.from-user {
  justify-content: flex-end;
}
.bubble {
  max-width: 78%;
  padding: 10px 13px;
  border-radius: 13px;
  background: var(--c-card);
  border: 1px solid var(--c-border-soft);
  color: var(--c-text);
}
.msg.from-user .bubble {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}
/* Kullanıcı balonunda kod parçası accent üzerinde okunmalı. */
.msg.from-user .bubble :deep(code) {
  background: rgba(255, 255, 255, 0.22);
  color: #fff;
}
.think {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  color: var(--c-soft);
}
.think .d {
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: var(--c-soft);
  animation: think 1.1s ease-in-out infinite;
}
.think .d:nth-child(2) {
  animation-delay: 0.16s;
}
.think .d:nth-child(3) {
  animation-delay: 0.32s;
  margin-right: 4px;
}
@keyframes think {
  0%, 60%, 100% { opacity: 0.3; }
  30% { opacity: 1; }
}

.composer {
  flex: none;
  display: flex;
  align-items: flex-end;
  gap: 9px;
  margin-top: 12px;
}
.input {
  flex: 1;
  min-height: 40px;
  max-height: 140px;
  padding: 10px 12px;
  border: 1px solid var(--c-border);
  border-radius: 10px;
  background: var(--c-input);
  color: var(--c-text);
  font-size: 12.5px;
  font-family: inherit;
  line-height: 1.5;
  resize: vertical;
}
.input:focus {
  outline: none;
  border-color: var(--accent);
}
.input:disabled {
  opacity: 0.55;
}
.send {
  flex: none;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 40px;
  padding: 0 16px;
  border: none;
  border-radius: 10px;
  background: var(--accent);
  color: #fff;
  font-size: 13px;
  font-weight: 590;
  cursor: pointer;
  font-family: inherit;
}
.send:hover:not(:disabled) {
  filter: brightness(1.06);
}
.send:disabled {
  opacity: 0.5;
  cursor: default;
}
.link {
  color: var(--accent);
  cursor: pointer;
  font-weight: 560;
}
</style>
