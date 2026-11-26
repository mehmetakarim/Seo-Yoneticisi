import { test } from "node:test";
import assert from "node:assert/strict";
import { handleIdeasoft } from "./handler.ts";

const workspace = "11111111-1111-4111-8111-111111111111";
const user = "22222222-2222-4222-8222-222222222222";
const anotherUser = "33333333-3333-4333-8333-333333333333";
const configuredOrigin = "https://merchant.example.com";
type MerchantFetch = (url: string, init?: RequestInit) => Response | Promise<Response>;
function fixture(options: { role?: string; member?: boolean; configured?: boolean; origin?: string; fallbackToken?: string | null; remoteStatus?: number; remoteBody?: unknown; expired?: boolean; merchant?: MerchantFetch; rpc?: MerchantFetch; tokenRpc?: MerchantFetch; pageRpc?: MerchantFetch; pageState?: MerchantFetch; inCatalog?: boolean } = {}) {
  const calls: { url: string; init?: RequestInit }[] = [];
  const env = (key: string) => ({
    SUPABASE_URL: "https://project.supabase.co", SUPABASE_ANON_KEY: "public-key", SUPABASE_SECRET_KEYS: '{"default":"sb_secret_test-only"}',
    [`TEAM_IDEASOFT_${workspace.replaceAll("-", "_")}`]: options.configured === false ? undefined : JSON.stringify({ origin: options.origin ?? configuredOrigin, token: options.fallbackToken === undefined ? "server-only-token" : options.fallbackToken }),
  })[key];
  const request: typeof fetch = async (input, init) => {
    const url = String(input);
    calls.push({ url, init });
    if (url.endsWith("/auth/v1/user")) return Response.json({
      id: new Headers(init?.headers).get("Authorization") === "Bearer user-session" ? user : anotherUser,
    }, { status: options.expired ? 401 : 200 });
    if (url.includes("/workspace_members?")) return Response.json(options.member === false ? [] : [{ role: options.role ?? "editor" }]);
    if (url.includes("/rest/v1/team_catalog?")) return Response.json(options.inCatalog === false ? [] : [{ sku: "SKU-1" }]);
    if (url.endsWith("/rpc/team_record_ideasoft_push")) return options.rpc ? options.rpc(url, init) : Response.json(null);
    if (url.includes("/rest/v1/team_store_pages?")) return options.pageState ? options.pageState(url, init) : Response.json([]);
    if (/\/rpc\/team_(?:confirm_ideasoft_page(?:_publication)?|claim_ideasoft_page_create)$/.test(url)) {
      return options.pageRpc ? options.pageRpc(url, init) : Response.json(null, { status: 503 });
    }
    if (url.endsWith("/rpc/team_get_ideasoft_token") || url.endsWith("/rpc/team_set_ideasoft_token")) {
      return options.tokenRpc ? options.tokenRpc(url, init) : Response.json(null);
    }
    if (options.merchant) return options.merchant(url, init);
    return Response.json(options.remoteBody ?? [{ id: 7, sku: "SKU-1", name: "Product" }], { status: options.remoteStatus ?? 200 });
  };
  const run = (body: object = { operation: "health" }, authorized: boolean | string = true) => handleIdeasoft(new Request("https://function.example.com", {
    method: "POST", headers: authorized ? { Authorization: `Bearer ${authorized === true ? "user-session" : authorized}` } : {}, body: JSON.stringify({ workspace, ...body }),
  }), env, request);
  return { run, calls };
}

test("missing or expired login never calls the merchant", async () => {
  const f = fixture();
  assert.equal((await f.run({}, false)).status, 401);
  assert.equal(f.calls.length, 0);
  const expired = fixture({ expired: true });
  assert.equal((await expired.run()).status, 401);
  assert.equal(expired.calls.length, 1);
});

test("nonmembers and viewers cannot use merchant integration credentials", async () => {
  for (const options of [{ member: false }, { role: "viewer" }]) {
    const f = fixture(options);
    assert.equal((await f.run()).status, 403);
    assert.equal(f.calls.length, 2);
  }
});

test("missing workspace configuration does not fall back to another workspace", async () => {
  const f = fixture({ configured: false });
  assert.deepEqual(await (await f.run()).json(), { configured: false });
  assert.equal(f.calls.length, 2);
});

test("health only reads the configured merchant and does not return its data or credential", async () => {
  const f = fixture();
  assert.deepEqual(await (await f.run({ operation: "health", url: "https://attacker.example.com", token: "attacker" })).json(), { configured: true, connected: true });
  const merchant = f.calls[3];
  assert.equal(merchant.url, "https://merchant.example.com/admin-api/products?limit=1");
  assert.equal(merchant.init?.method, "GET");
  assert.equal(merchant.init?.redirect, "error");
  assert.equal(new Headers(merchant.init?.headers).get("Authorization"), "Bearer server-only-token");
});

