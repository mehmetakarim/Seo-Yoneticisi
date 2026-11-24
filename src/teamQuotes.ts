import { invoke } from "@tauri-apps/api/core";
import { shallowRef } from "vue";
import { teamAccessIssue, teamClient, teamProject, teamReachable, teamSession, teamUser, teamWorkspace } from "./teamConnection";
import { teamContactProgress } from "./teamContacts";
import { sortQuotesByActivity } from "./quoteOrder";
import type { Quote, QuoteItem, QuoteSummary } from "./types";

const columns = "id,no,contact_id,status,currency,fx_rate,fx_date,valid_until,note,close_reason,created_at,sent_at,revision,updated_at,team_contacts(name),team_quote_items(id,sku,name,qty,unit_price,tax_rate,cost,sort),team_quote_versions(count)";
type QuoteRow = Omit<Quote, "contact_name" | "items" | "version_count"> & {
  updated_at: string;
  team_contacts: { name: string } | null;
  team_quote_items: (QuoteItem & { sort: number })[];
  team_quote_versions: { count: number }[];
};
export type TeamQuoteVersion = {
  version: number;
  at: string;
  snapshot_json: {
    quote: Pick<Quote, "no" | "currency" | "note" | "valid_until">;
    items: { name: string; qty: number | null; unit_price: number | null; tax_rate: number | null }[];
  };
};
export type TeamTrashedQuote = {
  id: string; no: string; status: string; currency: string; deleted_at: string; revision: number;
};
export type TeamQuoteConflict = {
  scope: string;
  operationIds: string[];
  local: Quote;
  remote: Quote | null;
  remoteState: "ready" | "offline" | "missing" | "unavailable";
};
type QuoteLifecycleAction = "revise" | "trash" | "restore";
type QuoteTrashMarker = { quote_id: string; revision: number; trashed: boolean };

function currentScope() {
  return JSON.stringify([teamProject.value, teamUser.value, teamWorkspace.value, "quotes"]);
}

function context() {
  const workspace = teamWorkspace.value;
  if (!workspace || !teamProject.value || !teamUser.value) {
    throw new Error("Sistem → Ekip bölümünden bir çalışma alanı seçin.");
  }
  return {
    workspace,
    scope: currentScope(),
    client: teamSession.value && (teamReachable.value ?? navigator.onLine) ? teamClient.value : null,
  };
}

const cache = <T>(scope: string, request: object) => invoke<T>("team_cache", { scope, request });
export const teamQuotesOffline = shallowRef(false);
export const teamQuoteQueue = shallowRef({ pending: 0, conflicts: 0 });
export const teamQuoteConflicts = shallowRef<ReadonlySet<Quote["id"]>>(new Set());
export const teamQuoteCanEdit = shallowRef(false);
export const teamQuoteSaving = shallowRef(false);
type Context = ReturnType<typeof context>;
type Operation = { id: string; contact: Quote; payload: Record<string, unknown>; conflict: string | null };
const sameScope = (ctx: Context) => ctx.scope === currentScope() && (!ctx.client || ctx.client === teamClient.value);
const pending = (scope: string) => cache<Operation[]>(scope, { action: "pending" });
const trashCacheKey = "quote-trash-state";
let trashCacheWrite: Promise<unknown> = Promise.resolve();
let flushing: Promise<void> | undefined;

export function resetTeamQuoteIndicators() {
  teamQuotesOffline.value = false;
  teamQuoteQueue.value = { pending: 0, conflicts: 0 };
  teamQuoteConflicts.value = new Set();
  teamQuoteCanEdit.value = false;
}

function assertQuoteScope(ctx: Context) {
  if (!sameScope(ctx)) throw new Error("Çalışma alanı değişti. Teklifi yeniden açın.");
  if (teamAccessIssue.value) throw new Error("Bu çalışma alanının tekliflerine erişilemiyor.");
}

