import { invoke } from "@tauri-apps/api/core";
import { shallowRef, watch } from "vue";
import { teamAccessIssue, teamClient, teamProject, teamReachable, teamSession, teamUser, teamWorkspace } from "./teamConnection";
import type { Contact, ContactEvent, ContactId, ContactProduct } from "./types";

export interface ContactDraft {
  id: ContactId | null;
  name: string;
  company: string;
  email: string;
  phone: string;
  channel: string;
  note: string;
  nextStepAt: string | null;
  nextStepNote: string;
  revision?: number;
}

const columns = "id,name,company,email,phone,channel,note,last_contact_at,next_step_at,next_step_note,archived,revision,team_contact_tags(tag),team_contact_events(count)";
const eventColumns = "id,contact_id,at,kind,note";
const productColumns = "team_contact_products(sku,contact_id,at)";
const eventKinds = ["call", "email", "meeting", "note", "followup_done"];
let sourceRevision = 0;
watch([teamProject, teamUser, teamWorkspace, teamAccessIssue], () => { sourceRevision++; }, { flush: "sync" });
export const teamContactSource = () => sourceRevision;
type ContactRow = Omit<Contact, "tags" | "event_count"> & {
  team_contact_tags: { tag: string }[];
  team_contact_events: { count: number }[];
};
type ContactProductRow = Pick<ContactProduct, "sku" | "contact_id" | "at">;
type ProductSnapshot = { revision: number; products: ContactProductRow[] };

function context() {
  const client = teamClient.value;
  const workspace = teamWorkspace.value;
  const user = teamUser.value;
  if (!user || !workspace || !teamProject.value) throw new Error("Sistem → Ekip bölümünden bir çalışma alanı seçin.");
  return { client: teamSession.value ? client : null, workspace, user, scope: JSON.stringify([teamProject.value, user, workspace]), sourceRevision };
}

function remoteId(id: ContactId): string {
  if (typeof id !== "string") throw new Error("Bu kayıt yerel veri kaynağına ait. Kişiyi listeden yeniden seçin.");
  return id;
}

function unpackContact(row: ContactRow): Contact {
  const { team_contact_tags, team_contact_events, ...contact } = row;
  return { ...contact, tags: team_contact_tags.map(({ tag }) => tag).sort(), event_count: team_contact_events[0]?.count ?? 0 };
}

function fail(error: { message: string; code?: string }) {
  return new Error(error.code === "40001" || error.code === "PT409"
    ? "Kişi kaydı değişmiş. Güncel kaydı tekrar açın." : error.message);
}

type ContactContext = ReturnType<typeof context>;
type ContactEventFields = { kind: string; note: string; next_step_at: string | null; next_step_note: string | null };
type SavePayload = { target_workspace: string; operation_id: string; target_contact: string; expected_revision: number | null; fields: object };
type ContactEventPayload = SavePayload & { expected_revision: number; occurred_at: string; fields: ContactEventFields };
type ContactProductAction = { action: "link" | "unlink"; fields: { sku: string } };
type ContactAction = ContactProductAction
  | { action: "tags"; fields: { tags: string[] } }
  | { action: "archive"; fields: { archived: boolean } };
