<script setup lang="ts">
/**
 * "Feed verisi onayınızdan sonra değişti" uyarısı + **ne değiştiğinin** karşılaştırması.
 *
 * ⚠️ Saha geri bildirimi (2026-08-01): uyarı yalnızca alan ADINI söylüyordu ("görseller") ve
 * yanında "İçerik hâlâ doğru" düğmesi duruyordu. Kullanıcının haklı sorusu: *neye bakarak*
 * hâlâ doğru diyeceğim? Karar vermesi istenen kişiye kararın dayanağı da verilmeli.
 *
 * Karşılaştırma **onay anına** göre yapılıyor, son senkrona göre değil: arada iki değişiklik
 * olduysa ikisi de gösterilmeli.
 */
import { computed, ref, watch } from "vue";
import { api } from "../api";
import { useStore } from "../store";
import type { FeedDiff } from "../types";
import Icon from "./Icon.vue";

// Şablonda doğrudan kullanılıyor → değişkene atanmıyor.
defineProps<{ changed: string }>();

const store = useStore();
const open = ref(false);
const busy = ref(false);
const diff = ref<FeedDiff | null>(null);
const error = ref("");

// Ürün değişince açık panel yeni ürünün verisini göstermez — kapanıyor.
watch(
  () => store.selectedSku,
  () => {
    open.value = false;
    diff.value = null;
    error.value = "";
  },
);

