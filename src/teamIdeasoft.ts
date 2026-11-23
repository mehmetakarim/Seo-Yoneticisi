import { teamAccessIssue, teamClient, teamProject, teamReachable, teamSession, teamUser, teamWorkspace } from "./teamConnection";
import { readIdeasoftInventory, storeKinds, type RemoteStorePage, type StoreKind } from "./ideasoftInventory";
import { invoke } from "@tauri-apps/api/core";
import { teamStorePageScope, type TeamStorePageWriteTarget } from "./teamStorePages";
import type { IdeasoftPreview, NewStorePage, ProductDetail } from "./types";

const scope = () => JSON.stringify([teamProject.value, teamUser.value, teamWorkspace.value]);
const messages: Record<string, string> = {
  sign_in_required: "Ekip oturumu doğrulanamadı. Oturumunuzu kontrol edin.",
  integration_access_denied: "Ortak IdeaSoft bağlantısını yalnızca yönetici ve düzenleyici üyeler kullanabilir.",
  invalid_configuration: "Ortak IdeaSoft bağlantı ayarları sunucuda geçersiz.",
  ideasoft_rotation_owner_required: "Ortak IdeaSoft token’ını çalışma alanı sahibi güncelleyebilir.",
  ideasoft_rotation_origin_mismatch: "Cihazdaki mağaza adresi ortak IdeaSoft mağazasıyla eşleşmiyor.",
  ideasoft_rotation_conflict: "Ortak token aynı anda değiştirilmiş olabilir. Bağlantıyı yeniden test edin.",
  ideasoft_token_store_unavailable: "Ortak token deposuna ulaşılamadı. Bağlantıyı yeniden test edin.",
  ideasoft_credentials_rejected: "Ortak IdeaSoft anahtarı geçersiz veya süresi dolmuş. Sunucudaki bağlantı anahtarı yenilenmeli.",
  ideasoft_rate_limited: "IdeaSoft hız sınırına ulaşıldı. Bir süre sonra yeniden deneyin.",
  ideasoft_not_found: "İstenen kayıt IdeaSoft’ta bulunamadı.",
  ideasoft_preview_stale: "Önizleme güncelliğini yitirdi. Açıp farkları yeniden inceleyin.",
  ideasoft_seo_rejected: "IdeaSoft SEO alanlarını kabul etmedi. İçeriği gözden geçirin.",
  ideasoft_write_unconfirmed: "IdeaSoft işlemi doğrulanamadı. Tekrar göndermeden önce önizlemeyi yeniden açın.",
  ideasoft_receipt_unavailable: "Ortak geçmiş kaydı doğrulanamadı. Gönderimden önce bağlantıyı kontrol edin.",
  ideasoft_catalog_product_missing: "Ürün ortak katalogda bulunamadı veya artık etkin değil.",
  ideasoft_page_revision_conflict: "Ortak sayfanın taslağı değişti. Sayfayı yenileyip son taslağı inceleyin.",
  ideasoft_page_no_changes: "Mağazaya göndermek için önce ortak SEO taslağını kaydedin.",
  ideasoft_page_rejected: "IdeaSoft alanları kabul etmedi. Kayıt bilgilerini gözden geçirin.",
  ideasoft_page_identity_changed: "Mağaza kaydı değişti. Envanteri yenileyip doğru kaydı yeniden açın.",
  ideasoft_page_duplicate: "IdeaSoft’ta aynı ad veya URL ile bir kayıt bulunuyor. Envanteri yenileyip mevcut kaydı açın.",
  ideasoft_page_create_in_progress: "Bu kayıt için bir oluşturma isteği zaten var. Mağaza envanterini yenileyip sonucu kontrol edin.",
  ideasoft_page_duplicate_check_unavailable: "IdeaSoft’ta aynı kayıt olup olmadığı doğrulanamadı. Envanteri ve mağaza bağlantısını kontrol edin.",
  ideasoft_page_state_unavailable: "Ortak sayfa Supabase’den okunamadı. Bağlantıyı kontrol edip sayfayı yenileyin.",
};
const receiptScope = () => `${scope()}:ideasoft-receipts`;
type Receipt = { id: string; payload: { receipt: string }; conflict: string | null };
const receiptCache = <T>(key: string, request: object) => invoke<T>("team_cache", { scope: key, request });
let receiptFlush: Promise<void> | undefined;

class IdeasoftCallError extends Error {
  constructor(message: string, readonly status: number) { super(message); }
}

export class TeamStorePageWriteUnconfirmed extends Error {}

