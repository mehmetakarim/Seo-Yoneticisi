<script setup lang="ts">
/**
 * İlk kurulum sihirbazı — feed → Gemini → (isteğe bağlı entegrasyonlar) → ilk senkron.
 *
 * ⚠️ **Bu bir "tek kullanıcı" uygulaması değil.** Vizyon baştan beri global: hedef kitle
 * IdeaSoft altyapısı kullanan herhangi bir işletme. Yeni kullanıcının gördüğü ilk ekran boş
 * bir liste ve "Ayarlar'a git" sezgisiyse, kodun kalitesi bir işe yaramıyor.
 *
 * Sihirbaz yeni bir yetenek EKLEMİYOR — mevcut test komutlarını (`test_feed_url`,
 * `test_gemini_key`, `test_ideasoft`, `test_gsc_credentials`, `test_capsolver_key`) doğru
 * sıraya diziyor. Tek kattığı şey sıra ve bağlam.
 */
import { computed, ref } from "vue";
import { api } from "../api";
import { useStore } from "../store";
import type { SyncSummary } from "../types";
import Icon from "./Icon.vue";
import ModalShell from "./ModalShell.vue";

const store = useStore();

/** 0 hoş geldiniz · 1 feed · 2 Gemini · 3 isteğe bağlı · 4 senkron · 5 bitti */
const step = ref(0);

// --- Adım 1: feed ---
const feedUrl = ref("");
const feedBusy = ref(false);
const feedCount = ref<number | null>(null);
const feedError = ref("");

// --- Adım 2: Gemini ---
const geminiKey = ref("");
const geminiBusy = ref(false);
const geminiOk = ref(false);
const geminiError = ref("");
/** Anahtarı olmayan kullanıcı ilerleyebilsin diye. */
const geminiSkipped = ref(false);

// --- Adım 3: isteğe bağlı ---
const ideasoftDomain = ref("");
const ideasoftToken = ref("");
const capsolverKey = ref("");
const gscSiteUrl = ref("");
const gscEmail = ref("");
const optBusy = ref("");
const optHint = ref("");
const optOk = ref(false);

// --- Adım 4: senkron ---
const syncBusy = ref(false);
const syncResult = ref<SyncSummary | null>(null);
const syncError = ref("");

const TITLES = [
  "SEO Yöneticisi'ne hoş geldiniz",
  "Ürün feed'iniz",
  "Gemini API anahtarı",
  "İsteğe bağlı entegrasyonlar",
  "İlk senkron",
  "Kurulum tamam",
];
const SUBS = [
  "Kurulum birkaç dakika sürer",
  "Adım 1/3 — zorunlu",
  "Adım 2/3 — üretim için gerekli",
  "Adım 3/3 — şimdi ya da sonra",
  "Katalogunuz alınıyor",
  "",
];

const title = computed(() => TITLES[step.value]);
const sub = computed(() => SUBS[step.value]);

/** Feed testten geçmeden 1. adımdan çıkılamaz — sihirbazın tek katı kuralı. */
const canLeaveFeed = computed(() => feedCount.value !== null);

async function testFeed() {
  feedError.value = "";
  feedCount.value = null;
  const u = feedUrl.value.trim();
  if (!u) {
    feedError.value = "Feed adresini girin.";
    return;
  }
  feedBusy.value = true;
  try {
    feedCount.value = await api.testFeedUrl(u);
  } catch (e) {
    feedError.value = String(e);
  } finally {
    feedBusy.value = false;
  }
}

async function testGemini() {
  geminiError.value = "";
  geminiOk.value = false;
  const k = geminiKey.value.trim();
  if (!k) {
    geminiError.value = "Anahtarı girin veya bu adımı atlayın.";
    return;
  }
  geminiBusy.value = true;
  try {
    await api.testGeminiKey(k);
    geminiOk.value = true;
    geminiSkipped.value = false;
  } catch (e) {
    geminiError.value = String(e);
  } finally {
    geminiBusy.value = false;
  }
}

/**
 * Toplanan değerleri kaydeder.
 *
 * ⚠️ `save_settings` YEDİ alanı birden alıyor → dokunmadığımız alanlar mevcut değerleriyle
 * geri yazılmalı. Sihirbaz Ayarlar'dan tekrar çalıştırılabildiği için bu gerçek bir risk:
 * pas geçilen bir alan boş gönderilirse kullanıcının kayıtlı anahtarı silinir.
 */
