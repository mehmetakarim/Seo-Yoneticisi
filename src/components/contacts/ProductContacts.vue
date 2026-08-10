<script setup lang="ts">
/**
 * "Bu ürünle ilgilenenler" — ürün detayında (Faz C2).
 *
 * 🔑 SEO tarafıyla CRM'i birleştiren tek yer. Ürün ekranında çalışırken o ürünü kimin
 * sorduğunu görmek, içeriği kimin için yazdığınızı hatırlatıyor.
 *
 * ⚠️ **Kopya sayaç yok:** burası da kişi kartı da `contact_products` tablosunu sorguluyor.
 * Ürün tarafına ayrı bir sayaç konsaydı biri güncellenip diğeri unutulabilirdi.
 *
 * Hiç bağ yoksa bölüm **hiç çizilmiyor**: boş bir "0 kişi" satırı her üründe duran bir
 * gürültü olurdu (Faz K'de feed rozetinde öğrenildi — sıfırken gösterilmiyor).
 */
import { ref, watch } from "vue";
import { api } from "../../api";
import { useStore } from "../../store";
import type { ContactProduct } from "../../types";
import Icon from "../Icon.vue";

const store = useStore();
const kisiler = ref<ContactProduct[]>([]);

watch(
  () => store.selectedSku,
  async (sku) => {
    kisiler.value = sku ? await api.contactsOfProduct(sku).catch(() => []) : [];
  },
  { immediate: true },
);

/** Kişi kartına gider — kuyruktaki maddeye tıklamakla aynı davranış. */
function ac(id: number) {
  store.page = "contacts";
  void store.openContact(id);
}
</script>

<template>
  <div v-if="kisiler.length" class="pk">
    <div class="pk-bas">
      <Icon name="users" :size="14" :stroke-width="2" />
      <b>{{ kisiler.length }} kişi bu ürünle ilgilendi</b>
    </div>
    <div class="pk-liste">
      <button v-for="k in kisiler" :key="k.contact_id" class="pk-kisi" @click="ac(k.contact_id)">
        {{ k.name }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.pk {
  margin-bottom: 14px;
  padding: 12px 14px;
  border: 1px solid var(--c-border);
  border-radius: 10px;
  background: var(--c-list);
}
.pk-bas {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 13px;
  color: var(--c-text);
}
.pk-liste {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 9px;
}
.pk-kisi {
  padding: 4px 10px;
  border: 1px solid var(--c-border);
  border-radius: 20px;
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 12px;
  font-weight: 540;
  cursor: pointer;
}
.pk-kisi:hover {
  border-color: var(--accent);
  color: var(--accent);
}
</style>