test("unsafe configured endpoints are rejected before sending credentials", async () => {
  for (const origin of ["http://merchant.example.com", "https://127.0.0.1", "https://[::1]", "https://localhost", "https://a.internal", "https://u:p@merchant.example.com", "https://merchant.example.com:444", "https://merchant.example.com/other", "https://merchant.example.com?next=x"]) {
    const f = fixture({ origin });
    assert.equal((await f.run()).status, 503, origin);
    assert.equal(f.calls.length, 2);
  }
});

test("unsupported writes, routes and pagination never reach external APIs", async () => {
  for (const body of [{ operation: "push" }, { operation: "product", id: "../users" }, { operation: "pages", kind: "__proto__", page: 1 }, { operation: "pages", kind: "brand", page: 501 }]) {
    const f = fixture();
    assert.equal((await f.run(body)).status, 400);
    assert.equal(f.calls.length, 0);
  }
});

test("merchant errors are actionable and never echo upstream response bodies", async () => {
  for (const [remoteStatus, status, error] of [[401, 502, "ideasoft_credentials_rejected"], [403, 502, "ideasoft_credentials_rejected"], [404, 404, "ideasoft_not_found"], [429, 429, "ideasoft_rate_limited"], [503, 502, "ideasoft_unavailable"]] as const) {
    const f = fixture({ remoteStatus, remoteBody: { secret: "server-only-token" } });
    const response = await f.run();
    assert.equal(response.status, status);
    assert.deepEqual(await response.json(), { error });
  }
});

test("SKU search selects exact matches and projects only product fields", async () => {
  const f = fixture({ remoteBody: [{ id: 1, sku: "SKU-11" }, { id: 2, sku: " sku-1 ", privateField: "hidden", detail: { extraDetails: "table" } }] });
  const result = await (await f.run({ operation: "resolve", sku: "SKU-1" })).json();
  assert.equal(result.product.id, 2);
  assert.equal(result.product.extra_details, "table");
  assert.equal(result.product.privateField, undefined);
});

test("product response must match the requested ID", async () => {
  const f = fixture({ remoteBody: { id: 8 } });
  assert.equal((await f.run({ operation: "product", id: 7 })).status, 502);
});

test("blog title and nullable showcase content match the existing StorePage format", async () => {
  const f = fixture({ remoteBody: [{ id: 2, title: "Article", status: 0, slug: "article" }] });
  const result = await (await f.run({ operation: "pages", kind: "blog", page: 1 })).json();
  assert.equal(result.pages[0].name, "Article");
  assert.equal(result.pages[0].showcase_content, null);
  assert.equal(result.pages[0].status, 0);
  assert.equal(result.has_more, false);
  assert.equal(f.calls[3].url, "https://merchant.example.com/admin-api/blogs?limit=100&page=1");
});

const pageDraft = {
  name: "Pilot sayfası", slug: "pilot-sayfasi", pageTitle: "Örnek SEO", metaDescription: "Örnek açıklama",
  excerpt: "Kısa özet", content: "Birinci <satır>\nve devamı\n\nİkinci", publish: false,
};

test("shared creation uses the workspace token and the correct defaults for all three page kinds", async () => {
  for (const [kind, resource] of [["category", "categories"], ["brand", "brands"], ["blog", "blogs"]]) {
    const f = fixture({ pageRpc: (url) => Response.json(url.endsWith("create") ? true : 1), merchant: (url, init) => {
      assert.equal(new Headers(init?.headers).get("Authorization"), "Bearer server-only-token");
      assert.equal(new URL(url).pathname, `/admin-api/${resource}`);
      if (init?.method === "GET") return Response.json([]);
      const body = JSON.parse(String(init?.body));
      return Response.json({ data: { id: 11, ...body } }, { status: 201 });
    } });
    const response = await f.run({ operation: "page_create", kind, draft: pageDraft });
    assert.equal(response.status, 200);
    const result = await response.json();
    assert.equal(result.page.status, 0);
    assert.equal(result.page.name, pageDraft.name);
    assert.equal(result.facts_recorded, true);
    const [post] = f.calls.filter(({ url, init }) => url.startsWith(configuredOrigin) && init?.method === "POST");
    const body = JSON.parse(String(post.init?.body));
    if (kind === "category") assert.equal(body.hasChildren, 1);
    if (kind === "brand") assert.equal(body.displayShowcaseContent, 0);
    if (kind === "blog") {
      assert.equal(body.title, pageDraft.name);
      assert.equal(body.content, "<p>Birinci &lt;satır&gt;<br>ve devamı</p><p>İkinci</p>");
      assert.deepEqual(body.categories, []);
      assert.equal(body.displayShowcaseContent, undefined);
    }
    const record = f.calls.find(({ url }) => url.endsWith("/rpc/team_confirm_ideasoft_page"));
    assert.equal(JSON.parse(String(record?.init?.body)).track_seo, false);
  }
});

