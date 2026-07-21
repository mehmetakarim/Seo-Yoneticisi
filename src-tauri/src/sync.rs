use crate::feed::FeedProduct;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize)]
pub struct SyncSummary {
    pub run_at: String,
    pub active: i64,
    pub added: i64,
    pub updated: i64,
    pub deleted: i64,
    pub duplicate_skipped: i64,
}

/// Spec'teki senkron mantığı: sku bazlı upsert (seo_status'a dokunmadan),
/// feed'de olmayanların tam silinmesi (cascade) ve sync_log kaydı.
pub fn sync_products(conn: &mut Connection, feed: Vec<FeedProduct>) -> Result<SyncSummary, String> {
    let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let tx = conn.transaction().map_err(|e| format!("İşlem başlatılamadı: {e}"))?;

    let mut seen: HashSet<String> = HashSet::new();
    let mut added = 0i64;
    let mut updated = 0i64;
    let mut duplicate_skipped = 0i64;

    for p in &feed {
        let sku = match p.sku.as_deref() {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => continue, // sku'suz kayıt eşleştirilemez, atla
        };
        if !seen.insert(sku.clone()) {
            duplicate_skipped += 1;
            continue;
        }

        let exists: bool = tx
            .query_row("SELECT 1 FROM products WHERE sku = ?1", [&sku], |_| Ok(true))
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(false),
                other => Err(format!("Ürün sorgulanamadı: {other}")),
            })?;

        if exists {
            tx.execute(
                "UPDATE products SET id=?2, name=?3, brand=?4, main_category=?5, category=?6,
                   quantity=?7, url=?8, img_url=?9, title=?10, descriptions=?11, keywords=?12,
                   search_keywords=?13, details=?14, last_synced_at=?15
                 WHERE sku=?1",
                params![
                    sku,
                    p.id,
                    p.name.as_deref().unwrap_or(""),
                    p.product_brand,
                    p.main_category,
                    p.category,
                    p.quantity_i64(),
                    p.url,
                    p.img_url,
                    p.title,
                    p.descriptions,
                    p.keywords,
                    p.search_keywords,
                    p.details,
                    now,
                ],
            )
            .map_err(|e| format!("Ürün güncellenemedi ({sku}): {e}"))?;
            updated += 1;
        } else {
            tx.execute(
                "INSERT INTO products (sku, id, name, brand, main_category, category, quantity,
                   url, img_url, title, descriptions, keywords, search_keywords, details, last_synced_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
                params![
                    sku,
                    p.id,
                    p.name.as_deref().unwrap_or(""),
                    p.product_brand,
                    p.main_category,
                    p.category,
                    p.quantity_i64(),
                    p.url,
                    p.img_url,
                    p.title,
                    p.descriptions,
                    p.keywords,
                    p.search_keywords,
                    p.details,
                    now,
                ],
            )
            .map_err(|e| format!("Ürün eklenemedi ({sku}): {e}"))?;
            tx.execute(
                "INSERT INTO seo_status (sku, meta_status, details_status, updated_at)
                 VALUES (?1, 'pending', 'pending', ?2)",
                params![sku, now],
            )
            .map_err(|e| format!("SEO durumu oluşturulamadı ({sku}): {e}"))?;
            added += 1;
        }
    }

    // Düşen ürün temizliği: DB'de olup feed'de olmayan sku'lar
    let db_skus: Vec<String> = {
        let mut stmt = tx
            .prepare("SELECT sku FROM products")
            .map_err(|e| format!("Sku listesi alınamadı: {e}"))?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| format!("Sku listesi okunamadı: {e}"))?;
        rows.filter_map(Result::ok).collect()
    };
    let mut deleted = 0i64;
    for sku in db_skus {
        if !seen.contains(&sku) {
            tx.execute("DELETE FROM products WHERE sku = ?1", [&sku])
                .map_err(|e| format!("Ürün silinemedi ({sku}): {e}"))?;
            deleted += 1;
        }
    }

    let active: i64 = tx
        .query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))
        .map_err(|e| format!("Ürün sayısı alınamadı: {e}"))?;

    tx.execute(
        "INSERT INTO sync_log (run_at, active, added, updated, deleted, duplicate_skipped)
         VALUES (?1,?2,?3,?4,?5,?6)",
        params![now, active, added, updated, deleted, duplicate_skipped],
    )
    .map_err(|e| format!("Senkron kaydı yazılamadı: {e}"))?;

    tx.commit().map_err(|e| format!("İşlem tamamlanamadı: {e}"))?;

    Ok(SyncSummary { run_at: now, active, added, updated, deleted, duplicate_skipped })
}

