<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { useTeamChat, type TeamAvailability } from "../teamChat";
import { teamProject, teamUser, teamWorkspace } from "../teamConnection";
import { refreshTeamAvatars, teamAvatarOnline, teamAvatarSources } from "../teamProfile";
import Icon from "./Icon.vue";
import TeamAvatar from "./TeamAvatar.vue";

const props = defineProps<{ open: boolean; pinned?: boolean }>();
const emit = defineEmits<{
  "update:open": [boolean];
  "update:pinned": [boolean];
  "unread-change": [total: number];
  "incoming-message": [notice: { id: string; senderId: string; senderName: string }];
}>();
const pinLabel = computed(() => props.pinned ? "Sabitlemeyi kaldır" : "Sabitle");
const messagePane = ref<HTMLElement | null>(null);
let readingMember: string | undefined;

function isReading(memberId: string) {
  const pane = messagePane.value;
  return props.open && readingMember === memberId && pane?.dataset.chatMember === memberId
    && pane.isConnected && pane.clientHeight > 0 && !pane.closest("[inert]")
    && pane.getAttribute("aria-busy") !== "true" && nearLatest()
    && typeof document !== "undefined" && document.visibilityState === "visible" && document.hasFocus();
}

const {
  members, selectedMember, messages, availability, loadingMessages, loadingOlder, hasOlder, sending, error,
  unreadTotal, unreadByMember, incomingNotice, markRead,
  memberStatusAvailable, memberStatusSaving, memberStatusError, setMemberStatus,
  selectMember, backToMembers, loadOlder, sendMessage, retry,
} = useTeamChat({ isReading });
type ChatMember = (typeof members.value)[number];

const presenceLabels = {
  online: "Çevrimiçi",
  offline: "Çevrimdışı",
  unknown: "Durum bilinmiyor",
};
const statusLabels = {
  available: "Müsait",
  busy: "Meşgul",
  in_meeting: "Toplantıda",
  on_break: "Molada",
} satisfies Record<TeamAvailability, string>;
const statusChoices = Object.entries(statusLabels);
const maxLength = 2000;
const closeButton = ref<HTMLButtonElement | null>(null);
const backButton = ref<HTMLButtonElement | null>(null);
const memberList = ref<HTMLElement | null>(null);
const historyButton = ref<HTMLButtonElement | null>(null);
const drafts = reactive(new Map<string, string>());
const newMessage = ref(false);
const sendError = ref("");
const latestNotice = ref("");
const requestedStatus = ref<TeamAvailability | null>(null);
const scopeVersion = ref(0);
const leavingSurfaces = new Set<HTMLElement>();
const composingInputs = new WeakSet<EventTarget>();

watch([teamProject, teamUser, teamWorkspace], () => {
  scopeVersion.value += 1;
  for (const surface of leavingSurfaces) surface.style.display = "none";
  drafts.clear();
  sendError.value = "";
  requestedStatus.value = null;
}, { flush: "sync" });
watch([() => props.open, teamAvatarOnline, teamProject, teamUser, teamWorkspace], ([open, online]) => {
  if (open && online) void refreshTeamAvatars();
}, { immediate: true });

const draft = computed({
  get: () => drafts.get(selectedMember.value?.id ?? "") ?? "",
  set: (text: string) => {
    if (selectedMember.value) drafts.set(selectedMember.value.id, text);
    sendError.value = "";
  },
});

const selectedMemberId = computed(() => selectedMember.value?.id);
const composerId = computed(() => `team-chat-message-${scopeVersion.value}-${selectedMemberId.value}`);
const lastMessage = computed(() => messages.value[messages.value.length - 1]);
const canSend = computed(() => availability.value === "online" && !sending.value && Boolean(draft.value.trim()));
const visibleError = computed(() => error.value || sendError.value);
const showConnection = computed(() => availability.value === "offline"
  || (availability.value === "unavailable" && !visibleError.value));
