/**
 * Uygulamanın sayfa kaydı — kenar çubuğu menüsü ve üst şerit başlığı buradan üretilir.
 *
 * Neden: menü öğeleri Sidebar.vue'da, sayfa eşlemesi App.vue'da elle yazılıydı; yeni bir
 * sayfa eklemek iki dosyayı da elden geçirmeyi gerektiriyordu. Kuyrukta GSC fırsat analizi
 * ve onboarding sihirbazı var — artık buraya bir satır eklemek yetiyor.
 *
 * `Page` tipi de bu listeden türetiliyor: kayda eklenmemiş bir sayfa anahtarı derlenmez.
 * Bilerek store'a bağımlı DEĞİL (döngüsel import olmasın diye) — duruma göre değişen üst
 * şerit alt metnini App.vue hesaplar.
 */
export interface NavItem {
  /** store.page değeri */
  key: string;
  /** kenar çubuğunda görünen ad */
  label: string;
  /** Icon.vue'daki ikon adı */
  icon: string;
  /** üst şeritteki sayfa başlığı */
  title: string;
}

export const NAV = [
  { key: "products", label: "Ürünler", icon: "box", title: "Ürünler" },
  { key: "settings", label: "Ayarlar", icon: "settings", title: "Ayarlar" },
] as const satisfies readonly NavItem[];

export type Page = (typeof NAV)[number]["key"];

export function titleOf(page: Page): string {
  return NAV.find((n) => n.key === page)?.title ?? "";
}
