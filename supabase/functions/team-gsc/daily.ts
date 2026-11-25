const dayMs = 86_400_000;
const refreshDays = 14;
const historyWindowDays = 540;
const batchSize = 6;
const maxTargets = 40;
const refreshIntervalMs = 20 * 60 * 60 * 1000;
const googleOrigin = "https://www.googleapis.com";
const pacificDate = new Intl.DateTimeFormat("en-CA", {
  timeZone: "America/Los_Angeles", year: "numeric", month: "2-digit", day: "2-digit",
});
const uuid = /^[a-f\d]{8}-[a-f\d]{4}-[a-f\d]{4}-[a-f\d]{4}-[a-f\d]{12}$/i;

type DayState = "final" | "provisional" | "missing" | "pending";
type Metrics = { clicks: number; impressions: number; position: number };
type DailyRow = { day: string; state: DayState; clicks: number | null; impressions: number | null; position: number | null };
type Target = { id: string; page: string; query: string };
type DailyContext = {
  supabase: string;
  workspace: string;
  property: string;
  serviceKey: string | undefined;
  request: typeof fetch;
  googleToken: () => Promise<string>;
  signal?: AbortSignal;
};

export class DailyError extends Error {
  code: string;
  status: number;

  constructor(code: string, status: number) {
    super(code);
    this.code = code;
    this.status = status;
  }
}

export const isoDay = (day: number) => new Date(day).toISOString().slice(0, 10);
export function parseDay(date: unknown): number {
  if (typeof date !== "string" || !/^\d{4}-\d{2}-\d{2}$/.test(date)) return Number.NaN;
  const day = Date.parse(`${date}T00:00:00Z`);
  return Number.isFinite(day) && isoDay(day) === date ? day : Number.NaN;
}

function daysBetween(start: number, end: number) {
  return Array.from({ length: Math.max(0, (end - start) / dayMs + 1) }, (_, index) => isoDay(start + index * dayMs));
}

function yesterdayPacific(now: number) {
  const parts = Object.fromEntries(pacificDate.formatToParts(now).map(({ type, value }) => [type, value]));
  return parseDay(`${parts.year}-${parts.month}-${parts.day}`) - dayMs;
}

function record(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : {};
}

function metrics(value: Record<string, unknown>): Metrics | null {
  const { clicks, impressions, position } = value;
  if (typeof clicks !== "number" || !Number.isSafeInteger(clicks) || clicks < 0 ||
      typeof impressions !== "number" || !Number.isSafeInteger(impressions) || impressions < clicks ||
      typeof position !== "number" || !Number.isFinite(position) || position < 0) return null;
  return { clicks, impressions, position };
}

function unmeasured(day: string, state: "missing" | "pending"): DailyRow {
  return { day, state, clicks: null, impressions: null, position: null };
}

function timeout(signal?: AbortSignal) {
  const timeout = AbortSignal.timeout(12_000);
  return signal ? AbortSignal.any([signal, timeout]) : timeout;
}

function serviceHeaders(ctx: DailyContext) {
  if (!ctx.serviceKey) throw new DailyError("service_unavailable", 503);
  return { apikey: ctx.serviceKey, Authorization: `Bearer ${ctx.serviceKey}` };
}

async function dbRequest(ctx: DailyContext, path: string, headers: Record<string, string>, args?: object) {
  const response = await ctx.request(`${ctx.supabase}/rest/v1/${path}`, {
    headers: args ? { ...headers, "Content-Type": "application/json" } : headers,
    method: args ? "POST" : "GET", ...(args ? { body: JSON.stringify(args) } : {}),
    redirect: "error", signal: timeout(ctx.signal),
  });
  if (response.ok) return response.json() as Promise<unknown>;
  const { code, message } = record(await response.json().catch(() => null));
  if (response.status === 404 && ["PGRST202", "PGRST205", "42P01", "42883"].includes(String(code))) {
    throw new DailyError("daily_schema_unavailable", 404);
  }
  if (path.startsWith("rpc/team_gsc_register_daily_target") &&
      (code === "PT409" || code === "PT422" || message === "daily_track_limit")) {
    throw new DailyError("daily_track_limit", 409);
  }
  if (response.status === 401 || response.status === 403) throw new DailyError("workspace_access_denied", 403);
  throw new DailyError("daily_storage_unavailable", 503);
}

async function lookupTarget(ctx: DailyContext, page: string, query: string, headers: Record<string, string>) {
  const params = new URLSearchParams({
    select: "id,slot", workspace_id: `eq.${ctx.workspace}`, property: `eq.${ctx.property}`,
    page: `eq.${page}`, query: `eq.${query}`, search_type: "eq.web", limit: "1",
  });
  const result = await dbRequest(ctx, `team_gsc_daily_targets?${params}`, headers);
  if (!Array.isArray(result)) throw new DailyError("daily_storage_unavailable", 503);
  if (result.length === 0) return null;
  const { id, slot } = record(result[0]);
  if (typeof id !== "string" || !uuid.test(id) || (slot !== null &&
      (typeof slot !== "number" || !Number.isInteger(slot) || slot < 1 || slot > maxTargets))) {
    throw new DailyError("daily_storage_unavailable", 503);
  }
  return { id, active: slot !== null };
}

