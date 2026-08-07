use crate::fingerprint::FeedFacts;
use rusqlite::Connection;

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

        -- ===== Ölçüm omurgası (Faz Ö) =====
        -- ⚠️ Anlık görüntüler DEĞİŞTİRİLEMEZ. Fırsat raporu tek bir `settings` anahtarına
        -- yazılıyordu ve her analiz bir öncekini siliyordu; "işe yaradı mı?" sorusunun
        -- cevapsız kalmasının sebebi buydu. Buraya yalnızca eklenir, güncellenmez.
        CREATE TABLE IF NOT EXISTS metric_snapshots (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          captured_at TEXT NOT NULL,
          window_start TEXT NOT NULL,
          window_end TEXT NOT NULL,
          source TEXT NOT NULL DEFAULT 'gsc',
          rows INTEGER NOT NULL DEFAULT 0,
          clicks REAL NOT NULL DEFAULT 0,
          impressions REAL NOT NULL DEFAULT 0,
          UNIQUE (window_start, window_end)
        );

        -- Satır eşiği ÖLÇÜLEREK seçildi (bkz. `metrics::kept`): tıklama > 0 veya
        -- gösterim >= 10. Gerçek veride satırların %34'ü tutuluyor ama tıklamaların
        -- %100'ü kapsanıyor — 12 anlık görüntü 8,7 MB yerine 2,9 MB.
        CREATE TABLE IF NOT EXISTS metric_page_rows (
          snapshot_id INTEGER NOT NULL REFERENCES metric_snapshots(id) ON DELETE CASCADE,
          url TEXT NOT NULL,
          sku TEXT,
          clicks REAL NOT NULL DEFAULT 0,
          impressions REAL NOT NULL DEFAULT 0,
          position REAL NOT NULL DEFAULT 0,
          PRIMARY KEY (snapshot_id, url)
        );
        CREATE INDEX IF NOT EXISTS idx_page_rows_sku ON metric_page_rows(sku);

        -- "Ne yaptık, ne zaman". ⚠️ `reaches_store` merkezi kural: yalnızca Google'ın
        -- göreceği değişiklikler (gönderim, canonical) puanlanıyor; yerel "tamamlandı"
        -- işaretleri zaman çizelgesinde bağlam olarak duruyor.
        CREATE TABLE IF NOT EXISTS work_events (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          at TEXT NOT NULL,
          sku TEXT,
          url TEXT,
          kind TEXT NOT NULL,
          reaches_store INTEGER NOT NULL DEFAULT 0,
          payload_json TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_work_events_sku ON work_events(sku, at);
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
    // IdeaSoft katalog önbelleği. XML feed bilinçli olarak sınırlı (bu mağazada 10.909
    // üründen 262'si); feed dışı sayfalar Google'dan ciddi trafik alıyor ve uygulama
    // onları hiç göremiyordu. Slug ile eşleştirme yapılabilsin diye ayrı tabloda tutulur.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ideasoft_catalog (
            slug TEXT PRIMARY KEY,
            id INTEGER NOT NULL,
            name TEXT NOT NULL,
            status INTEGER NOT NULL,
            stock REAL NOT NULL,
            canonical TEXT,
            synced_at TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("ideasoft_catalog tablosu oluşturulamadı: {e}"))?;
    // AI Asistanı sohbet geçmişi. Kullanıcı geri bildirimi (2026-07-30): uygulama kapanınca
    // sohbet kayboluyordu; meta/açıklama/teknik tabloda olduğu gibi geçmişe erişilebilmeli.
    // ⚠️ Mesajlar JSON sütunda: projenin mevcut geçmiş idiomu bu (`*_history_json`) ve
    // sohbet her zaman bir bütün olarak okunup yazılıyor — satır bazlı sorgu ihtiyacı yok.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS chat_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            tool_page TEXT,
            messages_json TEXT NOT NULL,
            model TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("chat_sessions tablosu oluşturulamadı: {e}"))?;
    // Feed değişikliği tespiti (bkz. core/src/fingerprint.rs).
    // `feed_fp`      → ürünün ŞU ANKİ parmak izi, her senkronda güncellenir (sahibi: senkron).
    // `feed_changed` → son değişiklikte hangi alanların oynadığı, kullanıcıya gösterilir.
    // `reviewed_fp`  → kullanıcı "tamamlandı" işaretlediği andaki iz (sahibi: kullanıcı eylemi).
    // İkisi ayrışırsa ürün "feed verisi değişti, gözden geçir" olarak işaretlenir.
    add_column_if_missing(conn, "products", "feed_fp", "TEXT")?;
    add_column_if_missing(conn, "products", "feed_changed", "TEXT")?;
    add_column_if_missing(conn, "seo_status", "reviewed_fp", "TEXT")?;
    // "Ne değişti?" sorusunun cevabı: onay anındaki alan değerleri (bkz. fingerprint::FeedFacts).
    add_column_if_missing(conn, "seo_status", "reviewed_facts_json", "TEXT")?;
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

