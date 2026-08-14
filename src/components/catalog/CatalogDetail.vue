<script setup lang="ts">
/**
 * Katalog sayfası iyileştirme paneli — kategori · marka · içerik (Faz İ2).
 *
 * `StorePageModal`in gövdesi buraya taşındı: aynı iş artık modalda değil kendi ekranında,
 * ürün detayıyla aynı ağırlıkta. Modal kaldırıldı — iki yerde çizilseydi biri sapardı.
 *
 * **Neden fark gösterimi burada ürününkinden daha önemli.** Ürün meta'sında model bir ürünü
 * anlatıyor ve teknik tabloda her sayı kaynağa karşı doğrulanıyor. Burada doğrulanacak
 * kaynak yok: kategori tanıtım metnini karşılaştıracağımız bir şey mağazada zaten yok —
 * üretmemizin sebebi o. Kalan tek güvence operatörün okuması, o yüzden mağazadaki hâl ile
 * taslak yan yana duruyor ve gönderim ayrı bir adım.
 *
 * **Üretim neye dayandı?** Panel bunu da gösteriyor (`queries`): operatör "bu metin nereden
 * çıktı" diye sorabilmeli. Sorgu listesi boşsa üretim zayıf bağlamla çalışmıştır ve panel
 * bunu söylüyor — sessizce zayıf metin üretmek, üretmemekten kötü.
 */
import { computed, ref, watch } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "../../api";
import { useStore } from "../../store";
import { TIP_AD, tanitimMetniVar } from "../../catalog";
import Icon from "../Icon.vue";

const props = defineProps<{ kind: string }>();
const store = useStore();

const veri = computed(() => store.storePage);
const uretiliyor = ref(false);
const gonderiliyor = ref(false);

// Düzenlenebilir taslak alanları — operatör metni değiştirebilmeli.
const tBaslik = ref("");
const tAciklama = ref("");
const tKelime = ref("");
const tMetin = ref("");

/** Ürün kuralıyla aynı sınırlar (`validation::TITLE_MAX` / `DESC_MAX`). */
const BASLIK_MAX = 60;
const ACIKLAMA_MAX = 155;

const metinAlaniVar = computed(() => tanitimMetniVar(props.kind));

const kirli = computed(
  () =>
    !!veri.value &&
    (tBaslik.value !== veri.value.draft_page_title ||
      tAciklama.value !== veri.value.draft_meta_description ||
      tKelime.value !== veri.value.draft_target_keyword ||
      tMetin.value !== veri.value.draft_showcase),
);
const taslakVar = computed(() => !!(tBaslik.value.trim() || tMetin.value.trim()));

/**
 * Seçim değişince ya da kayıt tazelenince yerel alanları doldur.
 *
 * ⚠️ `remote_id` de izleniyor: yalnızca nesne kimliğine bakılsaydı aynı sayfayı yeniden
 * seçmek alanları sıfırlamazdı.
 */
watch(
  () => store.storePage,
  (d) => {
    tBaslik.value = d?.draft_page_title ?? "";
    tAciklama.value = d?.draft_meta_description ?? "";
    tKelime.value = d?.draft_target_keyword ?? "";
    tMetin.value = d?.draft_showcase ?? "";
  },
  { immediate: true },
);

async function uret() {
  if (!veri.value) return;
  uretiliyor.value = true;
  try {
    store.patchStorePage(await api.generateStorePage(props.kind, veri.value.remote_id));
    store.toast("Taslak üretildi — göndermeden önce okuyun.", "ok");
  } catch (e) {
    store.toast(String(e), "error");
  } finally {
    uretiliyor.value = false;
  }
}

async function kaydet() {
  if (!veri.value) return;
  try {
    store.patchStorePage(
      await api.saveStorePageDraft(
        props.kind,
        veri.value.remote_id,
        tBaslik.value,
        tAciklama.value,
        tKelime.value,
        tMetin.value,
      ),
    );
    store.toast("Taslak kaydedildi.", "ok");
  } catch (e) {
    store.toast(String(e), "error");
  }
}

/**
 * 🔴 Mağazayı değiştiren tek düğme — ve bilerek tek tek, toplu değil (kullanıcı kısıtı).
 * Kaydedilmemiş değişiklik varsa önce kaydediliyor: yoksa ekranda görünen metin ile
 * gönderilen metin farklı olur ve kullanıcı bunu fark etmez.
 */
