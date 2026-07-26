<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { api } from "../api";
import { useStore } from "../store";
import type { MetaBadge, TechGroup, TechVersionMeta } from "../types";
import { BADGE_LABEL } from "../validation";
import Icon from "./Icon.vue";

const props = defineProps<{
  sourceText: string;
  specs: TechGroup[] | null;
  techDone: boolean;
  badge: MetaBadge;
  history: TechVersionMeta[];
}>();

const store = useStore();

const source = ref("");
const editing = ref(false);

watch(
  () => [props.sourceText, store.selectedSku],
  () => {
    source.value = props.sourceText ?? "";
    editing.value = false;
  },
  { immediate: true },
);

const groups = computed(() => props.specs ?? []);
const rowCount = computed(() => groups.value.reduce((a, g) => a + g.rows.length, 0));
const hasSpecs = computed(() => rowCount.value > 0);

const badgeStyle = computed(() => ({
  background: `var(--badge-${props.badge}-bg)`,
  color: `var(--badge-${props.badge}-c)`,
}));

// Ham metin kaydı — debounce (ProductDetail deseni)
let srcTimer: number | undefined;
function onSource(e: Event) {
  source.value = (e.target as HTMLTextAreaElement).value;
  window.clearTimeout(srcTimer);
  srcTimer = window.setTimeout(() => store.saveTechSource(source.value), 600);
}

async function structure() {
  window.clearTimeout(srcTimer);
  await store.saveTechSource(source.value);
  await store.structureTech();
}

const copied = ref(false);
async function copyHtml() {
  if (!store.selectedSku) return;
  try {
    const html = await api.techTableHtml(store.selectedSku);
    await writeText(html);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1600);
  } catch (e) {
    store.toast(String(e), "error");
  }
}

// --- satır düzenleme ---
function edit(gi: number, ri: number, field: "label" | "value", e: Event) {
  const next = groups.value.map((g) => ({ group: g.group, rows: g.rows.map((r) => ({ ...r })) }));
  next[gi].rows[ri][field] = (e.target as HTMLElement).innerText.trim();
  store.saveTechSpecs(next);
}
// --- sürüm geçmişi ---
const histOpen = ref(false);
function fmtDate(s: string): string {
  return s.replace("T", " ").slice(0, 16);
}
function restore(i: number) {
  store.restoreTechVersion(i);
  histOpen.value = false;
}

function removeRow(gi: number, ri: number) {
  const next = groups.value
    .map((g, i) => ({
      group: g.group,
      rows: g.rows.filter((_, j) => !(i === gi && j === ri)),
    }))
    .filter((g) => g.rows.length > 0);
  store.saveTechSpecs(next);
}
</script>

