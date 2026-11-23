import assert from "node:assert/strict";
import { test } from "node:test";
import { handleGemini } from "./handler.ts";

const workspace = "11111111-1111-4111-8111-111111111111";
const otherWorkspace = "22222222-2222-4222-8222-222222222222";
const user = "33333333-3333-4333-8333-333333333333";
const model = "gemini-flash-latest";
const apiKey = "synthetic-server-credential";
const candidateKey = "synthetic-owner-candidate-credential";
const serviceApiKey = "sb_secret_synthetic-test-server";
const base = "https://generativelanguage.googleapis.com/v1beta/models";
const sample = {
  system_instruction: { parts: [{ text: "Return only JSON." }] },
  contents: [{ parts: [{ text: "Create a title for Ürün." }] }],
  generationConfig: {
    responseMimeType: "application/json",
    responseSchema: { type: "OBJECT", properties: { title: { type: "STRING" } }, required: ["title"] },
    temperature: 0.6,
  },
};

type FixtureOptions = {
  role?: string | null;
  authStatus?: number;
  providerStatus?: number;
  providerBody?: object;
  secret?: string | null;
  missingVaultKey?: boolean;
  useModernPublishableKey?: boolean;
  vault?: { stored?: unknown; getStatus?: number; setStatus?: number; legacyServiceKey?: boolean };
  catalog?: (url: URL) => Response;
};
function fixture(options: FixtureOptions = {}) {
  const calls: { url: string; init?: RequestInit }[] = [];
  let stored: unknown = options.vault?.stored ?? null;
  const env: Record<string, string | undefined> = {
    SUPABASE_URL: "https://workspace.supabase.co",
    SUPABASE_ANON_KEY: "public-project-key",
    [`TEAM_GEMINI_${workspace.replaceAll("-", "_")}`]: options.secret === null ? undefined
      : options.secret ?? JSON.stringify({ api_key: apiKey, models: [model] }),
  };
  if (options.useModernPublishableKey) {
    env.SUPABASE_PUBLISHABLE_KEYS = JSON.stringify({ default: "sb_publishable_synthetic-test" });
    env.SUPABASE_ANON_KEY = undefined;
  }
  if (options.vault?.legacyServiceKey) env.SUPABASE_SERVICE_ROLE_KEY = "legacy.synthetic.service-key";
  else if (!options.missingVaultKey) env.SUPABASE_SECRET_KEYS = JSON.stringify({ default: serviceApiKey });
  const transport: typeof fetch = async (input, init) => {
    const url = String(input);
    calls.push({ url, init });
    if (url.endsWith("/auth/v1/user")) return Response.json({ id: user }, { status: options.authStatus ?? 200 });
    if (url.includes("/rest/v1/workspace_members?")) {
      return Response.json(options.role === null ? [] : [{ role: options.role ?? "editor" }]);
    }
    if (url.endsWith("/rpc/team_get_gemini_settings")) {
      return Response.json(stored, { status: options.vault?.getStatus ?? 200 });
    }
    if (url.endsWith("/rpc/team_set_gemini_settings")) {
      if (options.vault?.setStatus) return Response.json({ upstream: candidateKey }, { status: options.vault.setStatus });
      const args = JSON.parse(String(init?.body));
      const version = args.expected_version + 1;
      stored = { api_key: args.new_api_key, models: args.allowed_models, version };
      return Response.json({ version, api_key: candidateKey });
    }
    if (url.startsWith(`${base}?pageSize=200`) && options.catalog) return options.catalog(new URL(url));
    return Response.json(options.providerBody ?? (url.startsWith(`${base}?pageSize=200`) ? { models: [aiModel(model)] } : {
      candidates: [{ content: { parts: [{ text: "hidden thought", thought: true },
        { text: '{"title":' }, { text: '"Ürün"}' }] }, finishReason: "STOP" }],
      privateMetadata: apiKey,
    }), { status: options.providerStatus ?? 200 });
  };
  const run = (body: object, auth = true) => handleGemini(new Request("https://workspace.supabase.co/functions/v1/team-gemini", {
    method: "POST", headers: auth ? { Authorization: "Bearer test-session" } : {},
    body: JSON.stringify({ workspace, ...body }),
  }), (name) => env[name], transport);
  return { calls, run };
}

const configure = { operation: "configure", api_key: candidateKey };
const aiModel = (id: string, method = "generateContent") => ({ name: `models/${id}`, supportedGenerationMethods: [method] });
const googleCalls = (f: ReturnType<typeof fixture>) => f.calls.filter((call) => call.url.startsWith(base));
const vaultWrites = (f: ReturnType<typeof fixture>) => f.calls.filter((call) => call.url.endsWith("/rpc/team_set_gemini_settings"));

