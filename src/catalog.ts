/**
 * Katalog sayfa tipleri — kategori · marka · içerik (Faz İ2).
 *
 * **Neden ayrı dosya.** Üç ekran tek bileşenle çiziliyor ve bileşen "hangi tipim?" sorusunu
 * `store.page`den cevaplıyor. Bu eşleme store'da, ekranda ve içerik açığı yönlendirmesinde
 * ayrı ayrı yazılsaydı üç kopya olurdu — bu projede kopyalanan tablo beş kez saptı.
 *
 * ⚠️ Arka uçtaki `kind` değerleriyle birebir aynı olmak zorunda
 * (`ideasoft::STORE_PAGE_KINDS`: category · brand · blog). Buradaki bir yazım hatası
 * derlenir ama sessizce boş liste döndürür.
 */
import type { Page } from "./navigation";

export const KIND_BY_PAGE: Partial<Record<Page, string>> = {
  categories: "category",
  brands: "brand",
  blogs: "blog",
};

export const PAGE_BY_KIND: Record<string, Page> = {
  category: "categories",
  brand: "brands",
  blog: "blogs",
};

/**
 * Ekranda görünen tip adı.
 *
 * ⚠️ Blog için "İçerik": IdeaSoft'ta kayıt tipi blog ama kullanıcı için bunlar tek tek
 * yazılar. Menüde de "İçerikler" yazıyor — aynı şeye iki ad vermiyoruz.
 */
export const TIP_AD: Record<string, string> = {
  category: "Kategori",
  brand: "Marka",
  blog: "İçerik",
};

/**
 * Tanıtım metni alanı bu tipte var mı.
 *
 * 🔴 Blogda alan **hiç yok** (ölçüldü: 250 blog kaydının 250'sinde boş, çünkü IdeaSoft
 * blog uçlarında `showcaseContent` diye bir alan dönmüyor). Arka uç da göndermiyor
 * (`put_store_page`, `showcase: None`) — gönderirse uç 400 verebilir.
 */
export function tanitimMetniVar(kind: string): boolean {
  return kind !== "blog";
}
