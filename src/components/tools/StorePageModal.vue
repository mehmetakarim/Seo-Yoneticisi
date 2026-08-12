<script setup lang="ts">
/**
 * Mağaza sayfası iyileştirme paneli — kategori · marka · blog (Faz İ4).
 *
 * **Neden fark gösterimi burada daha da önemli.** Ürün meta'sında model bir ürünü anlatıyor
 * ve teknik tabloda her sayı kaynağa karşı doğrulanıyor. Burada doğrulanacak kaynak yok:
 * kategori tanıtım metnini karşılaştıracağımız bir şey mağazada zaten yok — üretmemizin
 * sebebi o. Kalan tek güvence operatörün okuması, o yüzden mağazadaki hâl ile taslak yan
 * yana duruyor ve gönderim ayrı bir adım.
 *
 * **Üretim neye dayandı?** Panel bunu da gösteriyor (`queries`): operatör "bu metin nereden
 * çıktı" diye sorabilmeli. Sorgu listesi boşsa üretim zayıf bağlamla çalışmıştır ve panel
 * bunu söylüyor — sessizce zayıf metin üretmek, üretmemekten kötü.
 */
import { computed, ref, watch } from "vue";
import { api } from "../../api";
import { useStore } from "../../store";
import Icon from "../Icon.vue";
import ModalShell from "../ModalShell.vue";
import type { StorePageDetail } from "../../types";

const props = defineProps<{ open: boolean; kind: string; pageId: number | null }>();
const emit = defineEmits<{ close: [] }>();
const store = useStore();

const veri = ref<StorePageDetail | null>(null);
const yukleniyor = ref(false);
const uretiliyor = ref(false);
const gonderiliyor = ref(false);

// Düzenlenebilir taslak alanları — operatör metni değiştirebilmeli.
const tBaslik = ref("");
const tAciklama = ref("");
const tKelime = ref("");
const tMetin = ref("");

const TIP_AD: Record<string, string> = {
  category: "Kategori",
  brand: "Marka",
  blog: "İçerik",
};

/** Ürün kuralıyla aynı sınırlar (`validation::TITLE_MAX` / `DESC_MAX`). */
const BASLIK_MAX = 60;
const ACIKLAMA_MAX = 155;

const kirli = computed(
  () =>
    !!veri.value &&
    (tBaslik.value !== veri.value.draft_page_title ||
      tAciklama.value !== veri.value.draft_meta_description ||
      tKelime.value !== veri.value.draft_target_keyword ||
      tMetin.value !== veri.value.draft_showcase),
);

const taslakVar = computed(() => !!(tBaslik.value.trim() || tMetin.value.trim()));
/** Blogda tanıtım metni alanı yok — arka uçta da göndermiyoruz. */
const metinAlaniVar = computed(() => props.kind !== "blog");

function tasla(d: StorePageDetail) {
  veri.value = d;
  tBaslik.value = d.draft_page_title;
  tAciklama.value = d.draft_meta_description;
  tKelime.value = d.draft_target_keyword;
  tMetin.value = d.draft_showcase;
}

watch(
  () => [props.open, props.pageId] as const,
  async () => {
    if (!props.open || props.pageId == null) return;
    yukleniyor.value = true;
    try {
      tasla(await api.getStorePage(props.kind, props.pageId));
    } catch (e) {
      store.toast(String(e), "error");
    } finally {
      yukleniyor.value = false;
    }
  },
  { immediate: true },
);

async function uret() {
  if (props.pageId == null) return;
  uretiliyor.value = true;
  try {
    tasla(await api.generateStorePage(props.kind, props.pageId));
    store.toast("Taslak üretildi — göndermeden önce okuyun.", "ok");
  } catch (e) {
    store.toast(String(e), "error");
  } finally {
    uretiliyor.value = false;
  }
}

async function kaydet() {
  if (props.pageId == null) return;
  try {
    tasla(
      await api.saveStorePageDraft(
        props.kind, props.pageId, tBaslik.value, tAciklama.value, tKelime.value, tMetin.value,
      ),
    );
    store.toast("Taslak kaydedildi.", "ok");
  } catch (e) {
    store.toast(String(e), "error");
  }
}

