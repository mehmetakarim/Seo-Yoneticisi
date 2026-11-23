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
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useStore } from "../../store";
import { KIND_BY_PAGE } from "../../catalog";
import { teamAccessIssue, teamClient, teamReachable, teamUser, teamWorkspace } from "../../teamConnection";
import CatalogList from "./CatalogList.vue";
import CatalogDetail from "./CatalogDetail.vue";
import CreateCatalogPage from "./CreateCatalogPage.vue";
import type { StorePageDetail } from "../../types";

const store = useStore();
const kind = computed(() => KIND_BY_PAGE[store.page] ?? "category");
const showCreate = ref(false);
const createTitle = computed(() => ({
  category: "Yeni kategori",
  brand: "Yeni marka",
  blog: "Yeni blog yazısı",
})[kind.value] ?? "Yeni kayıt");

async function onCreated(page: StorePageDetail) {
  showCreate.value = false;
  store.storePageSearch = "";
  store.storePageFilter = "hepsi";
  await store.loadStorePages(page.kind);
  if (kind.value === page.kind) store.selectStorePage(page.remote_id);
  store.toast(page.team_pending ? "Mağaza kaydı oluşturuldu. Ortak kayıt eşitleme kuyruğunda."
    : page.status === 1 ? "Kayıt oluşturuldu ve yayımlandı." : "Pasif kayıt oluşturuldu.", "ok");
}

const listRef = ref<InstanceType<typeof CatalogList> | null>(null);
async function closeCreate() {
  showCreate.value = false;
  await nextTick();
  listRef.value?.focusCreate();
}
/** App.vue'daki ⌘F kısayolu aramayı buradan odaklar — Ürünler ve Kişiler'deki gibi. */
defineExpose({ focusSearch: () => listRef.value?.focusSearch() });

watch([teamClient, teamWorkspace, teamUser, teamReachable, teamAccessIssue, kind], ([client, workspace, user, reachable, denied, pageKind], _, onCleanup) => {
  if (!client || !workspace || !user || !reachable || denied) return;
  let refreshTimer: number | undefined;
  const channel = client.channel(`store-pages:${workspace}:${pageKind}`)
    .on("postgres_changes", { event: "*", schema: "public", table: "team_store_pages", filter: `workspace_id=eq.${workspace}` }, ({ new: row, old }) => {
      if (("kind" in row ? row.kind : "kind" in old ? old.kind : undefined) !== pageKind) return;
      window.clearTimeout(refreshTimer);
      refreshTimer = window.setTimeout(() => void store.loadStorePages(pageKind), 240);
    }).subscribe();
  onCleanup(() => {
    window.clearTimeout(refreshTimer);
    void client.removeChannel(channel);
  });
}, { immediate: true });

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
    <CatalogList ref="listRef" :kind="kind" :create-title="createTitle" @create="showCreate = true" />
    <CatalogDetail :kind="kind" />
    <CreateCatalogPage v-if="showCreate" :kind="kind" :title="createTitle" @close="closeCreate" @created="onCreated" />
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
