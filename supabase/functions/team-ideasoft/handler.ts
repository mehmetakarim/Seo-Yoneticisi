// Workspace-scoped IdeaSoft operations. Only owners can rotate the merchant token.
type Env = (name: string) => string | undefined;
const cors = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Headers": "authorization, apikey, content-type, x-client-info",
  "Access-Control-Allow-Methods": "POST, OPTIONS",
  "Cache-Control": "no-store",
};
const reply = (body: object, status = 200) => Response.json(body, { status, headers: cors });
const signal = () => AbortSignal.timeout(12_000);
const resources = { category: "categories", brand: "brands", blog: "blogs" } as const;
type PageKind = keyof typeof resources;
type Input = { workspace: string; operation: string; id?: number; sku?: string; kind?: PageKind; page?: number; changes?: unknown; confirmation?: unknown; receipt?: unknown; token?: unknown; origin?: unknown; replace_valid?: unknown; draft?: unknown; revision?: unknown };
type Row = Record<string, unknown>;
const workspaceId = /^[a-f\d]{8}-[a-f\d]{4}-[a-f\d]{4}-[a-f\d]{4}-[a-f\d]{12}$/i;
const string = (row: Row, key: string) => typeof row[key] === "string" ? row[key] as string : "";
const object = (value: unknown): value is Row => !!value && typeof value === "object" && !Array.isArray(value);
const positiveId = (value: unknown): value is number => typeof value === "number" && Number.isSafeInteger(value) && value > 0;
const pageKind = (kind: unknown): kind is PageKind => typeof kind === "string" && Object.hasOwn(resources, kind);

function validSku(value: unknown): value is string {
  return typeof value === "string" && !!value.trim() && value.length <= 256;
}

function bearerToken(value: unknown): string | null {
  if (typeof value !== "string" || /[\r\n]/.test(value)) return null;
  const token = value.trim();
  return token.length <= 3000 && /^[A-Za-z0-9._~+/-]+=*$/.test(token) ? token : null;
}

function productSearch(sku: string): string {
  return `products?${new URLSearchParams({ s: sku.trim(), limit: "20" })}`;
}

function target(input: Input): string {
  switch (input.operation) {
    case "health":
    case "rotate_token": return "products?limit=1";
    case "product":
      if (positiveId(input.id)) return `products/${input.id}`;
      break;
    case "resolve":
    case "product_preview":
    case "product_push":
      if (validSku(input.sku)) return productSearch(input.sku);
      break;
    case "pages":
      if (pageKind(input.kind) && positiveId(input.page) && input.page <= 500) {
        return `${resources[input.kind]}?limit=100&page=${input.page}`;
      }
      break;
    case "page_create":
      if (pageKind(input.kind)) return resources[input.kind];
      break;
    case "page_push":
    case "page_publish":
      if (pageKind(input.kind) && positiveId(input.id) && positiveId(input.revision)) return `${resources[input.kind]}/${input.id}`;
      break;
  }
  throw new Error("invalid_request");
}

// The endpoint is provisioned by the server administrator, never from a desktop request.
function origin(value: unknown): string {
  if (typeof value !== "string") throw new Error("invalid_configuration");
  const url = new URL(value);
  if (url.protocol !== "https:" || url.username || url.password || url.port || url.search || url.hash ||
      !["/", "/admin-api", "/admin-api/"].includes(url.pathname) ||
      !/^(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z]{2,}$/i.test(url.hostname) ||
      /\.(localhost|local|internal|test|invalid)$/i.test(url.hostname)) throw new Error("invalid_configuration");
  return url.origin;
}

function product(row: Row) {
  if (!positiveId(row.id)) throw new Error("invalid_response");
  const detail = object(row.detail) ? row.detail : {};
  return {
    id: row.id, sku: string(row, "sku"), name: string(row, "name"),
    page_title: string(row, "pageTitle"), meta_description: string(row, "metaDescription"),
    meta_keywords: string(row, "metaKeywords"), search_keywords: string(row, "searchKeywords"),
    target_keyword: string(row, "targetKeyword"), details: string(detail, "details"),
    extra_details: string(detail, "extraDetails"),
    seo_rule_count: Number.isSafeInteger(row.seoTotalRuleCount) ? row.seoTotalRuleCount : null,
  };
}

const metaFields = ["pageTitle", "metaDescription", "metaKeywords", "searchKeywords", "targetKeyword"] as const;
const detailFields = ["details", "extraDetails"] as const;
type ProductSeoChanges = Partial<Record<typeof metaFields[number], string>> & {
  detail?: Partial<Record<typeof detailFields[number], string>>;
};
const maxWriteBody = 1_000_000;
const maxDetailLength = 250_000;
const encoder = new TextEncoder();

async function readInput(req: Request): Promise<Input> {
  if (Number(req.headers.get("content-length")) > maxWriteBody || !req.body) throw new Error();
  const decoder = new TextDecoder("utf-8", { fatal: true });
  const reader = req.body.getReader();
  let raw = "", size = 0;
  try {
    for (;;) {
      const { value, done } = await reader.read();
      if (done) break;
      size += value.byteLength;
      if (size > maxWriteBody) {
        await reader.cancel();
        throw new Error();
      }
      raw += decoder.decode(value, { stream: true });
    }
  } finally {
    reader.releaseLock();
  }
  const input = JSON.parse(raw + decoder.decode());
  if (!object(input) || typeof input.workspace !== "string" || typeof input.operation !== "string" ||
      !workspaceId.test(input.workspace)) throw new Error();
  if (size > 4096 && !["product_preview", "product_push", "page_create", "page_receipt"].includes(input.operation)) throw new Error();
  return input as Input;
}

