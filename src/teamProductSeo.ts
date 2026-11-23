import { invoke } from "@tauri-apps/api/core";
import { shallowRef } from "vue";
import { teamAccessIssue, teamClient, teamProject, teamReachable, teamSession, teamUser, teamWorkspace } from "./teamConnection";

type Draft = Record<string, unknown>;
export type TeamSeoRecord = { id: string; sku: string; revision: number; draft: Draft };
type Operation = { id: string; contact: TeamSeoRecord; payload: object; conflict: string | null };
type Context = ReturnType<typeof context>;

const scopeKey = () => JSON.stringify([teamProject.value, teamUser.value, teamWorkspace.value, "product-seo"]);
export const teamProductSeoScope = scopeKey;
function context() {
  if (!teamUser.value || !teamWorkspace.value || teamAccessIssue.value) {
    throw new Error("Ortak SEO kayıtlarına erişilemiyor.");
  }
  return {
    scope: scopeKey(), workspace: teamWorkspace.value, user: teamUser.value,
    client: teamSession.value && (teamReachable.value ?? navigator.onLine) ? teamClient.value : null,
  };
}

const cache = <T>(scope: string, request: object) => invoke<T>("team_cache", { scope, request });
const pending = (ctx: Context) => cache<Operation[]>(ctx.scope, { action: "pending" });
const transportFailure = (status: number) => status === 0 || status >= 500;
export const teamSeoSync = shallowRef({ pending: 0, conflicts: 0 });
let flushing: Promise<void> | undefined;
let generation = 0;
let writes: Promise<unknown> = Promise.resolve();

async function overlay(ctx: Context, rows: TeamSeoRecord[]) {
  const ops = await pending(ctx);
  if (ctx.scope === scopeKey()) {
    const blocked = new Set(ops.filter((op) => op.conflict).map((op) => op.contact.sku));
    teamSeoSync.value = {
      pending: ops.filter((op) => !blocked.has(op.contact.sku)).length, conflicts: blocked.size,
    };
  }
  const result = new Map(rows.filter(Boolean).map((row) => [row.sku, row]));
  for (const op of ops) result.set(op.contact.sku, op.contact);
  return result;
}

export async function listTeamProductSeo() {
  const ctx = context();
  const cached = () => cache<TeamSeoRecord[]>(ctx.scope, { action: "contacts" });
  if (!ctx.client) return overlay(ctx, await cached());
  const started = generation;
  const rows: TeamSeoRecord[] = [];
  for (let offset = 0; ; offset += 200) {
    const page = await ctx.client.from("team_product_seo").select("sku,draft,revision")
      .eq("workspace_id", ctx.workspace).order("sku").range(offset, offset + 199).retry(false);
    if (page.error) {
      if (transportFailure(page.status)) return overlay(ctx, await cached());
      throw new Error("Ortak SEO kayıtları okunamadı.");
    }
    rows.push(...page.data.map((row) => ({ ...row, id: row.sku })));
    if (page.data.length < 200) break;
  }
  if (ctx.scope !== scopeKey()) throw new Error("Çalışma alanı değişti.");
  if (started !== generation) return overlay(ctx, await cached());
  await cache(ctx.scope, { action: "snapshot", contacts: rows });
  const role = await ctx.client.from("workspace_members").select("role").eq("workspace_id", ctx.workspace)
    .eq("user_id", ctx.user).maybeSingle().retry(false);
  if (!role.error) {
    await cache(ctx.scope, { action: "write", key: "can-edit", data: role.data?.role === "owner" || role.data?.role === "editor" });
  }
  return overlay(ctx, rows);
}

export async function getTeamProductSeo(sku: string) {
  const ctx = context();
  const queued = (await pending(ctx)).filter((op) => op.contact.sku === sku).pop();
  if (queued) return queued.contact;
  if (ctx.client) {
    const started = generation;
    const result = await ctx.client.from("team_product_seo").select("sku,draft,revision")
      .eq("workspace_id", ctx.workspace).eq("sku", sku).maybeSingle().retry(false);
    if (ctx.scope !== scopeKey()) throw new Error("Çalışma alanı değişti.");
    if (!result.error && started === generation) {
      const row = result.data ? { ...result.data, id: sku } : null;
      await cache(ctx.scope, { action: "write", key: `contact:${sku}`, data: row });
      return row;
    }
    if (result.error && !transportFailure(result.status)) throw new Error("Ortak SEO taslağı okunamadı.");
  }
  return cache<TeamSeoRecord | null>(ctx.scope, { action: "read", key: `contact:${sku}` });
}