async function persist() {
  const cur = store.settings;
  await api.saveSettings(
    feedUrl.value.trim() || cur?.feed_url || "",
    geminiKey.value.trim() || cur?.gemini_api_key || "",
    capsolverKey.value.trim() || cur?.capsolver_api_key || "",
    cur?.seo_country || "tr",
    gscSiteUrl.value.trim() || cur?.gsc_site_url || "",
    ideasoftDomain.value.trim() || cur?.ideasoft_domain || "",
    ideasoftToken.value.trim() || cur?.ideasoft_token || "",
  );
  store.settings = await api.getSettings();
}

/**
 * ⚠️ `test_ideasoft` ve `test_gsc_credentials` DB'den okuyor (feed/Gemini testleri parametre
 * alıyordu) → önce kaydetmek gerekiyor. Kullanıcıya da bunun kaydedildiği söyleniyor.
 */
async function testIdeasoft() {
  optBusy.value = "ideasoft";
  optHint.value = "";
  optOk.value = false;
  try {
    await persist();
    optHint.value = await api.testIdeasoft();
    optOk.value = true;
  } catch (e) {
    optHint.value = String(e);
  } finally {
    optBusy.value = "";
  }
}

async function pickGscJson() {
  optBusy.value = "gsc";
  optHint.value = "";
  optOk.value = false;
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const path = await open({
      multiple: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path || Array.isArray(path)) return;
    gscEmail.value = await api.setGscServiceAccount(path);
    optHint.value = `Servis hesabı yüklendi: ${gscEmail.value}`;
    optOk.value = true;
  } catch (e) {
    optHint.value = String(e);
  } finally {
    optBusy.value = "";
  }
}

async function runSync() {
  syncBusy.value = true;
  syncError.value = "";
  try {
    await persist();
    syncResult.value = await api.syncFeed();
    await store.reload();
    store.lastSync = syncResult.value;
    step.value = 5;
  } catch (e) {
    syncError.value = String(e);
  } finally {
    syncBusy.value = false;
  }
}

async function next() {
  if (step.value === 0) {
    // Ayarlar'dan tekrar çalıştırıldıysa mevcut değerlerle başla.
    feedUrl.value = store.settings?.feed_url ?? "";
    geminiKey.value = store.settings?.gemini_api_key ?? "";
    ideasoftDomain.value = store.settings?.ideasoft_domain ?? "";
    ideasoftToken.value = store.settings?.ideasoft_token ?? "";
    gscSiteUrl.value = store.settings?.gsc_site_url ?? "";
    capsolverKey.value = store.settings?.capsolver_api_key ?? "";
    step.value = 1;
    return;
  }
  if (step.value === 3) {
    await persist();
    step.value = 4;
    return;
  }
  step.value += 1;
}

function skipGemini() {
  geminiSkipped.value = true;
  geminiKey.value = "";
  step.value = 3;
}

/** Sihirbazı kapat. Atlansa bile `setup_done` yazılır — her açılışta çıkması anlamsız olurdu. */
async function close() {
  await store.finishSetup();
}

async function finish(goto: "products" | "overview") {
  store.page = goto;
  await close();
}
</script>

