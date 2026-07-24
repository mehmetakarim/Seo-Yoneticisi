use crate::images::{self, ImageCheck};
use crate::seo_data::{self, SeoInsights};
use crate::validation::{
    details_badge, image_badge, meta_badge, overall_status, MetaBadge, MetaInput, OverallStatus,
};
use crate::{db, feed, gemini, sync};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub conn: Mutex<Connection>,
    #[allow(dead_code)] // Faz 2/3: harici DB yolu işlemleri için saklanır
    pub db_path: PathBuf,
}

fn now_str() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

#[derive(Debug, Serialize)]
pub struct ProductRow {
    pub sku: String,
    pub name: String,
    pub brand: Option<String>,
    pub img_url: Option<String>,
    pub meta_badge: MetaBadge,
    pub details_badge: MetaBadge,
    pub overall: OverallStatus,
    pub meta_done: bool,
    pub details_done: bool,
}

#[derive(Debug, Serialize)]
pub struct ProductDetail {
    pub sku: String,
    pub name: String,
    pub brand: Option<String>,
    pub main_category: Option<String>,
    pub category: Option<String>,
    pub quantity: Option<i64>,
    pub url: Option<String>,
    pub img_url: Option<String>,
    pub title: Option<String>,
    pub descriptions: Option<String>,
    pub keywords: Option<String>,
    pub search_keywords: Option<String>,
    pub details: Option<String>,
    pub meta_status: String,
    pub details_status: String,
    pub target_keyword: Option<String>,
    pub draft_title: Option<String>,
    pub draft_descriptions: Option<String>,
    pub draft_search_keywords: Option<String>,
    pub draft_details: Option<String>,
    pub badge: MetaBadge,
    pub details_badge: MetaBadge,
    pub overall: OverallStatus,
    // Faz 7: galeri görselleri + skoru
    pub gallery: Vec<String>,
    pub image_count: usize,
    pub image_badge: MetaBadge,
    pub image_check: Option<Vec<ImageCheck>>,
}

#[derive(Debug, Serialize)]
pub struct Settings {
    pub feed_url: String,
    pub gemini_api_key: String,
    /// Faz 4: Ahrefs free-tools captcha'sını çözmek için CapSolver anahtarı.
    pub capsolver_api_key: String,
    /// Faz 4: SEO araştırma ülke kodu (Ahrefs/Trends), varsayılan "tr".
    pub seo_country: String,
    /// Faz 5: GSC mülkü (ör. `sc-domain:kurumsalit.com` veya `https://site/`).
    pub gsc_site_url: String,
    /// Faz 5: yüklü service-account'un e-postası (yalnızca gösterim; private key sızmaz).
    /// Boş → GSC yapılandırılmamış.
    pub gsc_client_email: String,
    pub theme: Option<String>,
    pub last_backup_at: Option<String>,
}

struct RowData {
    sku: String,
    name: String,
    brand: Option<String>,
    img_url: Option<String>,
    title: Option<String>,
    descriptions: Option<String>,
    details: Option<String>,
    meta_status: String,
    details_status: String,
    target_keyword: Option<String>,
    draft_title: Option<String>,
    draft_descriptions: Option<String>,
    draft_details: Option<String>,
}

/// Meta rozeti — taslak varsa taslak (NULL değilse) yoksa feed değeri üzerinden.
fn meta_badge_of(r: &RowData) -> MetaBadge {
    let title = r.draft_title.as_deref().unwrap_or(r.title.as_deref().unwrap_or(""));
    let desc = r
        .draft_descriptions
        .as_deref()
        .unwrap_or(r.descriptions.as_deref().unwrap_or(""));
    meta_badge(&MetaInput {
        title,
        descriptions: desc,
        target_keyword: r.target_keyword.as_deref().unwrap_or(""),
        meta_done: r.meta_status == "done",
    })
}

/// Details rozeti — taslak varsa taslak yoksa feed details üzerinden.
fn details_badge_of(r: &RowData) -> MetaBadge {
    let html = r.draft_details.as_deref().unwrap_or(r.details.as_deref().unwrap_or(""));
    details_badge(html, r.target_keyword.as_deref().unwrap_or(""), r.details_status == "done")
}

#[tauri::command]
pub async fn sync_feed(state: State<'_, AppState>) -> Result<sync::SyncSummary, String> {
    let url = {
        let conn = state.conn.lock().unwrap();
        db::feed_url(&conn)?
    };
    let items = feed::fetch_and_parse(&url).await?;
    let mut conn = state.conn.lock().unwrap();
    sync::sync_products(&mut conn, items)
}

#[tauri::command]
pub fn get_last_sync(state: State<'_, AppState>) -> Result<Option<sync::SyncSummary>, String> {
    let conn = state.conn.lock().unwrap();
    sync::last_sync(&conn)
}

