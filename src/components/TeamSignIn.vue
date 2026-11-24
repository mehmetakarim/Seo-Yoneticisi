<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { connectTeam, disconnectTeam, teamAccessIssue, teamConnectionRetry, teamSessionNotice, teamSigningOut, teamUser, teamWorkspaceError, teamWorkspaceLoading } from "../teamConnection";
import Icon from "./Icon.vue";
import ModalShell from "./ModalShell.vue";

const props = defineProps<{ checking: boolean }>();
const emailInput = ref<HTMLInputElement | null>(null);
const signingIn = ref(false);
const error = ref("");
const canSignIn = computed(() => !props.checking && !teamUser.value);

watch(canSignIn, async (enabled) => {
  if (enabled) {
    await nextTick();
    emailInput.value?.focus();
  }
}, { immediate: true });

async function login(event: SubmitEvent) {
  if (signingIn.value || teamSigningOut.value) return;
  const form = event.currentTarget as HTMLFormElement;
  const fields = new FormData(form);
  const email = String(fields.get("email") ?? "").trim();
  const password = String(fields.get("password") ?? "");
  if (!email || !password) {
    error.value = "E-posta adresinizi ve parolanızı girin.";
    return;
  }
  error.value = "";
  signingIn.value = true;
  try {
    await connectTeam(email, password);
    form.reset();
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : "Oturum açılamadı. Tekrar deneyin.";
  } finally {
    signingIn.value = false;
  }
}
</script>

<template>
  <main class="welcome">
    <ModalShell
      :open="true"
      :closable="false"
      :width="460"
      icon="search"
      :label="teamUser && !checking ? 'Ekip çalışma alanı' : 'Inside Manager’a hoş geldiniz'"
      :title="teamUser && !checking ? 'Ekip çalışma alanı' : 'Inside Manager’a hoş geldiniz'"
      :sub="canSignIn ? 'Ekip hesabınızla oturum açın.' : undefined"
    >
      <p v-if="checking || teamWorkspaceLoading" class="welcome-status" role="status">
        <Icon name="refresh" class="welcome-spinner" :size="15" />
        {{ checking ? "Kayıtlı oturum kontrol ediliyor…" : "Çalışma alanı açılıyor…" }}
      </p>

      <form v-else-if="canSignIn" class="welcome-form" @submit.prevent="login">
        <label>
          E-posta
          <input ref="emailInput" name="email" type="email" autocomplete="username" required />
        </label>
        <label>
          Parola
          <input name="password" type="password" autocomplete="current-password" required />
        </label>
        <p v-if="error || teamSessionNotice" class="welcome-error" role="alert">{{ error || teamSessionNotice }}</p>
        <button class="primary-action welcome-submit" :class="{ busy: signingIn }" :disabled="signingIn || teamSigningOut" type="submit">
          <Icon v-if="signingIn" name="refresh" class="welcome-spinner" :size="15" />
          {{ signingIn ? "Oturum açılıyor…" : "Oturum aç" }}
        </button>
      </form>

      <template v-else>
        <p class="welcome-status" role="status">
          {{ teamAccessIssue ? "Bu çalışma alanına erişim doğrulanamadı." :
            teamWorkspaceError || "Çalışma alanı yükleniyor…" }}
        </p>
        <p v-if="teamSessionNotice" class="welcome-error" role="alert">{{ teamSessionNotice }}</p>
        <div class="welcome-actions">
          <button class="primary-action" type="button" :disabled="teamSigningOut" @click="teamConnectionRetry++">Tekrar dene</button>
          <button class="secondary-action" type="button" :disabled="teamSigningOut" @click="disconnectTeam()">Oturumu kapat</button>
        </div>
      </template>
    </ModalShell>
  </main>
</template>

<style scoped>
.welcome {
  min-height: 100dvh;
  background: var(--c-bg);
}
.welcome-form {
  display: grid;
  gap: 14px;
}
.welcome-form label {
  display: grid;
  gap: 6px;
  color: var(--c-mid);
  font-size: 13px;
  font-weight: 550;
}
.welcome-form input {
  width: 100%;
  min-height: 40px;
  padding: 8px 12px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-input);
  color: var(--c-text);
}
.welcome-form input:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.welcome-submit {
  justify-content: center;
  margin-top: 4px;
}
.welcome-status, .welcome-error {
  margin: 0;
  font-size: 13px;
  line-height: 1.5;
}
.welcome-status {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--c-mid);
}
.welcome-error { color: var(--red); }
.welcome-actions { display: flex; flex-wrap: wrap; gap: 8px; }
.welcome-spinner {
  flex: none;
  animation: spin .8s linear infinite;
}
@media (prefers-reduced-motion: reduce) {
  .welcome-spinner { animation: none; }
}
</style>