<template>
  <ModalShell
    :open="store.setupOpen"
    label="Kurulum sihirbazı"
    icon="sparkles"
    :title="title"
    :sub="sub"
    :width="560"
    :closable="!syncBusy"
    @close="close()"
  >
    <!-- 0 — Hoş geldiniz -->
    <template v-if="step === 0">
      <p class="lead">
        Bu uygulama IdeaSoft mağazanızın ürün feed'ini okur, Google verisiyle nereye
        odaklanmanız gerektiğini söyler ve SEO içeriğinizi üretir.
      </p>
      <ul class="bullets">
        <li><b>Feed adresiniz</b> — ürünleriniz buradan gelir</li>
        <li><b>Gemini API anahtarı</b> — içerik üretimi için (ücretsiz katman yeterli)</li>
        <li><b>İsteğe bağlı</b> — IdeaSoft, Search Console, CapSolver</li>
      </ul>
    </template>

    <!-- 1 — Feed -->
    <template v-else-if="step === 1">
      <label class="lbl">Ürün XML feed adresi</label>
      <div class="row">
        <input
          v-model="feedUrl"
          class="inp"
          type="text"
          placeholder="https://magazaniz.com/output/..."
          @keyup.enter="testFeed()"
        />
        <button class="ghost" :disabled="feedBusy" @click="testFeed()">
          <Icon :name="feedBusy ? 'loader' : 'check'" :size="14" :class="{ spin: feedBusy }" />
          Test et
        </button>
      </div>
      <p class="hint">
        IdeaSoft panelinizde <b>Entegrasyon → XML</b> bölümünden alabilirsiniz.
      </p>
      <div v-if="feedCount !== null" class="ok">
        <Icon name="check" :size="13" :stroke-width="2.6" />
        <span><b>{{ feedCount }}</b> ürün bulundu — adres çalışıyor.</span>
      </div>
      <div v-if="feedError" class="warn">
        <Icon name="alert" :size="13" /><span>{{ feedError }}</span>
      </div>
    </template>

    <!-- 2 — Gemini -->
    <template v-else-if="step === 2">
      <label class="lbl">Gemini API anahtarı</label>
      <div class="row">
        <input
          v-model="geminiKey"
          class="inp"
          type="password"
          placeholder="AIza…"
          @keyup.enter="testGemini()"
        />
        <button class="ghost" :disabled="geminiBusy" @click="testGemini()">
          <Icon :name="geminiBusy ? 'loader' : 'check'" :size="14" :class="{ spin: geminiBusy }" />
          Test et
        </button>
      </div>
      <p class="hint">
        <b>aistudio.google.com</b> adresinden ücretsiz alabilirsiniz: Google hesabınızla girin,
        <b>Get API key</b> → <b>Create API key</b>. Anahtar yalnızca bu bilgisayardaki
        veritabanında saklanır, hiçbir yere gönderilmez.
      </p>
      <div v-if="geminiOk" class="ok">
        <Icon name="check" :size="13" :stroke-width="2.6" />
        <span>Anahtar çalışıyor.</span>
      </div>
      <div v-if="geminiError" class="warn">
        <Icon name="alert" :size="13" /><span>{{ geminiError }}</span>
      </div>
      <a class="link skip" @click="skipGemini()">Anahtarım yok, sonra eklerim →</a>
    </template>

    <!-- 3 — İsteğe bağlı -->
    <template v-else-if="step === 3">
      <p class="lead sm">
        Hepsi isteğe bağlı — boş bırakıp geçebilir, sonra Ayarlar'dan ekleyebilirsiniz.
      </p>

      <div class="opt">
        <div class="opt-h">IdeaSoft <span class="tag">üretilen içeriği mağazaya gönderir</span></div>
        <div class="row">
          <input v-model="ideasoftDomain" class="inp" placeholder="magazaniz.myideasoft.com" />
        </div>
        <div class="row">
          <input v-model="ideasoftToken" class="inp" type="password" placeholder="Admin API token" />
          <button class="ghost" :disabled="!!optBusy" @click="testIdeasoft()">
            <Icon :name="optBusy === 'ideasoft' ? 'loader' : 'check'" :size="14" :class="{ spin: optBusy === 'ideasoft' }" />
            Test
          </button>
        </div>
      </div>

      <div class="opt">
        <div class="opt-h">Search Console <span class="tag">fırsat analizleri için</span></div>
        <div class="row">
          <input v-model="gscSiteUrl" class="inp" placeholder="sc-domain:magazaniz.com" />
          <button class="ghost" :disabled="!!optBusy" @click="pickGscJson()">
            <Icon :name="optBusy === 'gsc' ? 'loader' : 'upload'" :size="14" :class="{ spin: optBusy === 'gsc' }" />
            JSON seç
          </button>
        </div>
      </div>

      <div class="opt">
        <div class="opt-h">CapSolver <span class="tag">anahtar kelime araştırması</span></div>
        <div class="row">
          <input v-model="capsolverKey" class="inp" type="password" placeholder="CAP-…" />
        </div>
      </div>

      <!-- ⚠️ Bu adımdaki testler kaydetmeyi gerektiriyor; kullanıcı bilsin. -->
      <div v-if="optHint" :class="optOk ? 'ok' : 'warn'">
        <Icon :name="optOk ? 'check' : 'alert'" :size="13" /><span>{{ optHint }}</span>
      </div>
      <p v-if="optHint" class="hint">Test için girilen bilgiler kaydedildi.</p>
    </template>

    <!-- 4 — Senkron -->
    <template v-else-if="step === 4">
      <p class="lead sm">
        Feed'iniz okunacak ve ürünleriniz uygulamaya alınacak. Bu işlem verilerinizi
        değiştirmez, yalnızca okur.
      </p>
      <div v-if="syncError" class="warn">
        <Icon name="alert" :size="13" /><span>{{ syncError }}</span>
      </div>
      <button v-if="!syncBusy" class="run wide" @click="runSync()">
        <Icon name="refresh" :size="15" /> Senkronu başlat
      </button>
      <div v-else class="ok">
        <Icon name="loader" :size="14" class="spin" /><span>Ürünler alınıyor…</span>
      </div>
    </template>

    <!-- 5 — Bitti -->
    <template v-else>
      <div class="done">
        <div class="icon-badge big"><Icon name="check" :size="20" :stroke-width="2.4" /></div>
        <p class="lead">
          <b>{{ syncResult?.active ?? 0 }}</b> ürün hazır.
          <template v-if="syncResult?.added">{{ syncResult.added }} yeni eklendi.</template>
        </p>
      </div>
      <ul class="bullets">
        <li v-if="geminiSkipped">
          <b>Gemini anahtarınızı eklemeyi unutmayın</b> — üretim bu anahtar olmadan çalışmaz.
        </li>
        <li v-else><b>Ürünler</b> ekranından meta ve açıklama üretmeye başlayabilirsiniz.</li>
        <li v-if="gscEmail || store.settings?.gsc_client_email">
          <b>Genel Bakış</b>'tan analizi çalıştırıp nereye odaklanacağınızı görün.
        </li>
        <li v-else>
          Search Console bağlarsanız hangi ürüne öncelik vereceğinizi Google söyler.
        </li>
      </ul>
    </template>

    <template #footer>
      <button v-if="step > 0 && step < 5" class="ghost" :disabled="syncBusy" @click="step -= 1">
        Geri
      </button>
      <a v-if="step === 0" class="link" @click="close()">Şimdilik atla</a>
      <div style="flex: 1"></div>

      <template v-if="step === 5">
        <button class="ghost" @click="finish('products')">Ürünler'e git</button>
        <button
          v-if="gscEmail || store.settings?.gsc_client_email"
          class="run"
          @click="finish('overview')"
        >
          Genel Bakış'a git
        </button>
        <button v-else class="run" @click="finish('products')">Bitir</button>
      </template>
      <button
        v-else-if="step !== 4"
        class="run"
        :disabled="step === 1 && !canLeaveFeed"
        @click="next()"
      >
        {{ step === 0 ? "Kuruluma başla" : step === 3 ? "Geç ve devam et" : "İleri" }}
      </button>
    </template>
  </ModalShell>
