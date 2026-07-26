<script setup lang="ts">
import { computed, ref } from "vue";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import type { MetaBadge } from "../types";
import { BADGE_LABEL, descChecks, glen, titleChecks } from "../validation";
import { useStore } from "../store";
import Icon from "./Icon.vue";
import SeoCard from "./SeoCard.vue";

const props = defineProps<{
  title: string;
  desc: string;
  searchKw: string;
  keyword: string;
  metaDone: boolean;
  badge: MetaBadge;
}>();

const emit = defineEmits<{
  "update:title": [string];
  "update:desc": [string];
  "update:searchKw": [string];
  toggleDone: [];
}>();

const store = useStore();

const tLen = computed(() => glen(props.title.trim()));
const dLen = computed(() => glen(props.desc.trim()));
const tChecks = computed(() => titleChecks(props.title, props.keyword));
const dChecks = computed(() => descChecks(props.desc, props.keyword));
const searchOk = computed(() => props.searchKw.trim().length > 0);
const searchWords = computed(
  () => props.searchKw.split(",").map((x) => x.trim()).filter(Boolean).length,
);

function barPct(len: number, max: number) {
  return Math.min(len / max, 1) * 100 + "%";
}
function lenColor(len: number, lo: number, hi: number) {
  if (len === 0) return "var(--c-faint)";
  return len >= lo && len <= hi ? "var(--green)" : "var(--red)";
}

const badgeStyle = computed(() => ({
  background: `var(--badge-${props.badge}-bg)`,
  color: `var(--badge-${props.badge}-c)`,
}));

// Kopyala butonları
const copied = ref<Record<string, boolean>>({});
async function copy(field: string, text: string) {
  try {
    await writeText(text || "");
    copied.value = { ...copied.value, [field]: true };
    setTimeout(() => (copied.value = { ...copied.value, [field]: false }), 1600);
  } catch (e) {
    store.toast("Panoya kopyalanamadı", "error");
  }
}

function genMeta() {
  store.generateMeta();
}
</script>

<template>
  <SeoCard icon="type" title="Meta SEO" :badge-label="BADGE_LABEL[badge]" :badge-style="badgeStyle">
      <!-- Sayfa Başlığı -->
      <div class="field-head">
        <div class="field-label">Sayfa Başlığı</div>
        <button class="copy" :class="{ ok: copied.title }" @click="copy('title', title)">
          <Icon name="copy" :size="12" />
          {{ copied.title ? "Kopyalandı" : "Kopyala" }}
        </button>
      </div>
      <input
        class="fx input"
        :value="title"
        @input="emit('update:title', ($event.target as HTMLInputElement).value)"
        placeholder="Sayfa başlığını yazın veya Gemini ile üretin"
      />
      <div class="meter">
        <div class="track">
          <div class="fill" :style="{ width: barPct(tLen, 60), background: lenColor(tLen, 20, 60) }"></div>
        </div>
        <span class="len" :style="{ color: lenColor(tLen, 20, 60) }">{{ tLen }}</span>
        <span class="range">/ 20–60</span>
      </div>
      <div class="checks">
        <span v-for="(c, i) in tChecks" :key="i" class="check" :style="{ color: c.ok ? 'var(--c-mid)' : 'var(--red)' }">
          <Icon :name="c.ok ? 'check' : 'x'" :size="13" :stroke-width="2.6" :style="{ color: c.ok ? 'var(--green)' : 'var(--red)' }" />
          {{ c.label }}
        </span>
      </div>

      <!-- Meta Açıklama -->
      <div class="field-head mt">
        <div class="field-label">Meta Açıklama</div>
        <button class="copy" :class="{ ok: copied.desc }" @click="copy('desc', desc)">
          <Icon name="copy" :size="12" />
          {{ copied.desc ? "Kopyalandı" : "Kopyala" }}
        </button>
      </div>
      <textarea
        class="fx textarea"
        rows="3"
        :value="desc"
        @input="emit('update:desc', ($event.target as HTMLTextAreaElement).value)"
        placeholder="Meta açıklamayı yazın veya Gemini ile üretin"
      ></textarea>
      <div class="meter">
        <div class="track">
          <div class="fill" :style="{ width: barPct(dLen, 155), background: lenColor(dLen, 50, 155) }"></div>
        </div>
        <span class="len" :style="{ color: lenColor(dLen, 50, 155) }">{{ dLen }}</span>
        <span class="range">/ 50–155</span>
      </div>
      <div class="checks">
        <span v-for="(c, i) in dChecks" :key="i" class="check" :style="{ color: c.ok ? 'var(--c-mid)' : 'var(--red)' }">
          <Icon :name="c.ok ? 'check' : 'x'" :size="13" :stroke-width="2.6" :style="{ color: c.ok ? 'var(--green)' : 'var(--red)' }" />
          {{ c.label }}
        </span>
      </div>

      <!-- Site İçi Arama Kelimeleri -->
      <div class="field-head mt">
        <div class="field-label">Site İçi Arama Kelimeleri</div>
        <button class="copy" :class="{ ok: copied.search }" @click="copy('search', searchKw)">
          <Icon name="copy" :size="12" />
          {{ copied.search ? "Kopyalandı" : "Kopyala" }}
        </button>
      </div>
      <textarea
        class="fx textarea small"
        rows="2"
        :value="searchKw"
        @input="emit('update:searchKw', ($event.target as HTMLTextAreaElement).value)"
        placeholder="virgülle ayırın · ör. kulaklık, kablosuz kulaklık, gürültü önleyici"
      ></textarea>
      <div class="search-foot">
        <span class="check" :style="{ color: searchOk ? 'var(--c-mid)' : 'var(--red)' }">
          <Icon :name="searchOk ? 'check' : 'x'" :size="13" :stroke-width="2.6" :style="{ color: searchOk ? 'var(--green)' : 'var(--red)' }" />
          Boş değil
        </span>
        <span class="hint">{{ searchWords }} kelime · karakter sınırı yok</span>
      </div>

    <template #actions>
      <button class="gen" :class="{ busy: store.generating }" :disabled="store.generating" @click="genMeta">
        <Icon :name="store.generating ? 'loader' : 'sparkles'" :size="15" :stroke-width="store.generating ? 2.2 : 1.9" :class="{ spin: store.generating }" />
        {{ store.generating ? "Üretiliyor…" : "Gemini ile Üret" }}
      </button>
        <button
          v-if="store.settings?.ideasoft_active"
          class="is-push"
          :disabled="store.ideasoftBusy"
          title="Bu kartın içeriğini IdeaSoft'a gönder"
          @click="store.openIdeasoftPreview(['meta'])"
        >
          <Icon name="upload" :size="14" />
          IdeaSoft'a Gönder
        </button>
      <div style="flex:1"></div>
      <button class="done" :class="{ active: metaDone }" @click="emit('toggleDone')">
        <Icon name="badgeCheck" :size="15" :stroke-width="2.2" />
        {{ metaDone ? "Meta tamamlandı ✓" : "Meta'yı Tamamlandı işaretle" }}
      </button>
    </template>
  </SeoCard>