function unpackQuote({ team_contacts, team_quote_items, team_quote_versions, ...quote }: QuoteRow) {
  return {
    ...quote,
    contact_name: team_contacts?.name ?? "",
    items: team_quote_items.sort((a, b) => a.sort - b.sort || String(a.id).localeCompare(String(b.id))),
    version_count: team_quote_versions[0]?.count ?? 0,
  };
}

function assertConflictScope(ctx: Context, signal?: AbortSignal) {
  if (signal?.aborted || !sameScope(ctx)) {
    throw new DOMException("Teklif karşılaştırması iptal edildi.", "AbortError");
  }
  assertQuoteScope(ctx);
}

async function readConflictRemote(ctx: Context, id: string, signal?: AbortSignal):
    Promise<Pick<TeamQuoteConflict, "remote" | "remoteState">> {
  const offline = () => teamReachable.value === false;
  assertConflictScope(ctx, signal);
  if (!ctx.client || offline()) return { remote: null, remoteState: "offline" };
  const query = ctx.client.from("team_quotes").select(columns)
    .eq("workspace_id", ctx.workspace).eq("id", id);
  if (signal) query.abortSignal(signal);
  const result = await query.maybeSingle().retry(false);
  assertConflictScope(ctx, signal);
  if (offline()) return { remote: null, remoteState: "offline" };
  if (result.error) {
    if ([401, 403].includes(result.status) || result.error.code === "42501") {
      throw new Error("Ortak teklif için erişiminizi doğrulayın.");
    }
    return { remote: null, remoteState: "unavailable" };
  }
  if (!result.data) return { remote: null, remoteState: "missing" };
  const row = result.data as unknown as QuoteRow;
  if (row.id !== id || !Number.isSafeInteger(row.revision) || (row.revision ?? 0) < 1) {
    return { remote: null, remoteState: "unavailable" };
  }
  const [remote] = await invoke<Quote[]>("calculate_team_quotes", { quotes: [unpackQuote(row)] });
  assertConflictScope(ctx, signal);
  if (offline()) return { remote: null, remoteState: "offline" };
  if (!remote || remote.id !== id || remote.revision !== row.revision) {
    return { remote: null, remoteState: "unavailable" };
  }
  return { remote, remoteState: "ready" };
}

async function inspectConflict(id: string, signal?: AbortSignal): Promise<TeamQuoteConflict> {
  const ctx = context();
  assertConflictScope(ctx, signal);
  const operations = (await pending(ctx.scope)).filter((op) => op.contact.id === id);
  assertConflictScope(ctx, signal);
  if (!operations.some((op) => op.conflict)) throw new Error("Bu teklif için bekleyen çakışma kalmadı.");
  const local = operations[operations.length - 1].contact;
  return { scope: ctx.scope, operationIds: operations.map((op) => op.id), local, ...await readConflictRemote(ctx, id, signal) };
}

async function acceptConflict(review: TeamQuoteConflict, signal?: AbortSignal): Promise<Quote> {
  const ctx = context();
  const id = review.local.id;
  if (typeof id !== "string" || !review.remote || review.scope !== ctx.scope) {
    throw new Error("Teklif karşılaştırmasını yeniden açın.");
  }
  if (flushing) await flushing;
  const { remote, remoteState } = await readConflictRemote(ctx, id, signal);
  if (!remote || remote.revision !== review.remote.revision || remote.updated_at !== review.remote.updated_at) {
    const message = remoteState === "ready"
      ? "Ortak teklif inceleme sırasında değişti. Karşılaştırmayı yenileyin."
      : "Güncel ortak teklif doğrulanamadı. Karşılaştırmayı yenileyin.";
    throw new Error(message);
  }
  assertConflictScope(ctx, signal);
  if (teamReachable.value === false) throw new Error("Güncel ortak teklif doğrulanamadı. Karşılaştırmayı yenileyin.");
  const accepted = await cache<boolean>(ctx.scope, {
    action: "resolve_quote_conflict", quote_id: id, expected_ids: review.operationIds, remote,
  });
  if (!accepted) throw new Error("Yerel teklif inceleme sırasında değişti. Karşılaştırmayı yenileyin.");
  const remaining = await pending(ctx.scope).catch(() => null);
  assertConflictScope(ctx, signal);
  if (remaining) reportQueue(remaining);
  return remote;
}

