<script setup lang="ts">
import { computed } from "vue";
import { useStore } from "../store";
import type { ImageCheck } from "../types";
import Icon from "./Icon.vue";
import SeoCard from "./SeoCard.vue";

const props = defineProps<{ gallery: string[]; imageCount: number }>();
const store = useStore();

const MIN = 3;
const checks = computed(() => store.imageCheck ?? []);
const checking = computed(() => store.imageChecking);

function checkFor(url: string): ImageCheck | undefined {
  return checks.value.find((c) => c.url === url);
}

// Genel durum → mevcut badge renk sistemi
type Status = "eksik" | "hatali" | "uygun" | "bekliyor";
const status = computed<Status>(() => {
  if (props.imageCount < MIN) return "eksik";
  if (!checks.value.length) return "bekliyor";
  return checks.value.every((c) => c.ok) ? "uygun" : "hatali";
});

const STATUS_LABEL: Record<Status, string> = {
  eksik: "Eksik",
  hatali: "Sorunlu",
  uygun: "Uygun",
  bekliyor: "Kontrol ediliyor",
};

const badgeStyle = computed(() => {
  const s = status.value === "bekliyor" ? "bekliyor" : status.value;
  return { background: `var(--badge-${s}-bg)`, color: `var(--badge-${s}-c)` };
});

function thumbState(url: string): { icon: string; color: string; text: string; spin?: boolean } {
  const c = checkFor(url);
  if (!c) {
    return checking.value
      ? { icon: "loader", color: "var(--c-soft)", text: "…", spin: true }
      : { icon: "image", color: "var(--c-faint)", text: "" };
  }
  if (c.error) return { icon: "x", color: "var(--red)", text: "hata" };
  if (c.ok) return { icon: "check", color: "var(--green)", text: `${c.width}×${c.height}` };
  if (!c.is_square) return { icon: "x", color: "var(--red)", text: `${c.width}×${c.height} · kare değil` };
  return { icon: "x", color: "var(--amber)", text: `${c.width}×${c.height} · düşük` };
}
</script>

<template>
  <SeoCard
    icon="image"
    title="Ürün Görselleri"
    :sub="`${imageCount}/4 görsel · minimum ${MIN} · 1:1 kare, ≥1000px`"
    :badge-label="STATUS_LABEL[status]"
    :badge-style="badgeStyle"
    bare
  >
    <div v-if="imageCount < MIN" class="warn">
      <Icon name="alert" :size="13" />
      En az {{ MIN }} görsel gerekli — açıklama üretimi için şart. Şu an {{ imageCount }} görsel var.
    </div>

    <div class="thumbs">
      <div v-for="(url, i) in gallery" :key="i" class="thumb">
        <img :src="url" alt="" loading="lazy" />
        <div class="thumb-badge" :style="{ color: thumbState(url).color }">
          <Icon
            :name="thumbState(url).icon"
            :size="12"
            :stroke-width="2.4"
            :class="{ spin: thumbState(url).spin }"
          />
        </div>
        <div class="thumb-dim" v-if="thumbState(url).text">{{ thumbState(url).text }}</div>
      </div>
      <!-- eksik slotlar -->
      <div v-for="n in Math.max(0, MIN - gallery.length)" :key="'e' + n" class="thumb empty">
        <Icon name="upload" :size="16" :stroke-width="1.7" />
        <div class="thumb-dim">eksik</div>
      </div>
    </div>
  </SeoCard>
</template>

<style scoped>
/* Yalnızca yerel fark; gerisi styles.css'teki `.warn` temelinden gelir. */
.warn {
  margin: 12px 16px 0;
}
.thumbs {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  padding: 14px 16px;
}
.thumb {
  position: relative;
  width: 84px;
  height: 84px;
  border-radius: 9px;
  overflow: hidden;
  border: 1px solid var(--c-border);
  background: var(--c-list);
  display: flex;
  align-items: center;
  justify-content: center;
  animation: popIn 0.25s ease both;
}
.thumb img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  background: #fff;
}
.thumb.empty {
  flex-direction: column;
  gap: 3px;
  color: var(--c-faint);
  border-style: dashed;
}
.thumb-badge {
  position: absolute;
  top: 4px;
  right: 4px;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--c-card);
  box-shadow: 0 1px 3px var(--heavy-shadow);
  display: flex;
  align-items: center;
  justify-content: center;
}
.thumb-dim {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  font-size: 9.5px;
  font-weight: 560;
  text-align: center;
  padding: 2px 3px;
  color: #fff;
  background: rgba(0, 0, 0, 0.55);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.thumb.empty .thumb-dim {
  position: static;
  background: transparent;
  color: var(--c-faint);
  padding: 0;
}
</style>