test("duplicate, claimed, and uncertain creations never send a second POST", async () => {
  for (const situation of ["duplicate", "claimed", "uncertain"] as const) {
    const f = fixture({
      pageRpc: () => Response.json(situation !== "claimed"),
      merchant: (_url, init) => init?.method === "POST" ? new Response(null, { status: 502 })
        : Response.json(situation === "duplicate" ? [{ id: 8, slug: pageDraft.slug }] : []),
    });
    const response = await f.run({ operation: "page_create", kind: "brand", draft: pageDraft });
    assert.equal(response.status, situation === "uncertain" ? 502 : 409);
    assert.equal(f.calls.filter(({ url, init }) => url.startsWith(configuredOrigin) && init?.method === "POST").length, situation === "uncertain" ? 1 : 0);
    if (situation === "uncertain") {
      assert.equal((await response.json()).error, "ideasoft_page_create_unconfirmed");
      assert.equal(f.calls.filter(({ url }) => url.endsWith("/rpc/team_claim_ideasoft_page_create")).length, 1);
    }
  }
});

const pagePublications = (calls: { url: string; init?: RequestInit }[]) =>
  calls.filter(({ url }) => url.endsWith("/rpc/team_confirm_ideasoft_page_publication"));

test("all three page kinds follow passive creation, shared SEO push, and publication in order", async () => {
  for (const [kind, resource] of [["category", "categories"], ["brand", "brands"], ["blog", "blogs"]] as const) {
    const path = `/admin-api/${resource}`;
    const id = 11;
    const state: {
      merchantPage?: Record<string, unknown>;
      sharedPage?: { kind: typeof kind; remote_id: number; revision: number; details: Record<string, unknown> };
    } = {};
    const f = fixture({
      pageState: () => Response.json(state.sharedPage ? [state.sharedPage] : []),
      pageRpc: (url, init) => {
        if (url.endsWith("/rpc/team_claim_ideasoft_page_create")) return Response.json(true);
        const { target_kind, target_id, confirmed_facts, track_seo } = JSON.parse(String(init?.body));
        assert.equal(target_kind, kind);
        assert.equal(target_id, id);
        assert.equal(confirmed_facts.status, state.merchantPage?.status);
        if (url.endsWith("_publication")) {
          assert.equal(confirmed_facts.status, 1);
          assert.equal(track_seo, false);
        }
        state.sharedPage = { kind, remote_id: id, revision: (state.sharedPage?.revision ?? 0) + 1,
          details: { ...state.sharedPage?.details, ...confirmed_facts } };
        return Response.json(state.sharedPage.revision);
      },
      merchant: (url, init) => {
        const pathname = new URL(url).pathname;
        if (pathname === path && init?.method === "GET") return Response.json([]);
        if (pathname === path && init?.method === "POST") {
          state.merchantPage = { id, ...JSON.parse(String(init.body)) };
          return Response.json({ data: state.merchantPage }, { status: 201 });
        }
        assert.equal(pathname, `${path}/${id}`);
        assert.ok(state.merchantPage);
        if (init?.method === "PUT") {
          assert.equal(new Headers(init.headers).get("If-Match"), '"page-etag"');
          Object.assign(state.merchantPage, JSON.parse(String(init.body)));
          return new Response(null, { status: 204 });
        }
        return Response.json(state.merchantPage, { headers: { ETag: '"page-etag"' } });
      },
    });

    const create = await f.run({ operation: "page_create", kind, draft: pageDraft });
    assert.equal(create.status, 200, kind);
    assert.equal((await create.json()).facts_recorded, true, kind);
    assert.ok(state.sharedPage);
    assert.equal(state.sharedPage.details.status, 0, kind);
    const originalBlogContent = state.merchantPage?.content;

    // The editor has saved the next draft revision to Supabase before sending it to the store.
    state.sharedPage.revision += 1;
    Object.assign(state.sharedPage.details, { draft_page_title: "Updated SEO title", draft_meta_description: "Updated SEO description" });
    const seo = await f.run({ operation: "page_push", kind, id, revision: state.sharedPage.revision });
    assert.equal(seo.status, 200, kind);
    assert.equal((await seo.json()).facts_recorded, true, kind);
    assert.equal(state.sharedPage.details.status, 0, kind);
    assert.equal(state.sharedPage.details.page_title, "Updated SEO title", kind);
    assert.equal(state.sharedPage.details.meta_description, "Updated SEO description", kind);
    assert.equal(pagePublications(f.calls).length, 0, kind);

    const publish = await f.run({ operation: "page_publish", kind, id, revision: state.sharedPage.revision });
    assert.equal(publish.status, 200, kind);
    assert.equal((await publish.json()).facts_recorded, true, kind);
    assert.equal(state.sharedPage.details.status, 1, kind);
    assert.equal(state.sharedPage.details.page_title, "Updated SEO title", kind);
    assert.equal(state.merchantPage?.content, originalBlogContent, kind);
    assert.equal(pagePublications(f.calls).length, 1, kind);

    const repeat = await f.run({ operation: "page_publish", kind, id, revision: state.sharedPage.revision });
    assert.equal(repeat.status, 200, kind);
    assert.equal((await repeat.json()).updated, false, kind);
    const merchantWrites = f.calls.filter(({ url, init }) => url.startsWith(configuredOrigin) && /^(POST|PUT)$/.test(init?.method ?? ""));
    assert.deepEqual(merchantWrites.map(({ init }) => init?.method), ["POST", "PUT", "PUT"], kind);
    assert.equal(pagePublications(f.calls).length, 1, kind);
  }
});