function selectedChanges(value: unknown): ProductSeoChanges {
  if (!object(value)) throw new Error();
  const allowed: readonly string[] = [...metaFields, "detail"];
  const allowedDetails: readonly string[] = detailFields;
  if (!Object.keys(value).length || Object.keys(value).some((key) => !allowed.includes(key))) throw new Error();
  const changes: ProductSeoChanges = {};
  for (const field of metaFields) {
    if (!Object.hasOwn(value, field)) continue;
    const content = value[field];
    if (typeof content !== "string" || !content.trim() || content.length > 5000) throw new Error();
    changes[field] = content.trim();
  }
  if (Object.hasOwn(value, "detail")) {
    const detail = value.detail;
    if (!object(detail) || !Object.keys(detail).length || Object.keys(detail).some((k) => !allowedDetails.includes(k))) throw new Error();
    changes.detail = {};
    for (const field of detailFields) {
      if (!Object.hasOwn(detail, field)) continue;
      const content = detail[field];
      if (typeof content !== "string" || !content.trim() || content.length > maxDetailLength) throw new Error();
      changes.detail[field] = content;
    }
  }
  return changes;
}

const normalizedSku = (sku: string) => sku.trim().toLowerCase();
const merchantError = (status: number) => {
  if (status === 401 || status === 403) return reply({ error: "ideasoft_credentials_rejected" }, 502);
  if (status === 404) return reply({ error: "ideasoft_not_found" }, 404);
  if (status === 429) return reply({ error: "ideasoft_rate_limited" }, 429);
  return reply({ error: "ideasoft_unavailable" }, 502);
};

function selectedRemote(row: Row, changes: ProductSeoChanges): ProductSeoChanges {
  const previous: ProductSeoChanges = {};
  for (const field of metaFields) if (changes[field] !== undefined) previous[field] = string(row, field);
  if (changes.detail) {
    previous.detail = {};
    const detail = object(row.detail) ? row.detail : {};
    for (const field of detailFields) if (changes.detail[field] !== undefined) previous.detail[field] = string(detail, field);
  }
  return previous;
}

async function signingKey(secret: string, purpose: string) {
  const algorithm = { name: "HMAC", hash: "SHA-256" };
  const key = await crypto.subtle.importKey("raw", encoder.encode(secret), algorithm, false, ["sign"]);
  const derived = await crypto.subtle.sign("HMAC", key, encoder.encode(`seo-yoneticisi:ideasoft-${purpose}:v1`));
  return crypto.subtle.importKey("raw", derived, algorithm, false, ["sign", "verify"]);
}

const hex = (bytes: ArrayBuffer) => Array.from(new Uint8Array(bytes), (byte) => byte.toString(16).padStart(2, "0")).join("");
const unhex = (value: string) => Uint8Array.from(value.match(/../g)!, (byte) => Number.parseInt(byte, 16));
const eventKeyPattern = /^ideasoft:[0-9a-f]{64}$/;
const receiptPattern = /^[A-Za-z0-9_-]{1,3000}\.[0-9a-f]{64}$/;
const timestampPattern = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/;

type PushReceipt = { eventKey: string; sku: string; id: number; pushedAt: string };
type PushRecord = { id: number; pushed_at: string };
type RecordPush = (targetSku: string, eventKey: string, confirmation?: PushReceipt) => Promise<PushRecord | null | undefined>;
type PushEvent = { key: string; record: RecordPush; receiptKey: CryptoKey };

function serviceSecret(env: Env) {
  try {
    const keys = JSON.parse(env("SUPABASE_SECRET_KEYS") ?? "{}");
    if (typeof keys.default === "string" && keys.default.startsWith("sb_secret_")) return keys.default;
  } catch { /* The legacy service-role key is available on older projects. */ }
  return env("SUPABASE_SERVICE_ROLE_KEY");
}

type ServiceRpc = (name: string, args: Row) => Promise<Response>;

function serviceRpc(supabase: string, serviceKey: string, request: typeof fetch): ServiceRpc {
  return (name, args) => request(`${supabase}/rest/v1/rpc/${name}`, {
    method: "POST", headers: { apikey: serviceKey, "Content-Type": "application/json",
      ...(!serviceKey.startsWith("sb_secret_") ? { Authorization: `Bearer ${serviceKey}` } : {}) },
    body: JSON.stringify(args), redirect: "error", signal: signal(),
  });
}

type StoredToken = { token: string; version: number };
type Rotation = { token: string; origin: string; replaceValid: boolean };

async function readStoredToken(rpc: ServiceRpc, workspace: string, base: string): Promise<StoredToken | null | undefined> {
  try {
    const response = await rpc("team_get_ideasoft_token", { target_workspace: workspace, target_origin: base });
    if (!response.ok) return undefined;
    const result: unknown = await response.json();
    if (result === null) return null;
    if (!object(result) || !positiveId(result.version)) return undefined;
    const token = bearerToken(result.token);
    return token === null ? undefined : { token, version: result.version };
  } catch { return undefined; }
}

async function merchantHealth(merchant: MerchantRequest) {
  const response = await merchant("products?limit=1");
  if (!response.ok) return { status: response.status, failure: merchantError(response.status) };
  const products: unknown = await response.json();
  const failure = !Array.isArray(products) || !products.every(object)
    ? reply({ error: "invalid_ideasoft_response" }, 502)
    : null;
  return { status: response.status, failure };
}

