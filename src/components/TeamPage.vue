<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { disconnectTeam, teamClient, teamSession, teamSessionNotice, teamSigningOut, teamUser, teamWorkspace } from "../teamConnection";
import SeoCard from "./SeoCard.vue";

type Workspace = { id: string; name: string };
type Membership = { workspace_id: string; role: "owner" | "editor" | "viewer" };
type Contact = { id: string; name: string; email: string; revision: number };
type Quote = { id: string; no: string; currency: string; status: string; revision: number; contact_id: string | null };
type QuoteLine = { sku?: string | null; name: string; qty: number; unit_price: number; tax_rate: number; cost: number | "" | null };
type QuoteVersion = { version: number; at: string; snapshot_json: { items: QuoteLine[]; quote: Quote } };

const workspaces = ref<Workspace[]>([]);
const memberships = ref<Membership[]>([]);
const workspace = teamWorkspace;
const contacts = ref<Contact[]>([]);
const quotes = ref<Quote[]>([]);
const editingContact = ref<Contact | null>(null);
const contactName = ref("");
const contactEmail = ref("");
const editingQuote = ref<Quote | null>(null);
const quoteNo = ref("");
const quoteContact = ref("");
const currency = ref("TRY");
const quoteLines = ref<QuoteLine[]>([]);
const quoteVersions = ref<QuoteVersion[]>([]);
const quoteLoading = ref(false);
const nextStatus = ref("");
const closeReason = ref("");
const transitions: Record<string, string[]> = {
  draft: ["sent"], sent: ["draft", "won", "lost", "expired"], expired: ["sent"],
};
const error = ref("");
const status = ref("");
const loadingWorkspaces = ref(false);
const loading = ref(false);
const writing = ref(false);
const live = ref(false);
const userId = computed(() => teamSession.value?.user.id ?? null);
const currentRole = computed(() => memberships.value.find((row) => row.workspace_id === workspace.value)?.role);
const roleLabels: Record<Membership["role"], string> = {
  owner: "Çalışma alanı sahibi",
  editor: "Düzenleyici",
  viewer: "Görüntüleyici",
};
const canWrite = computed(() => currentRole.value === "owner" || currentRole.value === "editor");

watch(userId, async (user) => {
  const previous = workspace.value;
  if (!teamUser.value) workspace.value = "";
  workspaces.value = [];
  memberships.value = [];
  error.value = "";
  loadingWorkspaces.value = false;
  const client = teamClient.value;
  if (!client || !user) return;
  loadingWorkspaces.value = true;
  try {
    const [spaces, access] = await Promise.all([
      client.from("workspaces").select("id,name").order("name"),
      client.from("workspace_members").select("workspace_id,role").eq("user_id", user),
    ]);
    if (userId.value !== user || teamClient.value !== client) return;
    if (spaces.error || access.error) throw new Error("Çalışma alanı bilgisi alınamadı. Pilot şemasını ve bağlantıyı kontrol edin.");
    workspaces.value = spaces.data ?? [];
    memberships.value = (access.data ?? []) as Membership[];
    workspace.value = workspaces.value.find((space) => space.id === previous)?.id ?? workspaces.value[0]?.id ?? "";
  } catch (cause) {
    if (userId.value === user && teamClient.value === client) error.value = (cause as Error).message;
  } finally {
    if (userId.value === user && teamClient.value === client) loadingWorkspaces.value = false;
  }
}, { immediate: true });

function editContact(row: Contact | null) {
  editingContact.value = row;
  contactName.value = row?.name ?? "";
  contactEmail.value = row?.email ?? "";
}