/// Kayıtlı feed adresi; **ayarlanmamışsa boş string**.
///
/// 🔴 Burada eskiden `DEFAULT_FEED_URL` sabiti vardı ve tek bir mağazanın (kurumsalit.com)
/// feed adresine düşüyordu. Yani uygulamayı kuran **herhangi bir işletme** varsayılan olarak
/// başka birinin kataloğunu senkronluyordu — vizyonun açık ihlali (*"kullanıcıya özel değer
/// gömmeden, ayarlanabilir olmalı"*) ve ilk çalıştırmada somut bir veri kazası.
///
/// Boş dönmesi bilinçli: çağıranlar "ayarlanmamış" durumunu görüp kullanıcıya ne yapması
/// gerektiğini söyleyebilsin (bkz. `sync_feed`).
pub fn feed_url(conn: &Connection) -> Result<String, String> {
    Ok(get_setting(conn, "feed_url")?.unwrap_or_default())
}

/// Kurulum sihirbazı gösterilmeli mi?
///
/// **Üç koşul birden** aranıyor — çünkü asıl risk yanlış pozitif: mevcut bir kullanıcı
/// yükseltme yaptığında sihirbazın açılması, çalışan bir kurulumun üstüne kurulum teklif
/// etmek olurdu.
///
/// Durum baştan hesaplanıyor; şema göçü ve tek seferlik geriye dönük yazma (backfill) YOK.
pub fn needs_setup(conn: &Connection) -> Result<bool, String> {
    if get_setting(conn, "setup_done")?.is_some() {
        return Ok(false);
    }
    if get_setting(conn, "feed_url")?.is_some_and(|v| !v.trim().is_empty()) {
        return Ok(false);
    }
    let products: i64 = conn
        .query_row("SELECT count(*) FROM products", [], |r| r.get(0))
        .map_err(|e| format!("Ürün sayısı okunamadı: {e}"))?;
    Ok(products == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        conn
    }

    #[test]
    fn taze_kurulumda_sihirbaz_gerekir() {
        let conn = fresh();
        assert!(needs_setup(&conn).unwrap());
        // Feed adresi ayarlanmamışsa BOŞ döner — eskiden tek bir mağazanın adresine düşüyordu.
        assert_eq!(feed_url(&conn).unwrap(), "");
    }

    /// ⚠️ Asıl korunan risk yanlış pozitif: çalışan bir kuruluma sihirbaz teklif etmemek.
    /// Üç çıkış yolunun üçü de ayrı ayrı sınanıyor.
    #[test]
    fn mevcut_kurulumda_sihirbaz_acilmaz() {
        // 1) Sihirbaz daha önce tamamlanmış
        let a = fresh();
        set_setting(&a, "setup_done", "2026-07-31T10:00").unwrap();
        assert!(!needs_setup(&a).unwrap());

        // 2) Feed adresi girilmiş (sihirbaz atlanıp Ayarlar'dan yapılandırılmış olabilir)
        let b = fresh();
        set_setting(&b, "feed_url", "https://magaza.com/feed.xml").unwrap();
        assert!(!needs_setup(&b).unwrap());

        // 3) Katalog senkronlanmış — kullanıcı zaten çalışıyor
        let c = fresh();
        c.execute("INSERT INTO products (sku, name) VALUES ('A1', 'Ürün')", []).unwrap();
        assert!(!needs_setup(&c).unwrap());
    }

    #[test]
    fn bos_feed_ayari_sihirbazi_engellemez() {
        // Boş string "ayarlanmış" sayılmamalı; aksi halde yarım kalan bir kayıt
        // sihirbazı sonsuza dek kapatırdı.
        let conn = fresh();
        set_setting(&conn, "feed_url", "   ").unwrap();
        assert!(needs_setup(&conn).unwrap());
    }
}

/// Ürünün üretimi besleyen alanlarını `products` satırından okur.
///
/// Tek yerde duruyor çünkü iki farklı yol aynı alan kümesini okuyor: senkron (parmak izi
/// hesabı) ve onay damgası (karşılaştırma kaydı). İkisi ayrışırsa "değişti" bayrağı ile
/// gösterilen fark birbirini tutmaz.
pub fn read_feed_facts(conn: &Connection, sku: &str) -> Option<FeedFacts> {
    conn.query_row(
        "SELECT name, brand, main_category, category, details,
                img_url, picture2, picture3, picture4
         FROM products WHERE sku = ?1",
        [sku],
        |r| {
            let g = |i: usize| -> rusqlite::Result<String> {
                Ok(r.get::<_, Option<String>>(i)?.unwrap_or_default())
            };
            Ok(FeedFacts {
                name: g(0)?,
                brand: g(1)?,
                main_category: g(2)?,
                category: g(3)?,
                details: g(4)?,
                images: vec![g(5)?, g(6)?, g(7)?, g(8)?],
            })
        },
    )
    .ok()
}
