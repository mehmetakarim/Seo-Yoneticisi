<script setup lang="ts">
/**
 * Seans özeti — **sakin bir bilanço** (Faz S).
 *
 * 🚫 Kutlama değil. Yol haritasının açık yasağı: XP yok · lig yok · seri cezası yok ·
 * konfeti yok · "Harikasın! 🔥" yok. Ertesi sabah yine açılacak bir ekranda coşku yorar;
 * burada yalnızca ne olduğu yazıyor.
 */
import { computed } from "vue";
import { useStore } from "../store";
import ModalShell from "./ModalShell.vue";

const store = useStore();
const o = computed(() => store.sessionSummary);

const LABEL: Record<string, string> = {
  urgent: "Acil",
  leverage: "Yüksek kaldıraç",
  leak: "Kaçak trafik",
  review: "Sonuç kontrolü",
  upkeep: "Bakım",
};

/** Seansın neden bittiği — kullanıcı ne olduğunu bilsin. */
const neden = computed(() => {
  switch (o.value?.ended_reason) {
    case "queue_empty":
      return "Kuyrukta kilitlenecek iş kalmadı.";
    case "time_up":
      return "Planlanan süre doldu.";
    default:
      return "Seansı siz bitirdiniz.";
  }
});
</script>

<template>
  <ModalShell
    :open="!!o"
    label="Seans özeti"
    icon="clock"
    title="Seans bitti"
    :sub="neden"
    :width="440"
    @close="store.sessionSummary = null"
  >
    <div class="ozet">
      <div class="satir">
        <span class="k">Bitirilen iş</span>
        <span class="v">{{ o?.done_count ?? 0 }}</span>
      </div>
      <div v-if="o?.skipped_count" class="satir">
        <span class="k">Atlanan</span>
        <span class="v">{{ o.skipped_count }}</span>
      </div>
      <div class="satir">
        <span class="k">Süre</span>
        <span class="v">{{ o?.minutes ?? 0 }} dk</span>
      </div>
    </div>

    <div v-if="o?.buckets.length" class="kovalar">
      <span v-for="[b, n] in o.buckets" :key="b" class="kova">
        {{ LABEL[b] ?? b }} <b>{{ n }}</b>
      </span>
    </div>

    <p class="not">
      Bu seansta ölçülen süreler kuyruk tahminlerini besliyor. Bir kovada yeterli ölçüm
      birikince süre <b>tahmin olmaktan çıkıp ölçüme</b> dönüyor.
    </p>

    <template #footer>
      <button class="ghost" @click="store.sessionSummary = null">Kapat</button>
      <div style="flex: 1"></div>
      <button class="ana" @click="store.sessionSummary = null; store.startSession()">
        Yeni seans
      </button>
    </template>
  </ModalShell>
</template>

<style scoped>
.ozet {
  border: 1px solid var(--c-border-soft);
  border-radius: 9px;
  overflow: hidden;
}
.satir {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 9px 12px;
  font-size: 12.5px;
  border-bottom: 1px solid var(--c-border-soft);
}
.satir:last-child {
  border-bottom: 0;
}
.k {
  color: var(--c-soft);
}
.v {
  font-weight: 620;
  color: var(--c-text);
  font-variant-numeric: tabular-nums;
}
.kovalar {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 12px;
}
.kova {
  padding: 3px 9px;
  border-radius: 20px;
  background: var(--c-chip);
  color: var(--c-soft);
  font-size: 11px;
}
.kova b {
  color: var(--c-mid);
  font-variant-numeric: tabular-nums;
}
.not {
  margin: 14px 0 0;
  font-size: 11.5px;
  line-height: 1.55;
  color: var(--c-faint);
}
.ghost {
  height: 38px;
  padding: 0 14px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 12.5px;
  font-weight: 560;
  cursor: pointer;
  font-family: inherit;
}
.ghost:hover {
  background: var(--c-hover);
}
.ana {
  height: 38px;
  padding: 0 16px;
  border: none;
  border-radius: 9px;
  background: var(--accent);
  color: #fff;
  font-size: 13px;
  font-weight: 590;
  cursor: pointer;
  font-family: inherit;
}
.ana:hover {
  filter: brightness(1.06);
}
</style>
