import { test } from "node:test";
import assert from "node:assert/strict";
import { handleGsc, handleGscCollector } from "./handler.ts";

const workspace = "11111111-1111-4111-8111-111111111111";
const user = "22222222-2222-4222-8222-222222222222";
const property = "https://store.example.test/";
const project = "https://project.example.test";
const day = (daysAgo: number) => new Date(Date.now() - daysAgo * 86400000).toISOString().slice(0, 10);
const snapshot = { workspace, mode: "snapshot", start: day(50), end: day(22) };
const queryRequest = (body: object) => new Request(project + "/functions/v1/team-gsc", {
  method: "POST",
  headers: { Authorization: "Bearer user-session" },
  body: JSON.stringify(body),
});

const trackedPage = property + '3d-yazici?kampanya=1&dil=tr';
const trackedQuery = 'bambu lab (3d), "fiyat" & stok';
const trackedId = "33333333-3333-4333-8333-333333333333";
const collectorToken = "distinct-server-token-123456789-123456789";
const daily = { workspace, mode: "daily_comparison", page: trackedPage, query: trackedQuery, event_day: "2026-11-22" };
const dayRow = (day: string, clicks = 2, impressions = 20, position = 4) => ({ keys: [day], clicks, impressions, position });
const providerPath = `/webmasters/v3/sites/${encodeURIComponent(property)}/searchAnalytics/query`;

const privateKey = (async () => {
  const key = await crypto.subtle.generateKey({
    name: "RSASSA-PKCS1-v1_5", modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: "SHA-256",
  }, true, ["sign", "verify"]);
  return Buffer.from(await crypto.subtle.exportKey("pkcs8", key.privateKey)).toString("base64");
})();

async function dailyEnv() {
  const config: Record<string, string> = {
    SUPABASE_URL: project,
    SUPABASE_ANON_KEY: "publishable-key",
    SUPABASE_SERVICE_ROLE_KEY: "service-key",
    TEAM_GSC_DAILY_WORKSPACE: workspace,
    TEAM_GSC_DAILY_TOKEN: collectorToken,
    TEAM_GSC_11111111_1111_4111_8111_111111111111: JSON.stringify({
      property, client_email: "gsc@example.test", private_key: await privateKey,
    }),
  };
  return (name: string) => config[name];
}

function collectorRequest(body: object = { mode: "collect_daily" }, bearer = collectorToken) {
  return new Request(project + "/functions/v1/team-gsc-daily-collector", { method: "POST",
    headers: { Authorization: `Bearer ${bearer}` }, body: JSON.stringify(body) });
}

test("only editors and owners can request an exact, finalized historical snapshot", async () => {
  const key = await crypto.subtle.generateKey({
    name: "RSASSA-PKCS1-v1_5", modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: "SHA-256",
  }, true, ["sign", "verify"]);
  const privateKey = Buffer.from(await crypto.subtle.exportKey("pkcs8", key.privateKey)).toString("base64");
  const env = (name: string) => ({
    SUPABASE_URL: project,
    SUPABASE_ANON_KEY: "publishable-key",
    TEAM_GSC_11111111_1111_4111_8111_111111111111: JSON.stringify({
      property, client_email: "gsc@example.test", private_key: privateKey,
    }),
  })[name];

  for (const role of ["viewer", "editor", "owner"]) {
    const calls: string[] = [];
    const fetchFixture: typeof fetch = async (input, init) => {
      const address = new URL(String(input));
      calls.push(address.origin);
      if (address.pathname === "/auth/v1/user") return Response.json({ id: user });
      if (address.pathname === "/rest/v1/workspace_members") {
        assert.equal(address.searchParams.get("workspace_id"), `eq.${workspace}`);
        return Response.json([{ role }]);
      }
      if (address.origin === "https://oauth2.googleapis.com") return Response.json({ access_token: "test-google-token" });
      assert.equal(address.pathname, `/webmasters/v3/sites/${encodeURIComponent(property)}/searchAnalytics/query`);
      assert.deepEqual(JSON.parse(String(init?.body)), {
        startDate: snapshot.start, endDate: snapshot.end, rowLimit: 10000, dataState: "final", dimensions: ["page"], startRow: 0,
      });
      return Response.json({ rows: [{
        keys: [property + "sample"], clicks: 4, impressions: 50, ctr: 0.08, position: 6,
      }] });
    };
    const response = await handleGsc(queryRequest(snapshot), env, fetchFixture);
    if (role === "viewer") {
      assert.equal(response.status, 403);
      assert.deepEqual(calls, [project, project]);
    } else {
      assert.equal(response.status, 200);
      assert.deepEqual(await response.json(), {
        configured: true, rows: [{ page: property + "sample", clicks: 4, impressions: 50, ctr: 0.08, position: 6 }], next: null,
      });
    }
  }
});

