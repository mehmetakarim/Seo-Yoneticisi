import { invoke } from "@tauri-apps/api/core";
import { api } from "./api";
import { teamAccessIssue, teamClient, teamProject, teamReachable, teamRestoring, teamSession, teamUser, teamWorkspace } from "./teamConnection";
import type { QueryComparison } from "./types";
import { refreshTeamCatalog } from "./teamCatalog";
import { listTeamProductSeo } from "./teamProductSeo";

type Measurement = { workspace_id: string; sku: string; query: string; event_day: string; measurement: QueryComparison };
type Pending = { id: string; payload: Measurement; conflict: string | null };
const scope = () => JSON.stringify([teamProject.value, teamUser.value, teamWorkspace.value, "gsc"]);
const online = () => teamSession.value && (teamReachable.value ?? navigator.onLine) ? teamClient.value : null;
const cache = <T>(key: string, request: object) => invoke<T>("team_cache", { scope: key, request });
let flushing: Promise<void> | undefined;
type OutcomeEvent = { id: number; sku: string | null; at: string; url: string | null; reaches_store: boolean; kind: string };
type OutcomeInputs = { snapshots: { window_end: string }[]; events: OutcomeEvent[]; page_rows: unknown[] };
let outcomeRequest: { scope: string; task: Promise<OutcomeInputs | null> } | undefined;
let flushingSnapshots: Promise<void> | undefined;
type QueuedSnapshot = { id: number; snapshot: object; page_rows: object[] };

type AnalysisMode = "pages" | "previous" | "queries" | "snapshot";
type EdgePage = { configured: boolean; rows?: object[]; next?: number | null };

function teamGscReader() {
  if (teamRestoring.value) throw new Error("Ekip oturumu açılıyor. Biraz sonra tekrar deneyin.");
  if (teamAccessIssue.value) throw new Error("Bu çalışma alanının GSC verilerine erişilemiyor.");
  const key = teamMeasurementScope();
  const workspace = teamWorkspace.value;
  if (!key || !workspace) throw new Error("GSC verilerini almak için ekip oturumunu ve çalışma alanını kontrol edin.");
  const remote = online();
  if (!remote) throw new Error("Yeni GSC verisi almak için Supabase bağlantısı gerekli.");
  const fetchRows = async (mode: AnalysisMode, window?: [string, string]) => {
    const all: object[] = [];
    let offset = 0;
    for (;;) {
      if (teamMeasurementScope() !== key) throw new Error("Çalışma alanı değişti.");
      const result = await remote.functions.invoke<EdgePage>("team-gsc", {
        body: { workspace, mode, offset, ...(window ? { start: window[0], end: window[1] } : {}) },
      });
      if (teamMeasurementScope() !== key) throw new Error("Çalışma alanı değişti.");
      if (result.error) {
        const response = result.error.context;
        if (response instanceof Response) {
          if (response.status === 401) throw new Error("GSC verilerini almak için ekip oturumunu yeniden açın.");
          if (response.status === 403) throw new Error("Bu çalışma alanında yeni GSC verisi alma yetkiniz yok.");
          if (response.status === 404) throw new Error("Ortak GSC işlevi Supabase projesinde bulunamadı.");
        }
        throw new Error("Google Search Console’dan ortak veri alınamadı.");
      }
      if (result.data?.configured === false) throw new Error("Bu çalışma alanı için ortak Google Search Console bağlantısı henüz kurulmamış.");
      if (result.data?.configured !== true || !Array.isArray(result.data.rows)) throw new Error("GSC analiz satırları okunamadı.");
      all.push(...result.data.rows);
      const next = result.data.next;
      if (next == null) return all;
      if (next !== offset + 10000 || next >= (mode === "queries" ? 60000 : 25000)) throw new Error("GSC sayfalama bilgisi geçersiz.");
      offset = next;
    }
  };
  return { remote, key, workspace, fetchRows };
}

