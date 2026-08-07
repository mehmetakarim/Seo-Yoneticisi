<script setup lang="ts">
/**
 * Araç ekranlarının ortak tablo/liste bileşeni (Faz B).
 *
 * ⚠️ **Neden var:** altı SEO araç ekranı kendi tablosunu ayrı ayrı kurmuştu. Ölçüldüğünde
 * `.head` 5, `.note` 4, `.more` 3 ekranda kopyalanmıştı ve `.sku` üst boşluğu bir ekranda
 * 1px, diğerinde 2px'e sapmıştı (brain.md 0at). Kartlarda aynı hastalık Kalem 1/2'de
 * görülmüştü — kopyalanan geometri zamanla sapıyor. Geometri artık **yalnızca burada**.
 *
 * Tasarım kaynağı: `design/faz-b-tablo-sistemi-handoff.zip` (Claude Design, "Tablo Sistemi").
 * Ölçüler oradan birebir: hücre dolgusu 9/12, işlem sütunu 156px, ikon düğme 26px,
 * grup girintisi 14px, geçiş 180ms `cubic-bezier(.32,.72,0,1)`.
 *
 * ⚠️ `<table>` değil **CSS grid**: sütun genişlikleri tür bazlı (`minmax`) tanımlanıyor ve
 * gruplu varyantta başlık satırı tüm genişliği kaplayabiliyor. Tabloyla ikisi de zor.
 */
import { computed } from "vue";
import Icon from "../Icon.vue";

export type ColType = "text" | "num" | "pct" | "badge" | "change" | "actions";
/** İşlem sütununa girebilecek eylemler. Yenisi `ACTIONS`'a eklenir, ekranlar değişmez. */
export type ActionKey = "open" | "successor" | "canonical" | "review" | "queue" | "redirect";

export interface TableCol {
  key: string;
  label: string;
  type: ColType;
  /** Sabit genişlik gerekiyorsa; yoksa tür varsayılanı kullanılır. */
  w?: string;
  /** Sayı sütununu vurgula: kayıp kırmızı, kazanç yeşil. */
  emphasis?: "up" | "down";
}
export interface TableAction {
  key: ActionKey;
  /** Bu satır için geçerli değilse: kaybolmaz, pasifleşir — satır ritmi korunur. */
  disabled?: boolean;
  tip?: string;
}
export interface TableRow {
  id: string;
  name?: string;
  /** Adın altındaki ikincil satır (SKU, URL veya halef sonucu). */
  sub?: string;
  /** İkincil satırın baloncuğu — uzun gerekçe metni satıra sığmadığında. */
  subTip?: string;
  /**
   * Verilirse ikincil satır TIKLANABİLİR olur ve bu eylemi yayar.
   *
   * ⚠️ Neden var: İşlem sütunu isimsiz ikonlardan oluşuyor ve bir satırda "sıradaki adım"
   * belirdiğinde (örn. halef önerisi geldi → şimdi canonical ayarlanacak) hangi ikona
   * basılacağı görünmüyordu. Sonucun YAZDIĞI yer, eylemin de durduğu yer olmalı.
   * (Satışta olmayanlar ekranında saha hatası, 2026-08-07.)
   */
  subAction?: ActionKey;
  values?: Record<string, string | number | null | undefined>;
  /** `tip` verilirse mevcut `[data-tip]` baloncuk sistemi kullanılır. */
  badges?: Record<string, { label: string; tone: string; tip?: string }>;
  changes?: Record<string, { from: string; to: string; tone?: "up" | "down" }>;
  actions?: TableAction[];
  selected?: boolean;
}
export interface Chip {
  label: string;
  /** Rozetteki sayı — **her zaman satır adedi**. Başka bir birim konursa kullanıcı
   *  çipe tıklayınca beklediği kadar satır göremez (saha hatası, 2026-08-07). */
  count?: number;
  /** Sıralama ölçütü gibi ek bilgi buraya; rozete karışmaz. */
  tip?: string;
  active?: boolean;
  onClick?: () => void;
}
export interface TableGroup {
  label: string;
  count?: string;
  meta?: string;
  rows: TableRow[];
}

