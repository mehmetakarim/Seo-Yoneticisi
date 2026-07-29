//! IdeaSoft gönderim modülü (Faz 9) — **opsiyonel**.
//!
//! Üretilen meta/açıklama/teknik tabloyu IdeaSoft'a tek tıkla yazar. Ayarlar'da domain + token
//! girilmemişse modül kapalıdır ve uygulama bugünkü kopyala-yapıştır akışıyla çalışır (global vizyon:
//! IdeaSoft kullanmayan kullanıcı etkilenmemeli).
//!
//! **MCP'ye gerek yok:** `mcp.myideasoft.com` bir "keşif + genel çağırıcı" sarmalayıcıdır (LLM ajanları
//! için). Altındaki gerçek yüzey `https://{domain}/admin-api/...` olup `Authorization: Bearer {token}`
//! ile doğrudan çağrılır — Node/npx/mcp-remote yok (Faz 4 mimari kararıyla tutarlı).
//!
//! Canlı doğrulanan uçlar: `GET /admin-api/products?s={sku}` (sku→id), `GET /admin-api/products/{id}`,
//! `PUT /admin-api/products/{id}`. Gözlemlenen hız sınırı ~40 istek/dk.

use serde::Serialize;
use std::time::Duration;

/// Uzaktaki ürünün ilgili alanları (fark önizlemesi için).
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct RemoteProduct {
    pub id: i64,
    pub sku: String,
    pub name: String,
    pub page_title: String,
    pub meta_description: String,
    pub meta_keywords: String,
    pub search_keywords: String,
    pub target_keyword: String,
    pub details: String,
    pub extra_details: String,
    /// IdeaSoft'un kendi SEO kural skoru (bilgi amaçlı).
    pub seo_rule_count: Option<i64>,
}

/// Uygulamadaki (gönderilecek) içerik.
#[derive(Debug, Clone, Default)]
pub struct LocalContent {
    pub page_title: String,
    pub meta_description: String,
    pub meta_keywords: String,
    pub search_keywords: String,
    pub target_keyword: String,
    pub details_html: String,
    pub tech_html: String,
}

fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))
}

/// Kullanıcının girdiği domaini normalize eder (şema ekler, sondaki `/` ve `/admin-api`'yi atar).
fn base_url(domain: &str) -> Result<String, String> {
    let d = domain.trim().trim_end_matches('/');
    if d.is_empty() {
        return Err("IdeaSoft mağaza adresi ayarlı değil.".to_string());
    }
    let d = if d.starts_with("http://") || d.starts_with("https://") {
        d.to_string()
    } else {
        format!("https://{d}")
    };
    Ok(d.trim_end_matches("/admin-api").trim_end_matches('/').to_string())
}

