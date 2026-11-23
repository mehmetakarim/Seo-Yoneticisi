const googleModels = "https://generativelanguage.googleapis.com/v1beta/models";
const workspaceId = /^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/i;
const modelId = /^(?:gemini|gemma)-(?=[a-z0-9.-]{1,77}$)(?![a-z0-9.-]*-\.)[a-z0-9][a-z0-9-]*(?:\.[a-z0-9][a-z0-9-]*)*$/;
const maxInputBytes = 256 * 1024;
const maxResponseBytes = 1024 * 1024;
const maxOutputTokens = 8192;
const maxVaultVersion = 2_147_483_647;
const apiKeyPattern = /^[\x21-\x7e]{20,1024}$/;
// The initial allowlist follows seo_core::gemini::DEFAULT_MODEL_CHAIN.
const defaultModels = [
  "gemini-3.6-flash", "gemini-3.5-flash", "gemini-2.5-flash",
  "gemini-3.5-flash-lite", "gemini-3.1-flash-lite", "gemini-2.5-flash-lite",
  "gemini-flash-latest", "gemma-4-31b-it",
];
const cors = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Headers": "authorization, apikey, content-type, x-client-info",
  "Access-Control-Allow-Methods": "POST, OPTIONS",
  "Cache-Control": "no-store",
};

type Env = (name: string) => string | undefined;
type JsonObject = Record<string, unknown>;
type TextRequest = { contents: unknown[]; system_instruction?: unknown; generationConfig: JsonObject };
type Operation = { workspace: string; operation: "health" } |
  { workspace: string; operation: "configure"; apiKey: string } |
  { workspace: string; operation: "generate"; model: string; request: TextRequest };
type GeminiConfig = { apiKey: string; models: string[] };
type VaultConfig = GeminiConfig & { version: number };
type Member = { id: string; role: "owner" | "editor" };
type VaultRpc = (name: string, args: JsonObject) => Promise<Response>;

const reply = (body: object, status = 200) => Response.json(body, { status, headers: cors });
const object = (value: unknown): value is JsonObject => !!value && typeof value === "object" && !Array.isArray(value);
const only = (value: JsonObject, keys: string[]) => Object.keys(value).every((key) => keys.includes(key));

class BodyLimitError extends Error {}

async function readJson(source: Request | Response, limit: number): Promise<unknown> {
  const reader = source.body?.getReader();
  if (!reader) throw new Error("Missing JSON body");
  const decoder = new TextDecoder("utf-8", { fatal: true });
  let bytes = 0;
  let text = "";
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      bytes += value.byteLength;
      if (bytes > limit) {
        void reader.cancel().catch(() => {});
        throw new BodyLimitError();
      }
      text += decoder.decode(value, { stream: true });
    }
  } finally {
    reader.releaseLock();
  }
  return JSON.parse(text + decoder.decode());
}

function content(value: unknown, system = false): boolean {
  return object(value) && only(value, system ? ["parts"] : ["role", "parts"]) &&
    (value.role === undefined || value.role === "user" || value.role === "model") &&
    Array.isArray(value.parts) && value.parts.length > 0 && value.parts.length <= 8 &&
    value.parts.every((part) => object(part) && only(part, ["text"]) &&
      typeof part.text === "string" && part.text.length > 0);
}

// The current Rust callers use this structured JSON subset for generated copy and tables.
function responseSchema(value: unknown, depth = 0): boolean {
  if (!object(value) || depth > 8) return false;
  const description = value.description;
  if (description !== undefined && (typeof description !== "string" || description.length > 2000)) return false;
  if (value.type === "STRING") return only(value, ["type", "description"]);
  if (value.type === "ARRAY") {
    return only(value, ["type", "description", "items"]) && responseSchema(value.items, depth + 1);
  }
  if (value.type !== "OBJECT" || !only(value, ["type", "description", "properties", "required"]) ||
      !object(value.properties)) return false;
  const entries = Object.entries(value.properties);
  const required = value.required ?? [];
  return entries.length > 0 && entries.length <= 32 &&
    entries.every(([key, child]) => /^[a-z][a-z0-9_]{0,63}$/i.test(key) && responseSchema(child, depth + 1)) &&
    Array.isArray(required) && required.length <= entries.length &&
    required.every((key) => typeof key === "string" && Object.hasOwn(value.properties as JsonObject, key));
}

