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
import { generateCatalogDraft } from "../../teamGemini";
import { TeamStorePageWriteUnconfirmed, writeTeamIdeasoftPage } from "../../teamIdeasoft";
import { checkStorePageDraft, flushTeamStorePages, queueTeamPageDraft, recordTeamStorePage, teamStorePageScope, type TeamStorePageWriteTarget } from "../../teamStorePages";
import type { StorePageDetail } from "../../types";
import { TIP_AD, tanitimMetniVar } from "../../catalog";
import Icon from "../Icon.vue";
import ModalShell from "../ModalShell.vue";
import SeoCard from "../SeoCard.vue";

const props = defineProps<{ kind: string }>();
const store = useStore();

const veri = computed(() => store.storePage);
const uretiliyor = ref(false);
const gonderiliyor = ref(false);
const yayimOnayi = ref(false);
const yayimlaniyor = ref(false);
watch([() => veri.value?.remote_id, teamStorePageScope], () => { yayimOnayi.value = false; });

async function writeSharedPage(target: TeamStorePageWriteTarget, page: StorePageDetail, operation: "page_publish" | "page_push") {
  if (page.revision == null) throw new Error("Ortak sayfanın sürümünü yenileyin.");
  const result = await writeTeamIdeasoftPage(target, { operation, kind: page.kind, id: page.remote_id, revision: page.revision });
  const confirmed = { ...page, ...result.facts, team_pending: false };
  if (result.recorded) return { page: confirmed, updated: result.updated };
  if (!result.receipt) throw new TeamStorePageWriteUnconfirmed("Mağaza işlemi doğrulandı. Ortak durumu görmek için mağaza envanterini yenileyin.");
  const saved = await recordTeamStorePage(target, confirmed, result.facts, result.receipt).catch(() => {
    throw new TeamStorePageWriteUnconfirmed("Mağaza işlemi doğrulandı. Ortak durum kaydedilemedi, mağaza envanterini yenileyin.");
  });
  return { page: saved, updated: result.updated };
}

function updateSharedPage(original: StorePageDetail, saved: StorePageDetail) {
  if (veri.value === original) store.patchStorePage(saved);
  if (original.team_scope === teamStorePageScope()) {
    void flushTeamStorePages().then(() => {
      if (original.team_scope === teamStorePageScope()) return store.loadStorePages(original.kind);
    }).catch(console.warn);
  }
}

async function yayimla() {
  const page = veri.value;
  if (!page || page.status !== 0 || yayimlaniyor.value || gonderiliyor.value) return;
  yayimlaniyor.value = true;
  try {
    const target = await checkStorePageDraft(page);
    if (veri.value !== page || (target && target.key !== teamStorePageScope())) return;
    if (target) {
      const result = await writeSharedPage(target, page, "page_publish");
      updateSharedPage(page, result.page);
      store.toast(result.page.team_pending ? "Mağaza yayını doğrulandı. Ortak kayıt eşitleme kuyruğunda."
        : result.updated ? "Kayıt mağazada yayımlandı." : "Kayıt mağazada zaten yayında.", "ok");
    } else {
      await api.publishStorePage(page.kind, page.remote_id);
      if (veri.value === page) store.patchStorePage({ ...page, status: 1 });
      store.toast("Kayıt mağazada yayımlandı.", "ok");
    }
    yayimOnayi.value = false;
  } catch (error) {
    store.toast(String(error), "error");
  } finally {
    yayimlaniyor.value = false;
  }
}

// Düzenlenebilir taslak alanları — operatör metni değiştirebilmeli.
const tBaslik = ref("");
const tAciklama = ref("");
const tKelime = ref("");
const tMetin = ref("");
const draftChangedElsewhere = ref(false);
const writing = computed(() => uretiliyor.value || gonderiliyor.value || yayimlaniyor.value);

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
function matchesDraft(d: StorePageDetail) {
  return tBaslik.value === d.draft_page_title && tAciklama.value === d.draft_meta_description &&
    tKelime.value === d.draft_target_keyword && tMetin.value === d.draft_showcase;
}

function loadDraft(d: StorePageDetail | null) {
  tBaslik.value = d?.draft_page_title ?? "";
  tAciklama.value = d?.draft_meta_description ?? "";
  tKelime.value = d?.draft_target_keyword ?? "";
  tMetin.value = d?.draft_showcase ?? "";
  draftChangedElsewhere.value = false;
}

