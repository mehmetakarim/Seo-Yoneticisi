import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";
import { createClient } from "@supabase/supabase-js";
import { createServer } from "vite";

const project = "https://quote-conflict.example.test";
const workspace = "10000000-0000-4000-8000-000000000001";
const user = "20000000-0000-4000-8000-000000000001";
const quoteId = "30000000-0000-4000-8000-000000000001";
const other = "30000000-0000-4000-8000-000000000002";
const scope = JSON.stringify([project, user, workspace, "quotes"]);
let vite, connection, teamQuotes, flushTeamQuotes;

before(async () => {
  vite = await createServer({
    root: fileURLToPath(new URL("..", import.meta.url)), configFile: false, appType: "custom",
    server: { middlewareMode: true, hmr: false, watch: null }, logLevel: "silent",
  });
  connection = await vite.ssrLoadModule("/src/teamConnection.ts");
  ({ teamQuotes, flushTeamQuotes } = await vite.ssrLoadModule("/src/teamQuotes.ts"));
});
after(async () => { await vite?.close(); });

function fixture(t) {
  const operations = [
    { id: "rejected", contact: { id: quoteId, revision: 2, note: "Cihaz taslağı" }, conflict: "revision changed" },
    { id: "unrelated", contact: { id: other, revision: 2 }, conflict: null },
    { id: "queued", contact: { id: quoteId, revision: 3, note: "En yeni cihaz taslağı" }, conflict: null },
  ].map((op) => ({ ...op, payload: { target_quote: op.contact.id, operation_id: op.id } }));
  const central = {
    status: 200, exists: true, beforeRead: null, reads: 0, beforeSync: null, synced: [],
    quote: {
      id: quoteId, revision: 4, note: "Ortak teklif", updated_at: "2026-11-24T09:00:00Z",
      team_contacts: { name: "Ortak kişi" }, team_quote_items: [], team_quote_versions: [{ count: 0 }],
    },
  };
  const writes = [];
  let compareResult = true;
  const client = createClient(project, "test-key", {
    auth: { persistSession: false, autoRefreshToken: false, detectSessionInUrl: false },
    global: {
      async fetch(input, init) {
        const url = new URL(input instanceof Request ? input.url : input);
        assert.equal(url.origin, project);
        if (url.pathname === "/rest/v1/rpc/team_sync_quote") {
          assert.equal(init.method, "POST");
          central.synced.push(JSON.parse(init.body).target_quote);
          await central.beforeSync?.();
          return Response.json({});
        }
        assert.equal(url.pathname, "/rest/v1/team_quotes");
        assert.equal(init.method, "GET");
        assert.equal(url.searchParams.get("workspace_id"), `eq.${workspace}`);
        assert.equal(url.searchParams.get("id"), `eq.${quoteId}`);
        central.reads++;
        await central.beforeRead?.();
        if (central.status !== 200) return Response.json({ message: "private backend detail" }, { status: central.status });
        return Response.json(central.exists ? [central.quote] : []);
      },
    },
  });
  const previousWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
  Object.defineProperty(globalThis, "window", { configurable: true, value: {
    __TAURI_INTERNALS__: {
      invoke: async (command, args) => {
        if (command === "calculate_team_quotes") return structuredClone(args.quotes);
        assert.equal(command, "team_cache");
        assert.equal(args.scope, scope);
        if (args.request.action === "pending") return structuredClone(operations);
        if (args.request.action === "acknowledge") {
          const index = operations.findIndex(({ id }) => id === args.request.id);
          assert.notEqual(index, -1);
          operations.splice(index, 1);
          return null;
        }
        assert.equal(args.request.action, "resolve_quote_conflict");
        assert.equal(args.request.quote_id, quoteId);
        writes.push(structuredClone(args.request));
        return compareResult;
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
  return { central, writes, operations, failCompare: () => { compareResult = false; } };
}

test("inspects the newest queued draft, then accepts only the reviewed quote against a second fresh read", async (t) => {
  const { central, writes } = fixture(t);
  const review = await teamQuotes.inspectConflict(quoteId);
  assert.equal(review.local.note, "En yeni cihaz taslağı");
  assert.equal(review.remote.contact_name, "Ortak kişi");
  assert.equal(review.remote.revision, 4);
  assert.deepEqual(writes, []);
  const accepted = await teamQuotes.acceptConflict(review);
  assert.equal(accepted.revision, 4);
  assert.equal(central.reads, 2);
  assert.deepEqual(writes.map(({ expected_ids }) => expected_ids), [["rejected", "queued"]]);
});

test("keeps queued work when offline, denied or the central quote is missing", async (t) => {
  const { central, writes } = fixture(t);
  const previous = await teamQuotes.inspectConflict(quoteId);
  connection.teamReachable.value = false;
  const offline = await teamQuotes.inspectConflict(quoteId);
  assert.equal(offline.remoteState, "offline");
  assert.equal(offline.local.revision, 3);
  await assert.rejects(teamQuotes.acceptConflict(previous), /Güncel ortak teklif doğrulanamadı/);
  connection.teamReachable.value = true;
  central.exists = false;
  const missing = await teamQuotes.inspectConflict(quoteId);
  assert.equal(missing.remoteState, "missing");
  await assert.rejects(teamQuotes.acceptConflict(previous), /Güncel ortak teklif doğrulanamadı/);
  central.status = 403;
  await assert.rejects(teamQuotes.acceptConflict(previous), /erişiminizi doğrulayın/);
  assert.deepEqual(writes, []);
});

test("rejects remote and local revisions changed during review", async (t) => {
  const { central, writes, failCompare } = fixture(t);
  const review = await teamQuotes.inspectConflict(quoteId);
  central.quote.revision++;
  await assert.rejects(teamQuotes.acceptConflict(review), /Ortak teklif inceleme sırasında değişti/);
  assert.deepEqual(writes, []);

  const fresh = await teamQuotes.inspectConflict(quoteId);
  failCompare();
  await assert.rejects(teamQuotes.acceptConflict(fresh), /Yerel teklif inceleme sırasında değişti/);
  assert.equal(writes.length, 1);
});

test("a workspace change during confirmation stops the local transaction", async (t) => {
  const { central, writes } = fixture(t);
  const review = await teamQuotes.inspectConflict(quoteId);
  const started = Promise.withResolvers();
  const resume = Promise.withResolvers();
  central.beforeRead = () => { started.resolve(); return resume.promise; };
  const controller = new AbortController();
  const pending = teamQuotes.acceptConflict(review, controller.signal);
  await started.promise;
  connection.teamWorkspace.value = "10000000-0000-4000-8000-000000000002";
  controller.abort();
  resume.resolve();
  await assert.rejects(pending, { name: "AbortError" });
  assert.deepEqual(writes, []);
});

test("skips all operations for a conflicted quote and waits for an active flush before the fresh read", async (t) => {
  const { central, writes, operations } = fixture(t);
  operations[0].conflict = null;
  operations[2].conflict = "revision changed";
  const review = await teamQuotes.inspectConflict(quoteId);
  const started = Promise.withResolvers();
  const resume = Promise.withResolvers();
  central.beforeSync = () => { started.resolve(); return resume.promise; };
  const flush = flushTeamQuotes();
  await started.promise;
  const accept = teamQuotes.acceptConflict(review);
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(central.reads, 1);
  assert.deepEqual(writes, []);

  resume.resolve();
  await flush;
  await accept;
  assert.equal(central.reads, 2);
  assert.deepEqual(central.synced, [other]);
  assert.deepEqual(writes.map(({ expected_ids }) => expected_ids), [["rejected", "queued"]]);
});