test("creating a live page starts tracking only after merchant readback confirms the status", async () => {
  for (const status of [1, 0, undefined]) {
    const f = fixture({ pageRpc: (url) => Response.json(url.endsWith("create") ? true : 1), merchant: (url, init) => {
      const page = { id: 11, title: pageDraft.name, slug: pageDraft.slug };
      if (init?.method === "POST") return Response.json(page, { status: 201 });
      if (url.endsWith("/11")) return Response.json({ ...page, status });
      return Response.json([]);
    } });
    const result = await f.run({ operation: "page_create", kind: "blog", draft: { ...pageDraft, publish: true } });
    assert.equal(result.status, status === undefined ? 502 : 200);
    assert.equal(pagePublications(f.calls).length, status === 1 ? 1 : 0);
    if (status !== undefined) assert.equal((await result.json()).page.status, status);
  }
});

function catalogWriteFixture(kind: "category" | "brand" | "blog", options: { revision?: number; role?: string; recordFails?: boolean; breaksReadback?: boolean } = {}) {
  const details = { name: "Page", slug: "page", draft_page_title: "Shared title", draft_meta_description: "ğ".repeat(170), draft_target_keyword: "Keyword", draft_showcase: "Text <source>" };
  const state = { id: 12, name: "Page", title: "Page", slug: "page", status: 0, pageTitle: "Old", content: "Untouched" };
  let recordFails = options.recordFails ?? false;
  const f = fixture({ role: options.role, pageState: () => Response.json([{ kind, remote_id: 12, revision: options.revision ?? 2, details }]),
    pageRpc: () => Response.json(recordFails ? null : 2, { status: recordFails ? 503 : 200 }),
    merchant: (_url, init) => {
      if (init?.method === "PUT") {
        if (!options.breaksReadback) Object.assign(state, JSON.parse(String(init.body)));
        return new Response(null, { status: 204 });
      }
      return Response.json(state, { headers: { ETag: '"page-etag"' } });
    },
  });
  return { ...f, state, recordAvailable: () => { recordFails = false; }, writes: () => f.calls.filter(({ init }) => init?.method === "PUT") };
}

test("shared pages use a checked draft revision and preserve untouched fields", async () => {
  for (const [kind, operation] of [["category", "page_publish"], ["blog", "page_push"]] as const) {
    const f = catalogWriteFixture(kind);
    const request = { operation, kind, id: 12, revision: 2 };
    const result = await (await f.run(request)).json();
    assert.equal(result.updated, true);
    assert.equal(result.facts_recorded, true);
    const [write] = f.writes();
    assert.equal(new Headers(write.init?.headers).get("If-Match"), '"page-etag"');
    assert.equal(f.state.content, "Untouched");
    const payload = JSON.parse(String(write.init?.body));
    if (operation === "page_publish") {
      assert.deepEqual(payload, { status: 1 });
      assert.equal(result.pushed_at, undefined);
      assert.equal(JSON.parse(String(pagePublications(f.calls)[0]?.init?.body)).track_seo, false);
    }
    else assert.deepEqual(payload, { pageTitle: "Shared title", metaDescription: "ğ".repeat(160), targetKeyword: "Keyword" });
    const replay = await (await f.run(request)).json();
    assert.equal(replay.updated, false);
    assert.equal(f.writes().length, 1);
    assert.equal(pagePublications(f.calls).length, operation === "page_publish" ? 1 : 0);
  }
});

test("a stale shared page or a viewer cannot write to the merchant", async () => {
  const request = { operation: "page_push", kind: "brand", id: 12, revision: 2 };
  const stale = catalogWriteFixture("brand", { revision: 3 });
  const viewer = catalogWriteFixture("brand", { role: "viewer" });
  assert.equal((await stale.run(request)).status, 409);
  assert.equal((await viewer.run(request)).status, 403);
  for (const f of [stale, viewer]) assert.equal(f.calls.some(({ url }) => url.startsWith(configuredOrigin)), false);
  const ambiguous = catalogWriteFixture("brand", { breaksReadback: true });
  const response = await ambiguous.run(request);
  assert.equal((await response.json()).error, "ideasoft_page_write_unconfirmed");
  assert.equal(ambiguous.calls.some(({ url }) => url.endsWith("/rpc/team_confirm_ideasoft_page")), false);
});

