/**
 * Uygulamanın sayfa kaydı — kenar çubuğu menüsü ve üst şerit başlığı buradan üretilir.
 *
 * Neden: menü öğeleri Sidebar.vue'da, sayfa eşlemesi App.vue'da elle yazılıydı; yeni bir
 * sayfa eklemek iki dosyayı da elden geçirmeyi gerektiriyordu. Artık buraya bir satır yetiyor.
 *
 * `Page` tipi de bu listeden türetiliyor: kayda eklenmemiş bir sayfa anahtarı derlenmez.
 * Bilerek store'a bağımlı DEĞİL (döngüsel import olmasın diye) — duruma göre değişen üst
 * şerit alt metnini App.vue hesaplar.
 */

/**
 * Kenar çubuğu grupları. Menü 3 öğeden 9'a çıktığında düz bir liste "araç kutusu" değil
 * "uzun liste" gibi görünüyordu; başlıklar bunu hiyerarşiye çeviriyor.
 *
 * ⚠️ Grup adı bilinçli olarak "SEO Ayarları" DEĞİL: "Ayarlar" zaten bir sayfa, iki "ayar"
 * kavramı kenar çubuğunda yan yana durunca karışıyor.
 */
/// ⚠️ İlk grubun adı BOŞ (`""`) — Sidebar başlığı `v-if` ile atlıyor. "Bugün" menünün en
/// üstünde tek başına duruyor; "BUGÜN" başlığı altında "Bugün" öğesi gibi bir tekrar olmuyor.
export const GROUPS = ["", "Katalog", "SEO Araçları", "Asistan", "Sistem"] as const;
export type Group = (typeof GROUPS)[number];

export interface NavItem {
  /** store.page değeri */
  key: string;
  /** kenar çubuğunda görünen ad */
  label: string;
  /** Icon.vue'daki ikon adı */
  icon: string;
  /** üst şeritteki sayfa başlığı */
  title: string;
  /** üst şeritteki sabit alt metin (duruma bağlı olanları App.vue hesaplar) */
  sub?: string;
  group: Group;
}

export const NAV = [
  // Menünün en üstü: sabah açan kişinin ilk gördüğü ekran. Diğer ekranlar "nerede ne var"
  // sorusuna cevap verir, bu ekran "bugün ne yapayım" sorusuna.
  {
    key: "today",
    label: "Bugün",
    icon: "sun",
    title: "Bugün",
    sub: "Bugün dokunulacak işler — en çok kazandıracaklar önce",
    group: "",
  },

  { key: "products", label: "Ürünler", icon: "box", title: "Ürünler", group: "Katalog" },

  // SEO araçları: her analiz kendi ekranında. Sıra bilinçli — önce "nereye bakayım"
  // (Genel Bakış), sonra en çok iş çıkaran araç, sonra sorgu düzeyi analizler.
  {
    key: "overview",
    label: "Genel Bakış",
    icon: "layout",
    title: "Genel Bakış",
    sub: "Search Console analizinin özeti ve araçlar",
    group: "SEO Araçları",
  },
  {
    key: "opportunities",
    label: "Fırsatlar",
    icon: "search",
    title: "Fırsatlar",
    sub: "Konumuna göre hak ettiği tıklamayı alamayan ürünler",
    group: "SEO Araçları",
  },
  {
    key: "striking",
    label: "Yükselmeye yakın",
    icon: "trendUp",
    title: "Yükselmeye yakın sorgular",
    sub: "4–20. sıradaki aramalar — küçük iyileştirme ilk sayfaya taşıyabilir",
    group: "SEO Araçları",
  },
  {
    key: "cannibal",
    label: "Yarışan sayfalar",
    icon: "split",
    title: "Birbiriyle yarışan sayfalar",
    sub: "Aynı aramada birden çok sayfanız görünüyor, hiçbiri öne çıkamıyor",
    group: "SEO Araçları",
  },
  {
    key: "decay",
    label: "Düşüşte olanlar",
    icon: "trendDown",
    title: "Düşüşte olanlar",
    sub: "Önceki döneme göre trafik veya sıra kaybeden sayfalar",
    group: "SEO Araçları",
  },
  {
    key: "eol",
    label: "Satışta olmayanlar",
    icon: "archive",
    title: "Satışta olmayan ama trafik alan sayfalar",
    sub: "Ziyaretçi geliyor ama ürünü satın alamıyor",
    group: "SEO Araçları",
  },

  {
    key: "assistant",
    // ⚠️ Kenar çubuğu 190–238px; "Yapay Zekâ Asistanı" oraya sığmayıp iki satıra
    // kırılıyordu ve tek satırlık diğer öğelerin arasında düzeni bozuyordu.
    // Üst şerit başlığı tam adı taşımaya devam ediyor — orada yer var.
    label: "AI Asistanı",
    icon: "message",
    title: "Yapay Zekâ Asistanı",
    sub: "Analiz verinizi konuşarak sorgulayın — asistan hiçbir değişiklik yapmaz",
    group: "Asistan",
  },

  { key: "settings", label: "Ayarlar", icon: "settings", title: "Ayarlar", group: "Sistem" },
] as const satisfies readonly NavItem[];

export type Page = (typeof NAV)[number]["key"];

/**
 * Kaydın kendi (dar) öğe tipi. `NavItem` ile aynı şekle sahip ama `key` alanı `string`e
 * genişlemiyor — böylece `Sidebar` bir öğeye tıkladığında `store.page` ataması tip güvenli
 * kalıyor. `NavItem` yalnızca kaydın şeklini DOĞRULAMAK için var (`satisfies`).
 */
type NavEntry = (typeof NAV)[number];

export function titleOf(page: Page): string {
  return NAV.find((n) => n.key === page)?.title ?? "";
}

/** Sabit alt başlık; duruma bağlı olanları App.vue hesaplar. */
export function subOf(page: Page): string {
  const item = NAV.find((n) => n.key === page);
  return item && "sub" in item ? item.sub : "";
}

/**
 * Analiz verisi gösteren araç ekranları. Asistan "hangi ekranın verisiyle konuşuyorum"
 * sorusunu buradan cevaplıyor; Genel Bakış listede YOK çünkü orada satır verisi değil özet var.
 */
export const TOOL_PAGES: readonly Page[] = [
  "opportunities",
  "striking",
  "cannibal",
  "decay",
  "eol",
];

/** Kenar çubuğu için: boş grup çizilmesin diye yalnızca öğesi olanlar döner. */
export function groupedNav(): { group: Group; items: NavEntry[] }[] {
  return GROUPS.map((group) => ({
    group,
    items: NAV.filter((n): n is NavEntry => n.group === group),
  })).filter((g) => g.items.length > 0);
}
