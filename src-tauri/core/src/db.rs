use rusqlite::Connection;

pub const DEFAULT_FEED_URL: &str = "https://www.kurumsalit.com/output/2567783262";

pub fn open(path: &std::path::Path) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| format!("Veritabanı açılamadı: {e}"))?;
    init(&conn)?;
    Ok(conn)
}

pub fn init(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS products (
          sku TEXT PRIMARY KEY,
          id TEXT,
          name TEXT NOT NULL,
          brand TEXT,
          main_category TEXT,
          category TEXT,
          quantity INTEGER,
          url TEXT,
          img_url TEXT,
          picture2 TEXT,
          picture3 TEXT,
          picture4 TEXT,
          title TEXT,
          descriptions TEXT,
          keywords TEXT,
          search_keywords TEXT,
          details TEXT,
          last_synced_at TEXT
        );

        CREATE TABLE IF NOT EXISTS seo_status (
          sku TEXT PRIMARY KEY REFERENCES products(sku) ON DELETE CASCADE,
          meta_status TEXT DEFAULT 'pending',
          details_status TEXT DEFAULT 'pending',
          target_keyword TEXT,
          draft_title TEXT,
          draft_descriptions TEXT,
          draft_keywords TEXT,
          draft_search_keywords TEXT,
          updated_at TEXT
        );

        CREATE TABLE IF NOT EXISTS sync_log (
          run_at TEXT,
          active INTEGER,
          added INTEGER,
          updated INTEGER,
          deleted INTEGER,
          duplicate_skipped INTEGER
        );

        CREATE TABLE IF NOT EXISTS settings (
          key TEXT PRIMARY KEY,
          value TEXT
        );
        "#,
    )
    .map_err(|e| format!("Şema oluşturulamadı: {e}"))?;

    migrate(conn)?;
    Ok(())
}

/// Eski DB'lere sonradan eklenen kolonları idempotent şekilde ekler.
fn migrate(conn: &Connection) -> Result<(), String> {
    add_column_if_missing(conn, "seo_status", "draft_details", "TEXT")?;
    // Faz 4: SEO araştırma çıktısı (SeoInsights JSON) ürün başına saklanır.
    add_column_if_missing(conn, "seo_status", "research_json", "TEXT")?;
    // Faz 7: galeri görsel slotları + boyut kontrolü cache'i.
    add_column_if_missing(conn, "products", "picture2", "TEXT")?;
    add_column_if_missing(conn, "products", "picture3", "TEXT")?;
    add_column_if_missing(conn, "products", "picture4", "TEXT")?;
    add_column_if_missing(conn, "seo_status", "image_check_json", "TEXT")?;
    add_column_if_missing(conn, "seo_status", "image_check_fp", "TEXT")?;
    // Faz 8: teknik özellik tablosu. Bu veri feed'de YOK → tek kaynağı uygulama, yedeklemede korunmalı.
    add_column_if_missing(conn, "seo_status", "tech_source_text", "TEXT")?;
    add_column_if_missing(conn, "seo_status", "tech_specs_json", "TEXT")?;
    add_column_if_missing(conn, "seo_status", "tech_status", "TEXT DEFAULT 'pending'")?;
    // Faz 8b: önceki teknik tablo sürümleri (yeniden üretim öncesi anlık görüntü).
    add_column_if_missing(conn, "seo_status", "tech_history_json", "TEXT")?;
    // Faz 9: IdeaSoft gönderim modülü (id cache + son gönderim zamanı).
    add_column_if_missing(conn, "seo_status", "ideasoft_product_id", "INTEGER")?;
    add_column_if_missing(conn, "seo_status", "ideasoft_pushed_at", "TEXT")?;
    add_column_if_missing(conn, "seo_status", "ideasoft_seo_rule", "INTEGER")?;
    // Hangi Gemini modeli üretti. Zincir kotaya takıldıkça alt modellere düştüğü için
    // kullanıcının bunu görmesi gerekiyor: içerik son çare modeliyle üretildiyse limitler
    // yenilendiğinde yeniden üretmek isteyebilir. Üç üretim türü ayrı izlenir.
    add_column_if_missing(conn, "seo_status", "meta_model", "TEXT")?;
    add_column_if_missing(conn, "seo_status", "details_model", "TEXT")?;
    add_column_if_missing(conn, "seo_status", "tech_model", "TEXT")?;
    // Sürüm geçmişi: yeniden üretmeden önceki hâl saklanır (bkz. core/src/history.rs).
    // Teknik tabloda zaten vardı; meta ve açıklamada içerik geri dönüşsüz kayboluyordu.
    add_column_if_missing(conn, "seo_status", "meta_history_json", "TEXT")?;
    add_column_if_missing(conn, "seo_status", "details_history_json", "TEXT")?;
    Ok(())
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    col_type: &str,
) -> Result<(), String> {
    let exists: bool = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .and_then(|mut stmt| {
            let names: Vec<String> = stmt
                .query_map([], |row| row.get::<_, String>(1))?
                .filter_map(Result::ok)
                .collect();
            Ok(names.iter().any(|n| n == column))
        })
        .map_err(|e| format!("{table} kolonları okunamadı: {e}"))?;
    if !exists {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {col_type}"),
            [],
        )
        .map_err(|e| format!("{table}.{column} eklenemedi: {e}"))?;
    }
    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        [key],
        |row| row.get::<_, String>(0),
    )
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(format!("Ayar okunamadı: {other}")),
    })
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )
    .map_err(|e| format!("Ayar kaydedilemedi: {e}"))?;
    Ok(())
}

pub fn feed_url(conn: &Connection) -> Result<String, String> {
    Ok(get_setting(conn, "feed_url")?.unwrap_or_else(|| DEFAULT_FEED_URL.to_string()))
}