function generationRequest(value: unknown): TextRequest | null {
  if (!object(value) || !only(value, ["contents", "system_instruction", "generationConfig"]) ||
      !Array.isArray(value.contents) || !value.contents.length || value.contents.length > 16 ||
      !value.contents.every((turn) => content(turn)) ||
      (value.system_instruction !== undefined && !content(value.system_instruction, true))) return null;
  const config = value.generationConfig;
  if (!object(config) || !only(config, ["temperature", "responseMimeType", "responseSchema", "maxOutputTokens"]) ||
      config.responseMimeType !== "application/json" || !responseSchema(config.responseSchema)) return null;
  const temperature = config.temperature;
  if (temperature !== undefined && (typeof temperature !== "number" || temperature < 0 || temperature > 2)) return null;
  const maxTokens = config.maxOutputTokens ?? maxOutputTokens;
  if (typeof maxTokens !== "number" || !Number.isInteger(maxTokens) || maxTokens < 1 || maxTokens > maxOutputTokens) return null;
  return {
    contents: value.contents,
    ...(value.system_instruction === undefined ? {} : { system_instruction: value.system_instruction }),
    generationConfig: { ...config, maxOutputTokens: maxTokens },
  };
}

function operation(value: unknown): Operation | null {
  if (!object(value) || typeof value.workspace !== "string" || !workspaceId.test(value.workspace)) return null;
  const workspace = value.workspace.toLowerCase();
  if (value.operation === "health" && only(value, ["workspace", "operation"])) return { workspace, operation: "health" };
  if (value.operation === "configure" && only(value, ["workspace", "operation", "api_key"]) &&
      typeof value.api_key === "string" && apiKeyPattern.test(value.api_key.trim())) {
    return { workspace, operation: "configure", apiKey: value.api_key.trim() };
  }
  if (value.operation !== "generate" || !only(value, ["workspace", "operation", "model", "request"]) ||
      typeof value.model !== "string" || !modelId.test(value.model)) return null;
  const request = generationRequest(value.request);
  return request ? { workspace, operation: "generate", model: value.model, request } : null;
}

function configuration(config: unknown): GeminiConfig | null {
  if (!object(config) || !only(config, ["api_key", "models"]) || typeof config.api_key !== "string" ||
      !apiKeyPattern.test(config.api_key.trim()) || !Array.isArray(config.models) ||
      !config.models.length || config.models.length > 32 || !config.models.every((model) =>
        typeof model === "string" && modelId.test(model))) return null;
  return { apiKey: config.api_key.trim(), models: [...new Set<string>(config.models)] };
}

function legacyConfiguration(secret: string | undefined): GeminiConfig | null {
  try { return secret ? configuration(JSON.parse(secret)) : null; } catch { return null; }
}

async function workspaceMember(
  url: string, headers: HeadersInit, workspace: string, request: typeof fetch, timeout: number,
): Promise<Member | Response> {
  const options = { headers, redirect: "error" as const, signal: AbortSignal.timeout(timeout) };
  try {
    const auth = await request(`${url}/auth/v1/user`, options);
    if (!auth.ok) return auth.status === 400 || auth.status === 401 || auth.status === 403
      ? reply({ error: "sign_in_required" }, 401)
      : reply({ error: "auth_unavailable" }, 503);
    const user = await readJson(auth, 64 * 1024);
    if (!object(user) || typeof user.id !== "string" || !workspaceId.test(user.id)) {
      return reply({ error: "sign_in_required" }, 401);
    }
    const params = new URLSearchParams({ select: "role", workspace_id: `eq.${workspace}`, user_id: `eq.${user.id}`, limit: "1" });
    const membership = await request(`${url}/rest/v1/workspace_members?${params}`, options);
    if (!membership.ok) return reply({ error: "membership_unavailable" }, 503);
    const members = await readJson(membership, 8 * 1024);
    if (!Array.isArray(members)) return reply({ error: "membership_unavailable" }, 503);
    const role = members[0]?.role;
    return role === "owner" || role === "editor" ? { id: user.id, role } : reply({ error: "integration_access_denied" }, 403);
  } catch {
    return reply({ error: "membership_unavailable" }, 503);
  }
}

function projectKey(env: Env, kind: "publishable" | "secret"): string | undefined {
  try {
    const keys = JSON.parse(env(`SUPABASE_${kind.toUpperCase()}_KEYS`) ?? "{}");
    if (typeof keys?.default === "string" && keys.default.startsWith(`sb_${kind}_`)) return keys.default;
  } catch { /* Older projects expose legacy keys separately. */ }
  return env(kind === "secret" ? "SUPABASE_SERVICE_ROLE_KEY" : "SUPABASE_ANON_KEY");
}

function vaultRpc(url: string, key: string, request: typeof fetch): VaultRpc {
  const headers = { apikey: key, "Content-Type": "application/json",
    ...(!key.startsWith("sb_secret_") ? { Authorization: `Bearer ${key}` } : {}) };
  return (name, args) => request(`${url}/rest/v1/rpc/${name}`, {
    method: "POST", headers, body: JSON.stringify(args), redirect: "error", signal: AbortSignal.timeout(5_000),
  });
}

