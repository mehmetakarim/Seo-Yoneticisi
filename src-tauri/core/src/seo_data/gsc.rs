//! Google Search Console — service-account ile gerçek arama sorguları (Faz 5).
//!
//! gsc-mcp'yi paketlemek yerine GSC REST'i doğrudan çağırıyoruz. Kimlik: **service-account**
//! (tarayıcı OAuth yok → bundling temiz). SA JSON'dan RS256 imzalı bir JWT üretilir
//! (`jsonwebtoken`), Google token endpoint'inden access token alınır, `searchAnalytics.query`
//! ürün sayfası (`page` filtresi = `products.url`) için gerçek sorguları döndürür.
//!
//! Kullanıcı SA'yı Google Cloud'da oluşturup GSC mülküne kullanıcı olarak ekler; SA JSON
//! Ayarlar'dan yüklenir ve SQLite `settings`'te saklanır (koda/git'e gömülmez).

use super::{GscQuery, PageStat, QueryPageStat};
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
    page_stats_offset(client, sa_json, site_url, days, 0, limit).await
}

/// `page_stats`'ın kaydırılmış dönem sürümü — **trend karşılaştırması için**.
///
/// `offset_days` kadar geriye kaydırılmış bir pencere döndürür: `offset_days = days` verilirse
/// "önceki 90 gün" alınır ve mevcut dönemle kıyaslanabilir. Aynı uzunlukta iki pencere
/// karşılaştırmak şart — farklı uzunluktaki dönemleri kıyaslamak sahte düşüş üretir.
pub async fn page_stats_offset(
    client: &reqwest::Client,
    sa_json: &str,
    site_url: &str,
    days: i64,
    offset_days: i64,
    limit: u32,
) -> Result<Vec<PageStat>, String> {
    let end = chrono::Utc::now().date_naive() - chrono::Duration::days(offset_days);
    let start = end - chrono::Duration::days(days);
    page_stats_range(
        client,
        sa_json,
        site_url,
        &start.format("%Y-%m-%d").to_string(),
        &end.format("%Y-%m-%d").to_string(),
        limit,
    )
    .await
}

