import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";
import { createClient } from "@supabase/supabase-js";
import { createServer } from "vite";

const project = "https://quote-history.example.test";
const workspace = "10000000-0000-4000-8000-000000000001";
const user = "20000000-0000-4000-8000-000000000001";
const quote = "30000000-0000-4000-8000-000000000001";
let vite, connection, teamQuotes;

before(async () => {
  vite = await createServer({
    root: fileURLToPath(new URL("..", import.meta.url)), configFile: false, appType: "custom",
    server: { middlewareMode: true, hmr: false, watch: null }, logLevel: "silent",
  });
  connection = await vite.ssrLoadModule("/src/teamConnection.ts");
  ({ teamQuotes } = await vite.ssrLoadModule("/src/teamQuotes.ts"));
});
after(async () => { await vite?.close(); });

function snapshot(version) {
  return {
    version, at: "2026-11-24T10:00:00+00:00",
    snapshot_json: {
      quote: { no: "TK-1", currency: "TRY", note: "Teklif notu", valid_until: null, contact_id: user },
      items: [{ name: "Kalem", qty: 2, unit_price: 30, tax_rate: 20, cost: 15 }],
    },
  };
}

function fixture(t) {
  const remote = { versions: [], status: 200, beforeRead: null };
  const requests = [];
  const cacheOps = [];
  const device = new Map();
  const client = createClient(project, "test-key", {
    auth: { persistSession: false, autoRefreshToken: false, detectSessionInUrl: false },
    global: {
      async fetch(input, init) {
        const url = new URL(input instanceof Request ? input.url : input);
        assert.equal(url.origin, project);
        assert.equal(url.pathname, "/rest/v1/team_quote_versions");
        assert.equal(init.method, "GET");
        assert.equal(url.searchParams.get("workspace_id"), `eq.${workspace}`);
        assert.equal(url.searchParams.get("quote_id"), `eq.${quote}`);
        const limit = Number(url.searchParams.get("limit"));
        const beforeVersion = Number(url.searchParams.get("version")?.slice(3) ?? Infinity);
        requests.push({ limit, beforeVersion });
        await remote.beforeRead?.();
        if (remote.status !== 200) return Response.json({ message: "Internal detail" }, { status: remote.status });
        return Response.json(remote.versions.filter(({ version }) => version < beforeVersion).slice(0, limit));
      },
    },
  });
  const previousWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
  Object.defineProperty(globalThis, "window", { configurable: true, value: {
    __TAURI_INTERNALS__: {
      invoke: async (command, { scope, request }) => {
        assert.equal(command, "team_cache");
        assert.deepEqual(JSON.parse(scope), [project, user, workspace, "quotes"]);
        assert.equal(request.key, `history:${quote}`);
        cacheOps.push(request.action);
        if (request.action === "write") device.set(request.key, structuredClone(request.data));
        else if (request.action === "read") return device.get(request.key) ?? null;
        else assert.fail(`Unexpected cache operation ${request.action}`);
      },
    },
  } });
  connection.teamProject.value = project;
  connection.teamWorkspace.value = workspace;
  connection.teamSession.value = { user: { id: user } };
  connection.teamClient.value = client;
  connection.teamReachable.value = true;
  t.after(() => {
    connection.teamAccessIssue.value = false;
    connection.teamClient.value = null;
    connection.teamSession.value = null;
    connection.teamWorkspace.value = "";
    connection.teamProject.value = "";
    connection.teamReachable.value = null;
    if (previousWindow) Object.defineProperty(globalThis, "window", previousWindow);
    else Reflect.deleteProperty(globalThis, "window");
  });
  return { remote, requests, cacheOps };
}

test("walks all version pages, limits the cached fields, and can read a complete offline copy", async (t) => {
  const { remote, requests, cacheOps } = fixture(t);
  remote.versions = Array.from({ length: 51 }, (_, index) => snapshot(51 - index));
  remote.versions[50].snapshot_json = { quote: { currency: "TRY" }, items: [{ name: "Eski kalem", qty: 2 }] };
  const online = await teamQuotes.history(quote);
  assert.equal(online.cached, false);
  assert.deepEqual(online.versions.map(({ version }) => version), Array.from({ length: 51 }, (_, index) => 51 - index));
  assert.deepEqual(requests, [{ limit: 50, beforeVersion: Infinity }, { limit: 50, beforeVersion: 2 }]);
  assert.equal("cost" in online.versions[0].snapshot_json.items[0], false);
  assert.deepEqual(online.versions[50].snapshot_json.items[0], { name: "Eski kalem", qty: 2, unit_price: null, tax_rate: null });

  remote.status = 503;
  assert.deepEqual(await teamQuotes.history(quote), { versions: online.versions, cached: true });
  assert.deepEqual(cacheOps, ["write", "read"]);

  remote.status = 403;
  await assert.rejects(teamQuotes.history(quote), /teklif geçmişine erişilemiyor/);
  assert.deepEqual(cacheOps, ["write", "read"]);
});

test("distinguishes an unknown offline history from a downloaded empty history", async (t) => {
  fixture(t);
  connection.teamReachable.value = false;
  await assert.rejects(teamQuotes.history(quote), /henüz indirilmedi/);
  connection.teamReachable.value = true;
  assert.deepEqual(await teamQuotes.history(quote), { versions: [], cached: false });
  connection.teamReachable.value = false;
  assert.deepEqual(await teamQuotes.history(quote), { versions: [], cached: true });
});

test("discarding a pending history request prevents a stale response from reaching the cache", async (t) => {
  const { remote, cacheOps } = fixture(t);
  const started = Promise.withResolvers();
  const release = Promise.withResolvers();
  remote.versions = [snapshot(1)];
  remote.beforeRead = () => { started.resolve(); return release.promise; };
  const controller = new AbortController();
  const result = teamQuotes.history(quote, controller.signal);
  await started.promise;
  connection.teamWorkspace.value = "10000000-0000-4000-8000-000000000002";
  controller.abort();
  release.resolve();
  await assert.rejects(result, { name: "AbortError" });
  assert.deepEqual(cacheOps, []);
});