test("signed SEO and publication receipts repair the shared record without calling the merchant again", async () => {
  for (const operation of ["page_push", "page_publish"]) {
    const f = catalogWriteFixture("brand", { recordFails: true });
    const result = await (await f.run({ operation, kind: "brand", id: 12, revision: 2 })).json();
    assert.equal(result.updated, true);
    assert.equal(result.facts_recorded, false);
    assert.equal(typeof result.pushed_at, operation === "page_push" ? "string" : "undefined");
    assert.equal(typeof result.receipt, "string");
    const request = { operation: "page_receipt", receipt: result.receipt };
    const upstreamCalls = () => f.calls.filter(({ url }) => url.startsWith(configuredOrigin)).length;
    const before = upstreamCalls();
    assert.equal((await f.run(request, "another-member")).status, 409);
    f.recordAvailable();
    assert.equal((await (await f.run(request)).json()).facts_recorded, true);
    assert.equal(upstreamCalls(), before);
    if (operation === "page_publish") {
      const confirmations = pagePublications(f.calls);
      assert.equal(confirmations.length, 2);
      assert.equal(confirmations[0].init?.body, confirmations[1].init?.body);
    }
  }
});

function writeFixture(putStatus = 204, tokenRpc?: MerchantFetch) {
  const state = { id: 7, sku: "SKU-1", pageTitle: "Old Title", detail: { details: "Old description", extraDetails: "Old table" } };
  const rpc = { recordFails: false, lookupFails: false };
  const records = new Map<string, { id: number; pushed_at: string }>();
  const f = fixture({ tokenRpc, rpc: (_url, init) => {
    const body = JSON.parse(String(init?.body));
    const prior = records.get(body.target_event_key);
    if (body.confirmed_at) {
      if (rpc.recordFails) return Response.json({}, { status: 503 });
      const row = prior ?? { id: body.confirmed_product_id, pushed_at: body.confirmed_at };
      records.set(body.target_event_key, row);
      return Response.json(row);
    }
    return rpc.lookupFails ? Response.json({}, { status: 503 }) : Response.json(prior ?? null);
  }, merchant: (url, init) => {
    if (url.includes("?")) return Response.json([{ id: 100, sku: "SKU-11" }, { id: 7, sku: " sku-1 " }]);
    if (init?.method === "PUT") return new Response(null, { status: putStatus });
    return Response.json(state, { headers: { ETag: '"current"' } });
  } });
  return { ...f, state, rpc, records, writes: () => f.calls.filter(({ init }) => init?.method === "PUT") };
}

test("product preview only reads and sends a scoped confirmation; push preserves the latest unselected detail", async () => {
  const f = writeFixture();
  const input = { sku: "SKU-1", changes: { targetKeyword: " Keyword ", detail: { extraDetails: "<table>Updated</table>" } } };
  const preview = await (await f.run({ ...input, operation: "product_preview" })).json();
  assert.equal(preview.configured, true);
  assert.equal(preview.id, 7);
  assert.equal(preview.remote.extra_details, "Old table");
  assert.equal(preview.local.targetKeyword, "Keyword");
  assert.match(preview.confirmation, /^\d{13}\.[a-f\d]{64}$/);
  assert.equal(f.writes().length, 0);
  f.state.detail.details = "Newest untouched side";
  const result = await f.run({ ...input, operation: "product_push", confirmation: preview.confirmation });
  const saved = await result.json();
  assert.deepEqual(saved, { configured: true, id: 7, updated: true, event_recorded: true, pushed_at: saved.pushed_at });
  assert.match(saved.pushed_at, /^\d{4}-\d{2}-\d{2}T/);
  const [write] = f.writes();
  assert.equal(write.url, "https://merchant.example.com/admin-api/products/7");
  assert.equal(write.init?.redirect, "error");
  assert.equal(new Headers(write.init?.headers).get("If-Match"), '"current"');
  assert.equal(new Headers(write.init?.headers).get("Authorization"), "Bearer server-only-token");
  assert.equal(new Headers(write.init?.headers).has("apikey"), false);
  assert.deepEqual(JSON.parse(String(write.init?.body)), { targetKeyword: "Keyword", detail: { details: "Newest untouched side", extraDetails: "<table>Updated</table>" } });
});

test("a changed merchant field, selected payload, caller, or deadline invalidates the preview", async () => {
  const input = { sku: "SKU-1", changes: { pageTitle: "New Title" } };
  for (const reason of ["field", "payload", "caller", "deadline"]) {
    const f = writeFixture();
    const preview = await (await f.run({ ...input, operation: "product_preview" })).json();
    if (reason === "field") f.state.pageTitle = "Changes after preview";
    const confirmation = reason === "deadline" ? `1000000000000.${preview.confirmation.split(".")[1]}` : preview.confirmation;
    const changes = reason === "payload" ? { pageTitle: "Unapproved Title" } : input.changes;
    const response = await f.run({ ...input, changes, operation: "product_push", confirmation }, reason === "caller" ? "another-editor" : true);
    assert.equal(response.status, 409, reason);
    assert.deepEqual(await response.json(), { error: "ideasoft_preview_stale" });
    assert.equal(f.writes().length, 0);
  }
});

test("unchanged SEO data does not produce a merchant write", async () => {
  const f = writeFixture();
  const input = { sku: "SKU-1", changes: { pageTitle: "Old Title" } };
  const preview = await (await f.run({ ...input, operation: "product_preview" })).json();
  const result = await f.run({ ...input, operation: "product_push", confirmation: preview.confirmation });
  assert.deepEqual(await result.json(), { configured: true, id: 7, updated: false });
  assert.equal(f.writes().length, 0);
  assert.equal(f.records.size, 0);
});

