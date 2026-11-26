import { invoke } from "@tauri-apps/api/core";
import { shallowRef, watch } from "vue";
import { teamAccessIssue, teamClient, teamProject, teamReachable, teamSession, teamUser, teamWorkspace } from "./teamConnection";

type Draft = Record<string, unknown>;
export type TeamSeoRecord = { id: string; sku: string; revision: number; draft: Draft };
type Operation = { id: string; contact: TeamSeoRecord; payload: object; conflict: string | null };
type Context = ReturnType<typeof context>;
type ConflictRemote = {
  remote: TeamSeoRecord | null;
  remoteState: "ready" | "offline" | "missing" | "unavailable";
  remoteAccessBlocked?: boolean;
};
export type TeamProductSeoConflict = ConflictRemote & {
  scope: string;
  sku: string;
  operationIds: string[];
  local: TeamSeoRecord;
  unsaved: Draft;
};
export type TeamProductSeoConflictArchive = {
  saved_at: string;
  local: TeamSeoRecord;
  remote: TeamSeoRecord;
  operations: Operation[];
};

const scopeKey = () => JSON.stringify([teamProject.value, teamUser.value, teamWorkspace.value, "product-seo"]);
export const teamProductSeoScope = scopeKey;
function context() {
  if (!teamUser.value || !teamWorkspace.value || teamAccessIssue.value) {
    throw new Error("Ortak SEO kayıtlarına erişilemiyor.");
  }
  return {
    scope: scopeKey(), workspace: teamWorkspace.value, user: teamUser.value, scopeRevision,
    client: teamSession.value && (teamReachable.value ?? navigator.onLine) ? teamClient.value : null,
  };
}

const cache = <T>(scope: string, request: object) => invoke<T>("team_cache", { scope, request });
const pending = (ctx: Context) => cache<Operation[]>(ctx.scope, { action: "pending" });
const transportFailure = (status: number) => status === 0 || status >= 500;
export const teamSeoSync = shallowRef({ pending: 0, conflicts: 0 });
const blockedSkus = shallowRef(new Set<string>());
let flushing: Promise<void> | undefined;
let generation = 0;
let scopeRevision = 0;
let operationRead = 0;
let operationReport = 0;
let writes: Promise<unknown> = Promise.resolve();

watch([teamProject, teamUser, teamWorkspace, teamAccessIssue], () => {
  scopeRevision++;
  blockedSkus.value = new Set();
  teamSeoSync.value = { pending: 0, conflicts: 0 };
}, { flush: "sync" });

export const productSeoHasConflict = (sku: string) => blockedSkus.value.has(sku);

function isCurrentScope(ctx: Context) {
  return ctx.scopeRevision === scopeRevision && ctx.scope === scopeKey() && !teamAccessIssue.value
    && (!ctx.client || ctx.client === teamClient.value);
}

function assertCurrentScope(ctx: Context) {
  if (!isCurrentScope(ctx)) throw new Error("Çalışma alanı değişti.");
}

async function readOperations(ctx: Context) {
  assertCurrentScope(ctx);
  const started = generation;
  const read = ++operationRead;
  const ops = await pending(ctx);
  assertCurrentScope(ctx);
  if (started === generation && read > operationReport) {
    const blocked = new Set(ops.filter((op) => op.conflict).map((op) => op.contact.sku));
    operationReport = read;
    blockedSkus.value = blocked;
    teamSeoSync.value = { pending: ops.filter((op) => !blocked.has(op.contact.sku)).length, conflicts: blocked.size };
  }
  return ops;
}

async function overlay(ctx: Context, rows: TeamSeoRecord[]) {
  const ops = await readOperations(ctx);
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
    assertCurrentScope(ctx);
    if (page.error) {
      if (transportFailure(page.status)) return overlay(ctx, await cached());
      throw new Error("Ortak SEO kayıtları okunamadı.");
    }
    rows.push(...page.data.map((row) => ({ ...row, id: row.sku })));
    if (page.data.length < 200) break;
  }
  if (started !== generation) return overlay(ctx, await cached());
  await cache(ctx.scope, { action: "snapshot", contacts: rows });
  assertCurrentScope(ctx);
  const role = await ctx.client.from("workspace_members").select("role").eq("workspace_id", ctx.workspace)
    .eq("user_id", ctx.user).maybeSingle().retry(false);
  assertCurrentScope(ctx);
  if (!role.error) {
    await cache(ctx.scope, { action: "write", key: "can-edit", data: role.data?.role === "owner" || role.data?.role === "editor" });
  }
  return overlay(ctx, started === generation ? rows : await cached());
}

