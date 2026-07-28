use seo_core::ideasoft;
use seo_core::images::{self, ImageCheck};
use seo_core::seo_data::{self, SeoInsights};
use seo_core::validation::{
    details_badge, image_badge, meta_badge, overall_status, MetaBadge, MetaInput, OverallInput,
    OverallStatus,
};
use seo_core::{db, feed, gemini, history, opportunity, sync};
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
    pub tech_done: bool,
    pub image_count: usize,
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
    pub draft_keywords: Option<String>,
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
    // Faz 8: teknik özellik tablosu
    pub tech_source_text: Option<String>,
    pub tech_specs: Option<Vec<gemini::TechGroup>>,
    pub tech_status: String,
    pub tech_badge: MetaBadge,
    /// Önceki sürümlerin hafif özeti (en yeni başta).
    pub tech_history: Vec<TechVersionMeta>,
    // Faz 9: IdeaSoft
    pub ideasoft_pushed_at: Option<String>,
    /// IdeaSoft'un kendi SEO kural skoru (yalnızca liste ucunda dolu gelir, cache'lenir).
    pub ideasoft_seo_rule: Option<i64>,
    /// İçeriği hangi Gemini modelinin ürettiği. Zincir kotaya takıldıkça alt modellere
    /// düşüyor; kullanıcı bunu görüp limitler yenilendiğinde yeniden üretmeye karar verebilir.
    pub meta_model: Option<String>,
    pub details_model: Option<String>,
    pub tech_model: Option<String>,
    /// Yeniden üretimden önceki hâller (en yeni başta) — hafif özet.
    pub meta_history: Vec<MetaVersionMeta>,
    pub details_history: Vec<DetailsVersionMeta>,
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
    /// Faz 9: IdeaSoft modülü (boşsa modül kapalı, kopyala-yapıştır akışı sürer).
    pub ideasoft_domain: String,
    pub ideasoft_token: String,
    pub ideasoft_active: bool,
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
    tech_status: String,
    tech_specs_json: Option<String>,
    image_count: usize,
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
                    s.target_keyword, s.draft_title, s.draft_descriptions, s.draft_details,
                    COALESCE(s.tech_status,'pending'), s.tech_specs_json,
                    p.picture2, p.picture3, p.picture4
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
                tech_status: row.get(13)?,
                tech_specs_json: row.get(14)?,
                image_count: [
                    row.get::<_, Option<String>>(3)?,   // img_url
                    row.get::<_, Option<String>>(15)?,  // picture2
                    row.get::<_, Option<String>>(16)?,  // picture3
                    row.get::<_, Option<String>>(17)?,  // picture4
                ]
                .into_iter()
                .filter(|u| u.as_deref().map_or(false, |x| !x.trim().is_empty()))
                .count(),
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
        let has_tech = r
            .tech_specs_json
            .as_deref()
            .map(str::trim)
            .map_or(false, |j| !j.is_empty() && j != "[]");
        let overall = overall_status(&OverallInput {
            meta,
            details,
            meta_done,
            details_done,
            tech_done: r.tech_status == "done",
            has_tech,
            image_count: r.image_count,
        });
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
                tech_done: r.tech_status == "done",
                image_count: r.image_count,
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
                s.draft_details, s.draft_keywords, p.picture2, p.picture3, p.picture4, s.image_check_json,
                s.tech_source_text, s.tech_specs_json, COALESCE(s.tech_status,'pending'),
                s.tech_history_json, s.ideasoft_pushed_at, s.ideasoft_seo_rule,
                s.meta_model, s.details_model, s.tech_model,
                s.meta_history_json, s.details_history_json
         FROM products p LEFT JOIN seo_status s ON s.sku = p.sku
         WHERE p.sku = ?1",
        [&sku],
        |row| {
            let img_url: Option<String> = row.get(7)?;
            let draft_keywords: Option<String> = row.get(20)?;
            let picture2: Option<String> = row.get(21)?;
            let picture3: Option<String> = row.get(22)?;
            let picture4: Option<String> = row.get(23)?;
            let check_json: Option<String> = row.get(24)?;
            let tech_source_text: Option<String> = row.get(25)?;
            let tech_specs: Option<Vec<gemini::TechGroup>> = row
                .get::<_, Option<String>>(26)?
                .as_deref()
                .and_then(|j| serde_json::from_str(j).ok());
            let tech_status: String = row.get(27)?;
            let tech_history: Vec<TechVersionMeta> =
                history::parse::<TechVersion>(row.get::<_, Option<String>>(28)?.as_deref())
                .into_iter()
                .map(|v| TechVersionMeta {
                    at: v.at,
                    rows: v.groups.iter().map(|g| g.rows.len()).sum(),
                    groups: v.groups.len(),
                })
                .collect();
            let meta_history: Vec<MetaVersionMeta> =
                history::parse::<MetaVersion>(row.get::<_, Option<String>>(34)?.as_deref())
                    .into_iter()
                    .map(|v| MetaVersionMeta { at: v.at, title: v.title, model: v.model })
                    .collect();
            let details_history: Vec<DetailsVersionMeta> =
                history::parse::<DetailsVersion>(row.get::<_, Option<String>>(35)?.as_deref())
                    .into_iter()
                    .map(|v| DetailsVersionMeta {
                        at: v.at,
                        words: seo_core::validation::word_count(&v.html),
                        model: v.model,
                    })
                    .collect();
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
                draft_keywords,
                badge: MetaBadge::Eksik,       // aşağıda hesaplanır
                details_badge: MetaBadge::Eksik, // aşağıda hesaplanır
                overall: OverallStatus::Eksik,   // aşağıda hesaplanır
                image_count: gallery.len(),
                image_badge: MetaBadge::Eksik, // aşağıda hesaplanır
                gallery,
                image_check,
                tech_badge: if tech_status == "done" {
                    MetaBadge::Tamamlandi
                } else if tech_specs.as_ref().map_or(false, |g| !g.is_empty()) {
                    MetaBadge::Uygun
                } else {
                    MetaBadge::Eksik
                },
                tech_source_text,
                tech_specs,
                tech_status,
                tech_history,
                ideasoft_pushed_at: row.get(29)?,
                ideasoft_seo_rule: row.get(30)?,
                meta_model: row.get(31)?,
                details_model: row.get(32)?,
                tech_model: row.get(33)?,
                meta_history,
                details_history,
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
        d.overall = overall_status(&OverallInput {
            meta: d.badge,
            details: d.details_badge,
            meta_done: d.meta_status == "done",
            details_done: d.details_status == "done",
            tech_done: d.tech_status == "done",
            has_tech: d.tech_specs.as_ref().map_or(false, |g| !g.is_empty()),
            image_count: d.image_count,
        });
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
    let produced = gemini::generate_meta(&api_key, &ctx).await?;
    let (meta, model) = (produced.value, produced.model);

    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;

    // Yeniden üretimden ÖNCEKİ hâli geçmişe al — elle düzeltilmiş bir başlık geri dönüşsüz
    // kaybolmasın. Boşsa (ilk üretim) veya sonuç aynıysa kayıt açma.
    let history_json = snapshot_meta(&conn, &sku, &meta)?;

    conn.execute(
        "UPDATE seo_status SET target_keyword = ?2, draft_title = ?3, draft_descriptions = ?4,
                draft_keywords = ?5, draft_search_keywords = ?6, updated_at = ?7, meta_model = ?8,
                meta_history_json = COALESCE(?9, meta_history_json)
         WHERE sku = ?1",
        params![
            sku,
            meta.target_keyword.trim(),
            meta.title.trim(),
            meta.descriptions.trim(),
            meta.keywords.trim(),
            meta.search_keywords.trim(),
            now_str(),
            model,
            history_json,
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
    let (new_html, model) = if details_html.trim().is_empty()
        || !gemini::has_rewritable_content(&details_html)
    {
        let p = gemini::generate_details_scratch(&api_key, &ctx, &gallery, &keyword).await?;
        (p.value, p.model)
    } else {
        let opt = gemini::optimize_details(&api_key, &ctx, &details_html, &keyword).await?;
        match opt.value {
            Some(html) => (html, opt.model),
            // Yapı beklenmedik → yapı-koruyan eski yol. Modeli o çağrıdan al.
            None => {
                let p = gemini::generate_details(&api_key, &ctx, &details_html, &keyword).await?;
                (p.value, p.model)
            }
        }
    };

    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    let history_json = snapshot_details(&conn, &sku, &new_html)?;
    conn.execute(
        "UPDATE seo_status SET draft_details = ?2, updated_at = ?3, details_model = ?4,
                details_history_json = COALESCE(?5, details_history_json)
         WHERE sku = ?1",
        params![sku, new_html, now_str(), model, history_json],
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

// ---- Faz 9: IdeaSoft gönderim modülü (opsiyonel) ----

#[derive(Debug, Serialize)]
pub struct IdeasoftPreview {
    pub id: i64,
    pub remote: ideasoft::RemoteProduct,
    /// Gönderilecek değerler (yalnızca seçilen parçalar).
    pub local: serde_json::Value,
}

/// Ayar + yerel içerikleri toplar; `parts` için gönderilecek `LocalContent` üretir.
fn ideasoft_local(conn: &Connection, sku: &str) -> Result<(String, String, ideasoft::LocalContent), String> {
    let domain = db::get_setting(conn, "ideasoft_domain")?.unwrap_or_default();
    let token = db::get_setting(conn, "ideasoft_token")?.unwrap_or_default();
    if domain.trim().is_empty() || token.trim().is_empty() {
        return Err("IdeaSoft bağlantısı ayarlı değil. Ayarlar'dan domain ve token girin.".to_string());
    }
    let d = read_detail(conn, sku)?;
    let tech_html = d
        .tech_specs
        .as_ref()
        .filter(|g| !g.is_empty())
        .map(|g| gemini::assemble_tech_html(g))
        .unwrap_or_default();
    let local = ideasoft::LocalContent {
        page_title: d.draft_title.clone().or(d.title.clone()).unwrap_or_default(),
        meta_description: d.draft_descriptions.clone().or(d.descriptions.clone()).unwrap_or_default(),
        // Üretilen anahtar kelimeler (draft) önceliklidir; yoksa feed'deki, o da yoksa arama kelimeleri
        // — böylece metaKeywords boş kalmaz (saha testi bulgusu).
        meta_keywords: d
            .draft_keywords
            .clone()
            .or(d.keywords.clone())
            .or(d.draft_search_keywords.clone())
            .unwrap_or_default(),
        search_keywords: d
            .draft_search_keywords
            .clone()
            .or(d.search_keywords.clone())
            .unwrap_or_default(),
        target_keyword: d.target_keyword.clone().unwrap_or_default(),
        details_html: d.draft_details.clone().or(d.details.clone()).unwrap_or_default(),
        tech_html,
    };
    Ok((domain, token, local))
}

/// sku → IdeaSoft id (önce cache, yoksa arama; bulununca cache'lenir).
async fn ideasoft_id_for(
    state: &State<'_, AppState>,
    sku: &str,
    domain: &str,
    token: &str,
) -> Result<i64, String> {
    let cached: Option<i64> = {
        let conn = state.conn.lock().unwrap();
        conn.query_row(
            "SELECT ideasoft_product_id FROM seo_status WHERE sku = ?1",
            [sku],
            |r| r.get(0),
        )
        .unwrap_or(None)
    };
    if let Some(id) = cached.filter(|v| *v > 0) {
        return Ok(id);
    }
    let r = ideasoft::resolve(domain, token, sku)
        .await?
        .ok_or_else(|| format!("Bu sku IdeaSoft'ta bulunamadı: {sku}"))?;
    {
        let conn = state.conn.lock().unwrap();
        ensure_seo_row(&conn, sku)?;
        conn.execute(
            "UPDATE seo_status SET ideasoft_product_id = ?2, ideasoft_seo_rule = ?3 WHERE sku = ?1",
            params![sku, r.id, r.seo_rule_count],
        )
        .map_err(|e| format!("IdeaSoft id kaydedilemedi: {e}"))?;
    }
    Ok(r.id)
}

#[tauri::command]
pub async fn test_ideasoft(state: State<'_, AppState>) -> Result<String, String> {
    let (domain, token) = {
        let conn = state.conn.lock().unwrap();
        (
            db::get_setting(&conn, "ideasoft_domain")?.unwrap_or_default(),
            db::get_setting(&conn, "ideasoft_token")?.unwrap_or_default(),
        )
    };
    ideasoft::test_connection(&domain, &token).await
}

/// Gönderim öncesi fark önizlemesi — uzaktaki mevcut değerler + gönderilecek gövde.
#[tauri::command]
pub async fn ideasoft_preview(
    state: State<'_, AppState>,
    sku: String,
    parts: Vec<String>,
) -> Result<IdeasoftPreview, String> {
    let (domain, token, local) = {
        let conn = state.conn.lock().unwrap();
        ideasoft_local(&conn, &sku)?
    };
    let id = ideasoft_id_for(&state, &sku, &domain, &token).await?;
    let remote = ideasoft::fetch_product(&domain, &token, id).await?;
    Ok(IdeasoftPreview { id, remote, local: ideasoft::build_payload(&parts, &local) })
}

/// IdeaSoft'taki hedef kelimeyi çeker ve yerel alana yazar (boş başlangıç sorununu çözer).
#[tauri::command]
pub async fn ideasoft_pull_keyword(
    state: State<'_, AppState>,
    sku: String,
) -> Result<ProductDetail, String> {
    let (domain, token) = {
        let conn = state.conn.lock().unwrap();
        let d = db::get_setting(&conn, "ideasoft_domain")?.unwrap_or_default();
        let t = db::get_setting(&conn, "ideasoft_token")?.unwrap_or_default();
        if d.trim().is_empty() || t.trim().is_empty() {
            return Err("IdeaSoft bağlantısı ayarlı değil.".to_string());
        }
        (d, t)
    };
    let id = ideasoft_id_for(&state, &sku, &domain, &token).await?;
    let remote = ideasoft::fetch_product(&domain, &token, id).await?;
    let kw = remote.target_keyword.trim().to_string();
    if kw.is_empty() {
        return Err("IdeaSoft'ta bu ürün için hedef kelime tanımlı değil.".to_string());
    }
    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    conn.execute(
        "UPDATE seo_status SET target_keyword = ?2, updated_at = ?3 WHERE sku = ?1",
        params![sku, kw, now_str()],
    )
    .map_err(|e| format!("Hedef kelime kaydedilemedi: {e}"))?;
    read_detail(&conn, &sku)
}

/// Seçilen parçaları IdeaSoft'a yazar. `parts` ∈ meta | keyword | details | tech.
#[tauri::command]
pub async fn ideasoft_push(
    state: State<'_, AppState>,
    sku: String,
    parts: Vec<String>,
) -> Result<ProductDetail, String> {
    let (domain, token, local) = {
        let conn = state.conn.lock().unwrap();
        ideasoft_local(&conn, &sku)?
    };
    let payload = ideasoft::build_payload(&parts, &local);
    if payload.as_object().map_or(true, |o| o.is_empty()) {
        return Err("Gönderilecek içerik yok — önce üretim yapın.".to_string());
    }
    let id = ideasoft_id_for(&state, &sku, &domain, &token).await?;
    // IdeaSoft `detail.details`'in null olmasına izin vermiyor → eksik alt alanları uzaktakiyle doldur
    // (dokunulmayan taraf aynen korunur).
    let mut payload = payload;
    if payload.get("detail").is_some() {
        let remote = ideasoft::fetch_product(&domain, &token, id).await?;
        ideasoft::fill_detail_from_remote(&mut payload, &remote);
    }
    ideasoft::push_product(&domain, &token, id, &payload).await?;

    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    conn.execute(
        "UPDATE seo_status SET ideasoft_pushed_at = ?2, updated_at = ?2 WHERE sku = ?1",
        params![sku, now_str()],
    )
    .map_err(|e| format!("Gönderim zamanı kaydedilemedi: {e}"))?;
    read_detail(&conn, &sku)
}

// ---- Faz 8: teknik özellik tablosu ----

/// En fazla saklanan önceki sürüm sayısı.

/// Üretimden önceki meta hâlini geçmişe iter.
///
/// `Ok(None)` → geçmiş DEĞİŞMEMELİ (mevcut içerik boş, ya da yeni üretim eskisiyle birebir aynı).
/// Aynı sonucu veren yeniden üretimi kaydetmek geçmişi çöple doldurur ve gerçek eski hâlleri
/// `history::MAX` sınırından erken düşürürdü.
fn snapshot_meta(
    conn: &Connection,
    sku: &str,
    fresh: &gemini::GeneratedMeta,
) -> Result<Option<String>, String> {
    let cur: (String, String, String, String, String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT COALESCE(draft_title,''), COALESCE(draft_descriptions,''),
                    COALESCE(draft_keywords,''), COALESCE(draft_search_keywords,''),
                    COALESCE(target_keyword,''), meta_model, meta_history_json
             FROM seo_status WHERE sku = ?1",
            [sku],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        )
        .map_err(|e| format!("Mevcut meta okunamadı: {e}"))?;

    // İlk üretim: saklanacak bir şey yok
    if cur.0.trim().is_empty() && cur.1.trim().is_empty() {
        return Ok(None);
    }
    // Sonuç aynıysa sürüm açma
    if cur.0.trim() == fresh.title.trim() && cur.1.trim() == fresh.descriptions.trim() {
        return Ok(None);
    }

    let hist = history::push(
        history::parse::<MetaVersion>(cur.6.as_deref()),
        MetaVersion {
            at: now_str(),
            title: cur.0,
            descriptions: cur.1,
            keywords: cur.2,
            search_keywords: cur.3,
            target_keyword: cur.4,
            model: cur.5.unwrap_or_default(),
        },
    );
    serde_json::to_string(&hist)
        .map(Some)
        .map_err(|e| format!("Meta geçmişi kaydedilemedi: {e}"))
}

/// Üretimden önceki açıklama hâlini geçmişe iter. Kurallar `snapshot_meta` ile aynı.
fn snapshot_details(conn: &Connection, sku: &str, fresh: &str) -> Result<Option<String>, String> {
    let (cur_html, cur_model, hist_json): (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT COALESCE(draft_details,''), details_model, details_history_json
             FROM seo_status WHERE sku = ?1",
            [sku],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|e| format!("Mevcut açıklama okunamadı: {e}"))?;

    if cur_html.trim().is_empty() || cur_html.trim() == fresh.trim() {
        return Ok(None);
    }
    let hist = history::push(
        history::parse::<DetailsVersion>(hist_json.as_deref()),
        DetailsVersion {
            at: now_str(),
            html: cur_html,
            model: cur_model.unwrap_or_default(),
        },
    );
    serde_json::to_string(&hist)
        .map(Some)
        .map_err(|e| format!("Açıklama geçmişi kaydedilemedi: {e}"))
}

/// Saklanan bir meta sürümü. Hedef kelime de içeride: o meta ona göre üretilmişti,
/// geri yüklerken ikisi birlikte dönmeli (kullanıcı kararı) yoksa çelişkili bir hâl oluşur.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct MetaVersion {
    pub at: String,
    pub title: String,
    pub descriptions: String,
    pub keywords: String,
    pub search_keywords: String,
    pub target_keyword: String,
    #[serde(default)]
    pub model: String,
}

/// Saklanan bir açıklama sürümü.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct DetailsVersion {
    pub at: String,
    pub html: String,
    #[serde(default)]
    pub model: String,
}

/// UI listesi için hafif özetler — tam sürümler payload'ı şişirmesin
/// (açıklama HTML'i ürün başına ortalama 3,7 KB).
#[derive(Debug, Serialize)]
pub struct MetaVersionMeta {
    pub at: String,
    /// Başlık, sürümü tanımanın en hızlı yolu.
    pub title: String,
    pub model: String,
}

#[derive(Debug, Serialize)]
pub struct DetailsVersionMeta {
    pub at: String,
    pub words: usize,
    pub model: String,
}

/// Saklanan bir teknik tablo sürümü (yeniden üretim öncesi anlık görüntü).
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct TechVersion {
    pub at: String,
    pub groups: Vec<gemini::TechGroup>,
    #[serde(default)]
    pub source: String,
}

/// UI listesi için hafif özet (tam sürümler payload'ı şişirmesin).
#[derive(Debug, Serialize)]
pub struct TechVersionMeta {
    pub at: String,
    pub rows: usize,
    pub groups: usize,
}


/// Kullanıcının yapıştırdığı ham teknik metni saklar (debounce'lu kayıt).
#[tauri::command]
pub fn save_tech_source(state: State<'_, AppState>, sku: String, text: String) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    conn.execute(
        "UPDATE seo_status SET tech_source_text = ?2, updated_at = ?3 WHERE sku = ?1",
        params![sku, text, now_str()],
    )
    .map_err(|e| format!("Teknik metin kaydedilemedi: {e}"))?;
    Ok(())
}

/// Ham metni gruplu spec'lere çevirir (kaynağa karşı doğrulanır) ve saklar.
#[tauri::command]
pub async fn structure_tech_specs(
    state: State<'_, AppState>,
    sku: String,
) -> Result<gemini::TechSpecsResult, String> {
    let (name, brand, category, main_category, source, prev_specs, prev_hist, api_key) = {
        let conn = state.conn.lock().unwrap();
        let key = db::get_setting(&conn, "gemini_api_key")?.unwrap_or_default();
        conn.query_row(
            "SELECT p.name, p.brand, p.category, p.main_category, COALESCE(s.tech_source_text,''),
                    s.tech_specs_json, s.tech_history_json
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
                    r.get::<_, Option<String>>(6)?,
                ))
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
            other => format!("Ürün okunamadı: {other}"),
        })
        .map(|t| (t.0, t.1, t.2, t.3, t.4, t.5, t.6, key))?
    };

    let ctx = gemini::ProductContext {
        name: &name,
        brand: brand.as_deref(),
        category: category.as_deref(),
        main_category: main_category.as_deref(),
        target_keyword: None,
        insights: None, // teknik tablo pazarlama verisi değil — SEO araştırması karıştırılmaz
    };
    let produced = gemini::structure_tech_specs(&api_key, &ctx, &source).await?;
    let (result, model) = (produced.value, produced.model);

    let json = serde_json::to_string(&result.groups)
        .map_err(|e| format!("Teknik tablo serialize edilemedi: {e}"))?;

    // Yeniden üretim: eski tabloyu kaybetmeden geçmişe al (bkz. core/src/history.rs).
    let old_groups: Vec<gemini::TechGroup> = prev_specs
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();
    let history_json = if old_groups.is_empty() {
        None
    } else {
        let hist = history::push(
            history::parse(prev_hist.as_deref()),
            TechVersion { at: now_str(), groups: old_groups, source: source.clone() },
        );
        serde_json::to_string(&hist).ok()
    };

    {
        let conn = state.conn.lock().unwrap();
        ensure_seo_row(&conn, &sku)?;
        match &history_json {
            Some(h) => conn.execute(
                "UPDATE seo_status SET tech_specs_json = ?2, tech_history_json = ?3, updated_at = ?4,
                        tech_model = ?5
                 WHERE sku = ?1",
                params![sku, json, h, now_str(), model],
            ),
            None => conn.execute(
                "UPDATE seo_status SET tech_specs_json = ?2, updated_at = ?3, tech_model = ?4
                 WHERE sku = ?1",
                params![sku, json, now_str(), model],
            ),
        }
        .map_err(|e| format!("Teknik tablo kaydedilemedi: {e}"))?;
    }
    Ok(result)
}