async function overlay(ctx: Context, quotes: Quote[], offline: boolean) {
  const operations = await pending(ctx.scope);
  const rows = new Map(quotes.map((quote) => [quote.id, quote]));
  const trashState = await cache<QuoteTrashMarker[] | null>(ctx.scope, { action: "read", key: trashCacheKey });
  const hidden = new Set((trashState ?? []).filter((marker) => marker.trashed
    && (rows.get(marker.quote_id)?.revision ?? 0) <= marker.revision).map(({ quote_id }) => quote_id));
  for (const quote of await cache<Quote[]>(ctx.scope, { action: "contacts" })) {
    const central = rows.get(quote.id);
    if ((offline || central) && (central?.revision ?? 0) < (quote.revision ?? 0)) rows.set(quote.id, quote);
  }
  for (const op of operations) rows.set(op.contact.id, op.contact);
  const conflicted = new Set(operations.filter((op) => op.conflict).map((op) => op.contact.id));
  for (const id of hidden) if (!conflicted.has(id)) rows.delete(id);
  const canEdit = await cache<boolean>(ctx.scope, { action: "read", key: "can-edit" });
  assertQuoteScope(ctx);
  teamQuotesOffline.value = offline;
  teamQuoteCanEdit.value = !!canEdit;
  reportQueue(operations);
  return sortQuotesByActivity([...rows.values()]);
}

function reportQueue(operations: Operation[]) {
  const conflicts = new Set(operations.filter((op) => op.conflict).map((op) => op.contact.id));
  teamQuoteConflicts.value = conflicts;
  teamQuoteQueue.value = {
    pending: operations.filter((op) => !conflicts.has(op.contact.id)).length,
    conflicts: conflicts.size,
  };
}

export function flushTeamQuotes(): Promise<void> {
  if (flushing) return flushing;
  const ctx = context();
  if (!ctx.client) return Promise.resolve();
  const client = ctx.client;
  flushing = (async () => {
    const operations = await pending(ctx.scope);
    const blocked = new Set(operations.filter((op) => op.conflict).map((op) => op.contact.id));
    for (const op of operations) {
      if (currentScope() !== ctx.scope) return;
      if (blocked.has(op.contact.id)) continue;
      const result = await client.rpc("team_sync_quote", op.payload);
      if (result.error) {
        if (result.error.code !== "40001"
          && (result.status === 0 || result.status >= 500 || result.status === 401 || result.status === 404)) return;
        // A locally created customer may still be waiting in the contact queue.
        if (result.error.code === "23503" && op.contact.contact_id
          && await teamContactProgress(op.contact.contact_id) === "pending") {
          blocked.add(op.contact.id);
          continue;
        }
        await cache(ctx.scope, { action: "conflict", id: op.id, reason: result.error.message });
        blocked.add(op.contact.id);
      } else {
        await cache(ctx.scope, { action: "acknowledge", id: op.id, contact: op.contact });
      }
    }
  })().finally(async () => {
    try {
      const operations = await pending(ctx.scope);
      if (ctx.scope === currentScope()) reportQueue(operations);
    } finally {
      flushing = undefined;
    }
  });
  return flushing;
}