/// HTTP durumunu anlaşılır Türkçe mesaja çevirir.
fn map_status(code: u16, body: &str) -> String {
    match code {
        401 | 403 => "IdeaSoft token'ı geçersiz veya süresi dolmuş. Ayarlar'dan yenileyin.".to_string(),
        404 => "Ürün IdeaSoft'ta bulunamadı.".to_string(),
        429 => "IdeaSoft hız sınırına takıldı, biraz bekleyip tekrar deneyin.".to_string(),
        _ => format!("IdeaSoft hatası (HTTP {code}): {}", short(body)),
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

fn s_of(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn to_remote(v: &serde_json::Value) -> RemoteProduct {
    let detail = v.get("detail").cloned().unwrap_or(serde_json::Value::Null);
    RemoteProduct {
        id: v.get("id").and_then(|x| x.as_i64()).unwrap_or(0),
        sku: s_of(v, "sku"),
        name: s_of(v, "name"),
        page_title: s_of(v, "pageTitle"),
        meta_description: s_of(v, "metaDescription"),
        meta_keywords: s_of(v, "metaKeywords"),
        search_keywords: s_of(v, "searchKeywords"),
        target_keyword: s_of(v, "targetKeyword"),
        details: s_of(&detail, "details"),
        extra_details: s_of(&detail, "extraDetails"),
        seo_rule_count: v.get("seoTotalRuleCount").and_then(|x| x.as_i64()),
    }
}

/// Arama sonucundan dönen özet (id + IdeaSoft'un kendi SEO kural skoru).
/// **Not:** `seoTotalRuleCount` yalnızca LİSTE ucunda dolu gelir; `/products/{id}` ucunda `null`.
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct Resolved {
    pub id: i64,
    pub seo_rule_count: Option<i64>,
}

/// sku → IdeaSoft ürün id'si (+ SEO kural skoru). `?s=` arama parametresi kullanılır
/// Katalogdaki bir ürünün hafif özeti — EOL sayfaları eşleştirmek için.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CatalogItem {
    pub id: i64,
    /// URL'nin son parçası (`/urun/<slug>`), eşleştirme anahtarı.
    pub slug: String,
    pub name: String,
    /// IdeaSoft ürün durumu (1 = aktif).
    pub status: i64,
    /// ⚠️ **Liste ucunda GÜVENİLİR DEĞİL.** 2026-07-29 ölçümü: 300 üründe hepsi 0 geldi,
    /// oysa detay ucu (`/products/{id}`) aynı ürün için 1.0 döndürüyordu — liste yanıtı bu
    /// alanı doldurmuyor. Saklanıyor ama **arayüzde gösterilmemeli**: "hepsi stokta yok" gibi
    /// yanıltıcı olur. Stok gerekiyorsa ürün başına detay ucundan çekilmeli.
    pub stock: f64,
    /// Şu an tanımlı canonical (boşsa yok).
    pub canonical: String,
}

/// **Tüm kataloğu** sayfalayarak çeker.
///
/// Neden gerekli: XML feed bilinçli olarak sınırlı (bu mağazada 10.909 üründen 262'si).
/// Feed dışı sayfalar Google'dan ciddi trafik alıyor ama uygulama onları hiç görmüyordu.
/// Sayfa başına tek tek sormak 40 istek/dk sınırında ~27 dakika sürerdi; kataloğu bir kez
/// çekmek ~110 istek.
///
/// ⚠️ **Yine de ~7 dakika sürüyor** — ölçüm (2026-07-29): 300 ürün 12,2 sn, yani sayfa başına
/// ~4 sn. Arka planda çalışmalı ve ilerleme gösterilmeli; kullanıcı donmuş sanmasın.
///
/// `on_progress` her sayfadan sonra (çekilen, toplam) ile çağrılır — uzun süren işlemde
/// kullanıcı ilerlemeyi görsün.
pub async fn fetch_catalog<F>(
    domain: &str,
    token: &str,
    max_items: usize,
    mut on_progress: F,
) -> Result<Vec<CatalogItem>, String>
where
    F: FnMut(usize, usize),
{
    const PAGE: usize = 100;
    let base = base_url(domain)?;
    let client = http()?;
    let mut out: Vec<CatalogItem> = Vec::new();
    let mut page = 1usize;
    let mut total_hint = 0usize;

    loop {
        let resp = client
            .get(format!("{base}/admin-api/products"))
            .query(&[("limit", PAGE.to_string()), ("page", page.to_string())])
            .bearer_auth(token.trim())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("IdeaSoft'a ulaşılamadı: {e}"))?;

        // Toplam sayı yanıt başlığında geliyor — ilerleme göstergesi için.
        if total_hint == 0 {
            if let Some(v) = resp.headers().get("total_count").and_then(|v| v.to_str().ok()) {
                total_hint = v.parse().unwrap_or(0);
            }
        }
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if status.as_u16() == 401 {
            return Err("IdeaSoft token süresi dolmuş — Ayarlar'dan yenileyin.".into());
        }
        if !status.is_success() {
            return Err(format!("IdeaSoft katalog hatası (HTTP {})", status.as_u16()));
        }
        let arr: Vec<serde_json::Value> =
            serde_json::from_str(&text).map_err(|e| format!("Katalog yanıtı okunamadı: {e}"))?;
        let got = arr.len();
        for v in arr {
            out.push(CatalogItem {
                id: v.get("id").and_then(|x| x.as_i64()).unwrap_or(0),
                slug: v.get("slug").and_then(|x| x.as_str()).unwrap_or("").to_lowercase(),
                name: v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                status: v.get("status").and_then(|x| x.as_i64()).unwrap_or(0),
                stock: v.get("stockAmount").and_then(|x| x.as_f64()).unwrap_or(0.0),
                canonical: v
                    .get("canonicalUrl")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
            });
        }
        on_progress(out.len(), total_hint);

        if got < PAGE || out.len() >= max_items {
            break;
        }
        page += 1;
        // Gözlemlenen hız sınırı 40 istek/dk → istekleri nazik tut.
        tokio::time::sleep(std::time::Duration::from_millis(1600)).await;
    }
    out.truncate(max_items);
    Ok(out)
}

/// (`?sku=`/`?name=` yok sayılıyor). Birden fazla sonuçta **sku'su birebir eşleşen** seçilir.
pub async fn resolve(domain: &str, token: &str, sku: &str) -> Result<Option<Resolved>, String> {
    let base = base_url(domain)?;
    let client = http()?;
    let resp = client
        .get(format!("{base}/admin-api/products"))
        .query(&[("s", sku), ("limit", "20")])
        .bearer_auth(token.trim())
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("IdeaSoft'a ulaşılamadı: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(map_status(status.as_u16(), &text));
    }
    let arr: Vec<serde_json::Value> =
        serde_json::from_str(&text).map_err(|e| format!("IdeaSoft yanıtı okunamadı: {e}"))?;
    Ok(pick_exact_sku(&arr, sku))
}

/// Arama sonucundan sku'su birebir eşleşeni seçer (test edilebilir, ağsız).
fn pick_exact_sku(arr: &[serde_json::Value], sku: &str) -> Option<Resolved> {
    let want = sku.trim().to_lowercase();
    arr.iter()
        .find(|p| s_of(p, "sku").trim().to_lowercase() == want)
        .and_then(|p| {
            p.get("id").and_then(|x| x.as_i64()).map(|id| Resolved {
                id,
                seo_rule_count: p.get("seoTotalRuleCount").and_then(|x| x.as_i64()),
            })
        })
}

/// **IdeaSoft kısıtı:** `detail` nesnesi gönderilirken `details` alanı null olamaz
/// (`Validation Error … "details":"This value should not be null."`). Bu yüzden kısmi güncellemede
/// eksik alt alanlar **uzaktaki mevcut değerle** doldurulur — böylece dokunulmayan alan korunur.
pub fn fill_detail_from_remote(payload: &mut serde_json::Value, remote: &RemoteProduct) {
    let Some(obj) = payload.as_object_mut() else { return };
    let Some(detail) = obj.get_mut("detail").and_then(|d| d.as_object_mut()) else { return };
    if !detail.contains_key("details") {
        detail.insert("details".into(), serde_json::Value::String(remote.details.clone()));
    }
    if !detail.contains_key("extraDetails") {
        detail.insert(
            "extraDetails".into(),
            serde_json::Value::String(remote.extra_details.clone()),
        );
    }
}

pub async fn fetch_product(domain: &str, token: &str, id: i64) -> Result<RemoteProduct, String> {
    let base = base_url(domain)?;
    let client = http()?;
    let resp = client
        .get(format!("{base}/admin-api/products/{id}"))
        .bearer_auth(token.trim())
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("IdeaSoft'a ulaşılamadı: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(map_status(status.as_u16(), &text));
    }
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("IdeaSoft yanıtı okunamadı: {e}"))?;
    Ok(to_remote(&v))
}

