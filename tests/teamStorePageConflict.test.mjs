import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";
import { createClient } from "@supabase/supabase-js";
import { createServer } from "vite";

const project = "https://store-page-conflict.example.test";
const workspace = "10000000-0000-4000-8000-000000000001";
const user = "20000000-0000-4000-8000-000000000001";
const scope = JSON.stringify([project, user, workspace, "store-pages"]);
const remoteId = 71;
let vite, connection, pages;

before(async () => {
  vite = await createServer({
    root: fileURLToPath(new URL("..", import.meta.url)), configFile: false, appType: "custom",
    server: { middlewareMode: true, hmr: false, watch: null }, logLevel: "silent",
  });
  connection = await vite.ssrLoadModule("/src/teamConnection.ts");
  pages = await vite.ssrLoadModule("/src/teamStorePages.ts");
});
after(async () => { await vite?.close(); });

function fixture(t, kind = "category", hasDrafts = true) {
  const id = `${kind}:${remoteId}`;
  const page = { id, kind, remote_id: remoteId, revision: 3, team_scope: scope, status: 1, slug: "ornek", name: "Örnek",
    page_title: "Mağazada görünen başlık", meta_description: "", target_keyword: "", showcase_content: null,
    draft_page_title: "Cihazdaki son başlık", draft_meta_description: "Cihazdaki açıklama", draft_target_keyword: "",
    draft_showcase: "", draft_model: "", draft_at: "2026-11-25T10:00:00", pushed_at: "", queries: [], impressions: 0, clicks: 0 };
  const operations = [];
  function draft(opId, revision, conflict = null, record = page) {
    return { id: opId, contact: { ...record, revision }, conflict, payload: { target_workspace: workspace,
      target_kind: record.kind, target_id: record.remote_id, operation_id: opId, expected_revision: revision - 1,
      changes: { draft_page_title: record.draft_page_title } } };
  }
  if (hasDrafts) {
    operations.push(draft("rejected", 2, "revision changed"));
    operations.push({ id: "receipt", contact: { ...page, revision: 2 }, conflict: "receipt temporarily rejected",
      payload: { facts: { status: 1, page_title: "Mağaza kaydı" }, receipt: "fixture" } });
    operations.push(draft("another-kind", 2, "revision changed", { ...page, id: `blog:${remoteId}`, kind: "blog" }));
    operations.push(draft("latest", 3));
  }
  const central = { row: { remote_id: remoteId, revision: 5, details: {
    ...page, draft_page_title: "Ortak başlık", draft_meta_description: "Ortak açıklama",
  } }, status: 200, reads: 0, beforeRead: null };
  const writes = [];
  const history = [];
  let cachedRows = [];
  const testClient = createClient(project, "test-key", {
    auth: { persistSession: false, autoRefreshToken: false, detectSessionInUrl: false },
    global: { async fetch(input, init) {
      const url = new URL(input instanceof Request ? input.url : input);
      assert.equal(init.method, "GET");
      assert.equal(url.searchParams.get("workspace_id"), `eq.${workspace}`);
      if (url.pathname === "/rest/v1/workspace_members") {
        assert.equal(url.searchParams.get("user_id"), `eq.${user}`);
        return Response.json([{ role: "editor" }]);
      }
      assert.equal(url.pathname, "/rest/v1/team_store_pages");
      assert.equal(url.searchParams.get("kind"), `eq.${kind}`);
      if (url.searchParams.has("remote_id")) assert.equal(url.searchParams.get("remote_id"), `eq.${remoteId}`);
      else assert.equal(url.searchParams.get("order"), "remote_id.asc");
      const row = structuredClone(central.row);
      central.reads++;
      await central.beforeRead?.();
      if (central.status !== 200) return Response.json({ message: "backend detail" }, { status: central.status });
      return Response.json(row ? [row] : []);
    } },
  });
  const previousWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
  Object.defineProperty(globalThis, "window", { configurable: true, value: { __TAURI_INTERNALS__: {
    invoke: async (command, { scope: actualScope, request }) => {
      assert.equal(command, "team_cache");
      assert.equal(actualScope, scope);
      switch (request.action) {
        case "pending": return structuredClone(operations);
        case "read":
          if (request.key === kind) return structuredClone(cachedRows);
          assert.equal(request.key, `store-page-conflict:${id}`);
          return structuredClone(history);
        case "write":
          if (request.key === kind) cachedRows = structuredClone(request.data);
          else assert.ok(["available", "can-edit"].includes(request.key));
          return null;
        case "resolve_store_page_conflict": {
          writes.push(structuredClone(request));
          assert.equal(request.kind, kind);
          assert.equal(request.remote_id, remoteId);
          assert.equal(request.remote.id, id);
          const matching = operations.filter((op) => op.contact.id === id && !("facts" in op.payload));
          if (JSON.stringify(matching.map((op) => op.id)) !== JSON.stringify(request.expected_ids)) return false;
          const local = structuredClone(matching.at(-1)?.contact ?? request.remote);
          Object.assign(local, request.unsaved);
          history.unshift({ saved_at: "2026-11-25T10:00:00Z", local, remote: request.remote, operations: matching });
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
  connection.teamClient.value = testClient;
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
  return { page, operations, draft, central, writes };
}

test("compares and archives only the targeted draft queue and keeps a receipt and another kind", async (t) => {
  const { page, operations, central, writes } = fixture(t);
  const unsaved = { draft_meta_description: "Henüz kaydedilmemiş metin" };
  const review = await pages.inspectTeamStorePageConflict(page, unsaved);
  assert.equal(review.remoteState, "ready");
  assert.deepEqual(review.operationIds, ["rejected", "latest"]);
  assert.equal(review.local.draft_page_title, "Cihazdaki son başlık");
  assert.equal(review.local.draft_meta_description, unsaved.draft_meta_description);
  assert.equal(review.remote.draft_page_title, "Ortak başlık");
  assert.equal(writes.length, 0);
  const adopted = await pages.acceptTeamStorePageConflict(review);
  assert.equal(adopted.revision, 5);
  assert.equal(adopted.team_draft_conflict, false);
  assert.equal(adopted.team_conflict, true);
  assert.equal(central.reads, 2);
  assert.deepEqual(operations.map(({ id }) => id), ["receipt", "another-kind"]);
  const archive = await pages.teamStorePageConflictHistory(page);
  assert.equal(archive[0].local.draft_meta_description, unsaved.draft_meta_description);
  assert.deepEqual(archive[0].operations.map(({ id }) => id), ["rejected", "latest"]);
});

test("retains a queued brand draft when the shared revision changes again", async (t) => {
  const { page, central, operations, writes } = fixture(t, "brand");
  const review = await pages.inspectTeamStorePageConflict(page, {});
  central.row.revision++;
  await assert.rejects(pages.acceptTeamStorePageConflict(review));
  assert.equal(writes.length, 0);
  assert.equal(operations.length, 4);
});

test("previews an unsaved blog edit over the current shared draft and can archive it with no queue", async (t) => {
  const { page, writes } = fixture(t, "blog", false);
  const review = await pages.inspectTeamStorePageConflict(page, { draft_page_title: "Kaydedilmemiş blog başlığı" });
  assert.deepEqual(review.operationIds, []);
  assert.equal(review.local.draft_page_title, "Kaydedilmemiş blog başlığı");
  assert.equal(review.local.draft_meta_description, "Ortak açıklama");
  assert.equal((await pages.acceptTeamStorePageConflict(review)).draft_page_title, "Ortak başlık");
  assert.deepEqual(writes[0].expected_ids, []);
  assert.equal((await pages.teamStorePageConflictHistory(page))[0].local.draft_page_title, "Kaydedilmemiş blog başlığı");
});

test("shows a queued draft when the shared record is unavailable and never applies it as a resolution", async (t) => {
  const { page, central, writes } = fixture(t);
  connection.teamReachable.value = false;
  const list = await pages.listTeamStorePages(page.kind);
  assert.equal(list[0].team_draft_conflict, true);
  const offline = await pages.inspectTeamStorePageConflict(page, {});
  assert.equal(offline.remoteState, "offline");
  assert.equal(offline.local.draft_page_title, "Cihazdaki son başlık");
  await assert.rejects(pages.acceptTeamStorePageConflict(offline));
  connection.teamReachable.value = true;
  central.row = null;
  assert.equal((await pages.inspectTeamStorePageConflict(page, {})).remoteState, "missing");
  central.status = 503;
  assert.equal((await pages.inspectTeamStorePageConflict(page, {})).remoteState, "unavailable");
  assert.equal(writes.length, 0);
});

test("blocks a previously inspected draft after changing workspaces during verification", async (t) => {
  const { page, central, writes } = fixture(t);
  const review = await pages.inspectTeamStorePageConflict(page, {});
  central.beforeRead = () => { connection.teamWorkspace.value = "10000000-0000-4000-8000-000000000002"; };
  await assert.rejects(pages.acceptTeamStorePageConflict(review));
  assert.equal(writes.length, 0);
});

test("keeps the newer snapshot when overlapping lists finish in reverse order", async (t) => {
  const { page, central } = fixture(t, "category", false);
  let firstStarted, finishFirst;
  const started = new Promise((resolve) => { firstStarted = resolve; });
  const finish = new Promise((resolve) => { finishFirst = resolve; });
  central.beforeRead = () => {
    if (central.reads === 1) {
      firstStarted();
      return finish;
    }
  };
  const older = pages.listTeamStorePages(page.kind);
  try {
    await started;
    central.row = { ...central.row, revision: 6 };
    assert.equal((await pages.listTeamStorePages(page.kind))[0].revision, 6);
  } finally {
    finishFirst();
  }
  assert.equal((await older)[0].revision, 6);
  connection.teamReachable.value = false;
  assert.equal((await pages.listTeamStorePages(page.kind))[0].revision, 6);
});
