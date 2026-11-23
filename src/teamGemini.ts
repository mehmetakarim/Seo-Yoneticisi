import { api } from "./api";
import { refreshTeamCatalog } from "./teamCatalog";
import { getTeamFunctionSession, teamAccessIssue, teamClient, teamProject, teamReachable, teamRestoring, teamSession, teamUser, teamWorkspace } from "./teamConnection";
import { checkStorePageDraft } from "./teamStorePages";
import type { StorePageDetail } from "./types";

type GeminiConnection =
  | { operation: "health" }
  | { operation: "configure"; api_key: string };

type GeminiConnectionStatus = {
  configured: boolean;
  connected?: boolean;
  models?: string[];
  version?: number;
};

function geminiWorkspace() {
  const client = teamClient.value;
  const workspace = teamWorkspace.value;
  const user = teamUser.value;
  if (!client || !workspace || !user || !teamSession.value || teamAccessIssue.value) throw new Error("Önce ekip çalışma alanını açın.");
  if (teamReachable.value === false || !navigator.onLine) throw new Error("Ortak Gemini için Supabase bağlantısı gerekiyor.");
  const current = () => client === teamClient.value && workspace === teamWorkspace.value && user === teamUser.value &&
    !!teamSession.value && !teamAccessIssue.value;
  return { client, workspace, user, current };
}

export async function teamGeminiRole(): Promise<"owner" | "editor" | "viewer" | null> {
  const { client, workspace, user, current } = geminiWorkspace();
  const { data, error } = await client.from("workspace_members").select("role")
    .eq("workspace_id", workspace).eq("user_id", user).maybeSingle().retry(false);
  const role = data?.role;
  return current() && !error && (role === "owner" || role === "editor" || role === "viewer") ? role : null;
}

async function checkGeminiConnection(operation: GeminiConnection): Promise<GeminiConnectionStatus & { models: string[] }> {
  const { client, workspace, current } = geminiWorkspace();
  const configuring = operation.operation === "configure";
  const result = await client.functions.invoke<GeminiConnectionStatus>("team-gemini", {
    body: { workspace, ...operation },
  });
  if (!current()) {
    throw new Error("Çalışma alanı veya oturum değişti.");
  }
  if (result.error) {
    const response = result.error.context;
    if (response instanceof Response) {
      if (response.status === 404) throw new Error("Ortak Gemini işlevi bu Supabase projesinde henüz yayımlanmadı.");
      if (response.status === 403) throw new Error(configuring
        ? "Ortak Gemini anahtarını çalışma alanı sahibi güncelleyebilir."
        : "Ortak Gemini bağlantısı için düzenleme yetkisi gerekiyor.");
      const info = await response.json().catch(() => null);
      if (info?.error === "gemini_credentials_rejected") throw new Error(configuring
        ? "Gemini bu anahtarı kabul etmedi. Anahtarı ve erişim izinlerini kontrol edin."
        : "Gemini sunucudaki ortak anahtarı reddetti.");
      if (info?.error === "gemini_rate_limited") throw new Error("Gemini istek sınırına ulaşıldı. Bir süre sonra tekrar deneyin.");
      if (info?.error === "gemini_no_compatible_models") throw new Error("Bu anahtarla çalışma alanının kullandığı Gemini modellerine erişilemiyor.");
      if (info?.error === "gemini_credentials_store_unavailable") throw new Error("Supabase’te ortak Gemini ayarlarına ulaşılamadı. Bir süre sonra tekrar deneyin.");
      if (configuring && response.status === 409) throw new Error("Ortak bağlantı aynı anda değişti. Anahtarı yeniden doğrulayın.");
      if (configuring && info?.error === "invalid_request") throw new Error("Ortak Gemini kurulum işlevi bu Supabase projesinde henüz etkin değil.");
    }
    throw new Error(configuring
      ? "Ortak Gemini anahtarının kaydedildiği doğrulanamadı. Ortak bağlantıyı test edin."
      : "Ortak Gemini bağlantısı doğrulanamadı. Yeniden deneyin.");
  }
  if (!result.data?.configured) throw new Error("Bu çalışma alanı için ortak Gemini bağlantısı henüz ayarlanmadı.");
  if (!result.data.connected || !Array.isArray(result.data.models)) throw new Error("Ortak Gemini bağlantısı doğrulanamadı.");
  return { ...result.data, models: result.data.models };
}

export async function testTeamGemini(): Promise<string> {
  const connection = await checkGeminiConnection({ operation: "health" });
  return `Supabase üzerinden Gemini bağlantısı doğrulandı. ${connection.models.length} model kullanılabilir.`;
}

export async function configureTeamGemini(key: string): Promise<string> {
  const apiKey = key.trim();
  if (!/^[\x21-\x7e]{20,1024}$/.test(apiKey)) throw new Error("Geçerli bir Gemini API anahtarı girin.");
  const connection = await checkGeminiConnection({ operation: "configure", api_key: apiKey });
  if (!Number.isSafeInteger(connection.version) || (connection.version ?? 0) < 1 || !connection.models.length) {
    throw new Error("Ortak Gemini ayarının kaydedildiği doğrulanamadı. Ortak bağlantıyı test edin.");
  }
  return `Ortak Gemini anahtarı kaydedildi. ${connection.models.length} model kullanıma hazır.`;
}

export async function previewTeamGeminiMeta() {
  const { current } = geminiWorkspace();
  const preview = await api.previewTeamGeminiMeta(getTeamFunctionSession());
  if (!current()) throw new Error("Çalışma alanı veya oturum değişti.");
  return preview;
}

export async function generateCatalogDraft(page: StorePageDetail) {
  if (!page.team_scope && !teamUser.value && !teamWorkspace.value && !teamRestoring.value) {
    return api.generateStorePage(page.kind, page.remote_id);
  }

  const scope = () => JSON.stringify([teamProject.value, teamUser.value, teamWorkspace.value, "store-pages"]);
  const client = teamClient.value;
  const current = () => client === teamClient.value && page.team_scope === scope() && !!teamSession.value && !teamAccessIssue.value;
  if (!page.team_scope || !current()) throw new Error("Ortak mağaza sayfasını yeniden açın.");
  if (!await checkStorePageDraft(page)) throw new Error("Ortak sayfa taslağı doğrulanamadı.");

  let products: string[] = [];
  if (page.kind === "category" || page.kind === "brand") {
    const snapshot = await refreshTeamCatalog();
    if (!snapshot) throw new Error("Taslak üretmek için ortak ürün kataloğu okunamadı.");
    const normalized = (value?: string | null) => value?.trim().toLocaleLowerCase("tr-TR") ?? "";
    const name = normalized(page.name);
    products = snapshot.products.filter(({ facts }) => page.kind === "category"
      ? normalized(facts.category) === name || normalized(facts.main_category) === name
      : normalized(facts.brand) === name).slice(0, 8).map(({ name }) => name);
  }

  if (!current()) throw new Error("Üretim sırasında çalışma alanı değişti.");
  const session = getTeamFunctionSession();
  const draft = await api.generateStorePage(page.kind, page.remote_id, { page, products, session });
  if (!current()) throw new Error("Üretim sırasında çalışma alanı değişti.");
  return draft;
}
