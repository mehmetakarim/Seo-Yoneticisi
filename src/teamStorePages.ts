import { invoke } from "@tauri-apps/api/core";
import { watch } from "vue";
import { api } from "./api";
import { teamAccessIssue, teamClient, teamProject, teamReachable, teamRestoring, teamSession, teamUser, teamWorkspace } from "./teamConnection";
import type { StorePageDetail } from "./types";
import { confirmTeamIdeasoftPageReceipt, fetchTeamIdeasoftInventory } from "./teamIdeasoft";
import { storeKinds, type StoreKind } from "./ideasoftInventory";

const draftFields = ["draft_page_title", "draft_meta_description", "draft_target_keyword", "draft_showcase"] as const;
type DraftValues = Pick<StorePageDetail, typeof draftFields[number]>;
type Draft = DraftValues & Partial<Pick<StorePageDetail, "draft_model">>;
type Page = StorePageDetail & { id: string; revision: number; team_scope: string };
type Facts = Partial<Pick<StorePageDetail, "name" | "slug" | "status" | "page_title" | "meta_description" | "target_keyword" | "showcase_content" | "pushed_at">> & { meta_keywords?: string };
type Pending = { id: string; contact: Page; payload: object; conflict: string | null };
type Inventory = { id: string; rejected?: boolean; startedAt?: string; rows: { kind: string; remote_id: number; facts: Facts }[] };
type InventorySync = { total: number; synced_at: string };
type ConflictRemote = {
  remote: Page | null;
  remoteState: "ready" | "offline" | "missing" | "unavailable";
  remoteAccessBlocked?: boolean;
};
export type TeamStorePageConflict = ConflictRemote & {
  scope: string;
  kind: StoreKind;
  remoteId: number;
  operationIds: string[];
  local: StorePageDetail;
  unsaved: Partial<DraftValues>;
};
export type TeamStorePageConflictArchive = { saved_at: string; local: Page; remote: Page; operations: Pending[] };
export type TeamStorePageWriteTarget = { key: string; workspace: string };
export const teamStorePageScope = () => JSON.stringify([teamProject.value, teamUser.value, teamWorkspace.value, "store-pages"]);
const scope = teamStorePageScope;
const rowId = (kind: string, id: number) => `${kind}:${id}`;
const client = () => teamSession.value && (teamReachable.value ?? navigator.onLine) ? teamClient.value : null;
const cache = <T>(key: string, request: object) => invoke<T>("team_cache", { scope: key, request });
const pending = (key: string) => cache<Pending[]>(key, { action: "pending" });
const draftOperation = (op: Pending) => !("facts" in op.payload) && !("receipt" in op.payload);
let flushing: Promise<void> | undefined;
let syncingInventory: { key: string; task: ReturnType<typeof syncSharedInventory> } | undefined;
let cacheWrites: Promise<unknown> = Promise.resolve();
let scopeRevision = 0;
let listRequest = 0;
const cacheRevisions = new Map<string, number>();
const committedLists = new Map<string, number>();

watch([teamProject, teamUser, teamWorkspace, teamAccessIssue], () => {
  scopeRevision++;
  cacheRevisions.clear();
  committedLists.clear();
}, { flush: "sync" });

function serializeCacheWrite<T>(write: () => Promise<T>): Promise<T> {
  const task = cacheWrites.catch(() => {}).then(write);
  cacheWrites = task;
  return task;
}

const cachedKindRevision = (kind: string) => cacheRevisions.get(kind) ?? 0;
function markCacheChanged(kind: string) {
  cacheRevisions.set(kind, cachedKindRevision(kind) + 1);
}

function readContext() {
  return { key: scope(), scopeRevision, workspace: teamWorkspace.value, user: teamUser.value, remote: client() };
}

type ReadContext = ReturnType<typeof readContext>;
function isCurrent(ctx: ReadContext) {
  return ctx.scopeRevision === scopeRevision && ctx.key === scope() && !teamAccessIssue.value &&
    (!ctx.remote || ctx.remote === teamClient.value);
}

function assertCurrent(ctx: ReadContext, signal?: AbortSignal) {
  if (signal?.aborted) throw new DOMException("Mağaza taslağı karşılaştırması iptal edildi.", "AbortError");
  if (!isCurrent(ctx)) throw new Error("Çalışma alanı değişti.");
}