type ContactActionPayload = SavePayload & ContactAction & { expected_revision: number; occurred_at: string };
type Operation = {
  id: string; contact: Contact; conflict: string | null;
  payload: SavePayload | ContactEventPayload | ContactActionPayload;
};
export type TeamContactConflictOperation = Operation;
type ContactConflictRemote = {
  remote: Contact | null;
  remoteState: "ready" | "offline" | "missing" | "unavailable";
  products: ContactProductRow[];
  remoteAccessBlocked?: boolean;
};
export type TeamContactConflict = ContactConflictRemote & {
  scope: string;
  operationIds: string[];
  local: Contact;
  localProducts: ContactProductRow[];
  operations: Operation[];
  reason: string;
};
export type TeamContactConflictArchive = {
  saved_at: string;
  local: Contact;
  remote: Contact;
  products: ContactProductRow[];
  operations: Operation[];
};
export const contactSync = shallowRef({ pending: 0, conflicts: 0, offline: false });
const flushes = new Map<string, Promise<void>>();
const enqueues = new Map<string, Promise<void>>();
let cacheRevision = 0;
const cache = <T>(scope: string, request: object) => invoke<T>("team_cache", { scope, request });
const pending = (scope: string) => cache<Operation[]>(scope, { action: "pending" });
const online = (ctx: ContactContext) => !!ctx.client && !teamAccessIssue.value && teamReachable.value !== false && navigator.onLine !== false;
const unavailable = (status: number) => status === 0 || status >= 500;
const currentScope = (ctx: ContactContext) => ctx.sourceRevision === sourceRevision && !teamAccessIssue.value;
const isAction = (payload: Operation["payload"]): payload is ContactActionPayload => "action" in payload;
const isEvent = (payload: Operation["payload"]): payload is ContactEventPayload => "occurred_at" in payload && !isAction(payload);
const isProductAction = (payload: Operation["payload"]): payload is ContactActionPayload & ContactProductAction =>
  isAction(payload) && (payload.action === "link" || payload.action === "unlink");

function assertCurrentScope(ctx: ContactContext) {
  if (!currentScope(ctx)) throw new Error("Ekip oturumu veya çalışma alanı değişti.");
}

async function requireEditor(ctx: ContactContext) {
  if (!(await cache<boolean>(ctx.scope, { action: "read", key: "can-edit" }))) {
    throw new Error("Düzenleme yetkinizi doğrulamak için Kişiler ekranını bağlantı varken açın.");
  }
  assertCurrentScope(ctx);
}

function enqueueContact(ctx: ContactContext, id: string, prepare: () => Promise<void>) {
  const key = JSON.stringify([ctx.scope, id]);
  const next = (enqueues.get(key) ?? Promise.resolve()).catch(() => {}).then(prepare);
  enqueues.set(key, next);
  return next.finally(() => {
    if (enqueues.get(key) === next) enqueues.delete(key);
  });
}

async function report(ctx: ContactContext, offline: boolean) {
  const rows = await pending(ctx.scope);
  if (currentScope(ctx)) contactSync.value = {
    pending: rows.filter((op) => !op.conflict).length,
    conflicts: rows.filter((op) => !!op.conflict).length,
    offline,
  };
  return rows;
}

export function flushTeamContacts(): Promise<void> {
  if (!teamUser.value || !teamWorkspace.value || !teamProject.value) return Promise.resolve();
  const ctx = context();
  const existing = flushes.get(ctx.scope);
  if (existing) return existing;
  const flushing = (async () => {
    let offline = !online(ctx);
    const rows = await report(ctx, offline);
    const client = ctx.client;
    if (offline || !client) return;
    const locked = new Set<string>();
    for (const op of rows) {
      if (!currentScope(ctx) || !online(ctx)) break;
      const id = remoteId(op.contact.id);
      if (op.conflict) locked.add(id);
      if (locked.has(id)) continue;
      const eventWrite = isEvent(op.payload);
      const actionWrite = isAction(op.payload);
      const productWrite = isProductAction(op.payload);
      const rpc = eventWrite ? "team_sync_contact_event" : actionWrite ? "team_sync_contact_action" : "team_sync_contact";
      const response = await client.rpc(rpc, op.payload).retry(false);
      if (!currentScope(ctx)) break;
      if (response.error) {
        if (response.error.code === "PGRST202" || response.status === 404) {
          locked.add(id);
          continue;
        }
        if (response.error.code !== "40001" && (unavailable(response.status) || response.status === 401)) {
          offline = unavailable(response.status);
          break;
        }
        await cache(ctx.scope, { action: "conflict", id: op.id, reason: fail(response.error).message });
        locked.add(id);
        continue;
      }
      if ((eventWrite || actionWrite) && response.data !== op.id) {
        locked.add(id);
        continue;
      }
      const result = await client.from("team_contacts").select(productWrite ? `${columns},${productColumns}` : columns)
        .eq("workspace_id", ctx.workspace).eq("id", id).single();
      if (!currentScope(ctx)) break;
      if (result.error) {
        offline = unavailable(result.status);
        if (offline || result.status === 401) break;
        locked.add(id);
        continue;
      }
      const { team_contact_products: products, ...row } = result.data as unknown as ContactRow & { team_contact_products?: ContactProductRow[] };
      const contact = unpackContact(row);
      if (productWrite) {
        if (!Array.isArray(products)) {
          locked.add(id);
          continue;
        }
        cacheRevision++;
        if (!(await cache<boolean>(ctx.scope, { action: "acknowledge_contact_products", id: op.id, contact, products }))) {
          locked.add(id);
        }
        continue;
      }
      if (!eventWrite) {
        cacheRevision++;
        await cache(ctx.scope, { action: "acknowledge", id: op.id, contact });
        continue;
      }
      const eventResult = await client.from("team_contact_events").select(eventColumns)
        .eq("workspace_id", ctx.workspace).eq("contact_id", id).eq("id", op.id).single();
      if (!currentScope(ctx)) break;
      if (eventResult.error) {
        offline = unavailable(eventResult.status);
        if (offline || eventResult.status === 401) break;
        locked.add(id);
        continue;
      }
      cacheRevision++;
      if (!(await cache<boolean>(ctx.scope, { action: "acknowledge_contact_event", id: op.id, contact, event: eventResult.data }))) {
        locked.add(id);
      }
    }
    await report(ctx, offline || !online(ctx));
  })().finally(() => { flushes.delete(ctx.scope); });
  flushes.set(ctx.scope, flushing);
  return flushing;
}