async function editQuote(row: Quote | null) {
  const client = teamClient.value;
  const space = workspace.value;
  editingQuote.value = row;
  quoteNo.value = row?.no ?? "";
  quoteContact.value = row?.contact_id ?? "";
  currency.value = row?.currency ?? "TRY";
  quoteLines.value = [];
  quoteVersions.value = [];
  nextStatus.value = "";
  closeReason.value = "";
  quoteLoading.value = !!row;
  if (!client || !row) return;
  const [lines, versions] = await Promise.all([
    client.from("team_quote_items").select("sku,name,qty,unit_price,tax_rate,cost")
      .eq("workspace_id", space).eq("quote_id", row.id).order("sort"),
    client.from("team_quote_versions").select("version,at,snapshot_json")
      .eq("workspace_id", space).eq("quote_id", row.id).order("version", { ascending: false }),
  ]);
  if (editingQuote.value !== row || workspace.value !== space || teamClient.value !== client) return;
  if (lines.error || versions.error) {
    error.value = "Teklif satırları veya geçmişi yüklenemedi. Teklifi yeniden açın.";
    return;
  }
  quoteLines.value = lines.data;
  quoteVersions.value = versions.data as QuoteVersion[];
  quoteLoading.value = false;
}

async function changeQuoteStatus() {
  const client = teamClient.value;
  const quote = editingQuote.value;
  const space = workspace.value;
  if (!client || !quote || !canWrite.value || !nextStatus.value || writing.value) return;
  writing.value = true;
  error.value = "";
  status.value = "";
  try {
    const { error: transitionError } = await client.rpc("team_transition_quote", {
      target_workspace: space, target_quote: quote.id, expected_revision: quote.revision,
      next_status: nextStatus.value, reason: closeReason.value.trim(),
    });
    if (workspace.value !== space || teamClient.value !== client) return;
    if (transitionError) {
      error.value = transitionError.code === "40001" ? "Teklif değişmiş. Güncel kaydı yeniden açın."
        : transitionError.message.includes("must have items") ? "Gönderildi işaretlemek için teklife satır ekleyin."
        : "Teklif durumu değiştirilemedi.";
      await loadRecords?.();
      return;
    }
    status.value = "Teklif durumu güncellendi.";
    await loadRecords?.();
    await editQuote(quotes.value.find((row) => row.id === quote.id) ?? null);
  } finally {
    writing.value = false;
  }
}

let loadRecords: (() => Promise<void>) | undefined;
watch(workspace, (id, _, onCleanup) => {
  editContact(null);
  editQuote(null);
  contacts.value = [];
  quotes.value = [];
  loadRecords = undefined;
  live.value = false;
  loading.value = false;
  status.value = "";
  const client = teamClient.value;
  if (!client || !id) return;
  error.value = "";
  let active = true;
  let generation = 0;
  const refresh = async () => {
    const request = ++generation;
    const [people, offers] = await Promise.all([
      client.from("team_contacts").select("id,name,email,revision").eq("workspace_id", id).eq("archived", false).order("name"),
      client.from("team_quotes").select("id,no,currency,status,revision,contact_id").eq("workspace_id", id).order("updated_at", { ascending: false }),
    ]);
    if (!active || request !== generation) return;
    loading.value = false;
    if (people.error || offers.error) {
      error.value = "Ortak kayıtlar alınamadı. Bağlantıyı ve erişim yetkinizi kontrol edin.";
      return;
    }
    contacts.value = people.data;
    quotes.value = offers.data;
  };
  loading.value = true;
  loadRecords = refresh;
  void refresh();
  const filter = `workspace_id=eq.${id}`;
  const channel = client.channel(`team-records:${id}`)
    .on("postgres_changes", { event: "*", schema: "public", table: "team_contacts", filter }, () => { void refresh(); })
    .on("postgres_changes", { event: "*", schema: "public", table: "team_quotes", filter }, () => { void refresh(); })
    .subscribe((state) => {
      if (!active) return;
      live.value = state === "SUBSCRIBED";
      if (live.value) void refresh();
    });
  onCleanup(() => {
    active = false;
    void client.removeChannel(channel);
  });
}, { immediate: true });