export async function getTeamProductSeo(sku: string) {
  const ctx = context();
  const queued = (await readOperations(ctx)).filter((op) => op.contact.sku === sku).pop();
  if (queued) return queued.contact;
  if (ctx.client) {
    const started = generation;
    const result = await ctx.client.from("team_product_seo").select("sku,draft,revision")
      .eq("workspace_id", ctx.workspace).eq("sku", sku).maybeSingle().retry(false);
    assertCurrentScope(ctx);
    if (!result.error && started === generation) {
      const row = result.data ? { ...result.data, id: sku } : null;
      await cache(ctx.scope, { action: "write", key: `contact:${sku}`, data: row });
      assertCurrentScope(ctx);
      return row;
    }
    if (result.error && !transportFailure(result.status)) throw new Error("Ortak SEO taslağı okunamadı.");
  }
  const cached = await cache<TeamSeoRecord | null>(ctx.scope, { action: "read", key: `contact:${sku}` });
  assertCurrentScope(ctx);
  return cached;
}

function assertConflictScope(ctx: Context, signal?: AbortSignal) {
  if (signal?.aborted) throw new DOMException("SEO karşılaştırması iptal edildi.", "AbortError");
  if (!isCurrentScope(ctx)) {
    throw new Error("Ekip bağlantısı değişti. Karşılaştırmayı yeniden açın.");
  }
}

function conflictOnline(ctx: Context) {
  return ctx.client && teamSession.value && (teamReachable.value ?? navigator.onLine);
}

async function readConflictRemote(ctx: Context, sku: string, signal?: AbortSignal): Promise<ConflictRemote> {
  assertConflictScope(ctx, signal);
  if (!ctx.client || !conflictOnline(ctx)) return { remote: null, remoteState: "offline" };
  const query = ctx.client.from("team_product_seo").select("sku,draft,revision")
    .eq("workspace_id", ctx.workspace).eq("sku", sku);
  if (signal) query.abortSignal(signal);
  const result = await query.maybeSingle().retry(false);
  assertConflictScope(ctx, signal);
  if (!conflictOnline(ctx)) return { remote: null, remoteState: "offline" };
  if (result.error) {
    const remoteAccessBlocked = [401, 403].includes(result.status) || result.error.code === "42501";
    return { remote: null, remoteState: "unavailable", remoteAccessBlocked };
  }
  if (!result.data) return { remote: null, remoteState: "missing" };
  const { revision, draft, sku: remoteSku } = result.data;
  if (remoteSku !== sku || !Number.isSafeInteger(revision) || revision < 1 || !draft
    || typeof draft !== "object" || Array.isArray(draft)) return { remote: null, remoteState: "unavailable" };
  return { remote: { id: sku, sku, revision, draft }, remoteState: "ready" };
}

const conflictEditFields = ["draft_title", "draft_descriptions", "draft_search_keywords", "target_keyword", "tech_source_text"];

export async function inspectTeamProductSeoConflict(sku: string, unsaved: Draft = {}, signal?: AbortSignal): Promise<TeamProductSeoConflict> {
  const ctx = context();
  assertConflictScope(ctx, signal);
  const buffer = Object.fromEntries(conflictEditFields.filter((key) => typeof unsaved[key] === "string")
    .map((key) => [key, unsaved[key]]));
  const operations = (await readOperations(ctx)).filter((op) => op.contact.sku === sku);
  assertConflictScope(ctx, signal);
  const latest = operations[operations.length - 1]?.contact;
  if (!latest || !operations.some((op) => op.conflict)) throw new Error("Bu ürün için bekleyen çakışma kalmadı.");
  return {
    scope: ctx.scope, sku, operationIds: operations.map((op) => op.id), unsaved: buffer,
    local: { ...latest, draft: { ...latest.draft, ...buffer } }, ...await readConflictRemote(ctx, sku, signal),
  };
}