function withoutPendingFlags(page: Page): Page {
  return page.team_pending || page.team_conflict || page.team_draft_conflict
    ? { ...page, team_conflict: false, team_draft_conflict: false, team_pending: false }
    : page;
}

function overlayStorePage(base: Page | undefined, op: Pending): Page {
  const draft = draftOperation(op);
  const { contact } = op;
  const changes = draft ? {
    draft_page_title: contact.draft_page_title, draft_meta_description: contact.draft_meta_description,
    draft_target_keyword: contact.draft_target_keyword, draft_showcase: contact.draft_showcase,
    draft_model: contact.draft_model, draft_at: contact.draft_at, revision: contact.revision,
  } : "facts" in op.payload ? op.payload.facts as Facts : {};
  return {
    ...(base ?? contact), ...changes,
    team_conflict: !!base?.team_conflict || !!op.conflict,
    team_draft_conflict: !!base?.team_draft_conflict || (draft && !!op.conflict),
    team_pending: true,
  };
}

export async function sharedStoreInventoryStatus() {
  if ((!teamWorkspace.value && !teamUser.value) || teamAccessIssue.value) return null;
  const key = scope();
  const [queued, latest] = await Promise.all([
    cache<Inventory | null>(key, { action: "read", key: "inventory" }),
    cache<InventorySync | null>(key, { action: "read", key: "last-inventory-sync" }),
  ]);
  return key === scope() && !teamAccessIssue.value ? { queued, latest } : null;
}

export async function listTeamStorePages(kind: string): Promise<StorePageDetail[]> {
  if (teamStorePageWriteMode() === "local") return api.listStorePages(kind);
  if (!teamWorkspace.value || !teamUser.value || teamAccessIssue.value) return [];
  const ctx = readContext();
  const { key, workspace, remote, user } = ctx;
  const request = ++listRequest;
  const started = cachedKindRevision(kind);
  const superseded = () => started !== cachedKindRevision(kind) || request < (committedLists.get(kind) ?? 0);
  const cached = () => cache<Page[] | null>(key, { action: "read", key: kind });
  let rows = await cached();
  assertCurrent(ctx);
  if (remote) {
    const fetched: Page[] = [];
    for (let start = 0; ; start += 100) {
      const result = await remote.from("team_store_pages").select("remote_id,details,revision")
        .eq("workspace_id", workspace).eq("kind", kind).order("remote_id").range(start, start + 99).retry(false);
      assertCurrent(ctx);
      if (result.error) {
        if (result.status === 0 || result.status >= 500 || result.status === 404) break;
        throw new Error("Ortak mağaza sayfaları okunamadı.");
      }
      fetched.push(...result.data.map((row) => ({
        ...row.details, id: rowId(kind, row.remote_id), kind, remote_id: row.remote_id,
        revision: row.revision, team_scope: key,
      })));
      if (result.data.length < 100) {
        rows = await serializeCacheWrite(async () => {
          assertCurrent(ctx);
          if (superseded()) return cached();
          await cache(key, { action: "write", key: kind, data: fetched });
          assertCurrent(ctx);
          committedLists.set(kind, request);
          await cache(key, { action: "write", key: "available", data: true });
          return fetched;
        });
        break;
      }
    }
    assertCurrent(ctx);
    const role = await remote.from("workspace_members").select("role").eq("workspace_id", workspace)
      .eq("user_id", user).maybeSingle().retry(false);
    assertCurrent(ctx);
    if (!role.error) {
      await cache(key, { action: "write", key: "can-edit", data: ["owner", "editor"].includes(role.data?.role) });
    }
  }
  return serializeCacheWrite(async () => {
    // Keep a newer list, acknowledged update or resolved row when fetches overlap.
    assertCurrent(ctx);
    if (superseded()) rows = await cached();
    const operations = await pending(key);
    assertCurrent(ctx);
    const merged = new Map((rows ?? []).map((page) => [page.id, withoutPendingFlags(page)]));
    for (const op of operations) {
      if (op.contact.kind === kind) merged.set(op.contact.id, overlayStorePage(merged.get(op.contact.id), op));
    }
    return [...merged.values()].sort((a, b) => b.impressions - a.impressions || a.name.localeCompare(b.name, "tr"));
  });
}

