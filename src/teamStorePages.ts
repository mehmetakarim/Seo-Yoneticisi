import { invoke } from "@tauri-apps/api/core";
import { api } from "./api";
import { teamAccessIssue, teamClient, teamProject, teamReachable, teamRestoring, teamSession, teamUser, teamWorkspace } from "./teamConnection";
import type { StorePageDetail } from "./types";
import { confirmTeamIdeasoftPageReceipt, fetchTeamIdeasoftInventory } from "./teamIdeasoft";
import { storeKinds } from "./ideasoftInventory";

type Draft = Pick<StorePageDetail, "draft_page_title" | "draft_meta_description" | "draft_target_keyword" | "draft_showcase">
  & Partial<Pick<StorePageDetail, "draft_model">>;
type Page = StorePageDetail & { id: string; revision: number; team_scope: string };
type Facts = Partial<Pick<StorePageDetail, "name" | "slug" | "status" | "page_title" | "meta_description" | "target_keyword" | "showcase_content" | "pushed_at">> & { meta_keywords?: string };
type Pending = { id: string; contact: Page; payload: object; conflict: string | null };
type Inventory = { id: string; rejected?: boolean; startedAt?: string; rows: { kind: string; remote_id: number; facts: Facts }[] };
type InventorySync = { total: number; synced_at: string };
export type TeamStorePageWriteTarget = { key: string; workspace: string };
export const teamStorePageScope = () => JSON.stringify([teamProject.value, teamUser.value, teamWorkspace.value, "store-pages"]);
const scope = teamStorePageScope;
const rowId = (kind: string, id: number) => `${kind}:${id}`;
const client = () => teamSession.value && (teamReachable.value ?? navigator.onLine) ? teamClient.value : null;
const cache = <T>(key: string, request: object) => invoke<T>("team_cache", { scope: key, request });
const pending = (key: string) => cache<Pending[]>(key, { action: "pending" });
let flushing: Promise<void> | undefined;
let syncingInventory: { key: string; task: ReturnType<typeof syncSharedInventory> } | undefined;

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
  const workspace = teamWorkspace.value;
  if (teamStorePageWriteMode() === "local") return api.listStorePages(kind);
  if (!workspace || !teamUser.value || teamAccessIssue.value) return [];
  const key = scope();
  const remote = client();
  let rows = await cache<Page[] | null>(key, { action: "read", key: kind });
  if (remote) {
    const fetched: Page[] = [];
    for (let start = 0; ; start += 100) {
      const result = await remote.from("team_store_pages").select("remote_id,details,revision")
        .eq("workspace_id", workspace).eq("kind", kind).order("remote_id").range(start, start + 99).retry(false);
      if (key !== scope()) throw new Error("Çalışma alanı değişti.");
      if (result.error) {
        if (result.status === 0 || result.status >= 500 || result.status === 404) break;
        throw new Error("Ortak mağaza sayfaları okunamadı.");
      }
      fetched.push(...result.data.map((row) => ({
        ...row.details, id: rowId(kind, row.remote_id), kind, remote_id: row.remote_id,
        revision: row.revision, team_scope: key,
      })));
      if (result.data.length < 100) {
        rows = fetched;
        await cache(key, { action: "write", key: kind, data: rows });
        await cache(key, { action: "write", key: "available", data: true });
        break;
      }
    }
    const role = await remote.from("workspace_members").select("role").eq("workspace_id", workspace)
      .eq("user_id", teamUser.value).maybeSingle().retry(false);
    if (!role.error && scope() === key) {
      await cache(key, { action: "write", key: "can-edit", data: ["owner", "editor"].includes(role.data?.role) });
    }
  }
  if (key !== scope()) throw new Error("Çalışma alanı değişti.");
  const merged = new Map((rows ?? []).map((page) => [page.id, page]));
  for (const op of await pending(key)) {
    if (op.contact.kind !== kind) continue;
    const server = merged.get(op.contact.id);
    const facts = "facts" in op.payload ? op.payload.facts as Facts : undefined;
    if (server || facts) merged.set(op.contact.id, {
      ...(server && facts ? { ...server, ...facts } : op.contact),
      team_conflict: server?.team_conflict || !!op.conflict, team_pending: true,
    });
  }
  return [...merged.values()].sort((a, b) => b.impressions - a.impressions || a.name.localeCompare(b.name, "tr"));
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
  const queued = (await pending(target.key)).some((op) => op.contact.id === rowId(page.kind, page.remote_id) && !("facts" in op.payload));
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
  const key = scope();
  if (!teamWorkspace.value || page.team_scope !== key || page.revision == null || teamAccessIssue.value) {
    throw new Error("Ortak sayfanın çalışma alanı değişti.");
  }
  if (!await cache<boolean>(key, { action: "read", key: "can-edit" })) throw new Error("Ortak taslak düzenleme yetkisi yok.");
  const id = rowId(page.kind, page.remote_id);
  const ops = (await pending(key)).filter((op) => op.contact.id === id && !("facts" in op.payload));
  if (ops.some((op) => op.conflict) || (ops.length && ops[ops.length - 1].contact.revision !== page.revision)) {
    throw new Error("Ortak taslakta sürüm çakışması var.");
  }
  if (key !== scope()) throw new Error("Çalışma alanı değişti.");
  const changes = { ...draft, draft_at: new Date().toISOString().slice(0, 19) };
  const operation = crypto.randomUUID();
  const saved: Page = { ...page, ...changes, id, revision: page.revision + 1, team_scope: key };
  await cache(key, { action: "queue", id: operation, contact: saved, payload: {
    target_workspace: teamWorkspace.value, target_kind: page.kind, target_id: page.remote_id,
    operation_id: operation, expected_revision: page.revision, changes,
  } });
  return { ...saved, team_pending: true };
}