async function flushQueuedContact(ctx: ContactContext, operationId: string) {
  // A running flush may have read its batch before this operation was enqueued.
  const inProgress = flushes.get(ctx.scope);
  if (inProgress) await inProgress.catch(() => {});
  if (!currentScope(ctx)) return "pending";
  try {
    await flushTeamContacts();
    const remaining = (await pending(ctx.scope)).find((op) => op.id === operationId);
    return remaining?.conflict ? "conflict" : remaining ? "pending" : "synced";
  } catch {
    // The operation was durably queued. A temporary read or sync failure can be retried.
    return "pending";
  }
}

function pendingEvent(op: Operation): ContactEvent | null {
  if (!isEvent(op.payload)) return null;
  return { id: op.id, at: op.payload.occurred_at, kind: op.payload.fields.kind, note: op.payload.fields.note };
}

async function loadContactTimeline(ctx: ContactContext, contactId: string): Promise<ContactEvent[]> {
  const operations = await pending(ctx.scope);
  let rows: ContactEvent[] | null = null;
  if (online(ctx) && ctx.client) {
    const response = await ctx.client.from("team_contact_events").select(eventColumns)
      .eq("workspace_id", ctx.workspace).eq("contact_id", contactId).order("at", { ascending: false });
    assertCurrentScope(ctx);
    if (!response.error) {
      rows = await cache<ContactEvent[]>(ctx.scope, { action: "merge_contact_events", contact_id: contactId, events: response.data ?? [] });
    } else if (!unavailable(response.status)) {
      throw fail(response.error);
    }
  }
  if (rows === null) {
    await report(ctx, true);
    rows = await cache<ContactEvent[] | null>(ctx.scope, { action: "read", key: `events:${contactId}` });
  }
  assertCurrentScope(ctx);
  const timeline = new Map((rows ?? []).map((event) => [event.id, event]));
  for (const op of operations) {
    const event = op.contact.id === contactId ? pendingEvent(op) : null;
    if (event && !timeline.has(event.id)) timeline.set(event.id, event);
  }
  return [...timeline.values()].sort((a, b) => Date.parse(b.at) - Date.parse(a.at));
}

export async function teamContactProgress(id: ContactId) {
  const { scope } = context();
  const rows = (await pending(scope)).filter((op) => op.contact.id === id);
  return rows.some((op) => op.conflict) ? "conflict" : rows.length ? "pending" : "synced";
}

function assertContactConflictScope(ctx: ContactContext, signal?: AbortSignal) {
  if (signal?.aborted) throw new DOMException("Kişi karşılaştırması iptal edildi.", "AbortError");
  assertCurrentScope(ctx);
  if (ctx.client && ctx.client !== teamClient.value) throw new Error("Ekip bağlantısı değişti. Karşılaştırmayı yenileyin.");
}

