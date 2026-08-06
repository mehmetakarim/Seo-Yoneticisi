//! # Feed değişikliği tespiti — "tedarikçi ürünü değiştirdi, SEO'n bayatladı"
//!
//! Sorun: bir ürünün adı veya açıklaması tedarikçide değişirse uygulama bunu fark etmiyordu.
//! Ürün "Tamamlandı" olarak kalıyor, üretilmiş içerik artık ürünü anlatmıyor ve bu **sessizce**
//! oluyor — kimse bakmadığı sürece fark edilmiyor.
//!
//! Çözüm: senkronda ürünün **üretimi besleyen** alanlarından bir parmak izi alınır. Kullanıcı
//! ürünü "tamamlandı" işaretlediğinde o anki parmak izi saklanır. İkisi ayrışırsa ürün
//! *"feed verisi değişti, gözden geçir"* diye işaretlenir.
//!
//! ## Hangi alanlar? — üretimin GİRDİLERİ, çıktıları değil
//!
//! Parmak izine yalnızca üretime giren alanlar konuyor (`ctx_parts` + `generate_details`):
//! ad, marka, kategori, ana kategori, açıklama HTML'i ve galeri görselleri.
//!
//! ⚠️ **`quantity` (stok) bilinçli olarak DIŞARIDA.** Stok üretimi beslemiyor; içeri alınsaydı
//! her stok hareketinde tüm katalog bayraklanır ve bayrak anlamsızlaşırdı.
//!
//! ⚠️ **`title`/`keywords`/`descriptions` de DIŞARIDA.** Bunlar mağazadaki MEVCUT SEO alanları,
//! yani üretimin girdisi değil rakibi. Değişmeleri "kaynak veri değişti" anlamına gelmiyor.
//!
//! ## Neden normalizasyon zorunlu — ölçüldü (2026-07-31, 254 ürünlük gerçek feed)
//!
//! | | bayraklanan ürün |
//! |---|---|
//! | ham karşılaştırma | **8** |
//! | boşluk normalize edilmiş | **1** |
//!
//! Aradaki 7 ürün sahte pozitifti: feed'in `details` alanında satır sonları `\r\n`, veritabanında
//! `\n` olarak duruyordu. İçerik aynıydı, yalnızca biçim farklıydı. Normalize edilmeseydi
//! **özellik ilk senkronda 7 yanlış bayrakla açılacaktı** ve kullanıcı ona güvenmeyi bırakırdı.
//!
//! Kalan 1 ürün gerçek: `SW.ARB.JL686B`'nin açıklama HTML'i baştan yazılmış
//! (`<section class="container">` → `<div class="container">`). Tam da yakalanması gereken vaka.

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Parmak izine giren alanlar — sıra ÖNEMLİ (hash sıraya duyarlı).
///
/// Kullanıcıya gösterilen adlar da buradan geliyor; yeni alan eklenirse ikisi birlikte değişir.
pub const FIELDS: &[&str] = &[
    "ad",
    "marka",
    "ana kategori",
    "kategori",
    "açıklama",
    "görseller",
];

/// Ürünün üretimi besleyen alanları. `FIELDS` ile aynı sırada.
///
/// `Serialize`/`Deserialize`: kullanıcı bir ürünü "tamamlandı" işaretlediğinde bu yapının o
/// anki hâli `seo_status.reviewed_facts_json` alanına yazılıyor. **Parmak izi "değişti mi?"
/// sorusunu cevaplıyor, bu kayıt ise "NE değişti?" sorusunu** — iz geri döndürülemez bir
/// özet olduğu için tek başına karşılaştırma yapmaya yetmiyor.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FeedFacts {
    pub name: String,
    pub brand: String,
    pub main_category: String,
    pub category: String,
    /// Açıklama HTML'i (feed'deki `details`).
    pub details: String,
    /// Galeri: ana görsel + picture2..4, sırayla.
    pub images: Vec<String>,
}

impl FeedFacts {
    /// `FIELDS` adına karşılık gelen metin değeri. Görseller metin değil (liste hâlinde
    /// küçük resim olarak gösteriliyor), o yüzden `None` dönüyor.
    pub fn text_of(&self, field: &str) -> Option<&str> {
        match field {
            "ad" => Some(&self.name),
            "marka" => Some(&self.brand),
            "ana kategori" => Some(&self.main_category),
            "kategori" => Some(&self.category),
            "açıklama" => Some(&self.details),
            _ => None,
        }
    }

    fn parts(&self) -> [String; 6] {
        [
            norm(&self.name),
            norm(&self.brand),
            norm(&self.main_category),
            norm(&self.category),
            norm(&self.details),
            self.images.iter().map(|s| norm(s)).collect::<Vec<_>>().join("|"),
        ]
    }
}

