<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import {
  disconnectTeam, teamAccessIssue, teamClient, teamProject, teamReachable,
  teamSession, teamSessionNotice, teamSigningOut, teamUser, teamWorkspace,
} from "../teamConnection";
import {
  avatarImageSource, prepareTeamAvatar, refreshTeamAvatars, saveTeamAvatar, teamAvatarAccept,
  teamAvatarAvailable, teamAvatarOnline, teamAvatarSaving, teamAvatarSources,
} from "../teamProfile";
import Icon from "./Icon.vue";
import SeoCard from "./SeoCard.vue";
import TeamAvatar from "./TeamAvatar.vue";

type Role = "owner" | "editor" | "viewer";
type Workspace = { id: string; name: string; role: Role };
const roleLabels: Record<Role, string> = {
  owner: "Çalışma alanı sahibi",
  editor: "Düzenleyici",
  viewer: "Görüntüleyici",
};

const workspaces = ref<Workspace[]>([]);
const workspace = computed(() => workspaces.value.find((space) => space.id === teamWorkspace.value));
const activeUser = computed(() => teamSession.value?.user.id);
const loading = ref(false);
const error = ref("");
const retry = ref(0);
const offline = computed(() => teamReachable.value === false || !teamSession.value);
const avatarFile = ref<HTMLInputElement | null>(null);
const avatarTrigger = ref<HTMLButtonElement | null>(null);
const avatarSave = ref<HTMLButtonElement | null>(null);
const pendingAvatar = ref<string | null>();
const avatarPreparing = ref(false);
const avatarError = ref("");
const avatarNotice = ref("");
let avatarSelection = 0;
const accountName = computed(() => {
  const user = teamSession.value?.user;
  const name = [user?.user_metadata.display_name, user?.user_metadata.full_name]
    .find((value): value is string => typeof value === "string" && !!value.trim());
  return name?.trim() || user?.email?.split("@")[0] || "Üye";
});
const savedAvatar = computed(() => teamAvatarSources.value.get(teamUser.value));
const shownAvatar = computed(() => pendingAvatar.value === undefined ? savedAvatar.value : avatarImageSource(pendingAvatar.value));
const avatarDisabled = computed(() => !teamAvatarOnline.value || avatarPreparing.value || teamAvatarSaving.value || teamSigningOut.value);

watch([teamProject, teamUser, teamWorkspace, teamClient, teamAccessIssue], () => {
  ++avatarSelection;
  pendingAvatar.value = undefined;
  avatarPreparing.value = false;
  avatarError.value = "";
  avatarNotice.value = "";
}, { flush: "sync" });
watch([teamAvatarOnline, teamProject, teamUser, teamWorkspace, teamClient], ([online]) => {
  if (online) void refreshTeamAvatars();
}, { immediate: true });

async function stageAvatar(event: Event) {
  const input = event.currentTarget as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file || avatarDisabled.value) return;
  const current = ++avatarSelection;
  avatarPreparing.value = true;
  avatarError.value = "";
  avatarNotice.value = "";
  try {
    const image = await prepareTeamAvatar(file);
    if (current !== avatarSelection) return;
    pendingAvatar.value = image;
  } catch (error) {
    if (current === avatarSelection) avatarError.value = error instanceof Error ? error.message : "Fotoğraf hazırlanamadı.";
  } finally {
    if (current === avatarSelection) avatarPreparing.value = false;
  }
  await nextTick();
  if (current === avatarSelection) avatarSave.value?.focus();
}

async function stageAvatarRemoval() {
  if (avatarDisabled.value) return;
  avatarError.value = "";
  avatarNotice.value = "";
  pendingAvatar.value = null;
  await nextTick();
  avatarSave.value?.focus();
}

async function cancelAvatar() {
  const current = avatarSelection;
  pendingAvatar.value = undefined;
  avatarError.value = "";
  await nextTick();
  if (current === avatarSelection) avatarTrigger.value?.focus();
}

async function commitAvatar() {
  const image = pendingAvatar.value;
  if (image === undefined || avatarDisabled.value) return;
  const current = avatarSelection;
  avatarError.value = "";
  try {
    if (!await saveTeamAvatar(image) || current !== avatarSelection) return;
    await cancelAvatar();
    if (current === avatarSelection) avatarNotice.value = image === null ? "Profil fotoğrafı kaldırıldı." : "Profil fotoğrafı güncellendi.";
  } catch (error) {
    if (current === avatarSelection) avatarError.value = error instanceof Error ? error.message : "Profil fotoğrafı kaydedilemedi.";
  }
}

