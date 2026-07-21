<script setup lang="ts">
import { computed, ref } from "vue";
import { useStore } from "../store";
import type { FilterKey } from "../types";
import Icon from "./Icon.vue";

const store = useStore();
const searchInput = ref<HTMLInputElement | null>(null);

defineExpose({ focusSearch: () => searchInput.value?.focus() });

const filterDefs: { key: FilterKey; label: string }[] = [
  { key: "eksik", label: "Eksik" },
  { key: "hatali", label: "Hatalı" },
  { key: "bekliyor", label: "Açıklama Bekliyor" },
  { key: "uygun", label: "Uygun" },
  { key: "tamamlandi", label: "Tamamlandı" },
  { key: "tumu", label: "Tümü" },
];

const rows = computed(() => store.rows);

function okBadge(b: string) {
  return b === "uygun" || b === "tamamlandi";
}
</script>

<template>
  <section class="list-panel">
    <div class="search-wrap">
      <div class="search">
        <Icon name="search" :size="16" class="search-icon" />
        <input
          ref="searchInput"
          class="fx"
          :value="store.search"
          @input="store.setSearch(($event.target as HTMLInputElement).value)"
          placeholder="Ürün adı veya SKU ara"
        />
      </div>
    </div>

    <div class="filters">
      <button
        v-for="f in filterDefs"
        :key="f.key"
        class="filter"
        :class="{ on: store.filter === f.key }"
        @click="store.setFilter(f.key)"
      >
        <span>{{ f.label }}</span>
        <span class="count">{{ store.counts[f.key] }}</span>
      </button>
    </div>

    <div class="rows om-scroll">
      <template v-if="rows.length">
        <div
          v-for="p in rows"
          :key="p.sku"
          class="row"
          :class="{ sel: p.sku === store.selectedSku }"
          @click="store.select(p.sku)"
        >
          <div class="thumb">
            <img v-if="p.img_url" :src="p.img_url" :alt="p.name" @error="($event.target as HTMLImageElement).style.display='none'" />
            <Icon v-else name="image" :size="18" :stroke-width="1.7" class="thumb-ph" />
          </div>
          <div class="row-main">
            <div class="row-name">{{ p.name }}</div>
            <div class="row-sku">{{ p.sku }}</div>
          </div>

          <span v-if="p.meta_done && p.details_done" class="pill done">
            <Icon name="check" :size="11" :stroke-width="3" />Tamamlandı
          </span>
          <div v-else class="dual">
            <span class="mini" :class="{ ok: okBadge(p.meta_badge) }">
              <Icon v-if="okBadge(p.meta_badge)" name="check" :size="10" :stroke-width="3.4" />
              <span v-else class="ring"></span>
              Meta
            </span>
            <span class="mini" :class="{ ok: okBadge(p.details_badge) }">
              <Icon v-if="okBadge(p.details_badge)" name="check" :size="10" :stroke-width="3.4" />
              <span v-else class="ring"></span>
              Açıkl.
            </span>
          </div>
        </div>
      </template>
      <div v-else class="empty">
        <Icon name="check" :size="30" :stroke-width="1.6" style="margin-bottom:10px" />
        <div>Bu görünümde ürün yok</div>
      </div>
    </div>

    <div class="shortcuts">
      <span><kbd>↑↓</kbd> gezin</span>
      <span><kbd>G</kbd> meta</span>
      <span><kbd>⇧G</kbd> açıklama</span>
      <span><kbd>D</kbd> tamamla</span>
      <span><kbd>⌘F</kbd> ara</span>
    </div>
  </section>
</template>

<style scoped>
.list-panel {
  width: clamp(300px, 30vw, 392px);
  flex: none;
  border-right: 1px solid var(--c-border);
  display: flex;
  flex-direction: column;
  background: var(--c-list);
  min-height: 0;
}
.search-wrap {
  padding: 14px 16px 10px;
}
.search {
  position: relative;
}
.search-icon {
  position: absolute;
  left: 11px;
  top: 50%;
  transform: translateY(-50%);
  color: var(--c-faint);
}
.search input {
  width: 100%;
  height: 36px;
  padding: 0 12px 0 34px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-input);
  font-size: 13px;
  color: var(--c-text);
  outline: none;
}
.filters {
  display: flex;
  gap: 4px;
  padding: 0 14px 12px;
  overflow-x: auto;
}
.filter {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: none;
  padding: 6px 11px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 12.5px;
  font-weight: 500;
  cursor: pointer;
  white-space: nowrap;
}
.filter.on {
  background: var(--accent);
  color: #fff;
  border-color: var(--accent);
  font-weight: 600;
}
.filter .count {
  font-size: 11px;
  padding: 0 6px;
  border-radius: 20px;
  background: var(--c-chip);
  color: var(--c-soft);
  font-weight: 600;
}
.filter.on .count {
  background: rgba(255, 255, 255, 0.22);
  color: #fff;
}
.rows {
  flex: 1;
  overflow-y: auto;
  padding: 0 8px 12px;
  min-height: 0;
}
.row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 11px 12px;
  margin-bottom: 2px;
  border-radius: 10px;
  cursor: pointer;
}
.row:hover {
  background: var(--c-hover);
}
.row.sel {
  background: var(--accent-tint);
  box-shadow: inset 0 0 0 1px var(--accent);
}
.row.sel:hover {
  background: var(--accent-tint);
}
.thumb {
  width: 42px;
  height: 42px;
  flex: none;
  border-radius: 9px;
  background: var(--c-chip);
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid rgba(0, 0, 0, 0.04);
  overflow: hidden;
}
.thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.thumb-ph {
  color: rgba(140, 140, 150, 0.5);
}
.row-main {
  flex: 1;
  min-width: 0;
}
.row-name {
  font-size: 13.5px;
  font-weight: 600;
  color: var(--c-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  letter-spacing: -0.01em;
}
.row-sku {
  font-size: 11.5px;
  color: var(--c-faint);
  margin-top: 2px;
}
.pill {
  flex: none;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 9px;
  border-radius: 20px;
  font-size: 11px;
  font-weight: 600;
}
.pill.done {
  background: var(--badge-tamamlandi-bg);
  color: var(--badge-tamamlandi-c);
}
.dual {
  flex: none;
  display: flex;
  align-items: center;
  gap: 4px;
}
.mini {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 7px;
  border-radius: 7px;
  font-size: 10.5px;
  font-weight: 600;
  background: var(--soft-neutral-bg);
  color: var(--c-mid);
}
.mini.ok {
  background: var(--ok-soft-bg);
  color: var(--green);
}
.ring {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  border: 1.6px solid currentColor;
}
.empty {
  text-align: center;
  padding: 60px 24px;
  color: var(--c-faint);
  font-size: 13px;
}
.shortcuts {
  flex: none;
  border-top: 1px solid var(--c-border-soft);
  padding: 9px 14px;
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  font-size: 11px;
  color: var(--c-soft);
}
.shortcuts span {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}
kbd {
  min-width: 17px;
  height: 18px;
  padding: 0 4px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--c-border);
  border-bottom-width: 2px;
  border-radius: 5px;
  background: var(--c-input);
  font-size: 10.5px;
  font-weight: 600;
  color: var(--c-mid);
}
</style>