/// Kullanıcının elle düzenlediği tablo (doğruluk kaynağı kullanıcıdır).
#[tauri::command]
pub fn save_tech_specs(
    state: State<'_, AppState>,
    sku: String,
    specs: Vec<gemini::TechGroup>,
) -> Result<(), String> {
    let json = serde_json::to_string(&specs).map_err(|e| format!("Serialize edilemedi: {e}"))?;
    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    conn.execute(
        "UPDATE seo_status SET tech_specs_json = ?2, updated_at = ?3 WHERE sku = ?1",
        params![sku, json, now_str()],
    )
    .map_err(|e| format!("Teknik tablo kaydedilemedi: {e}"))?;
    Ok(())
}

/// IdeaSoft'a yapıştırılacak semantik HTML (deterministik, model devrede değil).
#[tauri::command]
pub fn tech_table_html(state: State<'_, AppState>, sku: String) -> Result<String, String> {
    let conn = state.conn.lock().unwrap();
    let json: Option<String> = conn
        .query_row("SELECT tech_specs_json FROM seo_status WHERE sku = ?1", [&sku], |r| r.get(0))
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
            other => format!("Teknik tablo okunamadı: {other}"),
        })?;
    let groups: Vec<gemini::TechGroup> = json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();
    if groups.is_empty() {
        return Err("Önce teknik tabloyu yapılandırın.".to_string());
    }
    Ok(gemini::assemble_tech_html(&groups))
}