const connection = computed(() => {
  if (offline.value) return { label: "Çevrimdışı", style: { background: "var(--c-chip)", color: "var(--c-mid)" } };
  if (teamReachable.value === true) return { label: "Çevrimiçi", style: { background: "var(--ok-soft-bg)", color: "var(--green)" } };
  return { label: "Bağlantı denetleniyor", style: { background: "var(--c-chip)", color: "var(--c-mid)" } };
});

watch([teamProject, teamUser], () => { workspaces.value = []; }, { flush: "sync" });

watch([teamClient, activeUser, teamProject, teamReachable, teamAccessIssue, retry], async ([client, user, project, reachable, denied], _, onCleanup) => {
  error.value = "";
  loading.value = false;
  if (!client || !user || reachable === false || denied) return;
  const request = new AbortController();
  onCleanup(() => request.abort());
  const current = () => !request.signal.aborted && activeUser.value === user && teamProject.value === project
    && teamClient.value === client && !teamAccessIssue.value && teamReachable.value !== false;
  loading.value = true;
  try {
    const [spaces, memberships] = await Promise.all([
      client.from("workspaces").select("id,name").order("name").abortSignal(request.signal).retry(false),
      client.from("workspace_members").select("workspace_id,role").eq("user_id", user).abortSignal(request.signal).retry(false),
    ]);
    if (!current()) return;
    if (spaces.error || memberships.error) throw new Error("Çalışma alanı bilgisi alınamadı.");
    const roles = new Map(memberships.data.map(({ workspace_id, role }) => [workspace_id, role as Role]));
    workspaces.value = spaces.data.flatMap((space) => {
      const role = roles.get(space.id);
      return role ? [{ ...space, role }] : [];
    });
    if (!workspace.value) teamWorkspace.value = workspaces.value[0]?.id ?? "";
  } catch {
    if (current()) error.value = "Çalışma alanı bilgisi alınamadı. Tekrar deneyin.";
  } finally {
    if (current()) loading.value = false;
  }
}, { immediate: true });
</script>

