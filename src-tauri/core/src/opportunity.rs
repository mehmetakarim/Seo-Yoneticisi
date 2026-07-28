//! # Fırsat analizi — hangi ürüne emek harcamak en çok getirir?
//!
//! Operatör 262 ürün arasında elle seçim yapıyordu. Google Search Console zaten "hangi sayfa
//! gösterim alıyor ama tıklanmıyor", "hangisi 2. sayfada takılmış" verisini tutuyor; bu modül
//! o veriyi sıralanabilir bir işe çeviriyor.
//!
//! **Tasarım kararı: soyut "puan" yerine somut sayı.** Çıktı `kaçırılan tıklama` — yani
//! "bu sayfa, konumunun normalde getirmesi gereken tıklamanın kaçını alamıyor". Operatör
//! 87 puanlık bir ürünle 62 puanlığı kıyaslayamaz ama "ayda 140 tıklama kaçırıyor" ile
//! "12 tıklama kaçırıyor"u kıyaslayabilir. Sayının nereden geldiği de açıklanabilir olmalı;
//! bu yüzden her satır ayrıca bir **sebep etiketi** taşıyor.
//!
//! Bu modül saf mantıktır (ağ/DB yok) → hızlı test edilir, `cargo test -p seo-core`.

use serde::{Deserialize, Serialize};

/// "Gösterim var ama tıklama yok" demek için gereken en az gösterim.
/// Altındaki sayılar istatistiksel gürültü: 3 gösterim / 0 tıklama bir şey ifade etmez.
const MIN_IMPRESSIONS_FOR_NO_CLICK: f64 = 50.0;

/// Google'ın ilk sayfası ~10 sonuç. 11+ = ikinci sayfa.
const FIRST_PAGE_LAST_POSITION: f64 = 10.0;
/// 20'den sonrası "ikinci sayfa fırsatı" sayılmaz — oraya çıkmak küçük bir iyileştirme değil.
const SECOND_PAGE_LAST_POSITION: f64 = 20.0;

/// Düşük CTR eşiği: beklenenin bu oranının altındaysa meta çekmiyordur.
const LOW_CTR_RATIO: f64 = 0.5;

/// Listeye girmek için gereken en az kaçırılan tıklama — altı gürültü.
const MIN_MISSED_CLICKS: f64 = 1.0;

/// Bir sayfanın konumuna göre **beklenen** tıklama oranı.
///
/// Sektör ortalaması bir eğri; kesin değil, sıralama için yeterli. Mutlak doğruluk gerekmiyor:
/// amaç ürünleri birbirine göre sıralamak, tıklama sayısı tahmin etmek değil. Konum ondalıklı
/// gelir (GSC ortalama verir), bu yüzden aralık bazlı.
pub fn expected_ctr(position: f64) -> f64 {
    match position {
        p if p < 1.5 => 0.280,
        p if p < 2.5 => 0.150,
        p if p < 3.5 => 0.110,
        p if p < 4.5 => 0.080,
        p if p < 5.5 => 0.070,
        p if p < 6.5 => 0.050,
        p if p < 7.5 => 0.040,
        p if p < 8.5 => 0.033,
        p if p < 9.5 => 0.028,
        p if p <= FIRST_PAGE_LAST_POSITION => 0.025,
        p if p <= SECOND_PAGE_LAST_POSITION => 0.010,
        _ => 0.005,
    }
}

/// Neden bu ürün listede? Sıralamayı belirlemez — operatöre *ne yapması gerektiğini* söyler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// Pozisyon 11–20: ilk sayfaya çıkmak için küçük bir itme yeterli.
    SecondPage,
    /// Gösterim alıyor ama hiç tıklanmıyor — başlık/açıklama çekmiyor.
    NoClicks,
    /// İlk sayfada ama CTR beklenenin çok altında — meta çalışması.
    LowCtr,
}

impl Reason {
    /// Arayüzde gösterilecek Türkçe etiket.
    pub fn label(self) -> &'static str {
        match self {
            Reason::SecondPage => "İkinci sayfa",
            Reason::NoClicks => "Gösterim var, tıklama yok",
            Reason::LowCtr => "Düşük CTR",
        }
    }
}

/// Fırsat listesindeki tek satır.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Opportunity {
    pub sku: String,
    pub name: String,
    pub url: String,
    pub clicks: f64,
    pub impressions: f64,
    pub ctr: f64,
    pub position: f64,
    /// Konumunun getirmesi gereken tıklamanın kaçını alamıyor (sıralama buna göre).
    pub missed_clicks: f64,
    pub reason: Reason,
}

