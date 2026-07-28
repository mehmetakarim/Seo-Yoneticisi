//! Google Search Console — service-account ile gerçek arama sorguları (Faz 5).
//!
//! gsc-mcp'yi paketlemek yerine GSC REST'i doğrudan çağırıyoruz. Kimlik: **service-account**
//! (tarayıcı OAuth yok → bundling temiz). SA JSON'dan RS256 imzalı bir JWT üretilir
//! (`jsonwebtoken`), Google token endpoint'inden access token alınır, `searchAnalytics.query`
//! ürün sayfası (`page` filtresi = `products.url`) için gerçek sorguları döndürür.
//!
//! Kullanıcı SA'yı Google Cloud'da oluşturup GSC mülküne kullanıcı olarak ekler; SA JSON
//! Ayarlar'dan yüklenir ve SQLite `settings`'te saklanır (koda/git'e gömülmez).

use super::{GscQuery, PageStat};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_TOKEN_URI: &str = "https://oauth2.googleapis.com/token";
/// Salt-okuma yeterli (yalnızca searchAnalytics + sites list).
const SCOPE: &str = "https://www.googleapis.com/auth/webmasters.readonly";
const SITES_URL: &str = "https://www.googleapis.com/webmasters/v3/sites";

#[derive(Deserialize)]
struct ServiceAccount {
    client_email: String,
    private_key: String,
    #[serde(default = "default_token_uri")]
    token_uri: String,
}
fn default_token_uri() -> String {
    DEFAULT_TOKEN_URI.to_string()
}

#[derive(Serialize)]
struct Claims<'a> {
    iss: &'a str,
    scope: &'a str,
    aud: &'a str,
    iat: u64,
    exp: u64,
}

/// SA JSON'dan yalnızca client_email'i çıkarır (UI'da göstermek için — private key sızmaz).
pub fn client_email_of(sa_json: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(sa_json)
        .ok()?
        .get("client_email")?
        .as_str()
        .map(String::from)
}

/// SA JSON'un geçerli biçimde olduğunu doğrular (parse + zorunlu alanlar).
pub fn validate_json(sa_json: &str) -> Result<String, String> {
    let sa: ServiceAccount = serde_json::from_str(sa_json)
        .map_err(|e| format!("Service-account JSON çözümlenemedi: {e}"))?;
    if sa.client_email.is_empty() || sa.private_key.is_empty() {
        return Err("JSON'da client_email/private_key eksik.".to_string());
    }
    // Anahtarın gerçekten yüklenebildiğini kontrol et
    jsonwebtoken::EncodingKey::from_rsa_pem(sa.private_key.as_bytes())
        .map_err(|e| format!("Service-account özel anahtarı okunamadı: {e}"))?;
    Ok(sa.client_email)
}

/// SA JSON → access token (RS256 JWT bearer akışı).
async fn access_token(client: &reqwest::Client, sa_json: &str) -> Result<String, String> {
    let sa: ServiceAccount = serde_json::from_str(sa_json)
        .map_err(|e| format!("Service-account JSON çözümlenemedi: {e}"))?;
    let now = chrono::Utc::now().timestamp() as u64;
    let claims = Claims {
        iss: &sa.client_email,
        scope: SCOPE,
        aud: &sa.token_uri,
        iat: now,
        exp: now + 3600,
    };
    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    let key = jsonwebtoken::EncodingKey::from_rsa_pem(sa.private_key.as_bytes())
        .map_err(|e| format!("Service-account özel anahtarı okunamadı: {e}"))?;
    let jwt = jsonwebtoken::encode(&header, &claims, &key)
        .map_err(|e| format!("JWT imzalanamadı: {e}"))?;

    let resp = client
        .post(&sa.token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", jwt.as_str()),
        ])
        .send()
        .await
        .map_err(|e| format!("Google token endpoint'ine ulaşılamadı: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("GSC token hatası (HTTP {}): {}", status.as_u16(), short(&text)));
    }
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v.get("access_token").and_then(|t| t.as_str()).map(String::from))
        .ok_or_else(|| "Token yanıtında access_token bulunamadı.".to_string())
}

/// GSC siteUrl'i (ör. `sc-domain:example.com`) yol segmenti olarak percent-encode eder.
fn pct(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Ürün sayfasının gerçek arama sorgularını çeker (son `days` gün, en çok gösterim alan `limit`).
pub async fn search_queries(
    client: &reqwest::Client,
    sa_json: &str,
    site_url: &str,
    page_url: &str,
    days: i64,
    limit: u32,
) -> Result<Vec<GscQuery>, String> {
    let token = access_token(client, sa_json).await?;
    let end = chrono::Utc::now().date_naive();
    let start = end - chrono::Duration::days(days);
    let body = serde_json::json!({
        "startDate": start.format("%Y-%m-%d").to_string(),
        "endDate": end.format("%Y-%m-%d").to_string(),
        "dimensions": ["query"],
        "rowLimit": limit,
        "dimensionFilterGroups": [{
            "filters": [{ "dimension": "page", "operator": "equals", "expression": page_url }]
        }]
    });
    let url = format!("{SITES_URL}/{}/searchAnalytics/query", pct(site_url));
    let resp = client
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("GSC'ye ulaşılamadı: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("GSC searchAnalytics hatası (HTTP {}): {}", status.as_u16(), short(&text)));
    }
    let data: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("GSC yanıtı okunamadı: {e}"))?;
    Ok(parse_rows(&data))
}

