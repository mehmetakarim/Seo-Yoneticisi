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
let vite, connection, teamQuotes, flushTeamQuotes, teamQuoteCanEdit;

before(async () => {
  vite = await createServer({
    root: fileURLToPath(new URL("..", import.meta.url)), configFile: false, appType: "custom",
    server: { middlewareMode: true, hmr: false, watch: null }, logLevel: "silent",
  });
  connection = await vite.ssrLoadModule("/src/teamConnection.ts");
  ({ teamQuotes, flushTeamQuotes, teamQuoteCanEdit } = await vite.ssrLoadModule("/src/teamQuotes.ts"));
});
after(async () => { await vite?.close(); });

function fixture(t, { legacyRuntime = false } = {}) {
  const draft = { id: quoteId, status: "draft", no: "T-1", contact_id: null,
    currency: "USD", fx_rate: 40, fx_date: "2026-11-24", valid_until: "2026-12-05", items: [] };
  const operations = [
    { id: "rejected", contact: { ...draft, revision: 2, note: "Cihaz taslağı" }, conflict: "revision changed" },
    { id: "unrelated", contact: { id: other, revision: 2 }, conflict: null },
    { id: "queued", contact: { ...draft, revision: 3, note: "En yeni cihaz taslağı" }, conflict: null },
  ].map((op) => ({ ...op, payload: { target_quote: op.contact.id, operation_id: op.id } }));
  const central = {
    status: 200, exists: true, beforeRead: null, reads: 0, beforeSync: null, synced: [], requests: [],
    membership: { role: "editor", status: 200, beforeRead: null, reads: 0 },
    quote: {
      ...draft, currency: "TRY", fx_rate: null, fx_date: null, valid_until: "2026-12-10",
      revision: 4, note: "Ortak teklif", updated_at: "2026-11-24T09:00:00Z", team_contacts: { name: "Ortak kişi" },
      team_quote_items: [{ id: "line", sku: null, name: "Ortak kalem", qty: 1, unit_price: 120, tax_rate: 20, cost: null, sort: 0 }],
      team_quote_versions: [{ count: 0 }],
    },
  };
  const writes = [];
  const archive = {
    beforeRead: null, reads: 0, entries: [
      { id: "older-resolution", saved_at: null, local: { ...draft, note: "Saklanan teklif" }, remote: null },
    ],
  };
  let compareResult = true;
  const client = createClient(project, "test-key", {
    auth: { persistSession: false, autoRefreshToken: false, detectSessionInUrl: false },
    global: {
      async fetch(input, init) {
        const url = new URL(input instanceof Request ? input.url : input);
        assert.equal(url.origin, project);
        if (url.pathname === "/rest/v1/rpc/team_sync_quote") {
          assert.equal(init.method, "POST");
          const body = JSON.parse(init.body);
          central.synced.push(body.target_quote);
          central.requests.push(body);
          const result = await central.beforeSync?.(body);
          if (result) return result;
          return Response.json({});
        }
        if (url.pathname === "/rest/v1/workspace_members") {
          assert.equal(init.method, "GET");
          assert.equal(url.searchParams.get("workspace_id"), `eq.${workspace}`);
          assert.equal(url.searchParams.get("user_id"), `eq.${user}`);
          central.membership.reads++;
          await central.membership.beforeRead?.();
          if (central.membership.status !== 200) {
            return Response.json({ code: "42501", message: "private backend detail" }, { status: central.membership.status });
          }
          return Response.json([{ role: central.membership.role }]);
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
        if (args.request.action === "quote_conflict_history") {
          assert.equal(args.request.quote_id, quoteId);
          archive.reads++;
          await archive.beforeRead?.();
          return structuredClone(archive.entries);
        }
        if (args.request.action === "pending") return structuredClone(operations);
        if (args.request.action === "read") {
          assert.equal(args.request.key, "can-edit");
          return true;
        }
        if (args.request.action === "acknowledge") {
          const index = operations.findIndex(({ id }) => id === args.request.id);
          assert.notEqual(index, -1);
          operations.splice(index, 1);
          return null;
        }
        if (args.request.action === "conflict") {
          operations.find(({ id }) => id === args.request.id).conflict = args.request.reason;
          return null;
        }
        const reapplies = args.request.action === "reapply_quote_conflict";
        if (legacyRuntime && reapplies) {
          throw "invalid args `request` for command `team_cache`: unknown variant `reapply_quote_conflict`";
        }
        assert.equal(args.request.action, reapplies ? "reapply_quote_conflict" : "resolve_quote_conflict");
        assert.equal(Object.hasOwn(args.request, "retry"), reapplies);
        assert.equal(args.request.quote_id, quoteId);
        writes.push(structuredClone(args.request));
        if (compareResult && reapplies) {
          for (const id of args.request.expected_ids) operations.splice(operations.findIndex((op) => op.id === id), 1);
          operations.push({ ...structuredClone(args.request.retry), conflict: null });
        }
        return compareResult;
      },
    },
  } });
  connection.teamProject.value = project;
  connection.teamWorkspace.value = workspace;
  connection.teamSession.value = { user: { id: user } };
  connection.teamClient.value = client;
  connection.teamReachable.value = true;
  teamQuoteCanEdit.value = true;
  t.after(() => {
    connection.teamAccessIssue.value = false;
    connection.teamClient.value = null;
    connection.teamSession.value = null;
    connection.teamWorkspace.value = "";
    connection.teamProject.value = "";
    connection.teamReachable.value = null;
    teamQuoteCanEdit.value = false;
    if (previousWindow) Object.defineProperty(globalThis, "window", previousWindow);
    else Reflect.deleteProperty(globalThis, "window");
  });
  return { central, writes, operations, archive, failCompare: () => { compareResult = false; } };
}