/**
 * ⚠️ Mağazayı değiştiren tek düğme. Kaydedilmemiş değişiklik varsa önce kaydediliyor —
 * yoksa ekranda görünen metin ile gönderilen metin farklı olur ve kullanıcı bunu fark etmez.
 */
async function gonder() {
  if (props.pageId == null) return;
  gonderiliyor.value = true;
  try {
    if (kirli.value) await kaydet();
    tasla(await api.pushStorePage(props.kind, props.pageId));
    store.toast("Mağazaya gönderildi.", "ok");
  } catch (e) {
    store.toast(String(e), "error");
  } finally {
    gonderiliyor.value = false;
  }
}
</script>

<template>
  <ModalShell
    :open="props.open"
    label="Sayfa iyileştirme"
    :title="veri?.name || 'Sayfa'"
    icon="fileEdit"
    :width="720"
    scroll
    @close="emit('close')"
  >
    <template #sub>
      {{ TIP_AD[props.kind] || props.kind }} ·
      <span v-if="veri">{{ Math.round(veri.impressions) }} gösterim · {{ Math.round(veri.clicks) }} tık</span>
    </template>

    <div v-if="yukleniyor" class="bekle">Yükleniyor…</div>
    <div v-else-if="veri">
      <!-- Üretimin dayanağı. Boşsa bunu SÖYLÜYORUZ; sessizce zayıf metin üretmek,
           üretmemekten kötü. -->
      <div class="dayanak">
        <div class="d-bas">Üretim neye dayanıyor</div>
        <div v-if="veri.queries.length" class="d-sorgular">
          <span v-for="q in veri.queries.slice(0, 8)" :key="q" class="d-rozet">{{ q }}</span>
        </div>
        <div v-else class="d-uyari">
          <Icon name="alert" :size="13" />
          Bu sayfa için ölçülmüş arama verisi yok. Üretim yalnızca sayfa adına dayanır —
          önce Fırsatlar ekranından analizi çalıştırmak daha iyi bir metin verir.
        </div>
      </div>

      <div class="satir">
        <div class="sut">
          <div class="s-bas">Mağazadaki hâli</div>
          <label class="lbl">Sayfa başlığı</label>
          <div class="mevcut">{{ veri.page_title || "—" }}</div>
          <label class="lbl">Meta açıklama</label>
          <div class="mevcut">{{ veri.meta_description || "—" }}</div>
          <label class="lbl">Hedef kelime</label>
          <div class="mevcut">{{ veri.target_keyword || "—" }}</div>
          <template v-if="metinAlaniVar">
            <label class="lbl">Tanıtım metni</label>
            <div class="mevcut uzun">{{ veri.showcase_content || "— (yazılmamış)" }}</div>
          </template>
        </div>

        <div class="sut">
          <div class="s-bas">
            Taslak
            <span v-if="veri.draft_model" class="model">{{ veri.draft_model }}</span>
          </div>
          <label class="lbl">
            Sayfa başlığı
            <span class="say" :class="{ asti: tBaslik.length > BASLIK_MAX }">
              {{ tBaslik.length }}/{{ BASLIK_MAX }}
            </span>
          </label>
          <input v-model="tBaslik" class="fx" />
          <label class="lbl">
            Meta açıklama
            <span class="say" :class="{ asti: tAciklama.length > ACIKLAMA_MAX }">
              {{ tAciklama.length }}/{{ ACIKLAMA_MAX }}
            </span>
          </label>
          <textarea v-model="tAciklama" class="fx" rows="3"></textarea>
          <label class="lbl">Hedef kelime</label>
          <input v-model="tKelime" class="fx" />
          <template v-if="metinAlaniVar">
            <label class="lbl">Tanıtım metni</label>
            <textarea v-model="tMetin" class="fx" rows="7"></textarea>
          </template>
        </div>
      </div>

      <!-- 🔴 Sınırı gizlemiyoruz. Bu metnin doğruluğu ürün teknik tablosu gibi
           doğrulanamıyor; okumadan göndermek riskli ve kullanıcı bunu bilmeli. -->
      <p class="uyari">
        Taslak metin <b>doğrulanmadı</b>. Sayısal iddialar engelleniyor ama diğer bilgiler
        kontrol edilemiyor — göndermeden önce okuyun.
        <span v-if="veri.pushed_at"> · Son gönderim: {{ veri.pushed_at.replace("T", " ").slice(0, 16) }}</span>
      </p>
    </div>

    <template #footer>
      <button class="ghost" @click="emit('close')">Kapat</button>
      <button class="ghost" :disabled="uretiliyor" @click="uret">
        <Icon name="sparkles" :size="14" :class="{ spin: uretiliyor }" />
        {{ uretiliyor ? "Üretiliyor…" : taslakVar ? "Yeniden üret" : "Taslak üret" }}
      </button>
      <button class="ghost" :disabled="!kirli" @click="kaydet">Taslağı kaydet</button>
      <button class="solid" :disabled="!taslakVar || gonderiliyor" @click="gonder">
        <Icon name="upload" :size="14" />
        {{ gonderiliyor ? "Gönderiliyor…" : "Mağazaya gönder" }}
      </button>
    </template>
  </ModalShell>