#[tauri::command]
pub fn list_products(
    state: State<'_, AppState>,
    filter: Option<String>,
    search: Option<String>,
) -> Result<Vec<ProductRow>, String> {
    let conn = state.conn.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT p.sku, p.name, p.brand, p.img_url, p.title, p.descriptions, p.details,
                    COALESCE(s.meta_status,'pending'), COALESCE(s.details_status,'pending'),
                    s.target_keyword, s.draft_title, s.draft_descriptions, s.draft_details
             FROM products p LEFT JOIN seo_status s ON s.sku = p.sku
             ORDER BY p.name COLLATE NOCASE",
        )
        .map_err(|e| format!("Ürün listesi hazırlanamadı: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(RowData {
                sku: row.get(0)?,
                name: row.get(1)?,
                brand: row.get(2)?,
                img_url: row.get(3)?,
                title: row.get(4)?,
                descriptions: row.get(5)?,
                details: row.get(6)?,
                meta_status: row.get(7)?,
                details_status: row.get(8)?,
                target_keyword: row.get(9)?,
                draft_title: row.get(10)?,
                draft_descriptions: row.get(11)?,
                draft_details: row.get(12)?,
            })
        })
        .map_err(|e| format!("Ürün listesi okunamadı: {e}"))?
        .filter_map(Result::ok);

    let q = search.unwrap_or_default().trim().to_lowercase();
    let f = filter.unwrap_or_default();
    let mut out = Vec::new();
    for r in rows {
        if !q.is_empty()
            && !r.name.to_lowercase().contains(&q)
            && !r.sku.to_lowercase().contains(&q)
        {
            continue;
        }
        let meta = meta_badge_of(&r);
        let details = details_badge_of(&r);
        let meta_done = r.meta_status == "done";
        let details_done = r.details_status == "done";
        let overall = overall_status(meta, details, meta_done, details_done);
        let keep = match f.as_str() {
            "" | "hepsi" => true, // filtre yok: tamamlananlar dahil her şey
            "tumu" => overall != OverallStatus::Tamamlandi,
            "eksik" => overall == OverallStatus::Eksik,
            "hatali" => overall == OverallStatus::Hatali,
            "bekliyor" => overall == OverallStatus::Bekliyor,
            "uygun" => overall == OverallStatus::Uygun,
            "tamamlandi" => overall == OverallStatus::Tamamlandi,
            other => return Err(format!("Bilinmeyen filtre: {other}")),
        };
        if keep {
            out.push(ProductRow {
                sku: r.sku,
                name: r.name,
                brand: r.brand,
                img_url: r.img_url,
                meta_badge: meta,
                details_badge: details,
                overall,
                meta_done,
                details_done,
            });
        }
    }
    Ok(out)
}

