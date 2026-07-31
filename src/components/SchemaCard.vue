<script setup lang="ts">
/**
 * Schema.org `Product` JSON-LD kartı.
 *
 * ⚠️ Çıktı BİLİNÇLİ olarak store'da tutulmuyor. Meta veya teknik tablo üretimi ürünün
 * verisini değiştiriyor; önbelleğe alınmış bir kod bloğu sessizce bayatlar ve kullanıcı
 * eski hâli kopyalar. Her "Göster"/"Kopyala" arka uçtan taze çekiyor — bu bir veritabanı
 * okuması, maliyeti yok.
 */
import { ref, watch } from "vue";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { api } from "../api";
import { useStore } from "../store";
import Icon from "./Icon.vue";
import SeoCard from "./SeoCard.vue";

const store = useStore();
const code = ref("");
const busy = ref(false);
const copied = ref(false);
const error = ref("");

// Ürün değişince açık kalan blok yeni ürünü göstermez — kapatılıyor.
watch(
  () => store.selectedSku,
  () => {
    code.value = "";
    error.value = "";
  },
);

async function fetchCode(): Promise<string> {
  if (!store.selectedSku) return "";
  busy.value = true;
  error.value = "";
  try {
    return await api.getJsonld(store.selectedSku);
  } catch (e) {
    error.value = String(e);
    return "";
  } finally {
    busy.value = false;
  }
}

async function toggle() {
  if (code.value) {
    code.value = "";
    return;
  }
  code.value = await fetchCode();
}

async function copy() {
  const fresh = await fetchCode();
  if (!fresh) return;
  // Görünen blok da tazeleniyor: kopyalanan ile ekrandaki aynı olmalı.
  if (code.value) code.value = fresh;
  await writeText(fresh);
  copied.value = true;
  setTimeout(() => (copied.value = false), 1600);
}
</script>

<template>
  <SeoCard
    icon="code"
    title="Schema.org (JSON-LD)"
    sub="Ürünü arama motorlarına yapılandırılmış veri olarak anlatır"
    stack
  >
    <div class="info-row">
      <span class="info">
        <Icon name="info" :size="13" />
        Yeni içerik üretmez — ad, marka, kategori, görseller ve teknik tablodan derlenir.
      </span>
      <button class="copy" :class="{ ok: copied }" :disabled="busy" @click="copy">
        <Icon name="copy" :size="12" />
        {{ copied ? "Kopyalandı" : "Kopyala" }}
      </button>
    </div>

    <!-- Neyin DIŞARIDA olduğu, neyin içeride olduğu kadar önemli: kullanıcı fiyatı burada
         arayıp bulamayınca eksiklik sanmasın. -->
    <div class="hint">
      <Icon name="info" :size="13" />
      Fiyat ve stok bilinçli olarak yok — mağazanız bunları sayfada zaten canlı basıyor,
      buradan yazılan değer bir gün sonra yanlış olurdu.
    </div>

    <div v-if="error" class="warn">
      <Icon name="alert" :size="14" />
      {{ error }}
    </div>

    <pre v-if="code" class="code om-scroll">{{ code }}</pre>

    <template #actions>
      <button class="gen" :disabled="busy" @click="toggle">
        <Icon
          :name="busy ? 'loader' : code ? 'x' : 'code'"
          :size="15"
          :stroke-width="busy ? 2.2 : 1.9"
          :class="{ spin: busy }"
        />
        {{ busy ? "Hazırlanıyor…" : code ? "Gizle" : "Kodu Göster" }}
      </button>
    </template>
  </SeoCard>
</template>

<style scoped>
.info-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}
.info {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 11.5px;
  color: var(--c-soft);
}
.copy {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex: none;
  height: 28px;
  padding: 0 10px;
  border: 1px solid var(--c-border);
  border-radius: 7px;
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 11.5px;
  font-weight: 560;
  cursor: pointer;
}
.copy:hover {
  background: var(--c-hover);
}
.copy.ok {
  color: var(--green);
  border-color: var(--green);
}
.code {
  max-height: 340px;
  overflow: auto;
  margin: 0;
  padding: 12px 14px;
  border: 1px solid var(--c-border-soft);
  border-radius: 10px;
  background: var(--c-list);
  color: var(--c-mid);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 11.5px;
  line-height: 1.55;
  white-space: pre;
}
.gen {
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
}
.gen:disabled {
  opacity: 0.55;
  cursor: default;
}
</style>
