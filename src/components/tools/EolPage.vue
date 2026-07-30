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
</script>

<template>
  <ToolShell
    :empty="!rows.length"
    empty-text="Katalog dışı sayfa trafiği yok — Google'da sıralanan her ürün sayfası satışta."
  >
    <div class="head">
      <div class="h-text">
        <b>{{ rows.length }}</b> sayfa
        <span class="cost">{{ clicks }} tıklama</span>
        <p class="note">
          Bu adresler kataloğunuzda yok ama Google'da hâlâ sıralanıyor — ziyaretçi geliyor,
          ürünü satın alamıyor. Çözüm genelde güncel nesle <b>301 yönlendirme</b>;
          yönlendirmeyi IdeaSoft panelinden tanımlamanız gerekir, uygulama bunu yapamaz.
          Bazı sayfaları bilinçli tutuyor olabilirsiniz — liste öneridir, karar sizin.
        </p>
      </div>
      <button
        class="succ-btn cat-sync"
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

    <div class="card">
      <table class="tbl">
        <thead>
          <tr>
            <th class="c-name">Sayfa</th>
            <th class="c-num">Tıklama</th>
            <th class="c-num">Gösterim</th>
            <th class="c-num">Konum</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="e in rows.slice(0, limit)" :key="e.url">
            <td class="c-name">
              <div class="nm">{{ e.slug }}</div>
              <!-- Halef önerisi: İSTEK ÜZERİNE, tek sayfa için. 2.190 sayfanın tamamı için
                   model çağırmak günlük kotayı (flash 20/gün) anında tüketirdi. -->
              <div class="succ">
                <button
                  v-if="!store.successors[e.url]"
                  class="succ-btn"
                  :disabled="!!store.successorBusy"
                  @click="store.suggestSuccessor(e.url)"
                >
                  <Icon
                    :name="store.successorBusy === e.url ? 'loader' : 'sparkles'"
                    :size="11"
                    :class="{ spin: store.successorBusy === e.url }"
                  />
                  {{ store.successorBusy === e.url ? "Bakılıyor…" : "Halef öner" }}
                </button>
                <template v-else>
                  <span v-if="store.successors[e.url].sku" class="succ-ok">
                    <Icon name="check" :size="11" :stroke-width="2.6" />
                    {{ store.successors[e.url].name }}
                  </span>
                  <span v-else class="succ-none">Uygun halef bulunamadı</span>
                  <!-- Halef bulunmasa da buton görünür: modelin halef bulamaması hedefin
                       olmadığı anlamına gelmiyor, karar zaten operatörde. -->
                  <button
                    class="succ-btn"
                    :disabled="store.canonicalBusy"
                    @click="store.startCanonical(e.slug, targetOf(e.url))"
                  >
                    <Icon name="upload" :size="11" />
                    {{ targetOf(e.url) ? "Canonical ayarla" : "Hedef seç ve ayarla" }}
                  </button>
                  <span class="succ-why">{{ store.successors[e.url].reason }}</span>
                </template>
              </div>
            </td>
            <td class="c-num miss">{{ Math.round(e.clicks) }}</td>
            <td class="c-num">{{ Math.round(e.impressions) }}</td>
            <td class="c-num">{{ e.position.toFixed(1) }}</td>
          </tr>
        </tbody>
      </table>
      <div v-if="rows.length > limit" class="more">
        <a class="link" @click="limit += 50">
          Sonraki 50'yi göster ({{ rows.length - limit }} kaldı)
        </a>
      </div>
    </div>

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
.h-text {
  min-width: 0;
}
.head b {
  font-weight: 660;
  font-variant-numeric: tabular-nums;
}
.cost {
  margin-left: 8px;
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--warn-bg);
  color: var(--warn-text);
  font-size: 11px;
  font-weight: 640;
  font-variant-numeric: tabular-nums;
}
.note {
  margin: 6px 0 0;
  max-width: 640px;
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--c-soft);
}
.more {
  padding: 10px 16px;
  border-top: 1px solid var(--c-border-soft);
  font-size: 12px;
}

/* Halef önerisi + canonical eylem satırı */
.succ {
  margin-top: 5px;
  display: flex;
  align-items: baseline;
  flex-wrap: wrap;
  gap: 6px;
  font-size: 11px;
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
}
.succ-ok {
  display: inline-flex;
  /* Metin sarınca ikon ortada kalmasın — üste hizalı dursun. */
  align-items: flex-start;
  gap: 5px;
  color: var(--green);
  font-weight: 580;
  line-height: 1.4;
}
.succ-ok svg {
  flex: none;
  margin-top: 2px;
}
/* "Halef yok" NÖTR renkte: başarısızlık değil, geçerli bir cevap. */
.succ-none {
  color: var(--c-soft);
  font-weight: 560;
}
.succ-why {
  color: var(--c-faint);
  line-height: 1.4;
}
.cat-sync {
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
