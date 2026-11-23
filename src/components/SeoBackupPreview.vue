<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { computed, onMounted, onBeforeUnmount, ref, watch } from "vue";
import { teamClient, teamProject, teamSession, teamWorkspace } from "../teamConnection";
import { useTeamTarget } from "../teamTarget";
import Icon from "./Icon.vue";
import ModalShell from "./ModalShell.vue";
import SeoImportReview from "./SeoImportReview.vue";

type Preview = {
  local_only: number;
  backup_only: number;
  identical: number;
  conflicts: { sku: string; differences: { field: string; local: unknown; backup: unknown }[] }[];
  local_rows: { sku: string; draft: Record<string, unknown> }[];
  backup_rows: { sku: string; draft: Record<string, unknown> }[];
};

const emit = defineEmits<{ close: [] }>();
const preview = ref<Preview | null>(null);
const fileName = ref("");
const busy = ref(false);
const uploading = ref(false);
const uploaded = ref(false);
const error = ref("");
const chooseButton = ref<HTMLButtonElement | null>(null);
const { workspaceName: target, canWrite: canStage, checking, statusMessage, retry } = useTeamTarget();
const reviewing = ref(false);
const reviewHasRecords = ref(false);
const applying = ref(false);
const preparedBatch = ref<string | null>(null);
const hasRecords = computed(() => !!(preview.value?.local_rows.length || preview.value?.backup_rows.length));
let app: HTMLElement | null = null;

watch([teamClient, teamWorkspace, () => teamSession.value?.user.id], () => {
  uploaded.value = false;
  reviewing.value = false;
  reviewHasRecords.value = false;
  preparedBatch.value = null;
});

onMounted(() => {
  app = document.getElementById("app");
  if (app) app.inert = true;
  chooseButton.value?.focus();
});
onBeforeUnmount(() => {
  if (app) app.inert = false;
});

async function compareBackup() {
  if (busy.value) return;
  busy.value = true;
  error.value = "";
  try {
    const path = await open({
      title: "Diğer cihazın SQLite SEO yedeğini seçin",
      filters: [{ name: "SQLite", extensions: ["db", "sqlite", "sqlite3"] }],
    });
    if (typeof path !== "string") return;
    uploaded.value = false;
    preparedBatch.value = null;
    fileName.value = path.split(/[\\/]/).pop() ?? "";
    preview.value = null;
    preview.value = await invoke<Preview>("compare_seo_backup", { path });
  } catch (cause) {
    error.value = String(cause);
  } finally {
    busy.value = false;
  }
}

function chooseFromReview() {
  reviewing.value = false;
  void compareBackup();
}

async function stageBackup() {
  const client = teamClient.value;
  const workspace = teamWorkspace.value;
  const actor = teamSession.value?.user.id;
  const source = preview.value;
  if (!client || !workspace || !actor || !source || !canStage.value || !hasRecords.value || uploading.value) return;
  error.value = "";
  uploading.value = true;
  const current = () => teamClient.value === client && teamWorkspace.value === workspace
    && teamSession.value?.user.id === actor;
  try {
    const payload = { local_rows: source.local_rows, backup_rows: source.backup_rows };
    const bytes = new TextEncoder().encode(JSON.stringify([teamProject.value, workspace, actor, payload]));
    const hash = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)).slice(0, 16);
    hash[6] = (hash[6] & 0x0f) | 0x80;
    hash[8] = (hash[8] & 0x3f) | 0x80;
    const hex = Array.from(hash, (byte) => byte.toString(16).padStart(2, "0")).join("");
    const id = hex.replace(/^(........)(....)(....)(....)(............)$/, "$1-$2-$3-$4-$5");
    if (!current()) return;
    const result = await client.rpc("team_stage_seo_import", {
      target_workspace: workspace, operation_id: id, ...payload,
    });
    if (!current()) return;
    if (result.error || result.data !== id) {
      throw new Error("Hazırlık kaydı doğrulanamadı. Bağlantıyı kontrol edip aynı karşılaştırmayla yeniden deneyin.");
    }
    uploaded.value = true;
    preparedBatch.value = id;
  } catch (cause) {
    if (current()) error.value = String(cause);
  } finally {
    uploading.value = false;
  }
}

function display(value: unknown) {
  if (value == null) return "—";
  return typeof value === "string" ? value : JSON.stringify(value, null, 2);
}
</script>