test("unauthenticated, stale, viewer and nonmember requests cannot reach Google", async () => {
  const unsigned = fixture();
  assert.equal((await unsigned.run({ operation: "health" }, false)).status, 401);
  assert.equal(unsigned.calls.length, 0);
  for (const [options, status, callCount] of [
    [{ authStatus: 401 }, 401, 1], [{ role: "viewer" }, 403, 2], [{ role: null }, 403, 2],
  ] as const) {
    const f = fixture(options);
    const response = await f.run({ operation: "generate", model, request: sample });
    assert.equal(response.status, status);
    assert.equal(f.calls.length, callCount);
  }
});

test("a workspace cannot inherit another workspace's provider secret", async () => {
  const f = fixture();
  const response = await f.run({ workspace: otherWorkspace, operation: "health" });
  assert.deepEqual(await response.json(), { configured: false });
  assert.equal(f.calls.length, 3);
  const memberParams = new URL(f.calls[1].url).searchParams;
  assert.equal(memberParams.get("workspace_id"), `eq.${otherWorkspace}`);
  assert.equal(memberParams.get("user_id"), `eq.${user}`);
  const generation = await f.run({ workspace: otherWorkspace, operation: "generate", model, request: sample });
  assert.equal(generation.status, 503);
  assert.deepEqual(await generation.json(), { error: "gemini_not_configured" });
  assert.equal(f.calls.length, 6);
});

test("health validates connectivity without consuming a generation and reports only allowed models", async () => {
  const f = fixture({ role: "owner", catalog: () =>
    Response.json({ models: [aiModel(model), aiModel("gemini-2.5-flash")] }) });
  const response = await f.run({ operation: "health" });
  assert.deepEqual(await response.json(), { configured: true, connected: true, models: [model] });
  assert.equal(googleCalls(f)[0].url, `${base}?pageSize=200`);
  assert.equal(googleCalls(f)[0].init?.method, "GET");
  assert.equal(response.headers.get("cache-control"), "no-store");
});

test("generation binds Google credentials to the fixed provider, preserves the Rust-compatible response, and caps output", async () => {
  const f = fixture();
  const response = await f.run({ operation: "generate", model, request: sample });
  assert.deepEqual(await response.json(), {
    candidates: [{ content: { parts: [{ text: '{"title":"Ürün"}' }] }, finishReason: "STOP" }],
  });
  const target = googleCalls(f)[0];
  assert.equal(target.url, `${base}/${model}:generateContent`);
  assert.equal(target.init?.method, "POST");
  assert.equal(target.init?.redirect, "error");
  const headers = new Headers(target.init?.headers);
  assert.equal(headers.get("x-goog-api-key"), apiKey);
  assert.equal(headers.get("authorization"), null);
  assert.deepEqual(JSON.parse(String(target.init?.body)), {
    ...sample, generationConfig: { ...sample.generationConfig, maxOutputTokens: 8192 },
  });
});

test("malformed model paths, tools, attachments, unsupported schemas, and excessive output never reach Google", async () => {
  for (const overrides of [
    { model: "gemini-flash-latest/../../upload" },
    { model: "gemini-2..5-flash" },
    { model: "gemini-2.5-flash%2fother" },
    { model: "gemini-2.5-flash\n" },
    { model: "gemini-2.5-flash." },
    { model: "gemini-2-.5-flash" },
    { model: "gemini-2.-5-flash" },
    { request: { ...sample, tools: [{ urlContext: {} }] } },
    { request: { ...sample, contents: [{ parts: [{ fileData: { fileUri: "gs://other/data" } }] }] } },
    { request: { ...sample, generationConfig: { ...sample.generationConfig, responseSchema: { $ref: "https://example.com" } } } },
    { request: { ...sample, generationConfig: { ...sample.generationConfig, maxOutputTokens: 8193 } } },
    { url: "https://example.com" },
  ]) {
    const f = fixture();
    assert.equal((await f.run({ operation: "generate", model, request: sample, ...overrides })).status, 400);
    assert.equal(f.calls.length, 0);
  }
  const f = fixture();
  const response = await f.run({ operation: "generate", model: "gemini-unlisted", request: sample });
  assert.deepEqual(await response.json(), { error: "gemini_model_not_allowed", retry_with_another_model: true });
  assert.equal(f.calls.length, 3);
});

test("approved model names with literal version dots reach only the Google model endpoint", async () => {
  const dotted = "gemini-2.5-flash";
  const f = fixture({ secret: JSON.stringify({ api_key: apiKey, models: [dotted] }) });
  const response = await f.run({ operation: "generate", model: dotted, request: sample });
  assert.equal(response.status, 200);
  assert.deepEqual(googleCalls(f).map(({ url }) => url), [`${base}/${dotted}:generateContent`]);
});

