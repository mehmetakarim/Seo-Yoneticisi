import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";
import { createClient } from "@supabase/supabase-js";
import { createServer } from "vite";

const project = "https://contact-events.example.test";
const workspace = "10000000-0000-4000-8000-000000000001";
const user = "20000000-0000-4000-8000-000000000001";
const contactId = "30000000-0000-4000-8000-000000000001";
const scope = JSON.stringify([project, user, workspace]);
const eventKey = `events:${contactId}`;
const productKey = `products:${contactId}`;
const historyKey = `contact-conflict:${contactId}`;
let vite, connection, teamContacts, flushTeamContacts, inspectTeamContactConflict,
  acceptTeamContactConflict, teamContactConflictHistory;

before(async () => {
  vite = await createServer({
    root: fileURLToPath(new URL("..", import.meta.url)), configFile: false, appType: "custom",
    server: { middlewareMode: true, hmr: false, watch: null }, logLevel: "silent",
  });
  connection = await vite.ssrLoadModule("/src/teamConnection.ts");
  ({ teamContacts, flushTeamContacts, inspectTeamContactConflict, acceptTeamContactConflict,
    teamContactConflictHistory } = await vite.ssrLoadModule("/src/teamContacts.ts"));
});
after(async () => { await vite?.close(); });

function fixture(t) {
  const contact = { id: contactId, name: "Örnek kişi", company: "", email: "", phone: "", channel: "",
    note: "", tags: [], event_count: 0, last_contact_at: null, next_step_at: null, next_step_note: "",
    archived: false, revision: 1 };
  const values = new Map([[`contact:${contactId}`, contact], ["can-edit", true]]);
  const operations = [];
  const attempted = [];
  const events = new Map();
  const products = new Map();
  const receipts = new Set();
  const resolutionCalls = [];
  const remote = { contact: { ...contact }, rpcMissing: false, loseResponse: false, conflict: false, readStatus: 200 };
  function mergeEvents(additions) {
    const merged = new Map((values.get(eventKey) ?? []).map((event) => [event.id, event]));
    for (const event of additions) merged.set(event.id, event);
    const current = [...merged.values()].sort((a, b) => Date.parse(b.at) - Date.parse(a.at));
    values.set(eventKey, current);
    return current;
  }
  const client = createClient(project, "fixture-key", {
    auth: { persistSession: false, autoRefreshToken: false, detectSessionInUrl: false },
    global: {
      async fetch(input, init) {
        const url = new URL(input instanceof Request ? input.url : input);
        assert.equal(url.origin, project);
        if (url.pathname === "/rest/v1/rpc/team_sync_contact_event" || url.pathname === "/rest/v1/rpc/team_sync_contact_action") {
          const body = JSON.parse(init.body);
          assert.equal(body.target_workspace, workspace);
          assert.equal(body.target_contact, contactId);
          attempted.push(body);
          if (remote.rpcMissing) return Response.json({ code: "PGRST202", message: "missing" }, { status: 404 });
          if (remote.conflict) return Response.json({ code: "PT409", message: "stale" }, { status: 409 });
          if (!receipts.has(body.operation_id)) {
            assert.equal(body.expected_revision, remote.contact.revision);
            receipts.add(body.operation_id);
            remote.contact.revision++;
            switch (body.action) {
              case "tags": remote.contact.tags = body.fields.tags; break;
              case "archive": remote.contact.archived = body.fields.archived; break;
              case "link": products.set(body.fields.sku, { sku: body.fields.sku, contact_id: contactId, at: body.occurred_at }); break;
              case "unlink": products.delete(body.fields.sku); break;
              default:
                events.set(body.operation_id, { id: body.operation_id, contact_id: contactId, at: body.occurred_at,
                  kind: body.fields.kind, note: body.fields.note });
                remote.contact.last_contact_at = body.occurred_at;
                remote.contact.next_step_at = body.fields.next_step_at;
                remote.contact.next_step_note = body.fields.next_step_note;
            }
          }
          if (remote.loseResponse) {
            remote.loseResponse = false;
            return Response.json({ code: "gateway", message: "temporary" }, { status: 503 });
          }
          return Response.json(body.operation_id);
        }
        assert.equal(url.searchParams.get("workspace_id"), `eq.${workspace}`);
        if (url.pathname === "/rest/v1/team_contacts") {
          assert.equal(url.searchParams.get("id"), `eq.${contactId}`);
          await remote.beforeRead?.();
          if (remote.readStatus !== 200) {
            return Response.json({ code: "42501", message: "private service detail" }, { status: remote.readStatus });
          }
          const fields = url.searchParams.get("select");
          const links = { team_contact_products: [...products.values()] };
          if (fields.startsWith("revision,")) return Response.json([{ revision: remote.contact.revision, ...links }]);
          const row = { ...remote.contact, team_contact_tags: remote.contact.tags.map((tag) => ({ tag })),
            team_contact_events: [{ count: events.size }], ...(fields.includes("team_contact_products(") && links) };
          return Response.json(init.headers.get("Accept") === "application/vnd.pgrst.object+json" ? row : [row]);
        }
        assert.equal(url.pathname, "/rest/v1/team_contact_events");
        assert.equal(url.searchParams.get("contact_id"), `eq.${contactId}`);
        const eventId = url.searchParams.get("id");
        if (!eventId) return Response.json([...events.values()]);
        const matching = events.get(eventId.slice(3));
        return Response.json(init.headers.get("Accept") === "application/vnd.pgrst.object+json" ? matching : [matching]);
      },
    },
  });
  const oldWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
  Object.defineProperty(globalThis, "window", { configurable: true, value: { __TAURI_INTERNALS__: {
    invoke: async (command, args) => {
      assert.equal(command, "team_cache");
      assert.equal(args.scope, scope);
      const request = structuredClone(args.request);
      switch (request.action) {
        case "read": return structuredClone(values.get(request.key) ?? null);
        case "write": values.set(request.key, request.data); return null;
        case "pending": return structuredClone(operations);
        case "contacts": return [structuredClone(values.get(`contact:${contactId}`))];
        case "queue": operations.push({ ...request, conflict: null }); return null;
        case "conflict": operations.find(({ id }) => id === request.id).conflict = request.reason; return null;
        case "merge_contact_events": return structuredClone(mergeEvents(request.events));
        case "cache_contact_products": {
          const snapshot = { revision: request.revision, products: request.products };
          values.set(productKey, snapshot);
          return structuredClone(snapshot);
        }
        case "acknowledge_contact_products":
          values.set(productKey, { revision: request.contact.revision, products: request.products });
          // The fake persistence confirms the parent and removes the operation as native does.
        case "acknowledge": {
          const index = operations.findIndex(({ id }) => id === request.id);
          assert.notEqual(index, -1);
          values.set(`contact:${request.contact.id}`, request.contact);
          operations.splice(index, 1);
          return true;
        }
        case "acknowledge_contact_event": {
          assert.equal(request.event.id, request.id);
          const index = operations.findIndex(({ id }) => id === request.id);
          assert.notEqual(index, -1);
          values.set(`contact:${request.contact.id}`, request.contact);
          mergeEvents([request.event]);
          operations.splice(index, 1);
          return true;
        }
        case "resolve_contact_conflict": {
          resolutionCalls.push(request);
          const reviewed = operations.filter(({ contact }) => contact.id === request.contact_id);
          if (!reviewed.some(({ conflict }) => conflict) ||
            JSON.stringify(reviewed.map(({ id }) => id)) !== JSON.stringify(request.expected_ids)) return false;
          values.set(historyKey, [{ saved_at: "2026-11-25T08:00:00Z", local: reviewed.at(-1).contact,
            remote: request.remote, products: request.products, operations: reviewed }, ...(values.get(historyKey) ?? [])]);
          values.set(`contact:${contactId}`, request.remote);
          values.set(productKey, { revision: request.remote.revision, products: request.products });
          for (const op of reviewed) operations.splice(operations.indexOf(op), 1);
          return true;
        }
        default: assert.fail(`Unexpected cache action ${request.action}`);
      }
    },
  } } });
  connection.teamProject.value = project;
  connection.teamWorkspace.value = workspace;
  connection.teamSession.value = { user: { id: user } };
  connection.teamClient.value = client;
  connection.teamReachable.value = false;
  t.after(() => {
    connection.teamSession.value = null;
    connection.teamClient.value = null;
    connection.teamWorkspace.value = "";
    connection.teamProject.value = "";
    connection.teamReachable.value = null;
    if (oldWindow) Object.defineProperty(globalThis, "window", oldWindow);
    else Reflect.deleteProperty(globalThis, "window");
  });
  return { values, operations, remote, events, attempted, products, resolutionCalls };
}