function unconfirmedPageWrite(operation: string) {
  return new TeamStorePageWriteUnconfirmed(operation === "page_create"
    ? "IdeaSoft’ta kayıt oluşturma sonucu doğrulanamadı. Tekrar oluşturmadan önce mağaza envanterini yenileyip kaydı arayın."
    : "IdeaSoft işlemi doğrulanamadı. Yeniden denemeden önce mağaza envanterini yenileyip kaydı inceleyin.");
}

async function callIdeasoft<T>(operation: { operation: string } & Record<string, unknown>): Promise<T> {
  const client = teamClient.value;
  const workspace = teamWorkspace.value;
  if (!client || !workspace || !teamSession.value || teamAccessIssue.value) throw new Error("Önce ekip çalışma alanını açın.");
  if (teamReachable.value === false || !navigator.onLine) throw new Error("Ortak IdeaSoft işlemi için internet ve Supabase bağlantısı gerekiyor.");
  const key = scope();
  const pageWrite = ["page_create", "page_publish", "page_push"].includes(operation.operation);
  const result = await client.functions.invoke<T & { configured: boolean }>("team-ideasoft", {
    body: { workspace, ...operation },
  }).catch((error: unknown) => {
    if (pageWrite) throw unconfirmedPageWrite(operation.operation);
    throw error;
  });
  if (scope() !== key || client !== teamClient.value || !teamSession.value || teamAccessIssue.value) {
    if (pageWrite) throw unconfirmedPageWrite(operation.operation);
    throw new Error("Çalışma alanı veya oturum değişti.");
  }
  if (result.error) {
    const response = result.error.context;
    if (response instanceof Response) {
      const body = await response.json().catch(() => null);
      if (pageWrite && ["ideasoft_write_unconfirmed", "ideasoft_page_create_unconfirmed", "ideasoft_page_write_unconfirmed"].includes(body?.error)) {
        throw unconfirmedPageWrite(operation.operation);
      }
      if (typeof body?.error === "string" && Object.prototype.hasOwnProperty.call(messages, body.error)) {
        throw new IdeasoftCallError(messages[body.error], response.status);
      }
      if (response.status === 404) throw new IdeasoftCallError("Ortak IdeaSoft servisi bu Supabase projesinde henüz kurulmadı.", 404);
      if (response.status === 400 && body?.error === "invalid_request" && operation.operation === "product_preview") {
        throw new IdeasoftCallError("Ortak IdeaSoft önizlemesi pilotta henüz etkinleşmedi veya içerik sınırı aşıldı.", 400);
      }
      if (response.status === 400 && body?.error === "invalid_request" && operation.operation === "rotate_token") {
        throw new IdeasoftCallError("Mağaza adresini ve token’ı kontrol edin. Ortak güncelleme pilotta henüz etkin değilse onu Supabase Secrets’tan yenileyin.", 400);
      }
      if (pageWrite && response.status === 400 && body?.error === "invalid_request") {
        throw new IdeasoftCallError("Kayıt bilgilerini kontrol edin. Bu işlem için ortak IdeaSoft işlevinin güncel sürümü gerekli.", 400);
      }
      if (pageWrite && response.status >= 500) throw unconfirmedPageWrite(operation.operation);
      throw new IdeasoftCallError("Ortak IdeaSoft isteği tamamlanamadı.", response.status);
    }
    if (pageWrite) throw unconfirmedPageWrite(operation.operation);
    if (operation.operation === "product_push") throw new Error(messages.ideasoft_write_unconfirmed);
    throw new Error("Ortak IdeaSoft servisine ulaşılamadı. Bağlantıyı kontrol edip yeniden deneyin.");
  }
  if (!result.data?.configured) throw new Error("Bu çalışma alanı için ortak IdeaSoft bağlantısı henüz yapılandırılmadı.");
  return result.data;
}

export async function testTeamIdeasoft(): Promise<string> {
  const data = await callIdeasoft<{ connected?: boolean }>({ operation: "health" });
  if (data.connected !== true) throw new Error("Ortak IdeaSoft bağlantısı doğrulanamadı.");
  return "Supabase üzerinden IdeaSoft mağaza erişimi doğrulandı.";
}

