<script setup lang="ts">
/**
 * Kişi listesi (Faz C).
 *
 * ⚠️ Sıralama **sonraki adım tarihine göre** (arka uçta): liste "bugün kime dönmeliyim"
 * sorusuna göre diziliyor, alfabeye göre değil. Tarihi olmayan kişiler sona düşüyor.
 *
 * Kroma (`.list-panel`, `.search`, `.filters`) `styles.css`ten geliyor — Ürünler ekranıyla
 * ortak. Kopyalanmadı: bu projede kopyalanan geometri üç kez saptı.
 *
 * ⚠️ Satırın ortak parçaları (`.row-name`, `.row-sub`, `.empty`, seçili satır) 2026-08-14'te
 * oraya taşındı: kroma paylaşılmıştı ama satır kopyalanmıştı ve ölçünce ayrıştığı görüldü
 * (ad ağırlığı 560 iken üründe 600'dü). Burada kalan `.row` çizgili düzen — bilinçli fark.
 */
import { computed, onMounted, ref } from "vue";
import { useStore } from "../../store";
import ContactImportModal from "./ContactImportModal.vue";
import Icon from "../Icon.vue";

const store = useStore();
const searchInput = ref<HTMLInputElement | null>(null);

/** Kanal listesi `seo_core::contacts::CHANNELS` ile aynı sırada. */
const KANALLAR = ["mail", "telefon", "instagram", "fuar", "referans", "diğer"];
const iceAktarAcik = ref(false);
defineExpose({ focusSearch: () => searchInput.value?.focus() });

onMounted(() => void store.loadContacts());

/** Bugün ve öncesi = dönülmesi gereken. Tarih karşılaştırması YYYY-AA-GG metniyle yeterli. */
const bugun = new Date().toISOString().slice(0, 10);
const zamaniGelmis = (d: string | null) => !!d && d.slice(0, 10) <= bugun;

const bekleyen = computed(
  () => store.contacts.filter((c) => zamaniGelmis(c.next_step_at) && !c.archived).length,
);

function tarihMetni(c: { next_step_at: string | null }) {
  if (!c.next_step_at) return "";
  const gun = Math.round(
    (new Date(c.next_step_at.slice(0, 10)).getTime() - new Date(bugun).getTime()) / 86400000,
  );
  if (gun === 0) return "bugün";
  if (gun === 1) return "yarın";
  if (gun < 0) return `${-gun} gün gecikti`;
  return `${gun} gün sonra`;
}
</script>

