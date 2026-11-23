import { timingSafeEqual } from "node:crypto";

const publisherPath = "/team-catalog-publisher";
const workspaceId = /^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/i;
const tokenFormat = /^[a-f0-9]{64}$/;
const maxSnapshotBytes = 4 * 1024 * 1024;
const encoder = new TextEncoder();

type Env = (name: string) => string | undefined;
type JsonObject = Record<string, unknown>;
type Snapshot = { expected_revision: number | null; products: JsonObject[] };

class BodyLimitError extends Error {}

function json(body: unknown, status = 200, extra: Record<string, string> = {}) {
  return Response.json(body, { status, headers: { "Cache-Control": "no-store", ...extra } });
}

function matchesToken(provided: string | null, expected: string | undefined): boolean {
  return typeof provided === "string" && typeof expected === "string" &&
    tokenFormat.test(provided) && tokenFormat.test(expected) &&
    timingSafeEqual(encoder.encode(provided), encoder.encode(expected));
}

function object(value: unknown): value is JsonObject {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function revision(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value > 0;
}

function serverKey(env: Env): string | undefined {
  try {
    const keys = JSON.parse(env("SUPABASE_SECRET_KEYS") ?? "{}");
    if (typeof keys?.default === "string" && keys.default.startsWith("sb_secret_")) return keys.default;
  } catch { /* Legacy projects expose their service-role JWT separately. */ }
  return env("SUPABASE_SERVICE_ROLE_KEY");
}

async function readJson(source: Request | Response, maxBytes: number): Promise<unknown> {
  if (Number(source.headers.get("content-length")) > maxBytes) throw new BodyLimitError();
  const reader = source.body?.getReader();
  if (!reader) throw new Error("Missing body");
  const decoder = new TextDecoder("utf-8", { fatal: true });
  let bytes = 0;
  let content = "";
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      bytes += value.byteLength;
      if (bytes > maxBytes) {
        void reader.cancel().catch(() => {});
        throw new BodyLimitError();
      }
      content += decoder.decode(value, { stream: true });
    }
    return JSON.parse(content + decoder.decode());
  } finally {
    reader.releaseLock();
  }
}

function snapshot(value: unknown, minimum: number): Snapshot | null {
  if (!object(value) || Object.keys(value).length !== 2 ||
      (value.expected_revision !== null && !revision(value.expected_revision)) ||
      !Array.isArray(value.products) || value.products.length < minimum) return null;

  const seen = new Set<string>();
  for (const product of value.products) {
    if (!object(product) || Object.keys(product).length !== 3 ||
        typeof product.sku !== "string" || typeof product.name !== "string" ||
        !product.sku.trim() || !product.name.trim() || !object(product.facts) ||
        !Object.values(product.facts).every((fact) =>
          fact === null || typeof fact === "string" || (typeof fact === "number" && Number.isFinite(fact)))) return null;
    const sku = product.sku.trim();
    if (seen.has(sku)) return null;
    seen.add(sku);
  }
  return { expected_revision: value.expected_revision, products: value.products };
}

export async function handleCatalogPublisher(request: Request, env: Env, transport: typeof fetch = fetch): Promise<Response> {
  const { pathname, search } = new URL(request.url);
  if ((pathname !== publisherPath && pathname !== `/functions/v1${publisherPath}`) || search) {
    return json({ error: "not_found" }, 404);
  }
  if (!matchesToken(request.headers.get("x-catalog-publisher-token"), env("TEAM_CATALOG_PUBLISHER_TOKEN"))) {
    return json({ error: "publisher_auth_required" }, 401);
  }
  if (request.method !== "GET" && request.method !== "POST") {
    return json({ error: "method_not_allowed" }, 405, { Allow: "GET, POST" });
  }

  const workspace = env("TEAM_CATALOG_WORKSPACE_ID");
  const url = env("SUPABASE_URL")?.replace(/\/+$/, "");
  const key = serverKey(env);
  const minimum = Number(env("TEAM_CATALOG_MIN_PRODUCTS") ?? 1);
  const unavailable = () => json({ error: "catalog_publisher_unavailable" }, 503);
  if (!workspace || !workspaceId.test(workspace) || !url || !key ||
      !Number.isSafeInteger(minimum) || minimum < 1) return unavailable();

  const headers = {
    apikey: key,
    ...(!key.startsWith("sb_secret_") ? { Authorization: `Bearer ${key}` } : {}),
  };
  const options = { redirect: "error" as const, signal: AbortSignal.timeout(20_000), headers };
  if (request.method === "GET") {
    const params = new URLSearchParams({ workspace_id: `eq.${workspace}`, select: "revision", limit: "1" });
    try {
      const response = await transport(`${url}/rest/v1/team_catalog_state?${params}`, options);
      if (!response.ok) return unavailable();
      const rows = await readJson(response, 4096);
      if (!Array.isArray(rows)) return unavailable();
      if (rows.length === 0) return json({ revision: null });
      if (rows.length === 1 && revision(rows[0]?.revision)) return json({ revision: rows[0].revision });
    } catch { /* Leave stored catalog intact when the baseline cannot be read. */ }
    return unavailable();
  }

  let candidate: Snapshot | null;
  try {
    candidate = snapshot(await readJson(request, maxSnapshotBytes), minimum);
  } catch (error) {
    return error instanceof BodyLimitError ? json({ error: "snapshot_too_large" }, 413)
      : json({ error: "invalid_catalog_snapshot" }, 400);
  }
  if (!candidate) return json({ error: "invalid_catalog_snapshot" }, 400);

  try {
    const response = await transport(`${url}/rest/v1/rpc/team_replace_catalog`, {
      ...options, method: "POST", headers: { ...headers, "Content-Type": "application/json" },
      body: JSON.stringify({ target_workspace: workspace, ...candidate }),
    });
    if (!response.ok) {
      const failure = await readJson(response, 4096).catch(() => null);
      if (response.status === 409 || (object(failure) && failure.code === "40001")) {
        return json({ error: "catalog_revision_conflict" }, 409);
      }
      return unavailable();
    }
    const updated = await readJson(response, 4096);
    if (revision(updated)) return json(updated);
  } catch { /* Hide upstream details, including any keys in gateway errors. */ }
  return unavailable();
}
