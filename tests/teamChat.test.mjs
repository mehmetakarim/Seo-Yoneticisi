import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";
import { createClient } from "@supabase/supabase-js";
import { effectScope, nextTick, watch } from "vue";
import { createServer } from "vite";

const project = "https://team-chat.example.test";
const workspace = "10000000-0000-4000-8000-000000000001";
const me = "20000000-0000-4000-8000-000000000001";
const colleague = "20000000-0000-4000-8000-000000000002";
const outsider = "20000000-0000-4000-8000-000000000003";
const colleagueTopic = `team-presence:${workspace}:${colleague}`;

let vite, connection, useTeamChat;

before(async () => {
  vite = await createServer({
    root: fileURLToPath(new URL("..", import.meta.url)), configFile: false, appType: "custom",
    server: { middlewareMode: true, hmr: false, watch: null }, logLevel: "silent",
  });
  connection = await vite.ssrLoadModule("/src/teamConnection.ts");
  ({ useTeamChat } = await vite.ssrLoadModule("/src/teamChat.ts"));
});
after(async () => { await vite?.close(); });

class TestChannel {
  handlers = [];
  presence = {};
  tracks = [];
  removed = false;

  constructor(topic, options) {
    this.topic = topic;
    this.options = options;
  }

  on(type, filter, callback) {
    this.handlers.push({ type, filter, callback });
    return this;
  }

  subscribe(callback) {
    this.status = callback;
    queueMicrotask(() => { if (!this.removed) callback?.("SUBSCRIBED"); });
    return this;
  }

  async track(payload) {
    this.tracks.push(payload);
    return this.beforeTrack?.() ?? "ok";
  }

  async untrack() { return "ok"; }
  presenceState() { return this.presence; }

  emit(type, payload, event) {
    for (const handler of this.handlers) {
      if (handler.type === type && handler.filter.event === event) handler.callback(payload);
    }
  }
}

async function eventually(predicate, message) {
  const end = Date.now() + 1_500;
  while (!predicate()) {
    assert.ok(Date.now() < end, message);
    await new Promise((resolve) => setTimeout(resolve, 5));
  }
}

function record(id, sender, recipient, body) {
  return { id, workspace_id: workspace, sender_id: sender, recipient_id: recipient, body, created_at: "2026-11-24T10:00:00Z" };
}