async function saveRecord(table: "team_contacts" | "team_quotes") {
  const client = teamClient.value;
  const id = workspace.value;
  if (!client || !canWrite.value || writing.value) return;
  const isContact = table === "team_contacts";
  const existing = isContact ? editingContact.value : editingQuote.value;
  const values = isContact
    ? { name: contactName.value.trim(), email: contactEmail.value.trim() }
    : { no: quoteNo.value.trim(), currency: currency.value, contact_id: quoteContact.value || null };
  if (isContact ? !contactName.value.trim() : !quoteNo.value.trim()) return;
  writing.value = true;
  error.value = "";
  status.value = "";
  try {
    const result = !isContact
      ? await client.rpc("team_save_quote", {
        target_workspace: id, target_quote: existing?.id ?? null, expected_revision: existing?.revision ?? null,
        quote_no: quoteNo.value.trim(), customer: quoteContact.value || null, currency_code: currency.value,
        items: quoteLines.value.map((line) => ({ ...line, cost: line.cost === "" ? null : line.cost })),
      })
      : existing
      ? await client.from(table).update({ ...values, revision: existing.revision + 1 })
        .eq("workspace_id", id).eq("id", existing.id).eq("revision", existing.revision).select("id").maybeSingle()
      : await client.from(table).insert({ ...values, workspace_id: id }).select("id").single();
    if (workspace.value !== id || teamClient.value !== client) return;
    if (result.error?.code === "23505") throw new Error("Bu teklif numarası zaten kullanılıyor.");
    if (result.error?.code === "40001" || (!result.error && !result.data)) {
      await loadRecords?.();
      throw new Error("Kayıt değişmiş veya düzenlemeye kapanmış. Güncel kaydı listeden tekrar seçin.");
    }
    if (result.error) throw new Error("Kayıt kaydedilemedi. Girdiğiniz bilgileri ve düzenleme yetkinizi kontrol edin.");
    if (isContact) editContact(null);
    else void editQuote(null);
    status.value = isContact ? "Müşteri kaydedildi." : "Teklif taslağı kaydedildi.";
    await loadRecords?.();
  } catch (cause) {
    if (workspace.value === id && teamClient.value === client) error.value = (cause as Error).message;
  } finally {
    writing.value = false;
  }
}

const quoteStatus: Record<string, string> = {
  draft: "Taslak", sent: "Gönderildi", won: "Kazanıldı", lost: "Kaybedildi", expired: "Süresi doldu",
};
</script>

