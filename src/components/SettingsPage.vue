<script setup lang="ts">
import { onMounted, ref } from "vue";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { api } from "../api";
import { useStore } from "../store";
import Icon from "./Icon.vue";

const store = useStore();

const feedUrl = ref("");
const geminiKey = ref("");
const capsolverKey = ref("");
const seoCountry = ref("tr");
const gscSiteUrl = ref("");
const ideasoftDomain = ref("");
const ideasoftToken = ref("");
const showIsToken = ref(false);
const isHint = ref("");
const isOk = ref(false);
const gscEmail = ref("");
const showKey = ref(false);
const showCapKey = ref(false);
const xmlHint = ref("");
const xmlOk = ref(false);
const keyHint = ref("");
const keyOk = ref(false);
const capHint = ref("");
const capOk = ref(false);
const gscHint = ref("");
const gscOk = ref(false);
const showGuide = ref(false);
const importHint = ref(".db veya .json dosyası seçin");

onMounted(async () => {
  if (!store.settings) store.settings = await api.getSettings();
  feedUrl.value = store.settings.feed_url;
  geminiKey.value = store.settings.gemini_api_key;
  capsolverKey.value = store.settings.capsolver_api_key;
  seoCountry.value = store.settings.seo_country || "tr";
  gscSiteUrl.value = store.settings.gsc_site_url;
  ideasoftDomain.value = store.settings.ideasoft_domain;
  ideasoftToken.value = store.settings.ideasoft_token;
  gscEmail.value = store.settings.gsc_client_email;
});

async function persist() {
  try {
    await api.saveSettings(
      feedUrl.value,
      geminiKey.value,
      capsolverKey.value,
      seoCountry.value,
      gscSiteUrl.value,
      ideasoftDomain.value,
      ideasoftToken.value,
    );
    store.settings = await api.getSettings();
  } catch (e) {
    store.toast(String(e), "error");
  }
}