function fixture(t, options) {
  const remote = {
    history: [], beforeHistory: null, failSend: false, rosterFailure: null, additionalMembers: [],
    statuses: null, beforeStatusRead: null, beforeStatusTouch: null, statusTouchFailure: false, beforePresenceTrack: null,
    statusClock: 100, otherRosters: new Map(),
  };
  const requests = [];
  const channels = [];
  const client = createClient(project, "test-key", {
    auth: { persistSession: false, autoRefreshToken: false, detectSessionInUrl: false },
    global: {
      async fetch(input, init) {
        const url = new URL(input instanceof Request ? input.url : String(input));
        assert.equal(url.origin, project);
        const body = typeof init?.body === "string" ? JSON.parse(init.body) : null;
        const actor = connection.teamUser.value;
        const activeWorkspace = connection.teamWorkspace.value;
        requests.push({ url, body, actor });
        if (url.pathname.endsWith("/rpc/team_chat_members")) {
          if (remote.rosterFailure) {
            return Response.json({ code: remote.rosterFailure.code, message: "Private server details" }, {
              status: remote.rosterFailure.status,
            });
          }
          return Response.json(body?.target_workspace === workspace ? [
            { user_id: me, display_name: "Deniz K" }, { user_id: colleague, display_name: "Ece A" },
            ...remote.additionalMembers,
          ] : remote.otherRosters.get(body?.target_workspace) ?? []);
        }
        if (url.pathname.endsWith("/team_direct_messages")) {
          const rows = structuredClone(remote.history);
          if (url.searchParams.has("recipient_id")) {
            assert.equal(url.searchParams.get("workspace_id"), `eq.${activeWorkspace}`);
            assert.equal(url.searchParams.get("recipient_id"), `eq.${actor}`);
            assert.equal(url.searchParams.get("select")?.split(",").includes("body"), false);
            const [createdAt, id] = /created_at\.gt\.([^,]+),and\(created_at\.eq\.[^,]+,id\.gt\.([\da-f-]+)/
              .exec(url.searchParams.get("or") ?? "")?.slice(1) ?? [];
            const order = url.searchParams.get("order");
            const direction = order === "created_at.asc,id.asc" ? 1 : -1;
            const page = rows.filter((row) => row.recipient_id === actor && row.workspace_id === activeWorkspace && (!createdAt
              || row.created_at > createdAt || (row.created_at === createdAt && row.id > id)))
              .sort((a, b) => direction * (a.created_at.localeCompare(b.created_at) || a.id.localeCompare(b.id)))
              .slice(0, Number(url.searchParams.get("limit")))
              .map(({ body: _body, ...metadata }) => metadata);
            return Response.json(page);
          }
          await remote.beforeHistory?.();
          return Response.json(rows);
        }
        if (url.pathname.endsWith("/team_member_status")) {
          assert.equal(url.searchParams.get("workspace_id"), `eq.${activeWorkspace}`);
          const rows = structuredClone(remote.statuses);
          await remote.beforeStatusRead?.();
          return rows ? Response.json(rows.filter((row) => row.workspace_id === activeWorkspace))
            : Response.json({ code: "PGRST205", message: "Fixture is running against the older schema" }, { status: 404 });
        }
        if (url.pathname.endsWith("/rpc/team_touch_member_status")) {
          assert.equal(body.target_workspace, activeWorkspace);
          await remote.beforeStatusTouch?.(body);
          if (remote.statusTouchFailure) return Response.json({ message: "Internal status failure" }, { status: 503 });
          const previous = remote.statuses.find((row) => row.user_id === actor && row.workspace_id === activeWorkspace);
          const row = statusRecord(actor, body.next_status ?? previous?.status ?? "available", ++remote.statusClock, activeWorkspace);
          remote.statuses = [...remote.statuses.filter((old) => old !== previous), row];
          return Response.json(row);
        }
        if (url.pathname.endsWith("/rpc/team_send_direct_message")) {
          if (remote.failSend) return Response.json({ message: "Test connection failure" }, { status: 503 });
          return Response.json(record(body.message_id, me, body.target_recipient, body.message_body));
        }
        assert.fail(`Unexpected request ${url.pathname}`);
      },
    },
  });
  client.channel = (topic, options) => {
    const channel = new TestChannel(topic, options);
    channel.beforeTrack = () => remote.beforePresenceTrack?.(topic);
    channels.push(channel);
    return channel;
  };
  client.removeChannel = async (channel) => { channel.removed = true; return "ok"; };

  connection.teamProject.value = project;
  connection.teamWorkspace.value = workspace;
  connection.teamSession.value = { user: { id: me } };
  connection.teamClient.value = client;
  connection.teamReachable.value = true;
  const scope = effectScope();
  const chat = scope.run(() => useTeamChat(options));
  t.after(() => {
    scope.stop();
    connection.teamClient.value = null;
    connection.teamSession.value = null;
    connection.teamWorkspace.value = "";
    connection.teamProject.value = "";
    connection.teamReachable.value = null;
  });
  return { chat, channels, remote, requests };
}

const chatChannels = (channels) => channels.filter((channel) => !channel.removed && channel.topic.startsWith("team-chat:"));
const deliver = (channel, row) => channel.emit("postgres_changes", { new: row }, "INSERT");
const statusChannels = (channels) => channels.filter((channel) => !channel.removed && channel.topic.startsWith("team-member-status:"));
const touches = (requests) => requests.filter(({ url }) => url.pathname.endsWith("/rpc/team_touch_member_status"));
function statusRecord(user, status, micros, target = workspace) {
  return { user_id: user, workspace_id: target, status, last_seen_at: `2026-11-25T10:00:00.${String(micros).padStart(6, "0")}Z` };
}

test("distinguishes an uninstalled chat, denied membership and a temporary connection failure", async (t) => {
  const { chat, channels, remote } = fixture(t);
  remote.rosterFailure = { code: "PGRST202", status: 404 };
  await eventually(() => chat.availability.value === "unavailable", "missing RPC should finish with a setup message");
  assert.match(chat.error.value, /Supabase’te henüz etkin değil/);
  assert.equal(channels.length, 0);

  remote.rosterFailure = { code: "", status: 503 };
  chat.retry();
  await eventually(() => chat.error.value.includes("Supabase bağlantısını kontrol"), "transient failures should show retry guidance");

  remote.rosterFailure = null;
  chat.retry();
  await eventually(() => chat.availability.value === "online", "retry should load the roster once the RPC is available");
  assert.equal(chat.error.value, "");
  assert.equal(chat.members.value.length, 2);
  chat.selectMember(colleague);

  remote.rosterFailure = { code: "42501", status: 403 };
  chat.retry();
  await eventually(() => chat.error.value.includes("ekip sohbetine erişiminiz doğrulanamadı"), "permission failures should show membership guidance");
  assert.deepEqual(chat.members.value, []);
  assert.equal(chat.selectedMember.value, null);
});

test("online status comes from each member's private channel and only self is tracked", async (t) => {
  const { chat, channels, requests } = fixture(t);
  await eventually(() => channels.some(({ topic }) => topic === colleagueTopic), "presence channels should start");
  const peer = channels.find(({ topic }) => topic === colleagueTopic);
  const self = channels.find(({ topic }) => topic === `team-presence:${workspace}:${me}`);
  assert.equal(peer.options.config.private, true);
  assert.equal(self.options.config.private, true);
  const peerStatus = () => chat.members.value.find(({ id }) => id === colleague)?.presence;
  assert.equal(peerStatus(), "unknown");
  peer.emit("presence", {}, "sync");
  assert.equal(peerStatus(), "offline");
  peer.presence = { [colleague]: [{ presence_ref: "workstation" }, { presence_ref: "other-window" }] };
  peer.emit("presence", {}, "sync");
  assert.equal(peerStatus(), "online");
  peer.presence[colleague].pop();
  peer.emit("presence", {}, "sync");
  assert.equal(peerStatus(), "online");
  assert.equal(peer.tracks.length, 0);
  await eventually(() => self.tracks.length === 1, "the local session should publish only its own status");
  await eventually(() => chatChannels(channels).length === 1, "older workspace should keep direct messages running");
  assert.equal(chat.memberStatusAvailable.value, false);
  assert.equal(chat.memberStatusError.value, "");
  assert.equal(statusChannels(channels).length, 0);
  assert.equal(touches(requests).length, 0);
});

test("status keeps actual Presence separate and heartbeats only the authenticated tracked self", async (t) => {
  t.mock.timers.enable({ apis: ["setInterval"] });
  const { chat, channels, remote, requests } = fixture(t);
  const releaseTrack = Promise.withResolvers();
  remote.beforePresenceTrack = () => releaseTrack.promise;
  const initialPeer = statusRecord(colleague, "busy", 1);
  remote.statuses = [initialPeer];
  const member = (id) => chat.members.value.find((item) => item.id === id);
  await eventually(() => chat.memberStatusAvailable.value && statusChannels(channels).length === 1, "load status for the active workspace");
  assert.equal(member(colleague).status, "busy");
  assert.equal(member(colleague).lastSeenAt, initialPeer.last_seen_at);
  assert.equal(member(colleague).presence, "unknown");
  assert.equal(member(me).lastSeenAt, null);
  t.mock.timers.tick(120_000);
  assert.equal(touches(requests).length, 0);

  releaseTrack.resolve("ok");
  await eventually(() => member(me).lastSeenAt !== null, "tracked self gets a server heartbeat");
  assert.deepEqual(touches(requests).map(({ body }) => body), [{ target_workspace: workspace, next_status: null }]);
  const selfSeen = member(me).lastSeenAt;
  t.mock.timers.tick(60_000);
  await eventually(() => member(me).lastSeenAt !== selfSeen, "one periodic authenticated heartbeat");
  assert.equal(touches(requests).length, 2);

  const self = channels.find(({ topic }) => topic === `team-presence:${workspace}:${me}`);
  self.status("TIMED_OUT");
  t.mock.timers.tick(120_000);
  assert.equal(touches(requests).length, 2);
  assert.equal(member(me).presence, "unknown");
  assert.equal(member(colleague).lastSeenAt, initialPeer.last_seen_at);

  remote.statusTouchFailure = true;
  const failed = chat.setMemberStatus("on_break");
  assert.equal(chat.memberStatusSaving.value, true);
  assert.equal(await chat.setMemberStatus("busy"), false);
  assert.equal(await failed, false);
  assert.equal(chat.memberStatusSaving.value, false);
  assert.match(chat.memberStatusError.value, /Durum kaydedilemedi/);
  assert.equal(member(me).status, "available");
  remote.statusTouchFailure = false;
  assert.equal(await chat.setMemberStatus("in_meeting"), true);
  assert.equal(member(me).status, "in_meeting");
  assert.equal(member(colleague).status, "busy");
  assert.equal(chat.memberStatusError.value, "");
});

test("status receives peer changes, ignores stale reads and isolates a new signed-in scope", async (t) => {
  const { chat, channels, remote, requests } = fixture(t);
  const member = (id) => chat.members.value.find((item) => item.id === id);
  remote.statuses = [statusRecord(colleague, "available", 1)];
  await eventually(() => statusChannels(channels).length === 1, "initial workspace should subscribe after loading");
  const [channel] = statusChannels(channels);
  assert.deepEqual(channel.handlers.map(({ filter }) => filter), ["INSERT", "UPDATE"].map((event) => ({
    table: "team_member_status", schema: "public", filter: `workspace_id=eq.${workspace}`, event,
  })));
  const joining = Promise.withResolvers();
  const releaseRead = Promise.withResolvers();
  remote.beforeStatusRead = () => { joining.resolve(); return releaseRead.promise; };
  channel.status("SUBSCRIBED");
  await joining.promise;
  const newer = statusRecord(colleague, "on_break", 2);
  newer.last_seen_at = "2026-11-25 10:00:00.000002+00";
  channel.emit("postgres_changes", { new: newer }, "UPDATE");
  channel.emit("postgres_changes", { new: statusRecord(colleague, "in_meeting", 3, outsider) }, "UPDATE");
  releaseRead.resolve();
  await nextTick();
  await new Promise((resolve) => setTimeout(resolve, 10));
  assert.equal(member(colleague).status, "on_break");
  assert.equal(member(colleague).lastSeenAt, "2026-11-25T10:00:00.000002+00:00");
  assert.equal(member(colleague).presence, "unknown");

  connection.teamReachable.value = false;
  await nextTick();
  assert.equal(member(colleague).status, "on_break");
  assert.equal(member(colleague).lastSeenAt, "2026-11-25T10:00:00.000002+00:00");
  assert.equal(await chat.setMemberStatus("busy"), false);
  await eventually(() => channel.removed, "former status channel must stop");

  remote.beforeStatusRead = null;
  remote.statuses = remote.statuses.map((row) => row.user_id === colleague ? statusRecord(colleague, "busy", 3) : row);
  connection.teamReachable.value = true;
  await eventually(() => member(colleague).status === "busy" && statusChannels(channels).length === 1,
    "reconnect must reload records missed during transport loss");
  const [currentChannel] = statusChannels(channels);
  const saved = Promise.withResolvers();
  const pending = Promise.withResolvers();
  remote.beforeStatusTouch = (body) => {
    if (body.next_status) { pending.resolve(); return saved.promise; }
  };
  const writing = chat.setMemberStatus("available");
  await pending.promise;
  const otherWorkspace = "10000000-0000-4000-8000-000000000002";
  remote.otherRosters.set(otherWorkspace, [
    { user_id: outsider, display_name: "Ada T" }, { user_id: colleague, display_name: "Ece A" },
  ]);
  remote.statuses.push(statusRecord(colleague, "in_meeting", 1, otherWorkspace));
  connection.teamSession.value = { user: { id: outsider } };
  connection.teamWorkspace.value = otherWorkspace;
  saved.resolve();
  assert.equal(await writing, false);
  assert.equal(chat.memberStatusAvailable.value, false);
  assert.equal(chat.memberStatusSaving.value, false);
  assert.equal(chat.members.value.length, 0);
  currentChannel.emit("postgres_changes", { new: statusRecord(colleague, "in_meeting", 4) }, "UPDATE");
  assert.equal(chat.memberStatusAvailable.value, false);
  await eventually(() => member(outsider)?.lastSeenAt && statusChannels(channels).length === 1, "second account should load its own workspace");
  assert.equal(member(me), undefined);
  assert.equal(member(colleague).status, "in_meeting");
  assert.equal(member(colleague).lastSeenAt, statusRecord(colleague, "in_meeting", 1, otherWorkspace).last_seen_at);
  assert.ok(touches(requests).every(({ body, actor }) => actor === (body.target_workspace === otherWorkspace ? outsider : me)));
});

test("direct messages deduplicate Realtime and keep the same operation id after an uncertain send", async (t) => {
  const { chat, channels, remote, requests } = fixture(t);
  const existing = record("30000000-0000-4000-8000-000000000001", colleague, me, "Test geçmişi");
  remote.history = [existing];
  await eventually(() => chat.members.value.length === 2, "the members should be loaded");
  await chat.selectMember(colleague);
  await eventually(() => chat.messages.value.some(({ id }) => id === existing.id), "the conversation should load");
  const incoming = channels.find((channel) => channel.handlers.some(({ type, filter }) =>
    type === "postgres_changes" && filter.table === "team_direct_messages"));
  assert.ok(incoming);
  const newMessage = {
    ...record("30000000-0000-4000-8000-000000000002", colleague, me, "İkinci mesaj"),
    created_at: "2026-11-24 10:00:00.000001+00",
  };
  incoming.emit("postgres_changes", { new: newMessage }, "INSERT");
  incoming.emit("postgres_changes", { new: newMessage }, "INSERT");
  incoming.emit("postgres_changes", { new: record("30000000-0000-4000-8000-000000000003", colleague, outsider, "Yabancı") }, "INSERT");
  assert.equal(chat.messages.value.filter(({ id }) => id === newMessage.id).length, 1);
  assert.equal(chat.messages.value.at(-1).createdAt, "2026-11-24T10:00:00.000001+00:00");
  assert.equal(chat.messages.value.some(({ body }) => body === "Yabancı"), false);

  remote.failSend = true;
  assert.equal(await chat.sendMessage("Kısa bir not"), false);
  remote.failSend = false;
  assert.equal(await chat.sendMessage("Kısa bir not"), true);
  const sends = requests.filter(({ url }) => url.pathname.endsWith("/rpc/team_send_direct_message"));
  assert.equal(sends.length, 2);
  assert.equal(sends[0].body.message_id, sends[1].body.message_id);
  assert.equal(sends[1].body.target_recipient, colleague);
  assert.equal(chat.messages.value.filter(({ body, mine }) => body === "Kısa bir not" && mine).length, 1);
});

test("connection loss reports unknown presence and reconnect catches up on missed messages", async (t) => {
  const { chat, channels, remote, requests } = fixture(t);
  await eventually(() => chat.members.value.length === 2, "the members should be loaded");
  chat.selectMember(colleague);
  await eventually(() => !chat.loadingMessages.value, "initial history should finish");
  const peer = channels.find(({ topic }) => topic === colleagueTopic);
  peer.presence = { [colleague]: [{ presence_ref: "workstation" }] };
  peer.emit("presence", {}, "sync");
  assert.equal(chat.selectedMember.value.presence, "online");

  connection.teamReachable.value = false;
  await nextTick();
  assert.equal(chat.availability.value, "offline");
  assert.equal(chat.selectedMember.value.presence, "unknown");
  assert.equal(await chat.sendMessage("Çevrimdışı taslak"), false);
  assert.equal(requests.some(({ url }) => url.pathname.endsWith("/rpc/team_send_direct_message")), false);
  const missed = record("30000000-0000-4000-8000-000000000004", colleague, me, "Bağlantı kesikken gelen mesaj");
  remote.history = [missed];

  connection.teamReachable.value = true;
  await eventually(() => chat.messages.value.some(({ id }) => id === missed.id), "missed message should appear after reconnect");
  assert.equal(chat.availability.value, "online");
});

test("unread spans conversations, flags each live inbound once and clears the member being read", async (t) => {
  let reading = "";
  const { chat, channels, remote } = fixture(t, { isReading: (id) => id === reading });
  remote.additionalMembers = [{ user_id: outsider, display_name: "Ada T" }];
  const seen = record("30000000-0000-4000-8000-000000000001", colleague, me, "Önceki kayıt");
  remote.history = [seen];
  await eventually(() => chatChannels(channels).length === 1, "the inbox subscription should start");
  const [channel] = chatChannels(channels);
  deliver(channel, seen);
  assert.deepEqual(chat.incomingNotice.value, null);
  assert.equal(chat.unreadTotal.value, 0);

  const displayed = record("30000000-0000-4000-8000-000000000007", colleague, me, "Geçmişte okunan mesaj");
  remote.history = [seen, displayed];
  chat.selectMember(colleague);
  reading = colleague;
  await eventually(() => chat.messages.value.some(({ id }) => id === displayed.id), "history should display the missed row");
  chat.markRead(colleague);
  const visible = record("30000000-0000-4000-8000-000000000002", colleague, me, "Açık konuşma");
  deliver(channel, visible);
  assert.equal(chat.messages.value.some(({ id }) => id === visible.id), true);
  assert.equal(chat.unreadTotal.value, 0);

  const another = record("30000000-0000-4000-8000-000000000003", outsider, me, "Diğer ekip üyesi");
  deliver(channel, another);
  deliver(channel, another);
  deliver(channel, record("30000000-0000-4000-8000-000000000004", me, outsider, "Giden kayıt"));
  deliver(channel, { ...another, id: "30000000-0000-4000-8000-000000000005", workspace_id: outsider });
  assert.deepEqual(chat.incomingNotice.value, { id: another.id, senderId: outsider, senderName: "Ada T" });
  assert.deepEqual([...chat.unreadByMember.value], [[outsider, 1]]);
  assert.equal(chat.messages.value.some(({ id }) => id === another.id), false);

  chat.backToMembers();
  reading = "";
  deliver(channel, displayed);
  assert.equal(chat.unreadTotal.value, 1);
  const closed = record("30000000-0000-4000-8000-000000000006", colleague, me, "Panel kapalı");
  deliver(channel, closed);
  assert.equal(chat.unreadTotal.value, 2);
  assert.equal(chat.messages.value.length, 0);
  chat.markRead(outsider);
  assert.equal(chat.unreadTotal.value, 1);
  assert.equal(chat.incomingNotice.value.id, closed.id);
  chat.markRead(colleague);
  deliver(channel, closed);
  assert.equal(chat.unreadTotal.value, 0);
  assert.equal(chat.incomingNotice.value, null);
});

test("reconnect catches up the global inbox in cursor order without duplicate or historical notices", async (t) => {
  const { chat, channels, remote, requests } = fixture(t);
  const notices = [];
  const stopWatch = watch(chat.incomingNotice, (notice) => { if (notice) notices.push(notice.id); }, { flush: "sync" });
  t.after(stopWatch);
  const row = (index) => record(`30000000-0000-4000-8000-${String(index).padStart(12, "0")}`, colleague, me, "Örnek mesaj");
  remote.history = [row(1)];
  await eventually(() => chatChannels(channels).length === 1, "the inbox subscription should start");
  const [original] = chatChannels(channels);
  original.status("TIMED_OUT");
  assert.equal(chat.error.value, "");
  deliver(original, row(2));
  assert.deepEqual(notices, [row(2).id]);

  connection.teamReachable.value = false;
  await nextTick();
  assert.equal(chat.unreadTotal.value, 1);
  remote.history = Array.from({ length: 45 }, (_, index) => row(index + 1));
  connection.teamReachable.value = true;
  await eventually(() => notices.length === 2, "missed data should publish one coalesced notice");
  assert.deepEqual(notices, [row(2).id, row(45).id]);
  assert.equal(chat.unreadTotal.value, 44);
  assert.equal(chat.selectedMember.value, null);
  const queries = requests.filter(({ url }) => url.searchParams.has("recipient_id"));
  assert.ok(queries.some(({ url }) => url.searchParams.get("or")?.includes(`id.gt.${row(41).id}`)));

  const [current] = chatChannels(channels);
  deliver(current, row(45));
  assert.equal(chat.unreadTotal.value, 44);
  connection.teamWorkspace.value = "10000000-0000-4000-8000-000000000002";
  assert.equal(chat.unreadTotal.value, 0);
  assert.equal(chat.incomingNotice.value, null);
  deliver(current, row(46));
  assert.equal(chat.unreadTotal.value, 0);
});

test("changing the signed-in scope unloads the conversation and ignores a late history result", async (t) => {
  const { chat, channels, remote } = fixture(t);
  const started = Promise.withResolvers();
  const release = Promise.withResolvers();
  remote.history = [record("30000000-0000-4000-8000-000000000001", colleague, me, "Kapsam değişti")];
  remote.beforeHistory = () => { started.resolve(); return release.promise; };
  await eventually(() => chat.members.value.length === 2, "the members should be loaded");
  const oldChannels = [...channels];
  const selecting = chat.selectMember(colleague);
  await started.promise;
  connection.teamWorkspace.value = "10000000-0000-4000-8000-000000000002";
  connection.teamSession.value = { user: { id: outsider } };
  await nextTick();
  release.resolve();
  await selecting;
  await eventually(() => oldChannels.every((channel) => channel.removed), "old channels should be cleaned up");
  assert.equal(chat.selectedMember.value, null);
  assert.deepEqual(chat.messages.value, []);
});
