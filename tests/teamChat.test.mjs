import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";
import { createClient } from "@supabase/supabase-js";
import { effectScope, nextTick } from "vue";
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
    return "ok";
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

function fixture(t) {
  const remote = { history: [], beforeHistory: null, failSend: false, rosterFailure: null };
  const requests = [];
  const channels = [];
  const client = createClient(project, "test-key", {
    auth: { persistSession: false, autoRefreshToken: false, detectSessionInUrl: false },
    global: {
      async fetch(input, init) {
        const url = new URL(input instanceof Request ? input.url : String(input));
        assert.equal(url.origin, project);
        const body = typeof init?.body === "string" ? JSON.parse(init.body) : null;
        requests.push({ url, body });
        if (url.pathname.endsWith("/rpc/team_chat_members")) {
          if (remote.rosterFailure) {
            return Response.json({ code: remote.rosterFailure.code, message: "Private server details" }, {
              status: remote.rosterFailure.status,
            });
          }
          return Response.json(body?.target_workspace === workspace ? [
            { user_id: me, display_name: "Deniz K" }, { user_id: colleague, display_name: "Ece A" },
          ] : []);
        }
        if (url.pathname.endsWith("/team_direct_messages")) {
          const rows = structuredClone(remote.history);
          await remote.beforeHistory?.();
          return Response.json(rows);
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
  const chat = scope.run(() => useTeamChat());
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
  const { chat, channels } = fixture(t);
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
