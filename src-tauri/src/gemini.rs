//! Gemini meta üretimi (Faz 2).
//!
//! - Model **kademeli fallback**: biri kota/limit (429) verirse sıradaki denenir.
//! - JSON structured output (responseSchema) → 5 alan.
//! - Üretim sonrası kural fail ederse **tek retry** (kısalt/uzat yönergesiyle).
//!
//! Faz 3: `generate_details` gerçek implementasyona kavuşacak (yapı korunur, img-src güvenliği).

use crate::validation::{grapheme_len, MetaBadge, MetaInput};
use serde::{Deserialize, Serialize};

/// Kota dolduğunda soldan sağa denenecek modeller.
pub const MODEL_CHAIN: &[&str] = &["gemini-2.0-flash", "gemini-2.5-flash", "gemini-1.5-flash"];

const API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedMeta {
    pub target_keyword: String,
    pub title: String,
    pub descriptions: String,
    pub keywords: String,
    pub search_keywords: String,
}

pub struct ProductContext<'a> {
    pub name: &'a str,
    pub brand: Option<&'a str>,
    pub category: Option<&'a str>,
    pub main_category: Option<&'a str>,
}

fn system_prompt() -> &'static str {
    "Sen bir Türk e-ticaret SEO uzmanısın. Verilen ürün için Türkçe, arama motoru \
     dostu meta alanları üretirsin. Kurallar KESİN: \
     1) Önce ürün adından kısa, doğal bir HEDEF KELIME türet (2-4 kelime, ör. 'bluetooth kulaklık'). \
     2) title: 20-60 karakter arası OLMALI ve hedef kelimeyi içermeli. \
     3) descriptions: 50-155 karakter arası OLMALI ve hedef kelimeyi içermeli, satışa yönlendiren doğal bir cümle. \
     4) keywords: virgülle ayrılmış 4-8 meta anahtar kelime. \
     5) search_keywords: kullanıcının bu ürünü sitede ararken yazabileceği ifadeler, virgülle ayrılmış 4-6 adet. \
     Karakter sayımını Türkçe harfleri tek karakter sayarak yap. Yalnızca istenen JSON'u döndür."
}

fn build_prompt(ctx: &ProductContext, correction: Option<&str>) -> String {
    let mut p = format!(
        "Ürün adı: {}\nMarka: {}\nKategori: {}\nÜst kategori: {}\n",
        ctx.name,
        ctx.brand.unwrap_or("-"),
        ctx.category.unwrap_or("-"),
        ctx.main_category.unwrap_or("-"),
    );
    if let Some(c) = correction {
        p.push_str("\nÖNEMLİ DÜZELTME: ");
        p.push_str(c);
        p.push_str(" Kuralları bu kez kesinlikle sağla.");
    }
    p
}

fn response_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "OBJECT",
        "properties": {
            "target_keyword":  { "type": "STRING" },
            "title":           { "type": "STRING" },
            "descriptions":    { "type": "STRING" },
            "keywords":        { "type": "STRING" },
            "search_keywords": { "type": "STRING" }
        },
        "required": ["target_keyword", "title", "descriptions", "keywords", "search_keywords"]
    })
}

/// Tek bir modele istek atar. Ok → üretilen metin; Err → (kota_mı, mesaj).
async fn call_model(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    prompt: &str,
) -> Result<GeneratedMeta, (bool, String)> {
    let url = format!("{API_BASE}/{model}:generateContent");
    let body = serde_json::json!({
        "system_instruction": { "parts": [{ "text": system_prompt() }] },
        "contents": [{ "parts": [{ "text": prompt }] }],
        "generationConfig": {
            "responseMimeType": "application/json",
            "responseSchema": response_schema(),
            "temperature": 0.7
        }
    });

    let resp = client
        .post(&url)
        .query(&[("key", api_key)])
        .json(&body)
        .send()
        .await
        .map_err(|e| (false, format!("İstek gönderilemedi: {e}")))?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        let code = status.as_u16();
        // 429 = kota/limit → fallback tetikle. 400 + API_KEY_INVALID = anahtar hatası.
        let is_quota = code == 429 || code == 503;
        let msg = if code == 400 && text.contains("API_KEY_INVALID") {
            "Gemini API anahtarı geçersiz.".to_string()
        } else if is_quota {
            format!("{model} kotası/limiti doldu (HTTP {code}).")
        } else {
            format!("Gemini hatası (HTTP {code}): {}", short(&text))
        };
        return Err((is_quota, msg));
    }

    // Başarılı yanıttan JSON metni çıkar
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| (false, format!("Yanıt çözümlenemedi: {e}")))?;
    let inner = v["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| (false, format!("Beklenmeyen yanıt biçimi: {}", short(&text))))?;
    let meta: GeneratedMeta = serde_json::from_str(inner)
        .map_err(|e| (false, format!("Üretilen JSON okunamadı: {e}")))?;
    Ok(meta)
}