fn read_detail(conn: &Connection, sku: &str) -> Result<ProductDetail, String> {
    conn.query_row(
        "SELECT p.sku, p.name, p.brand, p.main_category, p.category, p.quantity, p.url,
                p.img_url, p.title, p.descriptions, p.keywords, p.search_keywords, p.details,
                COALESCE(s.meta_status,'pending'), COALESCE(s.details_status,'pending'),
                s.target_keyword, s.draft_title, s.draft_descriptions, s.draft_search_keywords,
                s.draft_details, p.picture2, p.picture3, p.picture4, s.image_check_json
         FROM products p LEFT JOIN seo_status s ON s.sku = p.sku
         WHERE p.sku = ?1",
        [&sku],
        |row| {
            let img_url: Option<String> = row.get(7)?;
            let picture2: Option<String> = row.get(20)?;
            let picture3: Option<String> = row.get(21)?;
            let picture4: Option<String> = row.get(22)?;
            let check_json: Option<String> = row.get(23)?;
            let gallery: Vec<String> = [img_url.clone(), picture2, picture3, picture4]
                .into_iter()
                .filter_map(|u| u.filter(|s| !s.trim().is_empty()))
                .collect();
            let image_check: Option<Vec<ImageCheck>> = check_json
                .as_deref()
                .and_then(|j| serde_json::from_str(j).ok());
            Ok(ProductDetail {
                sku: row.get(0)?,
                name: row.get(1)?,
                brand: row.get(2)?,
                main_category: row.get(3)?,
                category: row.get(4)?,
                quantity: row.get(5)?,
                url: row.get(6)?,
                img_url,
                title: row.get(8)?,
                descriptions: row.get(9)?,
                keywords: row.get(10)?,
                search_keywords: row.get(11)?,
                details: row.get(12)?,
                meta_status: row.get(13)?,
                details_status: row.get(14)?,
                target_keyword: row.get(15)?,
                draft_title: row.get(16)?,
                draft_descriptions: row.get(17)?,
                draft_search_keywords: row.get(18)?,
                draft_details: row.get(19)?,
                badge: MetaBadge::Eksik,       // aşağıda hesaplanır
                details_badge: MetaBadge::Eksik, // aşağıda hesaplanır
                overall: OverallStatus::Eksik,   // aşağıda hesaplanır
                image_count: gallery.len(),
                image_badge: MetaBadge::Eksik, // aşağıda hesaplanır
                gallery,
                image_check,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
        other => format!("Ürün okunamadı: {other}"),
    })
    .map(|mut d| {
        let kw = d.target_keyword.as_deref().unwrap_or("");
        d.badge = meta_badge(&MetaInput {
            title: d.draft_title.as_deref().unwrap_or(d.title.as_deref().unwrap_or("")),
            descriptions: d
                .draft_descriptions
                .as_deref()
                .unwrap_or(d.descriptions.as_deref().unwrap_or("")),
            target_keyword: kw,
            meta_done: d.meta_status == "done",
        });
        let details_html = d.draft_details.as_deref().unwrap_or(d.details.as_deref().unwrap_or(""));
        d.details_badge = details_badge(details_html, kw, d.details_status == "done");
        d.overall = overall_status(
            d.badge,
            d.details_badge,
            d.meta_status == "done",
            d.details_status == "done",
        );
        // Görsel skoru: sayı + (varsa) cache'lenmiş boyut sonucu.
        let all_dims_ok = d.image_check.as_ref().map(|c| !c.is_empty() && c.iter().all(|x| x.ok));
        d.image_badge = image_badge(d.image_count, all_dims_ok);
        d
    })
}

#[tauri::command]
pub fn get_product(state: State<'_, AppState>, sku: String) -> Result<ProductDetail, String> {
    let conn = state.conn.lock().unwrap();
    read_detail(&conn, &sku)
}

fn ensure_seo_row(conn: &Connection, sku: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO seo_status (sku, meta_status, details_status, updated_at)
         VALUES (?1, 'pending', 'pending', ?2)
         ON CONFLICT(sku) DO NOTHING",
        params![sku, now_str()],
    )
    .map_err(|e| format!("SEO durumu oluşturulamadı: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn set_target_keyword(state: State<'_, AppState>, sku: String, kw: String) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    conn.execute(
        "UPDATE seo_status SET target_keyword = ?2, updated_at = ?3 WHERE sku = ?1",
        params![sku, kw.trim(), now_str()],
    )
    .map_err(|e| format!("Hedef kelime kaydedilemedi: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn save_meta_draft(
    state: State<'_, AppState>,
    sku: String,
    title: String,
    descriptions: String,
    search_keywords: String,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    conn.execute(
        "UPDATE seo_status SET draft_title = ?2, draft_descriptions = ?3,
                draft_search_keywords = ?4, updated_at = ?5
         WHERE sku = ?1",
        params![sku, title, descriptions, search_keywords, now_str()],
    )
    .map_err(|e| format!("Taslak kaydedilemedi: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn mark_meta_done(state: State<'_, AppState>, sku: String) -> Result<String, String> {
    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    let current: String = conn
        .query_row("SELECT meta_status FROM seo_status WHERE sku = ?1", [&sku], |r| r.get(0))
        .map_err(|e| format!("SEO durumu okunamadı: {e}"))?;
    let next = if current == "done" { "pending" } else { "done" };
    conn.execute(
        "UPDATE seo_status SET meta_status = ?2, updated_at = ?3 WHERE sku = ?1",
        params![sku, next, now_str()],
    )
    .map_err(|e| format!("SEO durumu güncellenemedi: {e}"))?;
    Ok(next.to_string())
}

#[tauri::command]
pub fn mark_details_done(state: State<'_, AppState>, sku: String) -> Result<String, String> {
    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    let current: String = conn
        .query_row("SELECT details_status FROM seo_status WHERE sku = ?1", [&sku], |r| r.get(0))
        .map_err(|e| format!("SEO durumu okunamadı: {e}"))?;
    let next = if current == "done" { "pending" } else { "done" };
    conn.execute(
        "UPDATE seo_status SET details_status = ?2, updated_at = ?3 WHERE sku = ?1",
        params![sku, next, now_str()],
    )
    .map_err(|e| format!("SEO durumu güncellenemedi: {e}"))?;
    Ok(next.to_string())
}

/// Faz 2: Gemini ile meta üretir, sonucu taslak alanlarına + hedef kelimeye yazar.
/// Not: SQLite kilidi await'lerin ötesine taşınmaz (Send güvenliği için bloklarda tutulur).
#[tauri::command]
pub async fn generate_meta(state: State<'_, AppState>, sku: String) -> Result<ProductDetail, String> {
    let (name, brand, category, main_category, target_keyword, research_json, api_key) = {
        let conn = state.conn.lock().unwrap();
        let key = db::get_setting(&conn, "gemini_api_key")?.unwrap_or_default();
        let row = conn
            .query_row(
                "SELECT p.name, p.brand, p.category, p.main_category,
                        COALESCE(s.target_keyword,''), s.research_json
                 FROM products p LEFT JOIN seo_status s ON s.sku = p.sku
                 WHERE p.sku = ?1",
                [&sku],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, Option<String>>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, Option<String>>(5)?,
                    ))
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
                other => format!("Ürün okunamadı: {other}"),
            })?;
        (row.0, row.1, row.2, row.3, row.4, row.5, key)
    };

    let insights = parse_insights(research_json.as_deref());
    let kw = target_keyword.trim();
    let ctx = gemini::ProductContext {
        name: &name,
        brand: brand.as_deref(),
        category: category.as_deref(),
        main_category: main_category.as_deref(),
        target_keyword: if kw.is_empty() { None } else { Some(kw) },
        insights: insights.as_ref().filter(|i| i.has_data()),
    };
    let meta = gemini::generate_meta(&api_key, &ctx).await?;

    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    conn.execute(
        "UPDATE seo_status SET target_keyword = ?2, draft_title = ?3, draft_descriptions = ?4,
                draft_keywords = ?5, draft_search_keywords = ?6, updated_at = ?7
         WHERE sku = ?1",
        params![
            sku,
            meta.target_keyword.trim(),
            meta.title.trim(),
            meta.descriptions.trim(),
            meta.keywords.trim(),
            meta.search_keywords.trim(),
            now_str(),
        ],
    )
    .map_err(|e| format!("Üretilen meta kaydedilemedi: {e}"))?;
    read_detail(&conn, &sku)
}

/// Faz 3: details HTML'ini yapıyı koruyarak yeniden üretir, taslağa yazar.
#[tauri::command]
pub async fn generate_details(
    state: State<'_, AppState>,
    sku: String,
) -> Result<ProductDetail, String> {
    let (name, brand, category, main_category, details_html, keyword, research_json, gallery, api_key) = {
        let conn = state.conn.lock().unwrap();
        let key = db::get_setting(&conn, "gemini_api_key")?.unwrap_or_default();
        conn.query_row(
            "SELECT p.name, p.brand, p.category, p.main_category,
                    COALESCE(s.draft_details, p.details), COALESCE(s.target_keyword,''),
                    s.research_json, p.img_url, p.picture2, p.picture3, p.picture4
             FROM products p LEFT JOIN seo_status s ON s.sku = p.sku
             WHERE p.sku = ?1",
            [&sku],
            |r| {
                let gallery: Vec<String> = [
                    r.get::<_, Option<String>>(7)?,
                    r.get::<_, Option<String>>(8)?,
                    r.get::<_, Option<String>>(9)?,
                    r.get::<_, Option<String>>(10)?,
                ]
                .into_iter()
                .filter_map(|u| u.filter(|s| !s.trim().is_empty()))
                .collect();
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, Option<String>>(6)?,
                    gallery,
                ))
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
            other => format!("Ürün okunamadı: {other}"),
        })
        .map(|t| (t.0, t.1, t.2, t.3, t.4.unwrap_or_default(), t.5, t.6, t.7, key))?
    };

    // Görsel kapısı: en az 3 galeri görseli (backend savunma; UI de engeller).
    if gallery.len() < 3 {
        return Err(format!(
            "En az 3 ürün görseli gerekli — şu an {}/4. Ürüne görsel ekleyin.",
            gallery.len()
        ));
    }

    let insights = parse_insights(research_json.as_deref());
    let ctx = gemini::ProductContext {
        name: &name,
        brand: brand.as_deref(),
        category: category.as_deref(),
        main_category: main_category.as_deref(),
        target_keyword: None, // details zaten `keyword` argümanını kullanır
        insights: insights.as_ref().filter(|i| i.has_data()),
    };
    // Açıklama akışı:
    //  1) İçerik yok / yeniden yazılabilir metin yok → sıfırdan semantik HTML (galeri görselleri).
    //  2) Düzenli yapı → OPTIMIZE: metin iyileştirilir + yapı semantikleştirilir + anlamlı alt eklenir.
    //  3) Düzensiz yapı → eski güvenli yol (yapıyı aynen koruyarak yalnızca metni yeniden yaz).
    let new_html = if details_html.trim().is_empty() || !gemini::has_rewritable_content(&details_html)
    {
        gemini::generate_details_scratch(&api_key, &ctx, &gallery, &keyword).await?
    } else {
        match gemini::optimize_details(&api_key, &ctx, &details_html, &keyword).await? {
            Some(html) => html,
            None => gemini::generate_details(&api_key, &ctx, &details_html, &keyword).await?,
        }
    };

    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    conn.execute(
        "UPDATE seo_status SET draft_details = ?2, updated_at = ?3 WHERE sku = ?1",
        params![sku, new_html, now_str()],
    )
    .map_err(|e| format!("Üretilen açıklama kaydedilemedi: {e}"))?;
    read_detail(&conn, &sku)
}

/// Faz 7: galeri görsellerinin 1:1 + çözünürlük kontrolü (async, `?revision` parmak iziyle cache'li).
#[tauri::command]
pub async fn check_images(state: State<'_, AppState>, sku: String) -> Result<Vec<ImageCheck>, String> {
    let (gallery, cached_json, cached_fp) = {
        let conn = state.conn.lock().unwrap();
        conn.query_row(
            "SELECT p.img_url, p.picture2, p.picture3, p.picture4, s.image_check_json, s.image_check_fp
             FROM products p LEFT JOIN seo_status s ON s.sku = p.sku
             WHERE p.sku = ?1",
            [&sku],
            |r| {
                let g: Vec<String> = [
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ]
                .into_iter()
                .filter_map(|u| u.filter(|s| !s.trim().is_empty()))
                .collect();
                Ok((g, r.get::<_, Option<String>>(4)?, r.get::<_, Option<String>>(5)?))
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
            other => format!("Ürün okunamadı: {other}"),
        })?
    };

    if gallery.is_empty() {
        return Ok(Vec::new());
    }
    let fp = gallery.join("|");
    // Görsel URL'leri (revision dahil) değişmemişse cache'i döndür
    if cached_fp.as_deref() == Some(fp.as_str()) {
        if let Some(cached) = cached_json.as_deref().and_then(|j| serde_json::from_str::<Vec<ImageCheck>>(j).ok())
        {
            return Ok(cached);
        }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;
    let checks = images::check_dimensions(&client, &gallery).await;
    let json = serde_json::to_string(&checks).unwrap_or_default();
    {
        let conn = state.conn.lock().unwrap();
        ensure_seo_row(&conn, &sku)?;
        conn.execute(
            "UPDATE seo_status SET image_check_json = ?2, image_check_fp = ?3, updated_at = ?4 WHERE sku = ?1",
            params![sku, json, fp, now_str()],
        )
        .map_err(|e| format!("Görsel kontrolü kaydedilemedi: {e}"))?;
    }
    Ok(checks)
}

/// `research_json` metnini SeoInsights'e çözer; bozuk/boşsa None.
fn parse_insights(json: Option<&str>) -> Option<SeoInsights> {
    let s = json?.trim();
    if s.is_empty() {
        return None;
    }
    serde_json::from_str::<SeoInsights>(s).ok()
}

/// Ürün adından tohum kelime türetir (ilk `n` anlamlı sözcük).
fn first_words(name: &str, n: usize) -> String {
    name.split_whitespace().take(n).collect::<Vec<_>>().join(" ")
}

/// URL'den alan adını (www'suz) çıkarır.
fn host_of(url: &str) -> Option<String> {
    reqwest::Url::parse(url)
        .ok()?
        .host_str()
        .map(|h| h.trim_start_matches("www.").to_string())
}

/// Faz 4: Kontrollü SEO araştırması — Ahrefs (keyword ideas + difficulty).
/// Tohum kelime: verilen `seed` → yoksa onaylı hedef kelime → kategori → ürün adının ilk 4 sözcüğü.
/// Sonuç `seo_status.research_json`'a kaydedilir ve panele döner. GSC/Trends Faz 5/6'da eklenir.
#[tauri::command]
pub async fn research_seo(
    state: State<'_, AppState>,
    sku: String,
    seed: Option<String>,
) -> Result<SeoInsights, String> {
    let (name, category, url, target_kw, capsolver_key, country, gsc_json, gsc_site) = {
        let conn = state.conn.lock().unwrap();
        let capsolver_key = db::get_setting(&conn, "capsolver_api_key")?.unwrap_or_default();
        let country = db::get_setting(&conn, "seo_country")?.unwrap_or_else(|| "tr".to_string());
        let gsc_json = db::get_setting(&conn, "gsc_service_account_json")?.unwrap_or_default();
        let gsc_site = db::get_setting(&conn, "gsc_site_url")?.unwrap_or_default();
        let (name, category, url, target_kw) = conn
            .query_row(
                "SELECT p.name, p.category, p.url, COALESCE(s.target_keyword,'')
                 FROM products p LEFT JOIN seo_status s ON s.sku = p.sku
                 WHERE p.sku = ?1",
                [&sku],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
                other => format!("Ürün okunamadı: {other}"),
            })?;
        (name, category, url, target_kw, capsolver_key, country, gsc_json, gsc_site)
    };

    let has_capsolver = !capsolver_key.trim().is_empty();
    let has_gsc = !gsc_json.trim().is_empty() && !gsc_site.trim().is_empty();
    if !has_capsolver && !has_gsc {
        return Err(
            "Araştırma için Ayarlar'dan CapSolver anahtarı ve/veya GSC service-account + mülk ekleyin."
                .to_string(),
        );
    }

    // Tohum kelime seçimi (kontrollü: kullanıcı panelde düzenleyebilir)
    let seed = seed
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            let t = target_kw.trim();
            if t.is_empty() { None } else { Some(t.to_string()) }
        })
        .or_else(|| category.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(String::from))
        .unwrap_or_else(|| first_words(&name, 4));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .cookie_store(true) // Google Trends explore için NID çerezi gerekir
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;

    let mut ins = SeoInsights {
        seed: seed.clone(),
        fetched_at: now_str(),
        ..Default::default()
    };

    let domain = url.as_deref().and_then(host_of);

    // Ahrefs (CapSolver varsa): keyword ideas + difficulty + domain overview — hepsi eşzamanlı.
    if has_capsolver {
        let overview_fut = async {
            match &domain {
                Some(d) => Some(seo_data::ahrefs::backlinks_overview(&client, &capsolver_key, d).await),
                None => None,
            }
        };
        let (ideas_res, kd_res, ov_res) = tokio::join!(
            seo_data::ahrefs::keyword_ideas(&client, &capsolver_key, &seed, &country),
            seo_data::ahrefs::keyword_difficulty(&client, &capsolver_key, &seed, &country),
            overview_fut,
        );
        match ideas_res {
            Ok(mut cands) => {
                cands.sort_by(|a, b| b.volume.cmp(&a.volume));
                ins.target_candidates = cands;
            }
            Err(e) => ins.notes.push(format!("Anahtar kelime fikirleri alınamadı: {e}")),
        }
        match kd_res {
            Ok(d) => ins.seed_difficulty = Some(d),
            Err(e) => ins.notes.push(format!("Zorluk verisi alınamadı: {e}")),
        }
        if let Some(ov) = ov_res {
            match ov {
                Ok(d) => ins.domain = Some(d),
                Err(e) => ins.notes.push(format!("Alan (backlink) özeti alınamadı: {e}")),
            }
        }
    }

    // Google Trends — hedef kelimeye ilgili sorgular (explore→relatedsearches) DEVRE DIŞI:
    // Google'ın anti-bot koruması API'yi HTTP 429 ile blokluyor (tarayıcı consent çerezi gerekiyor).
    // Kod `seo_data::trends`'te korunuyor; keyword-relevant ihtiyaç Ahrefs fikirleri + GSC sorgularıyla
    // zaten karşılanıyor. İleride güvenilir bir yol bulunursa yeniden etkinleştirilebilir.

    // GSC gerçek sorgular (SA + mülk varsa ve üründe URL varsa).
    if has_gsc {
        match url.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
            Some(page) => {
                match seo_data::gsc::search_queries(&client, &gsc_json, gsc_site.trim(), page, 90, 25)
                    .await
                {
                    Ok(q) => ins.gsc_queries = q,
                    Err(e) => ins.notes.push(format!("GSC sorguları alınamadı: {e}")),
                }
            }
            None => ins.notes.push("Bu üründe URL yok, GSC sorguları atlandı.".to_string()),
        }
    }

    if !ins.has_data() {
        let detail = ins.notes.join(" ");
        return Err(if detail.is_empty() {
            "Araştırma verisi alınamadı.".to_string()
        } else {
            detail
        });
    }

    // Sonucu kaydet (üretim prompt'ları buradan okur)
    let json = serde_json::to_string(&ins).map_err(|e| format!("Araştırma serialize edilemedi: {e}"))?;
    {
        let conn = state.conn.lock().unwrap();
        ensure_seo_row(&conn, &sku)?;
        conn.execute(
            "UPDATE seo_status SET research_json = ?2, updated_at = ?3 WHERE sku = ?1",
            params![sku, json, now_str()],
        )
        .map_err(|e| format!("Araştırma kaydedilemedi: {e}"))?;
    }
    Ok(ins)
}