async function readContactConflictRemote(ctx: ContactContext, id: string, signal?: AbortSignal): Promise<ContactConflictRemote> {
  assertContactConflictScope(ctx, signal);
  if (!online(ctx) || !ctx.client) return { remote: null, remoteState: "offline", products: [] };
  const query = ctx.client.from("team_contacts").select(`${columns},${productColumns}`)
    .eq("workspace_id", ctx.workspace).eq("id", id);
  if (signal) query.abortSignal(signal);
  const result = await query.maybeSingle().retry(false);
  assertContactConflictScope(ctx, signal);
  if (!online(ctx)) return { remote: null, remoteState: "offline", products: [] };
  if (result.error) {
    const remoteAccessBlocked = result.status === 401 || result.status === 403 || result.error.code === "42501";
    return { remote: null, remoteState: "unavailable", products: [], remoteAccessBlocked };
  }
  if (!result.data) return { remote: null, remoteState: "missing", products: [] };
  const { team_contact_products: products, ...row } = result.data as unknown as ContactRow & { team_contact_products?: ContactProductRow[] };
  if (row.id !== id || !Number.isSafeInteger(row.revision) || (row.revision ?? 0) < 1
    || !Array.isArray(row.team_contact_tags) || !Array.isArray(row.team_contact_events)
    || !Array.isArray(products) || products.some((product) => !product || product.contact_id !== id
      || typeof product.sku !== "string" || !product.sku.trim() || !Number.isFinite(Date.parse(product.at)))) {
    return { remote: null, remoteState: "unavailable", products: [] };
  }
  return { remote: unpackContact(row), remoteState: "ready", products: sortContactProducts([...products]) };
}

export async function inspectTeamContactConflict(id: string, signal?: AbortSignal): Promise<TeamContactConflict> {
  const ctx = context();
  assertContactConflictScope(ctx, signal);
  const operations = (await pending(ctx.scope)).filter((op) => op.contact.id === id);
  const first = operations.find((op) => op.conflict);
  const latest = operations[operations.length - 1];
  assertContactConflictScope(ctx, signal);
  if (!first?.conflict || !latest) throw new Error("Bu kişi için bekleyen çakışma kalmadı.");
  const stored = await cache<ProductSnapshot | ContactProductRow[] | null>(ctx.scope, { action: "read", key: `products:${id}` });
  const localProducts = projectContactProducts(stored, operations, id);
  const remote = await readContactConflictRemote(ctx, id, signal);
  return {
    scope: ctx.scope, operationIds: operations.map((op) => op.id), local: latest.contact,
    localProducts, operations, reason: first.conflict, ...remote,
  };
}

export async function acceptTeamContactConflict(review: TeamContactConflict, signal?: AbortSignal): Promise<Contact> {
  const ctx = context();
  assertContactConflictScope(ctx, signal);
  const id = review.local.id;
  if (typeof id !== "string" || !review.remote || review.remote.id !== id || review.scope !== ctx.scope) {
    throw new Error("Kişi karşılaştırmasını yeniden açın.");
  }
  await flushes.get(ctx.scope);
  const current = await readContactConflictRemote(ctx, id, signal);
  if (current.remoteAccessBlocked) throw new Error("Ortak kişi kaydı için erişiminizi doğrulayın.");
  if (!current.remote || !online(ctx)) throw new Error("Güncel ortak kişi kaydı doğrulanamadı. Karşılaştırmayı yenileyin.");
  if (JSON.stringify(current.remote) !== JSON.stringify(review.remote)
    || JSON.stringify(current.products) !== JSON.stringify(review.products)) {
    throw new Error("Ortak kişi kaydı inceleme sırasında değişti. Karşılaştırmayı yenileyin.");
  }
  cacheRevision++;
  const accepted = await cache<boolean>(ctx.scope, {
    action: "resolve_contact_conflict", contact_id: id, expected_ids: review.operationIds,
    remote: current.remote, products: current.products,
  }).catch((cause: unknown) => {
    if (String(cause).includes("unknown variant `resolve_contact_conflict`")) {
      throw new Error("Ortak sürümü seçmek için güncel masaüstü sürümü gerekiyor.");
    }
    throw cause;
  });
  if (!accepted) throw new Error("Kişi kaydı veya bekleyen işlemler inceleme sırasında değişti. Karşılaştırmayı yenileyin.");
  await report(ctx, !online(ctx)).catch(() => {});
  assertContactConflictScope(ctx, signal);
  return current.remote;
}