fn parse_rows(data: &serde_json::Value) -> Vec<GscQuery> {
    let rows = match data.get("rows").and_then(|r| r.as_array()) {
        Some(r) => r,
        None => return Vec::new(),
    };
    rows.iter()
        .filter_map(|row| {
            let query = row
                .get("keys")
                .and_then(|k| k.as_array())
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())?
                .to_string();
            Some(GscQuery {
                query,
                clicks: row.get("clicks").and_then(|v| v.as_f64()).unwrap_or(0.0),
                impressions: row.get("impressions").and_then(|v| v.as_f64()).unwrap_or(0.0),
                ctr: row.get("ctr").and_then(|v| v.as_f64()).unwrap_or(0.0),
                position: row.get("position").and_then(|v| v.as_f64()).unwrap_or(0.0),
            })
        })
        .collect()
}

/// **Fırsat analizi için:** tüm sayfaların toplam performansı — tek API çağrısı.
///
/// `search_queries`'den farkı: orada `page` bir FİLTRE (tek ürün), burada bir BOYUT (tüm site).
/// GSC `rowLimit` olarak 25.000'e izin veriyor; 262 ürünlük katalog tek istekte geliyor,
/// yani ürün başına çağrı yapmaya gerek yok (öyle olsaydı 262 istek + kota sorunu olurdu).
///
/// Dönen `page` değerleri `products.url` ile eşleşir — mevcut `search_queries` zaten
/// `page = products.url` filtresiyle çalışıyor, yani biçim uyumu kanıtlı.
pub async fn page_stats(
    client: &reqwest::Client,
    sa_json: &str,
    site_url: &str,
    days: i64,
    limit: u32,
) -> Result<Vec<PageStat>, String> {
    let token = access_token(client, sa_json).await?;
    let end = chrono::Utc::now().date_naive();
    let start = end - chrono::Duration::days(days);
    let body = serde_json::json!({
        "startDate": start.format("%Y-%m-%d").to_string(),
        "endDate": end.format("%Y-%m-%d").to_string(),
        "dimensions": ["page"],
        "rowLimit": limit,
    });
    let url = format!("{SITES_URL}/{}/searchAnalytics/query", pct(site_url));
    let resp = client
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("GSC'ye ulaşılamadı: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "GSC searchAnalytics hatası (HTTP {}): {}",
            status.as_u16(),
            short(&text)
        ));
    }
    let data: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("GSC yanıtı okunamadı: {e}"))?;
    Ok(parse_page_rows(&data))
}

fn parse_page_rows(data: &serde_json::Value) -> Vec<PageStat> {
    let rows = match data.get("rows").and_then(|r| r.as_array()) {
        Some(r) => r,
        None => return Vec::new(),
    };
    rows.iter()
        .filter_map(|row| {
            let page = row
                .get("keys")
                .and_then(|k| k.as_array())
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())?
                .to_string();
            Some(PageStat {
                page,
                clicks: row.get("clicks").and_then(|v| v.as_f64()).unwrap_or(0.0),
                impressions: row.get("impressions").and_then(|v| v.as_f64()).unwrap_or(0.0),
                ctr: row.get("ctr").and_then(|v| v.as_f64()).unwrap_or(0.0),
                position: row.get("position").and_then(|v| v.as_f64()).unwrap_or(0.0),
            })
        })
        .collect()
}

/// Ayarlarda "Bağlantıyı test et": token al + site listesini çek, yapılandırılan mülkü doğrula.
pub async fn test(sa_json: &str, site_url: &str) -> Result<String, String> {
    let email = validate_json(sa_json)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(25))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;
    let token = access_token(&client, sa_json).await?;
    let resp = client
        .get(SITES_URL)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| format!("GSC'ye ulaşılamadı: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("GSC erişimi reddedildi (HTTP {}): {}", status.as_u16(), short(&text)));
    }
    let sites: Vec<String> = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| {
            v.get("siteEntry").and_then(|e| e.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|s| s.get("siteUrl").and_then(|u| u.as_str()).map(String::from))
                    .collect()
            })
        })
        .unwrap_or_default();

    let site = site_url.trim();
    if site.is_empty() {
        return Ok(if sites.is_empty() {
            format!(
                "Token geçerli ({email}) ama erişilebilen mülk yok. SA e-postasını GSC mülküne kullanıcı olarak ekleyin."
            )
        } else {
            format!(
                "Bağlantı doğrulandı · {} · erişilebilen mülk(ler): {}. Birini yukarıya yapıştırın.",
                email,
                sites.join(", ")
            )
        });
    }
    if sites.iter().any(|s| s == site) {
        Ok(format!("Bağlantı doğrulandı · '{site}' mülküne erişim var ✓"))
    } else {
        Err(format!(
            "Token geçerli ama '{}' mülkü erişilebilir değil. SA e-postasını ({}) GSC mülküne kullanıcı olarak ekleyin. Erişilebilen: {}",
            site,
            email,
            if sites.is_empty() { "(yok)".into() } else { sites.join(", ") }
        ))
    }
}