async function rotateToken(
  rpc: ServiceRpc, workspace: string, owner: string, base: string,
  rotation: Rotation, stored: StoredToken | null, fallbackToken: string | null, request: typeof fetch,
) {
  const currentToken = stored?.token ?? fallbackToken;
  if (currentToken !== null && (currentToken === rotation.token || !rotation.replaceValid)) {
    const active = await merchantHealth(merchantRequest(base, currentToken, request));
    if (!active.failure) return reply({ configured: true, connected: true, rotated: false });
    if (currentToken === rotation.token || ![401, 403].includes(active.status)) return active.failure;
  }
  const candidate = await merchantHealth(merchantRequest(base, rotation.token, request));
  if (candidate.failure) return candidate.failure;

  try {
    const version = stored?.version ?? 0;
    const saved = await rpc("team_set_ideasoft_token", {
      target_workspace: workspace, target_owner: owner, target_origin: base, new_token: rotation.token, expected_version: version,
    });
    if (saved.status === 409) return reply({ error: "ideasoft_rotation_conflict" }, 409);
    if (saved.status === 403 && (await saved.json().catch(() => null))?.code === "42501") {
      return reply({ error: "ideasoft_rotation_owner_required" }, 403);
    }
    const result: unknown = saved.ok ? await saved.json() : null;
    if (object(result) && result.version === version + 1) {
      return reply({ configured: true, connected: true, rotated: true, version: result.version });
    }
  } catch { /* Do not forward details from a request carrying a credential. */ }
  return reply({ error: "ideasoft_token_store_unavailable" }, 503);
}

function eventRecorder(supabase: string, serviceKey: string, workspace: string, request: typeof fetch): RecordPush {
  const rpc = serviceRpc(supabase, serviceKey, request);
  return async (targetSku, eventKey, confirmation) => {
    try {
      const response = await rpc("team_record_ideasoft_push", {
        target_workspace: workspace, target_sku: targetSku, target_event_key: eventKey,
        ...(confirmation ? { confirmed_at: confirmation.pushedAt, confirmed_product_id: confirmation.id } : {}),
      });
      if (!response.ok) return undefined;
      const row: unknown = await response.json();
      if (row === null) return null;
      if (!object(row) || !positiveId(row.id) || typeof row.pushed_at !== "string" || !timestampPattern.test(row.pushed_at)) return undefined;
      return { id: row.id, pushed_at: row.pushed_at };
    } catch { return undefined; }
  };
}

function recordedPush(row: PushRecord) {
  return reply({ configured: true, updated: true, event_recorded: true, ...row });
}

async function signReceipt(receipt: PushReceipt, workspace: string, user: string, key: CryptoKey) {
  const encoded = encoder.encode(JSON.stringify({ ...receipt, workspace, user }));
  const value = btoa(String.fromCharCode(...encoded)).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
  const signature = await crypto.subtle.sign("HMAC", key, encoder.encode(value));
  return `${value}.${hex(signature)}`;
}

async function readReceipt(receipt: string, workspace: string, user: string, key: CryptoKey): Promise<PushReceipt | null> {
  const [value, signature] = receipt.split(".");
  if (!await crypto.subtle.verify("HMAC", key, unhex(signature), encoder.encode(value))) return null;
  try {
    const bytes = Uint8Array.from(atob(value.replaceAll("-", "+").replaceAll("_", "/")), (c) => c.charCodeAt(0));
    const row: unknown = JSON.parse(new TextDecoder().decode(bytes));
    if (!object(row) || row.workspace !== workspace || row.user !== user || !validSku(row.sku) || !positiveId(row.id) ||
        typeof row.eventKey !== "string" || !eventKeyPattern.test(row.eventKey) ||
        typeof row.pushedAt !== "string" || !timestampPattern.test(row.pushedAt) ||
        Date.now() - Date.parse(row.pushedAt) > 7 * 24 * 60 * 60 * 1000) return null;
    return { sku: row.sku, eventKey: row.eventKey, id: row.id, pushedAt: row.pushedAt };
  } catch { return null; }
}

function pendingPush(data: PushReceipt, receipt: string) {
  return reply({ configured: true, id: data.id, updated: true, event_recorded: false, pushed_at: data.pushedAt, receipt });
}

function stalePreview() {
  return reply({ error: "ideasoft_preview_stale" }, 409);
}

type MerchantRequest = (path: string, method?: "GET" | "PUT" | "POST", body?: Row, etag?: string) => Promise<Response>;

function merchantRequest(base: string, token: string, request: typeof fetch): MerchantRequest {
  return (path, method = "GET", body, etag) => request(`${base}/admin-api/${path}`, {
    method, headers: { Authorization: `Bearer ${token}`, Accept: "application/json",
      ...(body ? { "Content-Type": "application/json" } : {}), ...(etag ? { "If-Match": etag } : {}) },
    body: body ? JSON.stringify(body) : undefined,
    redirect: "error", signal: signal(),
  });
}

type PageMutation =
  | { kind: PageKind; operation: "page_create"; payload: Row }
  | { kind: PageKind; operation: "page_push" | "page_publish"; id: number; revision: number };

const pageNameField = (kind: PageKind) => kind === "blog" ? "title" : "name";
const clip = (text: string, limit: number) => Array.from(text.trim()).slice(0, limit).join("");

function wrapParagraphs(text: string) {
  return text.replace(/\r\n?/g, "\n").split("\n\n").map((part) =>
    part.trim().split("\n").map((line) => line.trim()).filter(Boolean).map((line) =>
      line.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;")
    ).join("<br>")
  ).filter(Boolean).map((part) => `<p>${part}</p>`).join("");
}

