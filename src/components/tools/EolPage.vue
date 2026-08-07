<script setup lang="ts">
/**
 * Satışta olmayan ama trafik alan sayfalar — bu uygulamanın en büyük SEO fırsatı.
 *
 * Ölçüm (2026-07-29, kurumsalit.com, 90 gün): satıştaki 237 sayfa 1.728 tıklama alırken,
 * feed'de olmayan 4.527 sayfa **3.840 tıklama** alıyor. Yani ürün trafiğinin %69'u satın
 * alınamayan sayfalara gidiyor. Kıyas: optimize ettiğimiz 60 fırsatın toplam kaybı 374 tıklama.
 *
 * Bu analizi QueryLoom gibi araçlar yapamaz: onlar yalnızca GSC'yi görür, hangi ürünün
 * satışta olduğunu bilmez. Biz kataloğu da biliyoruz.
 *
 * ⚠️ Canonical yazma akışı **toplu değildir**: her satır ayrı, önizlemeli, açık onayla.
 * `apply_canonical` bilinçli olarak liste almıyor — imza toplu kullanımı zorlaştırıyor.
 */
import { computed, ref } from "vue";
import { useStore } from "../../store";
import Icon from "../Icon.vue";
import ModalShell from "../ModalShell.vue";
import ToolShell from "./ToolShell.vue";
import SeoTable, { type TableCol, type TableRow } from "./SeoTable.vue";

const store = useStore();
const rows = computed(() => store.opportunity?.eol ?? []);
const clicks = computed(() => Math.round(store.opportunity?.eol_clicks ?? 0));

/** Liste uzun olabilir (bu mağazada 2.190 sayfa) — önce en değerlileri. */
const limit = ref(25);

/** Tam URL'den son yol parçası — canonical hedefi olarak gönderilir. */
const slugOf = (u: string) => u.trim().replace(/\/$/, "").split("/").pop() ?? "";

/** Bu sayfa için önerilen hedef slug; öneri yoksa boş (kullanıcı kendisi seçer). */
const targetOf = (eolUrl: string) => {
  const s = store.successors[eolUrl];
  return s?.sku && s.url ? slugOf(s.url) : "";
};

const cols: TableCol[] = [
  { key: "name", label: "Sayfa", type: "text" },
  { key: "clk", label: "Tıklama", type: "num", emphasis: "down" },
  { key: "imp", label: "Gösterim", type: "num" },
  { key: "pos", label: "Konum", type: "num" },
  { key: "act", label: "İşlem", type: "actions" },
];

const n = (x: number) => Math.round(x).toString();
const tableRows = computed<TableRow[]>(() =>
  rows.value.slice(0, limit.value).map((e) => {
    const s = store.successors[e.url];
    const bakiliyor = !!store.successorBusy[e.url];
    return {
      id: e.url,
      name: e.slug,
      // Halef sonucu ikincil satırda: eskiden ad hücresinin içine gömülü bir blok halindeydi
      // ve satır yüksekliğini ekrandan ekrana bozuyordu.
      //
      // 🔴 Sonuç metni ARTIK TIKLANABİLİR (saha geri bildirimi, 2026-08-07). Faz B'de etiketli
      // "Hedef seç ve ayarla" düğmesini isimsiz bir zincir ikonuna çevirmiştim; öneri gelince
      // ekranda "sıradaki adım burada" diyen hiçbir işaret kalmamıştı ve kullanıcı seçim
      // modalinin kaybolduğunu düşündü. Modal duruyordu — bulunamıyordu.
      sub: bakiliyor
        ? "Halef aranıyor…"
        : s
          ? (s.sku ? `Halef: ${s.name} — canonical ayarla` : "Uygun halef bulunamadı — hedef seçin")
          : "",
      subTip: s?.reason,
      subAction: s && !bakiliyor ? ("canonical" as const) : undefined,
      values: { clk: n(e.clicks), imp: n(e.impressions), pos: e.position.toFixed(1) },
      actions: [
        {
          key: "successor",
          // ⚠️ YALNIZCA bu satır pasif. Eskiden `!!store.successorBusy` idi ve bir satıra
          // basınca 25 satırın düğmesi birden sönüyordu (saha hatası).
          disabled: bakiliyor,
          tip: bakiliyor ? "Bakılıyor…" : s ? "Halefi yeniden öner" : "Halef öner",
        },
        {
          key: "canonical",
          disabled: store.canonicalBusy,
          // ⚠️ Halef bulunamasa da açık: modelin bulamaması hedefin olmadığı anlamına
          // gelmiyor (saha geri bildirimi). Hedef yoksa seçim modali açılıyor.
          tip: targetOf(e.url) ? "Canonical ayarla" : "Hedef seç ve ayarla",
        },
      ],
    };
  }),
);