<template>
  <section class="team-page om-scroll">
    <div class="team-content">
      <SeoCard v-if="!teamSession" icon="users" title="Ekip oturumu" sub="Çevrimdışı">
        <p class="team-state" role="status">{{ teamSessionNotice || "Bağlantı kurulduğunda ekip oturumu yeniden doğrulanır." }}</p>
        <button class="secondary-action" type="button" :disabled="teamSigningOut" @click="disconnectTeam()">Oturumu kapat</button>
      </SeoCard>

      <template v-else>
        <p v-if="teamSessionNotice" class="team-error" role="alert">{{ teamSessionNotice }}</p>
        <div class="team-toolbar">
          <label v-if="workspaces.length">
            Çalışma alanı
            <select v-model="workspace" class="fx" :disabled="writing">
              <option v-for="space in workspaces" :key="space.id" :value="space.id">{{ space.name }}</option>
            </select>
          </label>
          <span v-if="currentRole" class="team-state">
            {{ teamSession?.user.email ? `${teamSession.user.email} · ` : "" }}{{ roleLabels[currentRole] }}
          </span>
          <span v-if="workspace" class="team-state" :class="{ live }">{{ live ? "Anlık güncelleme açık" : "Anlık bağlantı bekleniyor" }}</span>
          <button v-if="workspace" class="secondary-action" @click="loadRecords?.()">Yenile</button>
          <button class="secondary-action" :disabled="writing || teamSigningOut" @click="disconnectTeam()">Oturumu kapat</button>
        </div>
        <p v-if="loadingWorkspaces || loading" class="team-state" role="status">Yükleniyor…</p>
        <p v-else-if="!workspace && !error" class="team-state">Bu hesabın çalışma alanı üyeliği yok.</p>
        <p v-if="error" class="team-error" role="alert">{{ error }}</p>
        <p v-if="status" class="team-state live" role="status">{{ status }}</p>

        <div v-if="workspace" class="team-grid">
          <SeoCard icon="users" title="Ortak müşteriler">
            <ul class="team-list">
              <li v-for="row in contacts" :key="row.id">
                <div><strong>{{ row.name }}</strong><small v-if="row.email">{{ row.email }}</small></div>
                <button v-if="canWrite" class="secondary-action" :disabled="writing" @click="editContact(row)">Düzenle</button>
              </li>
            </ul>
            <p v-if="!loading && !contacts.length" class="team-state">Henüz müşteri kaydı yok.</p>
            <form v-if="canWrite" class="team-form" @submit.prevent="saveRecord('team_contacts')">
              <label>Müşteri adı<input v-model="contactName" class="fx" required /></label>
              <label>E-posta<input v-model="contactEmail" class="fx" type="email" /></label>
              <div class="team-actions">
                <button class="primary-action" :disabled="writing">{{ editingContact ? "Değişiklikleri kaydet" : "Müşteri ekle" }}</button>
                <button v-if="editingContact" type="button" class="secondary-action" :disabled="writing" @click="editContact(null)">Vazgeç</button>
              </div>
            </form>
          </SeoCard>
          <SeoCard icon="fileEdit" title="Ortak teklifler">
            <ul class="team-list">
              <li v-for="row in quotes" :key="row.id">
                <div><strong>{{ row.no }}</strong><small>{{ quoteStatus[row.status] ?? row.status }} · {{ row.currency }}</small></div>
                <button class="secondary-action" :disabled="writing" @click="editQuote(row)">Teklifi aç</button>
              </li>
            </ul>
            <p v-if="!loading && !quotes.length" class="team-state">Henüz teklif kaydı yok.</p>
            <p v-if="quoteLoading" class="team-state">Teklif ayrıntıları yükleniyor…</p>
            <form v-if="canWrite && (!editingQuote || editingQuote.status === 'draft') && !quoteLoading" class="team-form" @submit.prevent="saveRecord('team_quotes')">
              <label>Teklif numarası<input v-model="quoteNo" class="fx" required /></label>
              <label>Müşteri
                <select v-model="quoteContact" class="fx">
                  <option value="">Müşteri seçilmedi</option>
                  <option v-if="quoteContact && !contacts.some((row) => row.id === quoteContact)" :value="quoteContact">Arşivlenmiş müşteri</option>
                  <option v-for="row in contacts" :key="row.id" :value="row.id">{{ row.name }}</option>
                </select>
              </label>
              <label>Para birimi
                <select v-model="currency" class="fx"><option value="TRY">TL</option><option value="USD">USD</option></select>
              </label>
              <div v-for="(line, index) in quoteLines" :key="index" class="quote-line">
                <label>Satır adı<input v-model="line.name" class="fx" required /></label>
                <div class="quote-numbers">
                  <label>Adet<input v-model.number="line.qty" class="fx" type="number" min="0" step="any" required /></label>
                  <label>Birim fiyat<input v-model.number="line.unit_price" class="fx" type="number" min="0" step="any" required /></label>
                  <label>KDV (%)<input v-model.number="line.tax_rate" class="fx" type="number" min="0" max="100" step="any" required /></label>
                </div>
                <label>Maliyet (isteğe bağlı)<input v-model.number="line.cost" class="fx" type="number" min="0" step="any" /></label>
                <button type="button" class="secondary-action" :disabled="writing" @click="quoteLines.splice(index, 1)">Satırı kaldır</button>
              </div>
              <button type="button" class="secondary-action" :disabled="writing || quoteLines.length >= 500"
                @click="quoteLines.push({ name: '', qty: 1, unit_price: 0, tax_rate: 0, cost: null })">Satır ekle</button>
              <div class="team-actions">
                <button class="primary-action" :disabled="writing">{{ editingQuote ? "Değişiklikleri kaydet" : "Teklif taslağı ekle" }}</button>
                <button v-if="editingQuote" type="button" class="secondary-action" :disabled="writing" @click="editQuote(null)">Vazgeç</button>
              </div>
            </form>
            <div v-if="editingQuote && !quoteLoading" class="quote-workflow">
              <template v-if="editingQuote.status !== 'draft' || !canWrite">
                <strong>{{ editingQuote.no }}</strong>
                <ul class="team-list">
                  <li v-for="(line, index) in quoteLines" :key="index">
                    {{ line.name }} · {{ line.qty }} × {{ line.unit_price }} {{ editingQuote.currency }}
                  </li>
                </ul>
              </template>
              <form v-if="canWrite && transitions[editingQuote.status]?.length" class="team-form" @submit.prevent="changeQuoteStatus">
                <label>Yeni durum
                  <select v-model="nextStatus" class="fx" required>
                    <option value="">Durum seçin</option>
                    <option v-for="option in transitions[editingQuote.status]" :key="option" :value="option">{{ quoteStatus[option] }}</option>
                  </select>
                </label>
                <label v-if="['won', 'lost', 'expired'].includes(nextStatus)">Sonuç notu<input v-model="closeReason" class="fx" /></label>
                <p v-if="nextStatus === 'sent'" class="team-state">Teklifin kaydedilmiş hâli sürüm geçmişine eklenir. Müşteriye e-posta gönderilmez.</p>
                <button class="secondary-action" :disabled="!nextStatus || writing">Durumu kaydet</button>
              </form>
              <details v-for="version in quoteVersions" :key="version.version" class="quote-version">
                <summary>Sürüm {{ version.version }} · {{ new Date(version.at).toLocaleString('tr-TR') }}</summary>
                <ul class="team-list">
                  <li v-for="(line, index) in version.snapshot_json.items" :key="index">
                    {{ line.name }} · {{ line.qty }} × {{ line.unit_price }} {{ version.snapshot_json.quote.currency }}
                  </li>
                </ul>
              </details>
              <button v-if="editingQuote.status !== 'draft' || !canWrite" class="secondary-action" :disabled="writing" @click="editQuote(null)">Teklifi kapat</button>
            </div>
          </SeoCard>
        </div>
      </template>
    </div>
  </section>