export async function acceptTeamProductSeoConflict(review: TeamProductSeoConflict, signal?: AbortSignal): Promise<boolean> {
  const ctx = context();
  const task = writes.catch(() => {}).then(async () => {
    assertConflictScope(ctx, signal);
    if (review.scope !== ctx.scope || review.local.sku !== review.sku || (review.remote && review.remote.sku !== review.sku)) {
      throw new Error("Ürün SEO karşılaştırmasını yeniden açın.");
    }
    if (!review.remote) throw new Error("Ortak SEO taslağını doğrulamak için karşılaştırmayı bağlantı varken yenileyin.");
    await flushing?.catch(() => {});
    const current = await readConflictRemote(ctx, review.sku, signal);
    if (current.remoteAccessBlocked) throw new Error("Ortak SEO taslağına erişiminizi doğrulayın.");
    if (!current.remote || !conflictOnline(ctx)) throw new Error("Ortak SEO taslağı doğrulanamadı. Karşılaştırmayı yenileyin.");
    if (current.remote.revision !== review.remote.revision || JSON.stringify(current.remote.draft) !== JSON.stringify(review.remote.draft)) {
      throw new Error("Ortak SEO taslağı inceleme sırasında değişti. Karşılaştırmayı yenileyin.");
    }
    assertConflictScope(ctx, signal);
    generation++;
    const accepted = await cache<boolean>(ctx.scope, {
      action: "resolve_product_seo_conflict", sku: review.sku, expected_ids: review.operationIds,
      remote: current.remote, unsaved: review.unsaved,
    }).catch((cause: unknown) => {
      if (String(cause).includes("unknown variant `resolve_product_seo_conflict`")) {
        throw new Error("Ortak sürümü seçmek için güncel masaüstü sürümü gerekiyor.");
      }
      throw cause;
    });
    generation++;
    await readOperations(ctx).catch(() => {});
    return accepted;
  });
  writes = task;
  return task;
}

export async function teamProductSeoConflictHistory(sku: string, signal?: AbortSignal): Promise<TeamProductSeoConflictArchive[]> {
  const ctx = context();
  assertConflictScope(ctx, signal);
  const history = await cache<TeamProductSeoConflictArchive[] | null>(ctx.scope, { action: "read", key: `product-seo-conflict:${sku}` });
  assertConflictScope(ctx, signal);
  return history ?? [];
}

async function readVerifiedProductSeo(sku: string, revision: number | null, includeDraft: boolean) {
  const ctx = context();
  if (!ctx.client) throw new Error("Bu işlem için Supabase bağlantısı gerekiyor.");
  await flushTeamProductSeo();
  const operations = await readOperations(ctx);
  if (operations.some((op) => op.contact.sku === sku)) {
    throw new Error("Bu ürünün ortak taslağını eşitleyip varsa çakışmayı çözün.");
  }
  const started = generation;
  const query = ctx.client.from("team_product_seo");
  const result = await (includeDraft ? query.select("sku,draft,revision") : query.select("revision"))
    .eq("workspace_id", ctx.workspace).eq("sku", sku).maybeSingle().retry(false);
  assertCurrentScope(ctx);
  if (result.error) throw new Error("Ortak taslak sürümü doğrulanamadı.");
  if ((result.data?.revision ?? null) !== revision) throw new Error("Ortak taslak değişti. Ürünü yeniden açın.");
  if (!includeDraft) return null;
  const row = result.data && "draft" in result.data ? { ...result.data, id: sku } : null;
  if (started === generation) await cache(ctx.scope, { action: "write", key: `contact:${sku}`, data: row });
  assertCurrentScope(ctx);
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
    assertCurrentScope(ctx);
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
    assertCurrentScope(ctx);
    await cache(ctx.scope, { action: "queue", id: operation, contact: record, payload: {
      target_workspace: ctx.workspace, operation_id: operation, target_sku: sku, expected_revision: expected, changes,
    } });
    generation++;
    await readOperations(ctx).catch(() => {});
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
      if (!isCurrentScope(ctx)) return;
      if (op.conflict) blocked.add(op.contact.sku);
      if (blocked.has(op.contact.sku)) continue;
      const response = await client.rpc("team_sync_product_seo", op.payload).retry(false);
      if (!isCurrentScope(ctx)) return;
      if (!response.error && response.data === op.contact.revision) {
        generation++;
        await cache(ctx.scope, { action: "acknowledge", id: op.id, contact: op.contact });
        generation++;
      } else if (transportFailure(response.status) || [401, 404].includes(response.status)) break;
      else {
        await cache(ctx.scope, { action: "conflict", id: op.id, reason: response.error?.message ?? "Sürüm doğrulanamadı." });
        generation++;
        blocked.add(op.contact.sku);
      }
    }
    if (isCurrentScope(ctx)) await readOperations(ctx);
  })().finally(() => { flushing = undefined; });
  return flushing;
}