export function flushTeamStorePages(): Promise<void> {
  if (flushing) return flushing;
  const remote = client();
  if (!remote || !teamUser.value || !teamWorkspace.value || teamAccessIssue.value) return Promise.resolve();
  const key = scope();
  const workspace = teamWorkspace.value;
  flushing = (async () => {
    const blocked = new Set<string>();
    for (const op of await pending(key)) {
      if (key !== scope()) return;
      const facts = "facts" in op.payload ? op.payload.facts as Facts : undefined;
      if (op.conflict && !facts) blocked.add(op.contact.id);
      if (op.conflict || (!facts && blocked.has(op.contact.id))) continue;
      if ("receipt" in op.payload && typeof op.payload.receipt === "string") {
        const receipt = await confirmTeamIdeasoftPageReceipt(op.payload.receipt);
        if (key !== scope()) return;
        if (receipt === "retry") return;
        if (receipt === "conflict") {
          await cache(key, { action: "conflict", id: op.id, reason: "Mağaza kaydı doğrulanamadı. Mağaza envanterini yenileyin." });
          continue;
        }
        const current = await remote.from("team_store_pages").select("details,revision")
          .eq("workspace_id", workspace).eq("kind", op.contact.kind).eq("remote_id", op.contact.remote_id).maybeSingle().retry(false);
        if (current.error || !current.data || key !== scope()) return;
        const exact: Page = { ...op.contact, ...current.data.details, revision: current.data.revision };
        const local = await cache<Page[]>(key, { action: "read", key: exact.kind });
        const rows = new Map((local ?? []).map((page) => [page.id, page]));
        rows.set(exact.id, exact);
        await cache(key, { action: "write", key: exact.kind, data: [...rows.values()] });
        await cache(key, { action: "acknowledge", id: op.id, contact: exact });
        continue;
      }
      const result = await remote.rpc(facts ? "team_record_store_page" : "team_patch_store_page_draft", op.payload).retry(false);
      if (!result.error && (facts ? typeof result.data === "number" : result.data === op.contact.revision)) {
        const local = await cache<Page[]>(key, { action: "read", key: op.contact.kind });
        const rows = new Map((local ?? []).map((page) => [page.id, page]));
        const prior = rows.get(op.contact.id);
        if (facts) rows.set(op.contact.id, prior ? { ...prior, ...facts } : op.contact);
        else if (prior && prior.revision < op.contact.revision) rows.set(op.contact.id, op.contact);
        await cache(key, { action: "write", key: op.contact.kind, data: [...rows.values()] });
        await cache(key, { action: "acknowledge", id: op.id, contact: op.contact });
      } else if (result.status === 0 || result.status >= 500 || [401, 404].includes(result.status)) {
        return;
      } else {
        await cache(key, { action: "conflict", id: op.id, reason: result.error?.message ?? "Sürüm doğrulanamadı." });
        if (!facts) blocked.add(op.contact.id);
      }
    }
    if (scope() !== key) return;
    const inventory = await cache<Inventory | null>(key, { action: "read", key: "inventory" });
    if (!inventory || inventory.rejected || scope() !== key) return;
    const result = await remote.rpc(inventory.startedAt ? "team_record_store_inventory_observed" : "team_record_store_inventory", {
      target_workspace: workspace, inventory: inventory.rows, ...(inventory.startedAt ? { observed_at: inventory.startedAt } : {}),
    }).retry(false);
    const accepted = !result.error && result.data === inventory.rows.length;
    const rejected = !!result.error && [400, 403, 409, 413, 422].includes(result.status);
    if (!accepted && !rejected) return;
    const latest = await cache<Inventory | null>(key, { action: "read", key: "inventory" });
    if (latest?.id !== inventory.id) return;
    if (accepted) await cache(key, { action: "write", key: "last-inventory-sync", data: {
      total: inventory.rows.length, synced_at: new Date().toISOString(),
    } });
    await cache(key, { action: "write", key: "inventory", data: accepted ? null : { ...inventory, rejected: true } });
  })().finally(() => { flushing = undefined; });
  return flushing;
}
