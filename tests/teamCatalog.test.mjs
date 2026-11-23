import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";
import { createClient } from "@supabase/supabase-js";
import { createPinia } from "pinia";
import { createServer } from "vite";

const project = "https://catalog.example.test";
const workspace = "11111111-1111-4111-8111-111111111111";
const user = "22222222-2222-4222-8222-222222222222";
const product = (sku, name, quantity) => ({ sku, name, facts: { quantity } });
let vite, connection, useStore;

before(async () => {
  vite = await createServer({
    root: fileURLToPath(new URL("..", import.meta.url)),
    configFile: false,
    appType: "custom",
    server: { middlewareMode: true, hmr: false, watch: null },
    logLevel: "silent",
  });
  connection = await vite.ssrLoadModule("/src/teamConnection.ts");
  ({ useStore } = await vite.ssrLoadModule("/src/store.ts"));
});

after(async () => { await vite?.close(); });

function fixture(t) {
  const remote = { revision: 4, products: [product("A", "İlk ad", 3)], beforeRevision: null };
  let saved = { revision: remote.revision, products: structuredClone(remote.products) };
  const requests = [];
  const commands = [];
  const client = createClient(project, "test-project-key", {
    auth: { persistSession: false, autoRefreshToken: false, detectSessionInUrl: false },
    global: {
      async fetch(input) {
        const url = new URL(String(input));
        assert.equal(url.origin, project);
        requests.push(url);
        if (url.pathname === "/rest/v1/team_catalog_state") {
          const revision = remote.revision;
          await remote.beforeRevision?.();
          return Response.json([{ revision }]);
        }
        if (url.pathname === "/rest/v1/team_catalog") return Response.json(remote.products);
        if (url.pathname === "/rest/v1/workspace_members") return Response.json([{ role: "editor" }]);
        if (["/rest/v1/team_product_seo", "/rest/v1/team_seo_work_events"].includes(url.pathname)) {
          return Response.json([]);
        }
        assert.fail(`Unexpected request ${url.pathname}`);
      },
    },
  });
  const currentWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
  Object.defineProperty(globalThis, "window", { configurable: true, value: {
    __TAURI_INTERNALS__: {
      async invoke(command, args) {
        commands.push(command);
        if (command === "team_cache") {
          assert.equal(JSON.parse(args.scope)[2], workspace);
          if (args.request.action === "pending") return [];
          if (args.request.key === "catalog") {
            if (args.request.action === "read") return saved;
            if (args.request.action === "write") saved = args.request.data;
          }
          return null;
        }
        if (command === "list_team_products") return args.products;
        if (command === "get_team_product") {
          return {
            sku: args.product.sku, name: args.product.name, quantity: args.product.facts.quantity,
            title: "Feed başlığı", draft_title: "Sunucudaki taslak", tech_source_text: "Sunucudaki teknik metin",
            seo_revision: 9,
          };
        }
        assert.fail(`Unexpected command ${command}`);
      },
    },
  } });
  t.after(() => {
    connection.teamSession.value = null;
    connection.teamClient.value = null;
    connection.teamWorkspace.value = "";
    connection.teamProject.value = "";
    connection.teamReachable.value = null;
    if (currentWindow) Object.defineProperty(globalThis, "window", currentWindow);
    else Reflect.deleteProperty(globalThis, "window");
  });
  connection.teamProject.value = project;
  connection.teamWorkspace.value = workspace;
  connection.teamSession.value = { user: { id: user } };
  connection.teamClient.value = client;
  connection.teamReachable.value = true;
  const store = useStore(createPinia());
  store.page = "products";
  store.allRows = structuredClone(remote.products);
  store.productCatalogRevision = 4;
  store.selectedSku = "A";
  store.detail = {
    sku: "A", name: "İlk ad", quantity: 3, draft_title: "Kaydedilmemiş meta",
    tech_source_text: "Kaydedilmemiş teknik metin", seo_revision: 2, team_catalog_revision: 4,
  };
  return { remote, requests, commands, store };
}

test("unchanged catalogue revision does not read rows or the SQLite cache", async (t) => {
  const { store, requests, commands } = fixture(t);
  await store.refreshOpenCatalog();
  assert.deepEqual(requests.map((url) => url.pathname), ["/rest/v1/team_catalog_state"]);
  assert.equal(requests[0].searchParams.get("select"), "revision");
  assert.equal(requests[0].searchParams.get("workspace_id"), `eq.${workspace}`);
  assert.deepEqual(commands, []);
});

test("changed catalogue refreshes feed fields without replacing the selected detail or its drafts", async (t) => {
  const { store, remote, requests } = fixture(t);
  const detail = store.detail;
  remote.revision = 5;
  remote.products = [product("A", "Güncel ad", 7), product("B", "Yeni ürün", 1)];

  await store.refreshOpenCatalog();

  assert.equal(store.productCatalogRevision, 5);
  assert.deepEqual(store.allRows.map(({ sku }) => sku), ["A", "B"]);
  assert.equal(store.detail, detail);
  assert.equal(detail.name, "Güncel ad");
  assert.equal(detail.quantity, 7);
  assert.equal(detail.title, "Feed başlığı");
  assert.equal(detail.team_catalog_revision, 5);
  assert.equal(detail.draft_title, "Kaydedilmemiş meta");
  assert.equal(detail.tech_source_text, "Kaydedilmemiş teknik metin");
  assert.equal(detail.seo_revision, 2);
  assert.equal(requests.filter((url) => url.pathname === "/rest/v1/team_catalog").length, 1);
});

test("an offline or obsolete scope probe leaves the displayed catalogue untouched", { timeout: 5_000 }, async (t) => {
  const { store, remote, requests, commands } = fixture(t);
  connection.teamReachable.value = false;
  await store.refreshOpenCatalog();
  assert.deepEqual(requests, []);
  connection.teamReachable.value = true;
  remote.revision = 5;
  const started = Promise.withResolvers();
  const finish = Promise.withResolvers();
  remote.beforeRevision = () => {
    started.resolve();
    return finish.promise;
  };
  const check = store.refreshOpenCatalog();
  await started.promise;
  connection.teamWorkspace.value = "33333333-3333-4333-8333-333333333333";
  finish.resolve();
  await check;
  assert.equal(store.productCatalogRevision, 4);
  assert.deepEqual(requests.map((url) => url.pathname), ["/rest/v1/team_catalog_state"]);
  assert.deepEqual(commands, []);
});
