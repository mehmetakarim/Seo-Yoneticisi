//! Sayfa tipi sınıflandırması (Faz İ).
//!
//! **Neden gerekli.** Uygulamanın altı aracı da ürün-merkezliydi: `striking_distance` ve
//! `cannibalization` katalogda olmayan sayfayı düşürüyor, `find_eol` ürün yoluna bakıyor.
//! Ölçüldü (2026-08-12): GSC'de ürün dışı **5.700 sorgu / 101.118 gösterim** var ve bunlar
//! hiçbir ekranda görünmüyordu. İçerik açığı tam olarak orada duruyor.
//!
//! 🔴 **Yol desenleri KODA GÖMÜLMEZ.** Uygulama kişiselleştirilmemiş — `/kategori/` her
//! IdeaSoft temasında aynı olmak zorunda değil ve bu, 0ao'da yaşanan hatanın (gömülü
//! varsayılan feed adresi) aynı sınıfı olurdu. Bu yüzden:
//!   - **ürün** segmenti katalog URL'lerinden ölçülerek gelir (`opportunity::common_path_prefix`),
//!   - kalan segmentler veriden çıkarılıp kullanıcıya sayılarıyla sunulur, kullanıcı bir kez
//!     etiketler,
//!   - etiketlenmemiş segment `Other` kalır ve **kuyruğa iş düşürmez** — bilmediğimiz bir
//!     sayfa tipi hakkında "niyet uyuşmuyor" demek uydurma olurdu.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Bir sayfanın ne olduğu. `Other` = "bilmiyoruz", boş küme değil.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageKind {
    Product,
    Category,
    Brand,
    Blog,
    Home,
    Other,
}

impl PageKind {
    pub fn label(&self) -> &'static str {
        match self {
            PageKind::Product => "Ürün",
            PageKind::Category => "Kategori",
            PageKind::Brand => "Marka",
            PageKind::Blog => "İçerik",
            PageKind::Home => "Anasayfa",
            PageKind::Other => "Bilinmiyor",
        }
    }

    /// Bu sayfa tipi **bilgi amaçlı** bir sorguyu karşılayabilir mi?
    ///
    /// Ölçüm bunu doğruluyor: bilgi sorgusuna kategori/ürün listesi çıkınca TO çöküyor
    /// ("access point" poz 6,07'de %0,26). Blog karşılar, vitrin sayfaları karşılamaz.
    ///
    /// ⚠️ `Other` **false değil `None`**: bilmediğimiz bir tipin niyeti karşılayıp
    /// karşılamadığını da bilmiyoruz. `false` demek, kullanıcıyı etiketlemediği bir segment
    /// için işe göndermek olurdu.
    pub fn serves_informational(&self) -> Option<bool> {
        match self {
            PageKind::Blog => Some(true),
            PageKind::Product | PageKind::Category | PageKind::Brand | PageKind::Home => {
                Some(false)
            }
            PageKind::Other => None,
        }
    }
}