export async function teamContactConflictHistory(id: string): Promise<TeamContactConflictArchive[]> {
  const ctx = context();
  assertCurrentScope(ctx);
  const history = await cache<TeamContactConflictArchive[] | null>(ctx.scope, { action: "read", key: `contact-conflict:${id}` });
  assertCurrentScope(ctx);
  return history ?? [];
}

async function queueContactAction(id: ContactId, revision: number | undefined, action: ContactAction) {
  if (revision == null) throw new Error("Kişiyi listeden tekrar açın.");
  const ctx = context();
  const contactId = remoteId(id);
  await requireEditor(ctx);
  const operationId = crypto.randomUUID();
  const occurredAt = new Date().toISOString();
  await enqueueContact(ctx, contactId, async () => {
    const prior = await editableContact(ctx, contactId, revision);
    const contact = { ...prior, revision: revision + 1 };
    if (action.action === "tags") contact.tags = action.fields.tags;
    if (action.action === "archive") contact.archived = action.fields.archived;
    await cache(ctx.scope, {
      action: "queue", id: operationId, contact,
      payload: {
        ...action, target_workspace: ctx.workspace, operation_id: operationId, target_contact: contactId,
        expected_revision: revision, occurred_at: occurredAt,
      } satisfies ContactActionPayload,
    });
  });
  return flushQueuedContact(ctx, operationId);
}

async function loadContactProducts(ctx: ContactContext, id: string): Promise<ContactProduct[]> {
  const operations = await pending(ctx.scope);
  assertCurrentScope(ctx);
  let stored: ProductSnapshot | ContactProductRow[] | null = null;
  if (online(ctx) && ctx.client) {
    const result = await ctx.client.from("team_contacts").select(`revision,${productColumns}`)
      .eq("workspace_id", ctx.workspace).eq("id", id).maybeSingle();
    assertCurrentScope(ctx);
    if (result.error) {
      if (!unavailable(result.status)) throw fail(result.error);
      await report(ctx, true);
    } else if (result.data) {
      stored = await cache<ProductSnapshot>(ctx.scope, {
        action: "cache_contact_products", contact_id: id,
        revision: result.data.revision, products: result.data.team_contact_products,
      });
    } else if (!operations.some((op) => op.contact.id === id && op.payload.expected_revision === null)) {
      return [];
    }
  }
  if (stored === null) {
    stored = await cache<ProductSnapshot | ContactProductRow[] | null>(ctx.scope, { action: "read", key: `products:${id}` });
  }
  assertCurrentScope(ctx);
  return projectContactProducts(stored, operations, id).map((row) => ({ ...row, name: row.sku }));
}

function sortContactProducts(rows: ContactProductRow[]) {
  return rows.sort((left, right) => Date.parse(right.at) - Date.parse(left.at) || left.sku.localeCompare(right.sku));
}

function projectContactProducts(stored: ProductSnapshot | ContactProductRow[] | null, operations: Operation[], id: string) {
  const products = new Map((Array.isArray(stored) ? stored : stored?.products ?? []).map((row) => [row.sku, row]));
  for (const { contact, payload } of operations) {
    if (contact.id !== id || !isProductAction(payload)) continue;
    const sku = payload.fields.sku;
    if (payload.action === "unlink") products.delete(sku);
    else if (!products.has(sku)) products.set(sku, { sku, contact_id: id, at: payload.occurred_at });
  }
  return sortContactProducts([...products.values()]);
}

