<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useStore } from "../store";
import Icon from "./Icon.vue";

const props = defineProps<{ open: boolean; keyword: string }>();
const emit = defineEmits<{ close: []; pick: [string] }>();

const store = useStore();
const seed = ref("");

// Panel açılınca tohum kelimeyi akıllıca doldur (onaylı kelime → kategori → ürün adı ilk 4 sözcük).
watch(
  () => props.open,
  (o) => {
    if (!o) return;
    if (store.research) {
      seed.value = store.research.seed;
      return;
    }
    const d = store.detail;
    seed.value =
      props.keyword?.trim() ||
      d?.category?.trim() ||
      (d?.name ? d.name.split(/\s+/).slice(0, 4).join(" ") : "");
  },
);

const research = computed(() => store.research);
const candidates = computed(() => research.value?.target_candidates ?? []);
const gscQueries = computed(() => research.value?.gsc_queries ?? []);
const trends = computed(() => research.value?.trends ?? []);
const domain = computed(() => research.value?.domain ?? null);
const notes = computed(() => research.value?.notes ?? []);

function fmtNum(n: number): string {
  return n.toLocaleString("tr-TR");
}

function runResearch() {
  const s = seed.value.trim();
  if (!s) {
    store.toast("Önce bir tohum kelime girin", "info");
    return;
  }
  store.runResearch(s);
}

function pick(kw: string) {
  emit("pick", kw);
  store.toast(`Hedef kelime: "${kw}"`, "ok");
}

// Zorluk → mevcut rozet renk sistemi (0-100)
function diffBadge(d: number): string {
  if (d <= 20) return "uygun";
  if (d <= 50) return "hatali";
  return "eksik";
}
function diffLabel(d: number): string {
  if (d <= 20) return "kolay";
  if (d <= 50) return "orta";
  return "zor";
}
function fmtVol(v: number): string {
  if (v >= 1000) {
    const k = v / 1000;
    return (Number.isInteger(k) ? k.toString() : k.toFixed(1)) + "B";
  }
  return v.toString();
}

const onEsc = (e: KeyboardEvent) => {
  if (e.key === "Escape" && props.open) emit("close");
};
watch(
  () => props.open,
  (o) => {
    if (o) window.addEventListener("keydown", onEsc);
    else window.removeEventListener("keydown", onEsc);
  },
);
</script>

