import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";
import { createClient } from "@supabase/supabase-js";
import { createServer } from "vite";

const project = "https://product-seo-conflict.example.test";
const workspace = "10000000-0000-4000-8000-000000000001";
const user = "20000000-0000-4000-8000-000000000001";
const sku = "PRODUCT/1";
const other = "PRODUCT/2";
const scope = JSON.stringify([project, user, workspace, "product-seo"]);
let vite, connection, seo;

before(async () => {
  vite = await createServer({
    root: fileURLToPath(new URL("..", import.meta.url)), configFile: false, appType: "custom",
    server: { middlewareMode: true, hmr: false, watch: null }, logLevel: "silent",
  });
  connection = await vite.ssrLoadModule("/src/teamConnection.ts");
  seo = await vite.ssrLoadModule("/src/teamProductSeo.ts");
});
after(async () => { await vite?.close(); });

function operation(id, sku, revision, draft_title, conflict = null) {
  return { id, contact: { id: sku, sku, revision, draft: { draft_title } }, conflict,
    payload: { target_workspace: workspace, target_sku: sku, operation_id: id } };
}

function fixture(t) {
  const operations = [
    operation("rejected", sku, 2, "Çakışan başlık", "revision changed"),
    operation("other", other, 2, "Başka ürün"),
    operation("queued", sku, 3, "Kuyruktaki son düzenleme"),
  ];
  const central = { row: { sku, revision: 4, draft: { draft_title: "Ortak başlık" } }, reads: 0,
    status: 200, beforeRead: null };
  const writes = [];
  const history = [];
  const client = createClient(project, "test-key", {
    auth: { persistSession: false, autoRefreshToken: false, detectSessionInUrl: false },
    global: { async fetch(input, init) {
      const url = new URL(input instanceof Request ? input.url : input);
      assert.equal(url.pathname, "/rest/v1/team_product_seo");
      assert.equal(init.method, "GET");
      assert.equal(url.searchParams.get("workspace_id"), `eq.${workspace}`);
      assert.equal(url.searchParams.get("sku"), `eq.${sku}`);
      central.reads++;
      await central.beforeRead?.();
      if (central.status !== 200) return Response.json({ message: "backend detail" }, { status: central.status });
      return Response.json(central.row ? [central.row] : []);
    } },
  });
  const previousWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
  Object.defineProperty(globalThis, "window", { configurable: true, value: { __TAURI_INTERNALS__: {
    invoke: async (command, { scope: actualScope, request }) => {
      assert.equal(command, "team_cache");
      assert.equal(actualScope, scope);
      switch (request.action) {
        case "pending": return structuredClone(operations);
        case "contacts": return [];
        case "read":
          assert.equal(request.key, `product-seo-conflict:${sku}`);
          return structuredClone(history);
        case "resolve_product_seo_conflict": {
          writes.push(structuredClone(request));
          const matching = operations.filter((op) => op.contact.sku === request.sku);
          if (JSON.stringify(matching.map((op) => op.id)) !== JSON.stringify(request.expected_ids)) return false;
          const local = structuredClone(matching.at(-1).contact);
          Object.assign(local.draft, request.unsaved);
          history.unshift({ saved_at: 1, local, remote: request.remote, operations: matching });
          for (const index of matching.map((op) => operations.indexOf(op)).reverse()) operations.splice(index, 1);
          return true;
        }
        default: assert.fail(`Beklenmeyen işlem: ${request.action}`);
      }
    },
  } } });
  connection.teamProject.value = project;
  connection.teamWorkspace.value = workspace;
  connection.teamSession.value = { user: { id: user } };
  connection.teamClient.value = client;
  connection.teamReachable.value = true;
  t.after(() => {
    connection.teamReachable.value = null;
    connection.teamClient.value = null;
    connection.teamSession.value = null;
    connection.teamWorkspace.value = "";
    connection.teamProject.value = "";
    if (previousWindow) Object.defineProperty(globalThis, "window", previousWindow);
    else Reflect.deleteProperty(globalThis, "window");
  });
  return { operations, central, writes };
}