async function list(): Promise<Quote[]> {
  const ctx = context();
  const { workspace, scope, client } = ctx;
  assertQuoteScope(ctx);
  if (client) {
    const trashMarkers = refreshTrashMarkers(ctx).catch(() => {});
    const rows: QuoteRow[] = [];
    const pageSize = 200;
    for (let start = 0; ; start += pageSize) {
      const result = await client.from("team_quotes").select(columns).eq("workspace_id", workspace)
        .order("id").range(start, start + pageSize - 1);
      assertQuoteScope(ctx);
      if (result.error) {
        if (result.status !== 0 && result.status < 500) throw new Error(result.error.message);
        break;
      }
      rows.push(...(result.data as unknown as QuoteRow[]));
      if (result.data.length < pageSize) {
        const documents = rows.map(unpackQuote);
        const quotes = await invoke<Quote[]>("calculate_team_quotes", { quotes: documents });
        assertQuoteScope(ctx);
        await cache(scope, { action: "write", key: "quotes", data: quotes });
        const membership = await client.from("workspace_members").select("role").eq("workspace_id", workspace)
          .eq("user_id", teamUser.value).maybeSingle();
        assertQuoteScope(ctx);
        if (!membership.error) {
          await cache(scope, { action: "write", key: "can-edit", data: ["owner", "editor"].includes(membership.data?.role ?? "") });
        }
        await trashMarkers;
        return overlay(ctx, quotes, false);
      }
    }
  }
  const quotes = await cache<Quote[] | null>(scope, { action: "read", key: "quotes" });
  return overlay(ctx, quotes ?? [], true);
}

const historyDenied = "Bu çalışma alanının teklif geçmişine erişilemiyor. Ekip oturumunuzu kontrol edin.";

function record(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : null;
}

function readVersion(value: unknown): TeamQuoteVersion {
  const row = record(value);
  const snapshot = record(row?.snapshot_json);
  const quote = record(snapshot?.quote);
  if (!row || typeof row.version !== "number" || !Number.isSafeInteger(row.version) || row.version < 1
    || typeof row.at !== "string" || !Number.isFinite(Date.parse(row.at)) || !quote || !Array.isArray(snapshot?.items)) {
    throw new Error("Teklif geçmişindeki bir kayıt okunamadı.");
  }
  const text = (value: unknown) => typeof value === "string" ? value : "";
  const number = (value: unknown) => typeof value === "number" && Number.isFinite(value) ? value : null;
  return {
    version: row.version,
    at: row.at,
    snapshot_json: {
      quote: { no: text(quote.no), currency: text(quote.currency), note: text(quote.note), valid_until: text(quote.valid_until) || null },
      items: snapshot.items.map((entry) => {
        const item = record(entry);
        if (!item) throw new Error("Teklif geçmişindeki bir kalem okunamadı.");
        return { name: text(item.name), qty: number(item.qty), unit_price: number(item.unit_price), tax_rate: number(item.tax_rate) };
      }),
    },
  };
}

async function history(id: string, signal?: AbortSignal): Promise<{ versions: TeamQuoteVersion[]; cached: boolean }> {
  const ctx = context();
  const checkScope = () => {
    if (signal?.aborted || !sameScope(ctx)) {
      throw new DOMException("Teklif geçmişi isteği iptal edildi.", "AbortError");
    }
    if (teamAccessIssue.value) throw new Error(historyDenied);
  };
  checkScope();
  const key = `history:${id}`;
  if (ctx.client) {
    const pageSize = 50;
    const versions: TeamQuoteVersion[] = [];
    let previousVersion = Infinity;
    for (;;) {
      const query = ctx.client.from("team_quote_versions").select("version,at,snapshot_json")
        .eq("workspace_id", ctx.workspace).eq("quote_id", id)
        .order("version", { ascending: false }).limit(pageSize).retry(false);
      if (Number.isFinite(previousVersion)) query.lt("version", previousVersion);
      if (signal) query.abortSignal(signal);
      const result = await query;
      checkScope();
      if (result.error) {
        if ([401, 403].includes(result.status) || result.error.code === "42501") throw new Error(historyDenied);
        if (result.status === 0 || result.status >= 500) break;
        throw new Error("Teklif geçmişi Supabase’den okunamadı. Tekrar deneyin.");
      }
      if (!Array.isArray(result.data)) throw new Error("Teklif geçmişi okunamadı.");
      for (const row of result.data) {
        const version = readVersion(row);
        if (version.version >= previousVersion) throw new Error("Teklif geçmişi sıralaması doğrulanamadı.");
        versions.push(version);
        previousVersion = version.version;
      }
      if (result.data.length < pageSize) {
        // A local cache failure must not hide a complete, authorized server response.
        await cache(ctx.scope, { action: "write", key, data: versions }).catch(() => {});
        checkScope();
        return { versions, cached: false };
      }
    }
  }

  const cached = await cache<TeamQuoteVersion[] | null>(ctx.scope, { action: "read", key })
    .catch(() => { throw new Error("Teklif geçmişinin bu cihazdaki kopyası okunamadı."); });
  checkScope();
  if (!Array.isArray(cached)) throw new Error("Teklif geçmişi bu cihaza henüz indirilmedi. Bağlantı geldiğinde yeniden deneyin.");
  return { versions: cached, cached: true };
}

