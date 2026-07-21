use crate::validation::{meta_badge, MetaBadge, MetaInput};
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
    pub badge: MetaBadge,
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
    pub badge: MetaBadge,
}

#[derive(Debug, Serialize)]
pub struct Settings {
    pub feed_url: String,
    pub gemini_api_key: String,
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
    meta_status: String,
    details_status: String,
    target_keyword: Option<String>,
    draft_title: Option<String>,
    draft_descriptions: Option<String>,
}

/// Rozet, taslak varsa taslak (NULL değilse) yoksa feed değeri üzerinden hesaplanır.
fn badge_of(r: &RowData) -> MetaBadge {
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
            "SELECT p.sku, p.name, p.brand, p.img_url, p.title, p.descriptions,
                    COALESCE(s.meta_status,'pending'), COALESCE(s.details_status,'pending'),
                    s.target_keyword, s.draft_title, s.draft_descriptions
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
                meta_status: row.get(6)?,
                details_status: row.get(7)?,
                target_keyword: row.get(8)?,
                draft_title: row.get(9)?,
                draft_descriptions: row.get(10)?,
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
        let badge = badge_of(&r);
        let keep = match f.as_str() {
            "" | "hepsi" => true, // filtre yok: tamamlananlar dahil her şey
            "tumu" => badge != MetaBadge::Tamamlandi,
            "eksik" => badge == MetaBadge::Eksik,
            "hatali" => badge == MetaBadge::Hatali,
            "uygun" => badge == MetaBadge::Uygun,
            "tamamlandi" => badge == MetaBadge::Tamamlandi,
            "bekliyor" => false, // "Açıklama Bekliyor" Faz 2'de dolacak
            other => return Err(format!("Bilinmeyen filtre: {other}")),
        };
        if keep {
            out.push(ProductRow {
                sku: r.sku,
                name: r.name,
                brand: r.brand,
                img_url: r.img_url,
                badge,
                meta_done: r.meta_status == "done",
                details_done: r.details_status == "done",
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
                s.target_keyword, s.draft_title, s.draft_descriptions, s.draft_search_keywords
         FROM products p LEFT JOIN seo_status s ON s.sku = p.sku
         WHERE p.sku = ?1",
        [&sku],
        |row| {
            Ok(ProductDetail {
                sku: row.get(0)?,
                name: row.get(1)?,
                brand: row.get(2)?,
                main_category: row.get(3)?,
                category: row.get(4)?,
                quantity: row.get(5)?,
                url: row.get(6)?,
                img_url: row.get(7)?,
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
                badge: MetaBadge::Eksik, // aşağıda hesaplanır
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
        other => format!("Ürün okunamadı: {other}"),
    })
    .map(|mut d| {
        d.badge = meta_badge(&MetaInput {
            title: d.draft_title.as_deref().unwrap_or(d.title.as_deref().unwrap_or("")),
            descriptions: d
                .draft_descriptions
                .as_deref()
                .unwrap_or(d.descriptions.as_deref().unwrap_or("")),
            target_keyword: d.target_keyword.as_deref().unwrap_or(""),
            meta_done: d.meta_status == "done",
        });
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
    let (name, brand, category, main_category, api_key) = {
        let conn = state.conn.lock().unwrap();
        let key = db::get_setting(&conn, "gemini_api_key")?.unwrap_or_default();
        let row = conn
            .query_row(
                "SELECT name, brand, category, main_category FROM products WHERE sku = ?1",
                [&sku],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, Option<String>>(3)?,
                    ))
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
                other => format!("Ürün okunamadı: {other}"),
            })?;
        (row.0, row.1, row.2, row.3, key)
    };

    let ctx = gemini::ProductContext {
        name: &name,
        brand: brand.as_deref(),
        category: category.as_deref(),
        main_category: main_category.as_deref(),
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

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let conn = state.conn.lock().unwrap();
    Ok(Settings {
        feed_url: db::feed_url(&conn)?,
        gemini_api_key: db::get_setting(&conn, "gemini_api_key")?.unwrap_or_default(),
        theme: db::get_setting(&conn, "theme")?,
        last_backup_at: db::get_setting(&conn, "last_backup_at")?,
    })
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, AppState>,
    feed_url: String,
    gemini_api_key: String,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let url = feed_url.trim();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Geçerli bir URL girin (http/https).".to_string());
    }
    db::set_setting(&conn, "feed_url", url)?;
    db::set_setting(&conn, "gemini_api_key", gemini_api_key.trim())?;
    Ok(())
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