test("an incomplete or wrongly sized historical window is rejected before contacting a service", async () => {
  const env = () => "set";
  const noNetwork: typeof fetch = async () => { assert.fail("Invalid windows must not reach an external service"); };
  for (const window of [{ start: day(30), end: day(2) }, { start: day(51), end: day(22) }]) {
    const response = await handleGsc(queryRequest({ ...snapshot, ...window }), env, noNetwork);
    assert.equal(response.status, 400);
    assert.deepEqual(await response.json(), { error: "invalid_window" });
  }
});

test("tracks the exact URL and query, preserves missing metrics and excludes the event and future days in Pacific time", async (t) => {
  t.mock.method(Date, "now", () => Date.parse("2026-11-25T03:10:00Z")); // Still November 24 in Search Console.
  const env = await dailyEnv();
  let persisted;
  const calls: string[] = [];
  const request: typeof fetch = async (input, init) => {
    const url = new URL(String(input));
    const headers = new Headers(init?.headers);
    calls.push(url.pathname);
    if (url.pathname === "/auth/v1/user") return Response.json({ id: user });
    if (url.pathname === "/rest/v1/workspace_members") return Response.json([{ role: "editor" }]);
    if (url.pathname === "/rest/v1/team_gsc_daily_targets") {
      assert.equal(headers.get("authorization"), "Bearer user-session");
      assert.equal(url.searchParams.get("workspace_id"), `eq.${workspace}`);
      assert.equal(url.searchParams.get("property"), `eq.${property}`);
      assert.equal(url.searchParams.get("page"), `eq.${trackedPage}`);
      assert.equal(url.searchParams.get("query"), `eq.${trackedQuery}`);
      return Response.json([]);
    }
    if (url.pathname === "/rest/v1/rpc/team_gsc_register_daily_target") {
      assert.equal(headers.get("authorization"), "Bearer user-session");
      assert.deepEqual(JSON.parse(String(init?.body)), { target_workspace: workspace, target_property: property,
        target_page: trackedPage, target_query: trackedQuery, target_search_type: "web" });
      return Response.json(trackedId);
    }
    if (url.pathname === "/rest/v1/team_gsc_daily_measurements") {
      assert.equal(url.searchParams.get("target_id"), `eq.${trackedId}`);
      assert.deepEqual(url.searchParams.getAll("day"), ["gte.2026-11-15", "lte.2026-11-29"]);
      return Response.json([]);
    }
    if (url.pathname === "/rest/v1/rpc/team_gsc_store_daily") {
      assert.equal(headers.get("authorization"), "Bearer service-key");
      persisted = JSON.parse(String(init?.body));
      return Response.json(8);
    }
    if (url.origin === "https://oauth2.googleapis.com") return Response.json({ access_token: "google-token" });
    assert.equal(url.pathname, providerPath);
    assert.equal(headers.get("authorization"), "Bearer google-token");
    const body = JSON.parse(String(init?.body));
    assert.deepEqual(body.dimensions, ["date"]);
    assert.equal(body.type, "web");
    assert.equal(body.startDate, "2026-11-15");
    assert.equal(body.endDate, "2026-11-23");
    if (body.dataState === "final") {
      assert.equal(body.dimensionFilterGroups, undefined);
      return Response.json({ rows: [dayRow("2026-11-15"), dayRow("2026-11-16"), dayRow("2026-11-23")] });
    }
    assert.equal(body.dataState, "all");
    assert.deepEqual(body.dimensionFilterGroups, [{ filters: [
      { dimension: "page", operator: "equals", expression: trackedPage },
      { dimension: "query", operator: "equals", expression: trackedQuery },
    ] }]);
    return Response.json({ rows: [dayRow("2026-11-15"), dayRow("2026-11-22"), dayRow("2026-11-23")],
      metadata: { first_incomplete_date: "2026-11-22" } });
  };
  const response = await handleGsc(queryRequest({ ...daily, track: true }), env, request);
  assert.equal(response.status, 200);
  const result = await response.json();
  assert.equal(result.configured, true);
  assert.equal(result.tracked, true);
  assert.equal(result.daily.length, 14);
  const byDay = Object.fromEntries(result.daily.map((row) => [row.day, row]));
  assert.deepEqual(byDay["2026-11-15"], { day: "2026-11-15", state: "final", clicks: 2, impressions: 20, position: 4 });
  assert.deepEqual(byDay["2026-11-16"], { day: "2026-11-16", state: "missing", clicks: null, impressions: null, position: null });
  assert.equal(byDay["2026-11-17"].state, "pending");
  assert.equal(byDay["2026-11-22"], undefined);
  assert.equal(byDay["2026-11-23"].state, "provisional");
  assert.equal(byDay["2026-11-24"].state, "pending");
  assert.deepEqual(persisted, { target_workspace: workspace, target_id: trackedId,
    samples: result.daily.filter(({ day }) => day <= "2026-11-23"), collected_at: "2026-11-25T03:10:00.000Z", mark_refreshed: false });
  assert.equal(calls.filter((entry) => entry.endsWith(providerPath)).length, 2);
});

