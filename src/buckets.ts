/**
 * Kova etiketleri ve rozet tonları — Bugün ekranı ile odak çubuğu ortak kullanıyor.
 *
 * ⚠️ **Neden ayrı dosya:** bu iki tablo `TodayPage.vue` ve `FocusBar.vue` içinde birebir
 * kopya duruyordu. Faz C altıncı kovayı (`contact`) eklerken ikisini de elle güncellemek
 * gerekti — yani kopya zaten sapmaya bir adım kalmıştı. Bu projede kopyalanan geometri üç
 * kez saptı (brain.md); tablo da geometridir.
 *
 * `Record<Bucket, …>` olduğu için yeni bir kova eklendiğinde **derleme durur**: eksik etiket
 * sessizce boş görünmez.
 */
import type { Bucket } from "./types";

/** Mevcut durum rozeti token'ları — yeni renk icat edilmiyor. */
export const BUCKET_TONE: Record<Bucket, string> = {
  urgent: "eksik",
  leverage: "uygun",
  leak: "bekliyor",
  review: "tamamlandi",
  upkeep: "hatali",
  // Müşteri işi bir SEO sorunu değil; nötr ton onu listede ayırt ediyor.
  contact: "notr",
  // İçerik işi de bir sayfa hatası değil, eksik: "bekliyor" tonu Kaçak trafikle aynı
  // sınıfta olduğunu söylüyor (ikisinde de çözüm kısmen uygulamanın dışında).
  content: "bekliyor",
};

export const BUCKET_LABEL: Record<Bucket, string> = {
  urgent: "Acil",
  leverage: "Yüksek kaldıraç",
  leak: "Kaçak trafik",
  review: "Sonuç kontrolü",
  upkeep: "Bakım",
  contact: "Müşteri",
  content: "İçerik",
};