const props = withDefaults(
  defineProps<{
    cols: TableCol[];
    rows?: TableRow[];
    /** Doluysa gruplu varyant çizilir (Yarışan sayfalar: bir sorgu → N sayfa). */
    groups?: TableGroup[] | null;
    state?: "normal" | "empty" | "loading" | "error";
    summary?: string;
    countLabel?: string;
    /**
     * Filtre çipleri, satır satır. Fırsatlar ekranında üç grup var (Kategori · Marka ·
     * durum); etiketsiz tek satır da geçerli — düz liste bir gruptur.
     */
    chipRows?: { label?: string; items: Chip[] }[];
    moreLabel?: string;
    emptyLabel?: string;
    emptyHint?: string;
    loadingLabel?: string;
    errorTitle?: string;
    errorDetail?: string;
  }>(),
  {
    rows: () => [],
    groups: null,
    state: "normal",
    summary: "",
    countLabel: "",
    chipRows: () => [],
    moreLabel: "",
    emptyLabel: "Bu görünümde kayıt yok",
    emptyHint: "Filtreleri genişletin veya yeni bir analiz çalıştırın.",
    loadingLabel: "Analiz sürüyor…",
    errorTitle: "Analiz tamamlanamadı",
    errorDetail: "",
  },
);

const emit = defineEmits<{
  row: [id: string];
  action: [payload: { id: string; key: ActionKey; danger: boolean }];
  more: [];
  retry: [];
}>();

/**
 * Eylem kataloğu — ikon, ipucu ve "dışarıya yazar mı" bilgisi tek yerde.
 *
 * ⚠️ `danger` yalnızca renk değil: mağazaya yazan, geri alınması yeni bir yazma gerektiren
 * eylemler. Çağıran ekran onay diyaloğunu bu bayrağa bakarak açar.
 */
const ACTIONS: Record<ActionKey, { tip: string; icon: string; danger?: boolean }> = {
  open: { tip: "Ürünü aç", icon: "external" },
  successor: { tip: "Halef öner", icon: "gitBranch" },
  canonical: { tip: "Canonical ayarla", icon: "link", danger: true },
  review: { tip: "Sonucu incele", icon: "chartLine" },
  queue: { tip: "Kuyruğa ekle", icon: "listPlus" },
  redirect: { tip: "301 listesine ekle", icon: "cornerUpRight", danger: true },
};

/** 5 yuva sığıyor; 6. eylemden itibaren son yuva "diğer eylemler" menüsüne dönüşür. */
const MAX_SLOTS = 5;

const sagaYasla = (t: ColType) => t === "num" || t === "pct" || t === "change" || t === "actions";

/**
 * Sütun türüne göre grid izi.
 *
 * 🔴 **KURAL: hiçbir iz içeriğe bağlı olamaz** (`min-content` · `max-content` · `auto` YOK).
 *
 * Sebebi yapısal: **her satır kendi grid konteyneri.** İçeriğe bağlı bir iz, her satırda O
 * SATIRIN içeriğine göre çözülür — "Çalışıldı" rozetli satırla "Dokunulmamış" rozetli satır
 * farklı genişlik alır ve tablo kayar. Başlık satırı da ayrı bir grid olduğu için sütun
 * başlıkları verinin üstüne oturmaz.
 *
 * Ölçüldü (2026-08-07, saha geri bildirimi): `min-content` tabanına geçince başlık x=512,
 * satırlar x=526 ve x=534 çıktı — üç farklı hizalama. Yalnızca **sabit px · % · fr** izleri
 * her satırda aynı çözülür.
 *
 * ⚠️ Bedeli kabul edilmiş: 10 sütunlu Fırsatlar dar pencerede yatay kaydırma açıyor. Hizasız
 * bir tablo, kaydırmalı bir tablodan daha kötü.
 */
function track(col: TableCol): string {
  if (col.type === "actions") return "156px";
  if (col.w) return col.w;
  if (col.type === "text") return "minmax(180px, 36%)";
  // Sabit: en uzun rozet "Dokunulmamış" (104px) + hücre dolgusu (24px) = 128px.
  if (col.type === "badge") return "132px";
  if (col.type === "change") return "minmax(104px, 1fr)";
  // 76px en geniş sayısal başlığı ("Gösterim" 75px) karşılıyor; artan yeri `1fr` dağıtıyor.
  return "minmax(76px, 1fr)";
}
/** Yatay kaydırmanın devreye gireceği eşik için kaba alt sınırlar (min-content tahmini 60px). */
const grid = computed(() => props.cols.map(track).join(" "));

