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
