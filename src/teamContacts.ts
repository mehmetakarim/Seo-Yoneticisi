import { invoke } from "@tauri-apps/api/core";
import { shallowRef } from "vue";
import { teamClient, teamProject, teamReachable, teamSession, teamUser, teamWorkspace } from "./teamConnection";
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
type ContactRow = Omit<Contact, "tags" | "event_count"> & {
  team_contact_tags: { tag: string }[];
  team_contact_events: { count: number }[];
};

function context() {
  const client = teamClient.value;
  const workspace = teamWorkspace.value;
  const user = teamUser.value;
  if (!user || !workspace || !teamProject.value) throw new Error("Sistem → Ekip bölümünden bir çalışma alanı seçin.");
  return { client: teamSession.value ? client : null, workspace, user, scope: JSON.stringify([teamProject.value, user, workspace]) };
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
type Operation = {
  id: string; contact: Contact; conflict: string | null;
  payload: { target_workspace: string; operation_id: string; target_contact: string; expected_revision: number | null; fields: object };
};
export const contactSync = shallowRef({ pending: 0, conflicts: 0, offline: false });
let flushing: Promise<void> | undefined;
let cacheRevision = 0;
const cache = <T>(scope: string, request: object) => invoke<T>("team_cache", { scope, request });
const pending = (scope: string) => cache<Operation[]>(scope, { action: "pending" });
const online = (ctx: ContactContext) => !!ctx.client && (teamReachable.value ?? navigator.onLine);
const unavailable = (status: number) => status === 0 || status >= 500;
const currentScope = (ctx: ContactContext) => teamUser.value === ctx.user && teamWorkspace.value === ctx.workspace
  && JSON.stringify([teamProject.value, teamUser.value, teamWorkspace.value]) === ctx.scope;

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
  if (flushing) return flushing;
  const ctx = context();
  flushing = (async () => {
    let offline = !online(ctx);
    const rows = await report(ctx, offline);
    if (offline || !ctx.client) return;
    const locked = new Set<string>();
    for (const op of rows) {
      if (!currentScope(ctx)) break;
      const id = remoteId(op.contact.id);
      if (op.conflict) locked.add(id);
      if (locked.has(id)) continue;
      const response = await ctx.client.rpc("team_sync_contact", op.payload);
      if (response.error) {
        if (response.error.code !== "40001" && (unavailable(response.status) || response.error.code === "PGRST202")) {
          offline = unavailable(response.status);
          break;
        }
        await cache(ctx.scope, { action: "conflict", id: op.id, reason: fail(response.error).message });
        locked.add(id);
        continue;
      }
      const result = await ctx.client.from("team_contacts").select(columns)
        .eq("workspace_id", ctx.workspace).eq("id", id).single();
      if (result.error) {
        offline = unavailable(result.status);
        break;
      }
      cacheRevision++;
      await cache(ctx.scope, {
        action: "acknowledge", id: op.id, contact: unpackContact(result.data as unknown as ContactRow),
      });
    }
    await report(ctx, offline);
  })().finally(() => { flushing = undefined; });
  return flushing;
}

export async function teamContactProgress(id: ContactId) {
  const { scope } = context();
  const rows = (await pending(scope)).filter((op) => op.contact.id === id);
  return rows.some((op) => op.conflict) ? "conflict" : rows.length ? "pending" : "synced";
}

export async function teamContactConflict(id: ContactId) {
  const ctx = context();
  const rows = (await pending(ctx.scope)).filter((op) => op.contact.id === id);
  const first = rows.find((op) => op.conflict);
  const latest = rows[rows.length - 1];
  if (!first || !latest) return null;
  if (!online(ctx) || !ctx.client) return { local: latest.contact, remote: null, reason: first.conflict };
  const result = await ctx.client.from("team_contacts").select(columns)
    .eq("workspace_id", ctx.workspace).eq("id", remoteId(id)).maybeSingle();
  return {
    local: latest.contact,
    remote: result.error || !result.data ? null : unpackContact(result.data as unknown as ContactRow),
    reason: first.conflict,
  };
}