/// Önceki bir sürümü geri yükler. **Takas mantığı**: mevcut tablo geçmişin başına konur, seçilen
/// sürüm güncel olur → geri yükleme de kayıpsızdır (istenirse geri dönülebilir).
/// Eski bir meta sürümünü geri yükler.
///
/// **Takas:** geri yüklenen sürüm listeden çıkar, mevcut içerik (boş değilse) geçmişe girer —
/// böylece geri yükleme de geri alınabilir. `restore_tech_version` ile aynı semantik.
/// Hedef kelime de birlikte döner: o meta ona göre üretilmişti (kullanıcı kararı).
#[tauri::command]
pub fn restore_meta_version(
    state: State<'_, AppState>,
    sku: String,
    index: usize,
) -> Result<ProductDetail, String> {
    let conn = state.conn.lock().unwrap();
    let cur: (String, String, String, String, String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT COALESCE(draft_title,''), COALESCE(draft_descriptions,''),
                    COALESCE(draft_keywords,''), COALESCE(draft_search_keywords,''),
                    COALESCE(target_keyword,''), meta_model, meta_history_json
             FROM seo_status WHERE sku = ?1",
            [&sku],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
            other => format!("Meta okunamadı: {other}"),
        })?;

    let mut hist = history::parse::<MetaVersion>(cur.6.as_deref());
    if index >= hist.len() {
        return Err("Bu sürüm artık mevcut değil.".to_string());
    }
    let restored = hist.remove(index);

    if !cur.0.trim().is_empty() || !cur.1.trim().is_empty() {
        hist = history::push(
            hist,
            MetaVersion {
                at: now_str(),
                title: cur.0,
                descriptions: cur.1,
                keywords: cur.2,
                search_keywords: cur.3,
                target_keyword: cur.4,
                model: cur.5.unwrap_or_default(),
            },
        );
    }
    let hist_json = serde_json::to_string(&hist)
        .map_err(|e| format!("Meta geçmişi kaydedilemedi: {e}"))?;

    conn.execute(
        "UPDATE seo_status SET draft_title = ?2, draft_descriptions = ?3, draft_keywords = ?4,
                draft_search_keywords = ?5, target_keyword = ?6, meta_model = ?7,
                meta_history_json = ?8, updated_at = ?9
         WHERE sku = ?1",
        params![
            sku,
            restored.title,
            restored.descriptions,
            restored.keywords,
            restored.search_keywords,
            restored.target_keyword,
            restored.model,
            hist_json,
            now_str(),
        ],
    )
    .map_err(|e| format!("Meta geri yüklenemedi: {e}"))?;
    read_detail(&conn, &sku)
}