function conflictContext(page: Pick<StorePageDetail, "kind" | "remote_id" | "team_scope">) {
  const ctx = readContext();
  const kind = storeKinds.find((entry) => entry === page.kind);
  if (!ctx.workspace || !ctx.user || page.team_scope !== ctx.key || teamAccessIssue.value || !kind ||
      !Number.isSafeInteger(page.remote_id) || page.remote_id <= 0) {
    throw new Error("Ortak sayfanın çalışma alanı doğrulanamadı.");
  }
  return { ctx, kind };
}

async function readConflictRemote(ctx: ReadContext, kind: StoreKind, id: number, signal?: AbortSignal): Promise<ConflictRemote> {
  assertCurrent(ctx, signal);
  if (!ctx.remote || !client()) return { remote: null, remoteState: "offline" };
  const query = ctx.remote.from("team_store_pages").select("remote_id,details,revision")
    .eq("workspace_id", ctx.workspace).eq("kind", kind).eq("remote_id", id);
  if (signal) query.abortSignal(signal);
  const result = await query.maybeSingle().retry(false);
  assertCurrent(ctx, signal);
  if (!client()) return { remote: null, remoteState: "offline" };
  if (result.error) {
    return { remote: null, remoteState: "unavailable", remoteAccessBlocked: [401, 403].includes(result.status) || result.error.code === "42501" };
  }
  if (!result.data) return { remote: null, remoteState: "missing" };
  const { remote_id, details, revision } = result.data;
  if (remote_id !== id || !Number.isSafeInteger(revision) || revision < 1 ||
      !details || typeof details !== "object" || Array.isArray(details)) return { remote: null, remoteState: "unavailable" };
  return {
    remoteState: "ready",
    remote: { ...details, id: rowId(kind, id), kind, remote_id, revision, team_scope: ctx.key },
  };
}

export async function inspectTeamStorePageConflict(page: StorePageDetail, unsaved: Partial<DraftValues> = {}, signal?: AbortSignal): Promise<TeamStorePageConflict> {
  const { ctx, kind } = conflictContext(page);
  assertCurrent(ctx, signal);
  const id = rowId(kind, page.remote_id);
  const buffered: Partial<DraftValues> = {};
  for (const key of draftFields) if (typeof unsaved[key] === "string") buffered[key] = unsaved[key];
  const operations = (await pending(ctx.key)).filter((op) => op.contact.id === id && draftOperation(op));
  assertCurrent(ctx, signal);
  const queued = operations[operations.length - 1]?.contact;
  if (queued ? !operations.some((op) => op.conflict) : !Object.keys(buffered).length) {
    throw new Error("Bu sayfa için incelenecek bir taslak çakışması kalmadı.");
  }
  const shared = await readConflictRemote(ctx, kind, page.remote_id, signal);
  return {
    ...shared, scope: ctx.key, kind, remoteId: page.remote_id,
    operationIds: operations.map((op) => op.id), unsaved: buffered,
    local: { ...(queued ?? shared.remote ?? page), ...buffered },
  };
}

export async function acceptTeamStorePageConflict(review: TeamStorePageConflict, signal?: AbortSignal): Promise<StorePageDetail | null> {
  const { ctx, kind } = conflictContext({ kind: review.kind, remote_id: review.remoteId, team_scope: review.scope });
  const id = rowId(kind, review.remoteId);
  if (!review.remote || review.remote.id !== id || review.remote.team_scope !== ctx.key) {
    throw new Error("Ortak mağaza taslağını doğrulamak için karşılaştırmayı bağlantı varken yenileyin.");
  }
  const reviewedRemote = review.remote;
  await flushing?.catch(() => {});
  return serializeCacheWrite(async () => {
    const result = await readConflictRemote(ctx, kind, review.remoteId, signal);
    if (result.remoteAccessBlocked) throw new Error("Ortak mağaza taslağına erişiminizi doğrulayın.");
    const fresh = result.remote;
    if (!fresh || !client()) throw new Error("Ortak taslak doğrulanamadı. Karşılaştırmayı yenileyin.");
    if (JSON.stringify(fresh) !== JSON.stringify(reviewedRemote)) {
      throw new Error("Ortak mağaza kaydı inceleme sırasında değişti. Karşılaştırmayı yenileyin.");
    }
    assertCurrent(ctx, signal);
    const accepted = await cache<boolean>(ctx.key, {
      action: "resolve_store_page_conflict", kind, remote_id: review.remoteId,
      expected_ids: review.operationIds, remote: fresh, unsaved: review.unsaved,
    }).catch((error: unknown) => {
      if (String(error).includes("unknown variant `resolve_store_page_conflict`")) {
        throw new Error("Ortak sürümü seçmek için güncel masaüstü sürümü gerekiyor.");
      }
      throw error;
    });
    if (!accepted) return null;
    if (isCurrent(ctx)) markCacheChanged(kind);
    // The archive is committed. A receipt may still be queued for this page independently.
    const operations = await pending(ctx.key).catch(() => []);
    return operations.filter((op) => op.contact.id === id).reduce(overlayStorePage, fresh);
  });
}