/**
 * ⚠️ Burada **`min-width` YOK** — bilinçli.
 *
 * Önce elle hesaplanan bir toplam (`sum(MIN[type])`) inline `min-width` olarak veriliyordu.
 * Bu, grid'in zaten yaptığı işin ikinci ve DAHA KÖTÜ bir modeliydi: Fırsatlar ekranı 10
 * sütuna çıkınca tahmin 972px dedi, gerçek içerik 865px'ti ve ekran boşuna yatay kaydırma
 * açtı (ölçüldü, 2026-08-07). Grid izleri `min-content` tabanlı olduğu için sütunlar zaten
 * içeriğin altına inemiyor; toplam kapsayıcıyı aşarsa `overflow-x` kendiliğinden devreye
 * giriyor. Tek doğruluk kaynağı grid.
 */

const isNormal = computed(() => props.state === "normal" || props.state === "error");
const isEmpty = computed(() => props.state === "empty");
const isLoading = computed(() => props.state === "loading");
const isError = computed(() => props.state === "error");

/** Gruplu varyant düz bir listeye açılır: grup başlığı + satırlar tek akışta. */
type BodyItem =
  | { kind: "group"; label: string; count?: string; meta?: string }
  | { kind: "row"; row: TableRow; indent: boolean };

const body = computed<BodyItem[]>(() => {
  if (props.groups) {
    const out: BodyItem[] = [];
    for (const g of props.groups) {
      out.push({ kind: "group", label: g.label, count: g.count, meta: g.meta });
      for (const r of g.rows) out.push({ kind: "row", row: r, indent: true });
    }
    return out;
  }
  return props.rows.map((row) => ({ kind: "row" as const, row, indent: false }));
});

/** Satırın eylemleri; taşma varsa son yuva "⋯" olur. */
function actionsOf(row: TableRow) {
  const defs = row.actions ?? [];
  const tasma = defs.length > MAX_SLOTS;
  const gorunen = tasma ? defs.slice(0, MAX_SLOTS - 1) : defs;
  const items = gorunen.map((a) => {
    const meta = ACTIONS[a.key];
    return {
      key: a.key,
      icon: meta.icon,
      danger: !!meta.danger,
      disabled: !!a.disabled,
      // Pasif düğmede sebebi söylemek gerekiyor; yoksa "bozuk mu?" diye düşünülüyor.
      tip: a.disabled
        ? `${a.tip ?? meta.tip} · bu satır için geçerli değil`
        : (a.tip ?? meta.tip),
    };
  });
  if (tasma) {
    items.push({ key: "more" as ActionKey, icon: "dots", danger: false, disabled: false, tip: "Diğer eylemler" });
  }
  return items;
}

function tetikle(row: TableRow, a: { key: ActionKey; danger: boolean; disabled: boolean }) {
  if (a.disabled) return;
  emit("action", { id: row.id, key: a.key, danger: a.danger });
}

function metin(row: TableRow, col: TableCol, ilk: boolean): string {
  if (ilk) return row.name ?? "";
  const v = row.values?.[col.key];
  return v === undefined || v === null || v === "" ? "" : String(v);
}
function sayi(row: TableRow, col: TableCol): string {
  const v = row.values?.[col.key];
  return v === undefined || v === null || v === "" ? "—" : String(v);
}

/** İskelet satırları sütun genişliklerini korur — tablo yükleme sırasında yerinden oynamaz. */
const skeletons = computed(() =>
  [0, 1, 2, 3, 4].map((i) => ({
    i,
    cells: props.cols.map((col, ci) => {
      const w = col.type === "text" ? ["62%", "74%", "55%", "68%"] : col.type === "actions" ? ["66px"] : ["38px", "46px", "30px"];
      return { w: w[(i + ci) % w.length], text: col.type === "text", opacity: 1 - i * 0.13 };
    }),
  })),
);
</script>