export async function teamGscAnalysis(sharedWindowEnd?: string) {
  const { key, fetchRows } = teamGscReader();
  const pages = await fetchRows("pages");
  const window = await api.gscSnapshotWindow(key, sharedWindowEnd);
  const [queries, previous, snapshot, catalog, seo] = await Promise.allSettled([
    fetchRows("queries"), fetchRows("previous"), window ? fetchRows("snapshot", window) : null,
    refreshTeamCatalog(), listTeamProductSeo(),
  ]);
  if (teamMeasurementScope() !== key) throw new Error("Çalışma alanı değişti.");
  if (queries.status === "rejected") throw queries.reason;
  if (previous.status === "rejected") throw previous.reason;
  if (catalog.status !== "fulfilled" || !catalog.value) throw new Error("GSC analizi için ortak katalog okunamadı.");
  if (seo.status === "rejected") throw seo.reason;
  const drafts = seo.value;
  const products = catalog.value.products.flatMap(({ sku, name, facts }) => facts.url ? [[sku, name, facts.url,
    facts.category ?? "", facts.brand ?? "", drafts.get(sku)?.draft.meta_status === "done" ? "done" : "pending",
    drafts.get(sku)?.draft.details_status === "done" ? "done" : "pending"]] : []);
  return {
    pages, products, queries: queries.value, previous: previous.value,
    snapshot: window && snapshot.status === "fulfilled" && snapshot.value
      ? { start: window[0], end: window[1], rows: snapshot.value } : null,
  };
}

export async function seedTeamGscHistory() {
  const { remote, key, workspace, fetchRows } = teamGscReader();
  await flushTeamGscSnapshots();
  if (teamMeasurementScope() !== key) throw new Error("Çalışma alanı değişti.");
  const existing = await remote.from("team_gsc_snapshots").select("window_start,window_end")
    .eq("workspace_id", workspace).order("window_end", { ascending: false }).limit(48).retry(false);
  if (teamMeasurementScope() !== key) throw new Error("Çalışma alanı değişti.");
  if (existing.error) {
    if (existing.status === 401 || existing.status === 403) throw new Error("Ortak GSC geçmişine erişilemiyor.");
    throw new Error("Ortak GSC geçmişi Supabase'den okunamadı.");
  }
  const snapshots = existing.data ?? [];
  const windows = await invoke<[string, string][]>("team_gsc_seed_windows", { sharedWindowEnd: snapshots[0]?.window_end });
  let staged = 0;
  for (const [start, end] of windows) {
    if (teamMeasurementScope() !== key) throw new Error("Çalışma alanı değişti.");
    if (snapshots.some((snapshot) => snapshot.window_start <= end && snapshot.window_end >= start)) continue;
    const rows = await fetchRows("snapshot", [start, end]);
    if (await invoke<boolean>("queue_team_gsc_snapshot", { scope: key, snapshot: { start, end, rows } })) ++staged;
    await flushTeamGscSnapshots();
  }
  return staged;
}

export function teamMeasurementScope() {
  return teamUser.value && teamWorkspace.value && !teamAccessIssue.value ? scope() : undefined;
}

export function teamGscMode() {
  return Boolean(teamProject.value || teamUser.value || teamWorkspace.value || teamRestoring.value);
}

export function flushTeamGscSnapshots(): Promise<void> {
  if (flushingSnapshots) return flushingSnapshots;
  const client = online();
  const key = teamMeasurementScope();
  const workspace = teamWorkspace.value;
  if (!client || !key || !workspace) return Promise.resolve();
  flushingSnapshots = (async () => {
    let updated = false;
    while (teamMeasurementScope() === key) {
      const next = await invoke<QueuedSnapshot | null>("team_metric_outbox", { scope: key });
      if (!next || teamMeasurementScope() !== key) break;
      const result = await client.rpc("team_sync_gsc_snapshot", {
        target_workspace: workspace, snapshot: next.snapshot, page_rows: next.page_rows,
      }).retry(false);
      if (result.error || typeof result.data !== "boolean") break;
      await invoke("team_metric_outbox", { scope: key, acknowledge: next.id });
      updated = true;
    }
    if (updated) {
      const existing = await cache<{ data: OutcomeInputs; checked: number } | null>(key, { action: "read", key: "outcomes" });
      if (existing) await cache(key, { action: "write", key: "outcomes", data: { ...existing, checked: 0 } });
    }
  })().finally(() => { flushingSnapshots = undefined; });
  return flushingSnapshots;
}

