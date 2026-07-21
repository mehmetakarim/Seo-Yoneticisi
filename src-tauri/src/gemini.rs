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

// ====================== Faz 3: details üretimi ======================
//
// Yaklaşım: **yapı korunur, yalnızca metin yenilenir.** Orijinal HTML iskeletinden
// h2/p elemanlarının iç metinleri sırayla çıkarılır, Gemini bunları yeniden yazar,
// aynı konumlara geri yerleştirilir. section/col-md/class ve <img> hiç dokunulmaz →
// görsel güvenliği by-design. Ek güvenlik: üretim öncesi/sonrası img src listesi
// karşılaştırılır; ayrıca yeniden yazılan metinler <strong>/<em> dışındaki etiketlerden
// arındırılır (script/img/başlık etiketi enjeksiyonu engellenir).

/// ASCII-lowercase kopya (byte uzunluğunu korur → orijinalle indeks hizalı).
fn ascii_lower_bytes(s: &str) -> Vec<u8> {
    s.bytes().map(|c| c.to_ascii_lowercase()).collect()
}

fn find_bytes(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || from >= hay.len() {
        return None;
    }
    hay[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

/// h2/p elemanlarının iç metin aralıklarını (byte indeksleri) belge sırasında döndürür.
/// h2/p bu feed'de yaprak elemandır (içlerinde aynı türden iç içe eleman yok).
fn extract_segments(html: &str) -> Vec<(usize, usize)> {
    let lb = ascii_lower_bytes(html);
    let n = lb.len();
    let mut segs = Vec::new();
    let mut i = 0;
    while i < n {
        if lb[i] == b'<' && i + 1 < n && lb[i + 1] != b'/' {
            let mut j = i + 1;
            while j < n && lb[j].is_ascii_alphanumeric() {
                j += 1;
            }
            let name = &lb[i + 1..j];
            let close: Option<&[u8]> = match name {
                b"h2" => Some(b"</h2>"),
                b"p" => Some(b"</p>"),
                _ => None,
            };
            if let Some(close) = close {
                if let Some(gt) = find_bytes(&lb, b">", j) {
                    let inner_start = gt + 1;
                    if let Some(ce) = find_bytes(&lb, close, inner_start) {
                        segs.push((inner_start, ce));
                        i = ce + close.len();
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    segs
}

/// <img ... src="..."> değerlerini sırayla toplar (invariant kontrolü için).
fn extract_img_srcs(html: &str) -> Vec<String> {
    let re = regex::Regex::new(r#"(?is)<img[^>]*?\bsrc\s*=\s*["']([^"']*)["']"#).unwrap();
    re.captures_iter(html)
        .map(|c| c[1].to_string())
        .collect()
}

/// Yeniden yazılan metni yalnızca <strong>/<b>/<em>/<br> inline etiketlerine izin
/// verecek biçimde temizler; diğer tüm etiketleri (img/script/section/h2/p...) atar.
fn sanitize_inline(text: &str) -> String {
    let re = regex::Regex::new(r"(?is)<[^>]+>").unwrap();
    let out = re.replace_all(text, |caps: &regex::Captures| {
        let t = caps[0].to_ascii_lowercase();
        let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
        let t = t.replace("< ", "<").replace(" >", ">");
        match t.as_str() {
            "<strong>" | "</strong>" | "<b>" | "</b>" | "<em>" | "</em>" | "<br>" | "<br/>"
            | "<br />" => caps[0].to_string(),
            _ => String::new(),
        }
    });
    out.trim().to_string()
}

fn splice(html: &str, segs: &[(usize, usize)], reps: &[String]) -> String {
    let mut out = String::with_capacity(html.len() + 256);
    let mut last = 0;
    for ((s, e), rep) in segs.iter().zip(reps.iter()) {
        out.push_str(&html[last..*s]);
        out.push_str(rep);
        last = *e;
    }
    out.push_str(&html[last..]);
    out
}

fn details_system_prompt() -> &'static str {
    "Sen bir Türk e-ticaret SEO uzmanısın. Sana bir ürünün açıklama bölümündeki metin \
     parçaları (başlıklar ve paragraflar) SIRAYLA bir JSON dizisi olarak verilir. \
     Her parçayı SEO uyumlu, akıcı ve özgün Türkçe ile YENİDEN YAZ. KURALLAR: \
     1) Dönen dizi, verilen diziyle AYNI uzunlukta ve AYNI sırada olmalı (parça sayısını değiştirme). \
     2) 'h2' etiketli parçalar kısa başlık, 'p' etiketli parçalar paragraf olarak kalmalı. \
     3) Hedef kelimeyi metinlere doğal biçimde serpiştir; genel yoğunluğu %2-3 civarında tut. \
     Paragraflarda hedef kelimeyi birkaç kez <strong>hedef kelime</strong> biçiminde vurgulayabilirsin. \
     4) YALNIZCA <strong> ve <em> inline etiketlerine izin var. <img>, <section>, <h2>, <p>, <script> \
     veya başka HTML EKLEME. 5) Toplam metin en az 50 kelime olmalı. \
     Yalnızca yeniden yazılmış metinlerden oluşan JSON string dizisini döndür."
}

async fn call_details_model(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    prompt: &str,
    expected: usize,
) -> Result<Vec<String>, (bool, String)> {
    let url = format!("{API_BASE}/{model}:generateContent");
    let body = serde_json::json!({
        "system_instruction": { "parts": [{ "text": details_system_prompt() }] },
        "contents": [{ "parts": [{ "text": prompt }] }],
        "generationConfig": {
            "responseMimeType": "application/json",
            "responseSchema": { "type": "ARRAY", "items": { "type": "STRING" } },
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
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| (false, format!("Yanıt çözümlenemedi: {e}")))?;
    let inner = v["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| (false, format!("Beklenmeyen yanıt biçimi: {}", short(&text))))?;
    let arr: Vec<String> = serde_json::from_str(inner)
        .map_err(|e| (false, format!("Üretilen dizi okunamadı: {e}")))?;
    let _ = expected; // uzunluk uyuşmazlığı çağıran tarafta best-effort ele alınır
    Ok(arr)
}

/// Details HTML'ini yapıyı koruyarak yeniden üretir.
pub async fn generate_details(
    api_key: &str,
    ctx: &ProductContext<'_>,
    details_html: &str,
    target_keyword: &str,
) -> Result<String, String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("Gemini API anahtarı ayarlı değil. Ayarlar'dan ekleyin.".to_string());
    }
    let segs = extract_segments(details_html);
    if segs.is_empty() {
        return Err("Bu üründe yeniden yazılacak açıklama metni bulunamadı.".to_string());
    }

    // Parçaların düz metinleri + etiket türü (h2/p) — model için girdi
    let lb = ascii_lower_bytes(details_html);
    let items: Vec<serde_json::Value> = segs
        .iter()
        .map(|(s, e)| {
            let inner = &details_html[*s..*e];
            // Etiket türünü kapanış etiketinden anla
            let tag = if lb[*e..].starts_with(b"</h2") { "h2" } else { "p" };
            serde_json::json!({ "tag": tag, "text": crate::validation::html_strip(inner) })
        })
        .collect();

    let original_imgs = extract_img_srcs(details_html);

    let prompt = format!(
        "Ürün adı: {}\nMarka: {}\nKategori: {}\nHedef kelime: {}\n\nParçalar (sırayla): {}",
        ctx.name,
        ctx.brand.unwrap_or("-"),
        ctx.category.unwrap_or("-"),
        target_keyword,
        serde_json::to_string(&items).unwrap_or_default(),
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;

    let mut last_err = String::from("Bilinmeyen hata");
    for model in MODEL_CHAIN.iter() {
        match call_details_model(&client, key, model, &prompt, segs.len()).await {
            Ok(mut arr) => {
                // Uzunluk uyuşmazsa aynı modelle tek retry
                if arr.len() != segs.len() {
                    let corr = format!(
                        "{prompt}\n\nÖNEMLİ: Tam olarak {} parça döndür, ne eksik ne fazla.",
                        segs.len()
                    );
                    if let Ok(arr2) = call_details_model(&client, key, model, &corr, segs.len()).await
                    {
                        arr = arr2;
                    }
                }
                // Best-effort: her parça için yeniden yazılanı (varsa) temizleyip kullan,
                // yoksa orijinal iç metni koru.
                let reps: Vec<String> = segs
                    .iter()
                    .enumerate()
                    .map(|(idx, (s, e))| {
                        let original = &details_html[*s..*e];
                        match arr.get(idx) {
                            Some(t) if !t.trim().is_empty() => sanitize_inline(t),
                            _ => original.to_string(),
                        }
                    })
                    .collect();

                let result = splice(details_html, &segs, &reps);

                // Img invariant: yeni HTML'deki src'ler orijinalle aynı olmalı.
                // sanitize_inline <img>'leri zaten atar, iskelet dokunulmadı → eşit beklenir.
                let new_imgs = extract_img_srcs(&result);
                if new_imgs != original_imgs {
                    // Teorik olarak ulaşılmaz; ulaşılırsa orijinal HTML'i koru (görsel güvenliği).
                    return Ok(details_html.to_string());
                }
                return Ok(result);
            }
            Err((is_quota, msg)) => {
                last_err = msg;
                if !is_quota {
                    return Err(last_err);
                }
            }
        }
    }
    Err(format!("Tüm modeller denendi, açıklama üretilemedi. Son hata: {last_err}"))
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

    const SAMPLE_DETAILS: &str = r#"<section class="yeni-aciklama center"><section class="container"><section class="row align-items-center"><section class="col-md-4"><img loading="lazy" class="img-big-size" alt="image" src="https://cdn.example/a.jpg" /></section><section class="col-md-8 text-center"><h2>Güçlü Performans</h2><p class="des-new-boyut">Bu ürün üstün performans sunar.</p></section></section></section></section><section class="yeni-aciklama left"><section class="col-md-4"><img src="https://cdn.example/b.jpg" /></section><h2>Net Ekran</h2><p>27 inç ekranı ile net görüntü.</p></section>"#;

    #[test]
    fn extract_segments_finds_h2_and_p_in_order() {
        let segs = extract_segments(SAMPLE_DETAILS);
        assert_eq!(segs.len(), 4);
        let texts: Vec<&str> = segs.iter().map(|(s, e)| &SAMPLE_DETAILS[*s..*e]).collect();
        assert_eq!(texts[0], "Güçlü Performans");
        assert_eq!(texts[1], "Bu ürün üstün performans sunar.");
        assert_eq!(texts[2], "Net Ekran");
        assert_eq!(texts[3], "27 inç ekranı ile net görüntü.");
    }

    #[test]
    fn extract_img_srcs_collects_all() {
        let imgs = extract_img_srcs(SAMPLE_DETAILS);
        assert_eq!(imgs, vec!["https://cdn.example/a.jpg", "https://cdn.example/b.jpg"]);
    }

    #[test]
    fn splice_preserves_structure_and_imgs() {
        let segs = extract_segments(SAMPLE_DETAILS);
        let reps: Vec<String> = vec![
            "Yeni Başlık".into(),
            "Yeni <strong>hedef</strong> paragraf.".into(),
            "İkinci Başlık".into(),
            "İkinci yeni paragraf metni.".into(),
        ];
        let out = splice(SAMPLE_DETAILS, &segs, &reps);
        // Metin değişti
        assert!(out.contains("Yeni Başlık"));
        assert!(out.contains("Yeni <strong>hedef</strong> paragraf."));
        assert!(!out.contains("Güçlü Performans"));
        // Yapı ve img'ler korundu
        assert!(out.contains(r#"<section class="col-md-8 text-center">"#));
        assert_eq!(extract_img_srcs(&out), extract_img_srcs(SAMPLE_DETAILS));
        assert_eq!(out.matches("<h2>").count(), 2);
        assert_eq!(out.matches("<img").count(), 2);
    }

    #[test]
    fn sanitize_inline_strips_disallowed_tags() {
        let dirty = "İyi <strong>hedef</strong> <script>alert(1)</script> \
                     <img src='x'> ürün <em>güzel</em> <p>yeni</p>";
        let clean = sanitize_inline(dirty);
        assert!(clean.contains("<strong>hedef</strong>"));
        assert!(clean.contains("<em>güzel</em>"));
        assert!(!clean.contains("<script"));
        assert!(!clean.contains("<img"));
        assert!(!clean.contains("<p>"));
        // script/img/p etiketleri gitti, metin kaldı
        assert!(clean.contains("ürün"));
    }

    /// Gerçek Gemini details üretimi. GEMINI_API_KEY gerekir.
    /// `GEMINI_API_KEY=... cargo test gen_details_real -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn gen_details_real() {
        let key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY ayarlı değil");
        let ctx = ProductContext {
            name: "Lenovo ThinkCentre Neo 50a 27'' All In One",
            brand: Some("Lenovo"),
            category: Some("All In One Bilgisayar"),
            main_category: Some("Bilgisayar"),
        };
        let out = generate_details(&key, &ctx, SAMPLE_DETAILS, "all in one bilgisayar")
            .await
            .expect("üretim başarısız");
        println!("--- ÜRETİLEN ---\n{out}\n----------------");
        // Yapı ve img'ler korunmalı
        assert_eq!(extract_img_srcs(&out), extract_img_srcs(SAMPLE_DETAILS));
        assert_eq!(out.matches("<h2>").count(), 2);
        assert_eq!(out.matches("<img").count(), 2);
        // Metin değişmiş olmalı
        assert!(!out.contains("Güçlü Performans") || !out.contains("Bu ürün üstün performans sunar."));
        println!("kelime: {}", crate::validation::word_count(&out));
    }

    #[tokio::test]
    #[ignore]
    async fn test_key_real() {
        let key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY ayarlı değil");
        let msg = test_key(&key).await.expect("anahtar testi başarısız");
        println!("{msg}");
    }
}
