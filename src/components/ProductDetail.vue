<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useStore } from "../store";
import { computeBadge } from "../validation";
import Icon from "./Icon.vue";
import MetaSeoCard from "./MetaSeoCard.vue";
import DetailsSeoCard from "./DetailsSeoCard.vue";

const store = useStore();
const detail = computed(() => store.detail);

// Düzenlenebilir yerel alanlar (taslak ?? feed değeri)
const title = ref("");
const desc = ref("");
const searchKw = ref("");
const keyword = ref("");

// Detay nesnesi tazelenince (seçim, senkron veya Gemini üretimi) yerel alanları güncelle.
// Not: taslak kaydı yalnızca listeyi yeniler, detail nesnesini değiştirmez → yazarken ezilmez.
watch(
  () => store.detail,
  (d) => {
    if (!d) return;
    title.value = d.draft_title ?? d.title ?? "";
    desc.value = d.draft_descriptions ?? d.descriptions ?? "";
    searchKw.value = d.draft_search_keywords ?? d.search_keywords ?? "";
    keyword.value = d.target_keyword ?? "";
  },
  { immediate: true },
);

const liveBadge = computed(() =>
  computeBadge(title.value, desc.value, keyword.value, store.detail?.meta_status === "done"),
);

// Debounced taslak kaydı
let draftTimer: number | undefined;
function scheduleDraftSave() {
  window.clearTimeout(draftTimer);
  draftTimer = window.setTimeout(async () => {
    await store.saveDraft(title.value, desc.value, searchKw.value);
    await store.reload();
  }, 600);
}

let kwTimer: number | undefined;
function scheduleKeywordSave() {
  window.clearTimeout(kwTimer);
  kwTimer = window.setTimeout(async () => {
    await store.saveKeyword(keyword.value);
    await store.reload();
  }, 500);
}

function onTitle(v: string) { title.value = v; scheduleDraftSave(); }
function onDesc(v: string) { desc.value = v; scheduleDraftSave(); }
function onSearch(v: string) { searchKw.value = v; scheduleDraftSave(); }
function onKeyword(e: Event) {
  keyword.value = (e.target as HTMLInputElement).value;
  scheduleKeywordSave();
}

async function openProduct() {
  if (detail.value?.url) {
    try {
      await openUrl(detail.value.url);
    } catch {
      store.toast("Bağlantı açılamadı", "error");
    }
  }
}
</script>

<template>
  <section class="detail om-scroll">
    <div v-if="detail" class="inner">
      <!-- ürün başlığı -->
      <div class="prod-head">
        <div style="min-width:0">
          <div class="title-row">
            <h1>{{ detail.name }}</h1>
            <a v-if="detail.url" class="ext" title="Ürün sayfasını aç" @click.prevent="openProduct">
              <Icon name="external" :size="14" />
            </a>
          </div>
          <div class="meta-line">
            <span class="brand">{{ detail.brand || "—" }}</span>
            <span class="sep">·</span>
            <span>{{ detail.category || detail.main_category || "Kategori yok" }}</span>
            <span class="sep">·</span>
            <span>Stok: {{ detail.quantity ?? "—" }}</span>
          </div>
        </div>
      </div>

      <!-- ortak hedef kelime -->
      <div class="keyword-row">
        <label>
          <Icon name="tag" :size="14" :stroke-width="1.9" />
          Hedef Kelime
        </label>
        <input
          class="fx kw-input"
          :value="keyword"
          @input="onKeyword"
          placeholder="ör. bluetooth kulaklık"
        />
        <span class="kw-hint">iki kart da bu kelimeyi kullanır</span>
      </div>

      <MetaSeoCard
        :title="title"
        :desc="desc"
        :search-kw="searchKw"
        :keyword="keyword"
        :meta-done="detail.meta_status === 'done'"
        :badge="liveBadge"
        @update:title="onTitle"
        @update:desc="onDesc"
        @update:search-kw="onSearch"
        @toggle-done="store.toggleMetaDone()"
      />

      <DetailsSeoCard
        :details-html="detail.details ?? ''"
        :keyword="keyword"
        :details-done="detail.details_status === 'done'"
      />
    </div>

    <div v-else class="no-sel">
      <div class="no-sel-icon">
        <Icon name="fileEdit" :size="30" :stroke-width="1.6" />
      </div>
      <div class="no-sel-title">Soldan bir ürün seçin</div>
      <div class="no-sel-sub">
        Meta SEO ve Açıklama SEO kartlarını düzenlemek için listeden bir ürüne
        tıklayın veya <b>↑ ↓</b> ile gezinin.
      </div>
    </div>
  </section>
</template>

<style scoped>
.detail {
  flex: 1;
  overflow-y: auto;
  min-width: 0;
  background: var(--c-bg);
}
.inner {
  max-width: 740px;
  margin: 0 auto;
  padding: 20px 32px 48px;
}
.prod-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 14px;
}
.title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
h1 {
  margin: 0;
  font-size: 19px;
  font-weight: 680;
  letter-spacing: -0.02em;
  line-height: 1.25;
  color: var(--c-text);
}
.ext {
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 7px;
  color: var(--c-soft);
  cursor: pointer;
}
.ext:hover {
  background: var(--c-hover);
  color: var(--accent);
}
.meta-line {
  display: flex;
  align-items: center;
  gap: 7px;
  margin-top: 6px;
  font-size: 12.5px;
  color: var(--c-soft);
  flex-wrap: wrap;
}
.brand {
  color: var(--c-mid);
  font-weight: 560;
}
.sep {
  color: var(--c-faint);
}
.keyword-row {
  margin-top: 16px;
  display: flex;
  align-items: center;
  gap: 10px;
}
.keyword-row label {
  font-size: 12.5px;
  font-weight: 580;
  color: var(--c-mid);
  display: flex;
  align-items: center;
  gap: 6px;
}
.kw-input {
  flex: 1;
  max-width: 260px;
  height: 32px;
  padding: 0 11px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-card);
  font-size: 13px;
  color: var(--c-text);
  outline: none;
}
.kw-hint {
  font-size: 11px;
  color: var(--c-faint);
}
.no-sel {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--c-faint);
  padding: 40px;
}
.no-sel-icon {
  width: 64px;
  height: 64px;
  border-radius: 16px;
  background: var(--c-panel);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 18px;
  color: var(--c-faint);
}
.no-sel-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--c-soft);
}
.no-sel-sub {
  font-size: 13px;
  margin-top: 6px;
  color: var(--c-faint);
  text-align: center;
  max-width: 340px;
}
</style>