<template>
  <section class="list-panel">
    <!-- ⚠️ Eylemler arama satırında, süzgeç şeridinde DEĞİL: altı denetim 300px'lik panele
         sığmıyordu ve şerit yatay kayıyordu (ölçüldü). Süzgeç şeridi yalnızca süzgeç. -->
    <div class="search-wrap">
      <div class="search">
        <Icon name="search" :size="16" class="search-icon" />
        <input
          ref="searchInput"
          class="fx"
          :value="store.contactSearch"
          @input="store.setContactSearch(($event.target as HTMLInputElement).value)"
          placeholder="Ad, firma, telefon ara"
        />
      </div>
      <button class="eylem birincil" title="Yeni kişi ekle" @click="store.contact = null">
        <Icon name="plus" :size="15" :stroke-width="2.4" />
      </button>
      <button class="eylem" title="CSV'den içe aktar" @click="iceAktarAcik = true">
        <Icon name="upload" :size="15" :stroke-width="2" />
      </button>
    </div>

    <div class="filters">
      <button
        class="filter"
        :class="{ on: store.contactArchived }"
        @click="
          store.contactArchived = !store.contactArchived;
          store.loadContacts();
        "
      >
        <span>Arşiv dahil</span>
      </button>
      <!-- ⚠️ Süzgeçler ARKA UÇTA: liste 300 kişiye çıktığında istemcide süzmek tüm kayıtları
           çekmeyi gerektirirdi. Kanal tek değerli (sütun), etiket çoklu (tablo). -->
      <select
        class="filter sec"
        :class="{ on: store.contactChannel }"
        :value="store.contactChannel"
        @change="
          store.contactChannel = ($event.target as HTMLSelectElement).value;
          store.loadContacts();
        "
      >
        <option value="">Kanal: hepsi</option>
        <option v-for="k in KANALLAR" :key="k" :value="k">{{ k }}</option>
      </select>
      <select
        v-if="store.contactTags.length"
        class="filter sec"
        :class="{ on: store.contactTag }"
        :value="store.contactTag"
        @change="
          store.contactTag = ($event.target as HTMLSelectElement).value;
          store.loadContacts();
        "
      >
        <option value="">Etiket: hepsi</option>
        <option v-for="t in store.contactTags" :key="t" :value="t">{{ t }}</option>
      </select>

      <!-- Bilgi, filtre değil: kaç kişiye bugün dönülecek. -->
      <span v-if="bekleyen" class="filter bilgi">
        <Icon name="calendarClock" :size="13" :stroke-width="2" />
        <span>{{ bekleyen }} bekliyor</span>
      </span>
    </div>

    <div class="rows om-scroll">
      <template v-if="store.contacts.length">
        <div
          v-for="c in store.contacts"
          :key="c.id"
          class="row"
          :class="{ sel: c.id === store.contact?.id, ars: c.archived }"
          @click="store.openContact(c.id)"
        >
          <div class="row-main">
            <div class="row-name">
              {{ c.name }}
              <span v-if="c.archived" class="ars-rozet">arşiv</span>
            </div>
            <div class="row-sub">
              {{ c.company || "—" }}
              <template v-if="c.event_count"> · {{ c.event_count }} temas</template>
              <template v-if="c.tags.length"> · {{ c.tags.join(", ") }}</template>
            </div>
          </div>
          <span
            v-if="c.next_step_at"
            class="adim"
            :class="{ gecmis: zamaniGelmis(c.next_step_at) }"
            :title="c.next_step_note || 'Sonraki adım'"
            >{{ tarihMetni(c) }}</span
          >
        </div>
      </template>

      <!-- Boş liste dürüstçe ne yapılacağını söylüyor: CRM verisi sıfırdan üretiliyor. -->
      <div v-else-if="!store.contactsBusy" class="empty">
        <Icon name="users" :size="30" :stroke-width="1.6" style="margin-bottom: 10px" />
        <div v-if="store.contactSearch">Aramaya uyan kişi yok</div>
        <template v-else>
          <div>Henüz kişi eklenmemiş</div>
          <p class="e-alt">
            Sağdaki formu doldurup ilk kişiyi ekleyin. Kişiye <b>sonraki adım tarihi</b>
            verdiğinizde o gün Bugün ekranındaki listede çıkar.
          </p>
        </template>
      </div>
    </div>

    <ContactImportModal :open="iceAktarAcik" @close="iceAktarAcik = false" />
  </section>
</template>

<style scoped>
/* ⚠️ Panel kroması `styles.css`te (Ürünler ile ortak). Buradakiler kişiye özel satırlar. */
.search-wrap {
  display: flex;
  align-items: center;
  gap: 6px;
}
.search-wrap .search {
  flex: 1;
  min-width: 0;
}
.eylem {
  flex: none;
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-input);
  color: var(--c-mid);
  cursor: pointer;
  transition: all 0.14s cubic-bezier(0.32, 0.72, 0, 1);
}
.eylem:hover {
  color: var(--accent);
  border-color: var(--accent);
}
.eylem.birincil {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}
.eylem.birincil:hover {
  color: #fff;
  opacity: 0.9;
}
.filter.bilgi {
  cursor: default;
  background: transparent;
  border-color: transparent;
  color: var(--c-faint);
}
/* Süzgeç kutuları `.filter` kromasını paylaşıyor; yalnızca ok işareti için pay veriliyor. */
.filter.sec {
  padding-right: 6px;
  font-family: inherit;
}
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
.row.ars {
  opacity: 0.55;
}
.row-main {
  flex: 1;
  min-width: 0;
}
.ars-rozet {
  font-size: 10px;
  color: var(--c-faint);
  border: 1px solid var(--c-border);
  border-radius: 5px;
  padding: 0 4px;
  margin-left: 5px;
  font-weight: 500;
}
.adim {
  flex: none;
  font-size: 11.5px;
  font-weight: 560;
  color: var(--c-faint);
  white-space: nowrap;
}
/* Zamanı gelmiş adım vurgulu: liste bu soruya göre dizili. */
.adim.gecmis {
  color: var(--warn-text);
}
.e-alt {
  margin: 8px auto 0;
  max-width: 250px;
  font-size: 12px;
  line-height: 1.55;
  color: var(--c-faint);
}
</style>