export function teamOutcomeInputs(): Promise<OutcomeInputs | null> {
  const workspace = teamWorkspace.value;
  if (!workspace || !teamUser.value || teamAccessIssue.value) return Promise.resolve(null);
  const key = scope();
  if (outcomeRequest?.scope === key) return outcomeRequest.task;
  const client = online();
  const task = (async () => {
    const cached = await cache<{ data: OutcomeInputs; checked: number } | null>(key, { action: "read", key: "outcomes" });
    if (key !== scope() || teamAccessIssue.value) return null;
    if (!client || (cached && Date.now() - cached.checked < 30000)) return cached?.data ?? null;
    const result = await client.rpc("team_gsc_outcome_inputs", { target_workspace: workspace }).retry(false);
    if (key !== scope() || teamAccessIssue.value) return null;
    if (result.error) return result.status === 0 || result.status >= 500 ? cached?.data ?? null : null;
    if (!result.data) return null;
    const fresh = result.data as OutcomeInputs;
    await cache(key, { action: "write", key: "outcomes", data: { data: fresh, checked: Date.now() } });
    return key === scope() && !teamAccessIssue.value ? fresh : null;
  })().finally(() => {
    if (outcomeRequest?.task === task) outcomeRequest = undefined;
  });
  outcomeRequest = { scope: key, task };
  return task;
}

export function flushTeamGsc(): Promise<void> {
  if (flushing) return flushing;
  const client = online();
  if (!client || !teamUser.value || !teamWorkspace.value || teamAccessIssue.value) return Promise.resolve();
  const key = scope();
  flushing = (async () => {
    for (const op of await cache<Pending[]>(key, { action: "pending" })) {
      if (teamMeasurementScope() !== key) return;
      if (op.conflict) continue;
      const response = await client.from("team_gsc_comparisons").upsert(op.payload, {
        onConflict: "workspace_id,sku,query,event_day", ignoreDuplicates: true,
      }).retry(false);
      if (response.error) {
        if (["22023", "23503", "23514", "42501"].includes(response.error.code)) {
          await cache(key, { action: "conflict", id: op.id, reason: "GSC karşılaştırması ortak geçmişe kaydedilemedi." });
          continue;
        }
        return;
      }
      await cache(key, { action: "acknowledge", id: op.id, contact: { id: op.id } });
    }
  })().finally(() => { flushing = undefined; });
  return flushing;
}