</template>

<style scoped>
.lead {
  margin: 0;
  font-size: 13px;
  line-height: 1.6;
  color: var(--c-text);
}
.lead.sm {
  font-size: 12.5px;
  color: var(--c-soft);
}
.bullets {
  margin: 0;
  padding-left: 18px;
  font-size: 12.5px;
  line-height: 1.7;
  color: var(--c-mid);
}
.lbl {
  font-size: 12px;
  font-weight: 600;
  color: var(--c-mid);
}
.row {
  display: flex;
  gap: 8px;
  align-items: center;
}
.inp {
  flex: 1;
  width: 100%;
  height: 38px;
  padding: 0 12px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-input);
  font-size: 13px;
  color: var(--c-text);
  outline: none;
  font-family: inherit;
}
.inp:focus {
  border-color: var(--accent);
}
.hint {
  margin: 0;
  font-size: 11.5px;
  line-height: 1.55;
  color: var(--c-soft);
}
.ok {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 9px;
  font-size: 12px;
  color: var(--green);
  background: var(--ok-soft-bg);
}
.skip {
  align-self: flex-start;
  font-size: 12px;
}

/* İsteğe bağlı adımın blokları */
.opt {
  display: flex;
  flex-direction: column;
  gap: 7px;
  padding: 11px 12px;
  border: 1px solid var(--c-border-soft);
  border-radius: 10px;
  background: var(--c-list);
}
.opt-h {
  font-size: 12px;
  font-weight: 620;
  color: var(--c-text);
}
.tag {
  margin-left: 6px;
  font-size: 10.5px;
  font-weight: 500;
  color: var(--c-faint);
}

.done {
  display: flex;
  align-items: center;
  gap: 12px;
}
.icon-badge.big {
  width: 40px;
  height: 40px;
  border-radius: 12px;
  background: var(--ok-soft-bg);
  color: var(--green);
}

.ghost {
  height: 38px;
  padding: 0 14px;
  display: inline-flex;
  align-items: center;
  gap: 7px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 12.5px;
  font-weight: 560;
  cursor: pointer;
  font-family: inherit;
}
.ghost:hover:not(:disabled) {
  background: var(--c-hover);
}
.ghost:disabled {
  opacity: 0.5;
  cursor: default;
}
.run {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 38px;
  padding: 0 16px;
  border: none;
  border-radius: 9px;
  background: var(--accent);
  color: #fff;
  font-size: 13px;
  font-weight: 590;
  cursor: pointer;
  font-family: inherit;
}
.run.wide {
  width: 100%;
  justify-content: center;
}
.run:hover:not(:disabled) {
  filter: brightness(1.06);
}
.run:disabled {
  opacity: 0.5;
  cursor: default;
}
.link {
  color: var(--accent);
  cursor: pointer;
  font-weight: 560;
  font-size: 12.5px;
}
</style>