export async function teamStorePageConflictHistory(page: StorePageDetail, signal?: AbortSignal): Promise<TeamStorePageConflictArchive[]> {
  const { ctx, kind } = conflictContext(page);
  assertCurrent(ctx, signal);
  const history = await cache<TeamStorePageConflictArchive[] | null>(ctx.key,
    { action: "read", key: `store-page-conflict:${rowId(kind, page.remote_id)}` });
  assertCurrent(ctx, signal);
  return history ?? [];
}

export function teamStorePageWriteMode(page?: StorePageDetail): "team" | "local" {
  return page?.team_scope || teamProject.value || teamUser.value || teamWorkspace.value || teamRestoring.value ? "team" : "local";
}

export async function storePageWriteTarget(page?: StorePageDetail): Promise<TeamStorePageWriteTarget | undefined> {
  if (teamStorePageWriteMode(page) === "local") return undefined;
  const key = scope();
  const workspace = teamWorkspace.value;
  const user = teamUser.value;
  const remote = client();
  if (!workspace || !user || !remote || teamAccessIssue.value || (page && page.team_scope !== key)) {
    throw new Error("Mağaza işlemi için ortak çalışma alanını çevrimiçi açın.");
  }
  if (!page && !await cache<boolean>(key, { action: "read", key: "available" })) {
    const probe = await remote.from("team_store_pages").select("remote_id").eq("workspace_id", workspace).limit(1).retry(false);
    if (probe.error || key !== scope()) throw new Error("Ortak mağaza alanı doğrulanamadı.");
    await cache(key, { action: "write", key: "available", data: true });
  }
  const role = await remote.from("workspace_members").select("role").eq("workspace_id", workspace)
    .eq("user_id", user).maybeSingle().retry(false);
  if (scope() !== key || role.error || !["owner", "editor"].includes(role.data?.role)) {
    throw new Error("Çalışma alanında mağaza düzenleme yetkisi doğrulanamadı.");
  }
  return { key, workspace };
}

export async function checkStorePageDraft(page: StorePageDetail) {
  const target = await storePageWriteTarget(page);
  if (!target) return;
  await flushTeamStorePages();
  const remote = client();
  if (target.key !== scope() || !remote) throw new Error("Çalışma alanı bağlantısı değişti.");
  const result = await remote.from("team_store_pages").select("revision").eq("workspace_id", target.workspace)
    .eq("kind", page.kind).eq("remote_id", page.remote_id).maybeSingle().retry(false);
  const queued = (await pending(target.key)).some((op) => op.contact.id === rowId(page.kind, page.remote_id) && draftOperation(op));
  if (target.key !== scope() || result.error || page.revision == null || result.data?.revision !== page.revision || queued) {
    throw new Error("Taslak sürümü doğrulanamadı. Ortak sayfayı yenileyin.");
  }
  return target;
}

export async function recordTeamStorePage(target: TeamStorePageWriteTarget, page: StorePageDetail, facts: Facts, receipt: string) {
  if (target.key !== scope() || target.workspace !== teamWorkspace.value) throw new Error("Çalışma alanı değişti.");
  const record: Page = { ...page, ...facts, id: rowId(page.kind, page.remote_id), revision: page.revision ?? 1, team_scope: target.key };
  await cache(target.key, { action: "queue", id: crypto.randomUUID(), contact: record,
    payload: { facts, receipt },
  });
  return { ...record, team_pending: true };
}

export function syncTeamStoreInventory() {
  if (teamStorePageWriteMode() === "local") return api.syncStorePages();
  const key = scope();
  if (syncingInventory?.key === key) return syncingInventory.task;
  const task = syncSharedInventory().finally(() => {
    if (syncingInventory?.task === task) syncingInventory = undefined;
  });
  syncingInventory = { key, task };
  return task;
}

