<script setup lang="ts">
/**
 * Modallerin ortak iskeleti (arka plan + başlık şeridi + gövde + eylem şeridi + animasyon).
 *
 * Neden: bu iskelet dört yerde ayrı yazılıydı (UpdateModal, IdeasoftPushModal ve Fırsatlar'ın
 * iki modali) ve zaten sapmıştı — üstelik iki farklı isimlendirmeyle: `.head/.body` ve
 * `.m-head/.m-body`. Ölçülen sapmalar:
 *   · arka plan `z-index`: 60 / 70 / 70
 *   · animasyon: aynı kural seti İKİ AYRI ADLA kopyalanmıştı (`upd` ve `push`), üçüncüsü ise
 *     zayıf varyanttı (yalnızca soluma, 0.18s, kart ölçeklenmiyor). UpdateModal'daki
 *     "diğer modallarla aynı animasyon dili" yorumu bu yüzden kısmen yanlıştı.
 *   · kapat düğmesi: ikisinde var, ikisinde yok
 *
 * Faz 3'te `SeoCard.vue` kartlar için aynı işi yapmıştı ve prop şekli bilinçli olarak ondan
 * kopyalandı (icon/title/sub + slotlar): iki iskelet birbirine benzesin, yeni bir dil doğmasın.
 *
 * Birleştirme **baskın değere** yapıldı; animasyonda ise en özenli olan seçildi (soluma +
 * ölçekleme + `prefers-reduced-motion` koruması) — kaybedilen bir incelik olmasın.
 */
import Icon from "./Icon.vue";

withDefaults(
  defineProps<{
    /** Modal açık mı — `Transition` ve `v-if` iskeletin içinde, çağıran sarmalamaz. */
    open: boolean;
    /** Ekran okuyucu için ad (`aria-label`). */
    label: string;
    title: string;
    /** Icon.vue'daki ikon adı; verilmezse rozet çizilmez. */
    icon?: string;
    /** Başlığın altındaki tek satır. Zengin metin gerekiyorsa `sub` slotunu kullanın. */
    sub?: string;
    /** Kapat düğmesi. Kapatması sakıncalı bir işlem sürüyorsa çağıran `false` verir. */
    closable?: boolean;
    /** Genişlik (px). Ölçülen mevcut değerler: 460 / 480 / 620. */
    width?: number;
    /** Gövde uzunsa: modal kendi içinde kayar, başlık ve eylem şeridi yapışık kalır. */
    scroll?: boolean;
  }>(),
  { closable: true, width: 480, scroll: false },
);

/**
 * ⚠️ Meşguliyet kilidi **bilinçli olarak burada DEĞİL.** UpdateModal indirme sırasında
 * kapanmamalı, canonical modali yazma sırasında kapanmamalı — ama koşullar farklı. İskelete
 * gömülseydi her modal aynı kilit modelini kabul etmek zorunda kalırdı; şimdi çağıran
 * `closable`ı kendi durumundan hesaplıyor.
 */
const emit = defineEmits<{ close: [] }>();
</script>

<template>
  <Transition name="upd">
    <div v-if="open" class="overlay" @click.self="closable && emit('close')">
      <div
        class="modal"
        :class="{ scroll }"
        :style="{ width: width + 'px' }"
        role="dialog"
        :aria-label="label"
      >
        <header class="m-head">
          <div class="m-left">
            <div v-if="icon" class="icon-badge"><Icon :name="icon" :size="15" /></div>
            <div>
              <div class="m-title">{{ title }}</div>
              <div v-if="sub || $slots.sub" class="m-sub">
                <slot name="sub">{{ sub }}</slot>
              </div>
            </div>
          </div>
          <button v-if="closable" class="m-close" title="Kapat" @click="emit('close')">
            <Icon name="x" :size="16" :stroke-width="2.2" />
          </button>
        </header>

        <div class="m-body"><slot /></div>

        <footer v-if="$slots.footer" class="m-foot"><slot name="footer" /></footer>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
/* `.overlay` global (styles.css) — üç bileşende aynıydı, orada birleşti. */
.modal {
  max-width: 100%;
  background: var(--c-card);
  border: 1px solid var(--c-border);
  border-radius: 16px;
  box-shadow: 0 24px 60px var(--heavy-shadow);
  display: flex;
  flex-direction: column;
}
/* Uzun gövde: modal kayar, şeritler yapışık kalır (IdeaSoft gönderim modalinin deseni). */
.modal.scroll {
  max-height: 86vh;
  overflow-y: auto;
}
.modal.scroll .m-head {
  position: sticky;
  top: 0;
  z-index: 1;
  background: var(--c-card);
}
.modal.scroll .m-foot {
  position: sticky;
  bottom: 0;
  background: var(--c-card);
}

.m-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 15px 18px;
  border-bottom: 1px solid var(--c-border-soft);
}
.m-left {
  display: flex;
  align-items: center;
  gap: 11px;
  min-width: 0;
}
.m-title {
  font-size: 14.5px;
  font-weight: 650;
  color: var(--c-text);
  letter-spacing: -0.01em;
}
.m-sub {
  font-size: 11.5px;
  color: var(--c-soft);
  margin-top: 1px;
  overflow-wrap: anywhere;
}
.m-sub :deep(b) {
  color: var(--accent);
}
.m-close {
  width: 30px;
  height: 30px;
  flex: none;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--c-soft);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}
.m-close:hover {
  background: var(--c-hover);
  color: var(--c-text);
}
.m-body {
  padding: 16px 18px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.m-foot {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 18px;
  border-top: 1px solid var(--c-border-soft);
}

/* Animasyon: üç modalin en özenlisi kazandı. Arka plan soluyor, kart hafifçe büyüyerek
   yerine oturuyor. Hareketi azaltılmış sistemlerde kart animasyonu kapanır. */
.upd-enter-active,
.upd-leave-active {
  transition: opacity 0.24s ease;
}
.upd-enter-from,
.upd-leave-to {
  opacity: 0;
}
.upd-enter-active .modal,
.upd-leave-active .modal {
  transition: transform 0.26s cubic-bezier(0.32, 0.72, 0, 1);
}
.upd-enter-from .modal,
.upd-leave-to .modal {
  transform: scale(0.96) translateY(8px);
}
@media (prefers-reduced-motion: reduce) {
  .upd-enter-active .modal,
  .upd-leave-active .modal {
    transition: none;
  }
  .upd-enter-from .modal,
  .upd-leave-to .modal {
    transform: none;
  }
}
</style>