function unavailableVault(): Response {
  return reply({ error: "gemini_credentials_store_unavailable" }, 503);
}

async function readVaultConfig(rpc: VaultRpc, workspace: string): Promise<VaultConfig | null | undefined> {
  try {
    const response = await rpc("team_get_gemini_settings", { target_workspace: workspace });
    if (!response.ok) return undefined;
    const saved = await readJson(response, 8 * 1024);
    if (saved === null) return null;
    if (!object(saved) || !only(saved, ["api_key", "models", "version"]) || typeof saved.version !== "number" ||
        !Number.isInteger(saved.version) || saved.version < 1 || saved.version > maxVaultVersion) return undefined;
    const config = configuration({ api_key: saved.api_key, models: saved.models });
    return config ? { ...config, version: saved.version } : undefined;
  } catch { return undefined; }
}

async function availableGenerationModels(apiKey: string, request: typeof fetch): Promise<Set<string> | Response> {
  const available = new Set<string>();
  const seenPages = new Set<string>();
  const signal = AbortSignal.timeout(8_000);
  const params = new URLSearchParams({ pageSize: "200" });
  try {
    for (let page = 0; page < 5; page++) {
      const response = await request(`${googleModels}?${params}`, {
        method: "GET", headers: { "x-goog-api-key": apiKey, accept: "application/json" },
        redirect: "error", signal,
      });
      if (!response.ok) return geminiFailure(response, false);
      const data = await readJson(response, maxResponseBytes);
      if (!object(data) || !Array.isArray(data.models)) return reply({ error: "invalid_gemini_response" }, 502);
      for (const model of data.models) {
        if (!object(model) || !Array.isArray(model.supportedGenerationMethods) ||
            !model.supportedGenerationMethods.includes("generateContent")) continue;
        const name = typeof model.name === "string" && model.name.startsWith("models/") ? model.name.slice(7) : "";
        if (modelId.test(name)) available.add(name);
        if (typeof model.baseModelId === "string" && modelId.test(model.baseModelId)) available.add(model.baseModelId);
      }
      const token = data.nextPageToken;
      if (token === undefined || token === "") return available;
      if (typeof token !== "string" || token.length > 4096 || seenPages.has(token)) break;
      seenPages.add(token);
      params.set("pageToken", token);
    }
    return reply({ error: "invalid_gemini_response" }, 502);
  } catch {
    return reply({ error: "gemini_unavailable" }, 502);
  }
}

async function configureGemini(
  input: Extract<Operation, { operation: "configure" }>, member: Member, secret: string | undefined,
  rpc: VaultRpc, request: typeof fetch,
): Promise<Response> {
  // Use the version before provider validation so a concurrent rotation cannot be overwritten.
  const saved = await readVaultConfig(rpc, input.workspace);
  if (saved === undefined) return unavailableVault();
  const version = saved?.version ?? 0;
  if (version === maxVaultVersion) return unavailableVault();

  const available = await availableGenerationModels(input.apiKey, request);
  if (available instanceof Response) return available;
  const policy = saved?.models ?? legacyConfiguration(secret)?.models ?? defaultModels;
  const models = policy.filter((model) => available.has(model));
  if (models.length === 0) return reply({ error: "gemini_no_compatible_models" }, 422);

  try {
    const response = await rpc("team_set_gemini_settings", {
      target_workspace: input.workspace, target_owner: member.id, new_api_key: input.apiKey,
      allowed_models: policy, expected_version: version,
    });
    if (response.status === 409) return reply({ error: "gemini_configuration_conflict" }, 409);
    if (response.status === 403) return reply({ error: "gemini_configuration_owner_required" }, 403);
    if (!response.ok) return unavailableVault();
    const result = await readJson(response, 8 * 1024);
    if (object(result) && result.version === version + 1) {
      return reply({ configured: true, connected: true, models, version: version + 1 });
    }
  } catch { /* Provider credentials and service responses stay inside the function. */ }
  return unavailableVault();
}

function upstreamError(error: string, status: number, upstreamStatus: number, retry = false) {
  return reply({ error, upstream_status: upstreamStatus, retry_with_another_model: retry }, status);
}