async function readVerifiedProductSeo(sku: string, revision: number | null, includeDraft: boolean) {
  const ctx = context();
  if (!ctx.client) throw new Error("Bu işlem için Supabase bağlantısı gerekiyor.");
  await flushTeamProductSeo();
  const operations = await pending(ctx);
  if (operations.some((op) => op.contact.sku === sku)) {
    throw new Error("Bu ürünün ortak taslağını eşitleyip varsa çakışmayı çözün.");
  }
  if (ctx.scope !== scopeKey()) throw new Error("Çalışma alanı değişti.");
  const started = generation;
  const query = ctx.client.from("team_product_seo");
  const result = await (includeDraft ? query.select("sku,draft,revision") : query.select("revision"))
    .eq("workspace_id", ctx.workspace).eq("sku", sku).maybeSingle().retry(false);
  if (ctx.scope !== scopeKey() || teamAccessIssue.value) throw new Error("Çalışma alanı değişti.");
  if (result.error) throw new Error("Ortak taslak sürümü doğrulanamadı.");
  if ((result.data?.revision ?? null) !== revision) throw new Error("Ortak taslak değişti. Ürünü yeniden açın.");
  if (!includeDraft) return null;
  const row = result.data && "draft" in result.data ? { ...result.data, id: sku } : null;
  if (started === generation) await cache(ctx.scope, { action: "write", key: `contact:${sku}`, data: row });
  if (ctx.scope !== scopeKey()) throw new Error("Çalışma alanı değişti.");
  return row;
}

export function verifyTeamProductSeoRevision(sku: string, revision: number | null) {
  return readVerifiedProductSeo(sku, revision, false);
}

export function readTeamProductSeoForGeneration(sku: string, revision: number | null) {
  return readVerifiedProductSeo(sku, revision, true);
}

export function patchTeamProductSeo(sku: string, revision: number | null, changes: Draft, strict = false): Promise<TeamSeoRecord> {
  const ctx = context();
  const task = writes.catch(() => {}).then(async () => {
    if (ctx.scope !== scopeKey()) throw new Error("Çalışma alanı değişti.");
    if (!(await cache<boolean>(ctx.scope, { action: "read", key: "can-edit" }))) {
      throw new Error("SEO düzenleme yetkinizi doğrulamak için Ürünler ekranını bağlantı varken açın.");
    }
    const ops = (await pending(ctx)).filter((op) => op.contact.sku === sku);
    if (ops.some((op) => op.conflict)) throw new Error("Bu ürün için çözülmesi gereken bir eşitleme çakışması var.");
    const latest = ops[ops.length - 1]?.contact
      ?? await cache<TeamSeoRecord | null>(ctx.scope, { action: "read", key: `contact:${sku}` });
    if ((latest?.revision ?? null) !== revision && (strict || !ops.length)) {
      throw new Error("Ortak taslak değişti. Ürünü yeniden açın.");
    }
    const expected = latest?.revision ?? null;
    const operation = crypto.randomUUID();
    const record = { id: sku, sku, revision: (expected ?? 0) + 1, draft: { ...latest?.draft, ...changes } };
    await cache(ctx.scope, { action: "queue", id: operation, contact: record, payload: {
      target_workspace: ctx.workspace, operation_id: operation, target_sku: sku, expected_revision: expected, changes,
    } });
    generation++;
    await overlay(ctx, []);
    return record;
  });
  writes = task;
  return task;
}

export function flushTeamProductSeo(): Promise<void> {
  if (flushing) return flushing;
  if (!teamUser.value || !teamWorkspace.value || teamAccessIssue.value) return Promise.resolve();
  const ctx = context();
  const client = ctx.client;
  if (!client) return Promise.resolve();
  flushing = (async () => {
    const blocked = new Set<string>();
    for (const op of await pending(ctx)) {
      if (ctx.scope !== scopeKey()) return;
      if (op.conflict) blocked.add(op.contact.sku);
      if (blocked.has(op.contact.sku)) continue;
      const response = await client.rpc("team_sync_product_seo", op.payload).retry(false);
      if (!response.error && response.data === op.contact.revision) {
        generation++;
        await cache(ctx.scope, { action: "acknowledge", id: op.id, contact: op.contact });
      } else if (transportFailure(response.status) || [401, 404].includes(response.status)) break;
      else {
        await cache(ctx.scope, { action: "conflict", id: op.id, reason: response.error?.message ?? "Sürüm doğrulanamadı." });
        blocked.add(op.contact.sku);
      }
    }
    await overlay(ctx, []);
  })().finally(() => { flushing = undefined; });
  return flushing;
}