/** İşlem sütunundan gelen eylemi ilgili akışa bağlar. */
function eylem(p: { id: string; key: string }) {
  const e = rows.value.find((x) => x.url === p.id);
  if (!e) return;
  if (p.key === "successor") store.suggestSuccessor(e.url);
  else if (p.key === "canonical") store.startCanonical(e.slug, targetOf(e.url));
}
</script>

<template>
  <ToolShell
    :empty="!rows.length"
    empty-text="Katalog dışı sayfa trafiği yok — Google'da sıralanan her ürün sayfası satışta."
  >
    <div class="head">
      <p class="note">
        Bu adresler kataloğunuzda yok ama Google'da hâlâ sıralanıyor — ziyaretçi geliyor,
        ürünü satın alamıyor. Çözüm genelde güncel nesle <b>301 yönlendirme</b>;
        yönlendirmeyi IdeaSoft panelinden tanımlamanız gerekir, uygulama bunu yapamaz.
        Bazı sayfaları bilinçli tutuyor olabilirsiniz — liste öneridir, karar sizin.
      </p>
      <button
        class="cat-sync"
        :disabled="store.catalogBusy"
        data-tip="Tüm kataloğu bir kez çeker (~7 dk). Canonical için GEREKMEZ — uygulama tek satırı anında bulur. Bu yalnızca listenin tamamını hızlandırır."
        @click="store.syncCatalog()"
      >
        <Icon
          :name="store.catalogBusy ? 'loader' : 'refresh'"
          :size="11"
          :class="{ spin: store.catalogBusy }"
        />
        {{ store.catalogBusy ? "Katalog alınıyor…" : "Katalogla eşleştir" }}
      </button>
    </div>

    <SeoTable
      :cols="cols"
      :rows="tableRows"
      :summary="`${rows.length} sayfa satışta değil · ${clicks} tıklama boşa gidiyor`"
      :count-label="`${tableRows.length} / ${rows.length} satır`"
      :more-label="rows.length > limit ? `Sonraki 50'yi göster (${rows.length - limit} kaldı)` : ''"
      @action="eylem"
      @more="limit += 50"
    />

    <!-- Hedef seçme: halef önerisi boş çıktığında ya da öneri değiştirilmek istendiğinde.
         Yine TEK satır için — toplu seçim yok. -->
    <ModalShell
      :open="!!store.canonicalPicker"
      label="Canonical hedefi seç"
      title="Canonical hedefini seçin"
      :sub="store.canonicalPicker?.eolSlug"
      :closable="!store.canonicalSearching"
      @close="store.cancelCanonicalPicker()"
    >
      <div class="pick-row">
        <input
          v-model="store.canonicalQuery"
          class="pick-in"
          type="text"
          placeholder="Ürün adı yazın (en az 3 harf)"
          @keyup.enter="store.searchCanonicalTarget()"
        />
        <button
          class="succ-btn"
          :disabled="store.canonicalSearching || store.canonicalQuery.trim().length < 3"
          @click="store.searchCanonicalTarget()"
        >
          <Icon
            :name="store.canonicalSearching ? 'loader' : 'search'"
            :size="11"
            :class="{ spin: store.canonicalSearching }"
          />
          {{ store.canonicalSearching ? "Aranıyor…" : "Ara" }}
        </button>
      </div>
      <!-- 🔴 Arama YALNIZCA satıştaki (feed'deki) ürünlerde. İlk sürüm IdeaSoft'un tam
           kataloğunda arıyordu ve satıştan kalkmış ürünleri de listeliyordu; ölü bir sayfayı
           başka bir ölü sayfaya işaret ettirmek sorunu taşımak olurdu (saha hatası). -->
      <div class="pick-hint">
        Yalnızca <b>satıştaki</b> ürünlerde aranır — canonical, ziyaretçinin satın alabileceği
        bir sayfayı işaret etmeli.
      </div>
      <div v-if="store.canonicalResults.length" class="pick-list">
        <div
          v-for="r in store.canonicalResults"
          :key="r.slug"
          class="pick-item"
          @click="store.pickCanonicalTarget(r.slug)"
        >
          <span class="pi-name">{{ r.name }}</span>
          <span class="pi-slug">{{ r.slug }}</span>
        </div>
      </div>
      <div
        v-else-if="!store.canonicalSearching && store.canonicalQuery.trim().length >= 3"
        class="pick-hint"
      >
        Satıştaki ürünler arasında eşleşme yok. Farklı bir sözcük deneyin; ürün gerçekten
        satışta değilse canonical hedefi olamaz.
      </div>

      <template #footer>
        <button class="ghost" :disabled="store.canonicalSearching" @click="store.cancelCanonicalPicker()">
          Vazgeç
        </button>
      </template>
    </ModalShell>

    <!-- Canonical onay modali: önce fark, sonra onay (Faz 9 gönderim modalinin deseni). -->
    <ModalShell
      :open="!!store.canonicalPending"
      label="Canonical onayı"
      title="Canonical ayarlanacak"
      :sub="store.canonicalPending?.product_name"
      :closable="!store.canonicalBusy"
      @close="store.cancelCanonical()"
    >
      <div class="diff">
        <div class="d-row">
          <span class="d-lab">Şu an</span>
          <span class="d-val muted">{{ store.canonicalPending?.current || "tanımlı değil" }}</span>
        </div>
        <div class="d-row hi">
          <span class="d-lab">Olacak</span>
          <span class="d-val">{{ store.canonicalPending?.proposed }}</span>
        </div>
        <!-- Hedefin ADI: slug'a bakarak yanlış ürünü onaylamak kolay. -->
        <div class="d-row">
          <span class="d-lab">Hedef</span>
          <span class="d-val">
            {{ store.canonicalPending?.target_name }}
            <a class="link chg" @click="store.changeCanonicalTarget()">değiştir</a>
          </span>
        </div>
      </div>
      <div class="warn">
        <Icon name="info" :size="13" />
        <span>
          <b>Bu bir yönlendirme değildir.</b> Ziyaretçi yine eski sayfaya düşer; yalnızca
          Google'a "asıl sayfa şu" sinyali gider. Gerçek 301 için IdeaSoft panelini
          kullanmanız gerekir.
          <template v-if="store.canonicalPending?.will_create">
            Bu ürünün SEO kaydı yok, oluşturulacak.
          </template>
        </span>
      </div>

      <template #footer>
        <button class="ghost" :disabled="store.canonicalBusy" @click="store.cancelCanonical()">
          Vazgeç
        </button>
        <div style="flex: 1"></div>
        <button class="run" :disabled="store.canonicalBusy" @click="store.confirmCanonical()">
          <Icon
            :name="store.canonicalBusy ? 'loader' : 'check'"
            :size="14"
            :class="{ spin: store.canonicalBusy }"
          />
          {{ store.canonicalBusy ? "Yazılıyor…" : "Onaylıyorum, yaz" }}
        </button>
      </template>
    </ModalShell>
  </ToolShell>
</template>

<style scoped>
.head {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  margin-bottom: 12px;
  font-size: 12.5px;
  color: var(--c-text);
}
.note {
  flex: 1;
  margin: 0;
  max-width: 640px;
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--c-soft);
}
.succ-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 2px 8px;
  border: 1px dashed var(--c-border);
  border-radius: 7px;
  background: transparent;
  color: var(--c-soft);
  font-size: 11px;
  cursor: pointer;
  font-family: inherit;
}
.succ-btn:hover:not(:disabled) {
  background: var(--c-hover);
  color: var(--c-mid);
}
.succ-btn:disabled {
  opacity: 0.5;
  cursor: default;
}/* "Halef yok" NÖTR renkte: başarısızlık değil, geçerli bir cevap. *//* Sayfa düzeyi eylem (satır eylemi DEĞİL) — bu yüzden işlem sütununda değil, başlıkta.
   Geometri `.succ-btn` ile aynı; ikisi de modal içindeki arama düğmesiyle kardeş. */
