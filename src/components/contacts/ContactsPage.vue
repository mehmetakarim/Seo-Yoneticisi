<script setup lang="ts">
/**
 * "Kişiler" sayfası (Faz C): liste/kart ikili görünümü.
 *
 * Düzen `ProductsPage` ile aynı — orada da liste solda, detay sağda. Sekiz satırlık `.split`
 * CSS'i paylaşmak yerine tekrar yazmak burada kabul edilebilir; asıl kopyalanma riski taşıyan
 * liste kroması `styles.css`e taşındı.
 */
import { ref } from "vue";
import ContactList from "./ContactList.vue";
import ContactCard from "./ContactCard.vue";

const listRef = ref<InstanceType<typeof ContactList> | null>(null);

/** App.vue'daki ⌘F kısayolu aramayı buradan odaklar. */
function focusSearch() {
  listRef.value?.focusSearch();
}
defineExpose({ focusSearch });
</script>

<template>
  <div class="split">
    <ContactList ref="listRef" />
    <ContactCard />
  </div>
</template>

<style scoped>
.split {
  flex: 1;
  display: flex;
  min-height: 0;
}
</style>