async function uploadSa() {
  try {
    const path = await open({
      title: "Service-account JSON seç",
      multiple: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path || Array.isArray(path)) return;
    gscEmail.value = await api.setGscServiceAccount(path);
    store.settings = await api.getSettings();
    gscOk.value = true;
    gscHint.value = "Service-account yüklendi";
    store.toast("GSC service-account yüklendi", "ok");
  } catch (e) {
    gscOk.value = false;
    gscHint.value = String(e);
  }
}

async function removeSa() {
  try {
    await api.clearGscServiceAccount();
    gscEmail.value = "";
    gscHint.value = "";
    store.settings = await api.getSettings();
  } catch (e) {
    store.toast(String(e), "error");
  }
}

async function testGsc() {
  gscHint.value = "Test ediliyor…";
  gscOk.value = false;
  try {
    const msg = await api.testGscCredentials();
    gscOk.value = true;
    gscHint.value = msg;
  } catch (e) {
    gscOk.value = false;
    gscHint.value = String(e);
  }
}

const guideSteps: { t: string; s: string; url?: string; urlLabel?: string }[] = [
  {
    t: "Google Cloud projesi oluştur",
    s: "console.cloud.google.com → üstteki proje seçiciden yeni bir proje oluştur.",
    url: "https://console.cloud.google.com/projectcreate",
    urlLabel: "Proje oluştur",
  },
  {
    t: "Search Console API'yi etkinleştir",
    s: "API Library'de 'Google Search Console API'yi bul ve Etkinleştir'e bas.",
    url: "https://console.cloud.google.com/apis/library/searchconsole.googleapis.com",
    urlLabel: "API'yi aç",
  },
  {
    t: "Service Account + JSON anahtar",
    s: "IAM & Admin → Service Accounts → oluştur → Keys sekmesi → Add Key → JSON indir.",
    url: "https://console.cloud.google.com/iam-admin/serviceaccounts",
    urlLabel: "Service Accounts",
  },
  {
    t: "GSC mülküne kullanıcı ekle",
    s: "Search Console → Ayarlar → Kullanıcılar ve izinler → service-account e-postasını 'Tam' yetkiyle ekle.",
    url: "https://search.google.com/search-console/users",
    urlLabel: "GSC kullanıcıları",
  },
  {
    t: "JSON'u buraya yükle",
    s: "İndirdiğin JSON'u aşağıdaki 'JSON yükle' ile seç, mülk adresini gir (ör. sc-domain:siteniz.com), Bağlantıyı test et.",
  },
];

// Teknik tablo için siteye bir kez eklenmesi gereken CSS — burada referans olarak saklanır,
// tema dosyasından silinirse buradan yeniden alınabilir.
const TECH_TABLE_CSS = `.teknik-tablo { width: 100%; border-collapse: collapse; margin-bottom: 24px; }

/* Grup başlığı — mevcut thead-light görünümüyle aynı, eski tablolar da uyumlu kalsın */
.teknik-tablo caption,
.teknik-tablo .thead-light th:first-child {
  caption-side: top;                /* Bootstrap'i ez */
  text-align: left;
  font-weight: 600; font-size: 15px; color: #1d1d1f;
  background: #f5f5f7; padding: 10px 12px; border-radius: 8px 8px 0 0;
}

.teknik-tablo th, .teknik-tablo td {
  padding: 12px; text-align: left; vertical-align: top;
  border-bottom: 1px solid #ececef;
  overflow-wrap: anywhere; line-height: 1.5;
}
.teknik-tablo th[scope="row"] { width: 35%; font-weight: 500; color: #5a5a5e; }
.teknik-tablo td { color: #1d1d1f; font-variant-numeric: tabular-nums; }
.teknik-tablo tr:last-child th, .teknik-tablo tr:last-child td { border-bottom: 0; }

@media (max-width: 576px) {
  .teknik-tablo th[scope="row"] { width: 42%; }
  .teknik-tablo th, .teknik-tablo td { padding: 11px 8px; font-size: 14px; }
}`;

const cssOpen = ref(false);
const cssCopied = ref(false);
async function copyCss() {
  try {
    await writeText(TECH_TABLE_CSS);
    cssCopied.value = true;
    setTimeout(() => (cssCopied.value = false), 1600);
  } catch {
    store.toast("Panoya kopyalanamadı", "error");
  }
}

async function openGuideLink(url?: string) {
  if (url) {
    try {
      await openUrl(url);
    } catch {
      store.toast("Bağlantı açılamadı", "error");
    }
  }
}

async function testIdeasoft() {
  isHint.value = "Test ediliyor…";
  isOk.value = false;
  try {
    await persist();
    isOk.value = true;
    isHint.value = await api.testIdeasoft();
  } catch (e) {
    isOk.value = false;
    isHint.value = String(e);
  }
}

async function testCapsolver() {
  try {
    const msg = await api.testCapsolverKey(capsolverKey.value);
    capOk.value = true;
    capHint.value = msg;
    await persist();
  } catch (e) {
    capOk.value = false;
    capHint.value = String(e);
  }
}

async function testXml() {
  xmlHint.value = "Test ediliyor…";
  xmlOk.value = false;
  try {
    const n = await api.testFeedUrl(feedUrl.value);
    xmlOk.value = true;
    xmlHint.value = `Bağlantı doğrulandı · ${n} ürün bulundu`;
    await persist();
  } catch (e) {
    xmlOk.value = false;
    xmlHint.value = String(e);
  }
}

async function testKey() {
  try {
    const msg = await api.testGeminiKey(geminiKey.value);
    keyOk.value = true;
    keyHint.value = msg;
    await persist();
  } catch (e) {
    keyOk.value = false;
    keyHint.value = String(e);
  }
}

function lastBackup(): string {
  const v = store.settings?.last_backup_at;
  if (!v) return "henüz yedek alınmadı";
  return v.replace("T", " ").slice(0, 16);
}

async function doExport(format: "db" | "json") {
  try {
    const path = await save({
      title: "Veritabanını dışa aktar",
      defaultPath: `seo-yedek-${new Date().toISOString().slice(0, 10)}.${format}`,
      filters: [{ name: format.toUpperCase(), extensions: [format] }],
    });
    if (!path) return;
    await api.exportDb(path, format);
    store.settings = await api.getSettings();
    store.toast(`Yedek dışa aktarıldı (.${format})`, "ok");
  } catch (e) {
    store.toast(String(e), "error");
  }
}

async function doImport() {
  try {
    const path = await open({
      title: "Yedekten içe aktar",
      multiple: false,
      filters: [{ name: "Yedek", extensions: ["db", "json"] }],
    });
    if (!path || Array.isArray(path)) return;
    const ok = window.confirm(
      "İçe aktarma mevcut veritabanının üzerine yazacak. Devam edilsin mi?",
    );
    if (!ok) return;
    await api.importDb(path);
    importHint.value = "Geri yükleme tamamlandı";
    await store.reload();
    if (store.rows.length) await store.select(store.rows[0].sku);
    store.settings = await api.getSettings();
    feedUrl.value = store.settings.feed_url;
    geminiKey.value = store.settings.gemini_api_key;
    store.lastSync = await api.getLastSync();
    store.toast("Yedek geri yüklendi", "ok");
  } catch (e) {
    store.toast(String(e), "error");
  }
}
</script>

<template>
  <div class="settings om-scroll">
    <div class="wrap">
      <!-- Kaynaklar -->
      <div class="card">
        <div class="card-head">
          <div class="ch-title">
            <Icon name="link" :size="17" style="color:var(--accent)" />
            Kaynaklar
          </div>
          <div class="ch-sub">
            Feed ve yapay zeka bağlantı bilgileriniz. Bu değerler yerel SQLite
            veritabanında saklanır.
          </div>
        </div>
        <div class="card-body">
          <div>
            <label class="lbl">XML Feed URL</label>
            <div class="input-row">
              <input class="fx inp" v-model="feedUrl" @change="persist" placeholder="https://siteniz.com/feed/urunler.xml" />
              <button class="ghost" @click="testXml">Test et</button>
            </div>
            <div class="fhint" :style="{ color: xmlHint ? (xmlOk ? 'var(--green)' : 'var(--red)') : 'var(--c-faint)' }">
              {{ xmlHint }}
            </div>
          </div>
          <div>
            <label class="lbl">Gemini API Anahtarı</label>
            <div class="input-row">
              <div class="key-wrap">
                <input
                  class="fx inp"
                  v-model="geminiKey"
                  @change="persist"
                  :type="showKey ? 'text' : 'password'"
                  placeholder="AIza…"
                />
                <button class="eye" title="Göster / Gizle" @click="showKey = !showKey">
                  <Icon name="eye" :size="15" />
                </button>
              </div>
              <button class="ghost" @click="testKey">Bağlantıyı test et</button>
            </div>
            <div class="fhint" :style="{ color: keyHint ? (keyOk ? 'var(--green)' : 'var(--red)') : 'var(--c-faint)' }">
              {{ keyHint }}
            </div>
          </div>

          <!-- SEO Araştırma (Faz 4): CapSolver + ülke -->
          <div>
            <label class="lbl">CapSolver API Anahtarı</label>
            <div class="input-row">
              <div class="key-wrap">
                <input
                  class="fx inp"
                  v-model="capsolverKey"
                  @change="persist"
                  :type="showCapKey ? 'text' : 'password'"
                  placeholder="CAP-…"
                />
                <button class="eye" title="Göster / Gizle" @click="showCapKey = !showCapKey">
                  <Icon name="eye" :size="15" />
                </button>
              </div>
              <button class="ghost" @click="testCapsolver">Anahtarı test et</button>
            </div>
            <div class="fhint" :style="{ color: capHint ? (capOk ? 'var(--green)' : 'var(--red)') : 'var(--c-faint)' }">
              {{ capHint || "“SEO Araştır” için gerekli · Ahrefs verilerine erişimi açar (capsolver.com)" }}
            </div>
          </div>

          <div>
            <label class="lbl">Araştırma Ülkesi</label>
            <div class="input-row">
              <input
                class="fx inp country"
                v-model="seoCountry"
                @change="persist"
                maxlength="2"
                placeholder="tr"
              />
              <span class="country-hint">İki harfli ülke kodu (ör. tr, us, de) · Ahrefs/Trends için</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Google Search Console (Faz 5) -->
      <div class="card">
        <div class="card-head">
          <div class="ch-title">
            <Icon name="search" :size="17" style="color:var(--accent)" />
            Google Search Console
          </div>
          <div class="ch-sub">
            Ürün sayfalarının Google'daki <b>gerçek arama sorgularını</b> üretime katar.
            Service-account ile bağlanır (tarayıcı girişi gerekmez).
            <a class="guide-link" @click="showGuide = true">Bu dosyayı nasıl alırım?</a>
          </div>
        </div>
        <div class="card-body">
          <div>
            <label class="lbl">Mülk (Site) Adresi</label>
            <div class="input-row">
              <input
                class="fx inp"
                v-model="gscSiteUrl"
                @change="persist"
                placeholder="sc-domain:siteniz.com  veya  https://siteniz.com/"
              />
            </div>
          </div>
          <div>
            <label class="lbl">Service-account (JSON)</label>
            <div v-if="gscEmail" class="sa-loaded">
              <div class="sa-info">
                <Icon name="badgeCheck" :size="15" style="color:var(--green)" />
                <span class="sa-email">{{ gscEmail }}</span>
              </div>
              <div class="sa-actions">
                <button class="ghost" @click="testGsc">Bağlantıyı test et</button>
                <button class="ghost danger" @click="removeSa">Kaldır</button>
              </div>
            </div>
            <div v-else class="input-row">
              <button class="ghost" @click="uploadSa">
                <Icon name="upload" :size="14" /> JSON yükle
              </button>
              <span class="country-hint">İndirdiğin service-account anahtarını seç</span>
            </div>
            <div class="fhint" :style="{ color: gscHint ? (gscOk ? 'var(--green)' : 'var(--red)') : 'var(--c-faint)' }">
              {{ gscHint }}
            </div>
          </div>
        </div>
      </div>

      <!-- IdeaSoft (opsiyonel modül) -->
      <div class="card">
        <div class="card-head">
          <div class="ch-title">
            <Icon name="upload" :size="17" style="color:var(--accent)" />
            IdeaSoft Bağlantısı
            <span class="opt-tag">opsiyonel</span>
          </div>
          <div class="ch-sub">
            Doldurulursa kartlarda <b>“IdeaSoft'a Gönder”</b> butonu çıkar ve üretilen içerik tek tıkla
            ürüne yazılır. Boş bırakırsanız uygulama kopyala-yapıştır akışıyla çalışmaya devam eder.
          </div>
        </div>
        <div class="card-body">
          <div>
            <label class="lbl">Mağaza Adresi (admin)</label>
            <div class="input-row">
              <input class="fx inp" v-model="ideasoftDomain" @change="persist"
                     placeholder="https://magazaniz.myideasoft.com" />
            </div>
          </div>
          <div>
            <label class="lbl">Access Token</label>
            <div class="input-row">
              <div class="key-wrap">
                <input class="fx inp" v-model="ideasoftToken" @change="persist"
                       :type="showIsToken ? 'text' : 'password'" placeholder="IdeaSoft panelinden aldığınız access token" />
                <button class="eye" title="Göster / Gizle" @click="showIsToken = !showIsToken">
                  <Icon name="eye" :size="15" />
                </button>
              </div>
              <button class="ghost" @click="testIdeasoft">Bağlantıyı test et</button>
            </div>
            <div class="fhint" :style="{ color: isHint ? (isOk ? 'var(--green)' : 'var(--red)') : 'var(--c-faint)' }">
              {{ isHint || "Token günlük yenilenir; gönderim yetki hatası verirse buradan güncelleyin." }}
            </div>
          </div>
        </div>
      </div>

      <!-- Uygulama / Güncelleme -->
      <div class="card">
        <div class="card-head">
          <div class="ch-title">
            <Icon name="refresh" :size="17" style="color:var(--accent)" />
            Uygulama Sürümü
          </div>
          <div class="ch-sub">
            Uygulama açılışta yeni sürümü kendisi denetler. Buradan elle de kontrol edebilirsiniz.
          </div>
        </div>
        <div class="card-body">
          <div class="css-row">
            <span class="ver">Yüklü sürüm: <b>v{{ store.appVersion || "—" }}</b></span>
            <button class="ghost" :disabled="store.updateChecking || store.updating" @click="store.checkUpdate(false)">
              <Icon name="refresh" :size="14" :class="{ spin: store.updateChecking }" />
              {{ store.updateChecking ? "Denetleniyor…" : "Şimdi denetle" }}
            </button>
          </div>
        </div>
      </div>

      <!-- Teknik Tablo CSS (referans) -->
      <div class="card">
        <div class="card-head">
          <div class="ch-title">
            <Icon name="code" :size="17" style="color:var(--accent)" />
            Teknik Tablo CSS
          </div>
          <div class="ch-sub">
            Üretilen teknik tablonun sitenizde doğru görünmesi için temanıza <b>bir kez</b> eklenmesi
            gereken stil. Silinirse veya kaybolursa buradan yeniden alabilirsiniz.
          </div>
        </div>
        <div class="card-body">
          <div class="css-row">
            <button class="ghost" @click="cssOpen = !cssOpen">
              <Icon name="eye" :size="14" />
              {{ cssOpen ? "Gizle" : "CSS'i göster" }}
            </button>
            <button class="solid" @click="copyCss">
              <Icon :name="cssCopied ? 'check' : 'copy'" :size="14" />
              {{ cssCopied ? "Kopyalandı" : "CSS'i kopyala" }}
            </button>
            <span class="country-hint">Tema → CSS dosyanıza yapıştırın</span>
          </div>
          <pre v-if="cssOpen" class="css-block om-scroll">{{ TECH_TABLE_CSS }}</pre>
          <div class="warn">
            <Icon name="info" :size="13" />
            <b>caption-side: top</b> satırı kritik — Bootstrap başlığı varsayılan olarak tablonun
            <b>altına</b> koyar. Mobilde yatay kaydırma yoktur, tablo 320px'e kadar okunaklıdır.
          </div>
        </div>
      </div>

      <!-- Yedekleme -->
      <div class="card">
        <div class="card-head">
          <div class="ch-title">
            <Icon name="database" :size="17" style="color:var(--accent)" />
            Yedekleme
          </div>
          <div class="ch-sub">
            Tüm ürün SEO verinizi ve ayarlarınızı dışa aktarın veya bir yedekten
            geri yükleyin.
          </div>
        </div>
        <div class="card-body">
          <div class="backup-row">
            <div class="bi accent">
              <Icon name="download" :size="17" />
            </div>
            <div class="bi-main">
              <div class="bi-title">Veritabanını dışa aktar</div>
              <div class="bi-sub">Son yedek: {{ lastBackup() }}</div>
            </div>
            <button class="solid" @click="doExport('db')">.db</button>
            <button class="ghost" @click="doExport('json')">.json</button>
          </div>
          <div class="backup-row">
            <div class="bi neutral">
              <Icon name="upload" :size="17" />
            </div>
            <div class="bi-main">
              <div class="bi-title">Yedekten içe aktar</div>
              <div class="bi-sub">{{ importHint }}</div>
            </div>
            <button class="ghost" @click="doImport">Dosya seç</button>
          </div>
          <div class="warn">
            <Icon name="info" :size="13" />
            İçe aktarma mevcut veritabanının üzerine yazar. Önce dışa aktarım
            almanız önerilir.
          </div>
        </div>
      </div>
    </div>

    <!-- GSC kurulum rehberi (animasyonlu modal) -->
    <Transition name="guide">
      <div v-if="showGuide" class="guide-overlay" @click.self="showGuide = false">
        <div class="guide-modal om-scroll" role="dialog" aria-label="GSC kurulum rehberi">
          <header class="guide-head">
            <div class="gh-title">
              <div class="icon-badge"><Icon name="search" :size="15" /></div>
              Search Console'a nasıl bağlanırım?
            </div>
            <button class="close" title="Kapat" @click="showGuide = false">
              <Icon name="x" :size="16" :stroke-width="2.2" />
            </button>
          </header>
          <div class="guide-body">
            <p class="guide-intro">Tek seferlik kurulum. Zaten bir service-account'un varsa 5. adıma geç.</p>
            <ol class="steps">
              <li v-for="(step, i) in guideSteps" :key="i" class="step">
                <div class="step-num">{{ i + 1 }}</div>
                <div class="step-main">
                  <div class="step-t">{{ step.t }}</div>
                  <div class="step-s">{{ step.s }}</div>
                  <button v-if="step.url" class="step-link" @click="openGuideLink(step.url)">
                    <Icon name="external" :size="12" /> {{ step.urlLabel }}
                  </button>
                </div>
              </li>
            </ol>
            <div class="warn guide-warn">
              <Icon name="info" :size="13" />
              JSON dosyası özel anahtar içerir; yalnızca yerel veritabanında saklanır, hiçbir yere gönderilmez.
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.settings {
  flex: 1;
  overflow-y: auto;
  background: var(--c-bg);
}
.wrap {
  max-width: 680px;
  margin: 0 auto;
  padding: 28px 32px 60px;
  display: flex;
  flex-direction: column;
  gap: 18px;
}
.card {
  border: 1px solid var(--c-border-soft);
  border-radius: 14px;
  background: var(--c-card);
  overflow: hidden;
}
.card-head {
  padding: 16px 18px;
  border-bottom: 1px solid var(--c-border-soft);
}
.ch-title {
  display: flex;
  align-items: center;
  gap: 9px;
  font-size: 14px;
  font-weight: 640;
  color: var(--c-text);
}
.ch-sub {
  font-size: 12.5px;
  color: var(--c-soft);
  margin-top: 4px;
}
.card-body {
  padding: 18px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.lbl {
  display: block;
  font-size: 12.5px;
  font-weight: 580;
  color: var(--c-mid);
  margin-bottom: 7px;
}
.input-row {
  display: flex;
  gap: 8px;
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
}
.country {
  flex: none;
  width: 90px;
  text-transform: lowercase;
}
.country-hint {
  display: flex;
  align-items: center;
  font-size: 11.5px;
  color: var(--c-faint);
}
.key-wrap {
  flex: 1;
  position: relative;
  display: flex;
}
.key-wrap .inp {
  padding-right: 40px;
}
.eye {
  position: absolute;
  right: 6px;
  top: 50%;
  transform: translateY(-50%);
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--c-soft);
  cursor: pointer;
}
.eye:hover {
  background: var(--c-hover);
}
.ghost {
  flex: none;
  height: 38px;
  padding: 0 14px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 12.5px;
  font-weight: 560;
  cursor: pointer;
}
.ghost:hover {
  background: var(--c-hover);
}
.fhint {
  font-size: 11.5px;
  margin-top: 6px;
  min-height: 14px;
}
.backup-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  border: 1px solid var(--c-border);
  border-radius: 10px;
  background: var(--c-input);
}
.bi {
  width: 34px;
  height: 34px;
  flex: none;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.bi.accent {
  background: var(--accent-tint);
  color: var(--accent);
}
.bi.neutral {
  background: var(--c-chip);
  color: var(--c-mid);
}
.bi-main {
  flex: 1;
  min-width: 0;
}
.bi-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--c-text);
}
.bi-sub {
  font-size: 11.5px;
  color: var(--c-soft);
  margin-top: 2px;
}
.solid {
  flex: none;
  height: 34px;
  padding: 0 13px;
  border: none;
  border-radius: 8px;
  background: var(--accent);
  color: #fff;
  font-size: 12.5px;
  font-weight: 580;
  cursor: pointer;
}
.solid:hover {
  filter: brightness(1.05);
}
.backup-row .ghost {
  height: 34px;
  border-radius: 8px;
}
.warn {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11.5px;
  color: var(--c-faint);
}