#[tauri::command]
pub async fn test_capsolver_key(key: String) -> Result<String, String> {
    seo_data::ahrefs::test_key(&key).await
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let conn = state.conn.lock().unwrap();
    Ok(Settings {
        feed_url: db::feed_url(&conn)?,
        gemini_api_key: db::get_setting(&conn, "gemini_api_key")?.unwrap_or_default(),
        capsolver_api_key: db::get_setting(&conn, "capsolver_api_key")?.unwrap_or_default(),
        seo_country: db::get_setting(&conn, "seo_country")?.unwrap_or_else(|| "tr".to_string()),
        gsc_site_url: db::get_setting(&conn, "gsc_site_url")?.unwrap_or_default(),
        gsc_client_email: db::get_setting(&conn, "gsc_service_account_json")?
            .as_deref()
            .and_then(seo_data::gsc::client_email_of)
            .unwrap_or_default(),
        theme: db::get_setting(&conn, "theme")?,
        last_backup_at: db::get_setting(&conn, "last_backup_at")?,
    })
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, AppState>,
    feed_url: String,
    gemini_api_key: String,
    capsolver_api_key: String,
    seo_country: String,
    gsc_site_url: String,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let url = feed_url.trim();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Geçerli bir URL girin (http/https).".to_string());
    }
    db::set_setting(&conn, "feed_url", url)?;
    db::set_setting(&conn, "gemini_api_key", gemini_api_key.trim())?;
    db::set_setting(&conn, "capsolver_api_key", capsolver_api_key.trim())?;
    let country = seo_country.trim().to_lowercase();
    let country = if country.is_empty() { "tr".to_string() } else { country };
    db::set_setting(&conn, "seo_country", &country)?;
    db::set_setting(&conn, "gsc_site_url", gsc_site_url.trim())?;
    Ok(())
}

