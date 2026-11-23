<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { open as openFile } from "@tauri-apps/plugin-dialog";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { teamClient, teamSession, teamWorkspace } from "../teamConnection";
import { useTeamTarget } from "../teamTarget";
import Icon from "./Icon.vue";
import ModalShell from "./ModalShell.vue";

type Backup = {
  contacts: { email: string; phone: string }[];
  quotes: { no: string }[];
  contact_events: unknown[];
  contact_tags: unknown[];
  contact_products: unknown[];
  quote_items: unknown[];
  quote_versions: unknown[];
};
const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ close: []; connect: [] }>();
const sourceName = ref("");
const path = ref<string | null>(null);
const { workspaceName, canWrite: canImport, checking: checkingWorkspace, connected, statusMessage, retry } = useTeamTarget();
const preview = ref<{ backup: Backup; contacts: number; quotes: number } | null>(null);
const result = ref<(Record<keyof Backup | "renamed_quotes", number> & { already_imported?: boolean }) | null>(null);
const error = ref("");
const busy = ref(false);
const fileName = computed(() => path.value?.split(/[\\/]/).pop() ?? "Bu bilgisayardaki yerel veritabanı");
let app: HTMLElement | null = null;
onMounted(() => {
  app = document.getElementById("app");
  if (app) app.inert = true;
});
onBeforeUnmount(() => {
  if (app) app.inert = false;
});
const labels: Record<keyof Backup, string> = {
  contacts: "Müşteriler", contact_events: "Temaslar", contact_tags: "Etiketler", contact_products: "Ürün bağlantıları",
  quotes: "Teklifler", quote_items: "Teklif satırları", quote_versions: "Teklif sürümleri",
};
const tables = Object.keys(labels) as (keyof Backup)[];

watch([() => props.open, teamClient, teamWorkspace, () => teamSession.value?.user.id], () => {
  clearPreview();
}, { immediate: true });

function clearPreview() {
  preview.value = null;
  result.value = null;
  error.value = "";
}

async function chooseFile() {
  try {
    const picked = await openFile({ title: "Aktarılacak SQLite yedeğini seçin", filters: [{ name: "SQLite", extensions: ["db", "sqlite", "sqlite3"] }] });
    if (typeof picked === "string") {
      path.value = picked;
      clearPreview();
    }
  } catch {
    error.value = "Yedek dosyası seçilemedi.";
  }
}

async function prepare() {
  const client = teamClient.value;
  const workspace = teamWorkspace.value;
  if (!client || !canImport.value || busy.value) return;
  clearPreview();
  busy.value = true;
  try {
    const backup = await invoke<Backup>("read_team_backup", { path: path.value });
    const emails = new Set<string>();
    const phones = new Set<string>();
    const numbers = new Set<string>();
    const normalizePhone = (value: string) => value.replace(/\D/g, "").slice(-10);
    const size = 200;
    for (let index = 0; ; index += size) {
      const [contacts, quotes] = await Promise.all([
        client.from("team_contacts").select("email,phone").eq("workspace_id", workspace).order("id").range(index, index + size - 1),
        client.from("team_quotes").select("no").eq("workspace_id", workspace).order("id").range(index, index + size - 1),
      ]);
      if (contacts.error || quotes.error) throw new Error("Hedef çalışma alanındaki kayıtlar karşılaştırılamadı.");
      for (const person of contacts.data) {
        if (person.email.trim()) emails.add(person.email.trim().toLowerCase());
        const phone = normalizePhone(person.phone);
        if (phone.length === 10) phones.add(phone);
      }
      for (const quote of quotes.data) numbers.add(quote.no);
      if (contacts.data.length < size && quotes.data.length < size) break;
    }
    if (teamClient.value !== client || teamWorkspace.value !== workspace) return;
    const contacts = backup.contacts.filter((person) => {
      const email = person.email.trim().toLowerCase();
      const phone = normalizePhone(person.phone);
      const duplicate = emails.has(email) || phones.has(phone);
      if (email) emails.add(email);
      if (phone.length === 10) phones.add(phone);
      return duplicate;
    }).length;
    preview.value = { backup, contacts, quotes: backup.quotes.filter((quote) => numbers.has(quote.no)).length };
  } catch (cause) {
    error.value = String(cause);
  } finally {
    busy.value = false;
  }
}

