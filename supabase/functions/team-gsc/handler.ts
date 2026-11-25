import { timingSafeEqual } from "node:crypto";
import { collectDaily, dailyComparison, DailyError, isoDay, parseDay } from "./daily.ts";

const tokenUrl = "https://oauth2.googleapis.com/token";
const scope = "https://www.googleapis.com/auth/webmasters.readonly";
const cors = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Headers": "authorization, apikey, content-type, x-client-info",
  "Access-Control-Allow-Methods": "POST, OPTIONS",
  "Cache-Control": "no-store",
};
const encoder = new TextEncoder();
const dayMs = 86_400_000;
const fetchTimeout = () => AbortSignal.timeout(12_000);

function reply(body: object, status = 200) {
  return Response.json(body, { status, headers: cors });
}

function base64url(bytes: Uint8Array): string {
  return btoa(String.fromCharCode(...bytes)).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

async function assertion(email: string, pem: string) {
  const bytes = Uint8Array.from(atob(pem.replace(/-----[^-]+-----|\s/g, "")), (char) => char.charCodeAt(0));
  const key = await crypto.subtle.importKey("pkcs8", bytes, { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" }, false, ["sign"]);
  const iat = Math.floor(Date.now() / 1000);
  const encode = (data: object) => base64url(encoder.encode(JSON.stringify(data)));
  const data = `${encode({ alg: "RS256", typ: "JWT" })}.${encode({ iss: email, scope, aud: tokenUrl, iat, exp: iat + 600 })}`;
  const signature = await crypto.subtle.sign("RSASSA-PKCS1-v1_5", key, encoder.encode(data));
  return `${data}.${base64url(new Uint8Array(signature))}`;
}

function belongsToProperty(page: URL, property: string): boolean {
  if (!['http:', 'https:'].includes(page.protocol) || page.username || page.password) return false;
  if (property.startsWith("sc-domain:")) {
    const domain = property.slice("sc-domain:".length).toLowerCase();
    return !!domain && (page.hostname === domain || page.hostname.endsWith(`.${domain}`));
  }
  const prefix = new URL(property);
  const path = prefix.pathname.replace(/\/$/, "");
  return page.origin === prefix.origin && (page.pathname === path || page.pathname.startsWith(`${path}/`));
}

type Env = (name: string) => string | undefined;
type GoogleRow = { keys?: string[]; clicks: number; impressions: number; ctr: number; position: number };
type GoogleConfig = { property: string; client_email: string; private_key: string };
const batchSize = 10000;
const workspaceId = /^[a-f\d]{8}-[a-f\d]{4}-[a-f\d]{4}-[a-f\d]{4}-[a-f\d]{12}$/i;
const workspaceSecret = (workspace: string) => `TEAM_GSC_${workspace.toUpperCase().replaceAll("-", "_")}`;

async function googleAccessToken(request: typeof fetch, cfg: GoogleConfig, signal = fetchTimeout()) {
  const token = await request(tokenUrl, {
    method: "POST", signal, redirect: "error", body: new URLSearchParams({
      grant_type: "urn:ietf:params:oauth:grant-type:jwt-bearer", assertion: await assertion(cfg.client_email, cfg.private_key),
    }),
  });
  if (!token.ok) throw new DailyError("google_auth_unavailable", 502);
  const { access_token } = await token.json();
  if (typeof access_token !== "string" || !access_token) throw new DailyError("google_auth_unavailable", 502);
  return access_token;
}

function safeError(error: unknown) {
  return error instanceof DailyError ? reply({ error: error.code }, error.status) : reply({ error: "service_unavailable" }, 503);
}

export async function handleGsc(req: Request, env: Env, request = fetch): Promise<Response> {
  if (req.method === "OPTIONS") return new Response(null, { headers: cors });
  if (req.method !== "POST") return reply({ error: "method_not_allowed" }, 405);
  const authorization = req.headers.get("Authorization");
  if (!authorization?.startsWith("Bearer ")) return reply({ error: "sign_in_required" }, 401);
  const supabase = env("SUPABASE_URL");
  const apikey = env("SUPABASE_ANON_KEY");
  if (!supabase || !apikey) return reply({ error: "service_unavailable" }, 503);

  let workspace: string, page: string, query: string, day: number;
  let mode: "comparison" | "daily_comparison" | "pages" | "previous" | "queries" | "snapshot";
  let start: number, end: number, offset: number;
  let track: boolean | undefined;
  try {
    const input = await req.text();
    if (input.length > 6000) return reply({ error: "invalid_request" }, 400);
    const body = JSON.parse(input);
    ({ workspace, page, query } = body);
    mode = body.mode ?? "comparison";
    if (typeof workspace !== "string" || !workspaceId.test(workspace)) return reply({ error: "invalid_request" }, 400);
    const today = Math.floor(Date.now() / dayMs) * dayMs;
    day = parseDay(body.event_day);
    offset = body.offset ?? 0;
    if (mode === "comparison" || mode === "daily_comparison") {
      if (typeof page !== "string" || page.length > 2048 || typeof query !== "string" || !query.trim() || query.length > 1024 || !Number.isFinite(day)) return reply({ error: "invalid_request" }, 400);
      new URL(page);
      if (mode === "daily_comparison") {
        if (day > today || (body.track !== undefined && typeof body.track !== "boolean")) return reply({ error: "invalid_request" }, 400);
        track = body.track;
      }
      start = day - 7 * dayMs;
      end = day + 7 * dayMs;
    } else if (["pages", "previous", "queries", "snapshot"].includes(mode)) {
      const max = mode === "queries" ? 60000 : 25000;
      if (!Number.isInteger(offset) || offset < 0 || offset >= max || offset % batchSize !== 0) return reply({ error: "invalid_offset" }, 400);
      end = mode === "previous" ? today - 90 * dayMs : today;
      start = end - 90 * dayMs;
      if (mode === "snapshot") {
        start = parseDay(body.start);
        end = parseDay(body.end);
        if (!Number.isFinite(start) || !Number.isFinite(end) || end > today - 3 * dayMs || start < today - 540 * dayMs || end - start !== 28 * dayMs) return reply({ error: "invalid_window" }, 400);
      }
    } else return reply({ error: "invalid_mode" }, 400);
  } catch {
    return reply({ error: "invalid_request" }, 400);
  }

  try {
    const headers = { apikey, Authorization: authorization };
    const auth = await request(`${supabase}/auth/v1/user`, { headers, signal: fetchTimeout() });
    if (!auth.ok) return reply({ error: "sign_in_required" }, auth.status >= 500 ? 503 : 401);
    const user = await auth.json();
    const filters = new URLSearchParams({ select: "role", workspace_id: `eq.${workspace}`, user_id: `eq.${user.id}`, limit: "1" });
    const membership = await request(`${supabase}/rest/v1/workspace_members?${filters}`, { headers, signal: fetchTimeout() });
    if (!membership.ok) return reply({ error: "membership_unavailable" }, 503);
    const members = await membership.json();
    if (!members.length) return reply({ error: "workspace_access_denied" }, 403);
    const canEdit = ["owner", "editor"].includes(members[0].role);
    if (mode === "daily_comparison" && track !== undefined && !canEdit) return reply({ error: "daily_tracking_access_denied" }, 403);
    if (mode !== "comparison" && mode !== "daily_comparison" && !canEdit) return reply({ error: "analysis_access_denied" }, 403);
    const value = env(workspaceSecret(workspace));
    if (!value) return reply({ configured: false });
    const cfg: GoogleConfig = JSON.parse(value);
    if ((mode === "comparison" || mode === "daily_comparison") && !belongsToProperty(new URL(page), cfg.property)) return reply({ error: "page_outside_property" }, 400);
    if (mode === "daily_comparison") {
      const signal = AbortSignal.any([req.signal, AbortSignal.timeout(55_000)]);
      return reply(await dailyComparison({
        supabase, workspace, property: cfg.property, serviceKey: env("SUPABASE_SERVICE_ROLE_KEY"), request, signal,
        googleToken: () => googleAccessToken(request, cfg, AbortSignal.any([signal, fetchTimeout()])),
      }, { page, query, eventDay: day, track, canEdit, headers }));
    }
    if (mode === "comparison" && Math.floor(Date.now() / dayMs) < day / dayMs + 11) return reply({ configured: true, pending: true });

    const access_token = await googleAccessToken(request, cfg);
    const rowLimit = mode === "comparison" ? 15 : Math.min(batchSize, (mode === "queries" ? 60000 : 25000) - offset);
    const body = {
      startDate: isoDay(start), endDate: isoDay(end), rowLimit, dataState: "final",
      dimensions: mode === "comparison" ? ["date"] : mode === "queries" ? ["page", "query"] : ["page"],
      ...(mode === "comparison" ? { dimensionFilterGroups: [{ filters: [
        { dimension: "page", operator: "equals", expression: page }, { dimension: "query", operator: "equals", expression: query },
      ] }] } : { startRow: offset }),
    };
    const result = await request(`https://www.googleapis.com/webmasters/v3/sites/${encodeURIComponent(cfg.property)}/searchAnalytics/query`, {
      method: "POST", signal: fetchTimeout(), headers: { Authorization: `Bearer ${access_token}`, "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    if (!result.ok) return reply({ error: "google_measurement_unavailable" }, 502);
    const { rows = [] }: { rows?: GoogleRow[] } = await result.json();
    const metrics = ({ clicks, impressions, position, ctr }: GoogleRow) => ({ clicks, impressions, position, ctr });
    return reply({ configured: true, rows: rows.map((row) => mode === "comparison"
      ? { day: row.keys?.[0], clicks: row.clicks, impressions: row.impressions, position: row.position }
      : { ...metrics(row), page: row.keys?.[0], ...(mode === "queries" ? { query: row.keys?.[1] } : {}) }),
      ...(mode === "comparison" ? {} : { next: rows.length === rowLimit && offset + rowLimit < (mode === "queries" ? 60000 : 25000) ? offset + rowLimit : null }),
    });
  } catch (error) {
    return safeError(error);
  }
}

export async function handleGscCollector(req: Request, env: Env, request = fetch): Promise<Response> {
  if (req.method !== "POST") return reply({ error: "method_not_allowed" }, 405);
  const expected = env("TEAM_GSC_DAILY_TOKEN") ?? "";
  const supplied = /^Bearer ([\w.-]{32,512})$/.exec(req.headers.get("Authorization") ?? "")?.[1] ?? "";
  if (!/^[\w.-]{32,512}$/.test(expected) || expected.length !== supplied.length ||
      !timingSafeEqual(encoder.encode(expected), encoder.encode(supplied))) return reply({ error: "collection_access_denied" }, 401);
  try {
    const raw = await req.text();
    const body = raw.length <= 100 ? JSON.parse(raw) : null;
    if (!body || body.mode !== "collect_daily" || Object.keys(body).length !== 1) return reply({ error: "invalid_request" }, 400);
  } catch {
    return reply({ error: "invalid_request" }, 400);
  }
  try {
    const workspace = env("TEAM_GSC_DAILY_WORKSPACE");
    const supabase = env("SUPABASE_URL");
    const serviceKey = env("SUPABASE_SERVICE_ROLE_KEY");
    if (!workspace || !workspaceId.test(workspace) || !supabase || !serviceKey) return reply({ error: "service_unavailable" }, 503);
    const value = env(workspaceSecret(workspace));
    if (!value) return reply({ error: "service_unavailable" }, 503);
    const cfg: GoogleConfig = JSON.parse(value);
    const signal = AbortSignal.any([req.signal, AbortSignal.timeout(55_000)]);
    return reply(await collectDaily({
      supabase, workspace, property: cfg.property, serviceKey, request, signal,
      googleToken: () => googleAccessToken(request, cfg, AbortSignal.any([signal, fetchTimeout()])),
    }, (page) => belongsToProperty(new URL(page), cfg.property)));
  } catch (error) {
    return safeError(error);
  }
}