watch(() => store.storePage, (d, previous) => {
  if (d?.team_scope && previous?.team_scope === d.team_scope && previous.kind === d.kind && previous.remote_id === d.remote_id &&
      !matchesDraft(previous) && !matchesDraft(d) && !uretiliyor.value) {
    if (d.revision !== previous.revision) draftChangedElsewhere.value = true;
    return;
  }
  loadDraft(d);
}, { immediate: true });

async function uret() {
  const page = veri.value;
  if (!page || uretiliyor.value) return;
  uretiliyor.value = true;
  try {
    const draft = await generateCatalogDraft(page);
    const saved = page.team_scope ? await queueTeamPageDraft(page, {
      draft_page_title: draft.draft_page_title, draft_meta_description: draft.draft_meta_description,
      draft_target_keyword: draft.draft_target_keyword, draft_showcase: draft.draft_showcase, draft_model: draft.draft_model,
    }) : draft;
    if (veri.value === page) store.patchStorePage(saved);
    if (page.team_scope) void flushTeamStorePages().then(() => store.loadStorePages(page.kind)).catch(console.warn);
    store.toast("Taslak üretildi — göndermeden önce okuyun.", "ok");
  } catch (e) {
    store.toast(String(e), "error");
  } finally {
    uretiliyor.value = false;
  }
}

async function kaydet(): Promise<boolean> {
  const page = veri.value;
  if (!page || draftChangedElsewhere.value) return false;
  try {
    if (page.team_scope) {
      const saved = await queueTeamPageDraft(page, {
        draft_page_title: tBaslik.value, draft_meta_description: tAciklama.value,
        draft_target_keyword: tKelime.value, draft_showcase: tMetin.value,
      });
      if (veri.value === page) store.patchStorePage(saved);
      store.toast("Taslak kaydedildi ve eşitleme kuyruğuna alındı.", "ok");
      void flushTeamStorePages().then(() => store.loadStorePages(page.kind)).catch(console.warn);
      return true;
    }
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
    return true;
  } catch (e) {
    store.toast(String(e), "error");
    return false;
  }
}

/**
 * 🔴 Mağazayı değiştiren tek düğme — ve bilerek tek tek, toplu değil (kullanıcı kısıtı).
 * Kaydedilmemiş değişiklik varsa önce kaydediliyor: yoksa ekranda görünen metin ile
 * gönderilen metin farklı olur ve kullanıcı bunu fark etmez.
 */