const fields = { kind: "call", note: "Fiyat görüşüldü", next_step_at: "2026-12-01", next_step_note: "Teklifi takip et" };

test("queues a contact event offline and retries the same timestamp and id after an uncertain result", async (t) => {
  const { operations, remote, events, attempted } = fixture(t);
  assert.equal(await teamContacts.addEvent(contactId, 1, fields), "pending");
  assert.equal(attempted.length, 0);
  assert.equal(operations.length, 1);
  const [local] = await teamContacts.events(contactId);
  assert.equal(local.id, operations[0].id);
  assert.equal(local.note, fields.note);
  assert.equal(local.at, operations[0].payload.occurred_at);
  assert.equal((await teamContacts.get(contactId)).next_step_at, fields.next_step_at);

  remote.loseResponse = true;
  connection.teamReachable.value = true;
  await flushTeamContacts();
  assert.equal(operations.length, 1);
  assert.equal(events.size, 1);
  assert.equal((await teamContacts.events(contactId)).length, 1, "server event and local projection share an ID");
  await flushTeamContacts();
  assert.equal(operations.length, 0);
  assert.equal(events.size, 1);
  assert.deepEqual(attempted, [attempted[0], attempted[0]]);
  assert.deepEqual(attempted[0].fields, fields);

  connection.teamReachable.value = false;
  const [cached] = await teamContacts.events(contactId);
  assert.equal(cached.id, local.id);
  assert.equal(cached.at, local.at);
  assert.equal((await teamContacts.get(contactId)).revision, 2);
});