export async function canRotateTeamIdeasoftToken(): Promise<boolean> {
  const client = teamClient.value;
  const workspace = teamWorkspace.value;
  const user = teamUser.value;
  const key = scope();
  if (!client || !workspace || !user || !teamSession.value || teamAccessIssue.value) {
    throw new Error("Ortak token için ekip oturumunu ve bağlantıyı kontrol edin.");
  }
  const result = await client.from("workspace_members").select("role")
    .eq("workspace_id", workspace).eq("user_id", user).maybeSingle().retry(false);
  if (key !== scope() || teamAccessIssue.value || !teamSession.value) throw new Error("Çalışma alanı veya oturum değişti.");
  if (result.error || !result.data) throw new Error("Ortak token güncelleme yetkisi doğrulanamadı.");
  return result.data.role === "owner";
}

export async function rotateTeamIdeasoftToken(origin: string, token: string, replaceValid: boolean): Promise<boolean> {
  const result = await callIdeasoft<{ connected?: boolean; rotated?: boolean }>({
    operation: "rotate_token", origin: origin.trim(), token: token.trim(), replace_valid: replaceValid,
  });
  if (result.connected !== true || typeof result.rotated !== "boolean") {
    throw new Error("Ortak token güncellemesi doğrulanamadı.");
  }
  return result.rotated;
}

export async function fetchTeamIdeasoftInventory() {
  const key = scope();
  return readIdeasoftInventory(
    (kind, page) => callIdeasoft<{ pages: RemoteStorePage[]; has_more: boolean }>({ operation: "pages", kind, page }),
    () => scope() === key && !!teamSession.value && !teamAccessIssue.value,
  );
}

type StorePageWrite =
  | { operation: "page_create"; kind: string; draft: NewStorePage }
  | { operation: "page_publish" | "page_push"; kind: string; id: number; revision: number };

type StorePageWriteResponse = {
  page: RemoteStorePage & { meta_keywords: string };
  updated: boolean;
  pushed_at?: string;
  facts_recorded?: boolean;
  receipt?: string;
};

export async function writeTeamIdeasoftPage(target: TeamStorePageWriteTarget, request: StorePageWrite) {
  if (target.key !== teamStorePageScope() || target.workspace !== teamWorkspace.value) {
    throw new Error("Çalışma alanı değişti. Ortak mağazayı yeniden açın.");
  }
  if (!storeKinds.includes(request.kind as StoreKind)) throw new Error("Geçersiz katalog türü.");
  if (request.operation !== "page_create" && (!Number.isSafeInteger(request.id) || request.id < 1 ||
      !Number.isSafeInteger(request.revision) || request.revision < 1)) throw new Error("Ortak sayfanın sürümünü yenileyin.");
  const result = await callIdeasoft<StorePageWriteResponse>(request);
  const { page, updated, pushed_at, facts_recorded, receipt } = result;
  if (!page || page.kind !== request.kind || !Number.isSafeInteger(page.id) || page.id < 1 ||
      ![0, 1].includes(page.status) || typeof page.name !== "string" || !page.name.trim() ||
      [page.slug, page.page_title, page.meta_description, page.meta_keywords, page.target_keyword].some((value) => typeof value !== "string") ||
      (page.showcase_content !== null && typeof page.showcase_content !== "string") ||
      typeof updated !== "boolean" || typeof facts_recorded !== "boolean" ||
      (receipt !== undefined && (typeof receipt !== "string" || !/^[A-Za-z0-9_-]{1,900000}\.[0-9a-f]{64}$/.test(receipt))) ||
      (pushed_at !== undefined && (typeof pushed_at !== "string" || !Number.isFinite(Date.parse(pushed_at)))) ||
      (request.operation !== "page_create" && page.id !== request.id) ||
      (request.operation === "page_publish" && page.status !== 1) ||
      (request.operation === "page_push" && updated && !pushed_at) || target.key !== teamStorePageScope()) {
    throw unconfirmedPageWrite(request.operation);
  }
  return {
    remoteId: page.id,
    updated,
    recorded: facts_recorded === true,
    receipt,
    facts: {
      name: page.name, slug: page.slug, status: page.status, page_title: page.page_title,
      meta_description: page.meta_description, meta_keywords: page.meta_keywords, target_keyword: page.target_keyword, showcase_content: page.showcase_content,
      ...(pushed_at ? { pushed_at } : {}),
    },
  };
}

export async function confirmTeamIdeasoftPageReceipt(receipt: string): Promise<"ok" | "retry" | "conflict"> {
  try {
    const result = await callIdeasoft<{ facts_recorded?: boolean }>({ operation: "page_receipt", receipt });
    return result.facts_recorded === true ? "ok" : "retry";
  } catch (error) {
    return error instanceof IdeasoftCallError && [400, 403, 409, 422].includes(error.status) ? "conflict" : "retry";
  }
}