function pageCreatePayload(value: unknown, kind: PageKind): Row {
  const fields = ["name", "slug", "pageTitle", "metaDescription", "excerpt", "content"] as const;
  const allowed: readonly string[] = [...fields, "publish"];
  if (!object(value) || Object.keys(value).some((key) => !allowed.includes(key)) ||
      fields.some((field) => typeof value[field] !== "string") || typeof value.publish !== "boolean") throw new Error();
  const name = string(value, "name").trim();
  const slug = string(value, "slug").trim();
  const pageTitle = string(value, "pageTitle").trim();
  const metaDescription = string(value, "metaDescription").trim();
  if (!name || Array.from(name).length > 255 || Array.from(pageTitle).length > 255 ||
      Array.from(metaDescription).length > 160 || !/^[a-z0-9-]{0,255}$/.test(slug)) throw new Error();

  const payload: Row = { [pageNameField(kind)]: name, status: Number(value.publish) };
  if (slug) payload.slug = slug;
  if (pageTitle) payload.pageTitle = pageTitle;
  if (metaDescription) payload.metaDescription = metaDescription;
  if (kind === "blog") {
    const excerpt = string(value, "excerpt").trim();
    const content = wrapParagraphs(string(value, "content"));
    if (!excerpt || !content || encoder.encode(excerpt).byteLength > 65_535 || encoder.encode(content).byteLength > 65_535) throw new Error();
    return { ...payload, excerpt, content, blockVisibility: 0, categories: [] };
  }
  const defaults = { sortOrder: 0, displayShowcaseContent: 0, displayShowcaseFooterContent: 0 };
  if (kind === "brand") return { ...payload, ...defaults };
  return { ...payload, ...defaults, showcaseContentDisplayType: 1, showcaseFooterContentDisplayType: 1, hasChildren: 1, isCombine: 1 };
}

function readPageMutation(input: Input): PageMutation | undefined {
  const { kind, operation } = input;
  if (!pageKind(kind)) return;
  if (operation === "page_create") {
    if (Object.keys(input).some((key) => !["workspace", "operation", "kind", "draft"].includes(key))) throw new Error();
    return { kind, operation, payload: pageCreatePayload(input.draft, kind) };
  }
  if (operation === "page_push" || operation === "page_publish") {
    if (Object.keys(input).some((key) => !["workspace", "operation", "kind", "id", "revision"].includes(key)) ||
        !positiveId(input.id) || !positiveId(input.revision)) throw new Error();
    return { kind, operation, id: input.id, revision: input.revision };
  }
}

function pageFacts(row: Row, kind: PageKind) {
  return {
    kind, id: row.id, slug: string(row, "slug").trim(), name: string(row, pageNameField(kind)).trim(),
    page_title: string(row, "pageTitle"), meta_description: string(row, "metaDescription"),
    meta_keywords: string(row, "metaKeywords"), target_keyword: string(row, "targetKeyword"),
    showcase_content: typeof row.showcaseContent === "string" ? row.showcaseContent : null,
    status: Number.isSafeInteger(row.status) ? row.status : 1,
  };
}

type ConfirmedPage = ReturnType<typeof pageFacts> & { id: number };
type PageReceipt = { page: ConfirmedPage; confirmedAt: string; trackSeo: boolean; publication?: true };
type PageRecorder = (page: ConfirmedPage, updated: boolean, trackSeo: boolean, publication?: boolean) => Promise<Response>;
const pageReceiptPattern = /^[A-Za-z0-9_-]{1,900000}\.[0-9a-f]{64}$/;

async function recordPageFacts(rpc: ServiceRpc, workspace: string, user: string, observation: PageReceipt) {
  const { page: { kind, id, ...facts }, confirmedAt, trackSeo, publication } = observation;
  try {
    const operation = publication ? "team_confirm_ideasoft_page_publication" : "team_confirm_ideasoft_page";
    const response = await rpc(operation, {
      target_workspace: workspace, target_actor: user, target_kind: kind, target_id: id,
      confirmed_facts: facts, confirmed_at: confirmedAt, track_seo: !publication && trackSeo,
    });
    return response.ok && positiveId(await response.json());
  } catch { return false; }
}

function pageRecorder(rpc: ServiceRpc, workspace: string, user: string, serviceKey: string): PageRecorder {
  return async (page, updated, trackSeo, publication = false) => {
    const confirmedAt = new Date().toISOString();
    const observation: PageReceipt = { page, confirmedAt, trackSeo, ...(publication ? { publication: true } : {}) };
    const recorded = await recordPageFacts(rpc, workspace, user, observation);
    const result = { configured: true, page, updated, facts_recorded: recorded, ...(trackSeo ? { pushed_at: confirmedAt } : {}) };
    if (recorded) return reply(result);
    const bytes = encoder.encode(JSON.stringify({ ...observation, workspace, user }));
    if (bytes.length > 650_000) return reply(result);
    let binary = "";
    for (let offset = 0; offset < bytes.length; offset += 8192) binary += String.fromCharCode(...bytes.subarray(offset, offset + 8192));
    const value = btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
    const signature = await crypto.subtle.sign("HMAC", await signingKey(serviceKey, "page-receipt"), encoder.encode(value));
    return reply({ ...result, receipt: `${value}.${hex(signature)}` });
  };
}

