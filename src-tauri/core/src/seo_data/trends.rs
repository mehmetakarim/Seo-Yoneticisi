//! Google Trends — **hedef kelimeye ilgili** trend sorguları (Faz 6). **Keyless**.
//!
//! google-news-trends MCP'yi (Python + Playwright) paketlemek yerine Google Trends'in
//! `explore` → `widgetdata/relatedsearches` akışını doğrudan çağırıyoruz:
//! 1) `explore` widget token'ını verir, 2) `relatedsearches` o token'la hedef kelimeye ait
//! **top + yükselen (rising)** sorguları döndürür. Yanıtlar `)]}',` ön ekiyle gelir → soyulur.
//! **Kırılgan kaynak**: Google biçim değiştirir / 429 verirse boş döner (graceful degrade).
//!
//! Not: (Eski `dailytrends` JSON 404; `/trending/rss` yalnızca geo-geneli günlük trendleri verirdi —
//! hedef kelimeyle alakasız olduğu için terk edildi.)

use super::TrendTerm;

const EXPLORE_URL: &str = "https://trends.google.com/trends/api/explore";
const RELATED_URL: &str = "https://trends.google.com/trends/api/widgetdata/relatedsearches";
const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
                  (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

/// Hedef kelimeye ilgili trend sorguları (top + rising). geo boşsa dünya geneli.
pub async fn related_queries(
    client: &reqwest::Client,
    keyword: &str,
    geo: &str,
    hl: &str,
    limit: usize,
) -> Result<Vec<TrendTerm>, String> {
    // 0) Isınma: NID çerezini al (explore aksi halde 429 verir). Çerez deposu açık istemci gerekir.
    let _ = client
        .get("https://trends.google.com/trends/explore")
        .query(&[("geo", geo), ("hl", hl)])
        .header("User-Agent", UA)
        .send()
        .await;

    // 1) explore → RELATED_QUERIES widget'ının token + request'i
    let explore_req = serde_json::json!({
        "comparisonItem": [{ "keyword": keyword, "geo": geo, "time": "today 12-m" }],
        "category": 0,
        "property": ""
    })
    .to_string();
    let explore = get_trends_json(
        client,
        EXPLORE_URL,
        &[("hl", hl), ("tz", "-180"), ("req", &explore_req)],
    )
    .await?;

    let widget = explore
        .get("widgets")
        .and_then(|w| w.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|w| w.get("id").and_then(|i| i.as_str()) == Some("RELATED_QUERIES"))
        })
        .ok_or("Bu kelime için ilgili sorgu verisi yok.")?;
    let token = widget
        .get("token")
        .and_then(|t| t.as_str())
        .ok_or("Trends token alınamadı.")?;
    let widget_req = serde_json::to_string(widget.get("request").unwrap_or(&serde_json::Value::Null))
        .map_err(|e| format!("Trends isteği kurulamadı: {e}"))?;

    // 2) relatedsearches → sıralı sorgular
    let data = get_trends_json(
        client,
        RELATED_URL,
        &[("hl", hl), ("tz", "-180"), ("req", &widget_req), ("token", token)],
    )
    .await?;
    Ok(parse_related(&data, limit))
}

/// Trends uç noktasından `)]}',` ön ekli JSON'u çeker ve ayrıştırır.
async fn get_trends_json(
    client: &reqwest::Client,
    url: &str,
    params: &[(&str, &str)],
) -> Result<serde_json::Value, String> {
    let resp = client
        .get(url)
        .query(params)
        .header("User-Agent", UA)
        .header("Accept", "application/json, text/plain, */*")
        .send()
        .await
        .map_err(|e| format!("Google Trends'e ulaşılamadı: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Google Trends hatası (HTTP {}).", resp.status().as_u16()));
    }
    let text = resp.text().await.map_err(|e| format!("Trends yanıtı okunamadı: {e}"))?;
    let start = text.find('{').ok_or("Trends yanıtı boş.")?;
    serde_json::from_str(&text[start..]).map_err(|e| format!("Trends JSON çözümlenemedi: {e}"))
}

/// `default.rankedList[].rankedKeyword[]` → { query, value } (top + rising birleşik, tekilleştirilmiş).
fn parse_related(data: &serde_json::Value, limit: usize) -> Vec<TrendTerm> {
    let lists = match data
        .get("default")
        .and_then(|d| d.get("rankedList"))
        .and_then(|r| r.as_array())
    {
        Some(l) => l,
        None => return Vec::new(),
    };
    let mut out: Vec<TrendTerm> = Vec::new();
    for list in lists {
        if let Some(items) = list.get("rankedKeyword").and_then(|k| k.as_array()) {
            for it in items {
                let term = it.get("query").and_then(|q| q.as_str()).unwrap_or("").trim();
                if term.is_empty() || out.iter().any(|t| t.term.eq_ignore_ascii_case(term)) {
                    continue;
                }
                let volume = it.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                out.push(TrendTerm { term: term.to_string(), volume });
                if out.len() >= limit {
                    return out;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn parse_related_merges_top_and_rising_dedup() {
        let data = serde_json::json!({
            "default": { "rankedList": [
                { "rankedKeyword": [
                    { "query": "all in one lenovo", "value": 100 },
                    { "query": "all in one hp", "value": 80 }
                ]},
                { "rankedKeyword": [
                    { "query": "All In One Lenovo", "value": 250 },
                    { "query": "all in one dokunmatik", "value": 200 }
                ]}
            ]}
        });
        let terms = parse_related(&data, 10);
        // 3 tekil (Lenovo tekrarı case-insensitive elendi)
        assert_eq!(terms.len(), 3);
        assert_eq!(terms[0].term, "all in one lenovo");
        assert_eq!(terms[0].volume, 100);
        assert!(terms.iter().any(|t| t.term == "all in one dokunmatik"));
    }

    #[test]
    fn parse_related_respects_limit() {
        let data = serde_json::json!({
            "default": { "rankedList": [ { "rankedKeyword": [
                { "query": "a" }, { "query": "b" }, { "query": "c" }
            ]}]}
        });
        assert_eq!(parse_related(&data, 2).len(), 2);
    }

    #[test]
    fn parse_related_bad_shape_empty() {
        assert!(parse_related(&serde_json::json!({}), 5).is_empty());
        assert!(parse_related(&serde_json::json!({"default":{}}), 5).is_empty());
    }

    /// Gerçek Google Trends related queries (keyless).
    /// `cargo test related_real -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn related_real() {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .cookie_store(true)
            .build()
            .unwrap();
        let kw = std::env::var("TREND_KW").unwrap_or_else(|_| "all in one bilgisayar".to_string());
        match related_queries(&client, &kw, "TR", "tr", 12).await {
            Ok(t) => {
                println!("'{kw}' için {} ilgili sorgu:", t.len());
                for x in &t {
                    println!("  {} ({})", x.term, x.volume);
                }
            }
            Err(e) => println!("HATA: {e}"),
        }
    }
}
