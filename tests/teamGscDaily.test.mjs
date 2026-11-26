import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";
import { createClient } from "@supabase/supabase-js";
import { createServer } from "vite";

const project = "https://gsc-daily.example.test";
const workspace = "10000000-0000-4000-8000-000000000001";
const user = "20000000-0000-4000-8000-000000000001";
const page = "https://example.test/urun/monitor";
const eventAt = "2026-11-25T02:30:00+00:00";
const pacificDay = "2026-11-24";
const sku = "GSC-1";
const term = "örnek monitör";
const dailyKey = JSON.stringify(["daily", sku, term]);
const legacyKey = JSON.stringify([sku, term]);
const scope = JSON.stringify([project, user, workspace, "gsc"]);
let vite, connection, compare;

before(async () => {
  vite = await createServer({
    root: fileURLToPath(new URL("..", import.meta.url)), configFile: false, appType: "custom",
    server: { middlewareMode: true, hmr: false, watch: null }, logLevel: "silent",
  });
  connection = await vite.ssrLoadModule("/src/teamConnection.ts");
  ({ compareTeamProductQuery: compare } = await vite.ssrLoadModule("/src/teamGsc.ts"));
});
after(async () => { await vite?.close(); });

const blankPeriod = { start: "2026-11-17", end: "2026-11-23", days: 0, clicks: 0, impressions: 0, position: null, ctr: null };
const legacyComparison = { event_day: "2026-11-25", before: blankPeriod, after: { ...blankPeriod, start: "2026-11-26", end: "2026-12-02" } };

function dailyWindow() {
  const date = new Date(`${pacificDay}T00:00:00Z`);
  return Array.from({ length: 15 }, (_, index) => index - 7).filter((offset) => offset !== 0).map((offset) => {
    const day = new Date(date.getTime() + offset * 86_400_000).toISOString().slice(0, 10);
    if (offset === -7) return { day, state: "final", clicks: 2, impressions: 20, position: 4 };
    if (offset === 1) return { day, state: "provisional", clicks: 7, impressions: 35, position: 2 };
    return { day, state: offset < 0 ? "missing" : "pending", clicks: null, impressions: null, position: null };
  });
}

function fixture(t) {
  const daily = dailyWindow();
  const server = { role: "editor", daily, dailyError: null, requests: [], legacyReads: 0, legacyMeasurement: legacyComparison,
    beforeDaily: null, tracked: false, refreshFailed: false, configured: true };
  const cached = new Map([[legacyKey, legacyComparison]]);
  const summarized = [];
  const client = createClient(project, "test-key", {
    auth: { persistSession: false, autoRefreshToken: false, detectSessionInUrl: false },
    global: { async fetch(input, init) {
      const url = new URL(input instanceof Request ? input.url : input);
      if (url.pathname === "/functions/v1/team-gsc") {
        assert.equal(init.method, "POST");
        const body = JSON.parse(init.body);
        server.requests.push(body);
        await server.beforeDaily?.();
        if (server.dailyError) return Response.json({ error: server.dailyError.code }, { status: server.dailyError.status });
        if (!server.configured) return Response.json({ configured: false });
        if (typeof body.track === "boolean") server.tracked = body.track;
        return Response.json({ configured: true, tracked: server.tracked, daily: server.daily, refreshFailed: server.refreshFailed });
      }
      assert.equal(url.searchParams.get("workspace_id"), `eq.${workspace}`);
      if (url.pathname === "/rest/v1/team_seo_work_events") return Response.json([{ occurred_at: eventAt, url: page }]);
      if (url.pathname === "/rest/v1/workspace_members") return Response.json([{ role: server.role }]);
      assert.equal(url.pathname, "/rest/v1/team_gsc_comparisons");
      assert.equal(url.searchParams.get("event_day"), "eq.2026-11-25");
      server.legacyReads++;
      return Response.json(server.legacyMeasurement ? [{ measurement: server.legacyMeasurement }] : []);
    } },
  });
  const previous = Object.getOwnPropertyDescriptor(globalThis, "window");
  Object.defineProperty(globalThis, "window", { configurable: true, value: { __TAURI_INTERNALS__: {
    invoke: async (command, args) => {
      if (command === "summarize_team_query") {
        summarized.push(args);
        return { event_day: args.eventDay, before: blankPeriod, after: { ...blankPeriod, start: "2026-11-25", end: "2026-12-01" } };
      }
      assert.equal(command, "team_cache");
      assert.equal(args.scope, scope);
      const { action, key, data } = args.request;
      if (action === "pending") return [];
      if (action === "read") return cached.get(key) ?? null;
      assert.equal(action, "write");
      cached.set(key, structuredClone(data));
    },
  } } });
  connection.teamProject.value = project;
  connection.teamWorkspace.value = workspace;
  connection.teamSession.value = { user: { id: user } };
  connection.teamClient.value = client;
  connection.teamReachable.value = true;
  t.after(() => {
    connection.teamClient.value = null;
    connection.teamSession.value = null;
    connection.teamWorkspace.value = "";
    connection.teamProject.value = "";
    connection.teamReachable.value = null;
    if (previous) Object.defineProperty(globalThis, "window", previous);
    else Reflect.deleteProperty(globalThis, "window");
  });
  return { server, cached, summarized };
}

