import type { RealtimeChannel, SupabaseClient } from "@supabase/supabase-js";
import { ref, shallowRef } from "vue";

const teamStatuses = ["available", "busy", "in_meeting", "on_break"] as const;
export type TeamAvailability = typeof teamStatuses[number];
type StatusRow = { userId: string; workspace: string; status: TeamAvailability; lastSeenAt: string };
type StatusTransport = {
  client: SupabaseClient; workspace: string; user: string; abort: AbortController; channels: RealtimeChannel[];
};
type StatusSession<Run> = {
  run: Run; supported: boolean; disabled: boolean; tracking: boolean;
  timer?: ReturnType<typeof setInterval>; channel?: RealtimeChannel;
  reading?: Promise<void>; reloadRequested?: boolean; heartbeat?: Promise<boolean>;
};

const columns = "workspace_id,user_id,status,last_seen_at";
const heartbeatInterval = 60_000;
const uuid = /^[\da-f]{8}(?:-[\da-f]{4}){3}-[\da-f]{12}$/i;
const timestamp = /^\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d{1,6})?(?:Z|[+-]\d{2}(?::?\d{2})?)$/;

export function normalizeServerTimestamp(value: unknown) {
  if (typeof value !== "string" || !timestamp.test(value)) return null;
  // Realtime can use PostgreSQL's space separator and short UTC offset.
  const date = value.replace(" ", "T").replace(/([+-]\d{2})(\d{2})?$/, (_, hours, minutes) => `${hours}:${minutes ?? "00"}`);
  return Number.isFinite(Date.parse(date)) ? date : null;
}

export function compareServerTimestamps(first: string, second: string) {
  const micros = (value: string) => Number((/\.(\d+)/.exec(value)?.[1] ?? "").padEnd(6, "0").slice(3));
  return Date.parse(first) - Date.parse(second) || micros(first) - micros(second);
}

function isTeamStatus(value: unknown): value is TeamAvailability {
  return teamStatuses.some((status) => status === value);
}

function statusRow(value: unknown, workspace: string): StatusRow | null {
  if (!value || typeof value !== "object") return null;
  const row = value as Record<string, unknown>;
  const lastSeenAt = normalizeServerTimestamp(row.last_seen_at);
  if (row.workspace_id !== workspace || typeof row.user_id !== "string" || !uuid.test(row.user_id)
    || !isTeamStatus(row.status) || !lastSeenAt) return null;
  return { userId: row.user_id, workspace, status: row.status, lastSeenAt };
}

function featureUnavailable(status: number, code: string) {
  return [401, 403, 404].includes(status) || ["42501", "42P01", "42883", "PGRST202", "PGRST205"].includes(code);
}