<template>
  <div class="tbl-shell">
    <!-- üst şerit: özet + sayaç + filtre çipleri -->
    <div v-if="summary" class="strip">
      <div class="strip-top">
        <div class="summary">{{ summary }}</div>
        <span v-if="countLabel" class="count">{{ countLabel }}</span>
      </div>
      <div v-for="(cr, ri) in chipRows" :key="ri" class="chips">
        <span v-if="cr.label" class="chips-label">{{ cr.label }}</span>
        <button
          v-for="ch in cr.items"
          :key="ch.label"
          class="chip"
          :class="{ on: ch.active, 'tip-below': !!ch.tip }"
          :data-tip="ch.tip"
          @click="ch.onClick?.()"
        >
          <span>{{ ch.label }}</span>
          <span v-if="ch.count !== undefined" class="chip-count">{{ ch.count }}</span>
        </button>
      </div>
    </div>

    <!-- hata şeridi: KALICI ve son başarılı veriyi gizlemez -->
    <div v-if="isError" class="err">
      <Icon name="alert" :size="15" :stroke-width="1.9" class="err-icon" />
      <div class="err-body">
        <div class="err-title">{{ errorTitle }}</div>
        <div v-if="errorDetail" class="err-detail">{{ errorDetail }}</div>
      </div>
      <button class="err-btn" @click="emit('retry')">Yeniden dene</button>
    </div>

    <div class="scroll om-scroll-x">
      <div>
        <div v-if="!isEmpty" class="head" :style="{ gridTemplateColumns: grid }">
          <div
            v-for="col in cols"
            :key="col.key"
            class="th"
            :class="{ right: sagaYasla(col.type) }"
          >
            {{ col.label }}
          </div>
        </div>

        <template v-if="isNormal">
          <template v-for="(item, i) in body" :key="item.kind === 'row' ? item.row.id : `g${i}`">
            <div v-if="item.kind === 'group'" class="group">
              <div class="group-left">
                <Icon name="search" :size="13" :stroke-width="1.9" class="group-icon" />
                <span class="group-label">{{ item.label }}</span>
                <span v-if="item.count" class="group-count">{{ item.count }}</span>
              </div>
              <span v-if="item.meta" class="group-meta">{{ item.meta }}</span>
            </div>

            <div
              v-else
              class="row"
              :class="{ sel: item.row.selected }"
              :style="{ gridTemplateColumns: grid }"
              @click="emit('row', item.row.id)"
            >
              <div
                v-for="(col, ci) in cols"
                :key="col.key"
                class="td"
                :class="{ right: sagaYasla(col.type) }"
              >
                <!-- metin: ad + ikincil satır -->
                <template v-if="col.type === 'text'">
                  <div :class="{ indent: ci === 0 && item.indent }">
                    <div class="cell-name">{{ metin(item.row, col, ci === 0) }}</div>
                    <component
                      :is="item.row.subAction ? 'button' : 'div'"
                      v-if="ci === 0 && item.row.sub"
                      class="cell-sub"
                      :class="{ 'tip-below': !!item.row.subTip, link: !!item.row.subAction }"
                      :data-tip="item.row.subTip"
                      @click.stop="
                        item.row.subAction &&
                          emit('action', { id: item.row.id, key: item.row.subAction, danger: false })
                      "
                    >
                      {{ item.row.sub }}
                    </component>
                  </div>
                </template>

                <!-- sayı / yüzde -->
                <span
                  v-else-if="col.type === 'num' || col.type === 'pct'"
                  class="cell-num"
                  :class="col.emphasis"
                >{{ sayi(item.row, col) }}</span>

                <!-- rozet -->
                <span
                  v-else-if="col.type === 'badge'"
                  class="cell-badge"
                  :class="{ 'tip-below': !!item.row.badges?.[col.key]?.tip }"
                  :data-tip="item.row.badges?.[col.key]?.tip"
                  :style="{
                    background: `var(--badge-${item.row.badges?.[col.key]?.tone ?? 'tamamlandi'}-bg)`,
                    color: `var(--badge-${item.row.badges?.[col.key]?.tone ?? 'tamamlandi'}-c)`,
                  }"
                >
                  <span class="dot"></span>{{ item.row.badges?.[col.key]?.label ?? "—" }}
                </span>

                <!-- değişim: önce soluk → sonra vurgulu -->
                <span v-else-if="col.type === 'change'" class="cell-change">
                  <span class="from">{{ item.row.changes?.[col.key]?.from ?? "—" }}</span>
                  <Icon name="arrowRight" :size="11" :stroke-width="1.9" class="arrow" />
                  <span class="to" :class="item.row.changes?.[col.key]?.tone">
                    {{ item.row.changes?.[col.key]?.to ?? "—" }}
                  </span>
                </span>

                <!-- işlem sütunu -->
                <div v-else-if="col.type === 'actions'" class="cell-actions">
                  <button
                    v-for="a in actionsOf(item.row)"
                    :key="a.key"
                    class="act"
                    :class="{ danger: a.danger, off: a.disabled }"
                    :disabled="a.disabled"
                    :title="a.tip"
                    @click.stop="tetikle(item.row, a)"
                  >
                    <Icon :name="a.icon" :size="14" :stroke-width="1.9" />
                  </button>
                </div>
              </div>
            </div>
          </template>
        </template>

        <!-- boş: sakin, suçlayıcı değil -->
        <div v-else-if="isEmpty" class="empty">
          <Icon name="trendUp" :size="24" :stroke-width="1.9" />
          <div class="empty-label">{{ emptyLabel }}</div>
          <div class="empty-hint">{{ emptyHint }}</div>
        </div>

        <template v-else-if="isLoading">
          <div
            v-for="sk in skeletons"
            :key="sk.i"
            class="row skel"
            :style="{ gridTemplateColumns: grid }"
          >
            <div v-for="(b, bi) in sk.cells" :key="bi" class="td" :class="{ right: !b.text }">
              <span class="bar" :style="{ width: b.w, opacity: b.opacity }"></span>
            </div>
          </div>
          <div class="loading">
            <Icon name="loader" :size="14" :stroke-width="2.2" class="spin" />
            {{ loadingLabel }}
          </div>
        </template>
      </div>
    </div>

    <button v-if="moreLabel" class="more" @click="emit('more')">{{ moreLabel }}</button>
  </div>