/* Teknik Tablo CSS bölümü */
.css-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.css-row .ghost,
.css-row .solid {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  height: 36px;
}
.css-block {
  margin: 0;
  max-height: 300px;
  overflow: auto;
  padding: 12px 14px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-input);
  color: var(--c-text);
  font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, monospace;
  font-size: 11.5px;
  line-height: 1.6;
  white-space: pre;
  animation: popIn 0.22s ease both;
}

/* GSC bölümü */
.guide-link {
  color: var(--accent);
  cursor: pointer;
  font-weight: 560;
  margin-left: 4px;
}
.guide-link:hover {
  text-decoration: underline;
}
.ver {
  font-size: 12.5px;
  color: var(--c-mid);
}
.spin {
  animation: spin 0.8s linear infinite;
}
.opt-tag {
  font-size: 10px;
  font-weight: 640;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--c-soft);
  background: var(--c-chip);
  border-radius: 999px;
  padding: 2px 8px;
}
.ghost.danger {
  color: var(--red);
}
.ghost.danger:hover {
  background: var(--badge-eksik-bg);
}
.sa-loaded {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 10px 12px;
  border: 1px solid var(--c-border);
  border-radius: 10px;
  background: var(--c-input);
  flex-wrap: wrap;
}
.sa-info {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.sa-email {
  font-size: 12.5px;
  color: var(--c-text);
  font-weight: 520;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.sa-actions {
  display: flex;
  gap: 8px;
  flex: none;
}
.sa-actions .ghost {
  height: 32px;
  border-radius: 8px;
}

/* GSC rehber modalı */
.guide-overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: var(--overlay-bg);
  backdrop-filter: saturate(1.1) blur(3px);
}
.guide-modal {
  width: 520px;
  max-width: 100%;
  max-height: 86vh;
  overflow-y: auto;
  background: var(--c-card);
  border: 1px solid var(--c-border);
  border-radius: 16px;
  box-shadow: 0 24px 60px var(--heavy-shadow);
}
.guide-head {
  position: sticky;
  top: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 18px;
  background: var(--c-card);
  border-bottom: 1px solid var(--c-border-soft);
}
.gh-title {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 14.5px;
  font-weight: 650;
  color: var(--c-text);
  letter-spacing: -0.01em;
}
.icon-badge {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  background: var(--accent-tint);
  color: var(--accent);
  display: flex;
  align-items: center;
  justify-content: center;
}
.close {
  width: 30px;
  height: 30px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--c-soft);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}