export async function compareTeamProductQuery(sku: string, query: string): Promise<QueryComparison> {
  if (!teamGscMode()) return api.compareProductQuery(sku, query);
  const workspace = teamWorkspace.value;
  const key = teamMeasurementScope();
  if (!workspace || !key) throw new Error("Ortak GSC ölçümü için ekip oturumunu ve çalışma alanını kontrol edin.");
  const checkScope = () => {
    if (key !== teamMeasurementScope()) throw new Error("Çalışma alanı değişti veya ölçüm erişimi kapandı.");
  };
  const slot = JSON.stringify([sku, query]);
  const client = online();
  let cached = await cache<QueryComparison | null>(key, { action: "read", key: slot });
  for (const { payload, conflict } of await cache<Pending[]>(key, { action: "pending" })) {
    if (!conflict && payload.sku === sku && payload.query === query && (!cached || payload.event_day > cached.event_day)) {
      cached = payload.measurement;
    }
  }
  checkScope();
  if (!client) {
    if (cached) return cached;
    throw new Error("Bu çalışma alanında karşılaştırma henüz indirilmedi. Yeni ölçüm için Supabase bağlantısı gerekli.");
  }

  const [event, member] = await Promise.all([
    client.from("team_seo_work_events").select("occurred_at,url")
      .eq("workspace_id", workspace).eq("sku", sku).eq("reaches_store", true)
      .order("occurred_at", { ascending: false }).order("event_key", { ascending: false }).limit(1).maybeSingle().retry(false),
    client.from("workspace_members").select("role").eq("workspace_id", workspace)
      .eq("user_id", teamUser.value).maybeSingle().retry(false),
  ]);
  checkScope();
  if ([event, member].some(({ status }) => status === 401 || status === 403) || (!member.error && !member.data)) {
    throw new Error("Bu çalışma alanının ortak GSC ölçümlerine erişilemiyor.");
  }
  if (event.error || member.error) {
    if (cached && [event, member].every(({ error, status }) => !error || status === 0 || status >= 500)) return cached;
    throw new Error("Ortak mağaza işlem geçmişi okunamadı.");
  }
  if (!event.data) throw new Error("Bu ürün için ortak mağaza işlem kaydı yok.");
  const eventDay = String(event.data.occurred_at).slice(0, 10);
  const page = event.data.url as string | null;
  if (!page) throw new Error("Ortak işlem kaydının sayfa adresi yok.");

  const stored = await client.from("team_gsc_comparisons").select("measurement")
    .eq("workspace_id", workspace).eq("sku", sku).eq("query", query).eq("event_day", eventDay)
    .maybeSingle().retry(false);
  checkScope();
  if (stored.status === 401 || stored.status === 403) throw new Error("Ortak GSC karşılaştırmasına erişilemiyor.");
  if (stored.data) {
    const measurement = stored.data.measurement as QueryComparison;
    await cache(key, { action: "write", key: slot, data: measurement });
    checkScope();
    return measurement;
  }
  if (cached?.event_day === eventDay) return cached;
  if (stored.error) throw new Error("Ortak GSC karşılaştırması Supabase’den okunamadı.");

  const response = await client.functions.invoke<{ configured: boolean; pending?: boolean; rows?: object[] }>("team-gsc", {
    body: { workspace, page, query, event_day: eventDay },
  });
  checkScope();
  if (response.error) {
    const status = response.error.context instanceof Response ? response.error.context.status : 0;
    if (status === 401 || status === 403) throw new Error("Ortak GSC ölçümü için ekip oturumunu ve yetkinizi kontrol edin.");
    if (status === 404) throw new Error("Ortak GSC işlevi Supabase projesinde bulunamadı.");
    throw new Error("Ortak GSC bağlantısı ölçümü tamamlayamadı.");
  }
  if (response.data?.configured === false) throw new Error("Bu çalışma alanının ortak Google Search Console bağlantısı henüz kurulmamış.");
  if (response.data?.pending) return { event_day: eventDay, before: null, after: null };
  if (response.data?.configured !== true || !Array.isArray(response.data.rows)) throw new Error("Ortak GSC ölçümü okunamadı.");
  const measurement = await api.summarizeTeamQuery(eventDay, response.data.rows);
  checkScope();
  if (!measurement.before || !measurement.after || !(measurement.before.days || measurement.after.days)) return measurement;
  if (member.data?.role === "owner" || member.data?.role === "editor") {
    const id = crypto.randomUUID();
    await cache(key, { action: "queue", id, contact: { id }, payload: {
      workspace_id: workspace, sku, query, event_day: eventDay, measurement,
    } });
    void flushTeamGsc().catch(console.warn);
  }
  await cache(key, { action: "write", key: slot, data: measurement });
  checkScope();
  return measurement;
}