async function contactDetail<T extends unknown[]>(key: string, read: (ctx: ContactContext) => Promise<{ data: T | null; status: number; error: { message: string } | null }>): Promise<T> {
  const ctx = context();
  if (online(ctx)) {
    const response = await read(ctx);
    if (!response.error) {
      await cache(ctx.scope, { action: "write", key, data: response.data });
      return response.data ?? [] as unknown as T;
    }
    if (!unavailable(response.status)) throw fail(response.error);
  }
  await report(ctx, true);
  return (await cache<T | null>(ctx.scope, { action: "read", key })) ?? [] as unknown as T;
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
    if (!(await cache<boolean>(ctx.scope, { action: "read", key: "can-edit" }))) {
      throw new Error("Düzenleme yetkinizi doğrulamak için Kişiler ekranını bağlantı varken açın.");
    }
    const prior = draft.id === null ? null : await teamContacts.get(draft.id);
    const id = draft.id === null ? crypto.randomUUID() : remoteId(draft.id);
    if ((await pending(ctx.scope)).some((op) => op.contact.id === id && op.conflict)) {
      throw new Error("Bu kişi için bekleyen eşitleme çakışması var.");
    }
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
      } satisfies Operation["payload"],
    });
    await flushTeamContacts();
    return id;
  },
  async archive(id: ContactId, archived: boolean, revision?: number) {
    const { client, workspace } = context();
    if (!client || !navigator.onLine) throw new Error("Arşiv işlemi için bağlantı gerekli.");
    if (await teamContactProgress(id) !== "synced") throw new Error("Arşivlemeden önce bekleyen kişi düzenlemesinin eşitlenmesi gerekiyor.");
    if (revision == null) throw new Error("Kişiyi listeden tekrar açın.");
    const { data, error } = await client.from("team_contacts").update({ archived, revision: revision + 1 })
      .eq("workspace_id", workspace).eq("id", remoteId(id)).eq("revision", revision).select("id").maybeSingle();
    if (error) throw fail(error);
    if (!data) throw new Error("Kişi kaydı değişmiş veya düzenleme yetkiniz yok. Güncel kaydı tekrar açın.");
  },
  async events(id: ContactId): Promise<ContactEvent[]> {
    return contactDetail(`events:${remoteId(id)}`, async ({ client, workspace }) =>
      await client!.from("team_contact_events").select("id,at,kind,note")
        .eq("workspace_id", workspace).eq("contact_id", remoteId(id)).order("at", { ascending: false }));
  },
  async tags() {
    const { scope } = context();
    const rows = await cache<Contact[]>(scope, { action: "contacts" });
    const changes = await pending(scope);
    return [...new Set([...rows, ...changes.map((entry) => entry.contact)].flatMap((row) => row.tags))]
      .sort((a, b) => a.localeCompare(b, "tr"));
  },
  async products(id: ContactId): Promise<ContactProduct[]> {
    const rows = await contactDetail<Pick<ContactProduct, "sku" | "contact_id" | "at">[]>(
      `products:${remoteId(id)}`, async ({ client, workspace }) =>
        await client!.from("team_contact_products").select("sku,contact_id,at")
          .eq("workspace_id", workspace).eq("contact_id", remoteId(id)).order("at", { ascending: false }),
    );
    return rows.map((row) => ({ ...row, name: row.sku }));
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
    const { client, workspace } = context();
    if (!client || !navigator.onLine) throw new Error("Temas, etiket ve ürün bağlantısı işlemleri için bağlantı gerekli.");
    if (await teamContactProgress(id) !== "synced") throw new Error("Bu işlem için bekleyen kişi düzenlemesinin eşitlenmesi gerekiyor.");
    if (revision == null) throw new Error("Kişiyi listeden tekrar açın.");
    const { error } = await client.rpc("team_contact_action", {
      target_workspace: workspace, target_contact: remoteId(id), expected_revision: revision, action, payload,
    });
    if (error) throw fail(error);
  },
};
