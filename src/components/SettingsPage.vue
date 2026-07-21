<script setup lang="ts">
import { onMounted, ref } from "vue";
import { open, save } from "@tauri-apps/plugin-dialog";
import { api } from "../api";
import { useStore } from "../store";
import Icon from "./Icon.vue";

const store = useStore();

const feedUrl = ref("");
const geminiKey = ref("");
const showKey = ref(false);
const xmlHint = ref("");
const xmlOk = ref(false);
const keyHint = ref("");
const keyOk = ref(false);
const importHint = ref(".db veya .json dosyası seçin");

onMounted(async () => {
  if (!store.settings) store.settings = await api.getSettings();
  feedUrl.value = store.settings.feed_url;
  geminiKey.value = store.settings.gemini_api_key;
});

async function persist() {
  try {
    await api.saveSettings(feedUrl.value, geminiKey.value);
    store.settings = await api.getSettings();
  } catch (e) {
    store.toast(String(e), "error");
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
</style>