async function syncSharedInventory() {
  const target = await storePageWriteTarget();
  if (!target) throw new Error("Ortak mağaza çalışma alanı hazır değil. Sistem → Ekip bağlantısını kontrol edin.");
  await flushTeamStorePages();
  if (target.key !== scope()) throw new Error("Çalışma alanı değişti.");
  const previous = await cache<Inventory | null>(target.key, { action: "read", key: "inventory" });
  if (previous && !previous.rejected) {
    throw new Error("Önceki envanterin Supabase'e eşitlenmesi bekleniyor. Bağlantıyı kontrol edin.");
  }
  const remote = client();
  if (!remote) throw new Error("Ortak çalışma alanı bağlantısı değişti.");
  const clock = await remote.rpc("team_begin_store_inventory", { target_workspace: target.workspace }).retry(false);
  const missingMigration = clock.status === 404 && clock.error?.code === "PGRST202";
  if ((!missingMigration && clock.error) || target.key !== scope()) throw new Error("Ortak envanter başlangıcı doğrulanamadı.");
  const startedAt = typeof clock.data === "string" ? clock.data : undefined;
  const pages = await fetchTeamIdeasoftInventory();
  if (target.key !== scope()) throw new Error("Çalışma alanı değişti.");
  const rows = pages.map((page) => ({ kind: page.kind, remote_id: page.id, facts: {
    name: page.name, slug: page.slug, status: page.status, page_title: page.page_title,
    meta_description: page.meta_description, meta_keywords: page.meta_keywords, target_keyword: page.target_keyword, showcase_content: page.showcase_content,
  } }));
  const inventory: Inventory = { id: crypto.randomUUID(), rows, startedAt };
  await cache(target.key, { action: "write", key: "inventory", data: inventory });
  await flushTeamStorePages();
  if (target.key !== scope()) throw new Error("Çalışma alanı değişti.");
  const remaining = await cache<Inventory | null>(target.key, { action: "read", key: "inventory" });
  if (remaining?.rejected) throw new Error("Supabase yeni envanteri kabul etmedi. Ortak kayıtlar korundu. Envanteri yeniden alabilirsiniz.");
  const shared_pending = !!remaining;
  const summary: [string, number, number, number][] = [];
  for (const kind of storeKinds) {
    const existing = await listTeamStorePages(kind);
    if (target.key !== scope()) throw new Error("Çalışma alanı değişti.");
    const visible = new Set(existing.filter((row) => row.impressions > 0).map((row) => row.remote_id));
    const fresh = pages.filter((row) => row.kind === kind);
    summary.push([kind, fresh.length, fresh.filter((row) => visible.has(row.id)).length,
      fresh.filter((row) => !row.page_title.trim() || !row.meta_description.trim()).length]);
  }
  return { total: pages.length, by_kind: summary, synced_at: new Date().toISOString(), shared_pending };
}

export async function queueTeamPageDraft(page: StorePageDetail, draft: Draft): Promise<StorePageDetail> {
  const ctx = readContext();
  const { key, workspace } = ctx;
  if (!workspace || !ctx.user || page.team_scope !== key || page.revision == null || teamAccessIssue.value) {
    throw new Error("Ortak sayfanın çalışma alanı değişti.");
  }
  const revision = page.revision;
  return serializeCacheWrite(async () => {
    assertCurrent(ctx);
    if (!await cache<boolean>(key, { action: "read", key: "can-edit" })) throw new Error("Ortak taslak düzenleme yetkisi yok.");
    const id = rowId(page.kind, page.remote_id);
    const ops = (await pending(key)).filter((op) => op.contact.id === id && draftOperation(op));
    if (ops.some((op) => op.conflict) || (ops.length && ops[ops.length - 1].contact.revision !== revision)) {
      throw new Error("Ortak taslakta sürüm çakışması var.");
    }
    assertCurrent(ctx);
    const changes = { ...draft, draft_at: new Date().toISOString().slice(0, 19) };
    const operation = crypto.randomUUID();
    const saved: Page = { ...page, ...changes, id, revision: revision + 1, team_scope: key };
    await cache(key, { action: "queue", id: operation, contact: saved, payload: {
      target_workspace: workspace, target_kind: page.kind, target_id: page.remote_id,
      operation_id: operation, expected_revision: revision, changes,
    } });
    return { ...saved, team_pending: true };
  });
}