/// Gönderilecek gövdeyi kurar. **Yalnızca istenen parçalar ve boş olmayan alanlar** yazılır —
/// uzaktaki dolu bir alanı yanlışlıkla silmeyelim. `details` + `tech` birlikte istenirse
/// tek bir `detail` nesnesinde birleşir (biri diğerini ezmesin).
pub fn build_payload(parts: &[String], local: &LocalContent) -> serde_json::Value {
    let has = |p: &str| parts.iter().any(|x| x == p);
    let mut body = serde_json::Map::new();

    if has("meta") {
        let mut put = |k: &str, v: &str| {
            if !v.trim().is_empty() {
                body.insert(k.to_string(), serde_json::Value::String(v.trim().to_string()));
            }
        };
        put("pageTitle", &local.page_title);
        put("metaDescription", &local.meta_description);
        put("metaKeywords", &local.meta_keywords);
        put("searchKeywords", &local.search_keywords);
        put("targetKeyword", &local.target_keyword);
    }

    // Yalnızca hedef kelime gönderimi (IdeaSoft'un SEO kural skorunu etkiler).
    if has("keyword") && !local.target_keyword.trim().is_empty() {
        body.insert(
            "targetKeyword".to_string(),
            serde_json::Value::String(local.target_keyword.trim().to_string()),
        );
    }

    let mut detail = serde_json::Map::new();
    if has("details") && !local.details_html.trim().is_empty() {
        detail.insert("details".into(), serde_json::Value::String(local.details_html.clone()));
    }
    if has("tech") && !local.tech_html.trim().is_empty() {
        detail.insert("extraDetails".into(), serde_json::Value::String(local.tech_html.clone()));
    }
    if !detail.is_empty() {
        body.insert("detail".into(), serde_json::Value::Object(detail));
    }
    serde_json::Value::Object(body)
}