/// Eski bir açıklama sürümünü geri yükler (takas semantiği `restore_meta_version` ile aynı).
#[tauri::command]
pub fn restore_details_version(
    state: State<'_, AppState>,
    sku: String,
    index: usize,
) -> Result<ProductDetail, String> {
    let conn = state.conn.lock().unwrap();
    let (cur_html, cur_model, hist_json): (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT COALESCE(draft_details,''), details_model, details_history_json
             FROM seo_status WHERE sku = ?1",
            [&sku],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
            other => format!("Açıklama okunamadı: {other}"),
        })?;

    let mut hist = history::parse::<DetailsVersion>(hist_json.as_deref());
    if index >= hist.len() {
        return Err("Bu sürüm artık mevcut değil.".to_string());
    }
    let restored = hist.remove(index);

    if !cur_html.trim().is_empty() {
        hist = history::push(
            hist,
            DetailsVersion {
                at: now_str(),
                html: cur_html,
                model: cur_model.unwrap_or_default(),
            },
        );
    }
    let new_hist = serde_json::to_string(&hist)
        .map_err(|e| format!("Açıklama geçmişi kaydedilemedi: {e}"))?;

    conn.execute(
        "UPDATE seo_status SET draft_details = ?2, details_model = ?3,
                details_history_json = ?4, updated_at = ?5
         WHERE sku = ?1",
        params![sku, restored.html, restored.model, new_hist, now_str()],
    )
    .map_err(|e| format!("Açıklama geri yüklenemedi: {e}"))?;
    read_detail(&conn, &sku)
}

