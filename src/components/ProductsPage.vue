<script setup lang="ts">
/**
 * "Ürünler" sayfası: senkron özeti + liste/detay ikili görünümü.
 * Daha önce App.vue içinde satır içi duruyordu; App.vue'nun sayfa-bağımsız bir kabuk
 * olabilmesi için ayrıldı (bkz. navigation.ts).
 */
import { ref } from "vue";
import ProductList from "./ProductList.vue";
import ProductDetail from "./ProductDetail.vue";
import SyncSummaryBar from "./SyncSummaryBar.vue";

const listRef = ref<InstanceType<typeof ProductList> | null>(null);

/** App.vue'daki ⌘F kısayolu aramayı buradan odaklar. */
function focusSearch() {
  listRef.value?.focusSearch();
}
defineExpose({ focusSearch });
</script>

<template>
  <div class="products">
    <SyncSummaryBar />
    <div class="split">
      <ProductList ref="listRef" />
      <ProductDetail />
    </div>
  </div>
</template>

<style scoped>
.products {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.split {
  flex: 1;
  display: flex;
  min-height: 0;
}
</style>
