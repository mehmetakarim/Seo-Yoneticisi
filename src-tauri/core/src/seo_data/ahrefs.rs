//! Ahrefs free-tools + CapSolver Turnstile — seo-research-mcp'nin Rust portu.
//!
//! Akış (her çağrı için): CapSolver ile ahrefs.com Turnstile captcha'sını çöz → token,
//! ardından Ahrefs `v4` free-tools uç noktasına POST. Anahtar (`capsolver_api_key`)
//! Ayarlar'dan gelir; koda gömülü değildir.
//!
//! Not: Bunlar gayriresmi uç noktalardır; Ahrefs biçim değiştirirse parse kırılabilir →
//! her fonksiyon hata durumunda anlaşılır bir mesaj döndürür, orkestratör graceful degrade eder.

use super::{unwrap_ok, DomainOverview, KeywordCand, KeywordDifficulty};
use std::time::Duration;

const CAPSOLVER_CREATE: &str = "https://api.capsolver.com/createTask";
const CAPSOLVER_RESULT: &str = "https://api.capsolver.com/getTaskResult";
/// ahrefs.com hedef sitesinin Turnstile site anahtarı (seo-research-mcp'den).
const AHREFS_SITE_KEY: &str = "0x4AAAAAAAAzi9ITzSN9xKMi";

const IDEAS_URL: &str = "https://ahrefs.com/v4/stGetFreeKeywordIdeas";
const KD_URL: &str = "https://ahrefs.com/v4/stGetFreeSerpOverviewForKeywordDifficultyChecker";
const BL_OVERVIEW_URL: &str = "https://ahrefs.com/v4/stGetFreeBacklinksOverview";

/// Zorluk etiketini 0-100 sayıya çevirir (seo-research-mcp `_map_difficulty_label_to_int`).
fn difficulty_to_int(v: &serde_json::Value) -> i64 {
    if let Some(n) = v.as_i64() {
        return n;
    }
    if let Some(f) = v.as_f64() {
        return f as i64;
    }
    let s = v.as_str().unwrap_or("").to_lowercase();
    match s.trim() {
        "very easy" => 5,
        "easy" => 15,
        "medium" => 40,
        "hard" => 70,
        "very hard" => 85,
        "super hard" => 90,
        "unknown" | "" => 0,
        other => coerce_int(other),
    }
}

/// Hacim etiketini sayıya (alt sınır) çevirir. Ahrefs free-tool artık isimli kova enum'ları
/// döndürüyor ("MoreThanOneThousand" vb.); "100-1K" aralıkları ve düz sayılar da desteklenir.
fn volume_to_int(v: &serde_json::Value) -> i64 {
    if let Some(n) = v.as_i64() {
        return n;
    }
    if let Some(f) = v.as_f64() {
        return f as i64;
    }
    let s = v.as_str().unwrap_or("").trim().to_string();
    if s.is_empty() {
        return 0;
    }
    // İsimli kovalar (büyükten küçüğe kontrol — "tenthousand" önce)
    let low = s.to_lowercase().replace([' ', '_'], "");
    if low.contains("tenthousand") {
        return 10_000;
    }
    if low.contains("thousand") {
        return 1_000;
    }
    if low.contains("hundred") {
        return 100;
    }
    if low.contains("ten") {
        return 10;
    }
    if low.contains("zero") || low == "none" {
        return 0;
    }
    // "100-1K" aralık → alt sınır
    if let Some((lo, _hi)) = s.split_once('-') {
        return coerce_int(lo);
    }
    coerce_int(&s)
}

/// "1,000" / "1.5k" / "2M" gibi metinleri sayıya çevirir (seo-research-mcp `_coerce_int`).
fn coerce_int(s: &str) -> i64 {
    let cleaned = s.replace(',', "").to_lowercase();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        return 0;
    }
    let (num_part, mult) = if let Some(stripped) = cleaned.strip_suffix('k') {
        (stripped, 1_000.0)
    } else if let Some(stripped) = cleaned.strip_suffix('m') {
        (stripped, 1_000_000.0)
    } else {
        (cleaned, 1.0)
    };
    num_part.parse::<f64>().map(|f| (f * mult) as i64).unwrap_or(0)
}