test("a missing event RPC keeps queued work, and an actual revision conflict preserves the local note", async (t) => {
  const { operations, remote, events, attempted, values } = fixture(t);
  values.set("can-edit", false);
  await assert.rejects(teamContacts.addEvent(contactId, 1, fields), /yetki/i);
  assert.equal(operations.length, 0);
  values.set("can-edit", true);
  assert.equal(await teamContacts.addEvent(contactId, 1, fields), "pending");
  await assert.rejects(teamContacts.addEvent(contactId, 1, fields), /yenile|tekrar|sürüm/i);
  assert.equal(operations.length, 1);

  connection.teamReachable.value = true;
  remote.rpcMissing = true;
  await flushTeamContacts();
  assert.equal(operations[0].conflict, null);
  remote.rpcMissing = false;
  remote.conflict = true;
  await flushTeamContacts();
  assert.ok(operations[0].conflict);
  assert.equal(events.size, 0);
  connection.teamReachable.value = false;
  assert.equal((await teamContacts.events(contactId))[0].note, fields.note);
  await assert.rejects(teamContacts.addEvent(contactId, 2, fields), /çakışma/i);
  assert.equal(attempted.length, 2);
});

test("serializes same-contact edits and sends distinct offline notes in revision order", async (t) => {
  const { operations, attempted, events } = fixture(t);
  const submitted = await Promise.allSettled([
    teamContacts.addEvent(contactId, 1, fields),
    teamContacts.addEvent(contactId, 1, { ...fields, note: "Eski sürümle ikinci gönderim" }),
  ]);
  assert.equal(submitted[0].status, "fulfilled");
  assert.equal(submitted[1].status, "rejected");
  assert.equal(operations.length, 1);

  const { revision } = await teamContacts.get(contactId);
  assert.equal(await teamContacts.addEvent(contactId, revision, { ...fields, note: "İkinci görüşme" }), "pending");
  assert.equal((await teamContacts.events(contactId)).length, 2);

  connection.teamReachable.value = true;
  await flushTeamContacts();
  assert.deepEqual(attempted.map((op) => op.expected_revision), [1, 2]);
  assert.equal(events.size, 2);
  assert.equal(operations.length, 0);
});

