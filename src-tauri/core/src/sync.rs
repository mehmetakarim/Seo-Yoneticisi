use crate::feed::FeedProduct;
use crate::fingerprint::{self, FeedFacts};
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

/// Feed kaydından parmak izi girdilerini toplar (bkz. core/src/fingerprint.rs).
///
/// ⚠️ Yalnızca ÜRETİMİ BESLEYEN alanlar; stok ve mağazadaki mevcut SEO alanları bilinçli
/// olarak dışarıda — gerekçesi fingerprint modülünde yazılı.
fn facts_of(p: &FeedProduct) -> FeedFacts {
    let s = |o: &Option<String>| o.clone().unwrap_or_default();
    FeedFacts {
        name: s(&p.name),
        brand: s(&p.product_brand),
        main_category: s(&p.main_category),
        category: s(&p.category),
        details: s(&p.details),
        images: vec![s(&p.img_url), s(&p.picture2), s(&p.picture3), s(&p.picture4)],
    }
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

        // Eski hâli parmak izi için oku (varsa). Aynı sorgu hem "var mı?" hem "neydi?"
        // sorusunu cevaplıyor — ek sorgu maliyeti yok.
        // Eski hâl + eski not + onay damgası tek sorguda. Aynı sorgu "var mı?" sorusunu da
        // cevaplıyor, ayrı bir SELECT gerekmiyor.
        // Alan okuması `db::read_feed_facts` üzerinden: aynı alan kümesi onay damgasında da
        // okunuyor, ikisi ayrışırsa bayrak ile gösterilen fark birbirini tutmaz.
        let before: Option<(FeedFacts, Option<String>, Option<String>, Option<String>)> =
            crate::db::read_feed_facts(&tx, &sku).map(|facts| {
                let meta: (Option<String>, Option<String>, Option<String>) = tx
                    .query_row(
                        "SELECT p.feed_fp, p.feed_changed, s.reviewed_fp
                         FROM products p LEFT JOIN seo_status s ON s.sku = p.sku
                         WHERE p.sku = ?1",
                        [&sku],
                        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                    )
                    .unwrap_or((None, None, None));
                (facts, meta.0, meta.1, meta.2)
            });

        let now_facts = facts_of(p);
        let fp = fingerprint::fingerprint(&now_facts);
        // Not YALNIZCA gerçekten değişiklik varsa yazılır; yoksa mevcut not korunur
        // (SQL tarafında COALESCE) — kullanıcı henüz gözden geçirmemiş olabilir.
        let changed: Option<String> =
            before.as_ref().and_then(|(b, old_fp, prev_note, reviewed)| {
                let fields = fingerprint::changed_fields(b, &now_facts);
                if fields.is_empty() {
                    return None;
                }
                // Not "onaydan beri neler değişti" demeli, "son senkronda ne değişti" değil:
                // ürün zaten bayraklıysa önceki değişiklikleri de kullanıcı GÖRMEDİ, üstüne
                // yazmak onları sessizce yutardı.
                let already_flagged = matches!((reviewed, old_fp), (Some(r), Some(o)) if r != o);
                let prev: Vec<&str> = match (already_flagged, prev_note.as_deref()) {
                    (true, Some(n)) => n.split(", ").collect(),
                    _ => Vec::new(),
                };
                // FIELDS sırasında birleştir — sıra sabit kalsın, tekrar olmasın.
                let merged: Vec<&str> = fingerprint::FIELDS
                    .iter()
                    .copied()
                    .filter(|f| fields.contains(f) || prev.contains(f))
                    .collect();
                Some(merged.join(", "))
            });

        let exists = before.is_some();
        if exists {
            tx.execute(
                // ⚠️ Fiyat alanları güncelleniyor ama `feed_fp`ye GİRMİYOR (bkz. fingerprint.rs):
                // dolar günlük oynuyor, girseydi her senkronda tüm katalog "değişti" olurdu.
                "UPDATE products SET id=?2, name=?3, brand=?4, main_category=?5, category=?6,
                   quantity=?7, url=?8, img_url=?9, title=?10, descriptions=?11, keywords=?12,
                   search_keywords=?13, details=?14, last_synced_at=?15,
                   picture2=?16, picture3=?17, picture4=?18,
                   feed_fp=?19, feed_changed=COALESCE(?20, feed_changed),
                   buying_price=?21, price1=?22, tax_rate=?23,
                   currency_abbr=?24, price_tl=?25
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
                    p.picture2,
                    p.picture3,
                    p.picture4,
                    fp,
                    changed,
                    p.buying_price_f64(),
                    p.price1_f64(),
                    p.tax_f64(),
                    p.currency_abbr.as_deref(),
                    p.price_tl_f64(),
                ],
            )
            .map_err(|e| format!("Ürün güncellenemedi ({sku}): {e}"))?;
            updated += 1;
        } else {
            tx.execute(
                "INSERT INTO products (sku, id, name, brand, main_category, category, quantity,
                   url, img_url, title, descriptions, keywords, search_keywords, details, last_synced_at,
                   picture2, picture3, picture4, feed_fp, buying_price, price1, tax_rate,
                   currency_abbr, price_tl)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24)",
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
                    p.picture2,
                    p.picture3,
                    p.picture4,
                    fp,
                    p.buying_price_f64(),
                    p.price1_f64(),
                    p.tax_f64(),
                    p.currency_abbr.as_deref(),
                    p.price_tl_f64(),
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

    // Taban çizgisi: özellik gelmeden ÖNCE "tamamlandı" işaretlenmiş ürünlerin damgası yok.
    // Damgasız ürün hiçbir zaman bayraklanmaz — yani mevcut kullanıcının onaylı kataloğu
    // özellikten hiç yararlanamaz. Bir kez, o ürünleri "bugünkü hâliyle onaylanmış" sayıyoruz.
    //
    // ⚠️ Alternatif (hepsini bayraklamak) çok daha kötü: kullanıcı ilk güncellemede tüm
    // kataloğu "değişti" görür, hiçbiri gerçek değildir ve bayrağa güveni biter.
    // Damgası olan satıra dokunmuyor (WHERE reviewed_fp IS NULL) → tekrarlansa da zararsız.
    tx.execute(
        "UPDATE seo_status
            SET reviewed_fp = (SELECT feed_fp FROM products WHERE products.sku = seo_status.sku)
          WHERE reviewed_fp IS NULL
            AND (meta_status = 'done' OR details_status = 'done' OR tech_status = 'done')",
        [],
    )
    .map_err(|e| format!("Onay damgası taban çizgisi yazılamadı: {e}"))?;

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

    /// Gerçek veritabanının KOPYASI + canlı feed üzerinde uçtan uca doğrulama.
    /// Kullanıcının asıl veritabanına dokunmaz — kopya yolu env ile verilir.
    ///
    /// 🔬 Faz T ölçümü: fiyatlar geliyor mu ve **bayrak sayısı ARTIYOR mu**?
    ///
    /// `SEO_DB_COPY=/tmp/k.db SEO_FEED_FILE=/tmp/feed.xml cargo test fiyat_senkron_real -- --ignored --nocapture`
    ///
    /// ⚠️ Asıl sınav ikincisi: fiyat parmak izine sızsaydı dolar her oynadığında tüm katalog
    /// "feed değişti" diye bayraklanır ve acil kovası çöpe dönerdi.
    #[test]
    #[ignore]
    fn fiyat_senkron_real() {
        let db = std::env::var("SEO_DB_COPY").expect("SEO_DB_COPY yok");
        let dosya = std::env::var("SEO_FEED_FILE").expect("SEO_FEED_FILE yok");
        let mut conn = Connection::open(&db).unwrap();
        db::init(&conn).unwrap();

        let bayrak = |c: &Connection| -> i64 {
            c.query_row("SELECT COUNT(*) FROM products WHERE feed_changed IS NOT NULL", [], |r| {
                r.get(0)
            })
            .unwrap()
        };
        let once = bayrak(&conn);

        let xml = std::fs::read_to_string(&dosya).unwrap();
        let urunler = feed::parse(&xml).unwrap();
        println!("feed: {} ürün", urunler.len());
        let ozet = sync_products(&mut conn, urunler).unwrap();
        println!("senkron: {} eklendi · {} güncellendi", ozet.added, ozet.updated);

        let (fiyatli, maliyetli, kdvli): (i64, i64, i64) = conn
            .query_row(
                "SELECT COUNT(price1), COUNT(buying_price), COUNT(tax_rate) FROM products",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();

        // 🔴 Katalog tek para biriminde değil — dağılım yazılıyor.
        let mut stmt = conn
            .prepare("SELECT COALESCE(currency_abbr,'?'), COUNT(*), COUNT(price_tl)
                      FROM products GROUP BY 1 ORDER BY 2 DESC")
            .unwrap();
        let birimler: Vec<(String, i64, i64)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        drop(stmt);
        for (b, n, tl) in &birimler {
            println!("  para birimi {b}: {n} ürün · TL fiyatı dolu {tl}");
        }
        let toplam: i64 =
            conn.query_row("SELECT COUNT(*) FROM products", [], |r| r.get(0)).unwrap();
        println!("fiyat dolu: {fiyatli}/{toplam} · maliyet: {maliyetli} · KDV: {kdvli}");

        let sonra = bayrak(&conn);
        println!("feed bayrağı: {once} → {sonra}");
        assert_eq!(once, sonra, "🔴 fiyat parmak izine sızmış: bayrak sayısı değişti");
        assert_eq!(fiyatli, toplam, "her üründe satış fiyatı olmalı");

        let negatif: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM products WHERE price1 > 0 AND buying_price > price1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        println!("negatif marjlı ürün: {negatif}");
    }

    /// `SEO_DB_COPY=/tmp/kopya.db FEED_URL=... cargo test sync_fingerprint_real -- --ignored --nocapture`
    ///
    /// Ölçtüğü şey: ilk senkronda kaç ürün bayraklanıyor. Beklenen **0** — bu senkron taban
    /// çizgisini kuruyor. İkinci koşuda da 0 olmalı (feed değişmediyse).
    #[tokio::test]
    #[ignore]
    async fn sync_fingerprint_real() {
        let db = std::env::var("SEO_DB_COPY").expect("SEO_DB_COPY yok");
        let url = std::env::var("FEED_URL").expect("FEED_URL yok");
        let mut conn = Connection::open(&db).unwrap();
        db::init(&conn).unwrap();

        for tur in 1..=2 {
            let items = feed::fetch_and_parse(&url).await.expect("feed");
            let s = sync_products(&mut conn, items).unwrap();
            let flagged: Vec<(String, String)> = conn
                .prepare(
                    "SELECT p.sku, COALESCE(p.feed_changed,'?') FROM products p
                     JOIN seo_status s ON s.sku = p.sku
                     WHERE s.reviewed_fp IS NOT NULL AND s.reviewed_fp <> p.feed_fp",
                )
                .unwrap()
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .unwrap()
                .filter_map(Result::ok)
                .collect();
            let stamped: i64 = conn
                .query_row("SELECT COUNT(*) FROM seo_status WHERE reviewed_fp IS NOT NULL", [], |r| r.get(0))
                .unwrap();
            println!(
                "tur {tur}: {} ürün · güncellenen {} · damgalı {stamped} · bayraklı {}",
                s.active, s.updated, flagged.len()
            );
            for (sku, note) in &flagged {
                println!("  ⚑ {sku} → {note}");
            }
            assert!(flagged.is_empty(), "feed değişmediği hâlde bayrak çıktı: {flagged:?}");
        }
    }

    /// Tek satırda "iz / damga / değişiklik notu" üçlüsü.
    fn fp_state(conn: &Connection, sku: &str) -> (Option<String>, Option<String>, Option<String>) {
        conn.query_row(
            "SELECT p.feed_fp, s.reviewed_fp, p.feed_changed FROM products p
             LEFT JOIN seo_status s ON s.sku = p.sku WHERE p.sku = ?1",
            [sku],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap()
    }

    /// Özelliğin çekirdeği: onaydan SONRA feed değişirse ürün işaretlenmeli.
    #[test]
    fn onaydan_sonra_degisen_feed_isaretlenir() {
        let mut conn = mem_conn();
        sync_products(&mut conn, feed::parse(&feed_xml(&[("A", "Ürün A")])).unwrap()).unwrap();
        conn.execute("UPDATE seo_status SET meta_status='done' WHERE sku='A'", []).unwrap();
        // Taban çizgisi damgası bir sonraki senkronda basılıyor.
        sync_products(&mut conn, feed::parse(&feed_xml(&[("A", "Ürün A")])).unwrap()).unwrap();

        let (fp, reviewed, changed) = fp_state(&conn, "A");
        assert_eq!(fp, reviewed, "damga taban çizgisinde ize eşitlenmedi");
        assert_eq!(changed, None, "değişiklik yokken not yazıldı");

        sync_products(&mut conn, feed::parse(&feed_xml(&[("A", "Ürün A v2")])).unwrap()).unwrap();
        let (fp2, reviewed2, changed2) = fp_state(&conn, "A");
        assert_ne!(fp2, reviewed2, "iz değişti ama damga ile ayrışmadı → bayrak çıkmaz");
        assert_eq!(changed2.as_deref(), Some("ad"), "hangi alanın değiştiği yazılmadı");
        assert_eq!(reviewed2, reviewed, "senkron kullanıcının onay damgasını EZDİ");
    }

    /// Kullanıcı bakmadan iki değişiklik üst üste gelirse İKİSİ de not edilmeli.
    /// Üstüne yazsaydık ilk değişiklik sessizce kaybolurdu — özelliğin bütün amacı bu
    /// sessizliği ortadan kaldırmak.
    #[test]
    fn onaydan_beri_degisen_alanlar_birikir() {
        let mut conn = mem_conn();
        sync_products(&mut conn, feed::parse(&feed_xml(&[("A", "Ürün A")])).unwrap()).unwrap();
        conn.execute("UPDATE seo_status SET meta_status='done' WHERE sku='A'", []).unwrap();
        sync_products(&mut conn, feed::parse(&feed_xml(&[("A", "Ürün A")])).unwrap()).unwrap();

        // 1) ad değişti
        sync_products(&mut conn, feed::parse(&feed_xml(&[("A", "Ürün A v2")])).unwrap()).unwrap();
        assert_eq!(fp_state(&conn, "A").2.as_deref(), Some("ad"));

        // 2) kullanıcı henüz bakmadan açıklama da değişti
        let xml = "<products><product><sku><![CDATA[A]]></sku><name><![CDATA[Ürün A v2]]></name>\
                   <details><![CDATA[yeni açıklama]]></details><quantity>5</quantity>\
                   <status>1</status></product></products>";
        sync_products(&mut conn, feed::parse(xml).unwrap()).unwrap();
        assert_eq!(
            fp_state(&conn, "A").2.as_deref(),
            Some("ad, açıklama"),
            "önceki değişiklik notu ezildi"
        );
    }

    /// 🔴 Ölçülen tuzak: feed `\r\n`, veritabanı `\n` kullanıyor. Normalizasyon olmasaydı
    /// gerçek katalogda 7 ürün ilk senkronda sahte bayrak alacaktı.
    #[test]
    fn sadece_bicim_degisirse_isaretlenmez() {
        let mut conn = mem_conn();
        let xml = |d: &str| {
            format!(
                "<products><product><sku><![CDATA[A]]></sku><name><![CDATA[Ürün A]]></name>\
                 <details><![CDATA[{d}]]></details><quantity>5</quantity><status>1</status></product></products>"
            )
        };
        sync_products(&mut conn, feed::parse(&xml("<p>Bir</p>\n<p>İki</p>")).unwrap()).unwrap();
        conn.execute("UPDATE seo_status SET details_status='done' WHERE sku='A'", []).unwrap();
        sync_products(&mut conn, feed::parse(&xml("<p>Bir</p>\n<p>İki</p>")).unwrap()).unwrap();

        sync_products(&mut conn, feed::parse(&xml("<p>Bir</p>\r\n  <p>İki</p>")).unwrap()).unwrap();
        let (fp, reviewed, changed) = fp_state(&conn, "A");
        assert_eq!(fp, reviewed, "yalnızca satır sonu değişti ama iz kaydı");
        assert_eq!(changed, None, "biçim farkı kullanıcıyı rahatsız etti");
    }

    /// ⚠️ Damga yalnızca "tamamlandı" olan ürünlere basılır; bekleyen ürün için bayrak
    /// anlamsızdır (zaten yapılacak iş olarak duruyor) ve gürültü yaratır.
    #[test]
    fn onaylanmamis_urune_damga_basilmaz() {
        let mut conn = mem_conn();
        sync_products(&mut conn, feed::parse(&feed_xml(&[("A", "Ürün A")])).unwrap()).unwrap();
        sync_products(&mut conn, feed::parse(&feed_xml(&[("A", "Ürün A v2")])).unwrap()).unwrap();
        let (fp, reviewed, _) = fp_state(&conn, "A");
        assert!(fp.is_some(), "iz hiç yazılmadı");
        assert_eq!(reviewed, None, "onaylanmamış ürüne damga basıldı");
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