fn short(s: &str) -> String {
    let t = s.trim();
    if t.len() > 240 {
        format!("{}…", &t[..240])
    } else {
        t.to_string()
    }
}

/// Kaç meta kuralının ihlal edildiğini sayar (daha iyi denemeyi seçmek için).
fn violation_count(meta: &GeneratedMeta) -> u32 {
    let tl = grapheme_len(meta.title.trim());
    let dl = grapheme_len(meta.descriptions.trim());
    let kw = meta.target_keyword.trim().to_lowercase();
    let mut n = 0;
    if !(20..=60).contains(&tl) {
        n += 1;
    }
    if !(50..=155).contains(&dl) {
        n += 1;
    }
    if !kw.is_empty() {
        if !meta.title.to_lowercase().contains(&kw) {
            n += 1;
        }
        if !meta.descriptions.to_lowercase().contains(&kw) {
            n += 1;
        }
    }
    n
}

/// Fazla uzun alanları kelime sınırında grapheme-bazlı kırpar (üretim tüketmez).
/// Yalnızca üst sınırı aşanları düzeltir; kısa olanlara dokunmaz.
fn clamp_lengths(mut meta: GeneratedMeta) -> GeneratedMeta {
    meta.title = clamp_to(meta.title.trim(), 60);
    meta.descriptions = clamp_to(meta.descriptions.trim(), 155);
    meta
}

fn clamp_to(s: &str, max: usize) -> String {
    use unicode_segmentation::UnicodeSegmentation;
    let graphemes: Vec<&str> = s.graphemes(true).collect();
    if graphemes.len() <= max {
        return s.to_string();
    }
    let mut cut: String = graphemes[..max].concat();
    // Son boşluğa kadar geri sar (kelimeyi ortadan kesme)
    if let Some(idx) = cut.rfind(' ') {
        if idx >= max / 2 {
            cut.truncate(idx);
        }
    }
    // Sondaki noktalama/boşlukları temizle
    cut.trim_end_matches([' ', ',', ';', ':', '-', '.', '…'])
        .to_string()
}

/// Meta kurallarını kontrol et; başarısızsa retry için düzeltme metni üret.
fn correction_for(meta: &GeneratedMeta) -> Option<String> {
    let badge = crate::validation::meta_badge(&MetaInput {
        title: &meta.title,
        descriptions: &meta.descriptions,
        target_keyword: &meta.target_keyword,
        meta_done: false,
    });
    if badge == MetaBadge::Uygun {
        return None;
    }
    let tl = grapheme_len(meta.title.trim());
    let dl = grapheme_len(meta.descriptions.trim());
    let mut msgs = Vec::new();
    if tl < 20 {
        msgs.push(format!("title {tl} karakter, çok kısa; 20-60 arasına uzat."));
    } else if tl > 60 {
        msgs.push(format!("title {tl} karakter, çok uzun; 20-60 arasına kısalt."));
    }
    if dl < 50 {
        msgs.push(format!("descriptions {dl} karakter, çok kısa; 50-155 arasına uzat."));
    } else if dl > 155 {
        msgs.push(format!("descriptions {dl} karakter, çok uzun; 50-155 arasına kısalt."));
    }
    let kw = meta.target_keyword.trim().to_lowercase();
    if !kw.is_empty() {
        if !meta.title.to_lowercase().contains(&kw) {
            msgs.push("title hedef kelimeyi içermeli.".to_string());
        }
        if !meta.descriptions.to_lowercase().contains(&kw) {
            msgs.push("descriptions hedef kelimeyi içermeli.".to_string());
        }
    }
    if msgs.is_empty() {
        Some("Kurallara tam uy.".to_string())
    } else {
        Some(msgs.join(" "))
    }
}

/// Model zincirini gezerek üretir; kural fail'inde tek retry yapar.
pub async fn generate_meta(
    api_key: &str,
    ctx: &ProductContext<'_>,
) -> Result<GeneratedMeta, String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("Gemini API anahtarı ayarlı değil. Ayarlar'dan ekleyin.".to_string());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;

    let mut last_err = String::from("Bilinmeyen hata");
    for (i, model) in MODEL_CHAIN.iter().enumerate() {
        let prompt = build_prompt(ctx, None);
        match call_model(&client, key, model, &prompt).await {
            Ok(meta) => {
                // Kural fail ederse aynı modelle tek retry; iki denemenin daha iyisini seç,
                // sonra fazla uzun alanları kelime sınırında kırparak uzunluğu garantile.
                let best = match correction_for(&meta) {
                    None => meta,
                    Some(correction) => {
                        let prompt2 = build_prompt(ctx, Some(&correction));
                        match call_model(&client, key, model, &prompt2).await {
                            Ok(meta2) if violation_count(&meta2) <= violation_count(&meta) => meta2,
                            _ => meta,
                        }
                    }
                };
                return Ok(clamp_lengths(best));
            }
            Err((is_quota, msg)) => {
                last_err = msg;
                // Kota değilse (anahtar/ağ/biçim hatası) fallback anlamsız, hemen dön
                if !is_quota {
                    return Err(last_err);
                }
                // Kota ise sıradaki modele geç
                let _ = i;
            }
        }
    }
    Err(format!("Tüm modeller denendi, üretim başarısız. Son hata: {last_err}"))
}