test("prefers daily observations and sends only finalized rows to the existing summarizer", async (t) => {
  const { server, summarized, cached } = fixture(t);
  const comparison = await compare(sku, term);
  assert.deepEqual(server.requests, [{ mode: "daily_comparison", workspace, page, query: term, event_day: pacificDay }]);
  assert.equal(server.legacyReads, 0);
  assert.equal(comparison.daily.canTrack, true);
  assert.equal(comparison.daily.observations.length, 14);
  assert.deepEqual(summarized, [{ eventDay: pacificDay, rows: [server.daily[0]] }]);
  assert.equal(cached.get(dailyKey).eventAt, eventAt);
  connection.teamReachable.value = false;
  const offline = await compare(sku, term);
  assert.equal(offline.event_day, pacificDay);
  assert.equal(offline.daily.cached, true);
  assert.equal(offline.daily.canTrack, false);
  assert.equal(server.requests.length, 1);
});

test("starting and pausing daily tracking are both explicit and require an authorized live member", async (t) => {
  const { server } = fixture(t);
  server.role = "viewer";
  assert.equal((await compare(sku, term)).daily.canTrack, false);
  await assert.rejects(compare(sku, term, true), /düzenleme yetkisi/u);
  await assert.rejects(compare(sku, term, false), /düzenleme yetkisi/u);
  assert.equal(server.requests.length, 1);
  server.role = "editor";
  assert.equal((await compare(sku, term, true)).daily.tracked, true);
  assert.equal(server.requests[1].track, true);
  assert.equal((await compare(sku, term, false)).daily.tracked, false);
  assert.equal(server.requests[2].track, false);
  assert.equal((await compare(sku, term)).daily.tracked, false);
  assert.equal(Object.hasOwn(server.requests[3], "track"), false);
});

test("older endpoint still returns the legacy summary without silently enabling tracking", async (t) => {
  const { server } = fixture(t);
  server.dailyError = { status: 400, code: "invalid_mode" };
  assert.deepEqual(await compare(sku, term), legacyComparison);
  await assert.rejects(compare(sku, term, true), /henüz etkin değil/u);
  assert.equal(server.legacyReads, 1);
});

test("a comparison outside the daily collection window still shows a saved legacy summary", async (t) => {
  const { server, cached } = fixture(t);
  server.dailyError = { status: 422, code: "daily_window_unavailable" };
  cached.delete(legacyKey);
  assert.deepEqual(await compare(sku, term), legacyComparison);
  assert.equal(server.legacyReads, 1);
  assert.equal(server.requests.length, 1);
  server.legacyMeasurement = null;
  cached.delete(legacyKey);
  await assert.rejects(compare(sku, term), /veri aralığı dışında/u);
  assert.equal(server.requests.length, 2);
});

test("an unconfigured GSC connection still shows saved history without attempting a second live request", async (t) => {
  const { server, cached } = fixture(t);
  server.configured = false;
  server.legacyMeasurement = null;
  assert.deepEqual(await compare(sku, term), legacyComparison);
  assert.equal(server.legacyReads, 1);
  await assert.rejects(compare(sku, term, true), /bağlantısı henüz kurulmamış/u);
  assert.equal(server.legacyReads, 1);
  cached.delete(legacyKey);
  await assert.rejects(compare(sku, term), /bağlantısı henüz kurulmamış/u);
  assert.equal(server.legacyReads, 2);
  assert.equal(server.requests.length, 3);
});

test("keeps the provider refresh failure visible while showing saved measurements", async (t) => {
  const { server } = fixture(t);
  server.tracked = true;
  server.refreshFailed = true;
  const comparison = await compare(sku, term);
  assert.equal(comparison.daily.tracked, true);
  assert.equal(comparison.daily.refreshFailed, true);
  assert.equal(comparison.daily.cached, undefined);
  assert.equal(comparison.daily.observations[0].state, "final");
});

test("scope changes and incomplete or contradictory daily payloads cannot update the cache", async (t) => {
  const { server, cached } = fixture(t);
  const inconsistent = server.daily.find(({ state }) => state === "missing");
  inconsistent.clicks = 0;
  await assert.rejects(compare(sku, term), /doğrulanamadı/u);
  inconsistent.clicks = null;
  server.beforeDaily = () => { connection.teamWorkspace.value = "another-workspace"; };
  await assert.rejects(compare(sku, term), /Çalışma alanı değişti/u);
  assert.equal(cached.has(dailyKey), false);
});