/// Bir sayfanın ölçümlerinden fırsat çıkar. Fırsat yoksa `None`.
///
/// Sıra önemli: önce "hiç tıklanmıyor" (en net sinyal), sonra "ikinci sayfa" (konum sorunu),
/// sonra "düşük CTR" (meta sorunu). Bir sayfa birden çok koşula uyabilir; en açıklayıcı olan seçilir.
pub fn classify(clicks: f64, impressions: f64, ctr: f64, position: f64) -> Option<(Reason, f64)> {
    if impressions <= 0.0 {
        return None;
    }
    let expected = expected_ctr(position);
    // Beklenenden İYİ performans gösteren sayfa fırsat değildir → negatife düşmesin.
    let missed = (impressions * (expected - ctr)).max(0.0);

    let reason = if clicks == 0.0 && impressions >= MIN_IMPRESSIONS_FOR_NO_CLICK {
        Reason::NoClicks
    } else if position > FIRST_PAGE_LAST_POSITION && position <= SECOND_PAGE_LAST_POSITION {
        Reason::SecondPage
    } else if position <= FIRST_PAGE_LAST_POSITION && ctr < expected * LOW_CTR_RATIO {
        Reason::LowCtr
    } else {
        return None;
    };

    if missed < MIN_MISSED_CLICKS {
        return None;
    }
    Some((reason, missed))
}

/// Fırsatları kaçırılan tıklamaya göre azalan sırala (en yüksek getirili iş başta).
pub fn sort_by_impact(list: &mut [Opportunity]) {
    list.sort_by(|a, b| {
        b.missed_clicks
            .partial_cmp(&a.missed_clicks)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_ctr_decreases_with_position() {
        let mut prev = f64::MAX;
        for p in [1.0, 2.0, 3.0, 5.0, 8.0, 10.0, 15.0, 30.0] {
            let c = expected_ctr(p);
            assert!(c < prev, "pozisyon {p} için CTR artmış olamaz ({c} >= {prev})");
            prev = c;
        }
    }

    #[test]
    fn second_page_boundary_is_exact() {
        // 10 = ilk sayfa (SecondPage DEĞİL), 11 = ikinci sayfa, 21 = artık fırsat değil
        assert_eq!(classify(5.0, 1000.0, 0.005, 10.0).unwrap().0, Reason::LowCtr);
        assert_eq!(classify(5.0, 1000.0, 0.005, 11.0).unwrap().0, Reason::SecondPage);
        assert_eq!(classify(5.0, 1000.0, 0.005, 20.0).unwrap().0, Reason::SecondPage);
        assert!(classify(5.0, 1000.0, 0.004, 21.0).is_none(), "21. sıra fırsat sayılmamalı");
    }

    #[test]
    fn no_clicks_needs_enough_impressions() {
        // 3 gösterim / 0 tıklama istatistiksel gürültü — fırsat değil
        assert!(classify(0.0, 3.0, 0.0, 8.0).is_none());
        // eşiğin üstü + anlamlı kayıp → fırsat
        let (reason, missed) = classify(0.0, 500.0, 0.0, 8.0).unwrap();
        assert_eq!(reason, Reason::NoClicks);
        assert!(missed > 15.0, "500 gösterim, 8. sıra, 0 tıklama → kayıp büyük olmalı: {missed}");
    }

    #[test]
    fn better_than_expected_is_not_an_opportunity() {
        // 1. sırada beklenen %28; %40 alıyorsa kaçırılan yok → negatife düşmemeli
        assert!(classify(400.0, 1000.0, 0.40, 1.0).is_none());
        // doğrudan hesap da negatif vermemeli
        let expected = expected_ctr(1.0);
        assert!((1000.0 * (expected - 0.40_f64)).max(0.0) == 0.0);
    }

    #[test]
    fn zero_impressions_is_not_an_opportunity() {
        assert!(classify(0.0, 0.0, 0.0, 0.0).is_none());
    }

    #[test]
    fn tiny_losses_are_filtered_out() {
        // 20 gösterim, 9. sıra, beklenene yakın CTR → kayıp 1 tıklamanın altında
        assert!(classify(0.0, 20.0, 0.02, 9.0).is_none());
    }

    #[test]
    fn sorting_puts_biggest_loss_first() {
        let mk = |sku: &str, missed: f64| Opportunity {
            sku: sku.into(), name: "x".into(), url: "u".into(),
            clicks: 0.0, impressions: 0.0, ctr: 0.0, position: 0.0,
            missed_clicks: missed, reason: Reason::NoClicks,
        };
        let mut v = vec![mk("a", 5.0), mk("b", 120.0), mk("c", 40.0)];
        sort_by_impact(&mut v);
        assert_eq!(
            v.iter().map(|o| o.sku.as_str()).collect::<Vec<_>>(),
            ["b", "c", "a"]
        );
    }
}
