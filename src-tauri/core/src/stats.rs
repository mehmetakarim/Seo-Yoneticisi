//! # Küçük istatistik yardımcıları — "ölçtük mü, yoksa tahmin mi ediyoruz?"
//!
//! Uygulamada iki yerde aynı soru soruluyor: elimizdeki örnekler bir sayı söylemeye yetiyor
//! mu, yetiyorsa o sayı ne? Faz S bunu **kova süresi** için sordu (`queue::calibrated_minutes`),
//! Faz C **temas aralığı** için soruyor (`contacts::sessizlik_onerisi`).
//!
//! ⚠️ Bu modül tam olarak bu yüzden var. İkinci çağıran geldiğinde ilkinin gövdesini
//! kopyalamak kolaydı; bu oturumda üç kez ölçtüğümüz tuzak da o: **kopyalanan mantık zamanla
//! sapar.** Medyan kuralı bir yerde düzelirse iki yerde birden düzelmeli.
//!
//! ## Neden medyan, ortalama değil
//!
//! Her iki ölçüm de **insan davranışı** ölçüyor ve insan davranışında uç değer kuraldır:
//! bir işin ortasında çay tazelenir (40 dakika), bir müşteriye bir yıl sonra dönülür (365 gün).
//! Ortalama bu tek değerin peşinden gider, medyan dayanır. Faz S'in testi bunu sabitliyor:
//! `[3,4,4,5,40]` → ortalama 11 (yanlış), medyan 4 (doğru).

/// Yeterli örnek varsa medyan, yoksa `None`.
///
/// Geçersiz örnekler (NaN, sonsuz, negatif) **süzülüyor ve süzüldükten SONRA** eşik yeniden
/// kontrol ediliyor: 5 örneğin 3'ü bozuksa elimizde 2 örnek var demektir, 5 değil.
///
/// `None` "veri yok" demek — çağıran elle yazılmış varsayılanı kullanır ve ekranda ikisini
/// ayırt eder ("≈2 dk" tahmin ↔ "4 dk" ölçüm). Sessizce bir sayı uydurulmaz.
pub fn median_sample(samples: &[f64], min: usize) -> Option<f64> {
    let mut v: Vec<f64> = samples.iter().copied().filter(|x| x.is_finite() && *x >= 0.0).collect();
    if v.len() < min || v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let orta = v.len() / 2;
    Some(if v.len() % 2 == 0 { (v[orta - 1] + v[orta]) / 2.0 } else { v[orta] })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 🔬 Faz S'in ölçümü, artık paylaşılan yerde: tek uç değer medyanı bozmamalı.
    #[test]
    fn medyan_uc_degerden_etkilenmiyor() {
        assert_eq!(median_sample(&[3.0, 4.0, 4.0, 5.0, 40.0], 5), Some(4.0));
        assert_eq!(median_sample(&[1.0, 2.0, 3.0, 4.0], 4), Some(2.5), "çift sayıda örnek");
    }

    #[test]
    fn yetersiz_ornekte_sayi_uydurmuyor() {
        assert_eq!(median_sample(&[3.0, 4.0, 5.0, 6.0], 5), None);
        assert_eq!(median_sample(&[], 1), None);
        assert_eq!(median_sample(&[7.0], 1), Some(7.0));
    }

    /// ⚠️ Eşik SÜZMEDEN SONRA kontrol ediliyor: 5 örneğin 3'ü bozuksa elde 2 örnek var.
    #[test]
    fn bozuk_ornekler_suzulup_esik_yeniden_bakiliyor() {
        let bozuk = [f64::NAN, -1.0, f64::INFINITY, 4.0, 6.0];
        assert_eq!(median_sample(&bozuk, 5), None, "süzülünce 2 örnek kaldı, eşik 5");
        assert_eq!(median_sample(&bozuk, 2), Some(5.0));
    }
}