</template>

<style scoped>
.tbl-shell {
  border: 1px solid var(--c-border-soft);
  border-radius: 12px;
  background: var(--c-card);
  overflow: hidden;
}

/* ---- üst şerit ---- */
.strip {
  padding: 12px 14px;
  border-bottom: 1px solid var(--c-border-soft);
  background: var(--c-list);
}
.strip-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.summary {
  font-size: 12.5px;
  color: var(--c-mid);
}
.count {
  font-size: 11px;
  color: var(--c-faint);
  font-variant-numeric: tabular-nums;
}
.chips {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  margin-top: 10px;
}
.chips-label {
  font-size: 11px;
  color: var(--c-faint);
  flex: none;
  align-self: center;
  min-width: 52px;
}
/* ⚠️ `.chip` / `.chip-count` burada DEĞİL, `styles.css`te — asistanın kaynak çipleriyle
   aynı geometriyi paylaşıyorlar. Buraya kopyalanırsa iki tanım zamanla ayrışır; bu oturumda
   kopyalanan ölçülerin saptığı üç kez ölçüldü (`.sku`, elle hesaplanan `min-width`,
   içeriğe bağlı grid izleri). Değişiklik gerekiyorsa global tanımda yapın. */

/* ---- hata ---- */
.err {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 11px 14px;
  background: var(--badge-eksik-bg);
  border-bottom: 1px solid var(--badge-eksik-bg);
}
.err-icon {
  flex: none;
  margin-top: 1px;
  color: var(--badge-eksik-c);
}
.err-body {
  flex: 1;
  min-width: 0;
}
.err-title {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--badge-eksik-c);
}
.err-detail {
  font-size: 11.5px;
  color: var(--badge-eksik-c);
  opacity: 0.85;
  margin-top: 2px;
}
.err-btn {
  flex: none;
  height: 28px;
  padding: 0 11px;
  border: 1px solid var(--badge-eksik-c);
  border-radius: 7px;
  background: var(--c-card);
  color: var(--badge-eksik-c);
  font-size: 11.5px;
  font-weight: 580;
  cursor: pointer;
}

/* ---- gövde ---- */
/* ⚠️ `overflow-y: hidden` ZORUNLU. CSS'te bir eksen `visible` değilse diğeri de `visible`
   kalamaz: yalnızca `overflow-x: auto` yazınca tarayıcı `overflow-y`'yi de `auto` yapıyor ve
   yatay çubuğun kapladığı 17px yüzünden İKİNCİ bir dikey çubuk beliriyor (ölçüldü: Fırsatlar
   ekranında 28px yatay taşma → 17px dikey taşma). Dikey kaydırma sayfanın işi, tablonun değil. */