#[tauri::command]
pub fn restore_tech_version(
    state: State<'_, AppState>,
    sku: String,
    index: usize,
) -> Result<ProductDetail, String> {
    let conn = state.conn.lock().unwrap();
    let (cur_json, hist_json, cur_source): (Option<String>, Option<String>, String) = conn
        .query_row(
            "SELECT tech_specs_json, tech_history_json, COALESCE(tech_source_text,'')
             FROM seo_status WHERE sku = ?1",
            [&sku],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
            other => format!("Teknik tablo okunamadı: {other}"),
        })?;

    let mut hist = history::parse(hist_json.as_deref());
    if index >= hist.len() {
        return Err("Bu sürüm artık mevcut değil.".to_string());
    }
    let restored = hist.remove(index);

    // Mevcut tablo boş değilse geçmişe geri koy (takas)
    let cur_groups: Vec<gemini::TechGroup> = cur_json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();
    if !cur_groups.is_empty() {
        hist = history::push(
            hist,
            TechVersion { at: now_str(), groups: cur_groups, source: cur_source },
        );
    }

    let specs_json = serde_json::to_string(&restored.groups)
        .map_err(|e| format!("Serialize edilemedi: {e}"))?;
    let hist_out = serde_json::to_string(&hist).map_err(|e| format!("Serialize edilemedi: {e}"))?;
    conn.execute(
        "UPDATE seo_status SET tech_specs_json = ?2, tech_history_json = ?3,
                tech_source_text = ?4, updated_at = ?5
         WHERE sku = ?1",
        params![sku, specs_json, hist_out, restored.source, now_str()],
    )
    .map_err(|e| format!("Sürüm geri yüklenemedi: {e}"))?;
    read_detail(&conn, &sku)
}

