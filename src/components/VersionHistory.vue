<script setup lang="ts">
/**
 * "Önceki sürümler" listesi — meta, açıklama ve teknik tablo kartlarının üçü de kullanır.
 *
 * Neden ortak bileşen: bu blok önce yalnızca teknik tablodaydı. Meta ve açıklamaya kopyalamak
 * üçe katlamak olurdu — projede tam bu hatanın bedeli iki kez ödendi (kart iskeleti 4 bileşende
 * birbirinden sapmıştı; Gemini hata sınıflandırması 4 yerde kopyalanıp dördü de yanlıştı).
 *
 * Özetin ne olduğu türe göre değişir (teknik tabloda "3 satır · 2 grup", metada başlık,
 * açıklamada kelime sayısı) — bu yüzden çağıran tarafta hazırlanıp `summary` olarak verilir.
 *
 * **Yalnızca paneli render eder, açma bağlantısını değil.** Teknik tabloda bağlantı bir bilgi
 * satırının içinde ("· kaynak metni düzenle · önceki sürümler (2)"), metada ise kendi satırında.
 * Bağlantıyı da bileşene almak o kartın düzenini bozardı; açık/kapalı durumu çağıran tutar.
 */
import Icon from "./Icon.vue";

export interface VersionItem {
  at: string;
  /** Sürümü tanımaya yarayan kısa metin — çağıran hazırlar. */
  summary: string;
  /** Üreten model (boşsa gösterilmez). */
  model?: string;
}

defineProps<{
  items: VersionItem[];
  /** Yardım satırındaki tür adı, ör. "tablo" / "meta" / "açıklama". */
  noun: string;
}>();

const emit = defineEmits<{ restore: [number] }>();

/** `gemini-3.6-flash` → `3.6 Flash` (ModelTag ile aynı kısaltma mantığı). */
function shortModel(m?: string): string {
  if (!m) return "";
  if (m.startsWith("gemma")) return m.replace(/^gemma-/, "Gemma ").replace(/-it$/, "");
  return m
    .replace(/^gemini-/, "")
    .replace(/-/g, " ")
    .replace(/\b\w/g, (c) => c.toUpperCase());
}

const fmtDate = (s: string) => s.replace("T", " ").slice(0, 16);
</script>

<template>
  <div class="hist">
      <div class="hist-head">
        <Icon name="refresh" :size="12" />
        Yeniden üretimden önceki {{ noun }} hâlleri — geri yüklemek mevcut hâli de saklar
      </div>
      <div v-for="(v, i) in items" :key="i" class="hist-row">
        <span class="hist-at">{{ fmtDate(v.at) }}</span>
        <span class="hist-meta">{{ v.summary }}</span>
        <span v-if="v.model" class="hist-model">{{ shortModel(v.model) }}</span>
        <button class="hist-btn" @click="emit('restore', i)">Geri yükle</button>
      </div>
  </div>
</template>

<style scoped>
.hist {
  border: 1px solid var(--c-border-soft);
  border-radius: 10px;
  background: var(--c-list);
  overflow: hidden;
  animation: popIn 0.22s ease both;
}
.hist-head {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--c-soft);
  background: var(--c-chip);
  padding: 7px 12px;
}
.hist-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--c-border-soft);
}
.hist-row:last-child {
  border-bottom: 0;
}
.hist-at {
  font-size: 12px;
  color: var(--c-text);
  font-variant-numeric: tabular-nums;
  flex: none;
}
/* Kalan alanı kaplar → model ve buton özet uzunluğundan bağımsız olarak hep aynı
   yerde durur. (flex:1 verilmeseydi kısa özetli satırda model sola kayıyordu.) */
.hist-meta {
  flex: 1;
  min-width: 0;
  font-size: 11.5px;
  color: var(--c-soft);
  /* uzun başlıklar satırı taşırmasın */
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.hist-model {
  flex: none;
  font-size: 10.5px;
  color: var(--c-faint);
  font-variant-numeric: tabular-nums;
}
.hist-btn {
  flex: none;
  height: 26px;
  padding: 0 10px;
  border: 1px solid var(--c-border);
  border-radius: 7px;
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 11.5px;
  font-weight: 560;
  cursor: pointer;
}
.hist-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
}
</style>
