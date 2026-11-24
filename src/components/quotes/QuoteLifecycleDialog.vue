<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import Icon from "../Icon.vue";
import ModalShell from "../ModalShell.vue";

const props = defineProps<{ action: "revise" | "trash"; number: string; busy: boolean }>();
const emit = defineEmits<{ close: []; confirm: [] }>();
const root = ref<HTMLElement | null>(null);
const cancel = ref<HTMLButtonElement | null>(null);
const title = computed(() => props.action === "revise" ? "Teklifi düzenle" : "Teklifi sil");
let app: HTMLElement | null = null;
let previousInert = false;

onMounted(() => {
  app = document.getElementById("app");
  if (app) {
    previousInert = app.inert;
    app.inert = true;
  }
  root.value?.querySelector('[role="dialog"]')?.setAttribute("aria-modal", "true");
  cancel.value?.focus();
});
onBeforeUnmount(() => { if (app) app.inert = previousInert; });

function onKey(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    if (!props.busy) emit("close");
  } else if (event.key === "Tab") {
    const buttons = [...(root.value?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [])];
    const focused = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (focused >= 0 && (event.shiftKey ? focused > 0 : focused < buttons.length - 1)) return;
    event.preventDefault();
    (event.shiftKey ? buttons[buttons.length - 1] : buttons[0])?.focus();
  }
}
</script>

<template>
  <Teleport to="body">
    <div ref="root" @keydown.stop="onKey">
      <ModalShell open :icon="action === 'revise' ? 'fileEdit' : 'archive'" :label="title" :title="title" :sub="number"
        :closable="!busy" @close="emit('close')">
        <p v-if="action === 'revise'" class="lifecycle-copy">
          Teklif taslağa alınır ve gönderilmiş sürümler korunur. Düzenledikten sonra gönderildi olarak işaretlediğinizde geçmişe yeni bir sürüm eklenir.
        </p>
        <p v-else class="lifecycle-copy">
          Teklif aktif listeden kaldırılır. Kalemleri ve gönderim geçmişiyle birlikte <strong>Silinenler</strong> bölümünden geri yükleyebilirsiniz.
        </p>
        <template #footer>
          <button ref="cancel" class="secondary-action" type="button" :disabled="busy" @click="emit('close')">Vazgeç</button>
          <button class="primary-action" :class="{ remove: action === 'trash', busy }" type="button" :disabled="busy" @click="emit('confirm')">
            <Icon v-if="busy" name="loader" :size="15" class="spin" aria-hidden="true" />
            {{ busy ? "İşleniyor…" : action === "revise" ? "Taslağa al ve düzenle" : "Teklifi sil" }}
          </button>
        </template>
      </ModalShell>
    </div>
  </Teleport>
</template>

<style scoped>
.lifecycle-copy { margin: 0; color: var(--c-mid); font-size: 13px; line-height: 1.6; }
.remove { background: var(--badge-eksik-c); }
</style>