test("opens a legacy conflict archive offline using only the quote's SQLite scope", async (t) => {
  const { central, archive, writes } = fixture(t);
  connection.teamReachable.value = false;
  const entries = await teamQuotes.conflictHistory(quoteId);
  assert.deepEqual(entries, archive.entries);
  assert.equal(entries[0].saved_at, null);
  assert.equal(entries[0].remote, null);
  assert.equal(archive.reads, 1);
  assert.equal(central.reads, 0);
  assert.deepEqual(writes, []);
});

test("does not show archived quotes when workspace changes or access is lost during a read", async (t) => {
  const { archive } = fixture(t);
  archive.beforeRead = () => { connection.teamWorkspace.value = other; };
  await assert.rejects(teamQuotes.conflictHistory(quoteId), { name: "AbortError" });
  connection.teamWorkspace.value = workspace;
  archive.beforeRead = () => { connection.teamAccessIssue.value = true; };
  await assert.rejects(teamQuotes.conflictHistory(quoteId), /tekliflerine erişilemiyor/u);
  await assert.rejects(teamQuotes.conflictHistory(quoteId), /tekliflerine erişilemiyor/u);
  assert.equal(archive.reads, 2);
});

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
  assert.equal(writes[0].action, "resolve_quote_conflict");
  assert.equal(Object.hasOwn(writes[0], "retry"), false);
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

test("retries only selected local fields against the reviewed draft and keeps unselected remote lines", async (t) => {
  const { central, writes, operations } = fixture(t);
  const review = await teamQuotes.inspectConflict(quoteId);
  const result = await teamQuotes.reapplyConflict(review, ["note"]);
  assert.equal(result.sync, "synced");
  assert.equal(result.quote.revision, 5);
  assert.equal(result.quote.note, review.local.note);
  assert.equal(result.quote.currency, "TRY");
  assert.equal(central.reads, 2);
  assert.equal(writes[0].action, "reapply_quote_conflict");
  assert.deepEqual(writes[0].expected_ids, ["rejected", "queued"]);
  assert.equal(writes[0].remote.note, "Ortak teklif");
  const body = central.requests.find((value) => value.target_quote === quoteId);
  assert.equal(body.expected_revision, 4);
  assert.equal(body.operation_id, writes[0].retry.id);
  assert.equal(body.fields.note, review.local.note);
  assert.equal(body.fields.valid_until, "2026-12-10");
  assert.deepEqual(body.items, [{ id: "line", sku: null, name: "Ortak kalem", qty: 1, unit_price: 120, tax_rate: 20, cost: null }]);
  assert.deepEqual(operations, []);
});