async function gonder() {
  if (!veri.value) return;
  gonderiliyor.value = true;
  try {
    if (kirli.value) await kaydet();
    store.patchStorePage(await api.pushStorePage(props.kind, veri.value.remote_id));
    store.toast("Mağazaya gönderildi.", "ok");
  } catch (e) {
    store.toast(String(e), "error");
  } finally {
    gonderiliyor.value = false;
  }
}

/** Sayfanın kendisi — "gönderdiğim metin sitede nasıl göründü" tek tıkla görülebilmeli. */
const sayfaAdresi = computed(() => {
  const d = veri.value;
  const alan = store.settings?.ideasoft_domain?.trim();
  if (!d || !alan) return "";
  const yol: Record<string, string> = { category: "kategori", brand: "marka", blog: "blog/icerik" };
  const k = yol[d.kind];
  return k ? `https://${alan.replace(/^https?:\/\//, "").replace(/\/$/, "")}/${k}/${d.slug}` : "";
});

async function sayfayiAc() {
  if (!sayfaAdresi.value) return;
  try {
    await openUrl(sayfaAdresi.value);
  } catch {
    store.toast("Bağlantı açılamadı", "error");
  }
}

const sayi = (n: number) => Math.round(n).toLocaleString("tr-TR");
const tarih = (s: string) => s.replace("T", " ").slice(0, 16);
</script>

<template>
  <section class="detay om-scroll">
    <div v-if="veri" class="ic">
      <div class="bas">
        <div class="bas-ad">
          <h1>{{ veri.name }}</h1>
          <a v-if="sayfaAdresi" class="ext" title="Sayfayı sitede aç" @click.prevent="sayfayiAc">
            <Icon name="external" :size="14" />
          </a>
        </div>
        <div class="bas-alt">
          <span>{{ TIP_AD[props.kind] || props.kind }}</span>
          <span class="ayrac">·</span>
          <span>/{{ veri.slug }}</span>
          <template v-if="veri.impressions > 0">
            <span class="ayrac">·</span>
            <span>{{ sayi(veri.impressions) }} gösterim · {{ sayi(veri.clicks) }} tıklama</span>
          </template>
          <template v-if="veri.pushed_at">
            <span class="ayrac">·</span>
            <span class="gonderildi">Gönderildi · {{ tarih(veri.pushed_at) }}</span>
          </template>
        </div>
      </div>

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
            <textarea v-model="tMetin" class="fx" rows="8"></textarea>
          </template>
          <!-- ⚠️ Blogda alan YOK; olmayan bir alanı boş kutu olarak göstermek yerine
               nedenini yazıyoruz, yoksa "neden dolduramıyorum" sorusu doğar. -->
          <p v-else class="yok">
            Blog kayıtlarında tanıtım metni alanı bulunmuyor — yazının gövdesi IdeaSoft
            panelinden düzenleniyor. Buradan yalnızca başlık ve açıklama gönderiliyor.
          </p>
        </div>
      </div>

      <!-- 🔴 Sınırı gizlemiyoruz. Bu metnin doğruluğu ürün teknik tablosu gibi
           doğrulanamıyor; okumadan göndermek riskli ve kullanıcı bunu bilmeli. -->
      <p class="uyari">
        Taslak metin <b>doğrulanmadı</b>. Sayısal iddialar engelleniyor ama diğer bilgiler
        kontrol edilemiyor — göndermeden önce okuyun.
      </p>

      <div class="eylemler">
        <button class="ghost" :disabled="uretiliyor" @click="uret">
          <Icon name="sparkles" :size="14" :class="{ spin: uretiliyor }" />
          {{ uretiliyor ? "Üretiliyor…" : taslakVar ? "Yeniden üret" : "Taslak üret" }}
        </button>
        <button class="ghost" :disabled="!kirli" @click="kaydet">Taslağı kaydet</button>
        <button class="solid" :disabled="!taslakVar || gonderiliyor" @click="gonder">
          <Icon name="upload" :size="14" />
          {{ gonderiliyor ? "Gönderiliyor…" : "Mağazaya gönder" }}
        </button>
      </div>
    </div>

    <div v-else class="secim-yok">
      <div class="sy-ikon"><Icon name="fileEdit" :size="30" :stroke-width="1.6" /></div>
      <div class="sy-bas">Soldan bir sayfa seçin</div>
      <div class="sy-alt">
        Başlık, açıklama ve tanıtım metnini düzenlemek için listeden bir sayfaya tıklayın.
        Liste, Google'da en çok görünen sayfalar üstte olacak şekilde sıralı.
      </div>
    </div>
  </section>