</template>

<style scoped>
.team-page { flex: 1; min-height: 0; overflow: auto; }
.team-content { max-width: 1000px; margin: 0 auto; padding: 12px 24px 36px; }
.team-content > .card { max-width: 460px; }
.team-toolbar, .team-actions { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
.team-toolbar label { display: flex; gap: 8px; align-items: center; margin-right: auto; }
.team-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 330px), 1fr)); gap: 16px; align-items: start; }
.team-form { display: grid; gap: 11px; }
.team-form label { display: grid; gap: 5px; }
.team-toolbar label, .team-form label { font-size: 13px; color: var(--c-mid); }
.team-form .primary-action { justify-self: start; }
.fx { min-width: 0; padding: 9px 12px; border: 1px solid var(--c-border); border-radius: 8px; background: var(--c-card); color: var(--c-text); }
.team-list { padding: 0; margin: 0 0 12px; list-style: none; }
.team-list li { display: flex; justify-content: space-between; align-items: center; gap: 8px; padding: 8px 0; border-bottom: 1px solid var(--c-border); overflow-wrap: anywhere; }
.team-list small { display: block; margin-top: 3px; color: var(--c-soft); }
.team-state, .team-error { font-size: 13px; color: var(--c-soft); }
.team-state.live { color: var(--green); }
.team-error { color: var(--red); }
.quote-line, .quote-workflow, .quote-version {
  display: grid; gap: 10px; padding: 12px; margin-top: 8px;
  border: 1px solid var(--c-border); border-radius: 8px;
}
.quote-numbers { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; }
.quote-version summary { cursor: pointer; }
</style>