export const teamContacts = {
  async list(search: string, includeArchived: boolean, channel: string, tag: string) {
    const ctx = context();
    const { client, workspace, scope } = ctx;
    let rows: Contact[] = [];
    let offline = !online(ctx);
    const startedAt = cacheRevision;
    const term = search.trim().toLocaleLowerCase("tr");
    const pageSize = 200;
    if (!offline && client) {
      for (let start = 0; ; start += pageSize) {
        const response = await client.from("team_contacts").select(columns).eq("workspace_id", workspace)
          .order("id").range(start, start + pageSize - 1);
        if (response.error) {
          if (!unavailable(response.status)) throw fail(response.error);
          offline = true;
          break;
        }
        rows.push(...(response.data as unknown as ContactRow[]).map(unpackContact));
        if (response.data.length < pageSize) break;
      }
      if (!offline && startedAt === cacheRevision && currentScope(ctx)) {
        await cache(scope, { action: "snapshot", contacts: rows });
      }
      if (!offline) {
        const permission = await client.from("workspace_members").select("role")
          .eq("workspace_id", workspace).eq("user_id", ctx.user).maybeSingle();
        if (!permission.error) {
          await cache(scope, { action: "write", key: "can-edit", data: ["owner", "editor"].includes(permission.data?.role ?? "") });
        }
      }
    }
    if (offline) rows = await cache<Contact[]>(scope, { action: "contacts" });
    const overlay = new Map(rows.map((row) => [row.id, row]));
    for (const op of await report(ctx, offline)) overlay.set(op.contact.id, op.contact);
    return [...overlay.values()].filter((row) =>
      (includeArchived || !row.archived) && (!channel || row.channel === channel)
      && (!tag || row.tags.includes(tag)) && (!term || [row.name, row.company, row.email, row.phone]
        .some((value) => value.toLocaleLowerCase("tr").includes(term))),
    ).sort((a, b) => (a.next_step_at ?? "\uffff").localeCompare(b.next_step_at ?? "\uffff") || a.name.localeCompare(b.name, "tr"));
  },
  async get(id: ContactId) {
    const ctx = context();
    const queued = (await pending(ctx.scope)).filter((op) => op.contact.id === id).pop();
    if (queued) return queued.contact;
    const key = `contact:${remoteId(id)}`;
    if (online(ctx) && ctx.client) {
      const result = await ctx.client.from("team_contacts").select(columns)
        .eq("workspace_id", ctx.workspace).eq("id", remoteId(id)).single();
      if (!result.error) {
        const contact = unpackContact(result.data as unknown as ContactRow);
        await cache(ctx.scope, { action: "write", key, data: contact });
        return contact;
      }
      if (!unavailable(result.status)) throw fail(result.error);
    }
    const contact = await cache<Contact | null>(ctx.scope, { action: "read", key });
    if (!contact) throw new Error("Bu kişi henüz çevrimdışı kullanım için alınmamış.");
    return contact;
  },
  async save(draft: ContactDraft): Promise<ContactId> {
    const ctx = context();
    const fields = {
      name: draft.name.trim(), company: draft.company.trim(), email: draft.email.trim(),
      phone: draft.phone.trim(), channel: draft.channel, note: draft.note,
      next_step_at: draft.nextStepAt || null, next_step_note: draft.nextStepNote,
    };
    const revision = draft.revision;
    if (draft.id !== null && revision == null) throw new Error("Düzenlemek için kişiyi listeden tekrar açın.");
    await requireEditor(ctx);
    const id = draft.id === null ? crypto.randomUUID() : remoteId(draft.id);
    await enqueueContact(ctx, id, async () => {
      assertCurrentScope(ctx);
      const prior = draft.id === null ? null : await editableContact(ctx, id, revision);
      const operationId = crypto.randomUUID();
      const contact: Contact = {
        ...fields, id, revision: (revision ?? 0) + 1, archived: prior?.archived ?? false,
        tags: prior?.tags ?? [], event_count: prior?.event_count ?? 0, last_contact_at: prior?.last_contact_at ?? null,
      };
      await cache(ctx.scope, {
        action: "queue", id: operationId, contact,
        payload: {
          target_workspace: ctx.workspace, operation_id: operationId, target_contact: id,
          expected_revision: revision ?? null, fields,
        } satisfies SavePayload,
      });
    });
    if (currentScope(ctx)) await flushTeamContacts();
    return id;
  },
  async addEvent(id: ContactId, revision: number | undefined, fields: ContactEventFields) {
    if (revision == null) throw new Error("Teması kaydetmek için kişiyi listeden tekrar açın.");
    if (!eventKinds.includes(fields.kind)) throw new Error("Geçerli bir temas türü seçin.");
    const ctx = context();
    const contactId = remoteId(id);
    await requireEditor(ctx);
    const operationId = crypto.randomUUID();
    const occurredAt = new Date().toISOString();
    await enqueueContact(ctx, contactId, async () => {
      const prior = await editableContact(ctx, contactId, revision);
      const lastContact = prior.last_contact_at;
      const contact = {
        ...prior,
        revision: revision + 1,
        last_contact_at: lastContact && Date.parse(lastContact) > Date.parse(occurredAt) ? lastContact : occurredAt,
        next_step_at: fields.next_step_at,
        next_step_note: fields.next_step_note ?? "",
        event_count: prior.event_count + 1,
      };
      await cache(ctx.scope, {
        action: "queue", id: operationId, contact,
        payload: {
          target_workspace: ctx.workspace, operation_id: operationId, target_contact: contactId,
          expected_revision: revision, occurred_at: occurredAt, fields,
        } satisfies ContactEventPayload,
      });
    });
    return flushQueuedContact(ctx, operationId);
  },
  async archive(id: ContactId, archived: boolean, revision?: number) {
    return queueContactAction(id, revision, { action: "archive", fields: { archived } });
  },
  async events(id: ContactId): Promise<ContactEvent[]> {
    return loadContactTimeline(context(), remoteId(id));
  },
  async tags() {
    const ctx = context();
    const rows = await cache<Contact[]>(ctx.scope, { action: "contacts" });
    const changes = await pending(ctx.scope);
    assertCurrentScope(ctx);
    const contacts = new Map(rows.map((row) => [row.id, row]));
    for (const { contact } of changes) contacts.set(contact.id, contact);
    return [...new Set([...contacts.values()].flatMap((row) => row.tags))]
      .sort((a, b) => a.localeCompare(b, "tr"));
  },
  async products(id: ContactId): Promise<ContactProduct[]> {
    return loadContactProducts(context(), remoteId(id));
  },
  async contactsOfProduct(sku: string): Promise<ContactProduct[]> {
    const { client, workspace } = context();
    if (!client || !navigator.onLine) return [];
    const { data, error } = await client.from("team_contact_products")
      .select("sku,contact_id,at,team_contacts!inner(name,company)")
      .eq("workspace_id", workspace).eq("sku", sku).eq("team_contacts.archived", false);
    if (error) throw fail(error);
    return (data as unknown as (Omit<ContactProduct, "name"> & { team_contacts: { name: string; company: string } })[])
      .map(({ team_contacts: person, ...row }) => ({ ...row, name: [person.name, person.company].filter(Boolean).join(" · ") }));
  },
  async act(id: ContactId, revision: number | undefined, action: string, payload: object) {
    if (action === "tags" && "tags" in payload && Array.isArray(payload.tags)
      && payload.tags.every((tag) => typeof tag === "string")) {
      const tags = [...new Set(payload.tags.map((tag: string) => tag.trim()).filter(Boolean))].sort();
      return queueContactAction(id, revision, { action, fields: { tags } });
    }
    if ((action === "link" || action === "unlink") && "sku" in payload && typeof payload.sku === "string" && payload.sku.trim()) {
      return queueContactAction(id, revision, { action, fields: { sku: payload.sku.trim() } });
    }
    throw new Error("Geçerli bir kişi işlemi seçin.");
  },
};

async function editableContact(ctx: ContactContext, id: string, revision: number | undefined) {
  const operations = await pending(ctx.scope);
  assertCurrentScope(ctx);
  if (operations.some((op) => op.contact.id === id && op.conflict)) {
    throw new Error("Bu kişi için bekleyen eşitleme çakışması var.");
  }
  const contact = await teamContacts.get(id);
  assertCurrentScope(ctx);
  if (contact.revision !== revision) throw new Error("Kişi kaydı değişmiş. Güncel kaydı tekrar açın.");
  return contact;
}