test("queues labels, product links, and archive state, then acknowledges them in revision order", async (t) => {
  const { operations, remote, products, values, attempted } = fixture(t);
  const legacyLink = { sku: "ESKI-001", contact_id: contactId, at: "2026-11-01T09:00:00Z" };
  values.set(productKey, [legacyLink]);
  products.set(legacyLink.sku, legacyLink);
  assert.equal(await teamContacts.act(contactId, 1, "tags", { tags: [" Acil ", "Acil", ""] }), "pending");
  assert.deepEqual((await teamContacts.get(contactId)).tags, ["Acil"]);
  assert.equal(await teamContacts.act(contactId, 2, "link", { sku: " YENI-002 " }), "pending");
  assert.equal(await teamContacts.act(contactId, 3, "unlink", { sku: legacyLink.sku }), "pending");
  assert.equal(await teamContacts.archive(contactId, true, 4), "pending");
  assert.equal((await teamContacts.list("", false, "", "")).length, 0);
  const [linked] = await teamContacts.products(contactId);
  assert.equal(linked.sku, "YENI-002");
  assert.equal(linked.at, operations[1].payload.occurred_at);
  assert.deepEqual(attempted, []);

  remote.loseResponse = true;
  connection.teamReachable.value = true;
  await flushTeamContacts();
  assert.equal(operations.length, 4);
  assert.equal(remote.contact.revision, 2);
  await flushTeamContacts();
  assert.equal(operations.length, 0);
  assert.deepEqual(attempted.map((op) => op.action), ["tags", "tags", "link", "unlink", "archive"]);
  assert.deepEqual(attempted[0], attempted[1]);
  assert.deepEqual([...products.keys()], ["YENI-002"]);
  assert.equal((await teamContacts.products(contactId)).length, 1);

  connection.teamReachable.value = false;
  assert.equal(await teamContacts.archive(contactId, false, 5), "pending");
  assert.equal((await teamContacts.list("", false, "", "Acil")).length, 1);
  assert.equal((await teamContacts.products(contactId))[0].at, linked.at);
  connection.teamReachable.value = true;
  await flushTeamContacts();
  assert.equal(operations.length, 0);
  assert.equal(remote.contact.archived, false);
  assert.equal(remote.contact.revision, 6);
});

test("preserves the optimistic product removal when a newer Supabase revision conflicts", async (t) => {
  const { operations, remote, products, values } = fixture(t);
  const existing = { sku: "ESKI-001", contact_id: contactId, at: "2026-11-01T09:00:00Z" };
  values.set(productKey, [existing]);
  products.set(existing.sku, existing);
  assert.equal(await teamContacts.act(contactId, 1, "unlink", { sku: existing.sku }), "pending");
  remote.conflict = true;
  connection.teamReachable.value = true;
  await flushTeamContacts();
  assert.ok(operations[0].conflict);
  assert.ok(products.has(existing.sku));
  assert.deepEqual(await teamContacts.products(contactId), []);
  await assert.rejects(teamContacts.archive(contactId, true, 2), /çakışma/i);
});

