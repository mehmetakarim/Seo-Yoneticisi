<script setup lang="ts">
/**
 * Ürün detayındaki kartların ortak iskeleti (çerçeve + başlık şeridi + rozet + eylem satırı).
 *
 * Neden: bu iskelet daha önce 4 bileşende ayrı ayrı yazılıydı ve zamanla birbirinden sapmıştı
 * (ikon 26 vs 30px, başlık 13.5 vs 15px, rozet yarıçapı 20 vs 999px…). "Kart 2'de gölge var,
 * Kart 1'de yok" hatası da bundan çıkmıştı. Artık tek yer: burayı değiştiren hepsini değiştirir.
 *
 * Geometri, 4 karttan 2'sinin (Teknik Tablo + Görseller) hâlihazırda kullandığı **kompakt**
 * ölçüde birleştirildi — en az görsel değişiklik, en güncel tasarım dili.
 */
import Icon from "./Icon.vue";

defineProps<{
  /** Icon.vue'daki ikon adı */
  icon: string;
  title: string;
  /** Başlığın altındaki açıklama satırı (opsiyonel) */
  sub?: string;
  /** Rozet metni; verilmezse rozet çizilmez */
  badgeLabel?: string;
  /** Çağıranın mevcut `badgeStyle` computed'ı: { background, color } */
  badgeStyle?: Record<string, string>;
  /** Gövde dikey akış olsun (kendi aralarında 12px boşluklu bloklar) */
  stack?: boolean;
  /** Gövde dolgusuz olsun — içerik kenara dayansın (ör. tam genişlik uyarı şeridi) */
  bare?: boolean;
}>();
</script>

<template>
  <div class="card">
    <div class="card-head">
      <div class="head-left">
        <div class="icon-badge"><Icon :name="icon" :size="15" /></div>
        <div>
          <div class="head-title">{{ title }}</div>
          <div v-if="sub" class="head-sub">{{ sub }}</div>
        </div>
      </div>
      <span v-if="badgeLabel" class="status" :style="badgeStyle">
        <span class="sdot" :style="{ background: badgeStyle?.color }"></span>{{ badgeLabel }}
      </span>
    </div>

    <div class="card-body" :class="{ stack, bare }">
      <slot />
    </div>

    <div v-if="$slots.actions" class="card-actions">
      <slot name="actions" />
    </div>
  </div>
</template>

<style scoped>
.card {
  margin-top: 16px;
  border: 1px solid var(--c-border-soft);
  border-radius: 13px;
  background: var(--c-card);
  overflow: hidden;
}
.card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 13px 16px;
  border-bottom: 1px solid var(--c-border-soft);
}
.head-left {
  display: flex;
  align-items: center;
  gap: 10px;
}
.icon-badge {
  width: 26px;
  height: 26px;
  border-radius: 8px;
  background: var(--accent-tint);
  color: var(--accent);
  display: flex;
  align-items: center;
  justify-content: center;
  flex: none;
}
.head-title {
  font-size: 13.5px;
  font-weight: 640;
  color: var(--c-text);
}
.head-sub {
  font-size: 11px;
  color: var(--c-soft);
  margin-top: 1px;
}
.status {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 3px 9px;
  border-radius: 999px;
  font-size: 11.5px;
  font-weight: 600;
  white-space: nowrap;
  flex: none;
}
.sdot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}
.card-body {
  padding: 16px;
}
.card-body.stack {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.card-body.bare {
  padding: 0;
}
.card-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
  padding: 14px 16px;
  border-top: 1px solid var(--c-border-soft);
  background: var(--c-input);
}
</style>