<template>
  <Transition name="research">
    <div v-if="open" class="overlay" @click.self="emit('close')">
      <aside class="drawer om-scroll" role="dialog" aria-label="SEO Araştır">
        <!-- Başlık -->
        <header class="head">
          <div class="head-l">
            <div class="icon-badge"><Icon name="search" :size="15" /></div>
            <div>
              <div class="h-title">SEO Araştır</div>
              <div class="h-sub">Gerçek verilerle hedef kelimeni seç</div>
            </div>
          </div>
          <button class="close" title="Kapat (Esc)" @click="emit('close')">
            <Icon name="x" :size="16" :stroke-width="2.2" />
          </button>
        </header>

        <div class="body">
          <!-- Tohum + Araştır -->
          <div class="seed-row">
            <input
              class="fx seed"
              v-model="seed"
              placeholder="ör. all in one bilgisayar"
              @keydown.enter="runResearch"
            />
            <button class="run" :disabled="store.researching" @click="runResearch">
              <Icon
                :name="store.researching ? 'loader' : 'sparkles'"
                :size="14"
                :class="{ spin: store.researching }"
              />
              {{ store.researching ? "Araştırılıyor…" : "Araştır" }}
            </button>
          </div>

          <!-- Yükleniyor iskeleti -->
          <div v-if="store.researching" class="skeleton">
            <div v-for="i in 5" :key="i" class="sk-row"></div>
          </div>

          <!-- Uyarılar / kısmi hata notları -->
          <div v-if="notes.length" class="notes">
            <div v-for="(n, i) in notes" :key="i" class="note">
              <Icon name="alert" :size="12" /> {{ n }}
            </div>
          </div>

          <!-- Sonuçlar -->
          <template v-if="research && !store.researching">
            <!-- GSC gerçek sorgular (en değerli — en üstte) -->
            <template v-if="gscQueries.length">
              <div class="sec-title">
                <Icon name="search" :size="13" /> Google'daki gerçek sorgular
                <span class="count">{{ gscQueries.length }}</span>
              </div>
              <ul class="cands">
                <li v-for="(q, i) in gscQueries" :key="'g' + i" class="cand gsc">
                  <div class="c-main">
                    <span class="c-kw">{{ q.query }}</span>
                  </div>
                  <div class="c-metrics">
                    <span class="c-vol" title="Gösterim · tıklama">
                      <Icon name="eye" :size="11" /> {{ Math.round(q.impressions) }}
                    </span>
                    <span class="c-pos" title="Ortalama sıra">#{{ q.position.toFixed(1) }}</span>
                    <button
                      class="pick"
                      :class="{ active: keyword.trim().toLowerCase() === q.query.toLowerCase() }"
                      @click="pick(q.query)"
                    >
                      {{ keyword.trim().toLowerCase() === q.query.toLowerCase() ? "Seçili ✓" : "Hedef yap" }}
                    </button>
                  </div>
                </li>
              </ul>
            </template>

            <!-- Tohum zorluğu -->
            <div v-if="research.seed_difficulty" class="seed-diff">
              <span class="sd-label">"{{ research.seed_difficulty.keyword }}" rekabet</span>
              <span
                class="chip"
                :style="{
                  background: `var(--badge-${diffBadge(research.seed_difficulty.difficulty)}-bg)`,
                  color: `var(--badge-${diffBadge(research.seed_difficulty.difficulty)}-c)`,
                }"
              >
                {{ research.seed_difficulty.difficulty }} · {{ diffLabel(research.seed_difficulty.difficulty) }}
              </span>
            </div>

            <!-- Anahtar kelime adayları -->
            <div v-if="candidates.length" class="sec-title">
              <Icon name="tag" :size="13" /> Anahtar kelime fikirleri
              <span class="count">{{ candidates.length }}</span>
            </div>
            <ul v-if="candidates.length" class="cands">
              <li v-for="(c, i) in candidates" :key="i" class="cand">
                <div class="c-main">
                  <span class="c-kw">{{ c.keyword }}</span>
                  <span v-if="c.kind === 'question'" class="c-q">soru</span>
                </div>
                <div class="c-metrics">
                  <span class="c-vol" title="Aylık arama hacmi (tahmini)">
                    <Icon name="search" :size="11" /> {{ fmtVol(c.volume) }}
                  </span>
                  <span
                    class="c-diff"
                    :style="{
                      background: `var(--badge-${diffBadge(c.difficulty)}-bg)`,
                      color: `var(--badge-${diffBadge(c.difficulty)}-c)`,
                    }"
                    title="Anahtar kelime zorluğu"
                  >
                    {{ c.difficulty }}
                  </span>
                  <button
                    class="pick"
                    :class="{ active: keyword.trim().toLowerCase() === c.keyword.toLowerCase() }"
                    @click="pick(c.keyword)"
                  >
                    {{ keyword.trim().toLowerCase() === c.keyword.toLowerCase() ? "Seçili ✓" : "Hedef yap" }}
                  </button>
                </div>
              </li>
            </ul>

            <!-- Google Trends — hedef kelimeye ilgili sorgular -->
            <template v-if="trends.length">
              <div class="sec-title">
                <Icon name="sparkles" :size="13" /> İlgili trend aramaları
                <span class="count">{{ trends.length }}</span>
              </div>
              <div class="trend-chips">
                <button
                  v-for="(t, i) in trends"
                  :key="'t' + i"
                  class="trend-chip"
                  :title="'Hedef yap: ' + t.term"
                  @click="pick(t.term)"
                >
                  {{ t.term }}
                </button>
              </div>
            </template>

            <!-- Alan (domain) özeti — bilgi amaçlı -->
            <div v-if="domain" class="domain-strip">
              <div class="ds-title">
                <Icon name="link" :size="12" /> {{ domain.domain }}
              </div>
              <div class="ds-metrics">
                <span class="ds-m"><b>DR {{ domain.domain_rating }}</b><i>otorite</i></span>
                <span class="ds-m"><b>{{ fmtNum(domain.backlinks) }}</b><i>backlink</i></span>
                <span class="ds-m"><b>{{ fmtNum(domain.ref_domains) }}</b><i>ref. domain</i></span>
              </div>
            </div>

            <div
              v-if="!candidates.length && !gscQueries.length && !trends.length && !domain && !notes.length"
              class="empty"
            >
              Bu tohum için sonuç bulunamadı. Farklı bir kelime deneyin.
            </div>
          </template>

          <!-- İlk açılış (henüz araştırma yok) -->
          <div v-else-if="!store.researching" class="intro">
            <div class="intro-icon"><Icon name="sparkles" :size="22" :stroke-width="1.6" /></div>
            <p class="intro-t">Gerçek verilerle içerik üret</p>
            <p class="intro-s">
              Tohum kelimeyi onayla ve <b>Araştır</b>'a bas. Ahrefs'ten gerçek anahtar
              kelime fikirleri ve zorluk verisiyle en iyi hedefi seç; ardından
              "Gemini ile Üret" bu veriye dayanarak yazar.
            </p>
          </div>

          <p class="note-hint">
            <Icon name="info" :size="12" />
            Seçtiğin hedef kelime her iki kartta da kullanılır ve üretime enjekte edilir.
          </p>
        </div>
      </aside>
    </div>
  </Transition>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 40;
  display: flex;
  justify-content: flex-end;
  background: var(--overlay-bg);
  backdrop-filter: saturate(1.1) blur(2px);
}
.drawer {
  width: 420px;
  max-width: 92vw;
  height: 100%;
  overflow-y: auto;
  background: var(--c-card);
  border-left: 1px solid var(--c-border);
  box-shadow: -18px 0 48px var(--heavy-shadow);
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
  padding: 15px 16px;
  background: var(--c-card);
  border-bottom: 1px solid var(--c-border-soft);
}
.head-l {
  display: flex;
  align-items: center;
  gap: 11px;
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
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.seed-row {
  display: flex;
  gap: 8px;
}
.seed {
  flex: 1;
  height: 38px;
  padding: 0 12px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-input);
  font-size: 13px;
  color: var(--c-text);
  outline: none;
}
.run {
  flex: none;
  display: inline-flex;
  align-items: center;
  gap: 7px;
  height: 38px;
  padding: 0 15px;
  border: none;
  border-radius: 9px;
  background: var(--accent);
  color: #fff;
  font-size: 13px;
  font-weight: 590;
  cursor: pointer;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
}
.run:hover {
  filter: brightness(1.05);
}
.run:disabled {
  opacity: 0.7;
  cursor: default;
}
.spin {
  animation: spin 0.8s linear infinite;
}
.skeleton {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.sk-row {
  height: 44px;
  border-radius: 10px;
  background: linear-gradient(
    90deg,
    var(--c-hover) 25%,
    var(--c-chip) 37%,
    var(--c-hover) 63%
  );
  background-size: 400% 100%;
  animation: shimmer 1.3s ease infinite;
}
@keyframes shimmer {
  0% { background-position: 100% 0; }
  100% { background-position: 0 0; }
}
.notes {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.note {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11.5px;
  color: var(--warn-text);
  background: var(--warn-bg);
  border: 1px solid var(--warn-border);
  border-radius: 8px;
  padding: 7px 9px;
}
.seed-diff {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border: 1px solid var(--c-border-soft);
  border-radius: 10px;
  background: var(--c-list);
}
.sd-label {
  font-size: 12.5px;
  color: var(--c-mid);
  font-weight: 560;
}
.chip {
  font-size: 11.5px;
  font-weight: 620;
  padding: 3px 9px;
  border-radius: 999px;
}
.sec-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 620;
  color: var(--c-mid);
  margin-top: 2px;
}
.count {
  font-size: 10.5px;
  font-weight: 600;
  color: var(--c-soft);
  background: var(--c-chip);
  border-radius: 999px;
  padding: 1px 7px;
}
.cands {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.cand {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 9px 11px;
  border: 1px solid var(--c-border-soft);
  border-radius: 10px;
  background: var(--c-list);
  animation: popIn 0.25s ease both;
}
.cand:hover {
  border-color: var(--c-border);
  background: var(--c-hover);
}
.c-main {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 7px;
}
.c-kw {
  font-size: 13px;
  color: var(--c-text);
  font-weight: 520;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.c-q {
  flex: none;
  font-size: 9.5px;
  font-weight: 640;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--c-soft);
  background: var(--c-chip);
  border-radius: 4px;
  padding: 1px 5px;
}
.c-metrics {
  flex: none;
  display: flex;
  align-items: center;
  gap: 8px;
}
.c-vol {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 11.5px;
  color: var(--c-soft);
  font-variant-numeric: tabular-nums;
}
.c-pos {
  font-size: 11px;
  font-weight: 600;
  color: var(--c-mid);
  font-variant-numeric: tabular-nums;
}
.cand.gsc {
  border-left: 2px solid var(--accent);
}
.c-diff {
  min-width: 26px;
  text-align: center;
  font-size: 11px;
  font-weight: 640;
  padding: 2px 6px;
  border-radius: 6px;
  font-variant-numeric: tabular-nums;
}
.pick {
  flex: none;
  height: 28px;
  padding: 0 10px;
  border: 1px solid var(--c-border);
  border-radius: 7px;
  background: var(--c-input);
  color: var(--c-mid);
  font-size: 11.5px;
  font-weight: 580;
  cursor: pointer;
}
.pick:hover {
  border-color: var(--accent);
  color: var(--accent);
}
.pick.active {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}
.trend-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.trend-chip {
  padding: 5px 10px;
  border: 1px solid var(--c-border-soft);
  border-radius: 999px;
  background: var(--c-list);
  color: var(--c-mid);
  font-size: 12px;
  font-weight: 520;
  cursor: pointer;
  transition: border-color 0.15s ease, color 0.15s ease;
}
.trend-chip:hover {
  border-color: var(--accent);
  color: var(--accent);
}
.domain-strip {
  padding: 11px 12px;
  border: 1px solid var(--c-border-soft);
  border-radius: 10px;
  background: var(--c-list);
}
.ds-title {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 11.5px;
  font-weight: 560;
  color: var(--c-soft);
  margin-bottom: 9px;
}
.ds-metrics {
  display: flex;
  gap: 18px;
}
.ds-m {
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.ds-m b {
  font-size: 14px;
  font-weight: 660;
  color: var(--c-text);
  font-variant-numeric: tabular-nums;
}
.ds-m i {
  font-size: 10.5px;
  font-style: normal;
  color: var(--c-faint);
}
.empty {
  font-size: 12.5px;
  color: var(--c-soft);
  text-align: center;
  padding: 20px 12px;
}
.intro {
  text-align: center;
  padding: 26px 14px 12px;
}
.intro-icon {
  width: 52px;
  height: 52px;
  border-radius: 14px;
  background: var(--accent-tint);
  color: var(--accent);
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 14px;
}
.intro-t {
  font-size: 14px;
  font-weight: 620;
  color: var(--c-text);
  margin: 0 0 6px;
}
.intro-s {
  font-size: 12.5px;
  line-height: 1.55;
  color: var(--c-soft);
  margin: 0;
}
.note-hint {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--c-faint);
  margin: 4px 0 0;
}

/* Apple hissi: backdrop fade + drawer sağdan kayış */
.research-enter-active,
.research-leave-active {
  transition: opacity 0.28s ease;
}
.research-enter-from,
.research-leave-to {
  opacity: 0;
}
.research-enter-active .drawer,
.research-leave-active .drawer {
  transition: transform 0.28s cubic-bezier(0.32, 0.72, 0, 1);
}
.research-enter-from .drawer,
.research-leave-to .drawer {
  transform: translateX(100%);
}
@media (prefers-reduced-motion: reduce) {
  .research-enter-active,
  .research-leave-active,
  .research-enter-active .drawer,
  .research-leave-active .drawer {
    transition: opacity 0.12s ease;
  }
  .research-enter-from .drawer,
  .research-leave-to .drawer {
    transform: none;
  }
  .sk-row,
  .cand {
    animation: none;
  }
}
</style>
