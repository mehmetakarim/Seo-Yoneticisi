//! Uzun açıklama (details) üretimi ve optimizasyonu.
//!
//! Üç yol var: içerik yoksa sıfırdan semantik HTML, düzenli yapı varsa OPTIMIZE,
//! düzensizse yapıyı koruyarak yalnızca metni yeniden yaz.

use super::*;

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
        return Err(classify_error(status.as_u16(), &text, model));
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

// ---- Hedef kelime yoğunluğu kontrolü (details_badge ile uyumlu 1.5–3.5) ----
const DENS_LO: f64 = 1.5;

const DENS_HI: f64 = 3.5;

const DENS_TARGET: f64 = 2.5;

/// Yoğunluk aralık dışıysa mevcut değeri (%) döner; içinde/anahtar boşsa None.
fn density_out_of_range(html: &str, keyword: &str) -> Option<f64> {
    if keyword.trim().is_empty() {
        return None;
    }
    let d = crate::validation::keyword_density(html, keyword);
    if !(DENS_LO..=DENS_HI).contains(&d) {
        Some(d)
    } else {
        None
    }
}

/// Retry için modele verilecek yoğunluk düzeltme talimatı.
fn density_correction(current: f64, keyword: &str) -> String {
    if current > DENS_HI {
        format!(
            "Hedef kelime yoğunluğu şu an %{current:.1} — ÇOK YÜKSEK. '{keyword}' ifadesini metinde \
             daha AZ tekrar et (yerine eş anlamlı, zamir veya 'bu ürün/cihaz' gibi ifadeler kullan). \
             Anlamı bozmadan yoğunluğu %2-3 aralığına indir."
        )
    } else {
        format!(
            "Hedef kelime yoğunluğu şu an %{current:.1} — ÇOK DÜŞÜK. '{keyword}' ifadesini metne birkaç \
             kez daha DOĞAL biçimde ekle. Yoğunluğu %2-3 aralığına çıkar."
        )
    }
}

/// Hedefe (%2.5) uzaklık — iki denemenin daha iyisini seçmek için.
fn density_dist(html: &str, keyword: &str) -> f64 {
    (crate::validation::keyword_density(html, keyword) - DENS_TARGET).abs()
}

/// Details HTML'ini yapıyı koruyarak yeniden üretir.
pub async fn generate_details(
    api_key: &str,
    ctx: &ProductContext<'_>,
    details_html: &str,
    target_keyword: &str,
) -> Result<Produced<String>, String> {
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
                let build = |arr: &[String]| -> String {
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
                    splice(details_html, &segs, &reps)
                };

                let mut result = build(&arr);

                // Img invariant: yeni HTML'deki src'ler orijinalle aynı olmalı.
                if extract_img_srcs(&result) != original_imgs {
                    // görsel güvenliği: orijinali koru
                    return Ok(Produced { value: details_html.to_string(), model });
                }

                // Yoğunluk aralık dışıysa tek retry; daha iyi (hedefe yakın) olanı seç.
                if let Some(d) = density_out_of_range(&result, target_keyword) {
                    let corr =
                        format!("{prompt}\n\nÖNEMLİ DÜZELTME: {}", density_correction(d, target_keyword));
                    if let Ok(arr2) = call_details_model(&client, key, model, &corr, segs.len()).await {
                        if arr2.len() == segs.len() {
                            let result2 = build(&arr2);
                            if extract_img_srcs(&result2) == original_imgs
                                && density_dist(&result2, target_keyword)
                                    < density_dist(&result, target_keyword)
                            {
                                result = result2;
                            }
                        }
                    }
                }
                return Ok(Produced { value: result, model });
            }
            Err((try_next, msg)) => {
                last_err = msg;
                if !try_next {
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
        return Err(classify_error(status.as_u16(), &text, model));
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
) -> Result<Produced<String>, String> {
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
                let mut result = assemble_scratch(ctx.name, &sections, images);
                // Yoğunluk aralık dışıysa tek retry; hedefe yakın olanı seç.
                if let Some(d) = density_out_of_range(&result, target_keyword) {
                    let corr =
                        format!("{prompt}\n\nÖNEMLİ DÜZELTME: {}", density_correction(d, target_keyword));
                    if let Ok(s2) = call_scratch_model(&client, key, model, &corr).await {
                        if !s2.is_empty() {
                            let result2 = assemble_scratch(ctx.name, &s2, images);
                            if density_dist(&result2, target_keyword)
                                < density_dist(&result, target_keyword)
                            {
                                result = result2;
                            }
                        }
                    }
                }
                return Ok(Produced { value: result, model });
            }
            Err((try_next, msg)) => {
                last_err = msg;
                if !try_next {
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
) -> Result<Produced<Option<String>>, String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("Gemini API anahtarı ayarlı değil. Ayarlar'dan ekleyin.".to_string());
    }
    let blocks = match extract_blocks(details_html) {
        Some(b) if !b.is_empty() => b,
        _ => return Ok(Produced { value: None, model: "" }),
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
                let orig_imgs = extract_img_srcs(details_html);
                let mut result = assemble_optimized(ctx.name, &blocks, &texts);
                // Görsel değişmezliği: yeni HTML orijinaldeki tüm src'leri aynı sırada içermeli.
                if extract_img_srcs(&result) != orig_imgs {
                    // güvenli tarafta kal → eski yol
                    return Ok(Produced { value: None, model });
                }
                // Yoğunluk aralık dışıysa tek retry; hedefe yakın olanı seç.
                if let Some(d) = density_out_of_range(&result, target_keyword) {
                    let corr =
                        format!("{prompt}\n\nÖNEMLİ DÜZELTME: {}", density_correction(d, target_keyword));
                    if let Ok(t2) = call_scratch_model(&client, key, model, &corr).await {
                        if t2.len() == blocks.len() {
                            let result2 = assemble_optimized(ctx.name, &blocks, &t2);
                            if extract_img_srcs(&result2) == orig_imgs
                                && density_dist(&result2, target_keyword)
                                    < density_dist(&result, target_keyword)
                            {
                                result = result2;
                            }
                        }
                    }
                }
                return Ok(Produced { value: Some(result), model });
            }
            Err((try_next, msg)) => {
                last_err = msg;
                if !try_next {
                    return Err(last_err);
                }
            }
        }
    }
    Err(format!("Tüm modeller denendi, açıklama optimize edilemedi. Son hata: {last_err}"))
}