.cat-sync {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 2px 8px;
  border: 1px dashed var(--c-border);
  border-radius: 7px;
  background: transparent;
  color: var(--c-soft);
  font-size: 11px;
  cursor: pointer;
  font-family: inherit;
  flex: none;
  align-self: flex-start;
}

/* Hedef seçme modali */
.pick-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
.pick-in {
  flex: 1;
  min-width: 0;
  padding: 7px 10px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-input);
  color: var(--c-text);
  font-size: 12px;
  font-family: inherit;
}
.pick-in:focus {
  outline: none;
  border-color: var(--accent);
}
.pick-hint {
  margin-top: 8px;
  color: var(--c-soft);
  font-size: 11px;
  line-height: 1.45;
}
.pick-list {
  margin-top: 10px;
  max-height: 260px;
  overflow-y: auto;
  border: 1px solid var(--c-border-soft);
  border-radius: 9px;
}
.pick-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 11px;
  border-bottom: 1px solid var(--c-border-soft);
  cursor: pointer;
  transition: background 0.12s cubic-bezier(0.32, 0.72, 0, 1);
}
.pick-item:last-child {
  border-bottom: 0;
}
.pick-item:hover {
  background: var(--c-hover);
}
.pi-name {
  color: var(--c-text);
  font-size: 12px;
  font-weight: 560;
}
.pi-slug {
  color: var(--c-soft);
  font-size: 11px;
  overflow-wrap: anywhere;
}