test("input byte limit rejects an oversized unicode body before using any credential", async () => {
  const f = fixture();
  const request = { ...sample, contents: [{ parts: [{ text: "ü".repeat(135 * 1024) }] }] };
  assert.equal((await f.run({ operation: "generate", model, request })).status, 413);
  assert.equal(f.calls.length, 0);
});

test("sanitized provider errors distinguish auth failure from model fallback and never return upstream messages", async () => {
  for (const [providerStatus, reason, status, error, retry] of [
    [400, "API_KEY_INVALID", 502, "gemini_credentials_rejected", false],
    [403, "PERMISSION_DENIED", 502, "gemini_credentials_rejected", false],
    [400, "INVALID_ARGUMENT", 400, "gemini_request_rejected", true],
    [404, "NOT_FOUND", 404, "gemini_model_unavailable", true],
    [429, "RESOURCE_EXHAUSTED", 429, "gemini_rate_limited", true],
  ] as const) {
    const f = fixture({ providerStatus, providerBody: { error: { message: apiKey, details: [{ reason }] } } });
    const response = await f.run({ operation: "generate", model, request: sample });
    assert.equal(response.status, status);
    assert.deepEqual(await response.json(), { error, upstream_status: providerStatus, retry_with_another_model: retry });
  }
});

test("configuration is restricted to owners with valid fixed-shape requests", async () => {
  for (const credentials of [{ api_key: "short" }, { endpoint: "https://example.com" }, { models: [model] }]) {
    const f = fixture({ role: "owner", vault: {} });
    const response = await f.run({ ...configure, ...credentials });
    assert.equal(response.status, 400);
    assert.equal(f.calls.length, 0);
  }
  const editor = fixture({ vault: {} });
  const response = await editor.run(configure);
  assert.equal(response.status, 403);
  assert.deepEqual(await response.json(), { error: "gemini_configuration_owner_required" });
  assert.equal(editor.calls.length, 2);
  const ownerWithoutVault = fixture({ role: "owner", missingVaultKey: true });
  assert.deepEqual(await (await ownerWithoutVault.run(configure)).json(), { error: "gemini_credentials_store_unavailable" });
  assert.equal(ownerWithoutVault.calls.length, 2);
  assert.deepEqual(await (await ownerWithoutVault.run({ operation: "health" })).json(), { error: "gemini_credentials_store_unavailable" });
  assert.equal(googleCalls(ownerWithoutVault).length, 0);
});

test("owner verifies model pages against the default policy before creating a Vault record", async () => {
  const validDefault = "gemini-2.5-flash";
  const opaqueToken = "page-2/+?=";
  const f = fixture({
    role: "owner", vault: {}, secret: null, useModernPublishableKey: true,
    catalog: (url) => url.searchParams.has("pageToken")
      ? Response.json({ models: [aiModel(validDefault), aiModel("gemini-external-allowlist")] })
      : Response.json({ models: [aiModel(model), aiModel("gemini-3.6-flash", "embedContent")], nextPageToken: opaqueToken }),
  });
  const response = await f.run({ ...configure, api_key: `  ${candidateKey}  ` });
  assert.equal(response.status, 200);
  assert.deepEqual(await response.json(), { configured: true, connected: true, models: [validDefault, model], version: 1 });
  const [first, next] = googleCalls(f);
  assert.equal(new URL(next.url).searchParams.get("pageToken"), opaqueToken);
  assert.ok(first && next);
  for (const target of [first, next]) {
    assert.equal(target.init?.method, "GET");
    assert.equal(target.init?.redirect, "error");
    assert.equal(target.url.includes(candidateKey), false);
    assert.equal(new Headers(target.init?.headers).get("x-goog-api-key"), candidateKey);
  }
  const [write] = vaultWrites(f);
  const { allowed_models, ...args } = JSON.parse(String(write.init?.body));
  assert.deepEqual(args, {
    target_owner: user, target_workspace: workspace, new_api_key: candidateKey, expected_version: 0,
  });
  assert.ok(allowed_models.includes(validDefault) && allowed_models.includes(model));
  assert.ok(!allowed_models.includes("gemini-external-allowlist"));
  assert.ok(f.calls.findIndex((call) => call.url.endsWith("/rpc/team_get_gemini_settings")) < f.calls.indexOf(first));
  assert.equal(new Headers(f.calls[0].init?.headers).get("apikey"), "sb_publishable_synthetic-test");
  assert.equal(new Headers(write.init?.headers).get("apikey"), serviceApiKey);
  assert.equal(new Headers(write.init?.headers).get("authorization"), null);
  assert.equal(response.headers.get("cache-control"), "no-store");
});