const unreadBadge = (memberId: string) => Math.min(100, unreadByMember.value.get(memberId) ?? 0);
watch(selectedMemberId, (memberId) => { readingMember = memberId; }, { immediate: true, flush: "sync" });
watch(unreadTotal, (total) => emit("unread-change", total), { immediate: true });
watch(incomingNotice, (notice) => { if (notice) emit("incoming-message", notice); }, { flush: "sync" });
watch(loadingMessages, (busy) => { if (!busy) markVisibleConversationRead(); }, { flush: "post" });

onMounted(() => {
  document.addEventListener("visibilitychange", markVisibleConversationRead);
  window.addEventListener("focus", markVisibleConversationRead);
});
onBeforeUnmount(() => {
  document.removeEventListener("visibilitychange", markVisibleConversationRead);
  window.removeEventListener("focus", markVisibleConversationRead);
  emit("unread-change", 0);
});
const dayFormat = new Intl.DateTimeFormat("tr-TR", { dateStyle: "medium" });
const timeFormat = new Intl.DateTimeFormat("tr-TR", { hour: "2-digit", minute: "2-digit" });
const lastSeenFormat = new Intl.DateTimeFormat("tr-TR", {
  day: "numeric", month: "short", year: "numeric", hour: "2-digit", minute: "2-digit", hourCycle: "h23",
});
const messageRows = computed(() => {
  let previousDay = "";
  return messages.value.map((message) => {
    const date = new Date(message.createdAt);
    const valid = Number.isFinite(date.valueOf());
    const day = valid ? dayFormat.format(date) : "";
    const dayHeading = day !== previousDay ? day : "";
    previousDay = day;
    return { ...message, dayHeading, time: valid ? timeFormat.format(date) : "" };
  });
});

function onlineStatus(member: ChatMember) {
  return !member.self && member.presence === "online" && memberStatusAvailable.value ? member.status : undefined;
}

function memberStatusLabel(member: ChatMember) {
  const status = onlineStatus(member);
  return status ? statusLabels[status] : presenceLabels[member.presence];
}

function showLastSeen(member: ChatMember) {
  return member.presence !== "online" && (memberStatusAvailable.value || Boolean(member.lastSeenAt));
}

function lastSeenText(value: string | null) {
  const date = new Date(value ?? "");
  return Number.isFinite(date.getTime()) ? lastSeenFormat.format(date) : "";
}

async function changeStatus(event: Event) {
  const control = event.currentTarget as HTMLSelectElement;
  const value = control.value as TeamAvailability;
  if (!memberStatusAvailable.value || requestedStatus.value || memberStatusSaving.value || availability.value !== "online"
    || !Object.prototype.hasOwnProperty.call(statusLabels, value)) return;
  const scope = scopeVersion.value;
  requestedStatus.value = value;
  try {
    await setMemberStatus(value);
  } finally {
    if (scope === scopeVersion.value) requestedStatus.value = null;
  }
}

function close() {
  emit("update:open", false);
}

function deactivateLeavingSurface(surface: Element) {
  if (!(surface instanceof HTMLElement)) return;
  leavingSurfaces.add(surface);
  surface.inert = true;
  if (surface.dataset.chatScope !== String(scopeVersion.value)) surface.style.display = "none";
}

function finishLeavingSurface(surface: Element) {
  if (surface instanceof HTMLElement) leavingSurfaces.delete(surface);
}

function nearLatest() {
  const pane = messagePane.value;
  return !pane || pane.scrollHeight - pane.clientHeight - pane.scrollTop <= 16;
}

function markVisibleConversationRead() {
  const memberId = selectedMemberId.value;
  if (memberId && messagePane.value?.querySelector("[data-chat-message]") && isReading(memberId)) markRead(memberId);
}

function onMessageScroll() {
  if (!nearLatest()) return;
  newMessage.value = false;
  markVisibleConversationRead();
}

function scrollToLatest() {
  const pane = messagePane.value;
  if (pane) pane.scrollTop = pane.scrollHeight;
  onMessageScroll();
}