async function recoverPageReceipt(receipt: string, rpc: ServiceRpc, workspace: string, user: string, serviceKey: string) {
  const [value, signature] = receipt.split(".");
  const valid = await crypto.subtle.verify("HMAC", await signingKey(serviceKey, "page-receipt"), unhex(signature), encoder.encode(value));
  if (!valid) return reply({ error: "invalid_ideasoft_receipt" }, 409);
  try {
    const bytes = Uint8Array.from(atob(value.replaceAll("-", "+").replaceAll("_", "/")), (c) => c.charCodeAt(0));
    const data = JSON.parse(new TextDecoder().decode(bytes));
    if (data.workspace !== workspace || data.user !== user || !timestampPattern.test(data.confirmedAt) ||
        (data.publication !== undefined && (data.publication !== true || data.trackSeo || data.page?.status !== 1)) ||
        Date.now() - Date.parse(data.confirmedAt) > 7 * 24 * 60 * 60 * 1000) return reply({ error: "invalid_ideasoft_receipt" }, 409);
    const recorded = await recordPageFacts(rpc, workspace, user, data);
    if (recorded) return reply({ configured: true, facts_recorded: true });
  } catch { return reply({ error: "invalid_ideasoft_receipt" }, 409); }
  return reply({ error: "ideasoft_page_state_unavailable" }, 503);
}

function pageUnconfirmed(operation: "create" | "write") {
  return reply({ error: operation === "create" ? "ideasoft_page_create_unconfirmed" : "ideasoft_page_write_unconfirmed" }, 502);
}

const duplicatedPage = () => reply({ error: "ideasoft_page_duplicate" }, 409);
const pageRevisionConflict = () => reply({ error: "ideasoft_page_revision_conflict" }, 409);
const pageIdentityChanged = () => reply({ error: "ideasoft_page_identity_changed" }, 409);
const normalizedPageKey = (value: string) => value.trim().normalize("NFC").toLocaleLowerCase("tr");

async function withPageCreateClaim(
  rpc: ServiceRpc, workspace: string, user: string, mutation: Extract<PageMutation, { operation: "page_create" }>,
  write: () => Promise<Response>,
) {
  const { kind, payload } = mutation;
  const slug = string(payload, "slug");
  const identity = slug ? `slug:${slug}` : `name:${normalizedPageKey(string(payload, pageNameField(kind)))}`;
  const args = { target_workspace: workspace, target_actor: user, target_kind: kind, target_identity: identity, target_claim: crypto.randomUUID() };
  const claim = await rpc("team_claim_ideasoft_page_create", { ...args, release: false }).catch(() => null);
  const claimed: unknown = claim?.ok ? await claim.json().catch(() => null) : null;
  if (claimed === false) return reply({ error: "ideasoft_page_create_in_progress" }, 409);
  if (claimed !== true) return reply({ error: "ideasoft_page_state_unavailable" }, 503);
  let uncertain = true;
  try {
    const result = await write();
    uncertain = result.status === 502 && (await result.clone().json().catch(() => null))?.error === "ideasoft_page_create_unconfirmed";
    return result;
  } finally {
    if (!uncertain) await rpc("team_claim_ideasoft_page_create", { ...args, release: true }).catch(() => null);
  }
}

async function readPageRow(response: Response): Promise<Row | null> {
  const data: unknown = await response.json().catch(() => null);
  if (!object(data)) return null;
  const row = object(data.data) ? data.data : data;
  return positiveId(row.id) ? row : null;
}

async function handlePageCreate(path: string, kind: PageKind, payload: Row, merchant: MerchantRequest, record: PageRecorder) {
  const slug = string(payload, "slug");
  const field = slug ? "slug" : pageNameField(kind);
  const requested = normalizedPageKey(string(payload, field));
  // The API may ignore arbitrary slug filters. Scan the proven paginated list and compare exactly.
  const seen = new Set<number>();
  for (let offset = 1; offset <= 51; offset++) {
    const result = await merchant(`${path}?limit=100&page=${offset}`);
    if (!result.ok) return merchantError(result.status);
    const rows: unknown = await result.json().catch(() => null);
    if (!Array.isArray(rows) || rows.length > 100 || !rows.every(object)) return reply({ error: "invalid_ideasoft_response" }, 502);
    for (const row of rows) {
      if (normalizedPageKey(string(row, field)) === requested) return duplicatedPage();
      if (!positiveId(row.id) || seen.has(row.id)) return reply({ error: "invalid_ideasoft_response" }, 502);
      seen.add(row.id);
    }
    if (rows.length < 100) break;
    if (offset === 51) return reply({ error: "ideasoft_page_duplicate_check_unavailable" }, 503);
  }
  let result: Response;
  // Creation is not retried. An unclear response requires checking the inventory before trying again.
  try { result = await merchant(path, "POST", payload); }
  catch { return pageUnconfirmed("create"); }
  if (result.status === 409) return duplicatedPage();
  if (result.status === 400 || result.status === 422) return reply({ error: "ideasoft_page_rejected" }, 422);
  if (result.status === 202 || result.status >= 500) return pageUnconfirmed("create");
  if (!result.ok) return merchantError(result.status);
  const created = await readPageRow(result);
  if (!created) return pageUnconfirmed("create");

  // Some successful creation responses omit fields, including the requested status.
  const page = pageFacts({ ...payload, ...created }, kind);
  if (!positiveId(page.id) || !page.name || (page.status !== 0 && page.status !== 1)) return pageUnconfirmed("create");
  if (payload.status === 1) {
    // A positive create reply can confirm existence, but only the merchant readback confirms visibility.
    try {
      const confirmed = await merchant(`${path}/${page.id}`);
      const actual = confirmed.ok ? await readPageRow(confirmed) : null;
      if (samePage(actual, page, kind, page.id) && (actual.status === 0 || actual.status === 1)) {
        return record({ ...pageFacts(actual, kind), id: page.id }, true, false, actual.status === 1);
      }
    } catch { /* The page may exist even though the visibility readback failed. */ }
    return pageUnconfirmed("create");
  }
  return record({ ...page, id: page.id }, true, false);
}

