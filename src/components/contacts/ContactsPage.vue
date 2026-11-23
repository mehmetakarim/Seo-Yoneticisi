<script setup lang="ts">
/**
 * "Kişiler" sayfası (Faz C): liste/kart ikili görünümü.
 *
 * Düzen `ProductsPage` ile aynı — orada da liste solda, detay sağda. Sekiz satırlık `.split`
 * CSS'i paylaşmak yerine tekrar yazmak burada kabul edilebilir; asıl kopyalanma riski taşıyan
 * liste kroması `styles.css`e taşındı.
 */
import { ref, watch } from "vue";
import { useStore } from "../../store";
import { teamClient, teamSession, teamUser, teamWorkspace } from "../../teamConnection";
import { contactSync } from "../../teamContacts";
import ContactList from "./ContactList.vue";
import ContactCard from "./ContactCard.vue";

const listRef = ref<InstanceType<typeof ContactList> | null>(null);
const store = useStore();

watch([teamClient, teamWorkspace], ([client, workspace], _previous, onCleanup) => {
  if (!client || !workspace || !teamSession.value) return;
  const channel = client.channel(`contact-list:${workspace}`)
    .on("postgres_changes", {
      event: "*", schema: "public", table: "team_contacts", filter: `workspace_id=eq.${workspace}`,
    }, () => { void store.loadContacts(); })
    .subscribe((state) => {
      if (state === "SUBSCRIBED") void store.loadContacts();
    });
  onCleanup(() => { void client.removeChannel(channel); });
}, { immediate: true });

/** App.vue'daki ⌘F kısayolu aramayı buradan odaklar. */
function focusSearch() {
  listRef.value?.focusSearch();
}
defineExpose({ focusSearch });
</script>

<template>
  <div class="contacts-page">
    <div v-if="teamUser && (contactSync.offline || contactSync.pending || contactSync.conflicts)" class="sync-note" role="status">
      <span v-if="contactSync.offline">Çevrimdışı · Bu cihazdaki ekip kopyası</span>
      <span v-if="contactSync.pending">{{ contactSync.pending }} değişiklik gönderim bekliyor</span>
      <span v-if="contactSync.conflicts">{{ contactSync.conflicts }} değişiklik çakıştı. Yerel düzenlemeler korunuyor.</span>
    </div>
    <div class="split">
      <ContactList ref="listRef" />
      <ContactCard />
    </div>
  </div>
</template>

<style scoped>
.contacts-page {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.sync-note {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 16px;
  padding: 8px 16px;
  color: var(--c-mid);
  background: var(--c-panel);
  border-bottom: 1px solid var(--c-border);
  font-size: 12px;
}
.split {
  flex: 1;
  display: flex;
  min-height: 0;
}
</style>