function lifecycleError(error: { code?: string }, status: number) {
  if (error.code === "PGRST202") return new Error("Teklif düzenleme ve çöp kutusu için Supabase güncellemesi gerekiyor.");
  if (error.code === "PT409" || error.code === "40001") return new Error("Teklif değişmiş. Listeyi yenileyip tekrar deneyin.");
  if (error.code === "42501" || status === 401 || status === 403) return new Error("Bu teklif işlemi için çalışma alanında düzenleme yetkisi gerekiyor.");
  if (status === 0 || status >= 500) return new Error("Supabase bağlantısını kontrol edip teklif listesini yenileyin.");
  return new Error("Teklif işlemi tamamlanamadı. Listeyi yenileyip tekrar deneyin.");
}

async function trashedQuotes(): Promise<TeamTrashedQuote[]> {
  const ctx = context();
  if (!ctx.client) throw new Error("Çöp kutusunu açmak için Supabase bağlantısı gerekiyor.");
  assertQuoteScope(ctx);
  const pageSize = 100;
  const quotes: TeamTrashedQuote[] = [];
  for (let start = 0; ; start += pageSize) {
    const result = await ctx.client.rpc("team_quote_trash", { target_workspace: ctx.workspace })
      .order("deleted_at", { ascending: false }).order("id").range(start, start + pageSize - 1).retry(false);
    assertQuoteScope(ctx);
    if (result.error) throw lifecycleError(result.error, result.status);
    if (!Array.isArray(result.data)) throw new Error("Çöp kutusu okunamadı. Tekrar deneyin.");
    const page = result.data as TeamTrashedQuote[];
    quotes.push(...page);
    if (page.length < pageSize) return quotes;
  }
}

function rememberTrashState(scope: string, markers: QuoteTrashMarker[]): Promise<boolean> {
  const write = trashCacheWrite.catch(() => {}).then(async () => {
    const entries = await cache<QuoteTrashMarker[] | null>(scope, { action: "read", key: trashCacheKey }) ?? [];
    const current = new Map(entries.map((entry) => [entry.quote_id, entry]));
    let changed = false;
    for (const marker of markers) {
      if ((current.get(marker.quote_id)?.revision ?? 0) >= marker.revision) continue;
      current.set(marker.quote_id, marker);
      changed = true;
    }
    if (changed) await cache(scope, { action: "write", key: trashCacheKey, data: [...current.values()] });
    return changed;
  });
  trashCacheWrite = write;
  return write;
}

function readTrashMarker(value: unknown): QuoteTrashMarker | null {
  const row = record(value);
  if (!row || typeof row.quote_id !== "string" || typeof row.revision !== "number"
    || !Number.isSafeInteger(row.revision) || row.revision < 1 || typeof row.trashed !== "boolean") return null;
  return { quote_id: row.quote_id, revision: row.revision, trashed: row.trashed };
}

