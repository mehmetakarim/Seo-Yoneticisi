<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useStore } from "../store";
import { computeBadge } from "../validation";
import Icon from "./Icon.vue";
import MetaSeoCard from "./MetaSeoCard.vue";
import DetailsSeoCard from "./DetailsSeoCard.vue";
import SeoResearchPanel from "./SeoResearchPanel.vue";
import ImageScoreCard from "./ImageScoreCard.vue";
import TechTableCard from "./TechTableCard.vue";
import SchemaCard from "./SchemaCard.vue";
import FeedChangeNotice from "./FeedChangeNotice.vue";
import OutcomeTimeline from "./OutcomeTimeline.vue";
import QueryComparisonModal from "./QueryComparisonModal.vue";
import ProductContacts from "./contacts/ProductContacts.vue";
import IdeasoftPushModal from "./IdeasoftPushModal.vue";
import { teamSeoSync } from "../teamProductSeo";

const store = useStore();
const detail = computed(() => store.detail);
const sharedWithoutDraft = computed(() => Boolean(detail.value?.team_seo_scope && detail.value.seo_revision == null));
const metaLocked = computed(() => store.generating || Boolean(detail.value?.team_seo_scope &&
  (store.generatingDetails || store.techStructuring || store.techTableSaving || store.ideasoftBusy)));
const showResearch = ref(false);
const showComparison = ref(false);
const comparisonButton = ref<HTMLButtonElement | null>(null);
const detailPane = ref<HTMLElement | null>(null);

watch(() => store.selectedSku, () => { showComparison.value = false; });

async function closeComparison() {
  showComparison.value = false;
  await nextTick();
  comparisonButton.value?.focus();
}

watch(
  () => detail.value?.sku,
  () => {
    if (detailPane.value) detailPane.value.scrollTop = 0;
  },
  { flush: "post" },
);

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
    const useDraft = !d.team_seo_scope || d.seo_revision != null;
    title.value = (useDraft ? d.draft_title : null) ?? d.title ?? "";
    desc.value = (useDraft ? d.draft_descriptions : null) ?? d.descriptions ?? "";
    searchKw.value = (useDraft ? d.draft_search_keywords : null) ?? d.search_keywords ?? "";
    keyword.value = useDraft ? d.target_keyword ?? "" : "";
  },
  { immediate: true },
);

watch(() => detail.value?.draft_title ?? detail.value?.title ?? "", (value, previous) => {
  if (title.value === previous) title.value = value;
});
watch(() => detail.value?.draft_descriptions ?? detail.value?.descriptions ?? "", (value, previous) => {
  if (desc.value === previous) desc.value = value;
});
watch(() => detail.value?.draft_search_keywords ?? detail.value?.search_keywords ?? "", (value, previous) => {
  if (searchKw.value === previous) searchKw.value = value;
});
watch(() => detail.value?.target_keyword ?? "", (value, previous) => {
  if (keyword.value.trim() === previous) keyword.value = value;
});
watch(
  () => sharedWithoutDraft.value ? "" : detail.value?.target_keyword ?? "",
  (next, previous) => {
    if (keyword.value.trim() === previous) keyword.value = next;
  },
);

const liveBadge = computed(() =>
  computeBadge(title.value, desc.value, keyword.value, !sharedWithoutDraft.value && store.detail?.meta_status === "done"),
);

function scheduleDraftSave() {
  const product = detail.value;
  if (product) store.scheduleProductMetaEdits(product, {
    draft_title: title.value, draft_descriptions: desc.value, draft_search_keywords: searchKw.value,
  });
}

function scheduleKeywordSave() {
  const product = detail.value;
  if (product) store.scheduleProductMetaEdits(product, { target_keyword: keyword.value.trim() });
}

function onTitle(v: string) { title.value = v; scheduleDraftSave(); }
function onDesc(v: string) { desc.value = v; scheduleDraftSave(); }
function onSearch(v: string) { searchKw.value = v; scheduleDraftSave(); }
function onKeyword(e: Event) {
  keyword.value = (e.target as HTMLInputElement).value;
  scheduleKeywordSave();
}