pub fn last_sync(conn: &Connection) -> Result<Option<SyncSummary>, String> {
    conn.query_row(
        "SELECT run_at, active, added, updated, deleted, duplicate_skipped
         FROM sync_log ORDER BY rowid DESC LIMIT 1",
        [],
        |row| {
            Ok(SyncSummary {
                run_at: row.get(0)?,
                active: row.get(1)?,
                added: row.get(2)?,
                updated: row.get(3)?,
                deleted: row.get(4)?,
                duplicate_skipped: row.get(5)?,
            })
        },
    )
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(format!("Senkron geçmişi okunamadı: {other}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, feed};

    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        db::init(&conn).unwrap();
        conn
    }

    fn feed_xml(products: &[(&str, &str)]) -> String {
        let body: String = products
            .iter()
            .map(|(sku, name)| {
                format!(
                    "<product><sku><![CDATA[{sku}]]></sku><name><![CDATA[{name}]]></name>\
                     <title><![CDATA[{name}]]></title><descriptions><![CDATA[{name}]]></descriptions>\
                     <quantity>5</quantity><status>1</status></product>"
                )
            })
            .collect();
        format!("<products>{body}</products>")
    }

    #[test]
    fn first_sync_inserts_all() {
        let mut conn = mem_conn();
        let items = feed::parse(&feed_xml(&[("A", "Ürün A"), ("B", "Ürün B")])).unwrap();
        let s = sync_products(&mut conn, items).unwrap();
        assert_eq!((s.added, s.updated, s.deleted, s.duplicate_skipped, s.active), (2, 0, 0, 0, 2));
        // seo_status satırları da oluştu
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM seo_status", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 2);
    }

    #[test]
    fn second_sync_updates_and_preserves_seo_status() {
        let mut conn = mem_conn();
        let items = feed::parse(&feed_xml(&[("A", "Ürün A"), ("B", "Ürün B")])).unwrap();
        sync_products(&mut conn, items).unwrap();

        // Kullanıcı A'yı done işaretledi + hedef kelime girdi
        conn.execute(
            "UPDATE seo_status SET meta_status='done', target_keyword='kulaklık' WHERE sku='A'",
            [],
        )
        .unwrap();

        let items2 = feed::parse(&feed_xml(&[("A", "Ürün A v2"), ("B", "Ürün B")])).unwrap();
        let s = sync_products(&mut conn, items2).unwrap();
        assert_eq!((s.added, s.updated, s.deleted), (0, 2, 0));

        let (status, kw): (String, String) = conn
            .query_row(
                "SELECT meta_status, target_keyword FROM seo_status WHERE sku='A'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "done");
        assert_eq!(kw, "kulaklık");
        let name: String = conn
            .query_row("SELECT name FROM products WHERE sku='A'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(name, "Ürün A v2");
    }

    #[test]
    fn dropped_product_is_deleted_with_cascade() {
        let mut conn = mem_conn();
        let items = feed::parse(&feed_xml(&[("A", "Ürün A"), ("B", "Ürün B")])).unwrap();
        sync_products(&mut conn, items).unwrap();

        let items2 = feed::parse(&feed_xml(&[("A", "Ürün A")])).unwrap();
        let s = sync_products(&mut conn, items2).unwrap();
        assert_eq!(s.deleted, 1);
        assert_eq!(s.active, 1);
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM seo_status", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1, "cascade ile seo_status da silinmeli");
    }

    #[test]
    fn duplicate_sku_keeps_first_and_counts() {
        let mut conn = mem_conn();
        let items = feed::parse(&feed_xml(&[("A", "İlk"), ("A", "İkinci"), ("B", "B")])).unwrap();
        let s = sync_products(&mut conn, items).unwrap();
        assert_eq!((s.added, s.duplicate_skipped, s.active), (2, 1, 2));
        let name: String = conn
            .query_row("SELECT name FROM products WHERE sku='A'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(name, "İlk");
    }

    /// Gerçek feed dosyasına karşı uçtan uca parse + senkron.
    /// SEO_FEED_FILE ortam değişkeni bir XML dosyasına işaret ederse çalışır.
    /// `SEO_FEED_FILE=... cargo test real_feed -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_feed_parses_and_syncs() {
        let path = std::env::var("SEO_FEED_FILE").expect("SEO_FEED_FILE ayarlı değil");
        let xml = std::fs::read_to_string(path).expect("feed dosyası okunamadı");
        let items = feed::parse(&xml).expect("parse başarısız");
        println!("Parse edilen ürün: {}", items.len());
        assert!(items.len() > 100, "beklenenden az ürün: {}", items.len());
        // İlk üründe temel alanlar dolu mu?
        let first = &items[0];
        assert!(first.sku.as_deref().map_or(false, |s| !s.is_empty()));
        assert!(first.title.is_some());
        assert!(first.descriptions.is_some());
        assert!(first.details.as_deref().map_or(false, |d| d.contains("<")));
        // quantityStatus trim edildi mi?
        if let Some(qs) = &first.quantity_status {
            assert_eq!(qs, qs.trim());
        }

        let mut conn = mem_conn();
        let s = sync_products(&mut conn, items).unwrap();
        assert_eq!(s.added as usize, s.active as usize);
        assert_eq!(s.updated, 0);
        println!(
            "Senkron → aktif:{} eklenen:{} güncellenen:{} silinen:{} mükerrer:{}",
            s.active, s.added, s.updated, s.deleted, s.duplicate_skipped
        );

        // İkinci kez aynı feed → hepsi güncellenir, ekleme 0
        let xml2 = std::fs::read_to_string(std::env::var("SEO_FEED_FILE").unwrap()).unwrap();
        let items2 = feed::parse(&xml2).unwrap();
        let s2 = sync_products(&mut conn, items2).unwrap();
        assert_eq!(s2.added, 0);
        assert_eq!(s2.updated, s.active);
        assert_eq!(s2.deleted, 0);
    }
}
