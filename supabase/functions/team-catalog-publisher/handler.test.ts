import assert from "node:assert/strict";
import { test } from "node:test";
import { handleCatalogPublisher } from "./handler.ts";

const project = "https://catalog.example.test";
const endpoint = project + "/functions/v1/team-catalog-publisher";
const workspace = "11111111-1111-4111-8111-111111111111";
const otherWorkspace = "22222222-2222-4222-8222-222222222222";
const token = "4".repeat(64);
const serviceKey = "sb_secret_test_fixture";
const product = { sku: "A", name: "Ürün A", facts: { url: "https://store.example.test/product", quantity: 5, price1: null } };
const candidate = { expected_revision: 4, products: [product] };

function fixture(overrides: Record<string, string | undefined> = {}, answer?: () => Response) {
  const calls: { url: URL; init?: RequestInit }[] = [];
  const env: Record<string, string | undefined> = {
    TEAM_CATALOG_WORKSPACE_ID: workspace,
    TEAM_CATALOG_PUBLISHER_TOKEN: token,
    SUPABASE_URL: project,
    SUPABASE_SECRET_KEYS: JSON.stringify({ default: serviceKey }),
    ...overrides,
  };
  const transport: typeof fetch = async (url, init) => {
    calls.push({ url: new URL(String(url)), init });
    return answer ? answer() : Response.json(init?.method === "POST" ? 5 : [{ revision: 4 }]);
  };
  const run = (request: Request) => handleCatalogPublisher(request, (name) => env[name], transport);
  const get = (url = endpoint, key = token) =>
    run(new Request(url, { headers: { "x-catalog-publisher-token": key } }));
  const post = (body: unknown, key = token) =>
    run(new Request(endpoint, { method: "POST", headers: { "x-catalog-publisher-token": key }, body: JSON.stringify(body) }));
  return { calls, run, get, post };
}

test("project public keys and incorrect scoped tokens cannot read or publish", async () => {
  const f = fixture();
  assert.equal((await f.run(new Request(endpoint, { headers: { apikey: "sb_publishable_synthetic" } }))).status, 401);
  assert.equal((await f.get(endpoint, "5".repeat(64))).status, 401);
  assert.equal((await f.post(candidate, "short-token")).status, 401);
  assert.equal(f.calls.length, 0);
});

test("GET reads only the configured workspace and returns only its baseline revision", async () => {
  const f = fixture({}, () => Response.json([{ revision: 7, unused: serviceKey }]));
  const response = await f.get(project + "/team-catalog-publisher");
  assert.equal(response.headers.get("cache-control"), "no-store");
  assert.deepEqual(await response.json(), { revision: 7 });
  const request = f.calls[0];
  assert.equal(request.url.origin, project);
  assert.equal(request.url.pathname, "/rest/v1/team_catalog_state");
  assert.equal(request.url.searchParams.get("workspace_id"), `eq.${workspace}`);
  assert.equal(request.url.searchParams.get("select"), "revision");
  assert.equal(new Headers(request.init?.headers).get("apikey"), serviceKey);
  assert.equal(new Headers(request.init?.headers).has("x-catalog-publisher-token"), false);
  assert.equal(new Headers(request.init?.headers).has("authorization"), false);
  assert.equal(request.init?.redirect, "error");
  assert.deepEqual(await (await fixture({}, () => Response.json([])).get()).json(), { revision: null });
});

test("POST sends the expected revision and the vetted snapshot to the only permitted RPC", async () => {
  const f = fixture();
  const response = await f.post(candidate);
  assert.equal(response.status, 200);
  assert.equal(await response.json(), 5);
  assert.equal(f.calls.length, 1);
  const request = f.calls[0];
  assert.equal(request.url.pathname, "/rest/v1/rpc/team_replace_catalog");
  assert.deepEqual(JSON.parse(String(request.init?.body)), { target_workspace: workspace, ...candidate });
  assert.equal(request.init?.method, "POST");
  const legacy = fixture({ SUPABASE_SECRET_KEYS: undefined, SUPABASE_SERVICE_ROLE_KEY: "legacy-service-role-test" });
  assert.equal((await legacy.post({ ...candidate, expected_revision: null })).status, 200);
  assert.equal(new Headers(legacy.calls[0].init?.headers).get("authorization"), "Bearer legacy-service-role-test");
});

test("rejects workspace selection, unsupported routes and invalid or incomplete snapshots", async () => {
  const f = fixture({ TEAM_CATALOG_MIN_PRODUCTS: "2" });
  const anotherProduct = { ...product, sku: "B" };
  for (const input of [
    candidate,
    { ...candidate, products: [product, { ...anotherProduct, sku: " A " }] },
    { ...candidate, products: [product, { ...anotherProduct, name: " " }] },
    { ...candidate, products: [product, { ...anotherProduct, facts: [] }] },
    { ...candidate, products: [product, { ...anotherProduct, facts: { nested: ["wrong"] } }] },
    { expected_revision: "4", products: [product, anotherProduct] },
    { products: [product, anotherProduct] },
    { ...candidate, products: [product, anotherProduct], target_workspace: otherWorkspace },
  ]) {
    assert.equal((await f.post(input)).status, 400);
  }
  assert.equal((await f.get(endpoint + "?workspace=" + otherWorkspace)).status, 404);
  assert.equal((await f.get(endpoint + "/anything")).status, 404);
  assert.equal((await f.run(new Request(endpoint, {
    method: "DELETE", headers: { "x-catalog-publisher-token": token },
  }))).status, 405);
  assert.equal(f.calls.length, 0);
  assert.equal((await f.post({ ...candidate, products: [product, anotherProduct] })).status, 200);
});

test("streamed bytes and the declared length are both bounded before calling the database", async () => {
  const f = fixture();
  const longText = "ğ".repeat(2 * 1024 * 1024 + 1);
  assert.equal((await f.post({ ...candidate, products: [{ ...product, name: longText }] })).status, 413);
  assert.equal((await f.run(new Request(endpoint, {
    method: "POST",
    headers: { "x-catalog-publisher-token": token, "content-length": "5000000" },
    body: "{}",
  }))).status, 413);
  assert.equal(f.calls.length, 0);
});

test("maps revision conflicts and sanitizes upstream and configuration failures", async () => {
  const conflict = fixture({}, () => Response.json({ code: "40001", details: serviceKey }, { status: 500 }));
  const conflicted = await conflict.post(candidate);
  assert.equal(conflicted.status, 409);
  assert.deepEqual(await conflicted.json(), { error: "catalog_revision_conflict" });
  const broken = fixture({}, () => Response.json({ code: "XX000", details: serviceKey }, { status: 500 }));
  for (const response of [await broken.get(), await broken.post(candidate)]) {
    assert.equal(response.status, 503);
    assert.deepEqual(await response.json(), { error: "catalog_publisher_unavailable" });
  }
  const unconfigured = fixture({ TEAM_CATALOG_WORKSPACE_ID: undefined });
  assert.equal((await unconfigured.get()).status, 503);
  assert.equal(unconfigured.calls.length, 0);
});