/// Faz 3: details HTML üretimi (şimdilik iskelet).
#[allow(dead_code)]
pub async fn generate_details(
    _api_key: &str,
    _details_html: &str,
    _target_keyword: &str,
) -> Result<String, String> {
    Err("Açıklama üretimi Faz 3'te aktif olacak.".to_string())
}

/// Ayarlardaki "Bağlantıyı test et" — hafif bir modelle gerçek bağlantı denemesi.
pub async fn test_key(api_key: &str) -> Result<String, String> {
    let key = api_key.trim();
    if key.len() < 20 {
        return Err("Anahtar çok kısa görünüyor.".to_string());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;
    // models listesini çekerek anahtarı doğrula (üretim tüketmez)
    let url = format!("{API_BASE}?key={key}");
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Gemini'ye ulaşılamadı: {e}"))?;
    if resp.status().is_success() {
        Ok("Anahtar geçerli · Gemini bağlantısı doğrulandı.".to_string())
    } else if resp.status().as_u16() == 400 || resp.status().as_u16() == 403 {
        Err("Anahtar reddedildi · geçersiz veya yetkisiz.".to_string())
    } else {
        Err(format!("Beklenmeyen yanıt: HTTP {}", resp.status().as_u16()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{meta_badge, MetaInput};

    /// Gerçek Gemini üretimi. GEMINI_API_KEY ortam değişkeni gerekir.
    /// `GEMINI_API_KEY=... cargo test gen_meta_real -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn gen_meta_real() {
        let key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY ayarlı değil");
        let ctx = ProductContext {
            name: "Lenovo ThinkCentre Neo 50a Gen 5 i7-13620H 16G 512G DOS 27''",
            brand: Some("Lenovo"),
            category: Some("All In One Bilgisayar"),
            main_category: Some("Bilgisayar"),
        };
        let meta = generate_meta(&key, &ctx).await.expect("üretim başarısız");
        println!("target_keyword: {}", meta.target_keyword);
        println!("title ({}): {}", grapheme_len(&meta.title), meta.title);
        println!("descriptions ({}): {}", grapheme_len(&meta.descriptions), meta.descriptions);
        println!("keywords: {}", meta.keywords);
        println!("search_keywords: {}", meta.search_keywords);

        let badge = meta_badge(&MetaInput {
            title: &meta.title,
            descriptions: &meta.descriptions,
            target_keyword: &meta.target_keyword,
            meta_done: false,
        });
        println!("rozet: {:?}", badge);
        assert!(!meta.title.trim().is_empty());
        assert!(!meta.descriptions.trim().is_empty());
        assert!(!meta.target_keyword.trim().is_empty());
    }

    #[test]
    fn clamp_trims_long_at_word_boundary() {
        // 200 karakterlik metni 155'e çeker, kelime ortasından kesmez
        let long = "Lenovo ThinkCentre Neo 50a ile iş yükünüzü hafifletin ve verimliliğinizi artırın. \
                    27 inç ekranı ve güçlü i7 işlemcisiyle hem şık hem de son derece performanslı bir masaüstü deneyimi yaşayın bugün.";
        let out = clamp_to(long, 155);
        assert!(grapheme_len(&out) <= 155, "kırpılmadı: {}", grapheme_len(&out));
        assert!(!out.ends_with(' '));
        // kısa metne dokunmaz
        assert_eq!(clamp_to("kısa metin", 155), "kısa metin");
    }

    #[test]
    fn violation_count_flags_length_and_keyword() {
        let bad = GeneratedMeta {
            target_keyword: "kulaklık".into(),
            title: "Kısa".into(),                 // <20
            descriptions: "Çok kısa açıklama".into(), // <50
            keywords: "".into(),
            search_keywords: "".into(),
        };
        // title uzunluk + desc uzunluk + title kw + desc kw = 4
        assert_eq!(violation_count(&bad), 4);
    }

    #[tokio::test]
    #[ignore]
    async fn test_key_real() {
        let key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY ayarlı değil");
        let msg = test_key(&key).await.expect("anahtar testi başarısız");
        println!("{msg}");
    }
}