/* Onay modali: şu an / olacak / hedef farkı */
.diff {
  border: 1px solid var(--c-border-soft);
  border-radius: 9px;
  overflow: hidden;
}
.d-row {
  display: flex;
  gap: 10px;
  padding: 9px 12px;
  font-size: 12px;
  border-bottom: 1px solid var(--c-border-soft);
}
.d-row:last-child {
  border-bottom: 0;
}
/* Vurgu YAZILACAK satırda — `:last-child` olsaydı hedef satırı eklenince kayardı. */
.d-row.hi {
  background: var(--ok-soft-bg);
}
.d-lab {
  width: 52px;
  flex: none;
  color: var(--c-soft);
}
.d-val {
  color: var(--c-text);
  overflow-wrap: anywhere;
}
.d-val.muted {
  color: var(--c-faint);
}
.chg {
  margin-left: 8px;
  font-size: 11px;
}
.link {
  color: var(--accent);
  cursor: pointer;
  font-weight: 560;
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
.ghost:hover:not(:disabled) {
  background: var(--c-hover);
}
.ghost:disabled {
  opacity: 0.5;
  cursor: default;
}
.run {
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
  font-family: inherit;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
}
.run:hover:not(:disabled) {
  filter: brightness(1.06);
}
.run:disabled {
  opacity: 0.8;
  cursor: default;
}
</style>
