<script setup lang="ts">
/**
 * Bir analiz bölümünün ortak kabuğu: kart çerçevesi + başlık (sayı ve özet rozetiyle) +
 * açıklama satırı + içerik.
 *
 * Neden: beş analiz (fırsatlar, yükselmeye yakın, yarışan sayfalar, düşüşte olanlar, satışta
 * olmayanlar) bu kabuğu birebir tekrar ediyordu. Her biri kendi ekranına taşınırken kabuğun
 * beş kopyaya çıkmaması için önce burada toplandı.
 *
 * ⚠️ Eski sınıf adları `inv-*` idi — ilk yazılan "Google'da görünmeyenler" bölümünden miras
 * kalmış ve sonradan beş bölümde birden kullanılmıştı; yani ad artık içeriği anlatmıyordu.
 * Burada anlamlı adlara geçildi.
 */
defineProps<{
  title: string;
  /** Başlıktaki parantez içi sayı. 0 anlamlı bir değer olduğu için opsiyonel değil. */
  count: number;
  /**
   * Başlığın yanındaki vurgulu özet ("907 tıklama kaybı" gibi). Bölümün *bedelini* tek
   * bakışta göstermek için; her bölümde olması gerekmiyor.
   */
  summary?: string;
}>();
</script>

<template>
  <div class="card sec">
    <header class="sec-head">
      <div class="sec-text">
        <div class="sec-title">
          {{ title }} ({{ count }})
          <span v-if="summary" class="sec-sum">{{ summary }}</span>
        </div>
        <!-- Açıklama zengin metin içeriyor (<b>, bağlantı) → prop değil slot. -->
        <div v-if="$slots.note" class="sec-note"><slot name="note" /></div>
      </div>
      <!-- Başlığın sağındaki eylem (ör. "Katalogla eşleştir"). Yalnızca bir bölümde var
           ama kabuğun parçası: ileride başka bölüm de isterse hizası hazır olsun. -->
      <slot name="action" />
    </header>
    <slot />
  </div>
</template>

<style scoped>
.sec-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 14px;
  padding: 13px 16px;
  border-bottom: 1px solid var(--c-border-soft);
}
.sec-text {
  min-width: 0;
}
.sec-title {
  font-size: 13.5px;
  font-weight: 640;
  color: var(--c-text);
}
.sec-sum {
  margin-left: 8px;
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--warn-bg);
  color: var(--warn-text);
  font-size: 11px;
  font-weight: 640;
  font-variant-numeric: tabular-nums;
}
.sec-note {
  font-size: 11.5px;
  color: var(--c-soft);
  margin-top: 2px;
  line-height: 1.5;
  max-width: 640px;
}
</style>