async function toggle() {
  if (open.value) {
    open.value = false;
    return;
  }
  open.value = true;
  if (diff.value || !store.selectedSku) return;
  busy.value = true;
  error.value = "";
  try {
    diff.value = await api.getFeedDiff(store.selectedSku);
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}

/** Giden görseller: onaydaki listede olup şimdi olmayanlar. */
const gidenler = computed(() =>
  (diff.value?.images_old ?? []).filter((u) => !diff.value!.images_new.includes(u)),
);
/** Gelen görseller: şimdi olup onayda olmayanlar. */
const gelenler = computed(() =>
  (diff.value?.images_new ?? []).filter((u) => !diff.value!.images_old.includes(u)),
);
const gorselDegisti = computed(() => diff.value?.changed_fields.includes("görseller") ?? false);

/** Uzun metinlerde "kaç karakterden kaça" bilgisi tek başına bile yön veriyor. */
function uzunluk(f: { old: string; new: string }) {
  return `${f.old.length} → ${f.new.length} karakter`;
}
</script>

<template>
  <div class="feed-notice">
    <div class="warn feed-warn">
      <Icon name="refresh" :size="14" :stroke-width="2" />
      <span class="feed-warn-text">
        Bu ürünün feed verisi onayınızdan sonra değişti (<b>{{ changed }}</b>).
        Üretilmiş içerik ürünü artık doğru anlatmıyor olabilir.
      </span>
      <button class="feed-warn-btn" @click="toggle">
        {{ open ? "Gizle" : "Neler değişti?" }}
      </button>
      <button class="feed-warn-btn" @click="store.dismissFeedChange()">
        İçerik hâlâ doğru
      </button>
    </div>

    <div v-if="open" class="diff">
      <div v-if="busy" class="diff-info">
        <Icon name="loader" :size="13" class="spin" />
        Karşılaştırılıyor…
      </div>
      <div v-else-if="error" class="diff-info">
        <Icon name="alert" :size="13" />
        {{ error }}
      </div>

      <template v-else-if="diff">
        <!-- Onay kaydı olmayan ürünler: özellik eklenmeden önce onaylanmışlar. Boş bir
             karşılaştırma göstermek yanıltıcı olurdu; durum açıkça yazılıyor. -->
        <div v-if="!diff.has_snapshot" class="diff-info">
          <Icon name="info" :size="13" />
          Bu ürün, karşılaştırma özelliği eklenmeden önce onaylanmıştı — hangi alanların
          değiştiği biliniyor ama <b>önceki değerler kayıtlı değil</b>. Bundan sonraki
          değişikliklerde eski ve yeni değer yan yana görünecek.
        </div>

        <div v-for="f in diff.fields" :key="f.field" class="field">
          <div class="field-head">
            <span class="field-name">{{ f.field }}</span>
            <span class="field-len">{{ uzunluk(f) }}</span>
          </div>
          <div class="pair">
            <div class="side">
              <div class="side-label">Onayladığınız hâl</div>
              <div class="side-text om-scroll">{{ f.old || "—" }}</div>
            </div>
            <div class="side">
              <div class="side-label now">Şu anki feed</div>
              <div class="side-text om-scroll">{{ f.new || "—" }}</div>
            </div>
          </div>
        </div>

        <!-- Görseller metin olarak karşılaştırılamaz: gidenler ve gelenler gösteriliyor. -->
        <div v-if="gorselDegisti" class="field">
          <div class="field-head">
            <span class="field-name">görseller</span>
            <span class="field-len">
              <template v-if="diff.has_snapshot">
                {{ diff.images_old.length }} → {{ diff.images_new.length }} görsel
              </template>
              <template v-else>{{ diff.images_new.length }} görsel</template>
            </span>
          </div>

          <!-- ⚠️ Onay kaydı yokken "çıkanlar/gelenler" YAZILAMAZ: hangisinin yeni olduğunu
               bilmiyoruz. Hepsini "gelen" diye etiketlemek olmayan bir geçmişi uydurmak olur —
               tam da bu özelliğin kaçındığı şey. O durumda yalnızca şu anki hâl gösteriliyor. -->
          <template v-if="diff.has_snapshot">
            <div v-if="gidenler.length" class="img-row">
              <div class="side-label">Çıkanlar</div>
              <div class="thumbs">
                <img v-for="u in gidenler" :key="u" :src="u" class="thumb out" :title="u" />
              </div>
            </div>
            <div v-if="gelenler.length" class="img-row">
              <div class="side-label now">Gelenler</div>
              <div class="thumbs">
                <img v-for="u in gelenler" :key="u" :src="u" class="thumb in" :title="u" />
              </div>
            </div>
            <div v-if="!gidenler.length && !gelenler.length" class="diff-info">
              <Icon name="info" :size="13" />
              Görsel adresleri aynı kaldı, yalnızca sıraları değişti.
            </div>
          </template>
          <div v-else class="img-row">
            <div class="side-label">Şu anki görseller</div>
            <div class="thumbs">
              <img v-for="u in diff.images_new" :key="u" :src="u" class="thumb" :title="u" />
            </div>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
/* ⚠️ Üst boşluk BİLİNÇLİ olarak 16px: ölçüldüğünde üstte 0, altta 16px vardı ve şerit
   başlığa yapışık duruyordu. Alttaki 16px zaten sonraki satırın margin-top'undan geliyor
   (margin'ler birleşiyor), bu yüzden burada yalnızca üst veriliyor. */
.feed-notice {
  margin: 16px 0 0;
}
.feed-warn {
  flex-wrap: wrap;
}
.feed-warn-text {
  flex: 1;
  min-width: 220px;
  line-height: 1.45;
}
/* Eylem "yeniden üret" DEĞİL: değişikliği görmek üretimi zorunlu kılmıyor. Üretmek isteyen
   aşağıdaki kartlardan yapar; buradaki iki iş bakmak ve bayrağı düşürmek. */
.feed-warn-btn {
  flex: none;
  padding: 4px 10px;
  font-size: 11px;
  font-weight: 600;
  color: var(--warn-text);
  background: transparent;
  border: 1px solid var(--warn-border);
  border-radius: 7px;
  cursor: pointer;
  transition: background 0.18s cubic-bezier(0.32, 0.72, 0, 1);
}
.feed-warn-btn:hover {
  background: var(--warn-border);
}
.diff {
  margin-top: 8px;
  padding: 12px 14px;
  border: 1px solid var(--c-border-soft);
  border-radius: 10px;
  background: var(--c-card);
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.diff-info {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--c-soft);
}
.field-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 6px;
}
.field-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--c-text);
}
.field-len {
  font-size: 11px;
  color: var(--c-faint);
}
.pair {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}
/* Dar pencerede yan yana iki metin okunmuyor; alt alta geçiyor. */
@media (max-width: 1100px) {
  .pair {
    grid-template-columns: 1fr;
  }
}
.side-label {
  font-size: 10.5px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--c-faint);
  margin-bottom: 4px;
}
.side-label.now {
  color: var(--warn-icon);
}
.side-text {
  max-height: 150px;
  overflow-y: auto;
  padding: 8px 10px;
  border: 1px solid var(--c-border-soft);
  border-radius: 8px;
  background: var(--c-list);
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--c-mid);
  white-space: pre-wrap;
  word-break: break-word;
}
.img-row + .img-row {
  margin-top: 10px;
}
.thumbs {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.thumb {
  width: 62px;
  height: 62px;
  object-fit: cover;
  border-radius: 8px;
  border: 1.5px solid var(--c-border-soft);
  background: var(--c-list);
}
.thumb.out {
  opacity: 0.55;
}
.thumb.in {
  border-color: var(--warn-border);
}
</style>
