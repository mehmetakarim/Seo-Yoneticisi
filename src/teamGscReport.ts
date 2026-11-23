import { invoke } from "@tauri-apps/api/core";
import { api } from "./api";
import { teamAccessIssue, teamClient, teamReachable, teamSession, teamWorkspace } from "./teamConnection";
import { teamGscMode, teamMeasurementScope } from "./teamGsc";
import type { OpportunityReport } from "./types";

type ReportOperation = { id: string; payload: { target_workspace: string; analysis: OpportunityReport }; conflict: string | null };
const scope = () => {
  const key = teamMeasurementScope();
  return key ? `${key}:report` : undefined;
};
const cache = <T>(key: string, request: object) => invoke<T>("team_cache", { scope: key, request });
const pending = (key: string) => cache<ReportOperation[]>(key, { action: "pending" });
const online = () => teamSession.value && (teamReachable.value ?? navigator.onLine) ? teamClient.value : null;
let flushing: Promise<void> | undefined;

export async function loadGscReport(): Promise<OpportunityReport | null> {
  if (!teamGscMode()) return api.getOpportunityCache();
  const key = scope();
  const workspace = teamWorkspace.value;
  if (!key || !workspace || teamAccessIssue.value) return null;
  let report = await cache<OpportunityReport | null>(key, { action: "read", key: "latest" });
  if (scope() !== key) return null;
  const remote = online();
  if (remote) {
    const result = await remote.from("team_gsc_reports").select("report").eq("workspace_id", workspace).maybeSingle().retry(false);
    if (scope() !== key) return null;
    if (!result.error) {
      report = result.data?.report as OpportunityReport ?? null;
      await cache(key, { action: "write", key: "latest", data: report });
    } else if ([401, 403].includes(result.status)) return null;
  }
  for (const { payload, conflict } of await pending(key)) {
    if (conflict) continue;
    if (!report || payload.analysis.analyzed_at > report.analyzed_at) report = payload.analysis;
  }
  return scope() === key ? report : null;
}

export async function queueGscReport(report: OpportunityReport, originalScope: string) {
  const key = scope();
  if (!key || `${originalScope}:report` !== key) throw new Error("Çalışma alanı değişti.");
  const id = crypto.randomUUID();
  await cache(key, { action: "queue", id, contact: { id }, payload: { target_workspace: teamWorkspace.value, analysis: report } });
}

export function flushGscReports(): Promise<void> {
  if (flushing) return flushing;
  const key = scope();
  const remote = online();
  if (!key || !remote) return Promise.resolve();
  flushing = (async () => {
    for (const op of await pending(key)) {
      if (scope() !== key) return;
      if (op.conflict) continue;
      const result = await remote.rpc("team_save_gsc_report", op.payload).retry(false);
      if (result.error || typeof result.data !== "boolean") {
        if ([400, 403, 413].includes(result.status)) {
          await cache(key, { action: "conflict", id: op.id, reason: "Rapor Supabase tarafından kabul edilmedi." });
          continue;
        }
        return;
      }
      if (result.data) {
        await cache(key, { action: "write", key: "latest", data: op.payload.analysis });
      }
      await cache(key, { action: "acknowledge", id: op.id, contact: { id: op.id } });
    }
  })().finally(() => { flushing = undefined; });
  return flushing;
}
