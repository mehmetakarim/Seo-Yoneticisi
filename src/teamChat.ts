import type { RealtimeChannel, SupabaseClient } from "@supabase/supabase-js";
import { computed, onScopeDispose, ref, shallowRef, watch } from "vue";
import {
  teamAccessIssue, teamClient, teamProject, teamReachable, teamSession, teamUser, teamWorkspace,
} from "./teamConnection";
import { compareServerTimestamps, normalizeServerTimestamp, useTeamMemberStatus, type TeamAvailability } from "./teamMemberStatus";

export type { TeamAvailability } from "./teamMemberStatus";

type Presence = "online" | "offline" | "unknown";
type Member = { id: string; name: string; presence: Presence; self: boolean; status: TeamAvailability; lastSeenAt: string | null };
type Message = { id: string; body: string; createdAt: string; mine: boolean };
type Envelope = {
  id: string; workspace_id: string; sender_id: string; recipient_id: string; created_at: string;
};
type MessageRow = Envelope & { body: string };
type Cursor = { id: string; createdAt: string };
type IncomingNotice = { id: string; senderId: string; senderName: string };
type Inbox = { initialized: boolean; baseline: Cursor | null; cursor: Cursor | null; seen: Set<string> };
type Transport = {
  client: SupabaseClient; workspace: string; user: string; scope: string;
  channels: RealtimeChannel[]; abort: AbortController;
  syncingInbox: boolean; inboxRefreshQueued: boolean;
};
type Conversation = {
  member: string; ready: boolean; oldest: Cursor | null; latest: Cursor | null;
  catchingUp: boolean; refreshQueued: boolean;
};
type PendingSend = { id: string };

const pageSize = 40;
const messageLimit = 2_000;
const inboxColumns = "id,workspace_id,sender_id,recipient_id,created_at";
const messageColumns = `${inboxColumns},body`;
const uuid = /^[\da-f]{8}(?:-[\da-f]{4}){3}-[\da-f]{12}$/i;
const rosterFeedback = {
  setup: "Ekip sohbeti Supabase’te henüz etkin değil. Kurulum tamamlandıktan sonra tekrar deneyin.",
  membership: "Bu çalışma alanında ekip sohbetine erişiminiz doğrulanamadı. Ekip üyeliğinizi kontrol edin.",
  connection: "Ekip listesi alınamadı. Supabase bağlantısını kontrol edip tekrar deneyin.",
};
type RosterIssue = keyof typeof rosterFeedback;

function messageEnvelope(value: unknown): Envelope | null {
  if (!value || typeof value !== "object") return null;
  const row = value as Partial<Envelope>;
  const createdAt = normalizeServerTimestamp(row.created_at);
  if (typeof row.id !== "string" || !uuid.test(row.id) || typeof row.workspace_id !== "string" || !uuid.test(row.workspace_id)
    || typeof row.sender_id !== "string" || !uuid.test(row.sender_id) || typeof row.recipient_id !== "string" || !uuid.test(row.recipient_id)
    || !createdAt) return null;
  const { id, workspace_id, sender_id, recipient_id } = row;
  return { id, workspace_id, sender_id, recipient_id, created_at: createdAt };
}

function messageRow(value: unknown): MessageRow | null {
  const row = messageEnvelope(value);
  const body = (value as Partial<MessageRow> | null | undefined)?.body;
  return row && typeof body === "string" ? { ...row, body } : null;
}

function messageOrder(a: Cursor, b: Cursor) {
  return compareServerTimestamps(a.createdAt, b.createdAt) || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0);
}

const envelopeCursor = (row: Envelope): Cursor => ({ id: row.id, createdAt: row.created_at });
const emptyInbox = (): Inbox => ({ initialized: false, baseline: null, cursor: null, seen: new Set() });