test("product writes reject unknown fields, empty SEO, missing confirmation and oversized HTML", async () => {
  const inputs = [
    { changes: { price: "10" } }, { changes: { detail: { seoEnabled: "true" } } },
    { changes: { pageTitle: " " } }, { changes: {} },
    { changes: { detail: { details: "a".repeat(250001) } } },
    { changes: { detail: { details: "ğ".repeat(250000), extraDetails: "ş".repeat(250000) } } },
    { changes: { pageTitle: "Title" }, operation: "product_push" },
  ];
  for (const input of inputs) {
    const f = writeFixture();
    assert.equal((await f.run({ operation: "product_preview", sku: "SKU-1", ...input })).status, 400);
    assert.equal(f.calls.length, 0);
  }
});

test("a long technical table below the documented limits can be previewed and pushed", async () => {
  const f = writeFixture();
  const details = "<table>" + "<tr><td>Özellik</td><td>Açıklama</td></tr>".repeat(1500) + "</table>";
  const input = { sku: "SKU-1", changes: { detail: { details } } };
  const preview = await (await f.run({ ...input, operation: "product_preview" })).json();
  assert.equal(preview.local.detail.details, details);
  assert.equal((await f.run({ ...input, operation: "product_push", confirmation: preview.confirmation })).status, 200);
  assert.equal(f.writes().length, 1);
});

test("viewers cannot access a product write preview or submit a product write", async () => {
  const f = fixture({ role: "viewer" });
  const input = { sku: "SKU-1", changes: { pageTitle: "Title" } };
  assert.equal((await f.run({ ...input, operation: "product_preview" })).status, 403);
  const confirmation = `1999999999999.${"0".repeat(64)}`;
  assert.equal((await f.run({ ...input, operation: "product_push", confirmation })).status, 403);
  assert.equal(f.calls.length, 4);
});

test("an ambiguous match or a mismatched SKU in the merchant detail cannot be previewed", async () => {
  for (const matches of [[{ sku: "SKU-1", id: 7 }, { sku: "sku-1", id: 8 }], [{ sku: "SKU-1", id: 7 }]]) {
    const f = fixture({ merchant: (url) => Response.json(url.includes("?") ? matches : { id: 7, sku: "SKU-11" }) });
    const response = await f.run({ operation: "product_preview", sku: "SKU-1", changes: { pageTitle: "Title" } });
    assert.equal(response.status, 502);
  }
});

test("a conditional write conflict or an unconfirmed upstream update tells the client to recheck", async () => {
  const input = { sku: "SKU-1", changes: { pageTitle: "New Title" } };
  for (const [from, to, error] of [[412, 409, "ideasoft_preview_stale"], [503, 502, "ideasoft_write_unconfirmed"]] as const) {
    const f = writeFixture(from);
    const preview = await (await f.run({ ...input, operation: "product_preview" })).json();
    const response = await f.run({ ...input, operation: "product_push", confirmation: preview.confirmation });
    assert.equal(response.status, to);
    assert.deepEqual(await response.json(), { error });
    assert.equal(f.writes().length, 1);
    assert.equal(f.records.size, 0);
  }
});

test("preview and push cannot write or measure a SKU missing from the active workspace catalog", async () => {
  const f = fixture({ inCatalog: false });
  const input = { sku: "SKU-1", changes: { pageTitle: "Title" } };
  const preview = await f.run({ ...input, operation: "product_preview" });
  const push = await f.run({ ...input, operation: "product_push", confirmation: `1999999999999.${"0".repeat(64)}` });
  for (const response of [preview, push]) {
    assert.equal(response.status, 409);
    assert.deepEqual(await response.json(), { error: "ideasoft_catalog_product_missing" });
  }
  assert.equal(f.calls.some(({ url }) => url.includes("merchant.example.com")), false);
});

test("an existing event prevents a repeated publication and preserves its timestamp", async () => {
  const f = writeFixture();
  const input = { sku: "SKU-1", changes: { pageTitle: "New title" } };
  const preview = await (await f.run({ ...input, operation: "product_preview" })).json();
  const push = { ...input, operation: "product_push", confirmation: preview.confirmation };
  const original = await (await f.run(push)).json();
  const retried = await (await f.run(push)).json();
  assert.deepEqual(retried, original);
  assert.equal(f.writes().length, 1);
  assert.equal(f.records.size, 1);
  const requests = f.calls.filter(({ url }) => url.endsWith("/rpc/team_record_ideasoft_push"));
  const body = JSON.parse(String(requests[1].init?.body));
  assert.equal(body.target_workspace, workspace);
  assert.equal(body.target_sku, "SKU-1");
  assert.equal(body.confirmed_product_id, 7);
  assert.match(body.target_event_key, /^ideasoft:[a-f\d]{64}$/);
  assert.equal(new Headers(requests[1].init?.headers).get("apikey"), "sb_secret_test-only");
  assert.equal(new Headers(requests[1].init?.headers).has("Authorization"), false);
  assert.equal(requests[1].init?.redirect, "error");
});