<template>
  <div class="card">
    <div class="card-head">
      <div class="head-left">
        <div class="icon-badge"><Icon name="clipboardList" :size="15" /></div>
        <div>
          <div class="head-title">Teknik Tablo</div>
          <div class="head-sub">Web site teknik özellik alanı için</div>
        </div>
      </div>
      <span class="status" :style="badgeStyle">
        <span class="sdot" :style="{ background: badgeStyle.color }"></span>
        {{ BADGE_LABEL[badge] }}
      </span>
    </div>

    <div class="card-body">
      <!-- Üst bilgi şeridi (DetailsSeoCard ile aynı düzen) -->
      <div class="info-row">
        <span class="info">
          <Icon name="info" :size="13" />
          Yalnızca yapıştırılan metindeki değerler kullanılır — özellik uydurulmaz.
        </span>
        <button v-if="hasSpecs" class="copy" :class="{ ok: copied }" @click="copyHtml">
          <Icon name="copy" :size="12" />
          {{ copied ? "Kopyalandı" : "HTML kopyala" }}
        </button>
      </div>

      <!-- Kaynak metin -->
      <div class="src-block" v-if="!hasSpecs || editing">
        <label class="lbl">
          <Icon name="fileText" :size="13" />
          Üreticinin resmi teknik özellik metnini yapıştırın
        </label>
        <textarea
          class="fx src"
          :value="source"
          @input="onSource"
          rows="7"
          placeholder="Örn. üreticinin ürün sayfasındaki teknik özellik bölümünü kopyalayıp buraya yapıştırın…"
        ></textarea>
      </div>

      <!-- Doğrulanamayan satır uyarısı -->
      <div v-if="store.techDropped.length" class="warn">
        <Icon name="alert" :size="13" />
        {{ store.techDropped.length }} satır kaynak metinde doğrulanamadığı için atlandı:
        <b>{{ store.techDropped.slice(0, 6).join(", ") }}</b>
      </div>

      <!-- Sonuç tablosu -->
      <div v-if="hasSpecs" class="preview om-scroll">
        <div v-for="(g, gi) in groups" :key="gi" class="grp">
          <div class="grp-title">{{ g.group }}</div>
          <div v-for="(r, ri) in g.rows" :key="ri" class="row">
            <div
              class="cell lab"
              contenteditable
              spellcheck="false"
              @blur="edit(gi, ri, 'label', $event)"
            >{{ r.label }}</div>
            <div
              class="cell val"
              contenteditable
              spellcheck="false"
              @blur="edit(gi, ri, 'value', $event)"
            >{{ r.value }}</div>
            <button class="del" title="Satırı sil" @click="removeRow(gi, ri)">
              <Icon name="x" :size="12" :stroke-width="2.4" />
            </button>
          </div>
        </div>
      </div>

      <div v-if="hasSpecs" class="meta-line">
        <span>{{ rowCount }} satır · {{ groups.length }} grup</span>
        <span class="sep">·</span>
        <a class="link" @click="editing = !editing">
          {{ editing ? "kaynak metni gizle" : "kaynak metni düzenle" }}
        </a>
        <template v-if="history.length">
          <span class="sep">·</span>
          <a class="link" @click="histOpen = !histOpen">
            önceki sürümler ({{ history.length }})
          </a>
        </template>
        <span class="hint">hücrelere tıklayarak düzenleyebilirsiniz</span>
      </div>

      <!-- Sürüm geçmişi -->
      <div v-if="histOpen && history.length" class="hist">
        <div class="hist-head">
          <Icon name="refresh" :size="12" />
          Yeniden üretimden önceki tablolar — geri yüklemek mevcut tabloyu da saklar
        </div>
        <div v-for="(v, i) in history" :key="i" class="hist-row">
          <span class="hist-at">{{ fmtDate(v.at) }}</span>
          <span class="hist-meta">{{ v.rows }} satır · {{ v.groups }} grup</span>
          <button class="hist-btn" @click="restore(i)">Geri yükle</button>
        </div>
      </div>
    </div>

    <div class="card-actions">
      <button
        class="gen"
        :class="{ busy: store.techStructuring }"
        :disabled="store.techStructuring || !source.trim()"
        @click="structure"
      >
        <Icon
          :name="store.techStructuring ? 'loader' : 'sparkles'"
          :size="15"
          :stroke-width="store.techStructuring ? 2.2 : 1.9"
          :class="{ spin: store.techStructuring }"
        />
        {{ store.techStructuring ? "Yapılandırılıyor…" : hasSpecs ? "Yeniden Yapılandır" : "Yapılandır" }}
      </button>
        <button
          v-if="store.settings?.ideasoft_active"
          class="is-push"
          :disabled="store.ideasoftBusy"
          title="Bu kartın içeriğini IdeaSoft'a gönder"
          @click="store.openIdeasoftPreview(['tech'])"
        >
          <Icon name="upload" :size="14" />
          IdeaSoft'a Gönder
        </button>
      <div style="flex:1"></div>
      <button class="done" :class="{ active: techDone }" @click="store.toggleTechDone()">
        <Icon name="badgeCheck" :size="15" :stroke-width="2.2" />
        {{ techDone ? "Tablo tamamlandı ✓" : "Tabloyu Tamamlandı işaretle" }}
      </button>
    </div>
  </div>
</template>