test("viewer can use completely stored coverage without contacting Google or writing, and cannot register tracking", async (t) => {
  t.mock.method(Date, "now", () => Date.parse("2026-11-30T15:00:00Z"));
  const env = await dailyEnv();
  const calls: string[] = [];
  const request: typeof fetch = async (input, init) => {
    const url = new URL(String(input));
    calls.push(url.pathname);
    assert.equal(url.origin, project);
    assert.equal(init?.method ?? "GET", "GET");
    if (url.pathname === "/auth/v1/user") return Response.json({ id: user });
    if (url.pathname === "/rest/v1/workspace_members") return Response.json([{ role: "viewer" }]);
    if (url.pathname === "/rest/v1/team_gsc_daily_targets") return Response.json([{ id: trackedId, slot: 1 }]);
    assert.equal(url.pathname, "/rest/v1/team_gsc_daily_measurements");
    return Response.json(Array.from({ length: 15 }, (_, index) => ({ day: `2026-11-${15 + index}`,
      state: "final", clicks: 2, impressions: 20, position: 4 })));
  };
  const allowed = await handleGsc(queryRequest(daily), env, request);
  assert.equal(allowed.status, 200);
  assert.equal((await allowed.json()).daily.length, 14);
  calls.length = 0;
  const denied = await handleGsc(queryRequest({ ...daily, track: true }), env, request);
  assert.equal(denied.status, 403);
  assert.deepEqual(await denied.json(), { error: "daily_tracking_access_denied" });
  assert.deepEqual(calls, ["/auth/v1/user", "/rest/v1/workspace_members"]);
});

test("pausing a tracked query retains prior measurements and needs no Google call", async (t) => {
  t.mock.method(Date, "now", () => Date.parse("2026-11-30T15:00:00Z"));
  const env = await dailyEnv();
  const request: typeof fetch = async (input, init) => {
    const url = new URL(String(input));
    assert.equal(url.origin, project);
    if (url.pathname === "/auth/v1/user") return Response.json({ id: user });
    if (url.pathname === "/rest/v1/workspace_members") return Response.json([{ role: "editor" }]);
    if (url.pathname === "/rest/v1/team_gsc_daily_targets") return Response.json([{ id: trackedId, slot: 2 }]);
    if (url.pathname === "/rest/v1/rpc/team_gsc_pause_daily_target") {
      assert.equal(new Headers(init?.headers).get("Authorization"), "Bearer user-session");
      assert.deepEqual(JSON.parse(String(init?.body)), { target_workspace: workspace, target_id: trackedId });
      return Response.json(true);
    }
    assert.equal(url.pathname, "/rest/v1/team_gsc_daily_measurements");
    return Response.json([{ day: "2026-11-15", state: "final", clicks: 2, impressions: 20, position: 4 }]);
  };
  const response = await handleGsc(queryRequest({ ...daily, track: false }), env, request);
  assert.equal(response.status, 200);
  const result = await response.json();
  assert.equal(result.tracked, false);
  assert.deepEqual(result.daily[0], { day: "2026-11-15", state: "final", clicks: 2, impressions: 20, position: 4 });
  assert.deepEqual(result.daily[1], { day: "2026-11-16", state: "pending", clicks: null, impressions: null, position: null });
});