/// CapSolver ile Turnstile token'ı çözer. Poll aralığı 1s, üst sınır 60s.
async fn capsolver_token(
    client: &reqwest::Client,
    api_key: &str,
    site_url: &str,
) -> Result<String, String> {
    let payload = serde_json::json!({
        "clientKey": api_key,
        "task": {
            "type": "AntiTurnstileTaskProxyLess",
            "websiteKey": AHREFS_SITE_KEY,
            "websiteURL": site_url,
            "metadata": { "action": "" }
        }
    });
    let create: serde_json::Value = client
        .post(CAPSOLVER_CREATE)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("CapSolver'a ulaşılamadı: {e}"))?
        .json()
        .await
        .map_err(|e| format!("CapSolver yanıtı okunamadı: {e}"))?;

    if create.get("errorId").and_then(|v| v.as_i64()).unwrap_or(0) != 0 {
        let desc = create
            .get("errorDescription")
            .and_then(|v| v.as_str())
            .unwrap_or("bilinmeyen hata");
        return Err(format!("CapSolver görevi reddedildi: {desc}"));
    }
    let task_id = create
        .get("taskId")
        .and_then(|v| v.as_str())
        .ok_or("CapSolver taskId döndürmedi (anahtar/kota?)")?
        .to_string();

    for _ in 0..60 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let poll: serde_json::Value = client
            .post(CAPSOLVER_RESULT)
            .json(&serde_json::json!({ "clientKey": api_key, "taskId": task_id }))
            .send()
            .await
            .map_err(|e| format!("CapSolver sonucu alınamadı: {e}"))?
            .json()
            .await
            .map_err(|e| format!("CapSolver sonucu okunamadı: {e}"))?;

        if poll.get("errorId").and_then(|v| v.as_i64()).unwrap_or(0) != 0 {
            return Err("CapSolver çözüm sırasında hata döndürdü.".to_string());
        }
        match poll.get("status").and_then(|v| v.as_str()) {
            Some("ready") => {
                return poll
                    .get("solution")
                    .and_then(|s| s.get("token"))
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .ok_or_else(|| "CapSolver token döndürmedi.".to_string());
            }
            Some("failed") => return Err("CapSolver captcha çözümü başarısız.".to_string()),
            _ => continue, // "processing" / "idle"
        }
    }
    Err("CapSolver zaman aşımı (60 sn içinde çözülemedi).".to_string())
}

/// Ahrefs v4 uç noktasına tarayıcı benzeri başlıklarla POST atar (400/anti-bot'tan kaçınmak için).
async fn ahrefs_post(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
    referer: &str,
) -> Result<reqwest::Response, String> {
    client
        .post(url)
        .header("Content-Type", "application/json; charset=utf-8")
        .header("Accept", "*/*")
        .header("Origin", "https://ahrefs.com")
        .header("Referer", referer)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
        )
        .json(body)
        .send()
        .await
        .map_err(|e| format!("Ahrefs'e ulaşılamadı: {e}"))
}

fn short_body(s: &str) -> String {
    let t = s.trim();
    if t.len() > 200 {
        format!("{}…", &t[..200])
    } else {
        t.to_string()
    }
}

/// ahrefs.com free-tool sayfa URL'sini kurar (CapSolver websiteURL'i için).
fn tool_url(path: &str, country: &str, keyword: &str) -> String {
    let mut u = reqwest::Url::parse(&format!("https://ahrefs.com/{path}/")).unwrap();
    u.query_pairs_mut()
        .append_pair("country", country)
        .append_pair("input", keyword);
    u.to_string()
}

/// Ahrefs keyword-generator: tohum kelimeden anahtar kelime fikirleri (+ zorluk/hacim).
pub async fn keyword_ideas(
    client: &reqwest::Client,
    api_key: &str,
    keyword: &str,
    country: &str,
) -> Result<Vec<KeywordCand>, String> {
    let site_url = tool_url("keyword-generator", country, keyword);
    let token = capsolver_token(client, api_key, &site_url).await?;
    // Not: Ahrefs `keyword`'ü artık düz string bekliyor (eski `["Some", kw]` sarmalı InvalidInput verir).
    let body = serde_json::json!({
        "withQuestionIdeas": true,
        "captcha": token,
        "searchEngine": "Google",
        "country": country,
        "keyword": keyword
    });
    let resp = ahrefs_post(client, IDEAS_URL, &body, &site_url).await?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "Ahrefs keyword-generator hatası (HTTP {}): {}",
            status.as_u16(),
            short_body(&text)
        ));
    }
    let data: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Ahrefs yanıtı okunamadı: {e}"))?;
    Ok(parse_ideas(&data))
}