/// Faz 5: seçilen service-account JSON dosyasını okur, doğrular + saklar. UI'ya client_email döner.
#[tauri::command]
pub fn set_gsc_service_account(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Dosya okunamadı: {e}"))?;
    let email = seo_data::gsc::validate_json(&json)?;
    let conn = state.conn.lock().unwrap();
    db::set_setting(&conn, "gsc_service_account_json", json.trim())?;
    Ok(email)
}

/// Faz 5: yüklü SA'yı kaldırır (GSC'yi devre dışı bırakır).
#[tauri::command]
pub fn clear_gsc_service_account(state: State<'_, AppState>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    db::set_setting(&conn, "gsc_service_account_json", "")?;
    Ok(())
}

/// Faz 5: Ayarlarda "Bağlantıyı test et" — token al + mülk erişimini doğrula.
#[tauri::command]
pub async fn test_gsc_credentials(state: State<'_, AppState>) -> Result<String, String> {
    let (json, site) = {
        let conn = state.conn.lock().unwrap();
        (
            db::get_setting(&conn, "gsc_service_account_json")?.unwrap_or_default(),
            db::get_setting(&conn, "gsc_site_url")?.unwrap_or_default(),
        )
    };
    if json.trim().is_empty() {
        return Err("Önce bir service-account JSON dosyası yükleyin.".to_string());
    }
    seo_data::gsc::test(&json, &site).await
}