/// URL'nin ilk yol parçası. Anasayfada `None`.
///
/// Sorgu dizesi ve sondaki `/` atılıyor: `?tp=1` gibi filtre parametreleri ayrı segment
/// sanılırsa kategori sayfaları iki tipe bölünürdü (denetimde `/kategori/is-istasyonu?tp=1`
/// dizinde görüldü).
pub fn first_segment(url: &str) -> Option<String> {
    let after = url.split("://").nth(1).unwrap_or(url);
    let path = after.split_once('/').map(|x| x.1).unwrap_or("");
    let path = path.split(['?', '#']).next().unwrap_or("");
    path.split('/')
        .find(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
}

/// Segment → tip eşlemesine göre sınıflandırır.
///
/// `product_segment`: katalogdan ölçülen ürün segmenti (ör. `urun`). Eşlemede ayrıca
/// bulunmasına gerek yok; ürün her zaman ölçümden gelir.
pub fn classify(
    url: &str,
    product_segment: Option<&str>,
    labels: &HashMap<String, PageKind>,
) -> PageKind {
    match first_segment(url) {
        None => PageKind::Home,
        Some(seg) => {
            if product_segment.map(|p| p == seg).unwrap_or(false) {
                return PageKind::Product;
            }
            labels.get(&seg).copied().unwrap_or(PageKind::Other)
        }
    }
}

/// Verideki segmentleri sayfa sayısıyla listeler — kullanıcı etiketleyebilsin diye.
///
/// Ürün segmenti ve zaten etiketlenmiş olanlar dışarıda bırakılıyor: ekranda kullanıcıya
/// **yalnızca karar bekleyen** segmentler gösterilmeli, yoksa liste her açılışta aynı işi
/// tekrar yapmayı öneriyor gibi görünür.
pub fn unlabeled_segments(
    urls: &[String],
    product_segment: Option<&str>,
    labels: &HashMap<String, PageKind>,
) -> Vec<(String, usize)> {
    let mut sayim: HashMap<String, usize> = HashMap::new();
    for u in urls {
        if let Some(seg) = first_segment(u) {
            if product_segment.map(|p| p == seg).unwrap_or(false) || labels.contains_key(&seg) {
                continue;
            }
            *sayim.entry(seg).or_insert(0) += 1;
        }
    }
    let mut out: Vec<(String, usize)> = sayim.into_iter().collect();
    // Çok sayfalı segment önce: kullanıcı en çok işe yarayacak etiketi ilk verir.
    out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn etiketler() -> HashMap<String, PageKind> {
        [
            ("kategori".to_string(), PageKind::Category),
            ("marka".to_string(), PageKind::Brand),
            ("blog".to_string(), PageKind::Blog),
        ]
        .into_iter()
        .collect()
    }

    #[test]
    fn segment_cikarimi() {
        assert_eq!(first_segment("https://m.com/urun/abc"), Some("urun".into()));
        assert_eq!(first_segment("https://m.com/"), None, "anasayfa segmentsiz");
        assert_eq!(first_segment("https://m.com"), None);
        // 🔴 Filtre parametresi segment sanılmamalı; yoksa kategori sayfaları ikiye bölünür.
        assert_eq!(
            first_segment("https://m.com/kategori/is-istasyonu?tp=1"),
            Some("kategori".into())
        );
        assert_eq!(first_segment("https://m.com/Blog/Icerik/x"), Some("blog".into()));
    }

    #[test]
    fn siniflandirma_urunu_olcumden_aliyor() {
        let e = etiketler();
        assert_eq!(classify("https://m.com/urun/x", Some("urun"), &e), PageKind::Product);
        assert_eq!(classify("https://m.com/blog/icerik/x", Some("urun"), &e), PageKind::Blog);
        assert_eq!(classify("https://m.com/", Some("urun"), &e), PageKind::Home);
        // Ürün segmenti bilinmiyorsa (karışık katalog) ürün sayfası da etiketsiz kalır —
        // uydurmak yerine bilinmiyor demek doğru.
        assert_eq!(classify("https://m.com/urun/x", None, &e), PageKind::Other);
    }

    /// 🔴 Etiketlenmemiş segment hakkında hüküm verilmez. `Other` için `None` dönüyor;
    /// `Some(false)` dönseydi kullanıcının hiç etiketlemediği bir segment "niyeti
    /// karşılamıyor" diye kuyruğa iş düşürürdü.
    #[test]
    fn bilinmeyen_tip_hakkinda_hukum_verilmiyor() {
        let e = etiketler();
        let bilinmeyen = classify("https://m.com/destek/sss", Some("urun"), &e);
        assert_eq!(bilinmeyen, PageKind::Other);
        assert_eq!(bilinmeyen.serves_informational(), None);
        assert_eq!(PageKind::Blog.serves_informational(), Some(true));
        assert_eq!(PageKind::Category.serves_informational(), Some(false));
    }

    #[test]
    fn etiketsiz_segmentler_sayiyla_ve_sirali() {
        let e = etiketler();
        let urls: Vec<String> = [
            "https://m.com/urun/a",      // ürün → listede yok
            "https://m.com/blog/b",      // etiketli → listede yok
            "https://m.com/destek/x",
            "https://m.com/destek/y",
            "https://m.com/destek/z",
            "https://m.com/kurumsal/k",
            "https://m.com/",            // anasayfa → segment yok
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let out = unlabeled_segments(&urls, Some("urun"), &e);
        assert_eq!(out, vec![("destek".to_string(), 3), ("kurumsal".to_string(), 1)]);
    }
}