/// `["Ok", { allIdeas:{results:[...]}, questionIdeas:{results:[...]} }]` → adaylar.
fn parse_ideas(data: &serde_json::Value) -> Vec<KeywordCand> {
    let inner = match unwrap_ok(data) {
        Some(v) => v,
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    for (group, kind) in [("allIdeas", "idea"), ("questionIdeas", "question")] {
        if let Some(results) = inner.get(group).and_then(|g| g.get("results")).and_then(|r| r.as_array())
        {
            for idea in results {
                let keyword = idea.get("keyword").and_then(|v| v.as_str()).unwrap_or("").trim();
                if keyword.is_empty() {
                    continue;
                }
                out.push(KeywordCand {
                    keyword: keyword.to_string(),
                    difficulty: difficulty_to_int(idea.get("difficultyLabel").unwrap_or(&serde_json::Value::Null)),
                    volume: volume_to_int(idea.get("volumeLabel").unwrap_or(&serde_json::Value::Null)),
                    kind: kind.to_string(),
                });
            }
        }
    }
    out
}

/// Ahrefs keyword-difficulty: bir kelimenin zorluk özeti.
pub async fn keyword_difficulty(
    client: &reqwest::Client,
    api_key: &str,
    keyword: &str,
    country: &str,
) -> Result<KeywordDifficulty, String> {
    let site_url = tool_url("keyword-difficulty", country, keyword);
    let token = capsolver_token(client, api_key, &site_url).await?;
    let body = serde_json::json!({
        "captcha": token,
        "country": country,
        "keyword": keyword
    });
    let resp = ahrefs_post(client, KD_URL, &body, &site_url).await?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "Ahrefs keyword-difficulty hatası (HTTP {}): {}",
            status.as_u16(),
            short_body(&text)
        ));
    }
    let data: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Ahrefs yanıtı okunamadı: {e}"))?;
    parse_difficulty(&data, keyword).ok_or_else(|| "Ahrefs zorluk verisi çözümlenemedi.".to_string())
}

/// ⚠️ **Ahrefs bu uçtan `difficulty` diye bir alan DÖNDÜRMÜYOR** (2026-07-29 ham yanıt
/// dökümüyle doğrulandı). Dönen şey bir SERP özeti: `{"serp":{"results":[...]}}` — kimlerin
/// sıralandığı, alan otoriteleri (DR), yönlendiren alan sayıları.
///
/// Eski hâli `unwrap_or(0)` ile eksik alanları 0'a düşürüyordu; sonuç **sessizce "zorluk 0"**
/// oluyordu ve 0, "çok kolay" demek. Yani veri yokluğu, en iyimser değer gibi görünüyordu.
/// Bu hem bu fonksiyonu hem de onu kullanan "SEO Araştır" panelini yanıltıyordu.
///
/// Artık: veri yoksa `None`. Zorluk, SERP'teki ilk sonuçların **yönlendiren alan sayılarının
/// medyanı** üzerinden türetiliyor — Ahrefs'in kendi formülü değil, yaygın bir yaklaşıklama.
/// Bu yüzden arayüzde "Ahrefs zorluğu" değil, ne olduğu doğru adlandırılmalı.
fn parse_difficulty(data: &serde_json::Value, keyword: &str) -> Option<KeywordDifficulty> {
    let inner = unwrap_ok(data)?;
    // Uzun kuyruk sorgularda Ahrefs `["Ok", null]` dönüyor — veri yok demek.
    if inner.is_null() {
        return None;
    }

    let results = inner.get("serp")?.get("results")?.as_array()?;
    // Her organik sonucun yönlendiren alan (referring domains) sayısı
    let mut domains: Vec<i64> = results
        .iter()
        .filter_map(|r| {
            let link = r.get("content")?.get(1)?.get("link")?.get(1)?;
            link.get("metrics")?.get("domains")?.as_i64()
        })
        .collect();
    if domains.is_empty() {
        return None;
    }
    domains.sort_unstable();
    let median = domains[domains.len() / 2];

    // Medyan yönlendiren alan → 0-100 ölçeği. Logaritmik: 1 alan ≈ 0, 10 ≈ 33, 100 ≈ 66,
    // 1000 ≈ 100. Kaba ama sıralama için yeterli; mutlak doğruluk iddiası yok.
    let difficulty = ((median.max(1) as f64).log10() / 3.0 * 100.0).round().clamp(0.0, 100.0) as i64;

    Some(KeywordDifficulty {
        keyword: keyword.to_string(),
        difficulty,
        // `shortage` bu uçta gelmiyor; medyan alan sayısını taşıyoruz (arayüzde açıklanır).
        shortage: median,
        last_update: String::new(),
    })
}

