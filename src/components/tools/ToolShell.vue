<script setup lang="ts">
/**
 * SEO araç ekranlarının ortak kabuğu: kaydırma alanı, hata bandı ve iki boş durum.
 *
 * Neden: beş araç ekranı da aynı üç soruyu cevaplamak zorunda — "analiz hiç çalıştı mı?",
 * "çalıştıysa bu araç için satır var mı?", "hata mı oldu?". Her ekrana ayrı yazılsaydı beş
 * kopya olurdu; üstelik en kritik olanı (analiz yok) yanlış yazmak boş ekran demek — bu
 * uygulamada v0.5.9'da yaşanmış bir saha hatası (kullanıcı "ekran bomboş" diye bildirdi).
 */
import { useStore } from "../../store";
import Icon from "../Icon.vue";

defineProps<{
  /** Analiz var ama BU araç için satır yok. */
  empty: boolean;
  /** O durumda ne yazacağı — araca özel, çünkü "satır yok" her araçta farklı şey demek. */
  emptyText: string;
}>();

const store = useStore();
</script>

<template>
  <div class="page om-scroll">
    <!-- Hata kalıcı gösterilir: toast kaybolur, kullanıcı sebebi göremez. -->
    <div v-if="store.opportunityError" class="err">
      <Icon name="alert" :size="14" />
      <span>{{ store.opportunityError }}</span>
    </div>

    <!-- Analiz hiç çalıştırılmamış: sessiz boş ekran bırakma, nereye gideceğini söyle. -->
    <div v-if="!store.opportunity && !store.opportunityBusy" class="clean big">
      <Icon name="search" :size="26" :stroke-width="1.6" style="color: var(--c-soft)" />
      <p class="c-title">Henüz analiz çalıştırılmadı</p>
      <p class="c-sub">
        Bu araç Google Search Console verisiyle çalışır.
        <a class="link" @click="store.page = 'overview'">Genel Bakış</a>'tan analizi bir kez
        çalıştırın — tüm araçlar aynı veriyi kullanır, tekrar beklemezsiniz.
      </p>
    </div>

    <div v-else-if="store.opportunityBusy && !store.opportunity" class="clean big">
      <Icon name="loader" :size="22" :stroke-width="2.2" class="spin" style="color: var(--accent)" />
      <p class="c-sub">Search Console verisi alınıyor…</p>
    </div>

    <div v-else-if="empty" class="clean">
      <Icon name="check" :size="16" :stroke-width="2.4" style="color: var(--green)" />
      {{ emptyText }}
    </div>

    <slot v-else />
  </div>
</template>

<style scoped>
.page {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 18px 22px 28px;
}
/* Bölmeden önceki Fırsatlar ekranıyla birebir aynı — görsel bir sapma olmasın. */
.err {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 10px 12px;
  margin-bottom: 14px;
  font-size: 12px;
  line-height: 1.5;
  color: var(--warn-text);
  background: var(--warn-bg);
  border: 1px solid var(--warn-border);
  border-radius: 9px;
}
.clean {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 16px 18px;
  font-size: 12.5px;
  color: var(--c-mid);
  background: var(--ok-soft-bg);
  border-radius: 11px;
}
/* "Analiz hiç çalışmadı" hâli: yeşil kutu değil, sakin bir boş ekran. */
.clean.big {
  flex-direction: column;
  justify-content: center;
  gap: 6px;
  padding: 64px 26px;
  background: transparent;
  color: var(--c-soft);
  text-align: center;
}
.c-title {
  margin: 6px 0 0;
  font-size: 14px;
  font-weight: 620;
  color: var(--c-text);
}
.c-sub {
  margin: 0;
  max-width: 420px;
  line-height: 1.6;
}
</style>
