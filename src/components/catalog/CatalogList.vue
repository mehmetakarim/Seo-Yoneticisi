<script setup lang="ts">
/**
 * Katalog sayfa listesi — kategori · marka · içerik, **tek bileşen** (Faz İ2).
 *
 * Kroma ve satırın ortak parçaları `styles.css`ten geliyor (Ürünler ve Kişiler ile aynı).
 * Buradaki `.row` çizgili düzen: bu sayfaların görseli yok, kişi listesiyle aynı biçim.
 *
 * 🔴 **Sıralama gösterime göre, alfabeye göre değil** — arka uçta (`list_store_pages`).
 * Ölçüldü (2026-08-14): 265 marka kaydının yalnızca 54'ü Google'da görünüyor. Görünmeyen
 * bir sayfanın meta'sını düzeltmek ölçülebilir sonuç üretmiyor, o yüzden iş çıkaracak
 * olanlar üstte. Süzgeç şeridi bu farkı ayrıca söylüyor.
 */
import { computed, ref } from "vue";
import { useStore } from "../../store";
import type { StorePageFilter } from "../../store";
import type { StorePageDetail } from "../../types";
import Icon from "../Icon.vue";

const props = defineProps<{ kind: string }>();
const store = useStore();
const searchInput = ref<HTMLInputElement | null>(null);
defineExpose({ focusSearch: () => searchInput.value?.focus() });

/** Eksik = mağazada başlık ya da açıklama yok. Üretimin ilk hedefi bunlar. */
const eksik = (r: StorePageDetail) => !r.page_title.trim() || !r.meta_description.trim();
const gorunen = (r: StorePageDetail) => r.impressions > 0;
const taslakli = (r: StorePageDetail) => !!r.draft_page_title.trim();

const SUZGEC: { key: StorePageFilter; label: string; test?: (r: StorePageDetail) => boolean }[] = [
  { key: "hepsi", label: "Tümü" },
  { key: "eksik", label: "Eksik alan", test: eksik },
  { key: "gorunen", label: "Google'da görünen", test: gorunen },
  { key: "taslak", label: "Taslak var", test: taslakli },
];

const sayilar = computed(() => {
  const c = {} as Record<StorePageFilter, number>;
  for (const s of SUZGEC) {
    c[s.key] = s.test ? store.storePages.filter(s.test).length : store.storePages.length;
  }
  return c;
});

const satirlar = computed(() => {
  const q = store.storePageSearch.trim().toLocaleLowerCase("tr");
  const test = SUZGEC.find((s) => s.key === store.storePageFilter)?.test;
  return store.storePages.filter(
    (r) =>
      (!test || test(r)) &&
      (!q ||
        r.name.toLocaleLowerCase("tr").includes(q) ||
        r.slug.toLocaleLowerCase("tr").includes(q)),
  );
});

const ARAMA_IPUCU: Record<string, string> = {
  category: "Kategori adı ara",
  brand: "Marka adı ara",
  blog: "Yazı başlığı ara",
};

/** Binlik ayraçlı tam sayı — 1.274 gösterim "1274" diye okunmasın. */
const sayi = (n: number) => Math.round(n).toLocaleString("tr-TR");
</script>

<template>
  <section class="list-panel">
    <div class="search-wrap">
      <div class="search">
        <Icon name="search" :size="16" class="search-icon" />
        <input
          ref="searchInput"
          class="fx"
          :value="store.storePageSearch"
          @input="store.storePageSearch = ($event.target as HTMLInputElement).value"
          :placeholder="ARAMA_IPUCU[props.kind] || 'Ara'"
        />
      </div>
    </div>

    <div class="filters">
      <button
        v-for="f in SUZGEC"
        :key="f.key"
        class="filter"
        :class="{ on: store.storePageFilter === f.key }"
        @click="store.storePageFilter = f.key"
      >
        <span>{{ f.label }}</span>
        <span class="count">{{ sayilar[f.key] }}</span>
      </button>
    </div>

    <div class="rows om-scroll">
      <template v-if="satirlar.length">
        <div
          v-for="r in satirlar"
          :key="r.remote_id"
          class="row"
          :class="{ sel: r.remote_id === store.storePage?.remote_id }"
          @click="store.selectStorePage(r.remote_id)"
        >
          <div class="row-main">
            <div class="row-name">{{ r.name }}</div>
            <div class="row-sub">/{{ r.slug }}</div>
          </div>

          <!-- ⚠️ Gösterim rozeti yalnızca görünen sayfalarda: "0 gösterim" yazmak
               satırların çoğunu sıfırla doldurup asıl işi görünmez yapardı. -->
          <span v-if="r.impressions > 0" class="gos" :title="`${sayi(r.impressions)} gösterim · ${sayi(r.clicks)} tıklama`">
            {{ sayi(r.impressions) }}
          </span>
          <span v-if="r.draft_page_title.trim()" class="mini taslak">Taslak</span>
          <span v-else-if="eksik(r)" class="mini eks">Eksik</span>
        </div>
      </template>
      <div v-else class="empty">
        <Icon name="check" :size="30" :stroke-width="1.6" style="margin-bottom: 10px" />
        <div v-if="store.storePagesBusy">Yükleniyor…</div>
        <div v-else-if="store.storePages.length">Bu görünümde sayfa yok</div>
        <!-- Envanter hiç çekilmemişse liste boş olur; kullanıcı nereye gideceğini bilmeli. -->
        <div v-else>
          Sayfa envanteri boş. Ayarlar'daki <b>Bakım</b> bölümünden envanteri çekin.
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* Satırın ortak parçaları (.row-name/.row-sub/.empty/seçili satır) `styles.css`te —
   Ürünler ve Kişiler ile ortak. Burada yalnızca bu listeye özgü olanlar var. */
.rows {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
}
.row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 11px 16px;
  border-bottom: 1px solid var(--c-border);
  cursor: pointer;
  transition: background 0.14s cubic-bezier(0.32, 0.72, 0, 1);
}
.row:hover {
  background: var(--c-hover);
}
.gos {
  flex: none;
  font-size: 10.5px;
  font-weight: 620;
  font-variant-numeric: tabular-nums;
  color: var(--c-soft);
  background: var(--c-chip);
  padding: 1px 7px;
  border-radius: 20px;
  cursor: help;
}
.mini {
  flex: none;
  padding: 3px 8px;
  border-radius: 7px;
  font-size: 10.5px;
  font-weight: 600;
}
.mini.taslak {
  background: var(--accent-tint);
  color: var(--accent);
}
.mini.eks {
  background: var(--badge-eksik-bg);
  color: var(--badge-eksik-c);
}
</style>