test("a tracked query keeps prior measurements available during a Google error", async (t) => {
  t.mock.method(Date, "now", () => Date.parse("2026-11-30T15:00:00Z"));
  const env = await dailyEnv();
  let malformed = false;
  const saved = { day: "2026-11-15", state: "final", clicks: 2, impressions: 20, position: 4 };
  const request: typeof fetch = async (input, init) => {
    const url = new URL(String(input));
    if (url.pathname === "/auth/v1/user") return Response.json({ id: user });
    if (url.pathname === "/rest/v1/workspace_members") return Response.json([{ role: "editor" }]);
    if (url.pathname === "/rest/v1/team_gsc_daily_targets") return Response.json([{ id: trackedId, slot: 2 }]);
    if (url.pathname === "/rest/v1/team_gsc_daily_measurements") {
      return Response.json([saved]);
    }
    if (url.origin === "https://oauth2.googleapis.com") {
      return malformed ? Response.json({ access_token: "google-token" }) : Response.json({ error: "private" }, { status: 503 });
    }
    assert.equal(url.pathname, providerPath);
    const body = JSON.parse(String(init?.body));
    return body.dataState === "final" ? Response.json({ rows: [dayRow("2026-11-16")] }) : Response.json([]);
  };
  for (malformed of [false, true]) {
    const response = await handleGsc(queryRequest(daily), env, request);
    assert.equal(response.status, 200);
    const result = await response.json();
    assert.equal(result.tracked, true);
    assert.equal(result.refreshFailed, true);
    assert.deepEqual(result.daily[0], saved);
    assert.equal(result.daily[1].state, "pending");
  }
});

test("keeps stored daily measurements past the live query window and signals legacy fallback when none exist", async (t) => {
  t.mock.method(Date, "now", () => Date.parse("2026-11-30T15:00:00Z"));
  const env = await dailyEnv();
  let saved = true;
  const row = { day: "2024-01-02", state: "final", clicks: 2, impressions: 20, position: 4 };
  const request: typeof fetch = async (input, init) => {
    const url = new URL(String(input));
    assert.equal(url.origin, project);
    assert.equal(init?.method ?? "GET", "GET");
    if (url.pathname === "/auth/v1/user") return Response.json({ id: user });
    if (url.pathname === "/rest/v1/workspace_members") return Response.json([{ role: "viewer" }]);
    if (url.pathname === "/rest/v1/team_gsc_daily_targets") return Response.json([{ id: trackedId, slot: null }]);
    assert.equal(url.pathname, "/rest/v1/team_gsc_daily_measurements");
    return Response.json(saved ? [row] : []);
  };
  const historical = { ...daily, event_day: "2024-01-06" };
  const history = await handleGsc(queryRequest(historical), env, request);
  assert.equal(history.status, 200);
  const { tracked, daily: stored } = await history.json();
  assert.equal(tracked, false);
  assert.deepEqual(stored.find(({ day }) => day === row.day), row);
  saved = false;
  const missing = await handleGsc(queryRequest(historical), env, request);
  assert.equal(missing.status, 422);
  assert.deepEqual(await missing.json(), { error: "daily_window_unavailable" });
});

