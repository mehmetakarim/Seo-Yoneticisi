<script setup lang="ts">
/**
 * "Teklifler" sayfası (Faz T): liste/düzenleyici ikili görünümü.
 *
 * Düzen Ürünler ve Kişiler ile aynı; liste kroması `styles.css`ten paylaşılıyor.
 * ⚠️ Kişi listesi de yükleniyor: düzenleyicideki "Müşteri" seçicisi ona bakıyor.
 */
import { ref } from "vue";
import QuoteList from "./QuoteList.vue";
import QuoteEditor from "./QuoteEditor.vue";

const listRef = ref<InstanceType<typeof QuoteList> | null>(null);

defineExpose({ focusSearch: () => listRef.value?.focusSearch() });
</script>

<template>
  <div class="split">
    <QuoteList ref="listRef" />
    <QuoteEditor />
  </div>
</template>

<style scoped>
.split {
  flex: 1;
  display: flex;
  min-height: 0;
}
</style>