async function savedRows(ctx: DailyContext, targetId: string, start: number, end: number, headers: Record<string, string>) {
  const params = new URLSearchParams({ select: "day,state,clicks,impressions,position", workspace_id: `eq.${ctx.workspace}`,
    target_id: `eq.${targetId}`, day: `gte.${isoDay(start)}` });
  params.append("day", `lte.${isoDay(end)}`);
  const result = await dbRequest(ctx, `team_gsc_daily_measurements?${params}`, headers);
  if (!Array.isArray(result)) throw new DailyError("daily_storage_unavailable", 503);
  const rows = new Map<string, DailyRow>();
  for (const raw of result) {
    const value = record(raw);
    const day = parseDay(value.day);
    const date = String(value.day);
    if (!Number.isFinite(day) || day < start || day > end) continue;
    if (value.state === "missing" || value.state === "pending") {
      rows.set(date, unmeasured(date, value.state));
    } else if (value.state === "final" || value.state === "provisional") {
      const measured = metrics(value);
      if (measured) rows.set(date, { ...measured, day: date, state: value.state });
    }
  }
  return rows;
}

async function persistRows(ctx: DailyContext, targetId: string, rows: DailyRow[], collectedAt: string, markRefreshed: boolean) {
  await dbRequest(ctx, "rpc/team_gsc_store_daily", serviceHeaders(ctx), {
    target_workspace: ctx.workspace, target_id: targetId, samples: rows,
    collected_at: collectedAt, mark_refreshed: markRefreshed,
  });
}

async function googleDays(ctx: DailyContext, token: string, start: number, end: number, target?: Pick<Target, "page" | "query">) {
  const url = `${googleOrigin}/webmasters/v3/sites/${encodeURIComponent(ctx.property)}/searchAnalytics/query`;
  const response = await ctx.request(url, {
    method: "POST", redirect: "error", signal: timeout(ctx.signal),
    headers: { Authorization: `Bearer ${token}`, "Content-Type": "application/json" },
    body: JSON.stringify({
      startDate: isoDay(start), endDate: isoDay(end), dimensions: ["date"], type: "web",
      rowLimit: (end - start) / dayMs + 1, dataState: target ? "all" : "final",
      ...(target ? { dimensionFilterGroups: [{ filters: [
        { dimension: "page", operator: "equals", expression: target.page },
        { dimension: "query", operator: "equals", expression: target.query },
      ] }] } : {}),
    }),
  });
  if (!response.ok) throw new DailyError("google_measurement_unavailable", 502);
  const payload = await response.json().catch(() => null);
  if (!payload || typeof payload !== "object" || Array.isArray(payload) || "error" in payload) {
    throw new DailyError("google_measurement_unavailable", 502);
  }
  const { rows = [], metadata } = record(payload);
  if (!Array.isArray(rows) || (metadata !== undefined && (!metadata || typeof metadata !== "object" || Array.isArray(metadata)))) {
    throw new DailyError("google_measurement_unavailable", 502);
  }
  const validDates = new Map<string, Metrics>();
  for (const raw of rows) {
    const row = record(raw);
    const date = Array.isArray(row.keys) ? row.keys[0] : null;
    const day = parseDay(date);
    const measured = metrics(row);
    if (typeof date !== "string" || !Number.isFinite(day) || day < start || day > end || !measured || validDates.has(date)) {
      throw new DailyError("google_measurement_unavailable", 502);
    }
    validDates.set(date, measured);
  }
  const firstIncomplete = record(metadata).first_incomplete_date;
  const incompleteDay = firstIncomplete === undefined ? Infinity : parseDay(firstIncomplete);
  if (Number.isNaN(incompleteDay)) throw new DailyError("google_measurement_unavailable", 502);
  return { rows: validDates, incompleteDay };
}

type GoogleDays = Awaited<ReturnType<typeof googleDays>>;
function classifyDays(days: string[], finalized: GoogleDays, exact: GoogleDays): DailyRow[] {
  return days.map((day) => {
    const isFinal = finalized.rows.has(day) && parseDay(day) < exact.incompleteDay;
    const value = exact.rows.get(day);
    if (!value) return unmeasured(day, isFinal ? "missing" : "pending");
    return { day, state: isFinal ? "final" : "provisional", ...value };
  });
}

function keepBetter(previous: DailyRow | undefined, newer: DailyRow) {
  if (previous?.state === "final") return previous;
  if (previous?.state === "missing" && newer.state !== "final") return previous;
  if (previous?.state === "provisional" && newer.state === "pending") return previous;
  return newer;
}