// ================= Faz 8: Teknik Özellik Tablosu (halüsinasyon-sıfır) =================
//
// IdeaSoft'ta teknik tablo ayrı alandadır ve feed'e girmez → uygulama sıfırdan üretir. Yanlış spec
// iade/mevzuat riski taşıdığı için model YALNIZCA yapılandırır (ham metin → gruplu anahtar-değer),
// HTML'i kod montajlar ve her değer **kaynak metne karşı doğrulanır** (aşağıdaki sayı kuralı).

#[cfg(test)]
mod tests {
    //! Açıklama üretimi/optimizasyonu testleri.
    use super::*;

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
    fn density_range_and_correction() {
        fn text(total: usize, kw: usize, k: &str) -> String {
            let mut v: Vec<String> = vec!["dolgu".into(); total - kw];
            for _ in 0..kw {
                v.push(k.into());
            }
            format!("<p>{}</p>", v.join(" "))
        }
        // %2 → aralıkta (None)
        assert!(density_out_of_range(&text(100, 2, "urun"), "urun").is_none());
        // %10 → yüksek
        let hi = density_out_of_range(&text(10, 1, "urun"), "urun").unwrap();
        assert!(hi > 3.5);
        // %1 → düşük
        let lo = density_out_of_range(&text(100, 1, "urun"), "urun").unwrap();
        assert!(lo < 1.5);
        // anahtar boş → kontrol yok
        assert!(density_out_of_range(&text(100, 5, "urun"), "").is_none());
        // düzeltme yönü
        assert!(density_correction(9.0, "urun").contains("YÜKSEK"));
        assert!(density_correction(1.0, "urun").contains("DÜŞÜK"));
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
        let produced = generate_details(&key, &ctx, SAMPLE_DETAILS, "all in one bilgisayar")
            .await
            .expect("üretim başarısız");
        println!("model: {}", produced.model);
        let out = produced.value;
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
        let produced = optimize_details(&key, &ctx, REAL_DETAILS, "all in one bilgisayar")
            .await
            .expect("optimize hatası");
        println!("model: {}", produced.model);
        let out = produced.value.expect("yapı desteklenmeliydi");
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
        let produced = generate_details_scratch(&key, &ctx, &images, "all in one bilgisayar")
            .await
            .expect("sıfırdan üretim başarısız");
        println!("model: {}", produced.model);
        let out = produced.value;
        println!("--- SIFIRDAN ÜRETİLEN ---\n{out}\n----------------");
        // Semantik yapı: dış section, iç div, iç içe section YOK
        assert_eq!(extract_img_srcs(&out), images);
        assert!(out.contains("<div class=\"container\">"));
        assert!(!out.contains("<section class=\"container\""));
        assert!(out.matches("<section").count() == 3);
        assert!(crate::validation::word_count(&out) >= 50);
    }
}