type SharedPage = (revision: number) => Promise<Row | null | undefined>;

function sharedPageReader(supabase: string, headers: Record<string, string>, workspace: string, kind: PageKind, id: number, request: typeof fetch): SharedPage {
  return async (revision) => {
    const filters = new URLSearchParams({ select: "kind,remote_id,revision,details", workspace_id: `eq.${workspace}`,
      kind: `eq.${kind}`, remote_id: `eq.${id}`, limit: "1" });
    const response = await request(`${supabase}/rest/v1/team_store_pages?${filters}`, { headers, redirect: "error", signal: signal() });
    if (!response.ok) return undefined;
    const rows: unknown = await response.json().catch(() => null);
    if (!Array.isArray(rows)) return undefined;
    const row = rows[0];
    if (!object(row) || row.kind !== kind || row.remote_id !== id || row.revision !== revision || !object(row.details)) return null;
    return row.details;
  };
}

function sharedPagePatch(kind: PageKind, details: Row): Row {
  const values: Row = {
    pageTitle: string(details, "draft_page_title").trim(),
    metaDescription: clip(string(details, "draft_meta_description"), 160),
    targetKeyword: string(details, "draft_target_keyword").trim(),
  };
  if (kind !== "blog") values.showcaseContent = wrapParagraphs(string(details, "draft_showcase"));
  return Object.fromEntries(Object.entries(values).filter(([, value]) => value !== ""));
}

function samePage(row: Row | null, details: Row, kind: PageKind, id: number): row is Row {
  if (!row || row.id !== id || !string(row, pageNameField(kind)).trim()) return false;
  const slug = normalizedPageKey(string(details, "slug"));
  if (slug) return normalizedPageKey(string(row, "slug")) === slug;
  const name = normalizedPageKey(string(details, "name"));
  return !!name && normalizedPageKey(string(row, pageNameField(kind))) === name;
}

const containsPagePatch = (row: Row, patch: Row) => Object.entries(patch).every(([field, value]) =>
  typeof value === "string" ? string(row, field).trim() === value : row[field] === value);

async function handlePageUpdate(
  path: string, action: Exclude<PageMutation, { operation: "page_create" }>, merchant: MerchantRequest, readShared: SharedPage, record: PageRecorder,
) {
  const { id, kind, revision } = action;
  const details = await readShared(revision);
  if (details === undefined) return reply({ error: "ideasoft_page_state_unavailable" }, 503);
  if (!details) return pageRevisionConflict();

  const patch = action.operation === "page_publish" ? { status: 1 } : sharedPagePatch(kind, details);
  if (!Object.keys(patch).length) return reply({ error: "ideasoft_page_no_changes" }, 422);

  const existing = await merchant(path);
  if (!existing.ok) return merchantError(existing.status);
  const current = await readPageRow(existing);
  if (!samePage(current, details, kind, id)) return pageIdentityChanged();
  if (containsPagePatch(current, patch)) return record({ ...pageFacts(current, kind), id }, false, false);

  // Recheck after the merchant read, before transmitting the reviewed shared revision.
  const latest = await readShared(revision);
  if (latest === undefined) return reply({ error: "ideasoft_page_state_unavailable" }, 503);
  if (!latest || !samePage(current, latest, kind, id)) return pageRevisionConflict();

  const upstreamEtag = existing.headers.get("ETag");
  const etag = upstreamEtag?.startsWith('"') ? upstreamEtag : undefined;
  let result: Response;
  try { result = await merchant(path, "PUT", patch, etag); }
  catch { return pageUnconfirmed("write"); }
  if (result.status === 409 || result.status === 412) return pageIdentityChanged();
  if (result.status === 400 || result.status === 422) return reply({ error: "ideasoft_page_rejected" }, 422);
  if (result.status === 202 || result.status >= 500) return pageUnconfirmed("write");
  if (!result.ok) return merchantError(result.status);
  try {
    const confirmed = await merchant(path);
    const actual = confirmed.ok ? await readPageRow(confirmed) : null;
    if (samePage(actual, latest, kind, id) && containsPagePatch(actual, patch)) {
      const publication = action.operation === "page_publish" && current.status === 0 && actual.status === 1;
      return record({ ...pageFacts(actual, kind), id }, true, action.operation === "page_push", publication);
    }
  } catch { /* This write has no verified merchant representation yet. */ }
  return pageUnconfirmed("write");
}