#[tauri::command]
pub fn mark_tech_done(state: State<'_, AppState>, sku: String) -> Result<String, String> {
    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    let current: String = conn
        .query_row(
            "SELECT COALESCE(tech_status,'pending') FROM seo_status WHERE sku = ?1",
            [&sku],
            |r| r.get(0),
        )
        .map_err(|e| format!("Durum okunamadı: {e}"))?;
    let next = if current == "done" { "pending" } else { "done" };
    conn.execute(
        "UPDATE seo_status SET tech_status = ?2, updated_at = ?3 WHERE sku = ?1",
        params![sku, next, now_str()],
    )
    .map_err(|e| format!("Durum güncellenemedi: {e}"))?;
    Ok(next.to_string())
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

// ---- Fırsat analizi: GSC verisiyle "önce hangi ürüne bakmalıyım?" ----

#[derive(Serialize)]
pub struct InvisibleProduct {
    pub sku: String,
    pub name: String,
    pub url: String,
}

#[derive(Serialize)]
pub struct OpportunityReport {
    pub analyzed_at: String,
    pub days: i64,
    /// Kaçırılan tıklamaya göre azalan sıralı.
    pub opportunities: Vec<opportunity::Opportunity>,
    /// GSC'de hiç satırı olmayan ürünler — farklı bir iş (indeksleme/görünürlük),
    /// meta üretimiyle çözülmediği için ayrı listede.
    pub invisible: Vec<InvisibleProduct>,
    pub total_products: usize,
    /// GSC verisiyle eşleşen ürün sayısı — eşleşme düşükse sorun URL biçimindedir.
    pub matched: usize,
}

/// GSC'nin döndürdüğü URL ile feed'deki URL arasındaki zararsız farkları törpüler
/// (sondaki `/`, büyük/küçük harf). Aksi halde tek karakterlik fark yüzünden ürün
/// "Google'da görünmüyor" gibi raporlanırdı.
fn norm_url(u: &str) -> String {
    u.trim().trim_end_matches('/').to_lowercase()
}

const OPPORTUNITY_DAYS: i64 = 90;

#[tauri::command]
pub async fn analyze_opportunities(
    state: State<'_, AppState>,
) -> Result<OpportunityReport, String> {
    let (gsc_json, gsc_site, products) = {
        let conn = state.conn.lock().unwrap();
        let gsc_json = db::get_setting(&conn, "gsc_service_account_json")?.unwrap_or_default();
        let gsc_site = db::get_setting(&conn, "gsc_site_url")?.unwrap_or_default();
        // Kategori/marka ve SEO iş durumu da alınır — fırsat listesinde "hiç dokunulmamış" ile
        // "çalışılmış ama hâlâ sorunlu" ayrımı için. Tek sorgu, ek ağ çağrısı yok.
        let mut stmt = conn
            .prepare(
                "SELECT p.sku, p.name, p.url, COALESCE(p.category,''), COALESCE(p.brand,''),
                        COALESCE(s.meta_status,'pending'), COALESCE(s.details_status,'pending')
                 FROM products p LEFT JOIN seo_status s ON s.sku = p.sku
                 WHERE p.url IS NOT NULL AND p.url <> ''",
            )
            .map_err(|e| format!("Ürünler okunamadı: {e}"))?;
        let rows: Vec<(String, String, String, String, String, String, String)> = stmt
            .query_map([], |r| {
                Ok((
                    r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?,
                ))
            })
            .map_err(|e| format!("Ürünler okunamadı: {e}"))?
            .filter_map(|r| r.ok())
            .collect();
        (gsc_json, gsc_site, rows)
    };

    // Yapılandırılmamışsa sessizce boş liste DÖNME — kullanıcı "fırsat yok" sanır.
    if gsc_json.trim().is_empty() || gsc_site.trim().is_empty() {
        return Err(
            "Google Search Console bağlantısı kurulmamış. Ayarlar'dan service-account \
             dosyasını yükleyip mülk adresini girin."
                .to_string(),
        );
    }
    if products.is_empty() {
        return Err("Önce ürünleri senkronize edin.".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;

    // Tek çağrı: sayfa boyutunda tüm site. Ürün başına istek atmak 262 çağrı olurdu.
    let stats = seo_data::gsc::page_stats(
        &client,
        &gsc_json,
        gsc_site.trim(),
        OPPORTUNITY_DAYS,
        25_000,
    )
    .await?;

    let by_url: std::collections::HashMap<String, &seo_data::PageStat> =
        stats.iter().map(|s| (norm_url(&s.page), s)).collect();

    let total_products = products.len();
    let mut opportunities = Vec::new();
    let mut invisible = Vec::new();
    let mut matched = 0usize;

    for (sku, name, url, category, brand, meta_status, details_status) in products {
        match by_url.get(&norm_url(&url)) {
            Some(st) => {
                matched += 1;
                if let Some((reason, missed)) =
                    opportunity::classify(st.clicks, st.impressions, st.ctr, st.position)
                {
                    opportunities.push(opportunity::Opportunity {
                        sku,
                        name,
                        url,
                        clicks: st.clicks,
                        impressions: st.impressions,
                        ctr: st.ctr,
                        position: st.position,
                        missed_clicks: missed,
                        reason,
                        category,
                        brand,
                        meta_status,
                        details_status,
                    });
                }
            }
            None => invisible.push(InvisibleProduct { sku, name, url }),
        }
    }

    opportunity::sort_by_impact(&mut opportunities);

    let report = OpportunityReport {
        analyzed_at: now_str(),
        days: OPPORTUNITY_DAYS,
        opportunities,
        invisible,
        total_products,
        matched,
    };

    // Önbelleğe al: GSC verisi günlük değişir, her sayfa açılışında API'ye gitmeye gerek yok.
    if let Ok(json) = serde_json::to_string(&report) {
        let conn = state.conn.lock().unwrap();
        let _ = db::set_setting(&conn, "opportunity_json", &json);
    }
    Ok(report)
}

/// Önbellekteki son analiz (API'ye gitmeden). Hiç çalıştırılmadıysa `None`.
#[tauri::command]
pub fn get_opportunity_cache(state: State<'_, AppState>) -> Result<Option<serde_json::Value>, String> {
    let conn = state.conn.lock().unwrap();
    let raw = db::get_setting(&conn, "opportunity_json")?;
    Ok(raw.and_then(|j| serde_json::from_str(&j).ok()))
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
        ideasoft_domain: db::get_setting(&conn, "ideasoft_domain")?.unwrap_or_default(),
        ideasoft_token: db::get_setting(&conn, "ideasoft_token")?.unwrap_or_default(),
        ideasoft_active: !db::get_setting(&conn, "ideasoft_domain")?.unwrap_or_default().trim().is_empty()
            && !db::get_setting(&conn, "ideasoft_token")?.unwrap_or_default().trim().is_empty(),
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
    ideasoft_domain: String,
    ideasoft_token: String,
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
    db::set_setting(&conn, "ideasoft_domain", ideasoft_domain.trim())?;
    db::set_setting(&conn, "ideasoft_token", ideasoft_token.trim())?;
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
            "last_synced_at", "picture2", "picture3", "picture4",
        ],
    )?;
    // Not: draft_details/research_json/tech_* alanları da yedeklenir. Teknik tablo feed'de YOK,
    // yani yedekte yoksa geri getirilemez (gerçek emek kaybı).
    let seo = dump(
        conn,
        "seo_status",
        &[
            "sku", "meta_status", "details_status", "target_keyword", "draft_title",
            "draft_descriptions", "draft_keywords", "draft_search_keywords", "updated_at",
            "draft_details", "research_json", "image_check_json", "image_check_fp",
            "tech_source_text", "tech_specs_json", "tech_status", "tech_history_json",
            "ideasoft_product_id", "ideasoft_pushed_at", "ideasoft_seo_rule",
            "meta_model", "details_model", "tech_model",
            "meta_history_json", "details_history_json",
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
               quantity, url, img_url, title, descriptions, keywords, search_keywords, details, last_synced_at,
               picture2, picture3, picture4)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
            params![
                s(&p, "sku"), s(&p, "id"), s(&p, "name").unwrap_or_default(), s(&p, "brand"),
                s(&p, "main_category"), s(&p, "category"), i(&p, "quantity"), s(&p, "url"),
                s(&p, "img_url"), s(&p, "title"), s(&p, "descriptions"), s(&p, "keywords"),
                s(&p, "search_keywords"), s(&p, "details"), s(&p, "last_synced_at"),
                s(&p, "picture2"), s(&p, "picture3"), s(&p, "picture4"),
            ],
        )
        .map_err(|e| format!("Ürün geri yüklenemedi: {e}"))?;
    }
    for r in arr("seo_status") {
        tx.execute(
            "INSERT OR REPLACE INTO seo_status (sku, meta_status, details_status, target_keyword,
               draft_title, draft_descriptions, draft_keywords, draft_search_keywords, updated_at,
               draft_details, research_json, image_check_json, image_check_fp,
               tech_source_text, tech_specs_json, tech_status, tech_history_json,
               ideasoft_product_id, ideasoft_pushed_at, ideasoft_seo_rule,
               meta_model, details_model, tech_model,
               meta_history_json, details_history_json)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,
                     ?21,?22,?23,?24,?25)",
            params![
                s(&r, "sku"),
                s(&r, "meta_status").unwrap_or_else(|| "pending".into()),
                s(&r, "details_status").unwrap_or_else(|| "pending".into()),
                s(&r, "target_keyword"), s(&r, "draft_title"), s(&r, "draft_descriptions"),
                s(&r, "draft_keywords"), s(&r, "draft_search_keywords"), s(&r, "updated_at"),
                s(&r, "draft_details"), s(&r, "research_json"), s(&r, "image_check_json"),
                s(&r, "image_check_fp"), s(&r, "tech_source_text"), s(&r, "tech_specs_json"),
                s(&r, "tech_status").unwrap_or_else(|| "pending".into()),
                s(&r, "tech_history_json"), i(&r, "ideasoft_product_id"),
                s(&r, "ideasoft_pushed_at"), i(&r, "ideasoft_seo_rule"),
                s(&r, "meta_model"), s(&r, "details_model"), s(&r, "tech_model"),
                s(&r, "meta_history_json"), s(&r, "details_history_json"),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ver(at: &str) -> TechVersion {
        TechVersion {
            at: at.into(),
            groups: vec![gemini::TechGroup {
                group: "Performans".into(),
                rows: vec![gemini::TechRow { label: "İşlemci".into(), value: "i7".into() }],
            }],
            source: format!("kaynak {at}"),
        }
    }

    #[test]
    fn push_history_keeps_newest_first_and_caps() {
        let mut h: Vec<TechVersion> = Vec::new();
        for i in 1..=7 {
            h = history::push(h, ver(&format!("v{i}")));
        }
        // En yeni başta, üst sınır aşılmaz
        assert_eq!(h.len(), history::MAX);
        assert_eq!(h[0].at, "v7");
        assert_eq!(h[history::MAX - 1].at, "v3"); // v1, v2 düştü
    }

    #[test]
    fn parse_history_tolerates_missing_and_broken() {
        assert!(history::parse::<TechVersion>(None).is_empty());
        assert!(history::parse::<TechVersion>(Some("")).is_empty());
        assert!(history::parse::<TechVersion>(Some("  ")).is_empty());
        assert!(history::parse::<TechVersion>(Some("{bozuk json")).is_empty());
        let json = serde_json::to_string(&vec![ver("v1")]).unwrap();
        assert_eq!(history::parse::<TechVersion>(Some(&json)).len(), 1);
    }

    #[test]
    fn history_roundtrip_preserves_source_and_rows() {
        let json = serde_json::to_string(&vec![ver("2026-07-25T10:00:00")]).unwrap();
        let back = history::parse::<TechVersion>(Some(&json));
        assert_eq!(back[0].source, "kaynak 2026-07-25T10:00:00");
        assert_eq!(back[0].groups[0].rows[0].value, "i7");
    }
}