fn short(s: &str) -> String {
    let t = s.trim();
    if t.len() > 200 {
        format!("{}…", &t[..200])
    } else {
        t.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pct_encodes_sc_domain() {
        assert_eq!(pct("sc-domain:kurumsalit.com"), "sc-domain%3Akurumsalit.com");
        assert_eq!(
            pct("https://www.kurumsalit.com/"),
            "https%3A%2F%2Fwww.kurumsalit.com%2F"
        );
    }

    #[test]
    fn client_email_extracted() {
        let json = r#"{"client_email":"svc@proj.iam.gserviceaccount.com","private_key":"x"}"#;
        assert_eq!(
            client_email_of(json).as_deref(),
            Some("svc@proj.iam.gserviceaccount.com")
        );
        assert!(client_email_of("not json").is_none());
    }

    #[test]
    fn validate_json_rejects_bad() {
        assert!(validate_json("{}").is_err());
        assert!(validate_json(r#"{"client_email":"a","private_key":""}"#).is_err());
    }

    #[test]
    fn parse_rows_reads_metrics() {
        let data = serde_json::json!({
            "rows": [
                { "keys": ["lenovo all in one"], "clicks": 5.0, "impressions": 120.0, "ctr": 0.041, "position": 7.3 },
                { "keys": ["all in one pc"], "clicks": 0.0, "impressions": 40.0, "ctr": 0.0, "position": 15.0 },
                { "clicks": 1.0 }
            ]
        });
        let rows = parse_rows(&data);
        assert_eq!(rows.len(), 2); // keys'siz satır atlandı
        assert_eq!(rows[0].query, "lenovo all in one");
        assert_eq!(rows[0].impressions, 120.0);
        assert_eq!(rows[1].query, "all in one pc");
    }

    #[test]
    fn parse_rows_empty_when_no_rows() {
        assert!(parse_rows(&serde_json::json!({})).is_empty());
    }

    /// Gerçek GSC. GSC_SA_FILE (SA JSON dosya yolu) gerekir; GSC_SITE + GSC_PAGE opsiyonel.
    /// Site boşsa erişilebilir mülkleri listeler; site+page verilirse gerçek sorguları çeker.
    /// `GSC_SA_FILE=/path/sa.json cargo test gsc_real -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn gsc_real() {
        let file = std::env::var("GSC_SA_FILE").expect("GSC_SA_FILE ayarlı değil");
        let sa_json = std::fs::read_to_string(&file).expect("SA dosyası okunamadı");
        let site = std::env::var("GSC_SITE").unwrap_or_default();

        // Mülk erişimini/doğrulamasını yap (site boşsa erişilebilenleri listeler)
        match test(&sa_json, &site).await {
            Ok(msg) => println!("TEST OK: {msg}"),
            Err(e) => println!("TEST HATA: {e}"),
        }

        // Sayfa verildiyse gerçek sorguları çek
        if let (false, Ok(page)) = (site.is_empty(), std::env::var("GSC_PAGE")) {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap();
            match search_queries(&client, &sa_json, &site, &page, 90, 25).await {
                Ok(q) => {
                    println!("{} sorgu:", q.len());
                    for r in q.iter().take(15) {
                        println!(
                            "  {} — gösterim {:.0}, tıklama {:.0}, sıra {:.1}",
                            r.query, r.impressions, r.clicks, r.position
                        );
                    }
                }
                Err(e) => println!("SORGU HATA: {e}"),
            }
        }
    }
    /// Fırsat analizinin can damarı: `page_stats` gerçekte hangi URL'leri döndürüyor?
    /// Bu URL'ler `products.url` ile eşleşmezse tüm özellik sessizce boş liste gösterir.
    /// `GSC_SA_FILE=... GSC_SITE=... cargo test page_stats_real -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn page_stats_real() {
        let file = std::env::var("GSC_SA_FILE").expect("GSC_SA_FILE ayarlı değil");
        let sa_json = std::fs::read_to_string(&file).expect("SA dosyası okunamadı");
        let site = std::env::var("GSC_SITE").expect("GSC_SITE ayarlı değil");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();
        match page_stats(&client, &sa_json, &site, 90, 25_000).await {
            Ok(rows) => {
                println!("TOPLAM_SAYFA={}", rows.len());
                // Tümünü basmak 8000+ satır olur; eşleşme kontrolü için ilk 400 yeterli.
                // (2026-07-28 ölçümü: 8087 sayfa, 262 üründen 260'ı eşleşti — %99.2)
                for r in rows.iter().take(400) {
                    println!(
                        "SAYFA\t{}\t{:.0}\t{:.0}\t{:.4}\t{:.1}",
                        r.page, r.clicks, r.impressions, r.ctr, r.position
                    );
                }
            }
            Err(e) => panic!("page_stats hatası: {e}"),
        }
    }

}
