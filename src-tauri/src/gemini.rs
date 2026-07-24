//! Gemini meta üretimi (Faz 2).
//!
//! - Model **kademeli fallback**: biri kota/limit (429) verirse sıradaki denenir.
//! - JSON structured output (responseSchema) → 5 alan.
//! - Üretim sonrası kural fail ederse **tek retry** (kısalt/uzat yönergesiyle).
//!
//! Faz 3: `generate_details` gerçek implementasyona kavuşacak (yapı korunur, img-src güvenliği).

use crate::seo_data::SeoInsights;
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
    /// Faz 4: kullanıcının araştırma panelinde onayladığı hedef kelime (meta üretiminde
    /// "türet" adımını devre dışı bırakır). None → mevcut davranış (model türetir).
    pub target_keyword: Option<&'a str>,
    /// Faz 4: gerçek SEO verisi (Ahrefs/GSC/Trends). None → prompt'a hiçbir şey eklenmez.
    pub insights: Option<&'a SeoInsights>,
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
    // Onaylı hedef kelime varsa: modele "türetme, bunu kullan" de.
    if let Some(kw) = ctx.target_keyword.map(str::trim).filter(|s| !s.is_empty()) {
        p.push_str(&format!(
            "\nOnaylı HEDEF KELIME: {kw}\n(Bunu aynen target_keyword olarak kullan; yeniden türetme.)\n"
        ));
    }
    // Gerçek arama verilerini enjekte et (varsa).
    if let Some(ins) = ctx.insights {
        p.push_str(&ins.prompt_block());
    }
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