test("lost event write returns a signed retry receipt and repeating it never writes to IdeaSoft again", async () => {
  const f = writeFixture();
  const input = { sku: "SKU-1", changes: { pageTitle: "New title" } };
  const preview = await (await f.run({ ...input, operation: "product_preview" })).json();
  f.rpc.recordFails = true;
  const response = await f.run({ ...input, operation: "product_push", confirmation: preview.confirmation });
  assert.equal(response.status, 200);
  const pending = await response.json();
  assert.equal(pending.updated, true);
  assert.equal(pending.event_recorded, false);
  assert.match(pending.receipt, /^[a-zA-Z0-9_-]+\.[a-f\d]{64}$/);
  const recovery = { operation: "product_push_receipt", receipt: pending.receipt };
  f.rpc.recordFails = false;
  assert.equal((await f.run(recovery, "another-editor")).status, 409);
  const tampered = `${pending.receipt.slice(0, -1)}${pending.receipt.endsWith("a") ? "b" : "a"}`;
  assert.equal((await f.run({ ...recovery, receipt: tampered })).status, 409);
  assert.equal(f.records.size, 0);
  const recorded = await (await f.run(recovery)).json();
  assert.deepEqual(await (await f.run(recovery)).json(), recorded);
  assert.deepEqual(recorded, { configured: true, id: 7, updated: true, event_recorded: true, pushed_at: pending.pushed_at });
  assert.equal(f.writes().length, 1);
  assert.equal(f.records.size, 1);
});

test("an unavailable receipt lookup blocks potentially duplicating a merchant write", async () => {
  const f = writeFixture();
  const input = { sku: "SKU-1", changes: { pageTitle: "New title" } };
  const preview = await (await f.run({ ...input, operation: "product_preview" })).json();
  f.rpc.lookupFails = true;
  const response = await f.run({ ...input, operation: "product_push", confirmation: preview.confirmation });
  assert.equal(response.status, 503);
  assert.deepEqual(await response.json(), { error: "ideasoft_receipt_unavailable" });
  assert.equal(f.writes().length, 0);
});

const rotate = { operation: "rotate_token", token: "replacement-token", origin: configuredOrigin, replace_valid: false };
const external = (calls: ReturnType<typeof fixture>["calls"]) => calls.filter(({ url }) => url.startsWith(`${configuredOrigin}/`));
const setters = (calls: ReturnType<typeof fixture>["calls"]) => calls.filter(({ url }) => url.endsWith("/rpc/team_set_ideasoft_token"));

test("the Vault token takes precedence over the legacy token and never leaves the service response", async () => {
  const f = fixture({ tokenRpc: () => Response.json({ token: "vault-only-token", version: 4 }) });
  const response = await f.run();
  assert.deepEqual(await response.json(), { configured: true, connected: true });
  const [get] = f.calls.filter(({ url }) => url.endsWith("/rpc/team_get_ideasoft_token"));
  assert.deepEqual(JSON.parse(String(get.init?.body)), { target_workspace: workspace, target_origin: configuredOrigin });
  assert.equal(new Headers(get.init?.headers).get("apikey"), "sb_secret_test-only");
  assert.equal(new Headers(get.init?.headers).has("Authorization"), false);
  assert.equal(new Headers(external(f.calls)[0].init?.headers).get("Authorization"), "Bearer vault-only-token");
});

test("rotation rejects non-owners, unsafe candidate input and mismatched shop origin before any merchant access", async () => {
  const editor = fixture();
  assert.deepEqual(await (await editor.run(rotate)).json(), { error: "ideasoft_rotation_owner_required" });
  assert.equal(editor.calls.length, 2);
  const owner = fixture({ role: "owner" });
  const mismatch = await owner.run({ ...rotate, origin: "https://another-shop.example.com" });
  assert.equal(mismatch.status, 409);
  assert.deepEqual(await mismatch.json(), { error: "ideasoft_rotation_origin_mismatch" });
  assert.equal(owner.calls.length, 2);
  for (const invalid of [{ token: "token\r\nAuthorization" }, { token: " " }, { token: "a".repeat(3001) }, { owner: user }, { origin: "http://merchant.example.com" }, { replace_valid: "true" }]) {
    const f = fixture({ role: "owner" });
    assert.equal((await f.run({ ...rotate, ...invalid })).status, 400);
    assert.equal(f.calls.length, 0);
  }
});