function acknowledgePage(ctx: ReadContext, op: Pending, exact?: Page, started?: number) {
  return serializeCacheWrite(async () => {
    const { id, kind } = op.contact;
    if (!isCurrent(ctx)) return;
    const local = await cache<Page[] | null>(ctx.key, { action: "read", key: kind });
    if (!isCurrent(ctx)) return;
    const rows = new Map((local ?? []).map((page) => [page.id, page]));
    const prior = rows.get(id);
    if (exact ? started === cachedKindRevision(kind) && (!prior || prior.revision <= exact.revision)
      : !draftOperation(op) || !prior || prior.revision < op.contact.revision) {
      const saved = exact ?? overlayStorePage(prior, op);
      rows.set(id, withoutPendingFlags(saved));
      await cache(ctx.key, { action: "write", key: kind, data: [...rows.values()] });
      if (isCurrent(ctx)) markCacheChanged(kind);
    }
    await cache(ctx.key, { action: "acknowledge", id: op.id, contact: exact ?? op.contact });
  });
}

export function flushTeamStorePages(): Promise<void> {
  if (flushing) return flushing;
  const ctx = readContext();
  const { key, user, workspace, remote } = ctx;
  if (!remote || !user || !workspace || teamAccessIssue.value) return Promise.resolve();
  flushing = (async () => {
    const blocked = new Set<string>();
    for (const op of await pending(key)) {
      if (!isCurrent(ctx)) return;
      const draft = draftOperation(op);
      if (op.conflict && draft) blocked.add(op.contact.id);
      if (op.conflict || (draft && blocked.has(op.contact.id))) continue;
      if ("receipt" in op.payload && typeof op.payload.receipt === "string") {
        const receipt = await confirmTeamIdeasoftPageReceipt(op.payload.receipt);
        if (!isCurrent(ctx)) return;
        if (receipt === "retry") return;
        if (receipt === "conflict") {
          await cache(key, { action: "conflict", id: op.id, reason: "Mağaza kaydı doğrulanamadı. Mağaza envanterini yenileyin." });
          continue;
        }
        const started = cachedKindRevision(op.contact.kind);
        const current = await remote.from("team_store_pages").select("details,revision")
          .eq("workspace_id", workspace).eq("kind", op.contact.kind).eq("remote_id", op.contact.remote_id).maybeSingle().retry(false);
        if (current.error || !current.data || !isCurrent(ctx)) return;
        const exact: Page = { ...op.contact, ...current.data.details, revision: current.data.revision };
        await acknowledgePage(ctx, op, exact, started);
        continue;
      }
      const result = await remote.rpc(draft ? "team_patch_store_page_draft" : "team_record_store_page", op.payload).retry(false);
      if (!isCurrent(ctx)) return;
      if (!result.error && (draft ? result.data === op.contact.revision : typeof result.data === "number")) {
        await acknowledgePage(ctx, op);
      } else if (result.status === 0 || result.status >= 500 || [401, 404].includes(result.status)) {
        return;
      } else {
        await cache(key, { action: "conflict", id: op.id, reason: result.error?.message ?? "Sürüm doğrulanamadı." });
        if (draft) blocked.add(op.contact.id);
      }
    }
    if (!isCurrent(ctx)) return;
    const inventory = await cache<Inventory | null>(key, { action: "read", key: "inventory" });
    if (!inventory || inventory.rejected || !isCurrent(ctx)) return;
    const result = await remote.rpc(inventory.startedAt ? "team_record_store_inventory_observed" : "team_record_store_inventory", {
      target_workspace: workspace, inventory: inventory.rows, ...(inventory.startedAt ? { observed_at: inventory.startedAt } : {}),
    }).retry(false);
    const accepted = !result.error && result.data === inventory.rows.length;
    const rejected = !!result.error && [400, 403, 409, 413, 422].includes(result.status);
    if ((!accepted && !rejected) || !isCurrent(ctx)) return;
    const latest = await cache<Inventory | null>(key, { action: "read", key: "inventory" });
    if (latest?.id !== inventory.id || !isCurrent(ctx)) return;
    if (accepted) await cache(key, { action: "write", key: "last-inventory-sync", data: {
      total: inventory.rows.length, synced_at: new Date().toISOString(),
    } });
    await cache(key, { action: "write", key: "inventory", data: accepted ? null : { ...inventory, rejected: true } });
  })().finally(() => { flushing = undefined; });
  return flushing;
}
