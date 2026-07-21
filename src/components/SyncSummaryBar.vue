<script setup lang="ts">
import { computed } from "vue";
import { useStore } from "../store";
import Icon from "./Icon.vue";

const store = useStore();

const chips = computed(() => {
  const s = store.lastSync;
  if (!s) return [];
  return [
    { label: "Aktif", value: s.active, color: "var(--green)" },
    { label: "Eklenen", value: s.added, color: "var(--c-mid)" },
    { label: "Güncellenen", value: s.updated, color: "var(--c-mid)" },
    { label: "Silinen (düşen)", value: s.deleted, color: "var(--c-soft)" },
    { label: "Mükerrer atlanan", value: s.duplicate_skipped, color: "var(--c-soft)" },
  ];
});
</script>

<template>
  <div v-if="store.showSummary && store.lastSync" class="bar">
    <Icon name="check" :size="14" :stroke-width="2.4" style="color:#2fa84f" />
    <span class="lead">Feed güncellendi</span>
    <span v-for="(c, i) in chips" :key="i" class="chip">
      {{ c.label }} <b :style="{ color: c.color }">{{ c.value }}</b>
    </span>
  </div>
</template>

<style scoped>
.bar {
  flex: none;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 9px 22px;
  background: var(--c-list);
  border-bottom: 1px solid var(--c-border-soft);
  font-size: 12px;
  color: var(--c-soft);
  animation: popIn 0.25s ease;
  flex-wrap: wrap;
}
.lead {
  color: var(--c-mid);
  font-weight: 560;
  margin-right: 2px;
}
.chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 2px 9px;
  background: var(--c-chip);
  border-radius: 20px;
}
.chip b {
  font-weight: 640;
}
</style>