<template>
  <Teleport to="body">
    <ModalShell
      open scroll :width="reviewing && reviewHasRecords ? 940 : !reviewing && preview ? 780 : 520" :closable="!busy && !uploading && !applying" icon="search"
      :label="reviewing ? 'Hazırlanan SEO aktarımları' : 'SEO yedeğini karşılaştır'"
      :title="reviewing ? 'Hazırlanan SEO aktarımları' : 'SEO yedeğini karşılaştır'"
      @close="emit('close')"
    >
      <p v-if="statusMessage" class="preview-note preview-muted" role="status">{{ statusMessage }}</p>
      <SeoImportReview v-if="reviewing" :workspace-name="target" :initial-batch="preparedBatch"
        @applying="applying = $event" @records="reviewHasRecords = $event" @choose="chooseFromReview" />
      <template v-else>
      <div v-if="!preview && !busy" class="preview-intro">
        <p>Diğer cihazdan aldığınız SQLite yedeğini seçin.</p>
        <p class="preview-muted">Taslaklar, teknik tablolar ve sürüm geçmişi SKU bazında salt okunur karşılaştırılır.</p>
      </div>
      <p v-if="busy" class="preview-note" role="status">SEO kayıtları karşılaştırılıyor…</p>
      <p v-if="error" role="alert" class="preview-note preview-error">{{ error }}</p>
      <template v-if="preview">
        <p class="preview-file"><Icon name="database" :size="14" aria-hidden="true" /> {{ fileName }}</p>
        <dl class="preview-counts">
          <div><dt>Yalnız bu cihazda</dt><dd>{{ preview.local_only }}</dd></div>
          <div><dt>Yalnız yedekte</dt><dd>{{ preview.backup_only }}</dd></div>
          <div><dt>Eşleşen</dt><dd>{{ preview.identical }}</dd></div>
          <div :class="{ 'needs-review': preview.conflicts.length }"><dt>İncelenecek</dt><dd>{{ preview.conflicts.length }}</dd></div>
        </dl>
        <details v-for="conflict in preview.conflicts" :key="conflict.sku" class="preview-item">
          <summary>{{ conflict.sku }} · {{ conflict.differences.length }} farklı alan</summary>
          <div v-for="field in conflict.differences" :key="field.field" class="preview-field">
            <b>{{ field.field }}</b>
            <div class="preview-values">
              <div><small>Bu cihaz</small><pre>{{ display(field.local) }}</pre></div>
              <div><small>Yedek</small><pre>{{ display(field.backup) }}</pre></div>
            </div>
          </div>
        </details>
        <p v-if="uploaded" class="preview-note" role="status">İki kaynağın SEO kayıtları {{ target }} çalışma alanında incelemeye hazır.</p>
        <div v-else-if="hasRecords && canStage" class="preview-intro preview-target">
          <p>Hedef çalışma alanı <strong>{{ target }}</strong></p>
          <p class="preview-muted">İki kaynağın SEO kayıtları ayrı saklanır. Ürünlerdeki SEO taslaklarını yayımlamak için ayrıca inceleme gerekir.</p>
        </div>
        <p v-else-if="hasRecords && !statusMessage && !checking" class="preview-note preview-muted">
          Supabase’te hazırlamak için düzenleme yetkisi olan bir ekip oturumu açın.
        </p>
      </template>
      </template>
      <template #footer>
        <button v-if="reviewing" class="secondary-action" type="button" :disabled="applying" @click="reviewing = false">Yedek karşılaştırmasına dön</button>
        <template v-else>
        <button class="secondary-action" type="button" :disabled="busy || uploading" @click="emit('close')">Kapat</button>
        <button ref="chooseButton" :class="preview && canStage && (hasRecords || uploaded) ? 'secondary-action' : 'primary-action'" type="button" :disabled="busy || uploading" @click="compareBackup">
          <Icon name="upload" :size="15" aria-hidden="true" />
          {{ busy ? "Karşılaştırılıyor…" : preview ? "Başka yedek seç" : "SQLite yedeği seç" }}
        </button>
        <button v-if="hasRecords && canStage && !uploaded" class="primary-action" :class="{ busy: uploading }"
          type="button" :disabled="busy || uploading" @click="stageBackup">
          {{ uploading ? "Hazırlanıyor…" : "Supabase’te hazırla" }}
        </button>
        <button v-if="statusMessage && teamSession && !canStage" class="secondary-action" type="button" :disabled="busy || uploading || checking" @click="retry">Yeniden dene</button>
        <button v-if="canStage" :class="uploaded ? 'primary-action' : 'secondary-action'" type="button" :disabled="busy || uploading" @click="reviewing = true">Hazırlananları incele</button>
        </template>
      </template>
    </ModalShell>
  </Teleport>
</template>

<style scoped>
.preview-intro { display: grid; gap: 5px; font-size: 13px; }
.preview-target { border-top: 1px solid var(--c-border); padding-top: 12px; }
.preview-intro p, .preview-note, .preview-file { margin: 0; line-height: 1.5; }
.preview-muted { color: var(--c-soft); }
.preview-note, .preview-file, .preview-item { font-size: 13px; }
.preview-file { display: flex; align-items: center; gap: 7px; overflow-wrap: anywhere; color: var(--c-mid); }
.preview-error, .needs-review dd { color: var(--warn-text); }
.preview-counts { display: grid; grid-template-columns: repeat(auto-fit, minmax(120px, 1fr)); gap: 8px; margin: 4px 0; font-size: 12px; }
.preview-counts div, .preview-item { padding: 10px 12px; border: 1px solid var(--c-border); border-radius: 9px; }
.preview-counts dt { color: var(--c-soft); }
.preview-counts dd { margin: 4px 0 0; font-weight: 600; font-size: 17px; font-variant-numeric: tabular-nums; }
.preview-item summary { cursor: pointer; font-weight: 560; }
.preview-field { margin-top: 12px; }
.preview-values { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 230px), 1fr)); gap: 10px; margin-top: 4px; }
.preview-values pre { white-space: pre-wrap; overflow-wrap: anywhere; max-height: 150px; overflow: auto; margin: 4px 0; padding: 8px; background: var(--c-panel); border-radius: 6px; }
</style>
