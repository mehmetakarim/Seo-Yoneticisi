<script setup lang="ts">
/**
 * Katalog sayfa ekranı — **tek bileşen, üç menü kaydı** (Kategoriler · Markalar · İçerikler).
 *
 * Düzen `ProductsPage` ile aynı: liste solda, detay sağda.
 *
 * ⚠️ Tip prop'la değil `store.page`den okunuyor: `App.vue` sayfaları
 * `<component :is="PAGES[store.page]">` ile çiziyor ve prop geçirmiyor. Kabuğun üç ekranı
 * özel olarak tanıması gerekmesin diye tip burada çözülüyor.
 *
 * 🔴 `App.vue`daki `:key="store.page"` bu ekran için ZORUNLU: üç anahtar da aynı bileşene
 * çıktığı için Vue örneği yeniden kurmaz ve Marka'dan Kategori'ye geçince önceki seçim,
 * arama metni ve liste olduğu gibi kalırdı (ölçüldü — anahtarsız hâlde bu davranış).
 */
import { computed, onMounted, ref } from "vue";
import { useStore } from "../../store";
import { KIND_BY_PAGE } from "../../catalog";
import CatalogList from "./CatalogList.vue";
import CatalogDetail from "./CatalogDetail.vue";

const store = useStore();
const kind = computed(() => KIND_BY_PAGE[store.page] ?? "category");

const listRef = ref<InstanceType<typeof CatalogList> | null>(null);
/** App.vue'daki ⌘F kısayolu aramayı buradan odaklar — Ürünler ve Kişiler'deki gibi. */
defineExpose({ focusSearch: () => listRef.value?.focusSearch() });

onMounted(async () => {
  // ⚠️ Bekleyen seçim (İçerik açığı → "düzenle") liste GELDİKTEN sonra tüketiliyor:
  // liste boşken seçim yapılamaz, satır bulunamaz ve istek sessizce düşerdi.
  const bekleyen = store.storePagePending;
  store.storePagePending = null;
  await store.loadStorePages(kind.value);
  if (bekleyen && bekleyen.kind === kind.value) store.selectStorePage(bekleyen.id);
});
</script>

<template>
  <div class="split">
    <CatalogList ref="listRef" :kind="kind" />
    <CatalogDetail :kind="kind" />
  </div>
</template>

<style scoped>
/* Sekiz satırlık `.split`, ProductsPage ve ContactsPage'de de var; paylaşmak yerine
   tekrar yazmak burada kabul edilebilir (aynı gerekçe ContactsPage başlığında). */
.split {
  flex: 1;
  display: flex;
  min-height: 0;
}
</style>
