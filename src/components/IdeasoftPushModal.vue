<script setup lang="ts">
import { computed } from "vue";
import { useStore } from "../store";
import Icon from "./Icon.vue";

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
  <Transition name="push">
    <div v-if="preview" class="overlay" @click.self="store.closeIdeasoftPreview()">
      <div class="modal om-scroll" role="dialog" aria-label="IdeaSoft'a gönder">
        <header class="head">
          <div class="h-left">
            <div class="icon-badge"><Icon name="upload" :size="15" /></div>
            <div>
              <div class="h-title">IdeaSoft'a Gönder</div>
              <div class="h-sub">
                Ürün #{{ preview.id }} · {{ changedCount }}/{{ rows.length }} alan değişecek
              </div>
            </div>
          </div>
          <button class="close" title="Kapat" @click="store.closeIdeasoftPreview()">
            <Icon name="x" :size="16" :stroke-width="2.2" />
          </button>
        </header>

        <div class="body">
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
        </div>

        <footer class="foot">
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
        </footer>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 60;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: var(--overlay-bg);
  backdrop-filter: saturate(1.1) blur(3px);
}
.modal {
  width: 620px;
  max-width: 100%;
  max-height: 86vh;
  overflow-y: auto;
  background: var(--c-card);
  border: 1px solid var(--c-border);
  border-radius: 16px;
  box-shadow: 0 24px 60px var(--heavy-shadow);
  display: flex;
  flex-direction: column;
}
.head {
  position: sticky;
  top: 0;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 15px 18px;
  background: var(--c-card);
  border-bottom: 1px solid var(--c-border-soft);
}
.h-left {
  display: flex;
  align-items: center;
  gap: 11px;
}
.icon-badge {
  width: 30px;
  height: 30px;
  border-radius: 9px;
  background: var(--accent-tint);
  color: var(--accent);
  display: flex;
  align-items: center;
  justify-content: center;
}
.h-title {
  font-size: 14.5px;
  font-weight: 650;
  color: var(--c-text);
  letter-spacing: -0.01em;
}
.h-sub {
  font-size: 11.5px;
  color: var(--c-soft);
  margin-top: 1px;
}
.close {
  width: 30px;
  height: 30px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--c-soft);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}
.close:hover {
  background: var(--c-hover);
  color: var(--c-text);
}
.body {
  padding: 16px 18px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
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
.warn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  font-size: 11.5px;
  color: var(--warn-text);
  background: var(--warn-bg);
  border: 1px solid var(--warn-border);
  border-radius: 8px;
}
.foot {
  position: sticky;
  bottom: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 18px;
  background: var(--c-card);
  border-top: 1px solid var(--c-border-soft);
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
.spin {
  animation: spin 0.8s linear infinite;
}

/* Apple hissi: fade + hafif ölçek (GSC rehber modalıyla aynı dil) */
.push-enter-active,
.push-leave-active {
  transition: opacity 0.24s ease;
}
.push-enter-from,
.push-leave-to {
  opacity: 0;
}
.push-enter-active .modal,
.push-leave-active .modal {
  transition: transform 0.26s cubic-bezier(0.32, 0.72, 0, 1);
}
.push-enter-from .modal,
.push-leave-to .modal {
  transform: scale(0.96) translateY(8px);
}
@media (prefers-reduced-motion: reduce) {
  .push-enter-active .modal,
  .push-leave-active .modal {
    transition: none;
  }
  .push-enter-from .modal,
  .push-leave-to .modal {
    transform: none;
  }
  .field {
    animation: none;
  }
}
</style>