<style scoped>
/* Kart kabuğu diğer üç kartla birebir aynı */
.card {
  margin-top: 16px;
  border: 1px solid var(--c-border-soft);
  border-radius: 13px;
  background: var(--c-card);
  overflow: hidden;
}
.card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 13px 16px;
  border-bottom: 1px solid var(--c-border-soft);
}
.head-left {
  display: flex;
  align-items: center;
  gap: 10px;
}
.icon-badge {
  width: 26px;
  height: 26px;
  border-radius: 8px;
  background: var(--accent-tint);
  color: var(--accent);
  display: flex;
  align-items: center;
  justify-content: center;
}
.head-title {
  font-size: 13.5px;
  font-weight: 640;
  color: var(--c-text);
}
.head-sub {
  font-size: 11px;
  color: var(--c-soft);
  margin-top: 1px;
}
.status {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 11.5px;
  font-weight: 600;
  padding: 3px 9px;
  border-radius: 999px;
}
.sdot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}
.card-body {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.lbl {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12.5px;
  font-weight: 580;
  color: var(--c-mid);
  margin-bottom: 7px;
}
.src {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-input);
  font-size: 12.5px;
  line-height: 1.55;
  color: var(--c-text);
  outline: none;
  resize: vertical;
  font-family: inherit;
}
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
.warn {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  padding: 8px 10px;
  font-size: 11.5px;
  color: var(--warn-text);
  background: var(--warn-bg);
  border: 1px solid var(--warn-border);
  border-radius: 8px;
}
.preview {
  max-height: 420px;
  overflow-y: auto;
  border: 1px solid var(--c-border-soft);
  border-radius: 10px;
  background: var(--c-list);
  padding: 4px 0;
}
.grp {
  animation: popIn 0.22s ease both;
}
.grp-title {
  font-size: 11.5px;
  font-weight: 660;
  color: var(--c-mid);
  background: var(--c-chip);
  padding: 7px 12px;
  position: sticky;
  top: 0;
}
.row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--c-border-soft);
}
.row:last-child {
  border-bottom: 0;
}
.cell {
  font-size: 12.5px;
  line-height: 1.5;
  outline: none;
  border-radius: 5px;
  padding: 2px 4px;
  overflow-wrap: anywhere;
}
.cell:focus {
  background: var(--accent-tint);
}
.lab {
  width: 35%;
  flex: none;
  color: var(--c-mid);
  font-weight: 550;
}
.val {
  flex: 1;
  color: var(--c-text);
  font-variant-numeric: tabular-nums;
}
.del {
  flex: none;
  width: 22px;
  height: 22px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--c-faint);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
}
.row:hover .del {
  opacity: 1;
}
.del:hover {
  background: var(--badge-eksik-bg);
  color: var(--red);
}
.meta-line {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11.5px;
  color: var(--c-soft);
  flex-wrap: wrap;
}
.sep {
  color: var(--c-faint);
}
.link {
  color: var(--accent);
  cursor: pointer;
  font-weight: 560;
}
.link:hover {
  text-decoration: underline;
}
.hint {
  color: var(--c-faint);
  margin-left: auto;
}
/* Sürüm geçmişi */
.hist {
  border: 1px solid var(--c-border-soft);
  border-radius: 10px;
  background: var(--c-list);
  overflow: hidden;
  animation: popIn 0.22s ease both;
}
.hist-head {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--c-soft);
  background: var(--c-chip);
  padding: 7px 12px;
}
.hist-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--c-border-soft);
}
.hist-row:last-child {
  border-bottom: 0;
}
.hist-at {
  font-size: 12px;
  color: var(--c-text);
  font-variant-numeric: tabular-nums;
}
.hist-meta {
  font-size: 11.5px;
  color: var(--c-soft);
}
.hist-btn {
  margin-left: auto;
  height: 26px;
  padding: 0 10px;
  border: 1px solid var(--c-border);
  border-radius: 7px;
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 11.5px;
  font-weight: 560;
  cursor: pointer;
}
.hist-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
}
.card-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 16px;
  border-top: 1px solid var(--c-border-soft);
  flex-wrap: wrap;
}
/* MetaSeoCard/DetailsSeoCard ile birebir aynı birincil buton */
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
.gen:disabled:not(.busy) {
  opacity: 0.45;
  cursor: not-allowed;
}
.ghost {
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
.ghost:hover {
  background: var(--c-hover);
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
.done.active {
  border-color: var(--green);
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
</style>