test("keeps a second conflict queued when the checked revision changes before the RPC", async (t) => {
  const { central, operations } = fixture(t);
  central.beforeSync = (body) => {
    if (body.target_quote !== quoteId) return;
    central.quote.revision++;
    return Response.json({ code: "PT409", message: "Revision conflict" }, { status: 409 });
  };
  const review = await teamQuotes.inspectConflict(quoteId);
  const result = await teamQuotes.reapplyConflict(review, ["pricing"]);
  assert.equal(result.sync, "conflict");
  assert.equal(result.quote.currency, "USD");
  assert.equal(result.quote.fx_rate, 40);
  assert.deepEqual(result.quote.items, []);
  assert.equal(result.quote.note, "Ortak teklif");
  assert.equal(operations.length, 1);
  assert.equal(operations[0].conflict, "Revision conflict");
  assert.equal(operations[0].payload.expected_revision, 4);
  assert.equal((await teamQuotes.inspectConflict(quoteId)).remote.revision, 5);
});

test("rejects non-draft and unchanged field retries before replacing the queue", async (t) => {
  const { central, writes } = fixture(t);
  await assert.rejects(teamQuotes.reapplyConflict(await teamQuotes.inspectConflict(quoteId), ["number"]), /alanları seçin/);
  central.quote.status = "sent";
  await assert.rejects(teamQuotes.reapplyConflict(await teamQuotes.inspectConflict(quoteId), ["note"]), /taslak olması/);
  assert.deepEqual(writes, []);
  assert.deepEqual(central.requests, []);
});

test("cached editor cannot reapply after demotion or live RLS denial", async (t) => {
  const { central, writes, operations } = fixture(t);
  const before = structuredClone(operations);
  const review = await teamQuotes.inspectConflict(quoteId);
  assert.equal(teamQuoteCanEdit.value, true);
  central.membership.role = "viewer";
  await assert.rejects(teamQuotes.reapplyConflict(review, ["note"]), /yetkinizi/);
  central.membership.status = 403;
  await assert.rejects(teamQuotes.reapplyConflict(review, ["note"]), /yetkinizi/);
  assert.equal(central.membership.reads, 2);
  assert.deepEqual(writes, []);
  assert.deepEqual(central.requests, []);
  assert.deepEqual(operations, before);
});

test("rejects reapply against an older desktop runtime without changing the queue", async (t) => {
  const { central, writes, operations } = fixture(t, { legacyRuntime: true });
  const before = structuredClone(operations);
  const review = await teamQuotes.inspectConflict(quoteId);
  await assert.rejects(teamQuotes.reapplyConflict(review, ["note"]), /güncel masaüstü sürümü gerekiyor/);
  assert.deepEqual(writes, []);
  assert.deepEqual(central.requests, []);
  assert.deepEqual(operations, before);
  await teamQuotes.acceptConflict(review);
  assert.equal(writes[0].action, "resolve_quote_conflict");
});

test("cancels the retry if the workspace changes while checking membership", async (t) => {
  const { central, writes, operations } = fixture(t);
  const before = structuredClone(operations);
  const review = await teamQuotes.inspectConflict(quoteId);
  const started = Promise.withResolvers();
  const resume = Promise.withResolvers();
  central.membership.beforeRead = () => { started.resolve(); return resume.promise; };
  const controller = new AbortController();
  const retry = teamQuotes.reapplyConflict(review, ["note"], controller.signal);
  await started.promise;
  connection.teamWorkspace.value = "10000000-0000-4000-8000-000000000002";
  controller.abort();
  resume.resolve();
  await assert.rejects(retry, { name: "AbortError" });
  assert.deepEqual(writes, []);
  assert.deepEqual(central.requests, []);
  assert.deepEqual(operations, before);
});

test("retains the same retry operation when the connection fails after local commit", async (t) => {
  const { central, operations, writes } = fixture(t);
  central.beforeSync = (body) => {
    if (body.target_quote === quoteId) return Response.json({ message: "Temporary error" }, { status: 500 });
  };
  const review = await teamQuotes.inspectConflict(quoteId);
  const result = await teamQuotes.reapplyConflict(review, ["note"]);
  assert.equal(result.sync, "pending");
  assert.equal(operations[0].id, writes[0].retry.id);
  assert.equal(operations[0].conflict, null);
  const firstAttempt = structuredClone(operations[0].payload);
  central.beforeSync = null;
  await flushTeamQuotes();
  assert.deepEqual(central.requests.at(-1), firstAttempt);
  assert.deepEqual(operations, []);
});