test("reports old schema and tracking cap without contacting Google or returning backend details", async (t) => {
  t.mock.method(Date, "now", () => Date.parse("2026-11-25T19:00:00Z"));
  const env = await dailyEnv();
  for (const failure of ["schema", "quota"]) {
    const request: typeof fetch = async (input) => {
      const url = new URL(String(input));
      assert.equal(url.origin, project);
      if (url.pathname === "/auth/v1/user") return Response.json({ id: user });
      if (url.pathname === "/rest/v1/workspace_members") return Response.json([{ role: "owner" }]);
      if (url.pathname === "/rest/v1/team_gsc_daily_targets") return failure === "schema"
        ? Response.json({ code: "PGRST205", message: "private" }, { status: 404 }) : Response.json([]);
      assert.equal(url.pathname, "/rest/v1/rpc/team_gsc_register_daily_target");
      return Response.json({ code: "PT409", message: "private" }, { status: 409 });
    };
    const response = await handleGsc(queryRequest({ ...daily, track: true }), env, request);
    assert.equal(response.status, failure === "schema" ? 404 : 409);
    assert.deepEqual(await response.json(), { error: failure === "schema" ? "daily_schema_unavailable" : "daily_track_limit" });
  }
});

test("daily collector rejects user tokens, extra workspace arguments and misuse of the desktop route before any external call", async () => {
  const env = await dailyEnv();
  const request: typeof fetch = async () => assert.fail("Must reject before making requests");
  for (const [input, status] of [[collectorRequest(undefined, "user-session"), 401],
    [collectorRequest(undefined, "malicious-same-length-123456789-123456789"), 401],
    [collectorRequest({ mode: "collect_daily", workspace }), 400]] as const) {
    assert.equal((await handleGscCollector(input, env, request)).status, status);
  }
  assert.equal((await handleGsc(queryRequest({ workspace, mode: "collect_daily" }), env, request)).status, 400);
});

test("collector rotates a bounded due set, isolates a failed target and persists later targets only to the fixed workspace", async (t) => {
  let now = Date.parse("2026-11-25T19:00:00Z");
  if (Math.floor(now / 3_600_000) % 2 !== 0) now -= 3_600_000;
  const firstHour = now;
  t.mock.method(Date, "now", () => now);
  const env = await dailyEnv();
  const targets = Array.from({ length: 7 }, (_, index) => ({ id: `a1234560-1234-4567-8234-123456789${index}00`,
    page: trackedPage, query: `target-${index}` }));
  const usedTargets: string[] = [];
  const request: typeof fetch = async (input, init) => {
    const url = new URL(String(input));
    if (url.pathname === "/rest/v1/team_gsc_daily_targets") {
      assert.equal(new Headers(init?.headers).get("authorization"), "Bearer service-key");
      assert.equal(url.searchParams.get("workspace_id"), `eq.${workspace}`);
      assert.equal(url.searchParams.get("limit"), "40");
      assert.equal(url.searchParams.get("slot"), "not.is.null");
      assert.match(url.searchParams.get("or") ?? "", /last_collected_at\.is\.null,last_collected_at\.lt\./);
      return Response.json(targets);
    }
    if (url.pathname === "/rest/v1/rpc/team_gsc_store_daily") {
      assert.equal(new Headers(init?.headers).get("authorization"), "Bearer service-key");
      const data = JSON.parse(String(init?.body));
      assert.equal(data.target_workspace, workspace);
      assert.equal(data.mark_refreshed, true);
      assert.equal(data.samples.length, 14);
      assert.equal(data.samples[0].day, "2026-11-11");
      assert.equal(data.samples[13].day, "2026-11-24");
      assert.equal(data.samples[13].state, "pending");
      usedTargets.push(data.target_id);
      return Response.json(14);
    }
    if (url.origin === "https://oauth2.googleapis.com") return Response.json({ access_token: "google-token" });
    assert.equal(url.pathname, providerPath);
    const body = JSON.parse(String(init?.body));
    assert.match(body.endDate, /^2026-11-24$/);
    if (body.dimensionFilterGroups?.[0].filters?.some(({ expression }) => expression === "target-0")) {
      return Response.json({ error: "provider-specific" }, { status: 429 });
    }
    return Response.json({ rows: [] });
  };
  const first = await handleGscCollector(collectorRequest(), env, request);
  assert.deepEqual(await first.json(), { processed: 5, failed: 1 });
  assert.deepEqual([...usedTargets].sort(), targets.slice(1, 6).map(({ id }) => id));
  now = firstHour + 3_600_000;
  const next = await handleGscCollector(collectorRequest(), env, request);
  assert.deepEqual(await next.json(), { processed: 1, failed: 0 });
  assert.equal(usedTargets.at(-1), targets[6].id);
});