/// Alan (domain) geneli backlink özeti: Domain Rating, backlink & ref-domain sayısı.
/// Yanıt: `["Ok", { data: { domainRating, backlinks, refdomains, ... }, signedInput: {...} }]`.
pub async fn backlinks_overview(
    client: &reqwest::Client,
    api_key: &str,
    domain: &str,
) -> Result<DomainOverview, String> {
    let site_url = format!("https://ahrefs.com/backlink-checker/?input={domain}&mode=subdomains");
    let token = capsolver_token(client, api_key, &site_url).await?;
    let body = serde_json::json!({ "captcha": token, "mode": "subdomains", "url": domain });
    let resp = ahrefs_post(client, BL_OVERVIEW_URL, &body, &site_url).await?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("Ahrefs backlink özeti hatası (HTTP {}): {}", status.as_u16(), short_body(&text)));
    }
    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Ahrefs yanıtı okunamadı: {e}"))?;
    parse_overview(&json, domain).ok_or_else(|| "Ahrefs backlink verisi çözümlenemedi.".to_string())
}

fn parse_overview(json: &serde_json::Value, domain: &str) -> Option<DomainOverview> {
    let data = unwrap_ok(json)?.get("data")?;
    Some(DomainOverview {
        domain: domain.to_string(),
        domain_rating: data.get("domainRating").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64,
        backlinks: data.get("backlinks").and_then(|v| v.as_i64()).unwrap_or(0),
        ref_domains: data.get("refdomains").and_then(|v| v.as_i64()).unwrap_or(0),
    })
}