test("only a read-verified recovery token is saved with the verified caller and protected version", async () => {
  const f = fixture({ role: "owner", origin: `${configuredOrigin}/admin-api/`, tokenRpc: (url) =>
    url.endsWith("team_get_ideasoft_token") ? Response.json({ token: "vault-expired-token", version: 3 }) : Response.json({ version: 4, token: "must-not-leak" }),
  merchant: (_url, init) => new Headers(init?.headers).get("Authorization") === "Bearer vault-expired-token"
    ? Response.json({ error: "ignored" }, { status: 401 }) : Response.json([]),
  });
  const response = await f.run({ ...rotate, origin: "merchant.example.com/admin-api/" });
  assert.equal(response.status, 200);
  assert.deepEqual(await response.json(), { configured: true, connected: true, rotated: true, version: 4 });
  const requests = external(f.calls);
  assert.equal(requests.length, 2);
  assert.deepEqual(requests.map(({ init }) => new Headers(init?.headers).get("Authorization")), ["Bearer vault-expired-token", "Bearer replacement-token"]);
  assert.ok(requests.every(({ url, init }) => url === `${configuredOrigin}/admin-api/products?limit=1` && init?.method === "GET" && init.redirect === "error"));
  const [save] = setters(f.calls);
  assert.deepEqual(JSON.parse(String(save.init?.body)), {
    target_workspace: workspace, target_owner: user, target_origin: configuredOrigin, new_token: "replacement-token", expected_version: 3,
  });
  assert.equal(new Headers(save.init?.headers).get("apikey"), "sb_secret_test-only");
  assert.equal(new Headers(save.init?.headers).has("Authorization"), false);
});

test("a healthy active token is retained by ordinary tests even when the local candidate is different", async () => {
  for (const token of ["replacement-token", "server-only-token"]) {
    const f = fixture({ role: "owner" });
    assert.deepEqual(await (await f.run({ ...rotate, token })).json(), { configured: true, connected: true, rotated: false });
    assert.equal(external(f.calls).length, 1);
    assert.equal(new Headers(external(f.calls)[0].init?.headers).get("Authorization"), "Bearer server-only-token");
    assert.equal(setters(f.calls).length, 0);
  }
});

test("explicit replacement validates the candidate directly and inserts when there is no Vault token", async () => {
  const f = fixture({ role: "owner", tokenRpc: (url) => url.endsWith("team_get_ideasoft_token") ? Response.json(null) : Response.json({ version: 1 }) });
  assert.deepEqual(await (await f.run({ ...rotate, replace_valid: true })).json(), { configured: true, connected: true, rotated: true, version: 1 });
  assert.equal(external(f.calls).length, 1);
  assert.equal(new Headers(external(f.calls)[0].init?.headers).get("Authorization"), "Bearer replacement-token");
  assert.equal(JSON.parse(String(setters(f.calls)[0].init?.body)).expected_version, 0);
});

test("never replaces on transient current-token failures, or writes an unverified candidate", async () => {
  for (const remoteStatus of [429, 500, 503]) {
    const f = fixture({ role: "owner", remoteStatus });
    assert.equal((await f.run(rotate)).ok, false);
    assert.equal(external(f.calls).length, 1);
    assert.equal(setters(f.calls).length, 0);
  }
  const f = fixture({ role: "owner", remoteStatus: 403, remoteBody: { private: "replacement-token" } });
  const response = await f.run({ ...rotate, replace_valid: true });
  assert.deepEqual(await response.json(), { error: "ideasoft_credentials_rejected" });
  assert.equal(setters(f.calls).length, 0);
});

test("no active or fallback token permits owner recovery and conflicts never expose credentials", async () => {
  const f = fixture({ role: "owner", fallbackToken: null, tokenRpc: (url) => url.endsWith("team_get_ideasoft_token")
    ? Response.json(null) : Response.json({ token: "replacement-token" }, { status: 409 }),
  });
  assert.deepEqual(await (await f.run()).json(), { configured: false });
  const response = await f.run(rotate);
  assert.equal(response.status, 409);
  assert.deepEqual(await response.json(), { error: "ideasoft_rotation_conflict" });
  assert.equal(external(f.calls).length, 1);
  assert.equal(setters(f.calls).length, 1);
});

test("an unavailable token store fails closed even when a legacy credential exists", async () => {
  for (const status of [404, 503]) {
    const f = fixture({ role: "owner", tokenRpc: () => Response.json({ token: "secret-from-error" }, { status }) });
    for (const response of [await f.run(), await f.run(rotate)]) {
      assert.equal(response.status, 503);
      assert.deepEqual(await response.json(), { error: "ideasoft_token_store_unavailable" });
    }
    assert.equal(external(f.calls).length, 0);
    assert.equal(setters(f.calls).length, 0);
  }
});

test("an already recorded push can replay without a working Vault token lookup", async () => {
  let stored: unknown = { token: "initial-vault-token", version: 1 };
  const f = writeFixture(204, () => stored ? Response.json(stored) : Response.json({}, { status: 503 }));
  const input = { sku: "SKU-1", changes: { pageTitle: "New title" } };
  const preview = await (await f.run({ ...input, operation: "product_preview" })).json();
  const push = { ...input, operation: "product_push", confirmation: preview.confirmation };
  const original = await (await f.run(push)).json();
  assert.equal(original.event_recorded, true);
  stored = null;
  assert.deepEqual(await (await f.run(push)).json(), original);
  assert.equal(f.writes().length, 1);
});
