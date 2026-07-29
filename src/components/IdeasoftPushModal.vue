<script setup lang="ts">
import { computed } from "vue";
import { useStore } from "../store";
import Icon from "./Icon.vue";
import ModalShell from "./ModalShell.vue";

const store = useStore();
const preview = computed(() => store.ideasoftPreview);

/** Gönderilecek gövdeyi "alan → (uzaktaki, yeni)" satırlarına düzleştirir. */
interface Row {
  label: string;
  before: string;
  after: string;
}
const rows = computed<Row[]>(() => {
  const p = preview.value;
  if (!p) return [];
  const r = p.remote;
  const local = p.local as Record<string, any>;
  const out: Row[] = [];
  const add = (label: string, before: string, after: unknown) => {
    if (after === undefined || after === null) return;
    out.push({ label, before: before ?? "", after: String(after) });
  };
  add("Sayfa Başlığı", r.page_title, local.pageTitle);
  add("Meta Açıklama", r.meta_description, local.metaDescription);
  add("Meta Anahtar Kelimeler", r.meta_keywords, local.metaKeywords);
  add("Arama Kelimeleri", r.search_keywords, local.searchKeywords);
  add("Hedef Kelime", r.target_keyword, local.targetKeyword);
  const d = local.detail as Record<string, any> | undefined;
  if (d?.details !== undefined) add("Ürün Açıklaması (HTML)", r.details, d.details);
  if (d?.extraDetails !== undefined) add("Teknik Tablo (HTML)", r.extra_details, d.extraDetails);
  return out;
});

function short(s: string, n = 220): string {
  const t = (s ?? "").trim();
  return t.length > n ? t.slice(0, n) + "…" : t || "—";
}
function changed(r: Row): boolean {
  return (r.before ?? "").trim() !== (r.after ?? "").trim();
}
const changedCount = computed(() => rows.value.filter(changed).length);
</script>

<template>
  <ModalShell
    :open="!!preview"
    label="IdeaSoft'a gönder"
    icon="upload"
    title="IdeaSoft'a Gönder"
    :sub="`Ürün #${preview?.id} · ${changedCount}/${rows.length} alan değişecek`"
    :width="620"
    scroll
    @close="store.closeIdeasoftPreview()"
  >
    <div v-if="!rows.length" class="empty">
      Gönderilecek içerik yok — önce üretim yapın.
    </div>

    <div v-for="(r, i) in rows" :key="i" class="field" :class="{ same: !changed(r) }">
      <div class="f-head">
        <span class="f-label">{{ r.label }}</span>
        <span v-if="!changed(r)" class="f-tag">değişiklik yok</span>
      </div>
      <div class="f-cols">
        <div class="col">
          <div class="c-title">IdeaSoft'ta şu an</div>
          <div class="c-val before">{{ short(r.before) }}</div>
        </div>
        <div class="arrow"><Icon name="external" :size="13" /></div>
        <div class="col">
          <div class="c-title">Gönderilecek</div>
          <div class="c-val after">{{ short(r.after) }}</div>
        </div>
      </div>
    </div>

    <div class="warn">
      <Icon name="alert" :size="13" />
      Bu işlem canlı mağazadaki ürünü günceller. Gönderilmeyen alanlara dokunulmaz.
    </div>

    <template #footer>
      <button class="ghost" @click="store.closeIdeasoftPreview()">Vazgeç</button>
      <div style="flex:1"></div>
      <button
        class="gen"
        :class="{ busy: store.ideasoftBusy }"
        :disabled="store.ideasoftBusy || !rows.length"
        @click="store.confirmIdeasoftPush()"
      >
        <Icon
          :name="store.ideasoftBusy ? 'loader' : 'upload'"
          :size="15"
          :class="{ spin: store.ideasoftBusy }"
        />
        {{ store.ideasoftBusy ? "Gönderiliyor…" : "Gönder" }}
      </button>
    </template>
  </ModalShell>
</template>

<style scoped>
.empty {
  font-size: 12.5px;
  color: var(--c-soft);
  text-align: center;
  padding: 20px;
}
.field {
  border: 1px solid var(--c-border-soft);
  border-radius: 10px;
  background: var(--c-list);
  padding: 10px 12px;
  animation: popIn 0.22s ease both;
}
.field.same {
  opacity: 0.55;
}
.f-head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}
.f-label {
  font-size: 12px;
  font-weight: 620;
  color: var(--c-mid);
}
.f-tag {
  font-size: 10px;
  color: var(--c-faint);
  background: var(--c-chip);
  border-radius: 999px;
  padding: 1px 7px;
}
.f-cols {
  display: flex;
  align-items: stretch;
  gap: 8px;
}
.col {
  flex: 1;
  min-width: 0;
}
.c-title {
  font-size: 10.5px;
  color: var(--c-faint);
  margin-bottom: 3px;
}
.c-val {
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--c-text);
  background: var(--c-input);
  border: 1px solid var(--c-border-soft);
  border-radius: 7px;
  padding: 7px 9px;
  overflow-wrap: anywhere;
  max-height: 110px;
  overflow-y: auto;
}
.c-val.before {
  color: var(--c-soft);
}
.c-val.after {
  border-color: var(--accent);
}
.arrow {
  display: flex;
  align-items: center;
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
}
.ghost:hover {
  background: var(--c-hover);
}
.gen {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 38px;
  padding: 0 16px;
  border: none;
  border-radius: 9px;
  background: var(--accent);
  color: #fff;
  font-size: 13px;
  font-weight: 590;
  cursor: pointer;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
}
.gen:hover {
  filter: brightness(1.05);
}
.gen.busy {
  opacity: 0.75;
  cursor: default;
}
.gen:disabled:not(.busy) {
  opacity: 0.45;
  cursor: not-allowed;
}
/* Modal iskeleti ve animasyonu artık ModalShell.vue'da. Buradaki `push` animasyonu
   UpdateModal'daki `upd` ile BİREBİR aynıydı — yalnızca adı farklıydı. */
</style>