async function gonder() {
  const page = veri.value;
  if (!page || gonderiliyor.value || yayimlaniyor.value) return;
  gonderiliyor.value = true;
  try {
    if (kirli.value && !(await kaydet())) return;
    const latest = veri.value;
    if (!latest || latest.kind !== page.kind || latest.remote_id !== page.remote_id || latest.team_scope !== page.team_scope) return;
    const target = await checkStorePageDraft(latest);
    if (veri.value !== latest || (target && target.key !== teamStorePageScope())) return;
    if (target) {
      const result = await writeSharedPage(target, latest, "page_push");
      updateSharedPage(latest, result.page);
      store.toast(result.page.team_pending ? "Mağaza işlemi doğrulandı. Ortak kayıt eşitleme kuyruğunda."
        : result.updated ? "Mağazaya gönderildi." : "Mağazadaki değerler taslakla eşleşiyor.", "ok");
    } else {
      const sent = await api.pushStorePage(latest.kind, latest.remote_id);
      if (veri.value === latest) store.patchStorePage(sent);
      store.toast("Mağazaya gönderildi.", "ok");
    }
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
const yayinTarihi = new Intl.DateTimeFormat("tr-TR", { dateStyle: "short", timeStyle: "short" });
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
          <span class="yayin-durumu" :class="{ aktif: veri.status === 1 }">
            {{ veri.status === 1 ? "Yayında" : "Pasif" }}
          </span>
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
          <template v-if="veri.published_at">
            <span class="ayrac">·</span>
            <span class="gonderildi">GSC takibi başladı · {{ yayinTarihi.format(new Date(veri.published_at)) }}</span>
          </template>
        </div>
        <button v-if="veri.status === 0" type="button" class="primary-action publish-action" :disabled="writing" @click="yayimOnayi = true">
          <Icon name="upload" :size="15" /> Yayımla
        </button>
      </div>

      <p v-if="veri.team_conflict || veri.team_pending" class="d-bas" role="status">
        {{ veri.team_conflict ? "Kayıt çakışması var. Bu cihazdaki değişiklik sırada saklanıyor." : "Kayıt eşitleme bekliyor." }}
      </p>
      <div v-if="draftChangedElsewhere" class="uyari" role="status">
        Ortak taslak başka bir oturumda güncellendi. Ekrandaki metni kopyaladıktan sonra güncel taslağı yükleyebilirsiniz.
        <button type="button" class="secondary-action" :disabled="writing" @click="loadDraft(veri)">Güncel taslağı yükle</button>
      </div>

      <SeoCard icon="type" title="Sayfa SEO" :model="veri.draft_model" stack>
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
          </div>
          <label class="lbl" for="catalog-page-title">
            Sayfa başlığı
            <span class="say" :class="{ asti: tBaslik.length > BASLIK_MAX }">
              {{ tBaslik.length }}/{{ BASLIK_MAX }}
            </span>
          </label>
          <input id="catalog-page-title" v-model="tBaslik" :readonly="writing" class="fx" />
          <label class="lbl" for="catalog-meta-description">
            Meta açıklama
            <span class="say" :class="{ asti: tAciklama.length > ACIKLAMA_MAX }">
              {{ tAciklama.length }}/{{ ACIKLAMA_MAX }}
            </span>
          </label>
          <textarea id="catalog-meta-description" v-model="tAciklama" :readonly="writing" class="fx" rows="3"></textarea>
          <label class="lbl" for="catalog-target-keyword">Hedef kelime</label>
          <input id="catalog-target-keyword" v-model="tKelime" :readonly="writing" class="fx" />
          <template v-if="metinAlaniVar">
            <label class="lbl" for="catalog-showcase-content">Tanıtım metni</label>
            <textarea id="catalog-showcase-content" v-model="tMetin" :readonly="writing" class="fx" rows="8"></textarea>
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

      <template #actions>
        <button class="primary-action" :disabled="writing || draftChangedElsewhere" @click="uret">
          <Icon name="sparkles" :size="14" :class="{ spin: uretiliyor }" />
          {{ uretiliyor ? "Üretiliyor…" : taslakVar ? "Yeniden üret" : "Taslak üret" }}
        </button>
        <button class="secondary-action" :disabled="!kirli || writing || draftChangedElsewhere" @click="kaydet">Taslağı kaydet</button>
        <button class="secondary-action" :disabled="!taslakVar || writing || draftChangedElsewhere" @click="gonder">
          <Icon name="upload" :size="14" />
          {{ gonderiliyor ? "Gönderiliyor…" : "Mağazaya gönder" }}
        </button>
      </template>
      </SeoCard>
      <Teleport to="body">
        <ModalShell
          :open="yayimOnayi"
          title="Mağazada yayımla"
          label="Mağazada yayımla"
          icon="upload"
          :sub="veri.name"
          :closable="!yayimlaniyor"
          @close="yayimOnayi = false"
        >
          <p class="publish-note">Kayıt, IdeaSoft’taki mevcut içeriğiyle mağazada görünür olacak.</p>
          <template #footer>
            <button type="button" class="secondary-action" :disabled="yayimlaniyor" @click="yayimOnayi = false">Vazgeç</button>
            <button type="button" class="primary-action" :disabled="yayimlaniyor" @click="yayimla">
              <Icon :name="yayimlaniyor ? 'loader' : 'upload'" :size="15" :class="{ spin: yayimlaniyor }" />
              {{ yayimlaniyor ? "Yayımlanıyor…" : "Yayımla" }}
            </button>
          </template>
        </ModalShell>
      </Teleport>
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
.bas {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px 12px;
}
.bas-ad {
  flex-basis: 100%;
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
  flex: 1;
  display: flex;
  align-items: center;
  gap: 7px;
  margin-top: 6px;
  font-size: 12.5px;
  color: var(--c-soft);
  flex-wrap: wrap;
}
.yayin-durumu {
  padding: 3px 8px;
  border-radius: 20px;
  background: var(--c-chip);
  color: var(--c-mid);
  font-weight: 600;
}
.yayin-durumu.aktif {
  background: var(--badge-ok-bg);
  color: var(--green);
}
.publish-note {
  margin: 0;
  color: var(--c-mid);
  line-height: 1.55;
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
  grid-template-columns: repeat(2, minmax(0, 1fr));
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
  overflow-wrap: anywhere;
}
.mevcut.uzun {
  max-height: 220px;
  overflow-y: auto;
}
.sut input.fx,
.sut textarea.fx {
  width: 100%;
  padding: 9px 12px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-card);
  color: var(--c-text);
  font-size: 13px;
  line-height: 1.5;
  outline: none;
}
.sut textarea.fx {
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
