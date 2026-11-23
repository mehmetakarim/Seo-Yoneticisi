import type { Session, SupabaseClient } from "@supabase/supabase-js";
import { invoke } from "@tauri-apps/api/core";
import { computed, shallowRef, watch } from "vue";

const projectUrl = "https://iwtygnaqgjcmnmygjequ.supabase.co";
const publishableKey = "sb_publishable_vOnGiFk-BpzqdFKjYgFbtg_Dg3CW3H9";

export const teamClient = shallowRef<SupabaseClient | null>(null);
export const teamSession = shallowRef<Session | null>(null);
export const teamWorkspace = shallowRef("");
export const teamSessionNotice = shallowRef("");
export const teamRestoring = shallowRef(false);
export const teamWorkspaceLoading = shallowRef(false);
export const teamWorkspaceError = shallowRef("");
export const teamSigningOut = shallowRef(false);
export const teamProject = shallowRef("");
export const teamReachable = shallowRef<boolean | null>(null);
export const teamAccessIssue = shallowRef(false);
export const teamServiceStatus = shallowRef<number | null>(null);
export const teamConnectionRetry = shallowRef(0);
export const teamCheckedAt = shallowRef<Date | null>(null);
let networkGeneration = 0;
const offlineUser = shallowRef("");
export const teamUser = computed(() => teamSession.value?.user.id || offlineUser.value);
let unsubscribe: (() => void) | undefined;
let storageWrite: Promise<unknown> = Promise.resolve();
let restoreAttempt: Promise<void> | undefined;
let workspaceAttempt: Promise<void> | undefined;

type SavedSession = {
  url: string; key: string; accessToken: string; refreshToken: string;
  user?: string; workspace?: string;
};
let activeConnection: Pick<SavedSession, "url" | "key"> | null = null;
const profileKey = "seo-team-session-profile";

export type TeamFunctionSession = { project: string; workspace: string; publishable_key: string; access_token: string };

export function getTeamFunctionSession(): TeamFunctionSession {
  const connection = activeConnection;
  const session = teamSession.value;
  const workspace = teamWorkspace.value;
  if (!connection || !session || !workspace || teamAccessIssue.value) throw new Error("Ekip oturumunu ve çalışma alanını kontrol edin.");
  if (teamReachable.value === false || !navigator.onLine) throw new Error("Ortak Gemini üretimi için Supabase bağlantısı gerekli.");
  return { project: connection.url, workspace, publishable_key: connection.key, access_token: session.access_token };
}

function sessionProfile(): string {
  const existing = localStorage.getItem(profileKey);
  if (existing && /^[\da-f-]{36}$/i.test(existing)) return existing;
  const id = crypto.randomUUID();
  localStorage.setItem(profileKey, id);
  return id;
}

function saveSession(saved: SavedSession | null): Promise<boolean> {
  const write = storageWrite.catch(() => {}).then(() => invoke<boolean>("write_team_session", {
    profile: sessionProfile(), value: saved ? JSON.stringify(saved) : null,
  }));
  storageWrite = write;
  return write;
}

async function rememberSession(current: Session, url: string, key: string) {
  if (teamSigningOut.value) return false;
  const saved = await saveSession({
    url, key, accessToken: current.access_token, refreshToken: current.refresh_token,
    user: current.user.id, workspace: teamWorkspace.value,
  }).catch(() => false);
  if (!saved && teamSession.value?.user.id === current.user.id && activeConnection?.url === url) {
    teamSessionNotice.value = "Oturum açıldı. Bu cihazda güvenli oturum kaydı yapılamadı.";
  }
  return saved;
}

watch(teamWorkspace, () => {
  if (teamSession.value && activeConnection) {
    void rememberSession(teamSession.value, activeConnection.url, activeConnection.key);
  }
});

async function makeClient(url: string, key: string) {
  const { createClient } = await import("@supabase/supabase-js").catch(() => {
    throw new Error("Ekip giriş bileşeni yüklenemedi. Uygulamayı yeniden açın.");
  });
  const client = createClient(url, key, {
    auth: { persistSession: false, detectSessionInUrl: false },
    global: {
      fetch(input, init) {
        const target = input instanceof Request ? input.url : String(input);
        const edgeBase = `${url.replace(/\/$/, "")}/functions/v1`;
        const wait = target === `${edgeBase}/team-ideasoft` ? 50_000 : target === `${edgeBase}/team-gemini` ? 35_000 : 8_000;
        const deadline = AbortSignal.timeout(wait);
        const caller = init?.signal ?? (input instanceof Request ? input.signal : null);
        return fetch(input, { ...init, signal: caller ? AbortSignal.any([caller, deadline]) : deadline });
      },
    },
  });
  const initialization = await client.auth.initialize();
  if (initialization.error) {
    await client.auth.stopAutoRefresh();
    throw new Error("Supabase oturum bileşeni başlatılamadı. Ekip ekranını yeniden açıp tekrar deneyin.");
  }
  return client;
}