.close:hover {
  background: var(--c-hover);
  color: var(--c-text);
}
.guide-body {
  padding: 18px;
}
.guide-intro {
  margin: 0 0 14px;
  font-size: 12.5px;
  color: var(--c-soft);
}
.steps {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.step {
  display: flex;
  gap: 12px;
}
.step-num {
  flex: none;
  width: 24px;
  height: 24px;
  border-radius: 999px;
  background: var(--accent-tint);
  color: var(--accent);
  font-size: 12px;
  font-weight: 680;
  display: flex;
  align-items: center;
  justify-content: center;
}
.step-main {
  min-width: 0;
}
.step-t {
  font-size: 13px;
  font-weight: 600;
  color: var(--c-text);
}
.step-s {
  font-size: 12px;
  color: var(--c-soft);
  margin-top: 3px;
  line-height: 1.5;
}
.step-link {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  margin-top: 7px;
  height: 28px;
  padding: 0 10px;
  border: 1px solid var(--c-border);
  border-radius: 7px;
  background: var(--c-input);
  color: var(--accent);
  font-size: 11.5px;
  font-weight: 560;
  cursor: pointer;
}
.step-link:hover {
  border-color: var(--accent);
}
.guide-warn {
  margin-top: 18px;
  padding: 10px 12px;
  border-radius: 9px;
  background: var(--warn-bg);
  border: 1px solid var(--warn-border);
  color: var(--warn-text);
}

/* modal animasyonu (Apple hissi: fade + hafif ölçek) */
.guide-enter-active,
.guide-leave-active {
  transition: opacity 0.24s ease;
}
.guide-enter-from,
.guide-leave-to {
  opacity: 0;
}
.guide-enter-active .guide-modal,
.guide-leave-active .guide-modal {
  transition: transform 0.26s cubic-bezier(0.32, 0.72, 0, 1);
}
.guide-enter-from .guide-modal,
.guide-leave-to .guide-modal {
  transform: scale(0.96) translateY(8px);
}
@media (prefers-reduced-motion: reduce) {
  .guide-enter-active .guide-modal,
  .guide-leave-active .guide-modal {
    transition: none;
  }
  .guide-enter-from .guide-modal,
  .guide-leave-to .guide-modal {
    transform: none;
  }
}
</style>