test("rotation preserves the legacy allowlist and subsequent generation uses the Vault key", async () => {
  const f = fixture({ role: "owner", vault: { legacyServiceKey: true }, catalog: () =>
    Response.json({ models: [aiModel(model), aiModel("gemini-2.5-flash")] }) });
  const legacy = await f.run({ operation: "health" });
  assert.equal(legacy.status, 200);
  const get = f.calls.find((call) => call.url.endsWith("/rpc/team_get_gemini_settings"));
  assert.ok(get);
  assert.equal(new Headers(get.init?.headers).get("authorization"), "Bearer legacy.synthetic.service-key");
  assert.equal(new Headers(googleCalls(f).at(-1)?.init?.headers).get("x-goog-api-key"), apiKey);

  const response = await f.run(configure);
  assert.equal(response.status, 200);
  assert.deepEqual(await response.json(), { configured: true, connected: true, models: [model], version: 1 });
  const next = await f.run({ operation: "generate", model, request: sample });
  assert.equal(next.status, 200);
  assert.equal(new Headers(googleCalls(f).at(-1)?.init?.headers).get("x-goog-api-key"), candidateKey);
  assert.equal((await f.run({ operation: "generate", model: "gemini-2.5-flash", request: sample })).status, 400);

  const repeat = await f.run(configure);
  assert.deepEqual(await repeat.json(), { configured: true, connected: true, models: [model], version: 2 });
  assert.equal(JSON.parse(String(vaultWrites(f)[1].init?.body)).expected_version, 1);
});

test("rotation and health report active models without losing a temporarily unavailable approved model", async () => {
  const fallback = "gemini-2.5-flash";
  let fallbackAvailable = false;
  const f = fixture({
    role: "owner",
    vault: { stored: { api_key: apiKey, models: [model, fallback], version: 4 } },
    catalog: () => Response.json({ models: [aiModel(model), ...(fallbackAvailable ? [aiModel(fallback)] : [])] }),
  });
  const response = await f.run(configure);
  assert.deepEqual(await response.json(), { configured: true, connected: true, models: [model], version: 5 });
  assert.deepEqual(JSON.parse(String(vaultWrites(f)[0].init?.body)).allowed_models, [model, fallback]);
  const reduced = await f.run({ operation: "health" });
  assert.deepEqual(await reduced.json(), { configured: true, connected: true, models: [model] });

  fallbackAvailable = true;
  const healthy = await f.run({ operation: "health" });
  assert.deepEqual(await healthy.json(), { configured: true, connected: true, models: [model, fallback] });
});

test("unverified candidate keys, missing compatible models and unsafe pagination never write the Vault", async () => {
  const denied = fixture({ role: "owner", vault: {}, providerStatus: 403, providerBody: { message: candidateKey } });
  const refused = await denied.run(configure);
  assert.equal(refused.status, 502);
  assert.equal((await refused.json()).error, "gemini_credentials_rejected");
  assert.equal(vaultWrites(denied).length, 0);
  const noMatches = fixture({ role: "owner", vault: {}, catalog: () =>
    Response.json({ models: [aiModel(model, "countTokens"), aiModel("gemini-unlisted")] }) });
  const invalidModels = await noMatches.run(configure);
  assert.equal(invalidModels.status, 422);
  assert.deepEqual(await invalidModels.json(), { error: "gemini_no_compatible_models" });
  assert.equal(vaultWrites(noMatches).length, 0);
  const repeats = fixture({ role: "owner", vault: {}, catalog: () =>
    Response.json({ models: [aiModel(model)], nextPageToken: "same-token" }) });
  const invalidPage = await repeats.run(configure);
  assert.equal(invalidPage.status, 502);
  assert.deepEqual(await invalidPage.json(), { error: "invalid_gemini_response" });
  assert.equal(vaultWrites(repeats).length, 0);
});

test("Vault errors fail closed and concurrent writes never produce a false success", async () => {
  const unreadable = fixture({ vault: { getStatus: 503 } });
  const failed = await unreadable.run({ operation: "generate", model, request: sample });
  assert.deepEqual(await failed.json(), { error: "gemini_credentials_store_unavailable" });
  assert.equal(googleCalls(unreadable).length, 0);

  for (const [status, error] of [[409, "gemini_configuration_conflict"], [403, "gemini_configuration_owner_required"],
    [503, "gemini_credentials_store_unavailable"]] as const) {
    const f = fixture({ role: "owner", vault: { stored: { api_key: apiKey, models: [model], version: 3 }, setStatus: status },
      catalog: () => Response.json({ models: [aiModel(model)] }) });
    const response = await f.run(configure);
    assert.equal(response.status, status);
    assert.deepEqual(await response.json(), { error });
    assert.equal(JSON.parse(String(vaultWrites(f)[0].init?.body)).expected_version, 3);
  }
});
