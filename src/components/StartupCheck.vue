<script setup lang="ts">
/**
 * Açılış bakım kontrolü — uygulama açılırken neyin bayat olduğunu söyler.
 *
 * 🔴 **Her şey yolundaysa KENDİLİĞİNDEN KAPANIR.** Bunu bir "hoş geldiniz" ekranına
 * çevirmek, her açılışta kapatılacak bir engel eklemek olurdu; kullanıcı üçüncü açılışta
 * içeriğini okumadan kapatmayı öğrenir ve panel gerçekten bir şey söylediğinde de görmez.
 * Bir uyarı ancak **nadir** olduğunda uyarıdır.
 *
 * ⚠️ Sihirbaz (`SetupWizard`) açıkken hiç gösterilmiyor: taze kurulumda her şey zaten bayat
 * ve iki katman üst üste binerdi.
 *
 * ⚠️ Kurulum sırasında değil, **kurulmuş bir sistemin bakımı** için. Liste ve gerekçeler
 * `HealthPanel`de; burada yalnızca ne zaman görüneceği kararı var.
 */
import { ref } from "vue";
import { useStore } from "../store";
import Icon from "./Icon.vue";
import HealthPanel from "./HealthPanel.vue";

const store = useStore();
const acik = ref(true);
const temiz = ref(false);

/**
 * Sorun yoksa "tamam" anını gösterip kapanır.
 *
 * ⚠️ Süre 1,6 sn: kontroller ~0,7 sn sürüyor, yani panel toplam ~2,3 sn görünüyor. Daha
 * kısası göz kırpması gibi olur ve kullanıcı bir şeyin olup olmadığından emin olamaz;
 * daha uzunu her açılışta bekleyen bir engele dönüşür.
 *
 * ⚠️ Bu süre ekran görüntüsüyle **doğrulanamadı** — araç turu pencereden uzun. Doğrulanan
 * şey davranış: sorun varken panel kalıyor, temizken kapanıyor.
 */
const KAPANMA_MS = 1600;

function bitti(sorunsuz: boolean) {
  temiz.value = sorunsuz;
  if (sorunsuz) setTimeout(() => (acik.value = false), KAPANMA_MS);
}
</script>

<template>
  <Transition name="hp-fade">
    <div v-if="acik && !store.setupOpen" class="kutu">
      <div class="bas">
        <Icon :name="temiz ? 'badgeCheck' : 'refresh'" :size="15"
              :style="{ color: temiz ? 'var(--green)' : 'var(--accent)' }" />
        <span>{{ temiz ? "Her şey güncel" : "Bakım kontrolü" }}</span>
        <button class="kapat" title="Kapat" @click="acik = false">
          <Icon name="x" :size="14" />
        </button>
      </div>
      <HealthPanel compact @done="bitti" />
    </div>
  </Transition>
</template>

<style scoped>
/* Sağ üstte, içeriğin üstünde durmayan bir kart: engel değil bildirim.
   Mevcut token'lar; yeni gölge/renk icat edilmiyor. */
.kutu {
  position: fixed;
  top: 62px;
  right: 20px;
  width: 380px;
  z-index: 60;
  background: var(--c-card);
  border: 1px solid var(--c-border);
  border-radius: 12px;
  box-shadow: var(--heavy-shadow);
  padding: 12px 14px;
}
.bas {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--c-mid);
  margin-bottom: 10px;
}
.kapat {
  margin-left: auto;
  background: none;
  border: 0;
  color: var(--c-faint);
  cursor: pointer;
  padding: 2px;
  display: inline-flex;
}
.hp-fade-enter-active,
.hp-fade-leave-active {
  transition: opacity 0.28s cubic-bezier(0.32, 0.72, 0, 1),
    transform 0.28s cubic-bezier(0.32, 0.72, 0, 1);
}
.hp-fade-enter-from,
.hp-fade-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}
@media (prefers-reduced-motion: reduce) {
  .hp-fade-enter-active,
  .hp-fade-leave-active {
    transition: none;
  }
}
</style>