</template>

<style scoped>
.field-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 7px;
}
.field-head.mt {
  margin-top: 16px;
}
.field-label {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--c-mid);
}
.copy {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 3px 9px;
  border: 1px solid var(--c-border);
  border-radius: 7px;
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 11px;
  font-weight: 560;
  cursor: pointer;
}
.copy:hover {
  background: var(--c-hover);
}
.copy.ok {
  color: var(--green);
}
.input {
  width: 100%;
  height: 38px;
  padding: 0 12px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  font-size: 14px;
  color: var(--c-text);
  background: var(--c-input);
  outline: none;
}
.textarea {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  font-size: 13.5px;
  line-height: 1.5;
  color: var(--c-text);
  background: var(--c-input);
  outline: none;
  resize: vertical;
  min-height: 66px;
}
.textarea.small {
  font-size: 13px;
  min-height: 52px;
}
.meter {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 9px;
}
.track {
  flex: 1;
  height: 5px;
  border-radius: 4px;
  background: var(--c-track);
  overflow: hidden;
}
.fill {
  height: 100%;
  border-radius: 4px;
  transition: width 0.2s ease, background 0.2s ease;
}
.len {
  font-size: 12px;
  font-weight: 600;
}
.range {
  font-size: 11.5px;
  color: var(--c-faint);
}
.checks {
  display: flex;
  flex-wrap: wrap;
  gap: 7px 16px;
  margin-top: 9px;
}
.check {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 11.5px;
}
.search-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 9px;
}
.hint {
  font-size: 11px;
  color: var(--c-faint);
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
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
}
.gen:hover {
  filter: brightness(1.05);
}
.gen.busy {
  opacity: 0.75;
  cursor: default;
}
.is-push {
  display: inline-flex;
  align-items: center;
  gap: 7px;
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
.is-push:hover {
  border-color: var(--accent);
  color: var(--accent);
}
.is-push:disabled {
  opacity: 0.5;
  cursor: default;
}
.spin {
  animation: spin 0.8s linear infinite;
}
.done {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  height: 38px;
  padding: 0 14px;
  border: 1px solid var(--badge-uygun-bg);
  border-radius: 9px;
  background: var(--badge-uygun-bg);
  color: var(--green);
  font-size: 12.5px;
  font-weight: 580;
  cursor: pointer;
}
.done:hover {
  filter: brightness(0.98);
}
</style>
