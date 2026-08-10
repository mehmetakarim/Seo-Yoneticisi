<script setup lang="ts">
/**
 * CSV içe aktarma — sütun eşleştirme + önizleme (Faz C2).
 *
 * 🔴 **Sabit sütun şeması yok.** Uygulama kişiselleştirilmemiş: hangi mağazanın hangi CSV'yi
 * vereceği bilinmiyor. Uygulama başlıklardan bir tahmin üretiyor, son sözü kullanıcı söylüyor.
 *
 * ⚠️ **Önizleme atlanamaz bir adım.** 300 satırlık yanlış eşleşmiş bir aktarım geri alınamaz;
 * beş satır göstermenin maliyeti bir ekran, faydası bir felaketin önlenmesi.
 */
import { ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "../../api";
import { useStore } from "../../store";
import type { CsvPreview, ImportSummary } from "../../types";
import Icon from "../Icon.vue";
import ModalShell from "../ModalShell.vue";

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ close: [] }>();
const store = useStore();

const yol = ref("");
const on = ref<CsvPreview | null>(null);
const esleme = ref<(number | null)[]>([]);
const yukleniyor = ref(false);
const ozet = ref<ImportSummary | null>(null);
const hata = ref("");

const AYRAC_ADI: Record<string, string> = {
  ";": "noktalı virgül",
  ",": "virgül",
  "\t": "sekme",
};

async function dosyaSec() {
  hata.value = "";
  ozet.value = null;
  const p = await open({ filters: [{ name: "CSV", extensions: ["csv", "txt"] }] });
  if (typeof p !== "string") return;
  yol.value = p;
  try {
    on.value = await api.previewContactCsv(p);
    esleme.value = [...on.value.mapping];
  } catch (e) {
    // Kodlama hatası burada çıkıyor ve ne yapılacağını söylüyor (Excel → CSV UTF-8).
    hata.value = String(e);
    on.value = null;
  }
}

async function aktar() {
  if (!on.value) return;
  yukleniyor.value = true;
  try {
    ozet.value = await api.importContactsCsv(yol.value, esleme.value);
    await store.loadContacts();
    void store.loadToday();
  } catch (e) {
    hata.value = String(e);
  } finally {
    yukleniyor.value = false;
  }
}

/** Ad eşleşmemişse aktarım anlamsız: adsız satırlar zaten atlanacak. */
const adEslesti = () => esleme.value[0] !== null && esleme.value[0] !== undefined;

function kapat() {
  on.value = null;
  ozet.value = null;
  hata.value = "";
  yol.value = "";
  emit("close");
}
</script>