pub async fn push_product(
    domain: &str,
    token: &str,
    id: i64,
    payload: &serde_json::Value,
) -> Result<(), String> {
    if payload.as_object().map_or(true, |o| o.is_empty()) {
        return Err("Gönderilecek içerik yok.".to_string());
    }
    let base = base_url(domain)?;
    let client = http()?;
    let resp = client
        .put(format!("{base}/admin-api/products/{id}"))
        .bearer_auth(token.trim())
        .header("Accept", "application/json")
        .json(payload)
        .send()
        .await
        .map_err(|e| format!("IdeaSoft'a ulaşılamadı: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(map_status(status.as_u16(), &text));
    }
    Ok(())
}

/// Ayarlardaki "Bağlantıyı test et".
pub async fn test_connection(domain: &str, token: &str) -> Result<String, String> {
    if token.trim().is_empty() {
        return Err("IdeaSoft token'ı ayarlı değil.".to_string());
    }
    let base = base_url(domain)?;
    let client = http()?;
    let resp = client
        .get(format!("{base}/admin-api/products"))
        .query(&[("limit", "1")])
        .bearer_auth(token.trim())
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("IdeaSoft'a ulaşılamadı: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(map_status(status.as_u16(), &text));
    }
    Ok("Bağlantı doğrulandı · mağazaya erişim var ✓".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local() -> LocalContent {
        LocalContent {
            page_title: "Başlık".into(),
            meta_description: "Açıklama".into(),
            meta_keywords: "a, b".into(),
            search_keywords: "c, d".into(),
            target_keyword: "hedef kelime".into(),
            details_html: "<section>açıklama</section>".into(),
            tech_html: "<table class=\"teknik-tablo\"></table>".into(),
        }
    }

    #[test]
    fn base_url_normalizes() {
        assert_eq!(base_url("3ekurumsal.myideasoft.com").unwrap(), "https://3ekurumsal.myideasoft.com");
        assert_eq!(base_url("https://x.com/").unwrap(), "https://x.com");
        assert_eq!(base_url("https://x.com/admin-api").unwrap(), "https://x.com");
        assert!(base_url("  ").is_err());
    }

    #[test]
    fn payload_meta_only() {
        let p = build_payload(&["meta".into()], &local());
        let o = p.as_object().unwrap();
        assert_eq!(o["pageTitle"], "Başlık");
        assert_eq!(o["targetKeyword"], "hedef kelime");
        assert!(!o.contains_key("detail")); // açıklama/tablo gönderilmedi
    }

    #[test]
    fn payload_details_and_tech_merge_into_one_detail() {
        let p = build_payload(&["details".into(), "tech".into()], &local());
        let d = p["detail"].as_object().unwrap();
        assert!(d.contains_key("details"));
        assert!(d.contains_key("extraDetails")); // biri diğerini ezmedi
        assert!(!p.as_object().unwrap().contains_key("pageTitle"));
    }

    #[test]
    fn payload_omits_empty_fields() {
        let mut l = local();
        l.meta_keywords = "  ".into();
        l.tech_html = "".into();
        let p = build_payload(&["meta".into(), "tech".into()], &l);
        let o = p.as_object().unwrap();
        assert!(!o.contains_key("metaKeywords")); // boş alan uzaktakini silmesin
        assert!(!o.contains_key("detail")); // boş tablo → detail hiç yok
    }

    #[test]
    fn payload_keyword_only_sends_target_keyword() {
        let p = build_payload(&["keyword".into()], &local());
        let o = p.as_object().unwrap();
        assert_eq!(o["targetKeyword"], "hedef kelime");
        assert_eq!(o.len(), 1); // başka hiçbir alana dokunulmaz
    }

    #[test]
    fn payload_empty_when_nothing_selected() {
        let p = build_payload(&[], &local());
        assert!(p.as_object().unwrap().is_empty());
    }

    #[test]
    fn pick_exact_sku_ignores_partial_matches() {
        let arr = vec![
            serde_json::json!({ "id": 1, "sku": "ABC-123-XL" }),
            serde_json::json!({ "id": 2, "sku": "ABC-123", "seoTotalRuleCount": 13 }),
        ];
        assert_eq!(pick_exact_sku(&arr, "ABC-123").unwrap().id, 2); // kısmi eşleşmeye kanmaz
        assert_eq!(pick_exact_sku(&arr, "abc-123").unwrap().id, 2); // büyük/küçük harf duyarsız
        assert_eq!(pick_exact_sku(&arr, "ABC-123").unwrap().seo_rule_count, Some(13));
        assert!(pick_exact_sku(&arr, "YOK").is_none());
    }

    #[test]
    fn fill_detail_keeps_untouched_side() {
        let remote = RemoteProduct {
            details: "<p>mevcut açıklama</p>".into(),
            extra_details: "<table>mevcut tablo</table>".into(),
            ..Default::default()
        };
        // Yalnızca teknik tablo gönderiliyor → details uzaktakiyle doldurulmalı (HTTP 400 önlenir)
        let mut p = build_payload(&["tech".into()], &local());
        fill_detail_from_remote(&mut p, &remote);
        let d = p["detail"].as_object().unwrap();
        assert_eq!(d["details"], "<p>mevcut açıklama</p>"); // dokunulmadı
        assert!(d["extraDetails"].as_str().unwrap().contains("teknik-tablo")); // yeni değer
        // detail hiç yoksa dokunma
        let mut only_meta = build_payload(&["meta".into()], &local());
        fill_detail_from_remote(&mut only_meta, &remote);
        assert!(!only_meta.as_object().unwrap().contains_key("detail"));
    }

    #[test]
    fn status_messages_are_actionable() {
        assert!(map_status(401, "").contains("süresi dolmuş"));
        assert!(map_status(404, "").contains("bulunamadı"));
        assert!(map_status(429, "").contains("hız sınırı"));
    }

    #[test]
    fn to_remote_reads_nested_detail() {
        let v = serde_json::json!({
            "id": 119894, "sku": "X", "name": "Ürün", "pageTitle": "T",
            "detail": { "details": "<p>a</p>", "extraDetails": "<table>t</table>" },
            "seoTotalRuleCount": 7
        });
        let r = to_remote(&v);
        assert_eq!(r.id, 119894);
        assert_eq!(r.details, "<p>a</p>");
        assert_eq!(r.extra_details, "<table>t</table>");
        assert_eq!(r.seo_rule_count, Some(7));
    }

    /// Gerçek mağaza — **YALNIZCA OKUMA** (canlı veriye yazmaz).
    /// `IDEASOFT_DOMAIN=... IDEASOFT_TOKEN=... cargo test ideasoft_read_real -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn ideasoft_read_real() {
        let domain = std::env::var("IDEASOFT_DOMAIN").expect("IDEASOFT_DOMAIN yok");
        let token = std::env::var("IDEASOFT_TOKEN").expect("IDEASOFT_TOKEN yok");
        let sku = std::env::var("IDEASOFT_SKU")
            .unwrap_or_else(|_| "MNL.GNX.WSH.ANY.WS3LC0WH-Y-O".to_string());

        println!("test: {}", test_connection(&domain, &token).await.expect("bağlantı"));
        let r = resolve(&domain, &token, &sku).await.expect("arama").expect("id bulunamadı");
        let id = r.id;
        println!("sku {sku} → id {id} · seoRule={:?}", r.seo_rule_count);
        let p = fetch_product(&domain, &token, id).await.expect("ürün");
        println!(
            "  {} · pageTitle={:?} · targetKeyword={:?}\n  details={} krk · extraDetails={} krk · seoRule={:?}",
            p.name, p.page_title, p.target_keyword,
            p.details.len(), p.extra_details.len(), p.seo_rule_count
        );
        assert_eq!(p.id, id);
    }
    /// Katalog çekimi canlı test — sayfalama, başlıktan toplam sayı, alan eşlemesi.
    /// `IDEASOFT_DOMAIN=... IDEASOFT_TOKEN=... cargo test catalog_real -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn catalog_real() {
        let domain = std::env::var("IDEASOFT_DOMAIN").expect("IDEASOFT_DOMAIN yok");
        let token = std::env::var("IDEASOFT_TOKEN").expect("IDEASOFT_TOKEN yok");
        let cap: usize = std::env::var("IDEASOFT_CAP")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300);
        let t0 = std::time::Instant::now();
        let items = fetch_catalog(&domain, &token, cap, |done, total| {
            println!("ILERLEME\t{done}/{total}");
        })
        .await
        .expect("katalog çekilemedi");
        println!("CEKILEN={} SURE_SN={:.1}", items.len(), t0.elapsed().as_secs_f64());
        let with_slug = items.iter().filter(|i| !i.slug.is_empty()).count();
        let active = items.iter().filter(|i| i.status == 1).count();
        let in_stock = items.iter().filter(|i| i.stock > 0.0).count();
        let with_canon = items.iter().filter(|i| !i.canonical.is_empty()).count();
        println!("SLUG_DOLU={with_slug} AKTIF={active} STOKTA={in_stock} CANONICAL_TANIMLI={with_canon}");
        for it in items.iter().take(3) {
            println!("ORNEK\tid={} status={} stok={} slug={}", it.id, it.status, it.stock, it.slug);
        }
        assert!(with_slug > 0, "slug alanı boş geliyorsa eşleştirme yapılamaz");
    }

}