/// `page_stats`'ın **açık tarih aralıklı** sürümü — ölçüm omurgasının anlık görüntüleri için.
///
/// Kaydırma yerine doğrudan `start`/`end` alıyor: geçmişi tohumlarken pencereler
/// `metrics::windows` tarafından hesaplanıyor ve buraya olduğu gibi veriliyor.
/// ⚠️ Ölçüldü (2026-08-07): GSC **17 ay** geriye veri veriyor, 28 günlük pencere 1,3–2,2 sn.
pub async fn page_stats_range(
    client: &reqwest::Client,
    sa_json: &str,
    site_url: &str,
    start: &str,
    end: &str,
    limit: u32,
) -> Result<Vec<PageStat>, String> {
    let token = access_token(client, sa_json).await?;
    let body = serde_json::json!({
        "startDate": start,
        "endDate": end,
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

/// **Sorgu × sayfa verisi** — fırsat analizinin "ne yazmalıyım" katmanı.
///
/// `page_stats` "bu sayfa toplam ne aldı" der; bu ise "bu sayfa ŞU SORGUDA kaçıncı sırada".
///
/// İki kısıt yönetiliyor:
/// - **Hacim:** sorgu kırılımı sayfa kırılımından kat kat büyük. `path_filter` verilirse
///   (ör. `/urun/`) GSC'ye `page contains` filtresi olarak geçilir; blog, kategori ve EOL
///   sayfaları hiç gelmez. Filtre yoksa her şey gelir ve çağıran istemcide eler.
/// - **Sayfalama:** GSC tek istekte en fazla 25.000 satır verir. `startRow` ile devam edilir;
///   dönen satır sayısı `rowLimit`'ten azsa son sayfadır.
///
/// `max_rows` bir emniyet freni: beklenmedik büyüklükte bir mülkte sonsuz döngüye girmesin.
pub async fn query_page_stats(
    client: &reqwest::Client,
    sa_json: &str,
    site_url: &str,
    days: i64,
    path_filter: Option<&str>,
    max_rows: usize,
) -> Result<Vec<QueryPageStat>, String> {
    const PAGE_SIZE: u32 = 25_000;
    let token = access_token(client, sa_json).await?;
    let end = chrono::Utc::now().date_naive();
    let start = end - chrono::Duration::days(days);
    let url = format!("{SITES_URL}/{}/searchAnalytics/query", pct(site_url));

    let mut out: Vec<QueryPageStat> = Vec::new();
    let mut start_row: usize = 0;

    loop {
        let mut body = serde_json::json!({
            "startDate": start.format("%Y-%m-%d").to_string(),
            "endDate": end.format("%Y-%m-%d").to_string(),
            "dimensions": ["page", "query"],
            "rowLimit": PAGE_SIZE,
            "startRow": start_row,
        });
        if let Some(pf) = path_filter {
            body["dimensionFilterGroups"] = serde_json::json!([{
                "filters": [{ "dimension": "page", "operator": "contains", "expression": pf }]
            }]);
        }

        let resp = client
            .post(&url)
            .bearer_auth(&token)
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
        let batch = parse_query_page_rows(&data);
        let got = batch.len();
        out.extend(batch);

        // Son sayfa: dönen satır istenen sayfadan az.
        if got < PAGE_SIZE as usize || out.len() >= max_rows {
            break;
        }
        start_row += got;
    }
    out.truncate(max_rows);
    Ok(out)
}

fn parse_query_page_rows(data: &serde_json::Value) -> Vec<QueryPageStat> {
    let rows = match data.get("rows").and_then(|r| r.as_array()) {
        Some(r) => r,
        None => return Vec::new(),
    };
    rows.iter()
        .filter_map(|row| {
            let keys = row.get("keys")?.as_array()?;
            Some(QueryPageStat {
                page: keys.first()?.as_str()?.to_string(),
                query: keys.get(1)?.as_str()?.to_string(),
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

    /// **Aşama 2'nin ön koşulu:** sorgu × sayfa verisi gerçekte kaç satır ve ne kadar sürüyor?
    /// Bu ölçüm yapılmadan sayfalama stratejisi kesinleştirilmemeli.
    /// `GSC_SA_FILE=... GSC_SITE=... GSC_PATH=/urun/ cargo test qp_volume -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn qp_volume() {
        let file = std::env::var("GSC_SA_FILE").expect("GSC_SA_FILE ayarlı değil");
        let sa_json = std::fs::read_to_string(&file).expect("SA dosyası okunamadı");
        let site = std::env::var("GSC_SITE").expect("GSC_SITE ayarlı değil");
        let path = std::env::var("GSC_PATH").ok();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap();
        let t0 = std::time::Instant::now();
        match query_page_stats(&client, &sa_json, &site, 90, path.as_deref(), 200_000).await {
            Ok(rows) => {
                let pages: std::collections::HashSet<&str> =
                    rows.iter().map(|r| r.page.as_str()).collect();
                let queries: std::collections::HashSet<&str> =
                    rows.iter().map(|r| r.query.as_str()).collect();
                println!("SATIR={}", rows.len());
                println!("BENZERSIZ_SAYFA={}", pages.len());
                println!("BENZERSIZ_SORGU={}", queries.len());
                println!("SURE_SN={:.1}", t0.elapsed().as_secs_f64());
                // GSC_DUMP=1 → tüm satırlar (katalogla karşılaştırma analizi için)
                let n = if std::env::var("GSC_DUMP").is_ok() { rows.len() } else { 5 };
                for r in rows.iter().take(n) {
                    println!(
                        "ORNEK\t{}\t{}\t{:.0}\t{:.0}\t{:.1}",
                        r.page, r.query, r.clicks, r.impressions, r.position
                    );
                }
            }
            Err(e) => panic!("query_page_stats hatası: {e}"),
        }
    }

    /// Trend eşiklerini gerçek veriyle doğrulamak için iki dönemi de döker.
    /// `GSC_SA_FILE=... GSC_SITE=... cargo test decay_dump -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn decay_dump() {
        let file = std::env::var("GSC_SA_FILE").expect("GSC_SA_FILE ayarlı değil");
        let sa_json = std::fs::read_to_string(&file).expect("SA okunamadı");
        let site = std::env::var("GSC_SITE").expect("GSC_SITE ayarlı değil");
        let client = reqwest::Client::builder().timeout(Duration::from_secs(120)).build().unwrap();
        for (label, offset) in [("SIMDI", 0i64), ("ONCE", 90i64)] {
            let rows = page_stats_offset(&client, &sa_json, &site, 90, offset, 25_000)
                .await
                .expect("çağrı");
            println!("{label}_TOPLAM={}", rows.len());
            for r in rows.iter() {
                println!("{label}\t{}\t{:.0}\t{:.0}\t{:.1}", r.page, r.clicks, r.impressions, r.position);
            }
        }
    }

}