<template>
  <ModalShell
    :open="props.open"
    label="Kişileri CSV'den içe aktar"
    title="CSV'den içe aktar"
    icon="upload"
    :width="640"
    scroll
    :closable="!yukleniyor"
    @close="kapat"
  >
    <template #sub>
      Excel'den dışa aktardığınız listeyi yükleyin; sütunları siz eşlersiniz.
    </template>

    <p v-if="hata" class="warn">{{ hata }}</p>

    <!-- 1. adım: dosya -->
    <button v-if="!on" class="solid" @click="dosyaSec">
      <Icon name="upload" :size="14" :stroke-width="2" /> Dosya seç
    </button>

    <template v-if="on && !ozet">
      <p class="bilgi">
        <b>{{ on.total_rows }} satır</b> okundu ·
        {{ AYRAC_ADI[on.delimiter] ?? on.delimiter }} ile ayrılmış · {{ on.headers.length }} sütun
      </p>

      <!-- 2. adım: eşleştirme -->
      <div class="es">
        <div v-for="(f, i) in on.fields" :key="f[0]" class="es-sat">
          <span class="es-ad">{{ f[1] }}<b v-if="i === 0" class="zor">*</b></span>
          <select
            class="fx"
            :value="esleme[i] ?? ''"
            @change="
              esleme[i] =
                ($event.target as HTMLSelectElement).value === ''
                  ? null
                  : Number(($event.target as HTMLSelectElement).value)
            "
          >
            <option value="">— aktarma —</option>
            <option v-for="(h, hi) in on.headers" :key="hi" :value="hi">{{ h }}</option>
          </select>
        </div>
      </div>

      <!-- 3. adım: önizleme — atlanamaz. -->
      <div class="on-bas">İlk {{ on.rows.length }} satır böyle aktarılacak</div>
      <div class="tablo om-scroll-x">
        <table>
          <thead>
            <tr>
              <th v-for="f in on.fields" :key="f[0]">{{ f[1] }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(r, ri) in on.rows" :key="ri">
              <td v-for="(f, fi) in on.fields" :key="f[0]">
                {{ esleme[fi] === null || esleme[fi] === undefined ? "—" : r[esleme[fi]!] || "—" }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <p v-if="!adEslesti()" class="warn">
        Ad sütunu eşleşmedi. Adsız satırlar aktarılmaz — kişiyi listede ayırt edecek tek alan o.
      </p>
      <div class="bosluk"></div>
      <p class="hint">
        <Icon name="info" :size="13" style="flex: none" />
        <span>
          Aynı e-posta veya telefona sahip kişi varsa <b>yeni kayıt açılmaz, mevcut kayıt
          güncellenir</b>. CSV'de boş bıraktığınız sütunlar uygulamadaki bilgiyi silmez.
        </span>
      </p>
    </template>

    <!-- 4. adım: sonuç -->
    <div v-if="ozet" class="sonuc">
      <Icon name="check" :size="18" :stroke-width="2.4" />
      <div>
        <b>{{ ozet.added }} eklendi</b> · {{ ozet.updated }} güncellendi
        <template v-if="ozet.skipped"> · {{ ozet.skipped }} atlandı</template>
        <div v-if="ozet.skip_reason" class="sonuc-alt">{{ ozet.skip_reason }}</div>
      </div>
    </div>

    <template #footer>
      <button class="ghost" @click="kapat">{{ ozet ? "Kapat" : "Vazgeç" }}</button>
      <button
        v-if="on && !ozet"
        class="solid"
        :disabled="yukleniyor || !adEslesti()"
        @click="aktar"
      >
        {{ yukleniyor ? "Aktarılıyor…" : `${on.total_rows} satırı aktar` }}
      </button>
    </template>
  </ModalShell>
</template>

<style scoped>
.bilgi {
  font-size: 12.5px;
  color: var(--c-mid);
  margin: 0 0 12px;
}
.es {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
  gap: 8px 14px;
  margin-bottom: 16px;
}
.es-sat {
  display: flex;
  align-items: center;
  gap: 8px;
}
.es-ad {
  flex: none;
  width: 78px;
  font-size: 12px;
  color: var(--c-faint);
  font-weight: 560;
}
.zor {
  color: var(--warn-text);
  margin-left: 2px;
}
select {
  flex: 1;
  min-width: 0;
  padding: 6px 8px;
  border: 1px solid var(--c-border);
  border-radius: 7px;
  background: var(--c-input);
  color: var(--c-text);
  font-size: 12.5px;
  font-family: inherit;
  outline: none;
}
.on-bas {
  font-size: 12px;
  font-weight: 600;
  color: var(--c-text);
  margin-bottom: 7px;
}
.tablo {
  overflow-x: auto;
  border: 1px solid var(--c-border);
  border-radius: 9px;
}
table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
th,
td {
  text-align: left;
  padding: 7px 10px;
  border-bottom: 1px solid var(--c-border);
  white-space: nowrap;
  max-width: 170px;
  overflow: hidden;
  text-overflow: ellipsis;
}
th {
  color: var(--c-faint);
  font-weight: 600;
  background: var(--c-list);
}
tbody tr:last-child td {
  border-bottom: none;
}
.bosluk {
  height: 10px;
}
.hint {
  align-items: flex-start;
  line-height: 1.55;
}
.sonuc {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 13px 14px;
  border-radius: 10px;
  background: var(--badge-uygun-bg);
  color: var(--badge-uygun-c);
  font-size: 13px;
}
.sonuc-alt {
  font-size: 11.5px;
  opacity: 0.8;
  margin-top: 2px;
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
.solid:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
