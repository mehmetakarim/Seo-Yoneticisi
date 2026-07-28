//! # Sürüm geçmişi — "yeniden ürettim, beğenmedim, eskisini geri istiyorum"
//!
//! Meta, açıklama ve teknik tablo; üçü de yeniden üretilebiliyor ve üçünde de eski hâli
//! kaybetme riski var. Saklama mantığı üçünde aynı olduğu için burada tek yerde duruyor.
//!
//! **Neden genel:** başlangıçta yalnızca teknik tabloda vardı ve `commands.rs` içine gömülüydü.
//! Meta ile açıklamaya kopyalamak üçe katlamak olurdu — bu projede tam bu hatanın bedeli iki kez
//! ödendi (kart iskeleti 4 bileşende sapmıştı; Gemini hata sınıflandırması 4 yerde kopyalanıp
//! dördü de yanlıştı). Kopyalamadan önce genelleştir.
//!
//! Saklanan sürümün ne içerdiği türe göre değişir (`MetaVersion`, `DetailsVersion`,
//! `TechVersion` — hepsi `commands.rs`'te), ama listenin nasıl yönetildiği değişmez.

use serde::de::DeserializeOwned;

/// Saklanan en fazla sürüm sayısı.
///
/// Sınır bilinçli: açıklama HTML'i ürün başına ortalama 3,7 KB (2026-07-28 ölçümü). Sınırsız
/// olsa 262 ürünlük katalogda veritabanı ve yedek dosyası kontrolsüz büyürdü. 5 sürüm, birkaç
/// yeniden üretim denemesini geri almaya yetiyor.
pub const MAX: usize = 5;

/// Sütunda saklanan JSON'u sürüm listesine çevirir.
///
/// Bozuk/boş JSON'da **panik atmaz, boş liste döner**: geçmiş yardımcı bir özellik, okunamaması
/// üretimi engellememeli. (Eski bir sürümden gelen şema uyumsuzluğu da buraya düşer.)
pub fn parse<T: DeserializeOwned>(json: Option<&str>) -> Vec<T> {
    json.map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|j| serde_json::from_str::<Vec<T>>(j).ok())
        .unwrap_or_default()
}

/// Mevcut hâli geçmişe iter: en yeni başa, [`MAX`]'ı aşan en eski kayıt düşer.
pub fn push<T>(mut hist: Vec<T>, current: T) -> Vec<T> {
    hist.insert(0, current);
    hist.truncate(MAX);
    hist
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newest_goes_first_and_oldest_falls_off() {
        let mut h: Vec<i32> = Vec::new();
        for i in 1..=7 {
            h = push(h, i);
        }
        // 7 kayıt itildi, MAX=5 → en yeni başta, en eski ikisi düşmüş
        assert_eq!(h, vec![7, 6, 5, 4, 3]);
        assert_eq!(h.len(), MAX);
    }

    #[test]
    fn parse_survives_bad_input() {
        // Geçmiş okunamıyorsa üretim durmamalı → her durumda boş liste
        assert!(parse::<i32>(None).is_empty());
        assert!(parse::<i32>(Some("")).is_empty());
        assert!(parse::<i32>(Some("   ")).is_empty());
        assert!(parse::<i32>(Some("{bozuk json")).is_empty());
        assert!(parse::<i32>(Some(r#"{"beklenmeyen":"şema"}"#)).is_empty());
    }

    #[test]
    fn parse_reads_valid_list() {
        assert_eq!(parse::<i32>(Some("[3,2,1]")), vec![3, 2, 1]);
    }
}