#[tauri::command]
pub fn set_theme(state: State<'_, AppState>, theme: String) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    db::set_setting(&conn, "theme", &theme)
}

#[tauri::command]
pub async fn test_feed_url(url: String) -> Result<i64, String> {
    let u = url.trim();
    if !u.starts_with("http://") && !u.starts_with("https://") {
        return Err("Geçerli bir URL girin (http/https).".to_string());
    }
    let items = feed::fetch_and_parse(u).await?;
    Ok(items.len() as i64)
}

#[tauri::command]
pub async fn test_gemini_key(key: String) -> Result<String, String> {
    gemini::test_key(&key).await
}

#[tauri::command]
pub fn export_db(state: State<'_, AppState>, path: String, format: String) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    match format.as_str() {
        "db" => {
            let mut dest = Connection::open(&path)
                .map_err(|e| format!("Hedef dosya açılamadı: {e}"))?;
            let backup = rusqlite::backup::Backup::new(&conn, &mut dest)
                .map_err(|e| format!("Yedekleme başlatılamadı: {e}"))?;
            backup
                .run_to_completion(100, std::time::Duration::from_millis(5), None)
                .map_err(|e| format!("Yedekleme tamamlanamadı: {e}"))?;
        }
        "json" => {
            let json = export_json(&conn)?;
            std::fs::write(&path, json).map_err(|e| format!("Dosya yazılamadı: {e}"))?;
        }
        other => return Err(format!("Bilinmeyen format: {other}")),
    }
    db::set_setting(&conn, "last_backup_at", &now_str())?;
    Ok(())
}

