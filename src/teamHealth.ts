import { teamAccessIssue, teamClient, teamReachable, teamSession, teamUser, teamWorkspace } from "./teamConnection";
import { sharedStoreInventoryStatus } from "./teamStorePages";
import type { HealthCheck } from "./types";

const feed = { key: "feed", label: "Ortak ürün kataloğu" };
const gsc = { key: "gsc", label: "Ortak GSC analizi" };
const inventory = { key: "inventory", label: "Ortak sayfa envanteri" };
const checks = [feed, gsc, inventory];
const shortDate = new Intl.DateTimeFormat("tr-TR", { dateStyle: "short", timeStyle: "short" });
const day = (date: Date) => Date.UTC(date.getFullYear(), date.getMonth(), date.getDate());

function row(check: typeof feed, state: "ok" | "stale" | "error" | "off", detail: string, action = ""): HealthCheck {
  return { ...check, state, detail, action };
}

function available<T extends { error: unknown }>(result: PromiseSettledResult<T>): T | undefined {
  return result.status === "fulfilled" && !result.value.error ? result.value : undefined;
}

function unverified(check: typeof feed): HealthCheck {
  return row(check, "error", "Supabase durumu doğrulanamadı");
}

function dated(check: typeof feed, at: string, staleAfter: number, action: string): HealthCheck {
  const date = new Date(at);
  if (!Number.isFinite(date.valueOf())) return unverified(check);
  const days = (day(new Date()) - day(date)) / 86_400_000;
  return row(check, days > staleAfter ? "stale" : "ok", shortDate.format(date), action);
}

export async function readTeamDataHealth(): Promise<HealthCheck[]> {
  const client = teamClient.value;
  const workspace = teamWorkspace.value;
  const userId = teamUser.value;
  const current = () => client === teamClient.value && workspace === teamWorkspace.value && userId === teamUser.value && !teamAccessIssue.value;
  const queued = workspace && userId && !teamAccessIssue.value
    ? sharedStoreInventoryStatus().then((status) => status?.queued).catch(() => undefined)
    : Promise.resolve(undefined);

  function inventoryQueue(operation: Awaited<typeof queued>, action: string) {
    if (operation?.rejected) return row(inventory, "error", "Yeni envanter Supabase tarafından reddedildi", action);
    if (operation) return row(inventory, "stale", "Supabase’e eşitlenmesi bekleniyor");
  }

  if (!client || !teamSession.value || !workspace || !userId || teamReachable.value !== true || !current()) {
    const operation = await queued;
    const detail = teamAccessIssue.value ? "Çalışma alanına erişim doğrulanamadı" : "Bağlantı doğrulanınca denetlenecek";
    return [
      row(feed, "off", detail),
      row(gsc, "off", detail),
      (current() ? inventoryQueue(operation, "") : undefined) ?? row(inventory, "off", detail),
    ];
  }

  const [feedResult, gscResult, inventoryResult, memberResult, queueResult] = await Promise.allSettled([
    client.from("team_catalog_state").select("updated_at").eq("workspace_id", workspace).maybeSingle().retry(false),
    client.from("team_gsc_reports").select("analyzed_at").eq("workspace_id", workspace).maybeSingle().retry(false),
    client.from("team_store_pages").select("remote_id", { count: "exact", head: true }).eq("workspace_id", workspace).retry(false),
    client.from("workspace_members").select("role").eq("workspace_id", workspace).eq("user_id", userId).maybeSingle().retry(false),
    queued,
  ]);
  if (!current() || teamReachable.value !== true) {
    return checks.map((check) => row(check, "off", "Bağlantı doğrulanınca denetlenecek"));
  }

  const role = available(memberResult)?.data?.role;
  const canEdit = role === "owner" || role === "editor";
  const inventoryAction = canEdit ? "sync_store_pages" : "";
  const analysisAction = canEdit ? "run_analysis" : "";
  const catalog = available(feedResult);
  const analysis = available(gscResult);
  const pageCount = available(inventoryResult)?.count;
  const pending = queueResult.status === "fulfilled" ? inventoryQueue(queueResult.value, inventoryAction) : undefined;
  return [
    !catalog ? unverified(feed) : catalog.data
      ? dated(feed, catalog.data.updated_at, 3, "sync_feed")
      : row(feed, "stale", "Ortak katalog henüz yayımlanmadı"),
    !analysis ? unverified(gsc) : analysis.data
      ? dated(gsc, analysis.data.analyzed_at, 7, analysisAction)
      : row(gsc, "off", "Henüz ortak analiz kaydı yok", analysisAction),
    pending ?? (pageCount == null ? unverified(inventory) : pageCount > 0
      ? row(inventory, "ok", `${pageCount.toLocaleString("tr-TR")} ortak kayıt`, inventoryAction)
      : row(inventory, "off", "Henüz ortak sayfa kaydı yok", inventoryAction)),
  ];
}