watch([() => props.open, scopeVersion], async ([open, scope]) => {
  latestNotice.value = "";
  if (!open) return;
  await nextTick();
  if (!props.open || scope !== scopeVersion.value) return;
  closeButton.value?.focus();
  scrollToLatest();
}, { immediate: true });

watch([selectedMemberId, () => lastMessage.value?.id], async ([member, last], [previousMember, previousLast]) => {
  if (!props.open) return;
  const scope = scopeVersion.value;
  const changedMember = member !== previousMember;
  if (changedMember) sendError.value = "";
  latestNotice.value = "";
  const follow = changedMember || Boolean(last && !previousLast) || nearLatest() || lastMessage.value?.mine;
  await nextTick();
  if (!props.open || scope !== scopeVersion.value || selectedMember.value?.id !== member || lastMessage.value?.id !== last) return;
  if (follow) scrollToLatest();
  else if (last && last !== previousLast) newMessage.value = true;
  if (!changedMember && previousLast && last !== previousLast && lastMessage.value && !lastMessage.value.mine) {
    latestNotice.value = `${selectedMember.value?.name} yeni bir mesaj gönderdi.`;
  }
});

async function openConversation(id: string) {
  if (!props.open || selectedMember.value) return;
  const scope = scopeVersion.value;
  selectMember(id);
  await nextTick();
  if (props.open && scope === scopeVersion.value && selectedMemberId.value === id) backButton.value?.focus();
}

async function openMembers() {
  const id = selectedMember.value?.id;
  if (!props.open || !id) return;
  const scope = scopeVersion.value;
  backToMembers();
  await nextTick();
  if (!props.open || scope !== scopeVersion.value || selectedMember.value) return;
  const row = [...(memberList.value?.querySelectorAll<HTMLButtonElement>("[data-team-member]") ?? [])]
    .find((button) => button.dataset.teamMember === id);
  (row ?? closeButton.value)?.focus();
}

async function showOlder() {
  const pane = messagePane.value;
  const firstMessage = pane?.querySelector<HTMLElement>("[data-chat-message]");
  if (!pane || !firstMessage || loadingOlder.value) return;
  const offset = firstMessage.offsetTop;
  const focusWasOnHistory = document.activeElement === historyButton.value;
  await loadOlder();
  await nextTick();
  if (pane !== messagePane.value || !firstMessage.isConnected) return;
  pane.scrollTop += firstMessage.offsetTop - offset;
  if (focusWasOnHistory && document.activeElement === document.body) {
    (historyButton.value ?? pane).focus({ preventScroll: true });
  }
}

function onComposerKeydown(event: KeyboardEvent) {
  const target = event.currentTarget;
  if (event.key !== "Enter" || event.shiftKey || event.altKey || event.ctrlKey || event.metaKey
    || event.isComposing || event.keyCode === 229 || (target && composingInputs.has(target))) return;
  event.preventDefault();
  if (!event.repeat) void submit();
}

function setComposerComposition(event: CompositionEvent) {
  const target = event.currentTarget;
  if (!target) return;
  if (event.type === "compositionstart") composingInputs.add(target);
  else composingInputs.delete(target);
}

async function submit() {
  const member = selectedMember.value;
  const original = draft.value;
  const body = original.trim();
  if (!member || !canSend.value || body.length > maxLength) return;
  const scope = scopeVersion.value;
  sendError.value = "";
  try {
    const sent = await sendMessage(body);
    if (scope !== scopeVersion.value) return;
    if (!sent) {
      if (selectedMember.value?.id === member.id) sendError.value = "Mesajın gönderildiği doğrulanamadı. Tekrar deneyin.";
      return;
    }
    if (drafts.get(member.id) === original) drafts.delete(member.id);
    await nextTick();
    if (props.open && scope === scopeVersion.value && selectedMember.value?.id === member.id) scrollToLatest();
  } catch {
    if (scope === scopeVersion.value && selectedMember.value?.id === member.id) sendError.value = "Mesajın gönderildiği doğrulanamadı. Tekrar deneyin.";
  }
}
</script>