export function useTeamMemberStatus<Run extends StatusTransport>(isCurrent: (run: Run) => boolean) {
  const rows = shallowRef(new Map<string, StatusRow>());
  const memberStatusAvailable = ref(false);
  const memberStatusSaving = ref(false);
  const memberStatusError = ref("");
  let active: StatusSession<Run> | null = null;

  function current(session: StatusSession<Run>) {
    return active === session && isCurrent(session.run);
  }

  function merge(session: StatusSession<Run>, values: unknown[]) {
    if (!current(session) || session.disabled) return;
    let next: Map<string, StatusRow> | undefined;
    for (const value of values) {
      const row = statusRow(value, session.run.workspace);
      if (!row) continue;
      const previous = (next ?? rows.value).get(row.userId);
      if (previous && compareServerTimestamps(row.lastSeenAt, previous.lastSeenAt) <= 0) continue;
      (next ??= new Map(rows.value)).set(row.userId, row);
    }
    if (next) rows.value = next;
  }

  function clearTimer(session: StatusSession<Run>) {
    if (session.timer !== undefined) clearInterval(session.timer);
    session.timer = undefined;
  }

  function disable(session: StatusSession<Run>) {
    session.disabled = true;
    clearTimer(session);
    rows.value = new Map();
    memberStatusAvailable.value = false;
  }

  async function touch(session: StatusSession<Run>, status: TeamAvailability | null) {
    const { client, workspace, user, abort } = session.run;
    const result = await client.rpc("team_touch_member_status", { target_workspace: workspace, next_status: status })
      .abortSignal(abort.signal).retry(false);
    if (!current(session) || session.disabled) return false;
    if (result.error && featureUnavailable(result.status, result.error.code)) {
      disable(session);
      return false;
    }
    const saved = statusRow(result.data, workspace);
    if (result.error || !saved || saved.userId !== user || (status !== null && saved.status !== status)) return false;
    merge(session, [result.data]);
    return true;
  }

  function recordHeartbeat(session: StatusSession<Run>) {
    if (!current(session) || !session.tracking || session.disabled || session.heartbeat || memberStatusSaving.value) return;
    if (!session.supported) {
      void refresh(session);
      return;
    }
    session.heartbeat = touch(session, null).catch(() => false).finally(() => {
      session.heartbeat = undefined;
    });
  }

  function subscribe(session: StatusSession<Run>) {
    const { client, workspace, user, channels } = session.run;
    const channel = client.channel(`team-member-status:${workspace}:${user}`);
    session.channel = channel;
    channels.push(channel);
    const config = { schema: "public", table: "team_member_status", filter: `workspace_id=eq.${workspace}` };
    const receive = ({ new: row }: { new: unknown }) => merge(session, [row]);
    channel.on("postgres_changes", { ...config, event: "INSERT" }, receive)
      .on("postgres_changes", { ...config, event: "UPDATE" }, receive)
      .subscribe((status) => {
        if (status === "SUBSCRIBED" && current(session)) refresh(session, true);
      });
  }

  function refresh(session: StatusSession<Run>, afterSubscribe = false) {
    if (!current(session) || session.disabled) return;
    if (session.reading) {
      if (afterSubscribe) session.reloadRequested = true;
      return;
    }
    const { client, workspace, abort } = session.run;
    session.reading = (async () => {
      const pageSize = 100;
      for (let offset = 0; ; offset += pageSize) {
        const result = await client.from("team_member_status").select(columns).eq("workspace_id", workspace)
          .order("user_id").range(offset, offset + pageSize - 1).abortSignal(abort.signal).retry(false);
        if (!current(session) || session.disabled) return;
        if (result.error) {
          if (featureUnavailable(result.status, result.error.code)) disable(session);
          return;
        }
        if (!Array.isArray(result.data)) return;
        merge(session, result.data);
        if (result.data.length < pageSize) break;
      }
      const firstLoad = !session.supported;
      session.supported = true;
      memberStatusAvailable.value = true;
      if (firstLoad) recordHeartbeat(session);
    })().catch(() => {}).finally(() => {
      session.reading = undefined;
      if (!current(session) || session.disabled) return;
      if (session.supported && !session.channel) {
        // Re-read after joining to cover changes after the initial query.
        subscribe(session);
      } else if (session.reloadRequested) {
        session.reloadRequested = false;
        refresh(session);
      }
    });
  }

  function start(run: Run) {
    active = { run, supported: false, disabled: false, tracking: false };
    refresh(active);
  }

  function setTracking(run: Run, tracking: boolean) {
    const session = active;
    if (!session || session.run !== run || !current(session) || session.tracking === tracking) return;
    session.tracking = tracking;
    clearTimer(session);
    if (tracking && !session.disabled) {
      session.timer = setInterval(() => recordHeartbeat(session), heartbeatInterval);
      recordHeartbeat(session);
    }
  }

  function stop(run: Run) {
    if (active?.run !== run) return;
    clearTimer(active);
    active = null;
    memberStatusSaving.value = false;
  }

  function reset() {
    if (active) stop(active.run);
    rows.value = new Map();
    memberStatusAvailable.value = false;
    memberStatusSaving.value = false;
    memberStatusError.value = "";
  }

  async function setMemberStatus(status: TeamAvailability) {
    const session = active;
    if (!session || !current(session) || !session.supported || session.disabled || memberStatusSaving.value || !isTeamStatus(status)) return false;
    memberStatusSaving.value = true;
    memberStatusError.value = "";
    try {
      await session.heartbeat;
      if (!current(session) || session.disabled) return false;
      if (await touch(session, status)) return true;
    } catch {
      // Explicit changes get a retry message once the current request fails.
    } finally {
      if (current(session)) memberStatusSaving.value = false;
    }
    if (current(session) && !session.disabled) memberStatusError.value = "Durum kaydedilemedi. Tekrar deneyin.";
    return false;
  }

  return {
    rows, start, setTracking, stop, reset, memberStatusAvailable, memberStatusSaving, memberStatusError, setMemberStatus,
  };
}