export async function fetchTeamIdeasoftProduct(sku: string) {
  const key = scope();
  type Product = { id: number; sku: string; target_keyword: string; extra_details: string };
  const resolved = await callIdeasoft<{ product: Product | null }>({ operation: "resolve", sku });
  if (!resolved.product) throw new Error("Ürün IdeaSoft’ta bulunamadı.");
  const result = await callIdeasoft<{ product: Product }>({ operation: "product", id: resolved.product.id });
  const product = result.product;
  if (!product || product.sku.trim().toLowerCase() !== sku.trim().toLowerCase()) throw new Error("IdeaSoft ürün eşleşmesi doğrulanamadı.");
  const text = await invoke<string>("ideasoft_tech_text", { html: product.extra_details });
  if (key !== scope() || !teamSession.value || teamAccessIssue.value) throw new Error("Çalışma alanı değişti.");
  return { keyword: product.target_keyword.trim(), techText: text };
}

export async function previewTeamIdeasoftProduct(detail: ProductDetail, parts: string[]) {
  const key = scope();
  const changes = await invoke<Record<string, unknown>>("ideasoft_shared_payload", {
    content: {
      page_title: detail.draft_title ?? detail.title ?? "",
      meta_description: detail.draft_descriptions ?? detail.descriptions ?? "",
      meta_keywords: detail.draft_keywords ?? detail.keywords ?? detail.draft_search_keywords ?? "",
      search_keywords: detail.draft_search_keywords ?? detail.search_keywords ?? "",
      target_keyword: detail.target_keyword ?? "",
      details_html: detail.draft_details ?? detail.details ?? "",
    },
    techSpecs: detail.tech_specs,
    parts,
  });
  if (key !== scope()) throw new Error("Çalışma alanı değişti.");
  if (!Object.keys(changes).length) throw new Error("Gönderilecek içerik yok.");
  const data = await callIdeasoft<IdeasoftPreview & { confirmation: string }>({ operation: "product_preview", sku: detail.sku, changes });
  if (typeof data.confirmation !== "string" || data.id !== data.remote?.id || !data.local) throw new Error("IdeaSoft önizlemesi doğrulanamadı.");
  return data;
}

export async function pushTeamIdeasoftProduct(sku: string, preview: IdeasoftPreview) {
  if (!preview.confirmation) throw new Error("Ortak IdeaSoft önizlemesini yeniden açın.");
  const key = receiptScope();
  const data = await callIdeasoft<{ id: number; updated: boolean; pushed_at?: string; event_recorded?: boolean; receipt?: string }>({
    operation: "product_push", sku, changes: preview.local, confirmation: preview.confirmation,
  });
  if (data.id !== preview.id || typeof data.updated !== "boolean") throw new Error("Mağaza işlemi doğrulanamadı. Önizlemeyi yeniden açın.");
  if (data.updated && !data.event_recorded && data.receipt) {
    const id = crypto.randomUUID();
    try {
      await receiptCache(key, { action: "queue", id, contact: { id }, payload: { receipt: data.receipt } });
      return { ...data, event_pending: true };
    } catch {
      // Mağaza işlemi başarılı; yerelde kayıt başarısız diye tekrar gönderilmemeli.
    }
  }
  return { ...data, event_pending: false };
}

export function flushTeamIdeasoftReceipts(): Promise<void> {
  if (receiptFlush) return receiptFlush;
  if (!teamWorkspace.value || !teamSession.value || teamAccessIssue.value || teamReachable.value !== true) return Promise.resolve();
  const key = receiptScope();
  receiptFlush = (async () => {
    for (const op of await receiptCache<Receipt[]>(key, { action: "pending" })) {
      if (key !== receiptScope() || !teamSession.value || teamAccessIssue.value) return;
      if (op.conflict) continue;
      try {
        const data = await callIdeasoft<{ event_recorded?: boolean }>({ operation: "product_push_receipt", ...op.payload });
        if (data.event_recorded !== true) return;
        await receiptCache(key, { action: "acknowledge", id: op.id, contact: { id: op.id } });
      } catch (error) {
        if (!(error instanceof IdeasoftCallError) || ![400, 409, 422].includes(error.status)) return;
        await receiptCache(key, { action: "conflict", id: op.id, reason: "Mağaza geçmiş kaydı doğrulanamadı." });
      }
    }
  })().finally(() => { receiptFlush = undefined; });
  return receiptFlush;
}