<template>
  <Transition name="surface-slide" @before-leave="deactivateLeavingSurface" @after-leave="finishLeavingSurface">
    <aside
      v-if="open"
      :key="scopeVersion"
      id="team-chat-panel"
      class="team-chat"
      :data-chat-scope="scopeVersion"
      aria-labelledby="team-chat-title"
      @keydown.stop
      @keydown.esc.prevent.stop="close"
    >
      <header class="panel-header">
        <button
          v-if="selectedMember"
          ref="backButton"
          type="button"
          class="icon-action back-action"
          aria-label="Ekip üyelerine dön"
          title="Ekip üyelerine dön"
          @click="openMembers"
        >
          <Icon name="arrowRight" :size="16" aria-hidden="true" />
        </button>
        <TeamAvatar v-if="selectedMember" class="avatar" :name="selectedMember.name" :source="teamAvatarSources.get(selectedMember.id)" />
        <span v-else class="icon-badge" aria-hidden="true"><Icon name="users" :size="17" /></span>
        <div class="header-copy">
          <h2 id="team-chat-title" :title="selectedMember?.name">{{ selectedMember?.name ?? "Ekip" }}</h2>
          <span
            v-if="selectedMember"
            class="member-status"
            :class="{ 'presence-pending': selectedMember.presence === 'unknown' }"
          >
            <span class="presence-dot" :data-presence="selectedMember.presence" :data-status="onlineStatus(selectedMember)" aria-hidden="true"></span>
            <span v-if="onlineStatus(selectedMember)" class="screen-reader-only">Çevrimiçi, </span>
            {{ memberStatusLabel(selectedMember) }}
          </span>
          <span v-else-if="members.length" class="member-status">{{ members.length }} ekip üyesi</span>
        </div>
        <div class="panel-actions">
          <button
            type="button"
            class="icon-action pin-action"
            aria-label="Ekip panelini sabitle"
            :aria-pressed="Boolean(pinned)"
            :title="pinLabel"
            @click="emit('update:pinned', !pinned)"
          >
            <Icon name="pin" :size="15" aria-hidden="true" />
          </button>
          <button ref="closeButton" type="button" class="icon-action" aria-label="Ekip panelini kapat" title="Ekip panelini kapat" @click="close">
            <Icon name="x" :size="17" aria-hidden="true" />
          </button>
        </div>
      </header>

      <div v-if="showConnection" class="connection-note">
        <Icon name="alert" :size="15" aria-hidden="true" />
        <span role="status">
          {{ availability === "offline" ? "Çevrimdışısınız. Mesaj göndermek için bağlantı gerekli." : "Ekip bağlantısı şu anda kullanılamıyor." }}
        </span>
        <button v-if="availability === 'unavailable'" type="button" class="text-action" @click="retry()">Yenile</button>
      </div>
      <div v-if="visibleError" class="error-note" role="alert">
        <Icon name="alert" :size="15" aria-hidden="true" />
        <span>{{ visibleError }}</span>
        <button v-if="availability !== 'offline' && !sendError" type="button" class="text-action" @click="retry()">Tekrar dene</button>
      </div>

      <div class="chat-body">
        <Transition name="surface-slide" @before-leave="deactivateLeavingSurface" @after-leave="finishLeavingSurface">
          <nav
            v-if="!selectedMember"
            key="member-list"
            ref="memberList"
            class="chat-view member-list om-scroll"
            :data-chat-scope="scopeVersion"
            aria-label="Ekip üyeleri"
            :aria-busy="availability === 'connecting'"
          >
            <ul v-if="members.length">
              <li v-for="member in members" :key="member.id">
                <div v-if="member.self" class="member-row">
                  <TeamAvatar :name="member.name" :source="teamAvatarSources.get(member.id)" />
                  <div class="member-copy">
                    <span class="member-name" :title="member.name">{{ member.name }} <span class="self-tag">Sen</span></span>
                    <span class="member-status">
                      <span class="presence-dot" :data-presence="member.presence" aria-hidden="true"></span>{{ presenceLabels[member.presence] }}
                    </span>
                    <span v-if="showLastSeen(member)" class="member-last-seen">
                      <template v-if="member.lastSeenAt">Son görülme · <time :datetime="member.lastSeenAt">{{ lastSeenText(member.lastSeenAt) }}</time></template>
                      <template v-else>Son görülme kaydı yok</template>
                    </span>
                  </div>
                  <div v-if="memberStatusAvailable" class="status-control">
                    <select
                      id="team-chat-status"
                      class="status-select"
                      aria-label="Ekip durumum"
                      :aria-describedby="memberStatusError ? 'team-chat-status-error' : undefined"
                      :aria-busy="Boolean(requestedStatus || memberStatusSaving)"
                      :disabled="Boolean(requestedStatus || memberStatusSaving) || availability !== 'online'"
                      :value="requestedStatus ?? member.status"
                      @change="changeStatus"
                    >
                      <option v-for="[value, label] in statusChoices" :key="value" :value="value">{{ label }}</option>
                    </select>
                  </div>
                </div>
                <button v-else class="member-row member-action" type="button" :data-team-member="member.id" @click="openConversation(member.id)">
                  <TeamAvatar :name="member.name" :source="teamAvatarSources.get(member.id)" />
                  <span class="member-copy">
                    <span class="member-name" :title="member.name">{{ member.name }}</span>
                    <span class="member-status">
                      <span class="presence-dot" :data-presence="member.presence" :data-status="onlineStatus(member)" aria-hidden="true"></span>
                      <span v-if="onlineStatus(member)" class="screen-reader-only">Çevrimiçi, </span>
                      {{ memberStatusLabel(member) }}
                    </span>
                    <span v-if="showLastSeen(member)" class="member-last-seen">
                      <template v-if="member.lastSeenAt">Son görülme · <time :datetime="member.lastSeenAt">{{ lastSeenText(member.lastSeenAt) }}</time></template>
                      <template v-else>Son görülme kaydı yok</template>
                    </span>
                  </span>
                  <template v-if="unreadBadge(member.id)">
                    <span class="member-unread" aria-hidden="true">{{ unreadBadge(member.id) === 100 ? "99+" : unreadBadge(member.id) }}</span>
                    <span class="screen-reader-only">{{ unreadByMember.get(member.id) }} okunmamış mesaj</span>
                  </template>
                  <Icon v-else name="message" :size="17" class="member-message" aria-hidden="true" />
                </button>
                <p v-if="member.self && memberStatusAvailable && memberStatusError" id="team-chat-status-error" class="status-error" role="alert">
                  {{ memberStatusError }}
                </p>
              </li>
            </ul>
            <div v-else-if="availability === 'online' && !error" class="empty-state">
              <span class="icon-badge"><Icon name="users" :size="19" /></span>
              <strong>Henüz ekip üyesi yok</strong>
            </div>
          </nav>

          <div v-else :key="selectedMember.id" class="chat-view conversation" :data-chat-scope="scopeVersion">
            <p class="screen-reader-only" role="status">{{ latestNotice }}</p>
            <section
              ref="messagePane"
              class="message-pane om-scroll"
              :data-chat-member="selectedMember.id"
              aria-label="Konuşma geçmişi"
              :aria-busy="loadingMessages"
              tabindex="0"
              @scroll="onMessageScroll"
            >
              <div v-if="hasOlder || loadingOlder" class="history-row">
                <button ref="historyButton" class="secondary-action" type="button" :disabled="loadingOlder" @click="showOlder">
                  <Icon :name="loadingOlder ? 'loader' : 'clock'" :size="14" :class="{ spin: loadingOlder }" aria-hidden="true" />
                  {{ loadingOlder ? "Yükleniyor…" : "Önceki mesajları yükle" }}
                </button>
              </div>
              <div v-if="!loadingMessages && !messages.length && !visibleError && availability === 'online'" class="empty-state">
                <span class="icon-badge"><Icon name="message" :size="19" /></span>
                <strong>Konuşmayı başlat</strong>
              </div>
              <ol v-if="messages.length" class="message-list" aria-label="Mesajlar">
                <li v-for="message in messageRows" :key="message.id">
                  <div v-if="message.dayHeading" class="day-divider">{{ message.dayHeading }}</div>
                  <div class="message" :class="{ mine: message.mine }" data-chat-message>
                    <div class="bubble">
                      <span class="screen-reader-only">{{ message.mine ? "Sen" : selectedMember.name }}</span>
                      <p>{{ message.body }}</p>
                      <time v-if="message.time" :datetime="message.createdAt">{{ message.time }}</time>
                    </div>
                  </div>
                </li>
              </ol>
            </section>

            <div v-if="newMessage" class="jump-row">
              <button type="button" class="secondary-action" @click="scrollToLatest">Yeni mesajlara git</button>
            </div>

            <form class="composer" @submit.prevent="submit">
              <div class="composer-label">
                <label :for="composerId">Mesaj</label>
                <span :id="`${composerId}-shortcut`" class="message-shortcut">Enter ile gönder · Shift+Enter ile satır ekle</span>
              </div>
              <textarea
                :id="composerId"
                v-model="draft"
                class="message-input fx om-scroll"
                rows="3"
                :maxlength="maxLength"
                placeholder="Mesajını yaz…"
                :aria-describedby="`${composerId}-shortcut ${composerId}-length`"
                @keydown="onComposerKeydown"
                @compositionstart="setComposerComposition"
                @compositionend="setComposerComposition"
              ></textarea>
              <div class="composer-footer">
                <span :id="`${composerId}-length`" class="message-length">{{ draft.length }} / {{ maxLength }}</span>
                <button type="submit" class="send-action" :disabled="!canSend">
                  <Icon :name="sending ? 'loader' : 'arrowRight'" :size="15" :class="{ spin: sending }" aria-hidden="true" />
                  {{ sending ? "Gönderiliyor…" : "Gönder" }}
                </button>
              </div>
            </form>
          </div>
        </Transition>
      </div>
    </aside>
  </Transition>
