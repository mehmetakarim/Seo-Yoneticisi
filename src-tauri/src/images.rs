//! Ürün galeri görsellerinin format kontrolü (Faz 7).
//!
//! Her galeri görselini indirip **`imagesize`** ile boyutunu (decode etmeden, salt başlıktan) okur;
//! **1:1 kare** (±%2 tolerans) ve **≥1000px** kurallarını değerlendirir. Sonuç `commands::check_images`
//! tarafından `seo_status`'a cache'lenir (`?revision` parmak izi değişince yeniden kontrol).

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Minimum kenar (feed galeri görsellerini 1000×1000 servis ediyor).
pub const MIN_DIM: u32 = 1000;
/// Kare oran toleransı (±%2 — hafif kırpma/dolgu farklarına izin).
const SQUARE_TOL: f64 = 0.02;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageCheck {
    pub url: String,
    pub width: u32,
    pub height: u32,
    pub is_square: bool,
    pub meets_min: bool,
    pub ok: bool,
    pub error: Option<String>,
}

/// Verilen görselleri sırayla kontrol eder (ürün başına ≤4 → sıralı yeterli).
pub async fn check_dimensions(client: &reqwest::Client, urls: &[String]) -> Vec<ImageCheck> {
    let mut out = Vec::with_capacity(urls.len());
    for url in urls {
        out.push(check_one(client, url).await);
    }
    out
}

async fn check_one(client: &reqwest::Client, url: &str) -> ImageCheck {
    match fetch_dims(client, url).await {
        Ok((w, h)) => {
            let (is_square, meets_min, ok) = evaluate(w, h);
            ImageCheck { url: url.to_string(), width: w, height: h, is_square, meets_min, ok, error: None }
        }
        Err(e) => ImageCheck {
            url: url.to_string(),
            width: 0,
            height: 0,
            is_square: false,
            meets_min: false,
            ok: false,
            error: Some(e),
        },
    }
}

/// (is_square, meets_min, ok) — kural değerlendirmesi (test edilebilir, ağ yok).
fn evaluate(w: u32, h: u32) -> (bool, bool, bool) {
    if w == 0 || h == 0 {
        return (false, false, false);
    }
    let ratio = w as f64 / h as f64;
    let is_square = (ratio - 1.0).abs() <= SQUARE_TOL;
    let meets_min = w >= MIN_DIM && h >= MIN_DIM;
    (is_square, meets_min, is_square && meets_min)
}

async fn fetch_dims(client: &reqwest::Client, url: &str) -> Result<(u32, u32), String> {
    let bytes = client
        .get(url)
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|e| format!("Görsele ulaşılamadı: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("Görsel indirilemedi: {e}"))?;
    let sz = imagesize::blob_size(&bytes).map_err(|e| format!("Boyut okunamadı: {e}"))?;
    Ok((sz.width as u32, sz.height as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluate_rules() {
        // 1000×1000 kare + min → ok
        assert_eq!(evaluate(1000, 1000), (true, true, true));
        // 1080×1080 → ok
        assert_eq!(evaluate(1080, 1080), (true, true, true));
        // kare ama küçük → meets_min false
        assert_eq!(evaluate(800, 800), (true, false, false));
        // dikdörtgen (kare değil)
        assert_eq!(evaluate(1200, 800), (false, false, false));
        // sıfır
        assert_eq!(evaluate(0, 0), (false, false, false));
    }

    #[test]
    fn evaluate_square_tolerance() {
        // %2 içinde → kare sayılır (1000 vs 1015 = %1.5)
        assert!(evaluate(1015, 1000).0);
        // %2 dışında → kare değil (1000 vs 1050 = %5)
        assert!(!evaluate(1050, 1000).0);
    }

    /// Gerçek galeri görseli kontrolü — env: IMG_URL (bir görsel URL'si).
    /// `IMG_URL=https://... cargo test check_one_real -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn check_one_real() {
        let url = std::env::var("IMG_URL").expect("IMG_URL ayarlı değil");
        let client = reqwest::Client::new();
        let c = check_one(&client, &url).await;
        println!("{}×{} · kare={} · min={} · ok={} · {:?}", c.width, c.height, c.is_square, c.meets_min, c.ok, c.error);
    }
}