<template>
  <section class="team-page om-scroll">
    <div class="team-content">
      <SeoCard icon="users" title="Ekip hesabı" :badge-label="connection.label" :badge-style="connection.style" stack>
        <div class="profile-row">
          <TeamAvatar class="profile-avatar" :name="accountName" :source="shownAvatar" />
          <div class="profile-controls">
            <strong>Profil fotoğrafı</strong>
            <span v-if="pendingAvatar !== undefined" class="profile-preview">{{ pendingAvatar === null ? "Fotoğraf kaldırılacak" : "Önizleme" }}</span>
            <div v-if="pendingAvatar !== undefined" class="profile-actions">
              <button ref="avatarSave" class="primary-action" type="button" :disabled="avatarDisabled" @click="commitAvatar">
                {{ teamAvatarSaving ? "Kaydediliyor…" : "Kaydet" }}
              </button>
              <button class="secondary-action" type="button" :disabled="teamAvatarSaving" @click="cancelAvatar">Vazgeç</button>
            </div>
            <div v-else-if="teamAvatarAvailable" class="profile-actions">
              <input ref="avatarFile" type="file" :accept="teamAvatarAccept" hidden @change="stageAvatar" />
              <button
                ref="avatarTrigger"
                class="secondary-action"
                type="button"
                :disabled="avatarDisabled"
                :title="!teamAvatarOnline ? 'Fotoğrafı değiştirmek için bağlantı gerekli' : undefined"
                @click="avatarFile?.click()"
              >
                <Icon :name="avatarPreparing ? 'loader' : 'image'" :class="{ spin: avatarPreparing }" :size="14" aria-hidden="true" />
                {{ avatarPreparing ? "Fotoğraf hazırlanıyor…" : savedAvatar ? "Fotoğrafı değiştir" : "Fotoğraf seç" }}
              </button>
              <button v-if="savedAvatar" class="secondary-action" type="button" :disabled="avatarDisabled" @click="stageAvatarRemoval">Kaldır</button>
            </div>
          </div>
        </div>
        <p v-if="avatarError" role="alert" class="account-note account-error"><Icon name="alert" :size="14" aria-hidden="true" /> {{ avatarError }}</p>
        <p v-if="avatarNotice" role="status" class="account-note"><Icon name="check" :size="14" aria-hidden="true" /> {{ avatarNotice }}</p>
        <dl v-if="teamSession?.user.email || workspace" class="account-details">
          <div v-if="teamSession?.user.email">
            <dt>Hesap</dt>
            <dd>{{ teamSession.user.email }}</dd>
          </div>
          <div v-if="workspace">
            <dt>
              <label v-if="workspaces.length > 1" for="team-workspace">Çalışma alanı</label>
              <template v-else>Çalışma alanı</template>
            </dt>
            <dd>
              <select
                v-if="workspaces.length > 1"
                id="team-workspace"
                v-model="teamWorkspace"
                class="fx workspace-select"
                :disabled="offline || loading || !!error || teamSigningOut"
              >
                <option v-for="space in workspaces" :key="space.id" :value="space.id">{{ space.name }}</option>
              </select>
              <template v-else>{{ workspace.name }}</template>
            </dd>
          </div>
          <div v-if="workspace">
            <dt>Yetki</dt>
            <dd>{{ roleLabels[workspace.role] }}</dd>
          </div>
        </dl>

        <p v-if="loading" class="account-note" role="status">
          <Icon name="loader" class="spin" :size="14" aria-hidden="true" /> Çalışma alanı yükleniyor…
        </p>
        <p v-if="error" class="account-note account-error" role="alert">
          <Icon name="alert" :size="14" aria-hidden="true" /> {{ error }}
        </p>
        <p v-if="teamSessionNotice || offline" class="account-note" role="status">
          {{ teamSessionNotice || "Bağlantı geri geldiğinde ekip oturumu kendiliğinden doğrulanır." }}
        </p>

        <template #actions>
          <button v-if="error" class="secondary-action" type="button" :disabled="loading || teamSigningOut" @click="retry++">
            <Icon name="refresh" :size="14" aria-hidden="true" /> Tekrar dene
          </button>
          <button class="secondary-action" type="button" :disabled="teamSigningOut" @click="disconnectTeam()">
            {{ teamSigningOut ? "Oturum kapatılıyor…" : "Oturumu kapat" }}
          </button>
        </template>
      </SeoCard>
    </div>
  </section>
</template>

<style scoped>
.team-page { flex: 1; min-height: 0; overflow: auto; }
.team-content { max-width: 676px; margin: 0 auto; padding: 8px 24px 36px; }
.profile-row { display: flex; align-items: flex-start; gap: 13px; padding-bottom: 13px; border-bottom: 1px solid var(--c-border-soft); }
.profile-avatar { --team-avatar-size: 54px; --team-avatar-radius: 13px; --team-avatar-font: 17px; }
.profile-controls { display: grid; min-width: 0; gap: 5px; align-content: start; }
.profile-controls strong { color: var(--c-text); font-size: 13px; font-weight: 620; }
.profile-preview { color: var(--c-mid); font-size: 11.5px; }
.profile-actions { display: flex; flex-wrap: wrap; gap: 6px; }
.profile-actions button { height: 32px; padding-inline: 10px; }
.profile-actions button:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.account-details { margin: 0; }
.account-details > div {
  display: grid;
  grid-template-columns: minmax(102px, 24%) minmax(0, 1fr);
  align-items: center;
  gap: 12px;
  padding: 11px 0;
  line-height: 1.4;
}
.account-details > div:first-child { padding-top: 2px; }
.account-details > div:last-child { padding-bottom: 2px; }
.account-details > div + div { border-top: 1px solid var(--c-border-soft); }
.account-details dt { color: var(--c-mid); font-size: 12px; }
.account-details dd { margin: 0; color: var(--c-text); font-size: 13px; font-weight: 580; overflow-wrap: anywhere; }
.workspace-select {
  width: 100%;
  max-width: 315px;
  padding: 8px 10px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-input);
  color: var(--c-text);
}
.account-note { display: flex; align-items: center; gap: 7px; margin: 0; color: var(--c-mid); font-size: 12px; line-height: 1.5; }
.account-note svg { flex: none; }
.account-error { color: var(--badge-eksik-c); }
</style>