async function transfer() {
  if (!preview.value || !canImport.value || !teamClient.value || busy.value) return;
  const client = teamClient.value;
  const workspace = teamWorkspace.value;
  busy.value = true;
  error.value = "";
  try {
    const { data, error: failure } = await client.rpc("team_import_backup", {
      target_workspace: workspace, source_label: sourceName.value.trim(), backup: preview.value.backup,
    });
    if (failure) {
      if (failure.code === "PGRST202") throw new Error("Bu projede yedek aktarımı migration’ı henüz uygulanmamış.");
      if (failure.code === "22023") throw new Error("Bu kaynak adı farklı bir yedekte kullanılmış veya yedek geçersiz.");
      throw new Error("Supabase aktarımı doğrulanamadı. Aynı yedeği tekrar denerseniz tamamlanmış aktarım çoğaltılmaz.");
    }
    result.value = data;
    preview.value = null;
  } catch (cause) {
    error.value = (cause as Error).message;
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <Teleport to="body">
    <div @keydown.stop @keydown.esc.stop="!busy && emit('close')">
      <ModalShell :open="props.open" title="Supabase’e aktar" label="Supabase’e aktar" icon="database" scroll :width="560" :closable="!busy" @close="emit('close')">
        <p v-if="statusMessage" class="backup-note" role="status">{{ statusMessage }}</p>
        <p v-else-if="!connected" class="backup-note">Aktarım için Sistem → Ekip bölümünde oturum açıp bir çalışma alanı seçin.</p>
        <template v-if="connected">
          <p v-if="checkingWorkspace" class="backup-note" role="status">Hedef çalışma alanı ve aktarım yetkisi doğrulanıyor…</p>
          <p v-if="workspaceName" class="backup-note">Hedef çalışma alanı <strong>{{ workspaceName }}</strong></p>
          <p v-if="workspaceName && !canImport" class="backup-note">Bu çalışma alanına aktarmak için düzenleme yetkisi gerekiyor.</p>
          <div v-if="canImport && !result" class="source">
            <label for="team-backup-source">Kaynak adı</label>
            <input id="team-backup-source" v-model="sourceName" class="fx" maxlength="80" placeholder="Ör. Ofis bilgisayarı" :disabled="busy" @input="clearPreview" />
            <div class="source-picker">
              <span>{{ fileName }}</span>
              <button class="secondary-action" type="button" :disabled="busy" @click="chooseFile">SQLite yedeği seç</button>
              <button v-if="path" class="secondary-action" type="button" :disabled="busy" @click="path = null; clearPreview()">Yerel veritabanını kullan</button>
            </div>
          </div>
          <p v-if="canImport && !result" class="backup-note">Müşteri, temas ve teklif verileri aktarılır. Ürün kataloğu, SEO verileri ve bağlantı anahtarları bu aktarıma dahil değildir.</p>
          <template v-if="preview">
            <dl class="counts">
              <div v-for="table in tables" :key="table"><dt>{{ labels[table] }}</dt><dd>{{ preview.backup[table].length }}</dd></div>
            </dl>
            <p v-if="preview.contacts" class="backup-note">{{ preview.contacts }} olası müşteri eşleşmesi var. Kayıtlar ayrı korunur.</p>
            <p v-if="preview.quotes" class="backup-note">{{ preview.quotes }} teklif numarası çakışıyor. Aktarılan numaralar ayırt edici bir ek alır.</p>
            <p class="backup-note">Onayladığınızda önizlemedeki müşteri bilgileri ve teklif kayıtları yukarıdaki çalışma alanına gönderilir.</p>
          </template>
          <template v-if="result">
            <p v-if="result.already_imported" class="backup-note" role="status">Bu yedek daha önce aktarılmış. Yeni kayıt eklenmedi.</p>
            <template v-else>
              <p class="backup-note" role="status">Aktarım tamamlandı. Yerel yedek duruyor.</p>
              <dl class="counts">
                <div v-for="table in tables" :key="table"><dt>{{ labels[table] }}</dt><dd>{{ result[table] }}</dd></div>
              </dl>
              <p v-if="result.renamed_quotes" class="backup-note">{{ result.renamed_quotes }} teklif numarasına ayırt edici ek eklendi.</p>
            </template>
          </template>
        </template>
        <p v-if="error" class="backup-error" role="alert">{{ error }}</p>
        <template #footer>
          <button type="button" class="secondary-action" :disabled="busy" @click="emit('close')">{{ result ? "Kapat" : "Vazgeç" }}</button>
          <button v-if="statusMessage && connected && !canImport" type="button" class="secondary-action" :disabled="checkingWorkspace || busy" @click="retry">Yeniden dene</button>
          <button v-else-if="!connected && !statusMessage" type="button" class="primary-action" @click="emit('connect')">Ekip ekranına git</button>
          <button v-else-if="canImport && !result" type="button" class="primary-action"
            :disabled="busy || sourceName.trim().length < 3 || (!!preview && !preview.backup.contacts.length && !preview.backup.quotes.length)"
            @click="preview ? transfer() : prepare()">
            <Icon :name="busy ? 'loader' : 'upload'" :size="15" :class="{ spin: busy }" />
            {{ busy ? "İşleniyor…" : preview ? "Aktarımı onayla" : "Önizle" }}
          </button>
        </template>
      </ModalShell>
    </div>
  </Teleport>
</template>

<style scoped>
.backup-note, .backup-error { font-size: 13px; line-height: 1.55; margin: 0 0 12px; color: var(--c-mid); }
.backup-error { color: var(--red); }
.source { display: grid; gap: 8px; margin-bottom: 12px; font-size: 13px; }
.source input { padding: 9px 12px; border: 1px solid var(--c-border); border-radius: 8px; background: var(--c-card); color: var(--c-text); }
.source-picker, .counts > div { display: flex; justify-content: space-between; align-items: center; gap: 8px; }
.source-picker { flex-wrap: wrap; overflow-wrap: anywhere; }
.counts { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 8px 14px; font-size: 13px; }
.counts > div { border-bottom: 1px solid var(--c-border); padding-bottom: 5px; }
.counts dd { margin: 0; font-weight: 600; }
</style>