export function markTeamDisconnected() {
  networkGeneration++;
  teamReachable.value = false;
  teamServiceStatus.value = null;
}

export function resetTeamConnectionStatus() {
  networkGeneration++;
  teamReachable.value = null;
  teamAccessIssue.value = false;
  teamServiceStatus.value = null;
  teamCheckedAt.value = null;
}

export async function probeTeamConnection(): Promise<boolean> {
  const client = teamClient.value;
  const workspace = teamWorkspace.value;
  const generation = networkGeneration;
  if (!client || !teamSession.value || !workspace) return false;
  const response = await client.from("workspaces").select("id").eq("id", workspace).maybeSingle().retry(false);
  if (generation !== networkGeneration || client !== teamClient.value
    || workspace !== teamWorkspace.value || !teamSession.value) return false;
  // An authorization failure must not switch reads to a previously authorized copy.
  teamReachable.value = response.status !== 0 && response.status < 500;
  if (teamReachable.value) teamAccessIssue.value = !!response.error || !response.data;
  teamServiceStatus.value = response.status >= 500 ? response.status : null;
  teamCheckedAt.value = new Date();
  return teamReachable.value;
}

export function resolveTeamWorkspace(): Promise<void> {
  const client = teamClient.value;
  if (!client || !teamSession.value || teamWorkspace.value) return Promise.resolve();
  if (workspaceAttempt) return workspaceAttempt;
  teamWorkspaceLoading.value = true;
  teamWorkspaceError.value = "";
  workspaceAttempt = (async () => {
    try {
      const { data, error } = await client.from("workspaces").select("id").order("name").limit(1).maybeSingle().retry(false);
      if (teamClient.value !== client || !teamSession.value || teamWorkspace.value) return;
      if (error) throw error;
      if (data) teamWorkspace.value = data.id;
      else teamWorkspaceError.value = "Bu hesabın bir çalışma alanı üyeliği yok. Ekip yöneticinizle iletişime geçin.";
    } catch {
      if (teamClient.value === client) teamWorkspaceError.value = "Çalışma alanı yüklenemedi. Bağlantınızı kontrol edip tekrar deneyin.";
    } finally {
      teamWorkspaceLoading.value = false;
      workspaceAttempt = undefined;
    }
  })();
  return workspaceAttempt;
}

function clearTeamScope() {
  unsubscribe?.();
  unsubscribe = undefined;
  teamClient.value = null;
  teamSession.value = null;
  offlineUser.value = "";
  activeConnection = null;
  teamWorkspace.value = "";
  teamProject.value = "";
  teamWorkspaceError.value = "";
  resetTeamConnectionStatus();
}

async function activateSession(client: SupabaseClient, session: Session, url: string, key: string) {
  const remember = (current: Session) => rememberSession(current, url, key);
  const previous = teamClient.value;
  unsubscribe?.();
  activeConnection = { url, key };
  teamProject.value = url;
  offlineUser.value = "";
  teamClient.value = client;
  teamSession.value = session;
  const { data: listener } = client.auth.onAuthStateChange((event, current) => {
    if (teamClient.value !== client || event === "INITIAL_SESSION") return;
    if (current) {
      const rotated = current.refresh_token !== teamSession.value?.refresh_token;
      teamSession.value = current;
      if (rotated || event === "TOKEN_REFRESHED") void remember(current);
    } else if (event === "SIGNED_OUT" && !teamSigningOut.value) {
      clearTeamScope();
      void client.removeAllChannels();
      teamSessionNotice.value = "Supabase oturumu sona erdi. Devam etmek için yeniden giriş yapın.";
      void saveSession(null).catch(() => {});
    }
  });
  unsubscribe = () => listener.subscription.unsubscribe();
  if (previous && previous !== client) {
    void previous.auth.stopAutoRefresh();
    void previous.removeAllChannels();
  }
  await Promise.all([remember(session), resolveTeamWorkspace()]);
}