async function refreshTrashMarkers(ctx: Context) {
  if (!ctx.client) return;
  const pageSize = 200;
  let previousId = "";
  for (;;) {
    const query = ctx.client.from("team_quote_trash_state").select("quote_id,revision,trashed")
      .eq("workspace_id", ctx.workspace).order("quote_id").limit(pageSize).retry(false);
    if (previousId) query.gt("quote_id", previousId);
    const result = await query;
    if (!sameScope(ctx) || teamAccessIssue.value) return;
    // An older backend can lack the visibility table while regular quote reads still work.
    if (result.error || !Array.isArray(result.data)) return;
    const markers = result.data.map(readTrashMarker);
    if (markers.some((marker) => !marker)) return;
    const page = markers.filter((marker): marker is QuoteTrashMarker => marker !== null);
    if (page.length) await rememberTrashState(ctx.scope, page);
    if (page.length < pageSize) return;
    const lastId = page[page.length - 1].quote_id;
    if (lastId <= previousId) return;
    previousId = lastId;
  }
}

async function observeTrash(value: unknown): Promise<QuoteTrashMarker | null> {
  const row = record(value);
  const marker = readTrashMarker(row);
  if (!row || !teamProject.value || !teamUser.value || !teamWorkspace.value || teamAccessIssue.value
    || row.workspace_id !== teamWorkspace.value || !marker) return null;
  const scope = currentScope();
  const accepted = await rememberTrashState(scope, [marker]).catch(() => true);
  return accepted && currentScope() === scope && !teamAccessIssue.value ? marker : null;
}

async function updateLifecycle(quote: Pick<Quote, "id" | "revision">, action: QuoteLifecycleAction) {
  const ctx = context();
  if (!ctx.client) throw new Error("Teklif işlemi için Supabase bağlantısı gerekiyor.");
  if (typeof quote.id !== "string" || quote.revision == null) throw new Error("Güncel teklifi listeden yeniden açın.");
  assertQuoteScope(ctx);
  await flushTeamQuotes();
  const operations = await pending(ctx.scope);
  assertQuoteScope(ctx);
  if (operations.some((op) => op.contact.id === quote.id)) {
    throw new Error("İşleme devam etmeden önce bu teklifin bekleyen değişikliklerini eşitleyin.");
  }
  const result = await ctx.client.rpc("team_quote_lifecycle", {
    target_workspace: ctx.workspace, target_quote: quote.id, expected_revision: quote.revision, action,
  }).retry(false);
  assertQuoteScope(ctx);
  if (result.error) throw lifecycleError(result.error, result.status);
  if (result.data !== quote.revision + 1) throw new Error("Teklif işlemi doğrulanamadı. Listeyi yenileyin.");
  if (action !== "revise") {
    await rememberTrashState(ctx.scope, [{ quote_id: quote.id, revision: result.data, trashed: action === "trash" }])
      .catch(() => {});
  }
}

export function summarizeTeamQuotes(quotes: Quote[]): QuoteSummary {
  const totals = new Map<string, number>();
  const reasons = new Map<string, number>();
  for (const quote of quotes) {
    if (quote.status === "won") totals.set(quote.currency, (totals.get(quote.currency) ?? 0) + quote.grand_total);
    if (quote.status === "lost" && quote.close_reason) {
      reasons.set(quote.close_reason, (reasons.get(quote.close_reason) ?? 0) + 1);
    }
  }
  return {
    open_count: quotes.filter((q) => q.status === "sent").length,
    won_count: quotes.filter((q) => q.status === "won").length,
    lost_count: quotes.filter((q) => q.status === "lost").length,
    won_totals: [...totals].sort(([a], [b]) => a.localeCompare(b)),
    lost_reasons: [...reasons].sort(([a, n], [b, m]) => m - n || a.localeCompare(b, "tr")),
  };
}

