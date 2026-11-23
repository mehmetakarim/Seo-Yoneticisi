import { invoke } from "@tauri-apps/api/core";
import { shallowRef } from "vue";
import { teamClient, teamProject, teamReachable, teamSession, teamUser, teamWorkspace } from "./teamConnection";
import { teamContactProgress } from "./teamContacts";
import type { Quote, QuoteItem, QuoteSummary } from "./types";

const columns = "id,no,contact_id,status,currency,fx_rate,fx_date,valid_until,note,close_reason,created_at,sent_at,revision,updated_at,team_contacts(name),team_quote_items(id,sku,name,qty,unit_price,tax_rate,cost,sort),team_quote_versions(count)";
type QuoteRow = Omit<Quote, "contact_name" | "items" | "version_count"> & {
  updated_at: string;
  team_contacts: { name: string } | null;
  team_quote_items: (QuoteItem & { sort: number })[];
  team_quote_versions: { count: number }[];
};

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
const pending = (scope: string) => cache<Operation[]>(scope, { action: "pending" });
let flushing: Promise<void> | undefined;

export function resetTeamQuoteIndicators() {
  teamQuotesOffline.value = false;
  teamQuoteQueue.value = { pending: 0, conflicts: 0 };
  teamQuoteConflicts.value = new Set();
  teamQuoteCanEdit.value = false;
}

async function overlay(ctx: Context, quotes: Quote[], offline: boolean) {
  const operations = await pending(ctx.scope);
  const rows = new Map(quotes.map((quote) => [quote.id, quote]));
  for (const quote of await cache<Quote[]>(ctx.scope, { action: "contacts" })) {
    const central = rows.get(quote.id);
    if ((offline || central) && (central?.revision ?? 0) < (quote.revision ?? 0)) rows.set(quote.id, quote);
  }
  for (const op of operations) rows.set(op.contact.id, op.contact);
  const canEdit = await cache<boolean>(ctx.scope, { action: "read", key: "can-edit" });
  if (ctx.scope === currentScope()) {
    teamQuotesOffline.value = offline;
    teamQuoteCanEdit.value = !!canEdit;
    reportQueue(operations);
  }
  return [...rows.values()];
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
    const blocked = new Set<string | number>();
    for (const op of await pending(ctx.scope)) {
      if (currentScope() !== ctx.scope) return;
      if (op.conflict) blocked.add(op.contact.id);
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
  if (client) {
    const rows: QuoteRow[] = [];
    const pageSize = 200;
    for (let start = 0; ; start += pageSize) {
      const result = await client.from("team_quotes").select(columns).eq("workspace_id", workspace)
        .order("id").range(start, start + pageSize - 1);
      if (result.error) {
        if (result.status !== 0 && result.status < 500) throw new Error(result.error.message);
        break;
      }
      rows.push(...(result.data as unknown as QuoteRow[]));
      if (result.data.length < pageSize) {
        const priority = (status: string) => status === "draft" ? 0 : status === "sent" ? 1 : 2;
        const documents = rows.sort((a, b) =>
          priority(a.status) - priority(b.status) || b.updated_at.localeCompare(a.updated_at),
        ).map(({ team_contacts, team_quote_items, team_quote_versions, ...quote }) => ({
          ...quote,
          contact_name: team_contacts?.name ?? "",
          items: team_quote_items.sort((a, b) => a.sort - b.sort || String(a.id).localeCompare(String(b.id))),
          version_count: team_quote_versions[0]?.count ?? 0,
        }));
        const quotes = await invoke<Quote[]>("calculate_team_quotes", { quotes: documents });
        await cache(scope, { action: "write", key: "quotes", data: quotes });
        const membership = await client.from("workspace_members").select("role").eq("workspace_id", workspace)
          .eq("user_id", teamUser.value).maybeSingle();
        if (!membership.error && ctx.scope === currentScope()) {
          await cache(scope, { action: "write", key: "can-edit", data: ["owner", "editor"].includes(membership.data?.role ?? "") });
        }
        return overlay(ctx, quotes, false);
      }
    }
  }
  const quotes = await cache<Quote[] | null>(scope, { action: "read", key: "quotes" });
  return overlay(ctx, quotes ?? [], true);
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
      quotes: [{ ...quote, revision: (expectedRevision ?? 0) + 1 }],
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
