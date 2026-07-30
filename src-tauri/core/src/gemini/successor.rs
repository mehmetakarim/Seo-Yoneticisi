//! Satıştan kalkmış (EOL) bir sayfa için halef ürün önerisi.
//!
//! Kod ADAY üretir (IDF ağırlıklı örtüşme), model KARAR verir. Modelin "halef yok"
//! diyebilmesi ZORUNLU — olmayan halefi varmış gibi göstermek gerçek SEO hasarı.

use super::*;

/// Satışta olmayan bir sayfanın **halefi** hangi ürün? (yönlendirme önerisi için)
///
/// Deterministik eşleştirme yeterli değildi — ölçümde `asus-zenbook-17-fold` için en iyi aday
/// "Microsoft Windows 11 Pro" çıkmıştı (yalnızca "windows" sözcüğü örtüştüğü için). Bu yüzden
/// aday listesini kod üretiyor, **kararı model veriyor**.
///
/// ⚠️ **Modelin "halef yok" diyebilmesi zorunlu.** Her EOL sayfanın güncel bir karşılığı yok;
/// olmayan bir halefi varmış gibi göstermek, yanlış 301 yönlendirmesine ve gerçek SEO hasarına
/// yol açar. Bu yüzden şema `successor_sku`'yu opsiyonel bırakıyor ve prompt açıkça
/// "emin değilsen boş bırak" diyor.
///
/// Dönen `sku` mutlaka adaylar arasından olmalı — kod tarafında doğrulanır (uydurma engellenir).
pub async fn suggest_successor(
    api_key: &str,
    eol_slug: &str,
    candidates: &[(String, String)],
) -> Result<Produced<Option<(String, String)>>, String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("Gemini API anahtarı ayarlı değil. Ayarlar'dan ekleyin.".to_string());
    }
    if candidates.is_empty() {
        return Ok(Produced { value: None, model: "" });
    }

    let list = candidates
        .iter()
        .enumerate()
        .map(|(i, (sku, name))| format!("{}. [{}] {}", i + 1, sku, name))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "Bir e-ticaret sitesinde satıştan kalkmış bir ürün sayfası var. Adres parçası:\n\
         \"{eol_slug}\"\n\n\
         Satıştaki ürünler arasından bu ürünün YERİNİ ALAN güncel nesli seç:\n{list}\n\n\
         KURALLAR:\n\
         - Yalnızca AYNI ürün hattının daha yeni nesli ya da doğrudan muadili sayılır.\n\
         - Farklı kategori (ör. dizüstü yerine monitör), farklı marka veya yalnızca ortak \
         kelime taşıyan ürün HALEF DEĞİLDİR.\n\
         - Emin değilsen ya da uygun bir halef yoksa successor_sku alanını BOŞ bırak. \
         Yanlış eşleştirme, eşleştirmemekten daha zararlıdır.\n\
         - reason alanına tek cümlelik gerekçe yaz (Türkçe)."
    );

    let schema = serde_json::json!({
        "type": "OBJECT",
        "properties": {
            "successor_sku": { "type": "STRING", "description": "adaylardan birinin SKU'su, yoksa boş" },
            "reason": { "type": "STRING" }
        },
        "required": ["successor_sku", "reason"]
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;

    let mut last_err = String::from("Bilinmeyen hata");
    for model in MODEL_CHAIN.iter() {
        let url = format!("{API_BASE}/{model}:generateContent");
        let body = serde_json::json!({
            "contents": [{ "parts": [{ "text": prompt }] }],
            "generationConfig": {
                "responseMimeType": "application/json",
                "responseSchema": schema,
                "temperature": 0.2
            }
        });
        let resp = match client.post(&url).query(&[("key", key)]).json(&body).send().await {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("İstek gönderilemedi: {e}");
                continue;
            }
        };
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            let (try_next, msg) = classify_error(status.as_u16(), &text, model);
            last_err = msg;
            if !try_next {
                return Err(last_err);
            }
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                last_err = format!("Yanıt çözümlenemedi: {e}");
                continue;
            }
        };
        let inner = v["candidates"][0]["content"]["parts"][0]["text"].as_str().unwrap_or("");
        let parsed: serde_json::Value = match serde_json::from_str(inner) {
            Ok(p) => p,
            Err(e) => {
                last_err = format!("Üretilen JSON okunamadı: {e}");
                continue;
            }
        };
        let sku = parsed["successor_sku"].as_str().unwrap_or("").trim().to_string();
        let reason = parsed["reason"].as_str().unwrap_or("").trim().to_string();

        // **Uydurma engeli:** dönen SKU adaylar arasında olmalı. Model listede olmayan bir
        // şey uydurursa "halef yok" sayılır — sessizce yanlış yönlendirme önerilmesin.
        let valid = candidates.iter().any(|(s, _)| s == &sku);
        let value = if sku.is_empty() || !valid {
            None
        } else {
            Some((sku, reason))
        };
        return Ok(Produced { value, model });
    }
    Err(format!("Halef önerisi alınamadı. Son hata: {last_err}"))
}

#[cfg(test)]
mod tests {
    //! EOL halef önerisi testleri.
    use super::*;

    /// Halef önerisi canlı test. En kritik davranış: model UYGUN HALEF YOKSA boş dönebilmeli.
    /// `GEMINI_API_KEY=... cargo test successor_real -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn successor_real() {
        let key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY ayarlı değil");

        // 1) Gerçek halefi OLAN vaka — ölçümde deterministik de bulmuştu
        let cands = vec![
            ("A".to_string(), "Creality Filament Maker M1 & Shredder R1 Filament Üretim ve Geri Dönüşüm Seti".to_string()),
            ("B".to_string(), "Creality Falcon T1 5'i 1 arada Lazer Kazıma Makinesi".to_string()),
            ("C".to_string(), "Bambu Lab A1 Combo PF002-A+SA005 3D Yazıcı".to_string()),
        ];
        let r = suggest_successor(&key, "creality-filament-maker-m1-uretim-makinesi", &cands)
            .await
            .expect("çağrı başarısız");
        println!("model: {}", r.model);
        println!("VAKA-1 (halef var) → {:?}", r.value);
        assert_eq!(r.value.as_ref().map(|(s, _)| s.as_str()), Some("A"),
                   "aynı ürün hattının güncel nesli seçilmeliydi");

        // 2) Halefi OLMAYAN vaka — deterministik burada "Windows 11 Pro" demişti.
        //    Model bunu halef SAYMAMALI.
        let cands2 = vec![
            ("X".to_string(), "Microsoft Windows 11 Pro FQC-10556 64 Bit OEM Türkçe Lisans".to_string()),
            ("Y".to_string(), "Dell Pro 24 E2425HSM IPS Full HD Monitör".to_string()),
            ("Z".to_string(), "Logitech MK345 Kablosuz Klavye Mouse Seti".to_string()),
        ];
        let r2 = suggest_successor(&key, "asus-zenbook-17-fold-i7-1250u-16gb-1tb-17-windows-11-home", &cands2)
            .await
            .expect("çağrı başarısız");
        println!("VAKA-2 (halef yok) → {:?}", r2.value);
        assert!(r2.value.is_none(),
                "alakasız ürünler halef sayılmamalıydı — yanlış 301 önerisi gerçek SEO hasarıdır");
    }
}