/// Yeniden yazılabilir içerik (h2/p metni) var mı? Yoksa sıfırdan üretim uygundur.
pub fn has_rewritable_content(html: &str) -> bool {
    !extract_segments(html).is_empty()
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

    let insights_block = ctx.insights.map(|i| i.prompt_block()).unwrap_or_default();
    let prompt = format!(
        "Ürün adı: {}\nMarka: {}\nKategori: {}\nHedef kelime: {}{}\n\nParçalar (sırayla): {}",
        ctx.name,
        ctx.brand.unwrap_or("-"),
        ctx.category.unwrap_or("-"),
        target_keyword,
        insights_block,
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

// ====================== Faz 7: sıfırdan (semantik) açıklama ======================
//
// Açıklaması olmayan ürünler için standart HTML açıklamayı **sıfırdan** üretir. Yapı semantik olarak
// DOĞRU: dış tematik blok tek `<section class="yeni-aciklama {center|left|right}">`, iç yerleşim `<div>`
// (Bootstrap grid), `<img>`'de anlamlı `alt`. Verilen CSS yalnızca dış `section.yeni-aciklama.*`'ı
// hedeflediği için görünüm birebir korunur. Görseller ürünün gerçek galeri fotoğraflarıdır.

fn scratch_system_prompt() -> &'static str {
    "Sen bir Türk e-ticaret SEO uzmanısın. Bir ürün için açıklama bölümleri üreteceksin. Sana kaç bölüm \
     istendiği söylenir; her bölüm bir ürün görseline eşlik eder. Her bölüm için kısa bir BAŞLIK (h2) ve \
     akıcı, özgün, satışa yönelik bir PARAGRAF (p) yaz. KURALLAR: 1) Tam olarak istenen sayıda bölüm \
     döndür. 2) Hedef kelimeyi metinlere doğal biçimde serpiştir; genel yoğunluğu %2-3 civarında tut; \
     paragraflarda birkaç kez <strong>hedef kelime</strong> biçiminde vurgulayabilirsin. 3) YALNIZCA \
     <strong> ve <em> inline etiketlerine izin var; başka HTML EKLEME. 4) Her paragraf en az 30 kelime \
     olsun. Yalnızca istenen JSON dizisini döndür: [{\"h2\":\"...\",\"p\":\"...\"}, ...]."
}

async fn call_scratch_model(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    prompt: &str,
) -> Result<Vec<(String, String)>, (bool, String)> {
    let url = format!("{API_BASE}/{model}:generateContent");
    let body = serde_json::json!({
        "system_instruction": { "parts": [{ "text": scratch_system_prompt() }] },
        "contents": [{ "parts": [{ "text": prompt }] }],
        "generationConfig": {
            "responseMimeType": "application/json",
            "responseSchema": {
                "type": "ARRAY",
                "items": {
                    "type": "OBJECT",
                    "properties": { "h2": { "type": "STRING" }, "p": { "type": "STRING" } },
                    "required": ["h2", "p"]
                }
            },
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
    let arr: Vec<serde_json::Value> = serde_json::from_str(inner)
        .map_err(|e| (false, format!("Üretilen dizi okunamadı: {e}")))?;
    Ok(arr
        .into_iter()
        .map(|o| {
            (
                o.get("h2").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                o.get("p").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            )
        })
        .collect())
}

/// HTML özel karakterlerini attribute/metin için kaçırır (alt metni güvenliği).
fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Blok sınıfı döngüsü: ilk blok `center`, sonrası `left`/`right` şeklinde alternatif
/// (center → left → right → left → right …).
fn class_for(i: usize) -> &'static str {
    if i == 0 {
        "center"
    } else if i % 2 == 1 {
        "left"
    } else {
        "right"
    }
}

/// Bölüm metinleri + galeri görsellerinden semantik HTML montajlar.
fn assemble_scratch(name: &str, sections: &[(String, String)], images: &[String]) -> String {
    let n = sections.len().min(images.len());
    let mut out = String::with_capacity(1024);
    for i in 0..n {
        let (h2, p) = &sections[i];
        let img = &images[i];
        let class = class_for(i);
        let h2c = sanitize_inline(h2);
        let pc = sanitize_inline(p);
        let alt = esc(&format!("{name} — {}", html_strip_local(&h2c)));
        // Çift indekste görsel solda, tekte sağda (görsel ritim); yapı yine semantik.
        let (col_a, col_b) = if i % 2 == 0 {
            (
                format!(r#"<div class="col-md-4"><img class="img-big-size imgRadius" loading="lazy" alt="{alt}" src="{}" /></div>"#, esc(img)),
                format!(r#"<div class="col-md-8 text-center"><h2>{h2c}</h2><p class="des-new-boyut">{pc}</p></div>"#),
            )
        } else {
            (
                format!(r#"<div class="col-md-8 text-center"><h2>{h2c}</h2><p class="des-new-boyut">{pc}</p></div>"#),
                format!(r#"<div class="col-md-4"><img class="img-big-size imgRadius" loading="lazy" alt="{alt}" src="{}" /></div>"#, esc(img)),
            )
        };
        out.push_str(&format!(
            r#"<section class="yeni-aciklama {class}"><div class="container"><div class="row align-items-center">{col_a}{col_b}</div></div></section>"#
        ));
    }
    out
}

/// Etiketleri sökerek düz metin (alt için). validation::html_strip'in yerel kopyası.
fn html_strip_local(s: &str) -> String {
    crate::validation::html_strip(s)
}

/// Açıklaması olmayan ürün için sıfırdan semantik HTML açıklama üretir.
pub async fn generate_details_scratch(
    api_key: &str,
    ctx: &ProductContext<'_>,
    images: &[String],
    target_keyword: &str,
) -> Result<String, String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("Gemini API anahtarı ayarlı değil. Ayarlar'dan ekleyin.".to_string());
    }
    if images.is_empty() {
        return Err("Sıfırdan açıklama için ürün görseli gerekli.".to_string());
    }
    let insights_block = ctx.insights.map(|i| i.prompt_block()).unwrap_or_default();
    let prompt = format!(
        "Ürün adı: {}\nMarka: {}\nKategori: {}\nHedef kelime: {}{}\n\nİstenen bölüm sayısı: {} \
         (her bölüm bir ürün görseline eşlik edecek).",
        ctx.name,
        ctx.brand.unwrap_or("-"),
        ctx.category.unwrap_or("-"),
        target_keyword,
        insights_block,
        images.len(),
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;

    let mut last_err = String::from("Bilinmeyen hata");
    for model in MODEL_CHAIN.iter() {
        match call_scratch_model(&client, key, model, &prompt).await {
            Ok(mut sections) => {
                // Bölüm sayısı görselden azsa tek retry (tam sayıyı iste)
                if sections.len() < images.len() {
                    let corr = format!(
                        "{prompt}\n\nÖNEMLİ: Tam olarak {} bölüm döndür.",
                        images.len()
                    );
                    if let Ok(s2) = call_scratch_model(&client, key, model, &corr).await {
                        if s2.len() > sections.len() {
                            sections = s2;
                        }
                    }
                }
                if sections.is_empty() {
                    return Err("Model boş içerik döndürdü.".to_string());
                }
                return Ok(assemble_scratch(ctx.name, &sections, images));
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

// ============ Faz 7b: MEVCUT açıklamayı optimize et (yapı + metin + alt) ============
//
// "Açıklamayı Üret"in asıl işi mevcut içeriği optimize etmek. Eski yol metni yerinde değiştirip
// (hatalı) iç içe `<section>` yapısını aynen koruyordu. Yeni yol: blokları çıkarır, metinleri
// Gemini ile optimize eder ve **semantik** olarak yeniden inşa eder (dış section + iç div),
// `<img>`'lere anlamlı `alt` ekler. Görseller birebir korunur (invariant); yapı beklenmedikse
// `None` döner ve çağıran eski güvenli yola düşer.

/// Çıkarılan bir üst-düzey blok.
struct Block {
    /// `yeni-aciklama` dışındaki özel sınıf (ör. `pre-order`) — varsa korunur.
    special_class: Option<String>,
    images: Vec<String>,
    h2: String,
    p: String,
}

/// Üst düzey `<section>` aralıklarını derinlik sayarak bulur. Aralarında boşluk dışında
/// içerik varsa (beklenmedik yapı) None döner.
fn split_top_sections(html: &str) -> Option<Vec<(usize, usize)>> {
    let lb = ascii_lower_bytes(html);
    let n = lb.len();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut i = 0usize;
    let mut depth = 0i32;
    let mut start = 0usize;
    while i < n {
        if lb[i] == b'<' {
            if lb[i..].starts_with(b"</section") {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
                if depth == 0 {
                    let gt = find_bytes(&lb, b">", i)?;
                    spans.push((start, gt + 1));
                    i = gt + 1;
                    continue;
                }
            } else if lb[i..].starts_with(b"<section") {
                if depth == 0 {
                    start = i;
                }
                depth += 1;
            }
        }
        i += 1;
    }
    if depth != 0 || spans.is_empty() {
        return None;
    }
    // Bloklar arasında/dışında yalnızca boşluk olmalı
    let mut cursor = 0usize;
    for (s, e) in &spans {
        if !html[cursor..*s].trim().is_empty() {
            return None;
        }
        cursor = *e;
    }
    if !html[cursor..].trim().is_empty() {
        return None;
    }
    Some(spans)
}

/// Blokları çıkarır. Her blok tam olarak 1 `<h2>` ve ≥1 `<p>` içermeli; aksi halde None
/// (düzensiz yapı → çağıran eski yola düşer).
fn extract_blocks(html: &str) -> Option<Vec<Block>> {
    let spans = split_top_sections(html)?;
    let class_re = regex::Regex::new(r#"(?is)^<section[^>]*class\s*=\s*["']([^"']*)["']"#).unwrap();
    let h2_re = regex::Regex::new(r"(?is)<h2[^>]*>(.*?)</h2>").unwrap();
    let p_re = regex::Regex::new(r"(?is)<p[^>]*>(.*?)</p>").unwrap();

    let mut blocks = Vec::new();
    for (s, e) in spans {
        let seg = &html[s..e];
        let classes = class_re.captures(seg).map(|c| c[1].to_string()).unwrap_or_default();
        if !classes.contains("yeni-aciklama") {
            return None; // beklenmedik kabuk
        }
        let special: Option<String> = classes
            .split_whitespace()
            .find(|c| !matches!(*c, "yeni-aciklama" | "center" | "left" | "right"))
            .map(|c| c.to_string());

        let h2s: Vec<String> = h2_re.captures_iter(seg).map(|c| c[1].trim().to_string()).collect();
        let ps: Vec<String> = p_re.captures_iter(seg).map(|c| c[1].trim().to_string()).collect();
        if h2s.len() != 1 || ps.is_empty() {
            return None; // düzensiz blok → güvenli tarafta kal
        }
        blocks.push(Block {
            special_class: special,
            images: extract_img_srcs(seg),
            h2: h2s[0].clone(),
            p: ps.join(" "),
        });
    }
    Some(blocks)
}

/// Optimize edilmiş metinleri orijinal görsellerle semantik HTML'e montajlar.
fn assemble_optimized(name: &str, blocks: &[Block], texts: &[(String, String)]) -> String {
    let mut out = String::with_capacity(1024);
    for (i, b) in blocks.iter().enumerate() {
        let (h2_raw, p_raw) = texts
            .get(i)
            .cloned()
            .unwrap_or_else(|| (b.h2.clone(), b.p.clone())); // eksikse orijinali koru
        let h2c = sanitize_inline(&h2_raw);
        let pc = sanitize_inline(&p_raw);
        let class = b.special_class.clone().unwrap_or_else(|| class_for(i).to_string());
        let alt = esc(&format!("{name} — {}", html_strip_local(&h2c)));

        let text_col_class = if b.images.is_empty() { "col-md-12" } else { "col-md-8" };
        let text_col = format!(
            r#"<div class="{text_col_class} text-center"><h2>{h2c}</h2><p class="des-new-boyut">{pc}</p></div>"#
        );
        let inner = if b.images.is_empty() {
            text_col
        } else {
            let imgs: String = b
                .images
                .iter()
                .map(|src| {
                    format!(
                        r#"<img class="img-big-size imgRadius" loading="lazy" alt="{alt}" src="{}" />"#,
                        esc(src)
                    )
                })
                .collect();
            let img_col = format!(r#"<div class="col-md-4">{imgs}</div>"#);
            // Görsel ritmi: çift indekste solda, tekte sağda
            if i % 2 == 0 {
                format!("{img_col}{text_col}")
            } else {
                format!("{text_col}{img_col}")
            }
        };
        out.push_str(&format!(
            r#"<section class="yeni-aciklama {class}"><div class="container"><div class="row align-items-center">{inner}</div></div></section>"#
        ));
    }
    out
}

/// Mevcut açıklamayı optimize eder: metinleri iyileştirir + yapıyı semantik hale getirir + alt ekler.
/// `Ok(None)` → yapı beklenmedik, çağıran eski (yapı-koruyan) yola düşmeli.
pub async fn optimize_details(
    api_key: &str,
    ctx: &ProductContext<'_>,
    details_html: &str,
    target_keyword: &str,
) -> Result<Option<String>, String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("Gemini API anahtarı ayarlı değil. Ayarlar'dan ekleyin.".to_string());
    }
    let blocks = match extract_blocks(details_html) {
        Some(b) if !b.is_empty() => b,
        _ => return Ok(None),
    };

    let items: Vec<serde_json::Value> = blocks
        .iter()
        .map(|b| {
            serde_json::json!({
                "h2": crate::validation::html_strip(&b.h2),
                "p": crate::validation::html_strip(&b.p),
            })
        })
        .collect();
    let insights_block = ctx.insights.map(|i| i.prompt_block()).unwrap_or_default();
    let prompt = format!(
        "Ürün adı: {}\nMarka: {}\nKategori: {}\nHedef kelime: {}{}\n\nAşağıda ürünün MEVCUT açıklama \
         bölümleri var. Her bölümün başlığını (h2) ve paragrafını (p) anlamını KORUYARAK SEO için \
         optimize et: daha akıcı, özgün ve satışa yönelik yaz, teknik bilgileri kaybetme. Tam olarak \
         {} bölüm döndür (aynı sırada).\n\nMevcut bölümler: {}",
        ctx.name,
        ctx.brand.unwrap_or("-"),
        ctx.category.unwrap_or("-"),
        target_keyword,
        insights_block,
        blocks.len(),
        serde_json::to_string(&items).unwrap_or_default(),
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;

    let mut last_err = String::from("Bilinmeyen hata");
    for model in MODEL_CHAIN.iter() {
        match call_scratch_model(&client, key, model, &prompt).await {
            Ok(mut texts) => {
                if texts.len() != blocks.len() {
                    let corr = format!("{prompt}\n\nÖNEMLİ: Tam olarak {} bölüm döndür.", blocks.len());
                    if let Ok(t2) = call_scratch_model(&client, key, model, &corr).await {
                        if t2.len() == blocks.len() {
                            texts = t2;
                        }
                    }
                }
                let result = assemble_optimized(ctx.name, &blocks, &texts);
                // Görsel değişmezliği: yeni HTML orijinaldeki tüm src'leri aynı sırada içermeli.
                if extract_img_srcs(&result) != extract_img_srcs(details_html) {
                    return Ok(None); // güvenli tarafta kal → eski yol
                }
                return Ok(Some(result));
            }
            Err((is_quota, msg)) => {
                last_err = msg;
                if !is_quota {
                    return Err(last_err);
                }
            }
        }
    }
    Err(format!("Tüm modeller denendi, açıklama optimize edilemedi. Son hata: {last_err}"))
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
            target_keyword: None,
            insights: None,
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

    /// Faz 4: onaylı hedef kelime + insights enjeksiyonu → model kelimeyi AYNEN kullanmalı,
    /// yeniden türetmemeli. GEMINI_API_KEY gerekir.
    /// `GEMINI_API_KEY=... cargo test gen_meta_with_insights_real -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn gen_meta_with_insights_real() {
        use crate::seo_data::{KeywordCand, KeywordDifficulty, SeoInsights};
        let key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY ayarlı değil");
        let insights = SeoInsights {
            seed: "hepsi bir arada bilgisayar".into(),
            target_candidates: vec![
                KeywordCand { keyword: "all in one bilgisayar".into(), difficulty: 18, volume: 4400, kind: "idea".into() },
                KeywordCand { keyword: "hepsi bir arada pc".into(), difficulty: 12, volume: 880, kind: "idea".into() },
            ],
            seed_difficulty: Some(KeywordDifficulty {
                keyword: "hepsi bir arada bilgisayar".into(),
                difficulty: 20,
                shortage: 0,
                last_update: "2026-07-01".into(),
            }),
            gsc_queries: vec![],
            trends: vec![],
            domain: None,
            fetched_at: "2026-07-22T00:00:00".into(),
            notes: vec![],
        };
        let ctx = ProductContext {
            name: "Lenovo ThinkCentre Neo 50a Gen 5 i7-13620H 16G 512G DOS 27''",
            brand: Some("Lenovo"),
            category: Some("All In One Bilgisayar"),
            main_category: Some("Bilgisayar"),
            target_keyword: Some("all in one bilgisayar"),
            insights: Some(&insights),
        };
        let meta = generate_meta(&key, &ctx).await.expect("üretim başarısız");
        println!("target_keyword: {}", meta.target_keyword);
        println!("title: {}", meta.title);
        println!("descriptions: {}", meta.descriptions);
        // Onaylı kelime aynen kullanılmalı
        assert_eq!(meta.target_keyword.trim().to_lowercase(), "all in one bilgisayar");
        assert!(meta.title.to_lowercase().contains("all in one bilgisayar"));
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
    fn assemble_scratch_is_semantic() {
        let sections = vec![
            ("Güçlü Performans".to_string(), "Bu ürün <strong>hızlı</strong> çalışır.".to_string()),
            ("Şık Tasarım".to_string(), "Zarif görünüm.".to_string()),
            ("Uzun Ömür".to_string(), "Dayanıklı yapı.".to_string()),
        ];
        let imgs = vec![
            "https://cdn/1.png".to_string(),
            "https://cdn/2.png".to_string(),
            "https://cdn/3.png".to_string(),
        ];
        let html = assemble_scratch("Lenovo Neo 50a", &sections, &imgs);
        // Dış tematik blok section, iç yerleşim div — iç içe section YOK
        assert_eq!(html.matches("<section").count(), 3);
        assert!(html.contains(r#"<div class="container">"#));
        assert!(html.contains(r#"<div class="col-md-4">"#));
        assert!(!html.contains("<section class=\"container\""));
        assert!(!html.contains("<section class=\"col-md-4\""));
        // Tüm görseller + anlamlı alt + başlık/metin
        assert!(html.contains("https://cdn/1.png"));
        assert!(html.contains("https://cdn/3.png"));
        assert!(html.contains(r#"alt="Lenovo Neo 50a — Güçlü Performans""#));
        assert!(html.contains("<h2>Güçlü Performans</h2>"));
        assert!(html.contains("<strong>hızlı</strong>"));
        // Sınıf döngüsü
        assert!(html.contains("yeni-aciklama center"));
        assert!(html.contains("yeni-aciklama left"));
        assert!(html.contains("yeni-aciklama right"));
    }

    /// Gerçek feed yapısı: iç içe <section>'lar, 1 img + 1 h2 + 1 p per blok.
    const REAL_DETAILS: &str = r#"<section class="yeni-aciklama center"><section class="container"><section class="row align-items-center"><section class="col-md-4"><img loading="lazy" class="img-big-size imgRadius" alt="image" src="https://cdn/a.jpg" /></section><section class="col-md-8 text-center"><h2>Güçlü Performans</h2><p class="des-new-boyut">Eski metin bir.</p></section></section></section></section><section class="yeni-aciklama left"><section class="container"><section class="row align-items-center"><section class="col-md-4"><img src="https://cdn/b.jpg" /></section><section class="col-md-8 text-center"><h2>Net Ekran</h2><p>Eski metin iki.</p></section></section></section></section>"#;

    #[test]
    fn class_cycle_is_center_then_alternating() {
        assert_eq!(class_for(0), "center");
        assert_eq!(class_for(1), "left");
        assert_eq!(class_for(2), "right");
        assert_eq!(class_for(3), "left");
        assert_eq!(class_for(4), "right");
    }

    #[test]
    fn extract_blocks_reads_real_structure() {
        let blocks = extract_blocks(REAL_DETAILS).expect("bloklar çıkarılmalı");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].h2, "Güçlü Performans");
        assert_eq!(blocks[0].p, "Eski metin bir.");
        assert_eq!(blocks[0].images, vec!["https://cdn/a.jpg"]);
        assert!(blocks[0].special_class.is_none());
        assert_eq!(blocks[1].images, vec!["https://cdn/b.jpg"]);
    }

    #[test]
    fn extract_blocks_rejects_irregular() {
        // 2 h2 tek blokta → düzensiz
        let bad = r#"<section class="yeni-aciklama center"><section class="col-md-12"><h2>A</h2><h2>B</h2><p>x</p></section></section>"#;
        assert!(extract_blocks(bad).is_none());
        // section dışı içerik → beklenmedik
        assert!(extract_blocks("<p>düz metin</p>").is_none());
    }

    #[test]
    fn optimize_assembly_is_semantic_and_keeps_images() {
        let blocks = extract_blocks(REAL_DETAILS).unwrap();
        let texts = vec![
            ("Üstün Performans".to_string(), "Yeni optimize metin bir.".to_string()),
            ("Berrak Ekran".to_string(), "Yeni optimize metin iki.".to_string()),
        ];
        let out = assemble_optimized("Lenovo Neo 50a", &blocks, &texts);
        // Semantik: dış section + iç div, iç içe section YOK
        assert_eq!(out.matches("<section").count(), 2);
        assert!(out.contains(r#"<div class="container">"#));
        assert!(!out.contains(r#"<section class="container""#));
        assert!(!out.contains(r#"<section class="col-md-4""#));
        // Görseller birebir korunur
        assert_eq!(extract_img_srcs(&out), extract_img_srcs(REAL_DETAILS));
        // alt="image" yerine anlamlı alt
        assert!(!out.contains(r#"alt="image""#));
        assert!(out.contains(r#"alt="Lenovo Neo 50a — Üstün Performans""#));
        // Metin optimize edildi
        assert!(out.contains("Yeni optimize metin bir."));
        assert!(!out.contains("Eski metin bir."));
        // Sınıf döngüsü
        assert!(out.contains("yeni-aciklama center"));
        assert!(out.contains("yeni-aciklama left"));
    }

    #[test]
    fn assemble_optimized_preserves_special_class_and_textonly() {
        let blocks = vec![Block {
            special_class: Some("pre-order".to_string()),
            images: vec![],
            h2: "Ön Sipariş".to_string(),
            p: "Bilgi".to_string(),
        }];
        let texts = vec![("Ön Sipariş".to_string(), "Yeni bilgi".to_string())];
        let out = assemble_optimized("Ürün", &blocks, &texts);
        assert!(out.contains("yeni-aciklama pre-order")); // özel sınıf korundu
        assert!(out.contains(r#"<div class="col-md-12 text-center">"#)); // görselsiz blok
        assert!(!out.contains("<img"));
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
            target_keyword: None,
            insights: None,
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

    /// Gerçek Gemini ile MEVCUT açıklama optimizasyonu (yapı + metin + alt).
    /// `GEMINI_API_KEY=... cargo test optimize_real -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn optimize_real() {
        let key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY ayarlı değil");
        let ctx = ProductContext {
            name: "Lenovo ThinkCentre Neo 50a 27'' All In One",
            brand: Some("Lenovo"),
            category: Some("All In One Bilgisayar"),
            main_category: Some("Bilgisayar"),
            target_keyword: None,
            insights: None,
        };
        let out = optimize_details(&key, &ctx, REAL_DETAILS, "all in one bilgisayar")
            .await
            .expect("optimize hatası")
            .expect("yapı desteklenmeliydi");
        println!("--- OPTİMİZE EDİLEN ---\n{out}\n----------------");
        // Görseller birebir korunmalı
        assert_eq!(extract_img_srcs(&out), extract_img_srcs(REAL_DETAILS));
        // Semantik yapı
        assert_eq!(out.matches("<section").count(), 2);
        assert!(out.contains(r#"<div class="container">"#));
        assert!(!out.contains(r#"<section class="container""#));
        // alt="image" gitmiş olmalı, anlamlı alt gelmeli
        assert!(!out.contains(r#"alt="image""#));
        assert!(out.contains("alt=\"Lenovo ThinkCentre"));
        // Metin optimize edilmiş olmalı
        assert!(!out.contains("Eski metin bir."));
    }

    /// Gerçek Gemini sıfırdan açıklama üretimi. GEMINI_API_KEY gerekir.
    /// `GEMINI_API_KEY=... cargo test gen_scratch_real -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn gen_scratch_real() {
        let key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY ayarlı değil");
        let ctx = ProductContext {
            name: "Lenovo ThinkCentre Neo 50a 27'' All In One",
            brand: Some("Lenovo"),
            category: Some("All In One Bilgisayar"),
            main_category: Some("Bilgisayar"),
            target_keyword: None,
            insights: None,
        };
        let images = vec![
            "https://cdn.example/1.png".to_string(),
            "https://cdn.example/2.png".to_string(),
            "https://cdn.example/3.png".to_string(),
        ];
        let out = generate_details_scratch(&key, &ctx, &images, "all in one bilgisayar")
            .await
            .expect("sıfırdan üretim başarısız");
        println!("--- SIFIRDAN ÜRETİLEN ---\n{out}\n----------------");
        // Semantik yapı: dış section, iç div, iç içe section YOK
        assert_eq!(extract_img_srcs(&out), images);
        assert!(out.contains("<div class=\"container\">"));
        assert!(!out.contains("<section class=\"container\""));
        assert!(out.matches("<section").count() == 3);
        assert!(crate::validation::word_count(&out) >= 50);
    }

    #[tokio::test]
    #[ignore]
    async fn test_key_real() {
        let key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY ayarlı değil");
        let msg = test_key(&key).await.expect("anahtar testi başarısız");
        println!("{msg}");
    }
}