// Araştırma panelinden hedef kelime seçildi → alanı doldur + kaydet.
function onPickKeyword(kw: string) {
  keyword.value = kw;
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
  <section ref="detailPane" class="detail om-scroll">
    <div v-if="detail" class="inner">
      <header class="detail-header">
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
              <template v-if="detail.ideasoft_seo_rule !== null">
                <span class="sep">·</span>
                <span class="seo-rule" title="IdeaSoft'un kendi SEO kural skoru">
                  IdeaSoft SEO: <b>{{ detail.ideasoft_seo_rule }}</b>
                </span>
              </template>
              <template v-if="detail.ideasoft_pushed_at">
                <span class="sep">·</span>
                <span class="pushed">IdeaSoft'a gönderildi · {{ detail.ideasoft_pushed_at.replace("T"," ").slice(5,16) }}</span>
              </template>
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
            :disabled="metaLocked || store.generatingDetails"
            @input="onKeyword"
            placeholder="ör. bluetooth kulaklık"
          />
          <button v-if="!detail.catalog_only" class="research-btn" title="Gerçek SEO verisiyle araştır" @click="showResearch = true">
            <Icon name="search" :size="13" :stroke-width="2" />
            SEO Araştır
          </button>
          <button
            v-if="!detail.catalog_only || detail.seo_revision != null"
            ref="comparisonButton"
            type="button"
            class="research-btn"
            aria-haspopup="dialog"
            title="7 günlük GSC sorgu karşılaştırmasını aç"
            @click="showComparison = true"
          >
            <Icon name="chartLine" :size="14" :stroke-width="1.9" />
            Sorgu Karşılaştır
          </button>
          <template v-if="store.productIdeasoftAvailable">
            <button
              class="research-btn"
              :disabled="store.ideasoftBusy"
              title="Hedef kelimeyi ve varsa teknik tablo verisini IdeaSoft'tan getir"
              @click="store.pullIdeasoftKeyword()"
            >
              <Icon name="download" :size="13" :stroke-width="2" />
              Getir
            </button>
            <button
              class="research-btn"
              :disabled="store.ideasoftBusy || !keyword.trim()"
              title="Hedef kelimeyi IdeaSoft'a gönder (SEO kural skorunu etkiler)"
              @click="store.openIdeasoftPreview(['keyword'])"
            >
              <Icon name="upload" :size="13" :stroke-width="2" />
              Gönder
            </button>
          </template>
        </div>
      </header>

      <p v-if="detail.seo_revision != null" class="shared-seo-status" role="status">
        Ortak SEO taslağı · v{{ detail.seo_revision }}
        <span v-if="teamSeoSync.pending"> · {{ teamSeoSync.pending }} değişiklik gönderim bekliyor</span>
        <span v-if="teamSeoSync.conflicts"> · {{ teamSeoSync.conflicts }} ürünün düzenlemesi çakıştı</span>
      </p>
      <!-- feed değişikliği uyarısı + karşılaştırma: onaydan SONRA kaynak veri değişmişse çıkar -->
      <FeedChangeNotice v-if="detail.feed_changed" :changed="detail.feed_changed" />

      <ImageScoreCard :gallery="detail.gallery" :image-count="detail.image_count" />

      <MetaSeoCard
        :key="detail.sku"
        :title="title"
        :desc="desc"
        :search-kw="searchKw"
        :keyword="keyword"
        :meta-done="!sharedWithoutDraft && detail.meta_status === 'done'"
        :model="sharedWithoutDraft ? null : detail.meta_model"
        :history="sharedWithoutDraft ? [] : detail.meta_history"
        :badge="liveBadge"
        :locked="metaLocked"
        @update:title="onTitle"
        @update:desc="onDesc"
        @update:search-kw="onSearch"
        @toggle-done="store.toggleMetaDone()"
      />

      <DetailsSeoCard
        :details-html="(sharedWithoutDraft ? null : detail.draft_details) ?? detail.details ?? ''"
        :keyword="keyword"
        :details-done="!sharedWithoutDraft && detail.details_status === 'done'"
        :badge="sharedWithoutDraft ? 'eksik' : detail.details_badge"
        :image-count="detail.image_count"
        :model="sharedWithoutDraft ? null : detail.details_model"
        :history="sharedWithoutDraft ? [] : detail.details_history"
      />

      <TechTableCard
        :source-text="detail.tech_source_text ?? ''"
        :specs="detail.tech_specs"
        :model="detail.tech_model"
        :tech-done="detail.tech_status === 'done'"
        :badge="detail.tech_badge"
        :history="detail.tech_history"
      />

      <!-- JSON-LD teknik tablodan besleniyor → hemen onun altında -->
      <SchemaCard />

      <template v-if="!detail.catalog_only || detail.seo_revision != null">
      <!-- İlgilenen kişiler de bir sonuçtur: üretim kartlarından sonra, geçmişten önce. -->
      <ProductContacts />

      <!-- "Ne yaptık, sonra ne oldu" — en altta: geçmiş, üretim kartlarının ardından okunur -->
      <OutcomeTimeline />
      <QueryComparisonModal
        v-if="showComparison"
        :sku="detail.sku"
        :product="detail.name"
        :keyword="keyword"
        @close="closeComparison"
      />
      </template>
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

    <IdeasoftPushModal />

    <SeoResearchPanel
      :open="showResearch"
      :keyword="keyword"
      @close="showResearch = false"
      @pick="onPickKeyword"
    />
  </section>
</template>

<style scoped>
.shared-seo-status { margin: 10px 0; font-size: 12px; color: var(--c-mid); }
.detail {
  flex: 1;
  overflow-y: auto;
  min-width: 0;
  background: var(--c-bg);
}
.inner {
  max-width: 860px;
  margin: 0 auto;
  padding: 0 32px 48px;
}
.detail-header {
  position: sticky;
  top: 0;
  z-index: 2;
  margin: 0 -12px 4px;
  padding: 20px 12px 12px;
  background: var(--c-bg);
  border-bottom: 1px solid var(--c-border);
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
.pushed {
  color: var(--green);
  font-weight: 560;
}
.seo-rule {
  color: var(--accent);
  font-weight: 560;
}
.research-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.keyword-row {
  margin-top: 16px;
  display: flex;
  flex-wrap: wrap;
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
  min-width: 130px;
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
.research-btn {
  flex: none;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 12px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-card);
  color: var(--c-mid);
  font-size: 12px;
  font-weight: 560;
  cursor: pointer;
  transition: border-color 0.15s ease, color 0.15s ease;
}
.research-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
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