async function measureComparisonDays(ctx: DailyContext, days: string[], target: Pick<Target, "page" | "query">) {
  const start = parseDay(days[0]);
  const end = parseDay(days[days.length - 1]);
  const token = await ctx.googleToken();
  const [finalized, exact] = await Promise.all([
    googleDays(ctx, token, start, end), googleDays(ctx, token, start, end, target),
  ]);
  return classifyDays(days, finalized, exact);
}

export async function dailyComparison(ctx: DailyContext, input: {
  page: string; query: string; eventDay: number; track?: boolean; canEdit: boolean; headers: Record<string, string>;
}) {
  const { page, query, eventDay, track, canEdit, headers } = input;
  const now = Date.now();
  const collectedAt = new Date(now).toISOString();
  const start = eventDay - 7 * dayMs;
  const end = eventDay + 7 * dayMs;
  const days = daysBetween(start, end).filter((day) => day !== isoDay(eventDay));
  let target = await lookupTarget(ctx, page, query, headers);
  if (track === true && !target?.active) {
    serviceHeaders(ctx);
    const registered = await dbRequest(ctx, "rpc/team_gsc_register_daily_target", headers, {
      target_workspace: ctx.workspace, target_property: ctx.property, target_page: page, target_query: query, target_search_type: "web",
    });
    if (typeof registered !== "string" || !uuid.test(registered)) throw new DailyError("daily_storage_unavailable", 503);
    target = { id: registered, active: true };
  } else if (track === false && target?.active) {
    const paused = await dbRequest(ctx, "rpc/team_gsc_pause_daily_target", headers, {
      target_workspace: ctx.workspace, target_id: target.id,
    });
    if (paused !== true) throw new DailyError("daily_storage_unavailable", 503);
    target = { id: target.id, active: false };
  }
  const saved = target ? await savedRows(ctx, target.id, start, end, headers) : new Map<string, DailyRow>();
  const lastDay = yesterdayPacific(now);
  const firstDay = lastDay - (historyWindowDays - 1) * dayMs;
  if (end < firstDay && !saved.size && track === undefined) throw new DailyError("daily_window_unavailable", 422);
  const due = days.filter((day) => {
    const date = parseDay(day);
    return date >= firstDay && date <= lastDay && !["final", "missing"].includes(saved.get(day)?.state ?? "");
  });
  let refreshFailed = false;
  if (track !== false && due.length) {
    const fresh = await measureComparisonDays(ctx, due, { page, query }).catch((error: unknown) => {
      if (!target?.active && !saved.size) throw error;
      refreshFailed = true;
      return [];
    });
    if (target?.active && canEdit && fresh.length) await persistRows(ctx, target.id, fresh, collectedAt, false);
    for (const row of fresh) saved.set(row.day, keepBetter(saved.get(row.day), row));
  }
  return { configured: true, tracked: !!target?.active, daily: days.map((day) => saved.get(day) ?? unmeasured(day, "pending")),
    ...(refreshFailed ? { refreshFailed: true } : {}) };
}

export async function collectDaily(ctx: DailyContext, belongs: (page: string) => boolean) {
  const now = Date.now();
  const collectedAt = new Date(now).toISOString();
  const lastDay = yesterdayPacific(now);
  const firstDay = lastDay - (refreshDays - 1) * dayMs;
  const headers = serviceHeaders(ctx);
  const dueAt = new Date(now - refreshIntervalMs).toISOString();
  const params = new URLSearchParams({
    select: "id,page,query", workspace_id: `eq.${ctx.workspace}`, property: `eq.${ctx.property}`, search_type: "eq.web", slot: "not.is.null",
    or: `(last_collected_at.is.null,last_collected_at.lt."${dueAt}")`, order: "last_collected_at.asc.nullsfirst,id.asc", limit: String(maxTargets),
  });
  const targets = await dbRequest(ctx, `team_gsc_daily_targets?${params}`, headers);
  if (!Array.isArray(targets) || targets.length > maxTargets) throw new DailyError("daily_storage_unavailable", 503);
  if (!targets.length) return { processed: 0, failed: 0 };
  // Rotate bounded batches so a repeatedly failing target cannot block the rest.
  const batch = Math.floor(now / 3_600_000) % Math.ceil(targets.length / batchSize);
  const selected = targets.slice(batch * batchSize, (batch + 1) * batchSize);
  const token = await ctx.googleToken();
  const finalized = await googleDays(ctx, token, firstDay, lastDay);
  const days = daysBetween(firstDay, lastDay);
  const outcomes = await Promise.allSettled(selected.map(async (raw) => {
    const { id, page, query } = record(raw);
    if (typeof id !== "string" || !uuid.test(id) || typeof page !== "string" || !belongs(page) ||
        typeof query !== "string" || !query.trim()) throw new DailyError("daily_storage_unavailable", 503);
    const exact = await googleDays(ctx, token, firstDay, lastDay, { page, query });
    await persistRows(ctx, id, classifyDays(days, finalized, exact), collectedAt, true);
  }));
  const processed = outcomes.filter((outcome) => outcome.status === "fulfilled").length;
  return { processed, failed: selected.length - processed };
}