/// CapSolver anahtarını hafifçe doğrular (getBalance) — üretim tüketmez.
pub async fn test_key(api_key: &str) -> Result<String, String> {
    let key = api_key.trim();
    if key.len() < 20 {
        return Err("CapSolver anahtarı çok kısa görünüyor.".to_string());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;
    let resp: serde_json::Value = client
        .post("https://api.capsolver.com/getBalance")
        .json(&serde_json::json!({ "clientKey": key }))
        .send()
        .await
        .map_err(|e| format!("CapSolver'a ulaşılamadı: {e}"))?
        .json()
        .await
        .map_err(|e| format!("CapSolver yanıtı okunamadı: {e}"))?;
    if resp.get("errorId").and_then(|v| v.as_i64()).unwrap_or(0) != 0 {
        return Err("CapSolver anahtarı reddedildi (geçersiz).".to_string());
    }
    match resp.get("balance").and_then(|v| v.as_f64()) {
        Some(b) => Ok(format!("CapSolver anahtarı geçerli · bakiye ${b:.2}")),
        None => Ok("CapSolver anahtarı geçerli.".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulty_label_mapping() {
        assert_eq!(difficulty_to_int(&serde_json::json!("Easy")), 15);
        assert_eq!(difficulty_to_int(&serde_json::json!("super hard")), 90);
        assert_eq!(difficulty_to_int(&serde_json::json!("Unknown")), 0);
        assert_eq!(difficulty_to_int(&serde_json::json!(42)), 42);
        assert_eq!(difficulty_to_int(&serde_json::Value::Null), 0);
    }

    #[test]
    fn volume_label_mapping() {
        // İsimli kovalar (Ahrefs free-tool güncel biçim)
        assert_eq!(volume_to_int(&serde_json::json!("MoreThanTenThousand")), 10_000);
        assert_eq!(volume_to_int(&serde_json::json!("MoreThanOneThousand")), 1_000);
        assert_eq!(volume_to_int(&serde_json::json!("MoreThanOneHundred")), 100);
        assert_eq!(volume_to_int(&serde_json::json!("MoreThanTen")), 10);
        assert_eq!(volume_to_int(&serde_json::json!("Zero")), 0);
        // Eski aralık/sayı biçimleri hâlâ desteklenir
        assert_eq!(volume_to_int(&serde_json::json!("100-1K")), 100);
        assert_eq!(volume_to_int(&serde_json::json!("1K")), 1000);
        assert_eq!(volume_to_int(&serde_json::json!("2M")), 2_000_000);
        assert_eq!(volume_to_int(&serde_json::json!("1,500")), 1500);
        assert_eq!(volume_to_int(&serde_json::json!(0)), 0);
    }

    #[test]
    fn parse_ideas_extracts_both_groups() {
        let data = serde_json::json!(["Ok", {
            "allIdeas": { "results": [
                { "keyword": "all in one pc", "difficultyLabel": "Easy", "volumeLabel": "1K-10K" },
                { "keyword": "", "difficultyLabel": "Hard", "volumeLabel": "100" }
            ]},
            "questionIdeas": { "results": [
                { "keyword": "all in one nedir", "difficultyLabel": "Medium", "volumeLabel": "100-1K" }
            ]}
        }]);
        let cands = parse_ideas(&data);
        assert_eq!(cands.len(), 2); // boş keyword atlandı
        assert_eq!(cands[0].keyword, "all in one pc");
        assert_eq!(cands[0].difficulty, 15);
        assert_eq!(cands[0].volume, 1000);
        assert_eq!(cands[0].kind, "idea");
        assert_eq!(cands[1].keyword, "all in one nedir");
        assert_eq!(cands[1].kind, "question");
        assert_eq!(cands[1].volume, 100);
    }

    #[test]
    fn parse_ideas_bad_shape_is_empty() {
        assert!(parse_ideas(&serde_json::json!(["Error", "x"])).is_empty());
        assert!(parse_ideas(&serde_json::json!({})).is_empty());
    }

    #[test]
    fn parse_overview_reads_domain_stats() {
        let data = serde_json::json!(["Ok", {
            "data": { "domainRating": 30.0, "backlinks": 2332, "refdomains": 672 },
            "signedInput": {}
        }]);
        let o = parse_overview(&data, "kurumsalit.com").unwrap();
        assert_eq!(o.domain_rating, 30);
        assert_eq!(o.backlinks, 2332);
        assert_eq!(o.ref_domains, 672);
        assert_eq!(o.domain, "kurumsalit.com");
        assert!(parse_overview(&serde_json::json!(["Error"]), "x").is_none());
    }

    #[test]
    /// ⚠️ Bu testin eski hâli `{"difficulty":34,"shortage":2,...}` biçimini varsayıyordu —
    /// **gerçek API bunu hiç döndürmüyor.** Hayali bir yanıta karşı yazıldığı için, kod
    /// üretimde sessizce 0 döndürürken bile yeşil yanıyordu. Aşağıdaki gövde 2026-07-29'da
    /// alınan GERÇEK yanıttan türetildi.
    fn parse_difficulty_reads_fields() {
        let data = serde_json::json!(["Ok", { "serp": { "results": [
            { "content": ["organic", { "link": ["Some", { "metrics": { "domains": 8 } }] }] },
            { "content": ["organic", { "link": ["Some", { "metrics": { "domains": 12 } }] }] },
            { "content": ["organic", { "link": ["Some", { "metrics": { "domains": 30 } }] }] }
        ] } }]);
        let kd = parse_difficulty(&data, "all in one bilgisayar").unwrap();
        assert_eq!(kd.keyword, "all in one bilgisayar");
        assert_eq!(kd.shortage, 12, "medyan yönlendiren alan");
        assert!(kd.difficulty > 0, "veri varken zorluk 0 olmamalı");
        assert!(parse_difficulty(&serde_json::json!(["Error"]), "x").is_none());
    }

    #[test]
    fn tool_url_encodes_keyword() {
        let u = tool_url("keyword-generator", "tr", "all in one bilgisayar");
        assert!(u.starts_with("https://ahrefs.com/keyword-generator/?"));
        assert!(u.contains("country=tr"));
        assert!(u.contains("input=all+in+one+bilgisayar") || u.contains("input=all%20in%20one%20bilgisayar"));
    }

    /// Gerçek CapSolver + Ahrefs. CAPSOLVER_API_KEY ortam değişkeni gerekir (koda gömülmez).
    /// `CAPSOLVER_API_KEY=... cargo test keyword_ideas_real -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn keyword_ideas_real() {
        let key = std::env::var("CAPSOLVER_API_KEY").expect("CAPSOLVER_API_KEY ayarlı değil");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap();
        let cands = keyword_ideas(&client, &key, "all in one bilgisayar", "tr")
            .await
            .expect("keyword_ideas başarısız");
        println!("{} aday bulundu:", cands.len());
        for c in cands.iter().take(15) {
            println!("  {} — zorluk {}, hacim {} [{}]", c.keyword, c.difficulty, c.volume, c.kind);
        }
        assert!(!cands.is_empty(), "hiç aday dönmedi");
    }

    /// Tanılama: ham Ahrefs keyword-generator yanıtını basar (alan adlarını doğrulamak için).
    /// `CAPSOLVER_API_KEY=... cargo test ideas_raw_dump -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn ideas_raw_dump() {
        let key = std::env::var("CAPSOLVER_API_KEY").expect("CAPSOLVER_API_KEY ayarlı değil");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap();
        let site_url = tool_url("keyword-generator", "tr", "all in one bilgisayar");
        let token = capsolver_token(&client, &key, &site_url).await.expect("token");
        let body = serde_json::json!({
            "withQuestionIdeas": true, "captcha": token, "searchEngine": "Google",
            "country": "tr", "keyword": "all in one bilgisayar"
        });
        let resp = ahrefs_post(&client, IDEAS_URL, &body, &site_url).await.unwrap();
        let data: serde_json::Value = resp.json().await.unwrap();
        if let Some(inner) = unwrap_ok(&data) {
            if let Some(first) = inner.get("allIdeas").and_then(|g| g.get("results")).and_then(|r| r.as_array()).and_then(|a| a.first()) {
                println!("İLK allIdeas KAYDI:\n{}", serde_json::to_string_pretty(first).unwrap());
            }
        } else {
            println!("Beklenmeyen biçim:\n{}", serde_json::to_string_pretty(&data).unwrap());
        }
    }

    /// Gerçek CapSolver + Ahrefs backlink özeti. CAPSOLVER_API_KEY gerekir.
    /// `CAPSOLVER_API_KEY=... cargo test backlinks_overview_real -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn backlinks_overview_real() {
        let key = std::env::var("CAPSOLVER_API_KEY").expect("CAPSOLVER_API_KEY ayarlı değil");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap();
        let d = backlinks_overview(&client, &key, "kurumsalit.com")
            .await
            .expect("backlinks_overview başarısız");
        println!(
            "domain {} · DR {} · {} backlink · {} ref-domain",
            d.domain, d.domain_rating, d.backlinks, d.ref_domains
        );
        assert_eq!(d.domain, "kurumsalit.com");
    }

    /// Gerçek CapSolver + Ahrefs zorluk. CAPSOLVER_API_KEY gerekir.
    /// `CAPSOLVER_API_KEY=... cargo test keyword_difficulty_real -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn keyword_difficulty_real() {
        let key = std::env::var("CAPSOLVER_API_KEY").expect("CAPSOLVER_API_KEY ayarlı değil");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap();
        let kd = keyword_difficulty(&client, &key, "all in one bilgisayar", "tr")
            .await
            .expect("keyword_difficulty başarısız");
        println!("zorluk: {} · shortage: {} · {}", kd.difficulty, kd.shortage, kd.last_update);
        assert_eq!(kd.keyword, "all in one bilgisayar");
    }
    /// Zorluk verisi canlı test — striking distance akışının "itmeye değer mi?" katmanı.
    /// ⚠️ Her çalıştırma bir CapSolver çözümü tüketir (ücretli). Tek sorguyla sınırlı tutuldu.
    /// `CAPSOLVER_API_KEY=... cargo test kd_real -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn kd_real() {
        let key = std::env::var("CAPSOLVER_API_KEY").expect("CAPSOLVER_API_KEY ayarlı değil");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap();
        // Gerçek striking-distance sorgusu: 6.2. sıra, 214 gösterim, 0 tıklama
        match keyword_difficulty(&client, &key, "çift monitör kolu", "tr").await {
            Ok(kd) => println!(
                "ZORLUK\tkeyword={}\tdifficulty={}\tshortage={}\tlast_update={}",
                kd.keyword, kd.difficulty, kd.shortage, kd.last_update
            ),
            Err(e) => panic!("keyword_difficulty hatası: {e}"),
        }
    }

    /// KD ham yanıt dökümü — biçim değişikliğini teşhis etmek için.
    /// `CAPSOLVER_API_KEY=... cargo test kd_raw_dump -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn kd_raw_dump() {
        let key = std::env::var("CAPSOLVER_API_KEY").expect("CAPSOLVER_API_KEY ayarlı değil");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap();
        let kw = std::env::var("KD_KW").unwrap_or_else(|_| "monitör".to_string());
        let kw = kw.as_str();
        let site_url = tool_url("keyword-difficulty", "tr", kw);
        let token = match capsolver_token(&client, &key, &site_url).await {
            Ok(t) => {
                println!("TOKEN_OK len={}", t.len());
                t
            }
            Err(e) => panic!("CapSolver token alınamadı: {e}"),
        };
        let body = serde_json::json!({ "captcha": token, "country": "tr", "keyword": kw });
        let resp = ahrefs_post(&client, KD_URL, &body, &site_url).await.expect("istek");
        println!("HTTP={}", resp.status().as_u16());
        let text = resp.text().await.unwrap_or_default();
        println!("HAM_YANIT_BASI:\n{}", &text.chars().take(1200).collect::<String>());
    }

    /// Regresyon: veri yokluğu "zorluk 0" (yani "çok kolay") gibi görünmemeli.
    /// Ahrefs uzun kuyruk sorgularda `["Ok", null]` dönüyor (2026-07-29 doğrulandı).
    #[test]
    fn missing_data_is_none_not_zero() {
        let v: serde_json::Value = serde_json::from_str(r#"["Ok",null]"#).unwrap();
        assert!(
            parse_difficulty(&v, "x").is_none(),
            "veri yoksa None dönmeli — 0 dönmek 'çok kolay' anlamına gelir ve yanıltır"
        );
    }

    #[test]
    fn difficulty_derives_from_median_referring_domains() {
        let mk = |d: i64| {
            serde_json::json!({
                "content": ["organic", { "link": ["Some", { "metrics": { "domains": d } }] }]
            })
        };
        let v = serde_json::json!(["Ok", { "serp": { "results": [mk(5), mk(10), mk(20)] } }]);
        let kd = parse_difficulty(&v, "test").expect("veri var");
        assert_eq!(kd.shortage, 10, "medyan yönlendiren alan sayısı taşınmalı");
        assert!(kd.difficulty > 25 && kd.difficulty < 45, "medyan 10 → ~33 bekleniyor: {}", kd.difficulty);

        // Daha rekabetçi SERP daha yüksek zorluk vermeli
        let v2 = serde_json::json!(["Ok", { "serp": { "results": [mk(400), mk(500), mk(900)] } }]);
        let kd2 = parse_difficulty(&v2, "test").expect("veri var");
        assert!(kd2.difficulty > kd.difficulty);
    }

    #[test]
    fn empty_serp_results_is_none() {
        let v = serde_json::json!(["Ok", { "serp": { "results": [] } }]);
        assert!(parse_difficulty(&v, "x").is_none());
    }

    /// **Kapsam ölçümü:** gerçek striking-distance sorgularının kaçında Ahrefs verisi var?
    /// Uzun kuyruk sorgularda `["Ok", null]` geldiğini biliyoruz; oran neyse özelliğin
    /// değerini o belirliyor. ⚠️ Her sorgu bir CapSolver çözümü tüketir.
    /// `CAPSOLVER_API_KEY=... cargo test kd_coverage -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn kd_coverage() {
        let key = std::env::var("CAPSOLVER_API_KEY").expect("CAPSOLVER_API_KEY ayarlı değil");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap();
        // Fırsatlar ekranındaki gerçek sorgular (2026-07-29 ölçümünden)
        let queries = [
            "21sr006rtx",
            "creality raptor pro",
            "logitech kablosuz",
            "bambu lab a2 combo",
            "ergotron",
        ];
        let mut with_data = 0;
        for q in queries {
            match keyword_difficulty(&client, &key, q, "tr").await {
                Ok(kd) => {
                    with_data += 1;
                    println!("VERI_VAR\t{}\tzorluk={}\tmedyan_alan={}", q, kd.difficulty, kd.shortage);
                }
                Err(e) => println!("VERI_YOK\t{}\t{}", q, e),
            }
        }
        println!("KAPSAM\t{}/{}", with_data, queries.len());
    }

}