fn export_json(conn: &Connection) -> Result<String, String> {
    use serde_json::{json, Value};
    fn dump(conn: &Connection, table: &str, cols: &[&str]) -> Result<Vec<Value>, String> {
        let sql = format!("SELECT {} FROM {}", cols.join(", "), table);
        let mut stmt = conn.prepare(&sql).map_err(|e| format!("{table} okunamadı: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                let mut obj = serde_json::Map::new();
                for (i, col) in cols.iter().enumerate() {
                    let v: Option<rusqlite::types::Value> = row.get(i)?;
                    let jv = match v {
                        Some(rusqlite::types::Value::Text(s)) => Value::String(s),
                        Some(rusqlite::types::Value::Integer(n)) => Value::from(n),
                        Some(rusqlite::types::Value::Real(f)) => Value::from(f),
                        _ => Value::Null,
                    };
                    obj.insert((*col).to_string(), jv);
                }
                Ok(Value::Object(obj))
            })
            .map_err(|e| format!("{table} okunamadı: {e}"))?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }
    let products = dump(
        conn,
        "products",
        &[
            "sku", "id", "name", "brand", "main_category", "category", "quantity", "url",
            "img_url", "title", "descriptions", "keywords", "search_keywords", "details",
            "last_synced_at",
        ],
    )?;
    let seo = dump(
        conn,
        "seo_status",
        &[
            "sku", "meta_status", "details_status", "target_keyword", "draft_title",
            "draft_descriptions", "draft_keywords", "draft_search_keywords", "updated_at",
        ],
    )?;
    let log = dump(
        conn,
        "sync_log",
        &["run_at", "active", "added", "updated", "deleted", "duplicate_skipped"],
    )?;
    let settings = dump(conn, "settings", &["key", "value"])?;
    let root = json!({
        "app": "seo-yoneticisi",
        "exported_at": now_str(),
        "products": products,
        "seo_status": seo,
        "sync_log": log,
        "settings": settings,
    });
    serde_json::to_string_pretty(&root).map_err(|e| format!("JSON oluşturulamadı: {e}"))
}