test("compares the latest scoped queued draft with a fresh shared revision and archives buffered edits", async (t) => {
  const { operations, central, writes } = fixture(t);
  connection.teamReachable.value = false;
  await seo.listTeamProductSeo();
  assert.equal(seo.productSeoHasConflict(sku), true);
  assert.equal(seo.productSeoHasConflict(other), false);
  connection.teamReachable.value = true;
  const unsaved = { draft_title: "Henüz kuyruğa girmeyen başlık", tech_source_text: "Teknik not" };
  const review = await seo.inspectTeamProductSeoConflict(sku, unsaved);
  assert.equal(review.remoteState, "ready");
  assert.deepEqual(review.operationIds, ["rejected", "queued"]);
  assert.deepEqual(review.local.draft, unsaved);
  assert.equal(review.remote.draft.draft_title, "Ortak başlık");
  assert.deepEqual(writes, []);
  assert.equal(await seo.acceptTeamProductSeoConflict(review), true);
  assert.equal(central.reads, 2);
  assert.deepEqual(writes[0].expected_ids, review.operationIds);
  assert.deepEqual(writes[0].unsaved, unsaved);
  assert.deepEqual(operations.map((op) => op.id), ["other"]);
  assert.equal(seo.productSeoHasConflict(sku), false);
  const history = await seo.teamProductSeoConflictHistory(sku);
  assert.deepEqual(history[0].local.draft, unsaved);
});

test("keeps the local queue when the shared draft changed during review", async (t) => {
  const { operations, central, writes } = fixture(t);
  const review = await seo.inspectTeamProductSeoConflict(sku);
  central.row = { ...central.row, revision: 5 };
  await assert.rejects(seo.acceptTeamProductSeoConflict(review), /inceleme sırasında değişti/);
  assert.equal(writes.length, 0);
  assert.equal(operations.length, 3);
});

test("rejects adoption if another local operation arrived after the review", async (t) => {
  const { operations } = fixture(t);
  const review = await seo.inspectTeamProductSeoConflict(sku);
  operations.push(operation("late", sku, 4, "Yeni yerel taslak"));
  assert.equal(await seo.acceptTeamProductSeoConflict(review), false);
  assert.equal(operations.length, 4);
});

test("shows the local draft offline and preserves it if the shared record is missing or unavailable", async (t) => {
  const { central, writes } = fixture(t);
  connection.teamReachable.value = false;
  const offline = await seo.inspectTeamProductSeoConflict(sku);
  assert.equal(offline.remoteState, "offline");
  assert.equal(offline.local.revision, 3);
  await assert.rejects(seo.acceptTeamProductSeoConflict(offline), /bağlantı varken yenileyin/u);
  connection.teamReachable.value = true;
  central.row = null;
  assert.equal((await seo.inspectTeamProductSeoConflict(sku)).remoteState, "missing");
  central.status = 503;
  assert.equal((await seo.inspectTeamProductSeoConflict(sku)).remoteState, "unavailable");
  assert.deepEqual(writes, []);
});

test("does not commit a resolved draft after the review is canceled", async (t) => {
  const { central, writes } = fixture(t);
  const review = await seo.inspectTeamProductSeoConflict(sku);
  const controller = new AbortController();
  central.beforeRead = () => { controller.abort(); };
  await assert.rejects(seo.acceptTeamProductSeoConflict(review, controller.signal), { name: "AbortError" });
  assert.deepEqual(writes, []);
});

test("invalidates a review if the workspace changes while fetching the shared draft", async (t) => {
  const { central, writes } = fixture(t);
  const review = await seo.inspectTeamProductSeoConflict(sku);
  central.beforeRead = () => { connection.teamWorkspace.value = "10000000-0000-4000-8000-000000000002"; };
  await assert.rejects(seo.acceptTeamProductSeoConflict(review), /Ekip bağlantısı değişti/);
  assert.deepEqual(writes, []);
  assert.equal(seo.productSeoHasConflict(sku), false);
});