export function restoreTeamSession(): Promise<void> {
  if (teamSession.value || teamSigningOut.value) return Promise.resolve();
  if (restoreAttempt) return restoreAttempt;
  teamRestoring.value = true;
  restoreAttempt = (async () => {
    let client: SupabaseClient | undefined;
    try {
      await storageWrite.catch(() => {});
      const stored = await invoke<string | null>("read_team_session", { profile: sessionProfile() });
      if (!stored) return;
      const saved = JSON.parse(stored) as SavedSession;
      if (saved.url !== projectUrl || !saved.accessToken || !saved.refreshToken) return;
      const useOfflineScope = () => {
        if (!saved.user) return;
        teamProject.value = projectUrl;
        teamWorkspace.value = saved.workspace ?? "";
        offlineUser.value = saved.user;
        if (!saved.workspace) teamWorkspaceError.value = "Çalışma alanını açmak için internet bağlantısı gerekli.";
      };
      if (!navigator.onLine) {
        useOfflineScope();
        markTeamDisconnected();
        teamSessionNotice.value = "Kayıtlı ekip oturumu bağlantı gelince yeniden denenecek.";
        return;
      }
      client = await makeClient(projectUrl, publishableKey);
      const { data, error } = await client.auth.setSession({
        access_token: saved.accessToken, refresh_token: saved.refreshToken,
      });
      if (!error && data.session) {
        teamWorkspace.value = saved.user === data.session.user.id ? saved.workspace ?? "" : "";
        teamSessionNotice.value = "";
        teamReachable.value = null;
        await activateSession(client, data.session, projectUrl, publishableKey);
        if (teamWorkspace.value) await probeTeamConnection();
        return;
      }
      if (!error?.status || error.status === 429 || error.status >= 500
        || error.name === "AuthRetryableFetchError") {
        useOfflineScope();
        teamReachable.value = false;
        teamServiceStatus.value = error?.status && error.status >= 500 ? error.status : null;
        teamSessionNotice.value = "Kayıtlı ekip oturumu bağlantı gelince yeniden denenecek.";
        return;
      }
      clearTeamScope();
      await saveSession(null);
      teamSessionNotice.value = "Kayıtlı ekip oturumu sona erdi. Yeniden giriş yapın.";
    } catch {
      teamSessionNotice.value = "Kayıtlı ekip oturumu geri yüklenemedi. Bağlantıyı kontrol edin veya yeniden giriş yapın.";
    } finally {
      if (client && client !== teamClient.value) await client.auth.stopAutoRefresh();
      teamRestoring.value = false;
      restoreAttempt = undefined;
    }
  })();
  return restoreAttempt;
}

export async function connectTeam(email: string, password: string) {
  if (restoreAttempt) await restoreAttempt;
  if (teamSession.value) return;
  teamSessionNotice.value = "";
  if (!navigator.onLine) throw new Error("Oturum açmak için internet bağlantısı gerekli.");
  const client = await makeClient(projectUrl, publishableKey);
  const { data, error } = await client.auth.signInWithPassword({ email: email.trim(), password });
  if (error || !data.session) {
    await client.auth.stopAutoRefresh();
    if (error?.code === "invalid_credentials") {
      throw new Error("E-posta adresi veya parola hatalı.");
    }
    if (error?.code === "email_not_confirmed") {
      throw new Error("Hesabınızın e-posta adresi henüz doğrulanmamış. Ekip yöneticinizle iletişime geçin.");
    }
    if (error?.code === "user_banned") {
      throw new Error("Hesabınız devre dışı. Ekip yöneticinizle iletişime geçin.");
    }
    if (error?.status === 429) {
      throw new Error("Çok fazla giriş denemesi yapıldı. Bir süre bekleyip tekrar deneyin.");
    }
    if (error?.status === 401 || error?.status === 403) {
      throw new Error("Ekip giriş ayarları doğrulanamadı. Ekip yöneticinizle iletişime geçin.");
    }
    if (error?.name === "AuthRetryableFetchError" || (error?.status ?? 0) >= 500) {
      throw new Error("Supabase giriş servisine şu anda ulaşılamıyor. Ağ bağlantısını ve proje durumunu kontrol edin.");
    }
    throw new Error(`Supabase oturum açamadı${error?.code ? ` (${error.code})` : ""}.`);
  }
  const workspace = teamUser.value === data.session.user.id ? teamWorkspace.value : "";
  teamWorkspace.value = workspace;
  await activateSession(client, data.session, projectUrl, publishableKey);
  if (workspace) await probeTeamConnection();
}

export async function disconnectTeam() {
  if (teamSigningOut.value) return;
  teamSigningOut.value = true;
  try {
    await restoreAttempt;
    const client = teamClient.value;
    try {
      await saveSession(null);
    } catch {
      teamSessionNotice.value = "Bu cihazdaki oturum kaldırılamadı. Oturumu kapatmayı yeniden deneyin.";
      return;
    }
    clearTeamScope();
    teamSessionNotice.value = "";
    if (client) {
      try {
        await client.auth.stopAutoRefresh();
        await client.removeAllChannels();
        const { error } = await client.auth.signOut({ scope: "local" });
        if (error) throw error;
      } catch {
        if (!teamUser.value) teamSessionNotice.value =
          "Bu cihazdaki oturum kapatıldı. Sunucuya çıkış bildirimi gönderilemedi.";
      }
    }
  } finally {
    teamSigningOut.value = false;
  }
}
