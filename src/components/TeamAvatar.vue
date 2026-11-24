<script setup lang="ts">
import { computed, ref, watch } from "vue";

const props = defineProps<{ name: string; source?: string | null }>();
const broken = ref(false);
watch(() => props.source, () => { broken.value = false; });
const initials = computed(() => props.name.trim().split(/\s+/).slice(0, 2)
  .map((word) => Array.from(word)[0] ?? "").join("").toLocaleUpperCase("tr-TR") || "?");
</script>

<template>
  <span class="team-avatar" aria-hidden="true">
    <img v-if="source && !broken" :key="source" :src="source" alt="" draggable="false" @error="broken = true" />
    <span v-else>{{ initials }}</span>
  </span>
</template>

<style scoped>
.team-avatar {
  flex: none;
  display: grid;
  place-items: center;
  width: var(--team-avatar-size, 34px);
  height: var(--team-avatar-size, 34px);
  overflow: hidden;
  border: 1px solid var(--c-border-soft);
  border-radius: var(--team-avatar-radius, 10px);
  background: var(--accent-tint);
  color: var(--badge-bekliyor-c);
  font-size: var(--team-avatar-font, 12px);
  font-weight: 650;
}
.team-avatar img { display: block; width: 100%; height: 100%; object-fit: cover; }
</style>