async function handleProductWrite(
  input: Input, workspaceUser: string, token: string, changes: ProductSeoChanges, search: Response, merchant: MerchantRequest,
  event?: PushEvent,
): Promise<Response> {
  const sku = normalizedSku(input.sku!);
  const rows: unknown = await search.json();
  if (!Array.isArray(rows) || !rows.every(object)) return reply({ error: "invalid_ideasoft_response" }, 502);
  const matches = rows.filter((row) => normalizedSku(string(row, "sku")) === sku);
  if (!matches.length) return reply({ error: "ideasoft_not_found" }, 404);
  const id = matches[0].id;
  if (matches.length !== 1 || !positiveId(id)) return reply({ error: "invalid_ideasoft_response" }, 502);
  const path = `products/${id}`;
  const current = await merchant(path);
  if (!current.ok) return merchantError(current.status);
  const row: unknown = await current.json();
  if (!object(row) || row.id !== id || normalizedSku(string(row, "sku")) !== sku) {
    return reply({ error: "invalid_ideasoft_response" }, 502);
  }
  const previous = selectedRemote(row, changes);
  const message = (deadline: string) => encoder.encode(JSON.stringify([deadline, input.workspace.toLowerCase(), workspaceUser, id, sku, changes, previous]));
  const key = await signingKey(token, "product-preview");
  if (input.operation === "product_preview") {
    const deadline = String(Date.now() + 10 * 60 * 1000);
    const signature = await crypto.subtle.sign("HMAC", key, message(deadline));
    const confirmation = `${deadline}.${hex(signature)}`;
    return reply({ configured: true, id, remote: product(row), local: changes, confirmation });
  }

  const [deadline, signature] = (input.confirmation as string).split(".");
  if (Number(deadline) < Date.now() || !await crypto.subtle.verify("HMAC", key,
    unhex(signature), message(deadline))) return stalePreview();
  if (JSON.stringify(previous) === JSON.stringify(changes)) return reply({ configured: true, id, updated: false });
  if (!event) return reply({ error: "service_unavailable" }, 503);

  const body = { ...changes };
  if (changes.detail) {
    const detail = object(row.detail) ? row.detail : {};
    // The API requires both fields when updating `detail`. Preserve the unselected value from this read.
    body.detail = { details: string(detail, "details"), extraDetails: string(detail, "extraDetails"), ...changes.detail };
  }
  const upstreamEtag = current.headers.get("ETag");
  const etag = upstreamEtag?.startsWith('"') ? upstreamEtag : undefined;
  let result: Response;
  try { result = await merchant(path, "PUT", body, etag); }
  catch { return reply({ error: "ideasoft_write_unconfirmed" }, 502); }
  if (result.status === 409 || result.status === 412) return stalePreview();
  if (result.status === 400 || result.status === 422) return reply({ error: "ideasoft_seo_rejected" }, 422);
  if (result.status >= 500) {
    return reply({ error: "ideasoft_write_unconfirmed" }, 502);
  }
  if (!result.ok) return merchantError(result.status);

  const data = { eventKey: event.key, sku: input.sku!.trim(), id, pushedAt: new Date().toISOString() };
  const recorded = await event.record(data.sku, data.eventKey, data);
  if (recorded) return recordedPush(recorded);
  const receipt = await signReceipt(data, input.workspace.toLowerCase(), workspaceUser, event.receiptKey);
  return pendingPush(data, receipt);
}