</template>

<style scoped>
.detay {
  flex: 1;
  overflow-y: auto;
  min-width: 0;
  background: var(--c-bg);
}
.ic {
  max-width: 860px;
  margin: 0 auto;
  padding: 20px 32px 48px;
}
.bas-ad {
  display: flex;
  align-items: center;
  gap: 8px;
}
h1 {
  margin: 0;
  font-size: 19px;
  font-weight: 680;
  letter-spacing: -0.02em;
  line-height: 1.25;
  color: var(--c-text);
}
.ext {
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 7px;
  color: var(--c-soft);
  cursor: pointer;
}
.ext:hover {
  background: var(--c-hover);
  color: var(--accent);
}
.bas-alt {
  display: flex;
  align-items: center;
  gap: 7px;
  margin-top: 6px;
  font-size: 12.5px;
  color: var(--c-soft);
  flex-wrap: wrap;
}
.ayrac {
  color: var(--c-faint);
}
.gonderildi {
  color: var(--green);
  font-weight: 560;
}
.dayanak {
  border: 1px solid var(--c-border);
  border-radius: 10px;
  padding: 10px 12px;
  margin: 16px 0 14px;
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
  padding: 2px 8px;
  border-radius: 20px;
  background: var(--c-chip);
  color: var(--c-mid);
  font-size: 11.5px;
}
.d-uyari {
  display: flex;
  align-items: flex-start;
  gap: 7px;
  font-size: 12px;
  color: var(--warn-text);
  line-height: 1.5;
}
.satir {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 18px;
}
/* Dar pencerede iki sütun 300px'in altına iniyor ve metin okunmaz oluyor → alt alta. */
@media (max-width: 1100px) {
  .satir {
    grid-template-columns: 1fr;
  }
}
.s-bas {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--c-faint);
  margin-bottom: 10px;
}
.model {
  text-transform: none;
  letter-spacing: 0;
  font-size: 10.5px;
  padding: 1px 7px;
  border-radius: 20px;
  background: var(--accent-tint);
  color: var(--accent);
}
.lbl {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  font-size: 11.5px;
  font-weight: 560;
  color: var(--c-mid);
  margin: 10px 0 4px;
}
.say {
  font-size: 10.5px;
  color: var(--c-faint);
  font-variant-numeric: tabular-nums;
}
.say.asti {
  color: var(--red);
  font-weight: 600;
}
.mevcut {
  border: 1px solid var(--c-border);
  border-radius: 8px;
  padding: 8px 10px;
  background: var(--c-panel);
  color: var(--c-mid);
  font-size: 12.5px;
  line-height: 1.5;
  min-height: 34px;
  white-space: pre-wrap;
}
.mevcut.uzun {
  max-height: 220px;
  overflow-y: auto;
}
.sut input.fx,
.sut textarea.fx {
  width: 100%;
  padding: 8px 10px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-card);
  color: var(--c-text);
  font-size: 12.5px;
  line-height: 1.5;
  outline: none;
  resize: vertical;
}
.yok {
  margin: 10px 0 0;
  font-size: 11.5px;
  line-height: 1.55;
  color: var(--c-faint);
}
.uyari {
  margin: 16px 0 0;
  padding: 9px 11px;
  border: 1px solid var(--warn-border);
  border-radius: 9px;
  background: var(--warn-bg);
  color: var(--warn-text);
  font-size: 12px;
  line-height: 1.55;
}
.eylemler {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 16px;
}
.secim-yok {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--c-faint);
  padding: 40px;
}
.sy-ikon {
  width: 64px;
  height: 64px;
  border-radius: 16px;
  background: var(--c-panel);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 18px;
}
.sy-bas {
  font-size: 16px;
  font-weight: 600;
  color: var(--c-soft);
}
.sy-alt {
  font-size: 13px;
  margin-top: 6px;
  text-align: center;
  max-width: 360px;
  line-height: 1.55;
}
</style>