</template>

<style scoped>
.team-chat {
  position: absolute;
  inset: 0 0 0 auto;
  width: min(100%, var(--team-chat-width, 380px));
  z-index: 45;
  display: flex;
  flex-direction: column;
  background: var(--c-bg);
  color: var(--c-text);
  border-left: 1px solid var(--c-border);
}
.panel-header {
  flex: none;
  min-height: 64px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--c-border);
}
.header-copy, .member-copy { min-width: 0; flex: 1; }
.panel-actions { flex: none; display: flex; align-items: center; gap: 4px; }
.header-copy h2 {
  margin: 0;
  color: var(--c-text);
  font-size: 16px;
  font-weight: 640;
  letter-spacing: -0.018em;
  text-overflow: ellipsis;
  white-space: nowrap;
  overflow: hidden;
}
.icon-action {
  flex: none;
  display: inline-grid;
  place-items: center;
  width: 32px;
  height: 32px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--c-mid);
  cursor: pointer;
}
.icon-action:hover, .member-action:hover { background: var(--c-hover); color: var(--c-text); }
.pin-action[aria-pressed="true"] { background: var(--accent-tint); color: var(--badge-bekliyor-c); }
.back-action svg { transform: rotate(180deg); }
.team-chat button:focus-visible, .message-pane:focus-visible, .status-select:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.connection-note, .error-note {
  flex: none;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 9px 14px;
  border-bottom: 1px solid var(--c-border);
  background: var(--c-list);
  color: var(--c-mid);
  font-size: 12px;
  line-height: 1.4;
}
.connection-note { color: var(--warn-text); border-color: var(--warn-border); background: var(--warn-bg); }
.error-note { color: var(--badge-eksik-c); background: var(--badge-eksik-bg); }
.connection-note > svg, .error-note > svg { flex: none; }
.connection-note span, .error-note span { min-width: 0; overflow-wrap: anywhere; }
.text-action {
  flex: none;
  margin-left: auto;
  padding: 3px 5px;
  border: 0;
  border-radius: 5px;
  color: inherit;
  background: transparent;
  font-size: 12px;
  font-weight: 620;
  text-decoration: underline;
  text-underline-offset: 2px;
  cursor: pointer;
}
.chat-body { position: relative; flex: 1; min-height: 0; overflow: clip; }
.chat-view { position: absolute; inset: 0; z-index: 1; background: var(--c-bg); }
.chat-view.surface-slide-leave-active { z-index: 0; }
.member-list {
  --slide-enter-offset: calc(-1 * var(--slide-distance, 20px));
  --slide-leave-offset: var(--slide-enter-offset);
  overflow: auto;
  padding: 10px;
}
.member-list ul, .message-list { margin: 0; padding: 0; list-style: none; }
.member-row {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  min-height: 58px;
  margin: 0 0 2px;
  padding: 8px 9px;
  border: 1px solid transparent;
  border-radius: 10px;
  background: transparent;
  color: var(--c-text);
  text-align: left;
}
.member-action { cursor: pointer; }
.member-action:hover { border-color: var(--c-border-soft); }
.member-name {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
  font-weight: 600;
}
.member-status {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 3px;
  color: var(--c-mid);
  font-size: 11.5px;
  line-height: 1.25;
}
.presence-dot { width: 7px; height: 7px; flex: none; border-radius: 50%; background: var(--c-faint); }
.presence-dot[data-presence="online"] { background: var(--green); box-shadow: 0 0 0 2px var(--ok-soft-bg); }
.presence-dot[data-status="busy"] { background: var(--badge-eksik-c); box-shadow: 0 0 0 2px var(--badge-eksik-bg); }
.presence-dot[data-status="in_meeting"] { background: var(--accent); box-shadow: 0 0 0 2px var(--accent-tint); }
.presence-dot[data-status="on_break"] { background: var(--amber); box-shadow: 0 0 0 2px var(--warn-bg); }
.presence-dot[data-presence="unknown"] { border: 1px solid var(--c-soft); background: transparent; }
.member-last-seen {
  display: block;
  margin-top: 3px;
  color: var(--c-mid);
  font-size: 11px;
  line-height: 1.35;
}
.member-last-seen time { white-space: nowrap; font-variant-numeric: tabular-nums; }
.status-control { position: relative; flex: none; }
.status-control::after {
  position: absolute;
  top: calc(50% - 5px);
  right: 11px;
  width: 6px;
  height: 6px;
  border: solid var(--c-mid);
  border-width: 0 1.5px 1.5px 0;
  transform: rotate(45deg);
  pointer-events: none;
  content: "";
}
.status-select {
  min-height: 31px;
  appearance: none;
  padding: 0 25px 0 9px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-input);
  color: var(--c-text);
  font: inherit;
  font-size: 12px;
  font-weight: 590;
  cursor: pointer;
}
.status-select:hover:enabled { border-color: var(--c-soft); }
.status-select:disabled { color: var(--c-mid); cursor: default; }
html[data-theme="dark"] .status-select { color-scheme: dark; }
.status-error { margin: 0 9px 8px; padding-left: 44px; color: var(--badge-eksik-c); font-size: 11.5px; line-height: 1.35; }
.self-tag {
  margin-left: 3px;
  padding: 2px 5px;
  border-radius: 5px;
  background: var(--c-chip);
  color: var(--c-mid);
  font-size: 10px;
  font-weight: 540;
}
.member-message { flex: none; color: var(--c-soft); }
.member-action:hover .member-message { color: var(--accent); }
.presence-pending { visibility: hidden; }
.member-unread {
  flex: none;
  min-width: 21px;
  padding: 2px 5px;
  border-radius: 999px;
  background: var(--accent-tint);
  color: var(--badge-bekliyor-c);
  font-size: 11px;
  font-weight: 650;
  text-align: center;
  font-variant-numeric: tabular-nums;
}
.conversation { display: flex; flex-direction: column; }
.message-pane {
  position: relative;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-anchor: none;
  padding: 12px 14px 14px;
  background: var(--c-list);
}
.message-pane:focus-visible { outline-offset: -2px; }
.empty-state {
  min-height: 150px;
  display: grid;
  place-content: center;
  justify-items: center;
  gap: 6px;
  padding: 18px 10px;
  text-align: center;
  font-size: 13px;
}
.empty-state .icon-badge { margin-bottom: 4px; }
.empty-state strong { font-weight: 630; }
.history-row { display: flex; justify-content: center; margin-bottom: 10px; }
.message-list { display: grid; gap: 8px; }
.day-divider {
  display: flex;
  align-items: center;
  gap: 9px;
  margin: 3px 0 9px;
  color: var(--c-mid);
  font-size: 11px;
  white-space: nowrap;
}
.day-divider::before, .day-divider::after { content: ""; flex: 1; height: 1px; background: var(--c-border); }
.message { display: flex; }
.message.mine { justify-content: flex-end; }
.bubble {
  max-width: 88%;
  min-width: 0;
  padding: 8px 10px 6px;
  border: 1px solid var(--c-border);
  border-radius: 11px 11px 11px 4px;
  background: var(--c-card);
  font-size: 13px;
  line-height: 1.45;
}
.mine .bubble { border-radius: 11px 11px 4px 11px; background: var(--accent-tint); }
.bubble p { margin: 0; white-space: pre-wrap; overflow-wrap: anywhere; }
.bubble time { display: block; margin-top: 4px; text-align: right; color: var(--c-mid); font-size: 10.5px; font-variant-numeric: tabular-nums; }
.jump-row { display: flex; justify-content: center; padding: 5px 8px; border-top: 1px solid var(--c-border); background: var(--c-list); }
.composer { flex: none; display: grid; gap: 7px; padding: 12px 14px 14px; border-top: 1px solid var(--c-border); }
.composer-label { display: flex; flex-wrap: wrap; justify-content: space-between; align-items: baseline; gap: 4px 8px; }
.composer label { font-size: 12px; font-weight: 600; }
.message-shortcut { color: var(--c-mid); font-size: 10.5px; line-height: 1.35; }
.message-input {
  width: 100%;
  min-height: 62px;
  max-height: min(150px, 28vh);
  resize: vertical;
  outline: 0;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  padding: 8px 10px;
  background: var(--c-input);
  color: var(--c-text);
  font-size: 13px;
  line-height: 1.4;
}
.composer-footer { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
.message-length { color: var(--c-mid); font-size: 11px; font-variant-numeric: tabular-nums; }
.secondary-action, .send-action {
  min-height: 33px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 0 10px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 12px;
  font-weight: 590;
  cursor: pointer;
}
.secondary-action:hover:not(:disabled) { background: var(--c-hover); color: var(--c-text); }
.send-action { min-height: 35px; padding: 0 13px; border-color: transparent; background: var(--accent); color: #fff; }
.send-action:hover:not(:disabled) { filter: brightness(1.06); }
.secondary-action:disabled, .send-action:disabled { opacity: 0.6; cursor: default; }
.screen-reader-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}
@media (prefers-reduced-motion: reduce) {
  .team-chat .spin { animation: none; }
}
@media (max-width: 380px) {
  .panel-header { gap: 6px; padding-inline: 8px; }
  .panel-header .avatar { --team-avatar-size: 30px; }
  .panel-header .icon-action { width: 28px; }
}
</style>