export const teamQuotes = {
  list,
  history,
  inspectConflict,
  acceptConflict,
  trash: trashedQuotes,
  observeTrash,
  revise: (quote: Quote) => updateLifecycle(quote, "revise"),
  remove: (quote: Quote) => updateLifecycle(quote, "trash"),
  restore: (quote: TeamTrashedQuote) => updateLifecycle(quote, "restore"),
  async get(id: string) {
    const quote = (await list()).find((item) => item.id === id);
    if (!quote) throw new Error("Teklif bu çalışma alanında bulunamadı.");
    return quote;
  },
  async save(quote: Quote, expectedRevision: number | null) {
    if (teamQuoteSaving.value) throw new Error("Teklif için devam eden kaydın tamamlanmasını bekleyin.");
    teamQuoteSaving.value = true;
    try {
      return await queueQuote(quote, expectedRevision);
    } finally {
      teamQuoteSaving.value = false;
    }
  },
  async transition(quote: Quote, status: string, reason: string) {
    const ctx = context();
    await flushTeamQuotes();
    if (!ctx.client || (await pending(ctx.scope)).some((op) => op.contact.id === quote.id)) {
      throw new Error("Durumu değiştirmek için teklifin Supabase ile eşitlenmesi gerekiyor.");
    }
    const result = await ctx.client.rpc("team_transition_quote", {
      target_workspace: ctx.workspace, target_quote: quote.id, expected_revision: quote.revision,
      next_status: status, reason,
    });
    if (result.error) throw new Error(result.error.message);
  },
};

async function queueQuote(quote: Quote, expectedRevision: number | null) {
    const ctx = context();
    if (typeof quote.id !== "string" || typeof quote.contact_id === "number" || quote.status !== "draft") {
      throw new Error("Yalnızca çalışma alanındaki taslak teklifler düzenlenebilir.");
    }
    if (!(await cache<boolean>(ctx.scope, { action: "read", key: "can-edit" }))) {
      throw new Error("Teklif düzenleme yetkinizi bağlantı varken doğrulayın.");
    }
    if (quote.items.length > 500 || quote.items.some((item) =>
      !item.name.trim() || !Number.isFinite(item.qty) || item.qty <= 0
      || !Number.isFinite(item.unit_price) || item.unit_price < 0
      || !Number.isFinite(item.tax_rate) || item.tax_rate < 0 || item.tax_rate > 100,
    )) throw new Error("Teklif kalemlerini kontrol edin. Adet pozitif, fiyat sıfır veya üstü ve KDV %0–100 olmalı. En fazla 500 kalem eklenebilir.");
    const prior = (await pending(ctx.scope)).filter((op) => op.contact.id === quote.id);
    const previous = prior[prior.length - 1];
    if (prior.some((op) => op.conflict) || (previous && previous.contact.revision !== expectedRevision)) {
      throw new Error("Teklifin daha yeni bir yerel sürümü var. Teklifi yeniden açın.");
  }
  const id = crypto.randomUUID();
  const [saved] = await invoke<Quote[]>("calculate_team_quotes", {
    quotes: [{ ...quote, updated_at: new Date().toISOString(), revision: (expectedRevision ?? 0) + 1 }],
  });
  const { no, contact_id, currency, fx_rate, fx_date, valid_until, note } = saved;
    const items = saved.items.map(({ id, sku, name, qty, unit_price, tax_rate, cost }) =>
      ({ id, sku, name, qty, unit_price, tax_rate, cost }),
    );
    await cache(ctx.scope, { action: "queue", id, contact: saved, payload: {
      target_workspace: ctx.workspace, operation_id: id, target_quote: saved.id,
      expected_revision: expectedRevision, fields: { no, contact_id, currency, fx_rate, fx_date, valid_until, note }, items,
    } });
    if (ctx.scope === currentScope()) await flushTeamQuotes();
    return saved;
}