export async function handleIdeasoft(req: Request, env: Env, request: typeof fetch = fetch): Promise<Response> {
  if (req.method === "OPTIONS") return new Response(null, { headers: cors });
  if (req.method !== "POST") return reply({ error: "method_not_allowed" }, 405);
  const authorization = req.headers.get("Authorization");
  if (!authorization?.startsWith("Bearer ")) return reply({ error: "sign_in_required" }, 401);
  let input: Input, path: string, changes: ProductSeoChanges | undefined, rotation: Rotation | undefined, pageAction: PageMutation | undefined;
  try {
    input = await readInput(req);
    path = ["product_push_receipt", "page_receipt"].includes(input.operation) ? "" : target(input);
    pageAction = readPageMutation(input);
    if (input.operation === "product_preview" || input.operation === "product_push") changes = selectedChanges(input.changes);
    if (input.operation === "product_push" && (typeof input.confirmation !== "string" || !/^\d{13}\.[0-9a-f]{64}$/.test(input.confirmation))) throw new Error();
    if (input.operation === "product_push_receipt" && (typeof input.receipt !== "string" || !receiptPattern.test(input.receipt))) throw new Error();
    if (input.operation === "page_receipt" && (typeof input.receipt !== "string" || !pageReceiptPattern.test(input.receipt) ||
        Object.keys(input).some((key) => !["workspace", "operation", "receipt"].includes(key)))) throw new Error();
    if (input.operation === "rotate_token") {
      if (Object.keys(input).some((key) => !["workspace", "operation", "token", "origin", "replace_valid"].includes(key))) throw new Error();
      const token = bearerToken(input.token);
      if (!token || typeof input.replace_valid !== "boolean" || typeof input.origin !== "string") throw new Error();
      const claimedOrigin = input.origin.trim();
      if (!claimedOrigin || !/^[a-z\d]/i.test(claimedOrigin)) throw new Error();
      const base = origin(claimedOrigin.includes("://") ? claimedOrigin : `https://${claimedOrigin}`);
      rotation = { token, origin: base, replaceValid: input.replace_valid };
    }
  } catch { return reply({ error: "invalid_request" }, 400); }

  const supabase = env("SUPABASE_URL"), apikey = env("SUPABASE_ANON_KEY");
  if (!supabase || !apikey) return reply({ error: "service_unavailable" }, 503);
  try {
    const headers = { apikey, Authorization: authorization };
    const auth = await request(`${supabase}/auth/v1/user`, { headers, signal: signal() });
    if (!auth.ok) return reply({ error: auth.status >= 500 ? "auth_unavailable" : "sign_in_required" }, auth.status >= 500 ? 503 : 401);
    const user = await auth.json();
    if (!object(user) || typeof user.id !== "string" || !workspaceId.test(user.id)) return reply({ error: "sign_in_required" }, 401);
    const filters = new URLSearchParams({ select: "role", workspace_id: `eq.${input.workspace}`, user_id: `eq.${user.id}`, limit: "1" });
    const membership = await request(`${supabase}/rest/v1/workspace_members?${filters}`, { headers, signal: signal() });
    if (!membership.ok) return reply({ error: "membership_unavailable" }, 503);
    const members = await membership.json();
    if (!Array.isArray(members)) return reply({ error: "membership_unavailable" }, 503);
    const role = members[0]?.role;
    if (role !== "owner" && role !== "editor") return reply({ error: "integration_access_denied" }, 403);
    if (rotation && role !== "owner") return reply({ error: "ideasoft_rotation_owner_required" }, 403);

    const workspace = input.workspace.toLowerCase();
    const serviceKey = serviceSecret(env);
    if (input.operation === "page_receipt") {
      if (!serviceKey) return reply({ error: "service_unavailable" }, 503);
      return await recoverPageReceipt(input.receipt as string, serviceRpc(supabase, serviceKey, request), workspace, user.id, serviceKey);
    }
    if (input.operation === "product_push_receipt") {
      if (!serviceKey) return reply({ error: "service_unavailable" }, 503);
      const receipt = input.receipt as string;
      const data = await readReceipt(receipt, workspace, user.id, await signingKey(serviceKey, "product-receipt"));
      if (!data) return reply({ error: "invalid_ideasoft_receipt" }, 409);
      const recorded = await eventRecorder(supabase, serviceKey, workspace, request)(data.sku, data.eventKey, data);
      return recorded ? recordedPush(recorded) : pendingPush(data, receipt);
    }

    const value = env(`TEAM_IDEASOFT_${input.workspace.toUpperCase().replaceAll("-", "_")}`);
    if (!value) return reply({ configured: false });
    let base: string, fallbackToken: string | null;
    try {
      const cfg: unknown = JSON.parse(value);
      if (!object(cfg)) throw new Error();
      base = origin(cfg.origin);
      fallbackToken = bearerToken(cfg.token);
    } catch { return reply({ error: "invalid_configuration" }, 503); }
    if (rotation && rotation.origin !== base) return reply({ error: "ideasoft_rotation_origin_mismatch" }, 409);
    if (!serviceKey) return reply({ error: "ideasoft_token_store_unavailable" }, 503);
    let event: PushEvent | undefined;
    if (changes) {
      const sku = input.sku!.trim();
      if (input.operation === "product_push") {
        const identity = encoder.encode(JSON.stringify(["ideasoft-push:v1", workspace, user.id, sku, changes, input.confirmation]));
        const key = `ideasoft:${hex(await crypto.subtle.digest("SHA-256", identity))}`;
        const record = eventRecorder(supabase, serviceKey, workspace, request);
        const known = await record(sku, key);
        if (known) return recordedPush(known);
        if (known === undefined) return reply({ error: "ideasoft_receipt_unavailable" }, 503);
        event = { key, record, receiptKey: await signingKey(serviceKey, "product-receipt") };
      }
      const filters = new URLSearchParams({ select: "sku", workspace_id: `eq.${workspace}`, sku: `eq.${sku}`, active: "eq.true", limit: "1" });
      const catalog = await request(`${supabase}/rest/v1/team_catalog?${filters}`, { headers, signal: signal() });
      if (!catalog.ok) return reply({ error: "catalog_unavailable" }, 503);
      const products: unknown = await catalog.json();
      if (!Array.isArray(products)) return reply({ error: "catalog_unavailable" }, 503);
      if (products[0]?.sku !== sku) return reply({ error: "ideasoft_catalog_product_missing" }, 409);
    }
    const rpc = serviceRpc(supabase, serviceKey, request);
    const stored = await readStoredToken(rpc, workspace, base);
    if (stored === undefined) return reply({ error: "ideasoft_token_store_unavailable" }, 503);
    if (rotation) return await rotateToken(rpc, workspace, user.id, base, rotation, stored, fallbackToken, request);

    const token = stored?.token ?? fallbackToken;
    if (!token) return reply({ configured: false });
    // Do not forward the merchant credential through redirects or expose upstream error bodies.
    const merchant = merchantRequest(base, token, request);
    if (pageAction) {
      const record = pageRecorder(rpc, workspace, user.id, serviceKey);
      if (pageAction.operation === "page_create") return await withPageCreateClaim(rpc, workspace, user.id, pageAction,
        () => handlePageCreate(path, pageAction.kind, pageAction.payload, merchant, record));
      const readShared = sharedPageReader(supabase, headers, workspace, pageAction.kind, pageAction.id, request);
      return await handlePageUpdate(path, pageAction, merchant, readShared, record);
    }
    const remote = await merchant(path);
    if (!remote.ok) return merchantError(remote.status);
    if (changes) return await handleProductWrite(input, user.id, token, changes, remote, merchant, event);
    const data: unknown = await remote.json();
    if (input.operation === "product") {
      if (!object(data) || data.id !== input.id) return reply({ error: "invalid_ideasoft_response" }, 502);
      return reply({ configured: true, product: product(data) });
    }
    if (!Array.isArray(data) || !data.every(object)) return reply({ error: "invalid_ideasoft_response" }, 502);
    if (input.operation === "health") return reply({ configured: true, connected: true });
    if (input.operation === "resolve") {
      const row = data.find((row) => normalizedSku(string(row, "sku")) === normalizedSku(input.sku!));
      return reply({ configured: true, product: row ? product(row) : null });
    }
    if (data.length > 100 || data.some((row) => !positiveId(row.id))) return reply({ error: "invalid_ideasoft_response" }, 502);
    return reply({ configured: true, has_more: data.length === 100, pages: data.map((row) => ({
      kind: input.kind, id: row.id, slug: string(row, "slug").trim(),
      name: string(row, input.kind === "blog" ? "title" : "name").trim(),
      page_title: string(row, "pageTitle"), meta_description: string(row, "metaDescription"),
      meta_keywords: string(row, "metaKeywords"), target_keyword: string(row, "targetKeyword"),
      showcase_content: typeof row.showcaseContent === "string" ? row.showcaseContent : null,
      status: Number.isSafeInteger(row.status) ? row.status : 1,
    })) });
  } catch { return reply({ error: "ideasoft_service_unavailable" }, 502); }
}