/// Boşluk normalizasyonu: her boşluk dizisi (satır sonu dahil) tek boşluğa iner, uçlar kırpılır.
///
/// ⚠️ Bu fonksiyon özelliğin işe yarar olmasının tek sebebi — ölçüm için modül başlığına bakın.
/// HTML'de girinti/satır sonu değişikliği SEO açısından hiçbir şey ifade etmiyor; içeriğin
/// kendisi değişmediyse kullanıcı rahatsız edilmemeli.
fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Alanların parmak izi. Aynı içerik → aynı iz; biçim farkı izi DEĞİŞTİRMEZ.
///
/// Kriptografik değil: amaç çakışmaya karşı savunma değil, "değişti mi?" sorusuna ucuz cevap.
pub fn fingerprint(f: &FeedFacts) -> String {
    let mut h = DefaultHasher::new();
    for p in f.parts() {
        p.hash(&mut h);
        // Alan sınırı: ("ab","c") ile ("a","bc") aynı ize düşmesin.
        0xFFu8.hash(&mut h);
    }
    format!("{:016x}", h.finish())
}

/// Hangi alanlar değişti — kullanıcıya "neye bakmalıyım" diyebilmek için.
///
/// Yalnızca bayrak göstermek "bir şey değişti" der; hangi alan olduğunu söylemek operatörü
/// doğrudan doğru yere götürüyor.
pub fn changed_fields(old: &FeedFacts, new: &FeedFacts) -> Vec<&'static str> {
    let (a, b) = (old.parts(), new.parts());
    FIELDS
        .iter()
        .enumerate()
        .filter(|(i, _)| a[*i] != b[*i])
        .map(|(_, name)| *name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> FeedFacts {
        FeedFacts {
            name: "Lenovo ThinkPad E16".into(),
            brand: "Lenovo".into(),
            main_category: "Bilgisayar".into(),
            category: "Notebook".into(),
            details: "<section>Güçlü performans</section>".into(),
            images: vec!["https://cdn/a.jpg".into(), "https://cdn/b.jpg".into()],
        }
    }

    /// 🔴 Bu testin koruduğu şey ÖLÇÜLDÜ: normalizasyon olmasaydı özellik ilk senkronda
    /// 7 yanlış bayrakla açılacaktı (feed'de `\r\n`, veritabanında `\n`).
    #[test]
    fn bosluk_ve_satir_sonu_farki_iz_degistirmez() {
        let a = facts();
        let mut b = facts();
        b.details = "<section>Güçlü\r\n\tperformans</section>".into();
        assert_eq!(fingerprint(&a), fingerprint(&b), "satır sonu farkı iz değiştirdi");
        assert!(changed_fields(&a, &b).is_empty());

        let mut c = facts();
        c.name = "  Lenovo   ThinkPad E16  ".into();
        assert_eq!(fingerprint(&a), fingerprint(&c), "fazla boşluk iz değiştirdi");
    }

    #[test]
    fn gercek_icerik_degisikligi_yakalanir() {
        let a = facts();
        let mut b = facts();
        // Ölçümdeki gerçek vaka: açıklama HTML'i baştan yazılmış.
        b.details = "<div class=\"container\">Yeni açıklama</div>".into();
        assert_ne!(fingerprint(&a), fingerprint(&b));
        assert_eq!(changed_fields(&a, &b), vec!["açıklama"]);
    }

    #[test]
    fn birden_cok_alan_degisirse_hepsi_bildirilir() {
        let a = facts();
        let mut b = facts();
        b.name = "Lenovo ThinkPad E16 Gen 2".into();
        b.category = "İş İstasyonu".into();
        b.images = vec!["https://cdn/yeni.jpg".into()];
        assert_eq!(changed_fields(&a, &b), vec!["ad", "kategori", "görseller"]);
    }

    /// ⚠️ Alan sınırı olmasaydı ("ab","c") ile ("a","bc") aynı ize düşerdi.
    #[test]
    fn alanlar_birbirine_karismaz() {
        let mut a = facts();
        let mut b = facts();
        a.name = "ab".into();
        a.brand = "c".into();
        b.name = "a".into();
        b.brand = "bc".into();
        assert_ne!(fingerprint(&a), fingerprint(&b));
    }

    #[test]
    fn gorsel_sirasi_onemli() {
        let a = facts();
        let mut b = facts();
        b.images.reverse();
        // Galeri sırası üretilen HTML'deki görsel yerleşimini belirliyor → sıra değişimi gerçek.
        assert_ne!(fingerprint(&a), fingerprint(&b));
    }

    #[test]
    fn iz_kararli_ve_ayni_girdide_ayni() {
        assert_eq!(fingerprint(&facts()), fingerprint(&facts()));
        assert_eq!(fingerprint(&facts()).len(), 16);
    }
}