export function useTeamChat({ isReading }: { isReading?: (memberId: string) => boolean } = {}) {
  const roster = shallowRef<{ id: string; name: string }[]>([]);
  const presence = shallowRef<Record<string, Presence>>({});
  const selectedId = ref("");
  const messages = shallowRef<Message[]>([]);
  const rosterStatus = ref<"loading" | "ready" | "unavailable">("loading");
  const rosterIssue = ref<RosterIssue | null>(null);
  const loadingMessages = ref(false);
  const loadingOlder = ref(false);
  const hasOlder = ref(false);
  const sending = ref(false);
  const actionError = ref("");
  const unread = shallowRef(new Map<string, number>());
  const incomingNotice = shallowRef<IncomingNotice | null>(null);
  const unreadByMember = computed<ReadonlyMap<string, number>>(() => unread.value);
  const unreadTotal = computed(() => [...unread.value.values()].reduce((total, count) => total + count, 0));
  const retryCount = ref(0);
  const pendingSends = new Map<string, PendingSend>();
  let conversation: Conversation | null = null;
  let activeSend: PendingSend | null = null;
  let transport: Transport | null = null;
  let inbox = emptyInbox();
  let removingChannels: Promise<unknown> = Promise.resolve();

  const scope = computed(() => !teamAccessIssue.value && teamProject.value && uuid.test(teamUser.value)
    && uuid.test(teamWorkspace.value) ? `${teamProject.value}/${teamUser.value}/${teamWorkspace.value}` : "");
  const onlineClient = computed(() => scope.value && teamReachable.value !== false
    && teamSession.value?.user.id === teamUser.value ? teamClient.value : null);
  const availability = computed(() => !scope.value ? "unavailable" : teamReachable.value === false ? "offline"
    : !onlineClient.value || rosterStatus.value === "loading" ? "connecting"
    : rosterStatus.value === "unavailable" ? "unavailable" : "online");
  const memberStatus = useTeamMemberStatus<Transport>(current);
  const members = computed<Member[]>(() => roster.value.map((member) => {
    const stored = memberStatus.rows.value.get(member.id);
    return {
      ...member, self: member.id === teamUser.value, presence: presence.value[member.id] ?? "unknown",
      status: stored?.status ?? "available", lastSeenAt: stored?.lastSeenAt ?? null,
    };
  }));
  const selectedMember = computed(() => members.value.find(({ id }) => id === selectedId.value) ?? null);
  const error = computed(() => (rosterIssue.value ? rosterFeedback[rosterIssue.value] : "") || actionError.value);

  function current(run: Transport, chat?: Conversation) {
    return transport === run && scope.value === run.scope && onlineClient.value === run.client
      && (!chat || conversation === chat);
  }

  function resetInbox() {
    inbox = emptyInbox();
    unread.value = new Map();
    incomingNotice.value = null;
  }

  function markRead(memberId: string) {
    // History may already display rows before the inbox replay reaches them.
    if (conversation?.member === memberId) {
      for (const message of messages.value) {
        if (!message.mine) inbox.seen.add(message.id);
      }
    }
    if (unread.value.has(memberId)) {
      const remaining = new Map(unread.value);
      remaining.delete(memberId);
      unread.value = remaining;
    }
    if (incomingNotice.value?.senderId === memberId) incomingNotice.value = null;
  }

  function inbound(row: Envelope, run: Transport) {
    return row.workspace_id === run.workspace && row.recipient_id === run.user && row.sender_id !== run.user;
  }

  function recordIncoming(row: Envelope, run: Transport) {
    if (!current(run) || !inbound(row, run) || inbox.seen.has(row.id)
      || (inbox.baseline && messageOrder(envelopeCursor(row), inbox.baseline) <= 0)) return;
    const member = roster.value.find(({ id }) => id === row.sender_id);
    if (!member) return;
    inbox.seen.add(row.id);
    if (isReading?.(member.id)) return;
    unread.value = new Map(unread.value).set(member.id, (unread.value.get(member.id) ?? 0) + 1);
    return { id: row.id, senderId: member.id, senderName: member.name };
  }

  async function refreshInbox(run: Transport) {
    if (!current(run)) return;
    if (run.syncingInbox) {
      run.inboxRefreshQueued = true;
      return;
    }
    run.syncingInbox = true;
    let notice: IncomingNotice | undefined;
    try {
      do {
        const ascending = inbox.initialized;
        const boundary = inbox.cursor;
        const query = run.client.from("team_direct_messages").select(inboxColumns)
          .eq("workspace_id", run.workspace).eq("recipient_id", run.user);
        if (ascending && boundary) {
          query.or(`created_at.gt.${boundary.createdAt},and(created_at.eq.${boundary.createdAt},id.gt.${boundary.id})`);
        }
        const result = await query.order("created_at", { ascending }).order("id", { ascending })
          .limit(ascending ? pageSize : 1).abortSignal(run.abort.signal).retry(false);
        if (!current(run) || result.error) return;
        const rows = (result.data ?? []).flatMap((entry) => {
          const row = messageEnvelope(entry);
          return row && inbound(row, run) && (!boundary || messageOrder(envelopeCursor(row), boundary) > 0) ? [row] : [];
        });
        if (!ascending) {
          // The first successful read establishes a baseline for this scope.
          inbox.baseline = rows[0] ? envelopeCursor(rows[0]) : null;
          inbox.cursor = inbox.baseline;
          inbox.initialized = true;
          return;
        }
        for (const row of rows) notice = recordIncoming(row, run) ?? notice;
        const latest = rows[rows.length - 1];
        if (!latest || !current(run)) return;
        inbox.cursor = envelopeCursor(latest);
        if ((result.data?.length ?? 0) < pageSize) return;
      } while (current(run));
    } catch {
      // Realtime can still deliver messages. The next successful subscription retries the inbox read.
    } finally {
      run.syncingInbox = false;
      if (current(run)) {
        if (notice && unread.value.has(notice.senderId) && !isReading?.(notice.senderId)) incomingNotice.value = notice;
        if (run.inboxRefreshQueued) {
          run.inboxRefreshQueued = false;
          void refreshInbox(run);
        }
      }
    }
  }

  function inConversation(row: MessageRow, run: Transport, chat: Conversation) {
    return row.workspace_id === run.workspace && ((row.sender_id === run.user && row.recipient_id === chat.member)
      || (row.recipient_id === run.user && row.sender_id === chat.member));
  }

  function mergeRows(rows: MessageRow[], run: Transport, chat: Conversation) {
    if (!current(run, chat)) return;
    const merged = new Map(messages.value.map((entry) => [entry.id, entry]));
    for (const row of rows) {
      if (inConversation(row, run, chat)) {
        merged.set(row.id, { id: row.id, body: row.body, createdAt: row.created_at, mine: row.sender_id === run.user });
      }
    }
    messages.value = [...merged.values()].sort(messageOrder);
  }

  async function history(run: Transport, chat: Conversation, direction: "older" | "newer", boundary: Cursor | null) {
    const query = run.client.from("team_direct_messages").select(messageColumns).eq("workspace_id", run.workspace);
    const participants = `and(sender_id.eq.${run.user},recipient_id.eq.${chat.member}),and(sender_id.eq.${chat.member},recipient_id.eq.${run.user})`;
    const comparison = direction === "older" ? "lt" : "gt";
    const filter = boundary
      ? `and(or(${participants}),or(created_at.${comparison}.${boundary.createdAt},and(created_at.eq.${boundary.createdAt},id.${comparison}.${boundary.id})))`
      : participants;
    const ascending = direction === "newer";
    const result = await query.or(filter).order("created_at", { ascending }).order("id", { ascending })
      .limit(pageSize + 1).abortSignal(run.abort.signal).retry(false);
    if (result.error) throw new Error("Mesaj geçmişi alınamadı. Tekrar deneyin.");
    return (result.data ?? []).flatMap((value) => {
      const row = messageRow(value);
      return row && inConversation(row, run, chat) ? [row] : [];
    });
  }

  async function loadInitial(run: Transport, chat: Conversation) {
    if (loadingMessages.value || !current(run, chat)) return;
    loadingMessages.value = true;
    try {
      const rows = await history(run, chat, "older", null);
      if (!current(run, chat)) return;
      const page = rows.slice(0, pageSize);
      const newest = page[0];
      const oldest = page[page.length - 1];
      chat.latest = newest ? { id: newest.id, createdAt: newest.created_at } : null;
      chat.oldest = oldest ? { id: oldest.id, createdAt: oldest.created_at } : null;
      chat.ready = true;
      hasOlder.value = rows.length > pageSize;
      mergeRows(page, run, chat);
    } catch {
      if (current(run, chat)) actionError.value = "Mesaj geçmişi alınamadı. Tekrar deneyin.";
    } finally {
      if (current(run, chat)) {
        loadingMessages.value = false;
        if (chat.ready && chat.refreshQueued) void refreshMessages(run);
      }
    }
  }

  async function refreshMessages(run: Transport) {
    const chat = conversation;
    if (!chat || !current(run, chat)) return;
    if (loadingMessages.value || chat.catchingUp) {
      chat.refreshQueued = true;
      return;
    }
    chat.refreshQueued = false;
    if (!chat.ready) return loadInitial(run, chat);
    chat.catchingUp = true;
    try {
      let more = true;
      while (more && current(run, chat)) {
        const rows = await history(run, chat, "newer", chat.latest);
        if (!current(run, chat)) return;
        const page = rows.slice(0, pageSize);
        const newest = page[page.length - 1];
        if (newest) chat.latest = { id: newest.id, createdAt: newest.created_at };
        mergeRows(page, run, chat);
        more = rows.length > pageSize;
      }
    } catch {
      if (current(run, chat)) actionError.value = "Son mesajlar yenilenemedi. Tekrar deneyin.";
    } finally {
      if (current(run, chat)) {
        chat.catchingUp = false;
        if (chat.refreshQueued) void refreshMessages(run);
      }
    }
  }

  function backToMembers() {
    conversation = null;
    selectedId.value = "";
    messages.value = [];
    loadingMessages.value = false;
    loadingOlder.value = false;
    hasOlder.value = false;
    actionError.value = "";
  }

  function selectMember(id: string) {
    if (selectedId.value === id || !members.value.some((member) => member.id === id && !member.self)) return;
    backToMembers();
    selectedId.value = id;
    conversation = { member: id, ready: false, oldest: null, latest: null, catchingUp: false, refreshQueued: false };
    const run = transport;
    if (run && rosterStatus.value === "ready") void loadInitial(run, conversation);
  }

  async function loadOlder() {
    const run = transport;
    const chat = conversation;
    if (!run || !chat?.oldest || !current(run, chat) || !hasOlder.value || loadingOlder.value) return;
    loadingOlder.value = true;
    actionError.value = "";
    try {
      const rows = await history(run, chat, "older", chat.oldest);
      if (!current(run, chat)) return;
      const page = rows.slice(0, pageSize);
      const oldest = page[page.length - 1];
      if (oldest) chat.oldest = { id: oldest.id, createdAt: oldest.created_at };
      hasOlder.value = rows.length > pageSize;
      mergeRows(page, run, chat);
    } catch {
      if (current(run, chat)) actionError.value = "Önceki mesajlar alınamadı. Tekrar deneyin.";
    } finally {
      if (current(run, chat)) loadingOlder.value = false;
    }
  }

  async function sendMessage(text: string): Promise<boolean> {
    const run = transport;
    const chat = conversation;
    const body = text.trim();
    if (!run || !chat || !current(run, chat) || availability.value !== "online" || sending.value || !body) return false;
    if ([...body].length > messageLimit) {
      actionError.value = `Mesaj en fazla ${messageLimit.toLocaleString("tr-TR")} karakter olabilir.`;
      return false;
    }
    const pendingKey = JSON.stringify([chat.member, body]);
    const pending = pendingSends.get(pendingKey) ?? { id: crypto.randomUUID() };
    pendingSends.set(pendingKey, pending);
    activeSend = pending;
    sending.value = true;
    actionError.value = "";
    try {
      const { data, error: sendError } = await run.client.rpc("team_send_direct_message", {
        target_workspace: run.workspace, target_recipient: chat.member, message_id: pending.id, message_body: body,
      }).abortSignal(run.abort.signal).retry(false);
      const row = messageRow(data);
      if (sendError || !row || row.id !== pending.id || row.sender_id !== run.user || row.body !== body || !inConversation(row, run, chat)) {
        throw new Error("Mesaj kaydı doğrulanamadı.");
      }
      if (activeSend !== pending || scope.value !== run.scope) return false;
      pendingSends.delete(pendingKey);
      mergeRows([row], run, chat);
      return true;
    } catch {
      if (current(run, chat)) actionError.value = "Mesajın gönderildiği doğrulanamadı. Tekrar deneyebilirsiniz.";
      return false;
    } finally {
      if (activeSend === pending) {
        activeSend = null;
        sending.value = false;
      }
    }
  }

  function stop(run: Transport) {
    if (transport !== run) return;
    transport = null;
    memberStatus.stop(run);
    run.abort.abort();
    presence.value = {};
    loadingMessages.value = false;
    loadingOlder.value = false;
    if (conversation) {
      conversation.catchingUp = false;
      conversation.refreshQueued = false;
    }
    removingChannels = Promise.allSettled([removingChannels, ...run.channels.map((channel) => run.client.removeChannel(channel))]);
  }

  function failRoster(issue: RosterIssue) {
    rosterStatus.value = "unavailable";
    rosterIssue.value = issue;
    if (issue === "membership") {
      backToMembers();
      roster.value = [];
      presence.value = {};
      resetInbox();
      memberStatus.reset();
      pendingSends.clear();
      activeSend = null;
      sending.value = false;
    }
  }

  function trackMember(run: Transport, member: Member) {
    const channel = run.client.channel(`team-presence:${run.workspace}:${member.id}`, {
      config: { private: true, presence: { key: run.user } },
    });
    run.channels.push(channel);
    let subscription = 0;
    channel.on("presence", { event: "sync" }, () => {
      if (!current(run)) return;
      const online = Object.values(channel.presenceState()).some((entries) => entries.length > 0);
      presence.value = { ...presence.value, [member.id]: online ? "online" : "offline" };
    }).subscribe((status) => {
      if (!current(run)) return;
      const join = ++subscription;
      if (member.self) memberStatus.setTracking(run, false);
      if (status !== "SUBSCRIBED") {
        presence.value = { ...presence.value, [member.id]: "unknown" };
      } else if (member.self) {
        void channel.track({ user_id: run.user }).then((result) => {
          if (!current(run) || join !== subscription) return;
          if (result === "ok") memberStatus.setTracking(run, true);
          else presence.value = { ...presence.value, [member.id]: "unknown" };
        }).catch(() => {});
      }
    });
  }

  async function start(run: Transport) {
    await removingChannels;
    if (!current(run)) return;
    try {
      const result = await run.client.rpc("team_chat_members", { target_workspace: run.workspace })
        .abortSignal(run.abort.signal).retry(false);
      if (!current(run)) return;
      if (result.error?.code === "PGRST202") return failRoster("setup");
      if (result.error?.code === "42501" || result.status === 403) return failRoster("membership");
      if (result.error || !Array.isArray(result.data)) return failRoster("connection");
      const byId = new Map<string, { id: string; name: string }>();
      for (const row of result.data) {
        if (typeof row?.user_id === "string" && uuid.test(row.user_id) && typeof row.display_name === "string") {
          byId.set(row.user_id, { id: row.user_id, name: row.display_name });
        }
      }
      if (!byId.has(run.user)) return failRoster("membership");
      roster.value = [...byId.values()];
      rosterStatus.value = "ready";
      if (selectedId.value && !byId.has(selectedId.value)) backToMembers();
      for (const id of unread.value.keys()) {
        if (!byId.has(id)) markRead(id);
      }
      memberStatus.start(run);
      for (const member of members.value) trackMember(run, member);

      if (!inbox.initialized) await refreshInbox(run);
      if (!current(run)) return;

      const channel = run.client.channel(`team-chat:${run.workspace}:${run.user}`);
      run.channels.push(channel);
      const receive = ({ new: value }: { new: unknown }) => {
        const row = messageRow(value);
        if (!row) return;
        if (conversation) mergeRows([row], run, conversation);
        const notice = recordIncoming(row, run);
        if (notice) incomingNotice.value = notice;
      };
      channel.on("postgres_changes", {
        event: "INSERT", schema: "public", table: "team_direct_messages", filter: `recipient_id=eq.${run.user}`,
      }, receive).on("postgres_changes", {
        event: "INSERT", schema: "public", table: "team_direct_messages", filter: `sender_id=eq.${run.user}`,
      }, receive).subscribe((status) => {
        if (!current(run) || status !== "SUBSCRIBED") return;
        void refreshInbox(run);
        void refreshMessages(run);
      });
      void refreshMessages(run);
    } catch {
      if (current(run)) failRoster("connection");
    }
  }

  watch(scope, () => {
    backToMembers();
    roster.value = [];
    rosterIssue.value = null;
    presence.value = {};
    resetInbox();
    memberStatus.reset();
    pendingSends.clear();
    activeSend = null;
    sending.value = false;
  }, { flush: "sync" });

  watch([scope, onlineClient, retryCount], ([key, client], _, onCleanup) => {
    if (!key || !client) return;
    const run: Transport = {
      client, scope: key, user: teamUser.value, workspace: teamWorkspace.value, channels: [], abort: new AbortController(),
      syncingInbox: false, inboxRefreshQueued: false,
    };
    transport = run;
    rosterStatus.value = "loading";
    rosterIssue.value = null;
    actionError.value = "";
    void start(run);
    onCleanup(() => stop(run));
  }, { immediate: true });

  onScopeDispose(() => {
    if (transport) stop(transport);
  });

  function retry() {
    retryCount.value += 1;
  }

  return {
    members, selectedMember, messages, availability, loadingMessages, loadingOlder, hasOlder, sending, error,
    unreadTotal, unreadByMember, incomingNotice, markRead, selectMember, backToMembers, loadOlder, sendMessage, retry,
    memberStatusAvailable: memberStatus.memberStatusAvailable, memberStatusSaving: memberStatus.memberStatusSaving,
    memberStatusError: memberStatus.memberStatusError, setMemberStatus: memberStatus.setMemberStatus,
  };
}