.scroll {
  overflow-x: auto;
  overflow-y: hidden;
}
.head {
  display: grid;
  border-bottom: 1px solid var(--c-border-soft);
  background: var(--c-card);
}
.th {
  padding: 9px 12px;
  font-size: 11px;
  font-weight: 620;
  color: var(--c-soft);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.th.right,
.td.right {
  text-align: right;
}
.row {
  display: grid;
  align-items: center;
  border-bottom: 1px solid var(--c-border-soft);
  cursor: pointer;
  transition: background 0.18s cubic-bezier(0.32, 0.72, 0, 1);
}
.row:hover {
  background: var(--c-hover);
}
.row.sel {
  background: var(--accent-tint);
}
.td {
  padding: 9px 12px;
  min-width: 0;
  /* Hücre içine sonradan eklenecek metin de doğru boyutu miras alsın. */
  font-size: 12.5px;
}
.indent {
  padding-left: 14px;
}
.cell-name {
  font-size: 12.5px;
  color: var(--c-text);
  line-height: 1.35;
}
.cell-sub {
  font-size: 10.5px;
  color: var(--c-faint);
  margin-top: 1px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* Tıklanabilir sonuç satırı — `subAction` verilmişse. Düğme görünümü değil, bağlantı
   görünümü: satırın içinde ikinci bir düğme gövdesi gürültü olurdu. */
button.cell-sub {
  display: block;
  max-width: 100%;
  padding: 0;
  border: none;
  background: transparent;
  font-family: inherit;
  text-align: left;
}
.cell-sub.link {
  color: var(--accent);
  font-weight: 560;
  cursor: pointer;
}
.cell-sub.link:hover {
  text-decoration: underline;
}
.cell-num {
  font-size: 12.5px;
  color: var(--c-text);
  font-variant-numeric: tabular-nums;
}
.cell-num.down {
  color: var(--red);
  font-weight: 600;
}
.cell-num.up {
  color: var(--green);
  font-weight: 600;
}
.cell-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 2px 8px;
  border-radius: 20px;
  font-size: 10.5px;
  font-weight: 600;
  white-space: nowrap;
}
.dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: currentColor;
  flex: none;
}
.cell-change {
  display: inline-flex;
  align-items: center;
  justify-content: flex-end;
  gap: 5px;
  font-size: 12.5px;
  font-variant-numeric: tabular-nums;
}
.cell-change .from,
.cell-change .arrow {
  color: var(--c-faint);
}
.cell-change .to {
  font-weight: 600;
  color: var(--c-text);
}
.cell-change .to.down {
  color: var(--red);
}
.cell-change .to.up {
  color: var(--green);
}

/* ---- işlem sütunu ---- */
.cell-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 4px;
}
.act {
  width: 26px;
  height: 26px;
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--c-soft);
  cursor: pointer;
  transition: background 0.18s cubic-bezier(0.32, 0.72, 0, 1),
    color 0.18s cubic-bezier(0.32, 0.72, 0, 1);
}
.act:hover:not(:disabled) {
  background: var(--c-hover);
  color: var(--c-text);
}
/* Dışarıya yazan eylem kırmızı: tıklamadan önce ne olduğu anlaşılmalı. */
.act.danger {
  color: var(--red);
}
.act.danger:hover:not(:disabled) {
  background: var(--badge-eksik-bg);
  color: var(--red);
}
.act.off {
  opacity: 0.32;
  cursor: default;
}

/* ---- durumlar ---- */
.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 52px 24px;
  color: var(--c-faint);
}
.empty-label {
  font-size: 12.5px;
  color: var(--c-soft);
}
.empty-hint {
  font-size: 11.5px;
}
.skel {
  cursor: default;
}
.skel:hover {
  background: transparent;
}
.bar {
  display: block;
  height: 9px;
  border-radius: 5px;
  background: var(--c-track);
}
.td.right .bar {
  margin-left: auto;
}
.loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 11px 16px;
  color: var(--c-soft);
  font-size: 11.5px;
}
.loading :deep(svg) {
  color: var(--accent);
}

/* ---- gruplu varyant ---- */
.group {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 12px;
  background: var(--c-list);
  border-bottom: 1px solid var(--c-border-soft);
}
.group-left {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.group-icon {
  flex: none;
  color: var(--c-soft);
}
.group-label {
  font-size: 12.5px;
  font-weight: 640;
  color: var(--c-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.group-count {
  font-size: 10.5px;
  color: var(--c-faint);
  flex: none;
}
.group-meta {
  font-size: 11px;
  color: var(--c-soft);
  font-variant-numeric: tabular-nums;
  flex: none;
}

/* ---- kısmi liste ---- */
.more {
  width: 100%;
  padding: 10px 16px;
  border: none;
  border-top: 1px solid var(--c-border-soft);
  background: var(--c-card);
  color: var(--accent);
  font-size: 12px;
  font-weight: 580;
  cursor: pointer;
  transition: background 0.18s cubic-bezier(0.32, 0.72, 0, 1);
}
.more:hover {
  background: var(--c-hover);
}
</style>