</template>

<style scoped>
.bekle {
  padding: 40px;
  text-align: center;
  color: var(--c-faint);
  font-size: 13px;
}
.dayanak {
  border: 1px solid var(--c-border);
  border-radius: 10px;
  padding: 10px 12px;
  margin-bottom: 14px;
  background: var(--c-input);
}
.d-bas {
  font-size: 10.5px;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--c-faint);
  margin-bottom: 7px;
}
.d-sorgular {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}
.d-rozet {
  font-size: 11.5px;
  padding: 2px 8px;
  border-radius: 6px;
  border: 1px solid var(--c-border);
  color: var(--c-mid);
}
.d-uyari {
  font-size: 11.5px;
  color: var(--amber);
  display: flex;
  align-items: flex-start;
  gap: 6px;
  line-height: 1.5;
}
.satir {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}
.s-bas {
  font-size: 12px;
  font-weight: 600;
  color: var(--c-mid);
  padding-bottom: 7px;
  border-bottom: 1px solid var(--c-border);
  margin-bottom: 10px;
  display: flex;
  justify-content: space-between;
  align-items: baseline;
}
.model {
  font-size: 10.5px;
  color: var(--c-faint);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}
.lbl {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  font-size: 11px;
  color: var(--c-faint);
  margin: 9px 0 4px;
}
.say {
  font-variant-numeric: tabular-nums;
  font-size: 10.5px;
}
.say.asti {
  color: var(--red);
}
.mevcut {
  font-size: 12.5px;
  color: var(--c-mid);
  line-height: 1.5;
  padding: 6px 8px;
  border-radius: 7px;
  background: var(--c-input);
  border: 1px solid var(--c-border);
  min-height: 30px;
}
.mevcut.uzun {
  max-height: 160px;
  overflow-y: auto;
  white-space: pre-wrap;
}
.fx {
  width: 100%;
  font-size: 12.5px;
  line-height: 1.5;
  padding: 6px 8px;
  border-radius: 7px;
  border: 1px solid var(--c-border);
  background: var(--c-bg);
  color: var(--c-fg);
  resize: vertical;
  font-family: inherit;
}
.uyari {
  margin: 14px 0 0;
  font-size: 11.5px;
  color: var(--c-faint);
  line-height: 1.6;
}
.solid,
.ghost {
  height: 32px;
  padding: 0 14px;
  border-radius: 8px;
  font-size: 12.5px;
  font-weight: 560;
  cursor: pointer;
  border: 1px solid var(--c-border);
  background: var(--c-input);
  color: var(--c-mid);
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.solid {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}
.solid:disabled,
.ghost:disabled {
  opacity: 0.45;
  cursor: default;
}
</style>