#[tauri::command]
pub fn import_db(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let mut conn = state.conn.lock().unwrap();
    let lower = path.to_lowercase();
    if lower.ends_with(".json") {
        let text = std::fs::read_to_string(&path).map_err(|e| format!("Dosya okunamadı: {e}"))?;
        import_json(&mut conn, &text)
    } else {
        // .db: kaynak dosyadan mevcut bağlantının üzerine geri yükle
        let src = Connection::open(&path).map_err(|e| format!("Yedek dosyası açılamadı: {e}"))?;
        src.query_row("SELECT COUNT(*) FROM sqlite_master WHERE name='products'", [], |r| {
            r.get::<_, i64>(0)
        })
        .ok()
        .filter(|n| *n > 0)
        .ok_or("Bu dosya geçerli bir SEO Yöneticisi yedeği değil.")?;
        {
            let backup = rusqlite::backup::Backup::new(&src, &mut conn)
                .map_err(|e| format!("Geri yükleme başlatılamadı: {e}"))?;
            backup
                .run_to_completion(100, std::time::Duration::from_millis(5), None)
                .map_err(|e| format!("Geri yükleme tamamlanamadı: {e}"))?;
        }
        db::init(&conn)?; // eksik tablo/pragma varsa tamamla
        Ok(())
    }
}

fn import_json(conn: &mut Connection, text: &str) -> Result<(), String> {
    use serde_json::Value;
    let root: Value = serde_json::from_str(text).map_err(|e| format!("JSON çözümlenemedi: {e}"))?;
    let obj = root.as_object().ok_or("Beklenmeyen JSON biçimi.")?;
    if !obj.contains_key("products") {
        return Err("Bu dosya geçerli bir SEO Yöneticisi yedeği değil.".to_string());
    }
    let tx = conn.transaction().map_err(|e| format!("İşlem başlatılamadı: {e}"))?;
    tx.execute_batch(
        "DELETE FROM seo_status; DELETE FROM products; DELETE FROM sync_log; DELETE FROM settings;",
    )
    .map_err(|e| format!("Mevcut veriler temizlenemedi: {e}"))?;

    fn s(v: &Value, key: &str) -> Option<String> {
        v.get(key).and_then(|x| x.as_str()).map(String::from)
    }
    fn i(v: &Value, key: &str) -> Option<i64> {
        v.get(key).and_then(|x| x.as_i64())
    }
    let arr = |key: &str| -> Vec<Value> {
        obj.get(key).and_then(|x| x.as_array()).cloned().unwrap_or_default()
    };

    for p in arr("products") {
        tx.execute(
            "INSERT OR REPLACE INTO products (sku, id, name, brand, main_category, category,
               quantity, url, img_url, title, descriptions, keywords, search_keywords, details, last_synced_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![
                s(&p, "sku"), s(&p, "id"), s(&p, "name").unwrap_or_default(), s(&p, "brand"),
                s(&p, "main_category"), s(&p, "category"), i(&p, "quantity"), s(&p, "url"),
                s(&p, "img_url"), s(&p, "title"), s(&p, "descriptions"), s(&p, "keywords"),
                s(&p, "search_keywords"), s(&p, "details"), s(&p, "last_synced_at"),
            ],
        )
        .map_err(|e| format!("Ürün geri yüklenemedi: {e}"))?;
    }
    for r in arr("seo_status") {
        tx.execute(
            "INSERT OR REPLACE INTO seo_status (sku, meta_status, details_status, target_keyword,
               draft_title, draft_descriptions, draft_keywords, draft_search_keywords, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                s(&r, "sku"),
                s(&r, "meta_status").unwrap_or_else(|| "pending".into()),
                s(&r, "details_status").unwrap_or_else(|| "pending".into()),
                s(&r, "target_keyword"), s(&r, "draft_title"), s(&r, "draft_descriptions"),
                s(&r, "draft_keywords"), s(&r, "draft_search_keywords"), s(&r, "updated_at"),
            ],
        )
        .map_err(|e| format!("SEO durumu geri yüklenemedi: {e}"))?;
    }
    for l in arr("sync_log") {
        tx.execute(
            "INSERT INTO sync_log (run_at, active, added, updated, deleted, duplicate_skipped)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                s(&l, "run_at"), i(&l, "active"), i(&l, "added"), i(&l, "updated"),
                i(&l, "deleted"), i(&l, "duplicate_skipped"),
            ],
        )
        .map_err(|e| format!("Senkron geçmişi geri yüklenemedi: {e}"))?;
    }
    for kv in arr("settings") {
        if let (Some(k), Some(v)) = (s(&kv, "key"), s(&kv, "value")) {
            tx.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                params![k, v],
            )
            .map_err(|e| format!("Ayar geri yüklenemedi: {e}"))?;
        }
    }
    tx.commit().map_err(|e| format!("İşlem tamamlanamadı: {e}"))?;
    Ok(())
}