async function queueContactConflict(t) {
  const scenario = fixture(t);
  assert.equal(await teamContacts.addEvent(contactId, 1, fields), "pending");
  assert.equal(await teamContacts.act(contactId, 2, "link", { sku: "YEREL-001" }), "pending");
  scenario.operations[0].conflict = "Kişi kaydı değişti.";
  scenario.remote.contact = { ...scenario.remote.contact, revision: 4, name: "Ortak kişi" };
  scenario.products.set("ORTAK-002", { sku: "ORTAK-002", contact_id: contactId, at: "2026-11-21T11:30:00Z" });
  return scenario;
}

test("reviews offline changes and archives precisely the compared operations when choosing the shared copy", async (t) => {
  const { operations, attempted, resolutionCalls, values } = await queueContactConflict(t);
  assert.deepEqual(await teamContactConflictHistory(contactId), []);
  const offline = await inspectTeamContactConflict(contactId);
  assert.equal(offline.remoteState, "offline");
  assert.equal(offline.localProducts[0].sku, "YEREL-001");
  assert.equal(offline.operations[0].payload.fields.note, fields.note);
  assert.equal(operations.length, 2);

  connection.teamReachable.value = true;
  const review = await inspectTeamContactConflict(contactId);
  assert.equal(review.remoteState, "ready");
  assert.equal(review.remote.name, "Ortak kişi");
  assert.equal(review.products[0].sku, "ORTAK-002");
  const operationIds = operations.map(({ id }) => id);
  assert.deepEqual(review.operationIds, operationIds);
  assert.equal((await acceptTeamContactConflict(review)).revision, 4);
  assert.deepEqual(resolutionCalls[0].expected_ids, operationIds);
  assert.equal(operations.length, 0);
  assert.deepEqual(attempted, [], "selecting the shared copy does not write to Supabase");

  connection.teamReachable.value = false;
  assert.equal((await teamContacts.get(contactId)).name, "Ortak kişi");
  assert.equal((await teamContacts.products(contactId))[0].sku, "ORTAK-002");
  const [saved] = await teamContactConflictHistory(contactId);
  assert.equal(saved.operations[0].payload.fields.note, fields.note);
  assert.equal(saved.operations[1].payload.fields.sku, "YEREL-001");
  assert.equal(saved.local.revision, 3);
  assert.ok(values.has(historyKey));
});

test("keeps queued work when the shared revision or the local queue changes during review", async (t) => {
  const { remote, operations, resolutionCalls } = await queueContactConflict(t);
  connection.teamReachable.value = true;
  const stale = await inspectTeamContactConflict(contactId);
  remote.contact.revision++;
  await assert.rejects(acceptTeamContactConflict(stale), /değiş|yenile/i);
  assert.equal(resolutionCalls.length, 0);

  const beforeNoteChange = await inspectTeamContactConflict(contactId);
  remote.contact.note = "İnceleme sırasında güncellendi";
  await assert.rejects(acceptTeamContactConflict(beforeNoteChange), /değiş|yenile/i);
  assert.equal(resolutionCalls.length, 0);

  const fresh = await inspectTeamContactConflict(contactId);
  operations.push({ ...operations[1], id: "later" });
  await assert.rejects(acceptTeamContactConflict(fresh), /değiş|yenile/i);
  assert.equal(operations.length, 3);
  assert.deepEqual(await teamContactConflictHistory(contactId), []);
});

test("refuses resolution after losing access or changing the signed-in workspace", async (t) => {
  const { remote, operations, resolutionCalls } = await queueContactConflict(t);
  connection.teamReachable.value = true;
  const review = await inspectTeamContactConflict(contactId);
  remote.readStatus = 403;
  await assert.rejects(acceptTeamContactConflict(review), (error) => !String(error).includes("private service detail"));
  remote.readStatus = 200;
  remote.beforeRead = () => { connection.teamWorkspace.value = "20000000-0000-4000-8000-000000000009"; };
  await assert.rejects(acceptTeamContactConflict(review), /çalışma alanı|oturum|kapsam/i);
  assert.equal(operations.length, 2);
  assert.deepEqual(resolutionCalls, []);
});