async function geminiFailure(response: Response, generating: boolean) {
  const status = response.status;
  if (status === 401 || status === 403) return upstreamError("gemini_credentials_rejected", 502, status);
  if (status === 400) {
    const data = await readJson(response, 64 * 1024).catch(() => null);
    const error = object(data) && object(data.error) ? data.error : null;
    const details = Array.isArray(error?.details) ? error.details : [];
    const invalidKey = details.some((entry) => object(entry) &&
      ["API_KEY_INVALID", "API_KEY_EXPIRED", "API_KEY_SERVICE_BLOCKED"].includes(String(entry.reason)));
    if (invalidKey || !generating) return upstreamError("gemini_credentials_rejected", 502, status);
    return upstreamError("gemini_request_rejected", 400, status, true);
  }
  if (status === 404 && generating) return upstreamError("gemini_model_unavailable", 404, status, true);
  if (status === 429) return upstreamError("gemini_rate_limited", 429, status, generating);
  return upstreamError("gemini_unavailable", 502, status, generating && status >= 500);
}

function generatedContent(data: unknown): Response {
  if (!object(data)) return reply({ error: "invalid_gemini_response" }, 502);
  if (!Array.isArray(data.candidates)) {
    if (object(data.promptFeedback) && typeof data.promptFeedback.blockReason === "string") {
      return reply({ error: "gemini_prompt_blocked" }, 422);
    }
    return reply({ error: "invalid_gemini_response" }, 502);
  }
  const candidates = data.candidates.filter(object).map((entry) => {
    const parts: unknown[] = object(entry.content) && Array.isArray(entry.content.parts) ? entry.content.parts : [];
    const text = parts.filter((part): part is JsonObject & { text: string } =>
      object(part) && part.thought !== true && typeof part.text === "string").map((part) => part.text).join("");
    return { content: { parts: text ? [{ text }] : [] }, ...(typeof entry.finishReason === "string" ? { finishReason: entry.finishReason } : {}) };
  });
  // Keep the provider's candidate envelope for the existing Rust parser; omit thoughts and unrelated metadata.
  return reply({ candidates });
}

export async function handleGemini(req: Request, env: Env, request: typeof fetch = fetch): Promise<Response> {
  if (req.method === "OPTIONS") return new Response(null, { headers: cors });
  if (req.method !== "POST") return reply({ error: "method_not_allowed" }, 405);
  const authorization = req.headers.get("Authorization");
  if (!authorization?.startsWith("Bearer ")) return reply({ error: "sign_in_required" }, 401);

  let input: Operation | null;
  try {
    input = operation(await readJson(req, maxInputBytes));
  } catch (error) {
    return error instanceof BodyLimitError ? reply({ error: "request_too_large" }, 413) : reply({ error: "invalid_request" }, 400);
  }
  if (!input) return reply({ error: "invalid_request" }, 400);

  const supabase = env("SUPABASE_URL");
  const apikey = projectKey(env, "publishable");
  if (!supabase || !apikey) return reply({ error: "service_unavailable" }, 503);
  const member = await workspaceMember(supabase, { apikey, Authorization: authorization }, input.workspace, request,
    input.operation === "configure" ? 6_000 : 8_000);
  if (member instanceof Response) return member;

  const secret = env(`TEAM_GEMINI_${input.workspace.toUpperCase().replaceAll("-", "_")}`);
  if (input.operation === "configure" && member.role !== "owner") {
    return reply({ error: "gemini_configuration_owner_required" }, 403);
  }
  const serviceKey = projectKey(env, "secret");
  if (!serviceKey) return unavailableVault();
  const rpc = vaultRpc(supabase, serviceKey, request);
  if (input.operation === "configure") {
    return configureGemini(input, member, secret, rpc, request);
  }
  const saved = await readVaultConfig(rpc, input.workspace);
  if (saved === undefined) return unavailableVault();
  if (!saved && !secret) return input.operation === "health"
    ? reply({ configured: false })
    : reply({ error: "gemini_not_configured" }, 503);
  const config = saved ?? legacyConfiguration(secret);
  if (!config) return reply({ error: "invalid_configuration" }, 503);
  if (input.operation === "health") {
    const available = await availableGenerationModels(config.apiKey, request);
    if (available instanceof Response) return available;
    const models = config.models.filter((model) => available.has(model));
    if (models.length === 0) return reply({ error: "gemini_no_compatible_models" }, 422);
    return reply({ configured: true, connected: true, models });
  }

  if (!config.models.includes(input.model)) {
    return reply({ error: "gemini_model_not_allowed", retry_with_another_model: true }, 400);
  }
  const headers = { "x-goog-api-key": config.apiKey, "content-type": "application/json", accept: "application/json" };
  try {
    const remote = await request(`${googleModels}/${input.model}:generateContent`, {
      method: "POST",
      headers,
      redirect: "error",
      signal: AbortSignal.timeout(45_000),
      body: JSON.stringify(input.request),
    });
    if (!remote.ok) return geminiFailure(remote, true);
    return generatedContent(await readJson(remote, maxResponseBytes));
  } catch {
    return reply({ error: "gemini_unavailable" }, 502);
  }
}
