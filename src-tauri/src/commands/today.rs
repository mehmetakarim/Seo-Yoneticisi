//! Bugün kuyruğunun Tauri katmanı — girdileri toplar, `seo_core::queue`e verir.
//!
//! Saf seçim mantığı (skor, tekilleştirme, kova sınırı) `seo_core::queue`de; burada yalnızca
//! veritabanı erişimi ve **kovaların tanımı** var.
//!
//! ⚠️ **Yeni GSC çağrısı YOK.** Beş kovanın dördü mevcut `opportunity_json` önbelleğinden
//! besleniyor. Kuyruk açmak kota harcamamalı; analiz eskiyse ekran kaç gün önce çalıştığını
//! yazıyor (`analyzed_at`), kendiliğinden yenilemiyor.

use super::*;
use seo_core::queue::{self, Bucket, Candidate, ItemRef, QueueItem};

/// Ön yüze giden kuyruk + kullanıcının kuyruğu yorumlaması için gereken bağlam.
#[derive(Serialize)]
pub struct TodayQueue {
    pub items: Vec<QueueItem>,
    /// Analiz ne zaman çalıştı — kuyruk bayat mı, kullanıcı görsün.
    pub analyzed_at: String,
    /// Gizlenmiş/ertelenmiş madde sayısı (geri alma düğmesi bunu gösteriyor).
    pub hidden: usize,
    /// Kovaların o anki aday sayısı — boş kova "neden boş" diyebilsin diye.
    pub bucket_counts: Vec<BucketCount>,
    /// Sonuç kontrolü kovası boşsa en erken ne zaman dolacağı (YYYY-AA-GG) — boşsa "".
    ///
    /// ⚠️ Ölçüldü (2026-08-07): 72 gönderimin tamamı 0–12 gün önce yapılmış, bu kova bugün
    /// BOŞ ve Eylül ortasına kadar boş kalacak. Sessizce boş bırakmak yerine tarihi söylüyoruz.
    pub review_ready_at: String,
}

#[derive(Serialize)]
pub struct BucketCount {
    pub bucket: Bucket,
    pub label: String,
    pub candidates: usize,
}

/// Kuyruktan çıkarılmış maddeler: `(kind, ref)` → gizli mi?
///
/// Erteleme süresi dolmuşsa gizli sayılmıyor; kayıt temizlenmesine gerek yok, sorgu bugünün
/// tarihine bakıyor.
fn dismissals(conn: &Connection) -> Vec<(String, String)> {
    let bugun = now_str()[..10].to_string();
    let mut stmt = match conn.prepare(
        "SELECT kind, ref FROM queue_dismissals WHERE until IS NULL OR until > ?1",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map([&bugun], |r| Ok((r.get(0)?, r.get(1)?)))
        .map(|rows| rows.filter_map(Result::ok).collect())
        .unwrap_or_default()
}

fn ref_key(r: &ItemRef) -> (&'static str, String) {
    match r {
        ItemRef::Product(s) => ("product", s.clone()),
        ItemRef::Page(s) => ("page", s.clone()),
    }
}

/// Ürün başına feed/gönderim durumu — acil kovasının kaynağı.
struct ProductState {
    sku: String,
    name: String,
    feed_changed: String,
    pushed_at: Option<String>,
    meta_done: bool,
    details_done: bool,
}

fn product_states(conn: &Connection) -> Vec<ProductState> {
    let mut stmt = match conn.prepare(
        "SELECT p.sku, p.name, COALESCE(p.feed_changed,''), s.ideasoft_pushed_at,
                COALESCE(s.meta_status,''), COALESCE(s.details_status,'')
         FROM products p LEFT JOIN seo_status s ON s.sku = p.sku",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map([], |r| {
        Ok(ProductState {
            sku: r.get(0)?,
            name: r.get(1)?,
            feed_changed: r.get(2)?,
            pushed_at: r.get(3)?,
            meta_done: r.get::<_, String>(4)? == "done",
            details_done: r.get::<_, String>(5)? == "done",
        })
    })
    .map(|rows| rows.filter_map(Result::ok).collect())
    .unwrap_or_default()
}

/// Gün farkı — `at` "YYYY-AA-GGTss:dd:ss" biçiminde.
fn days_since(at: &str) -> i64 {
    let d = at.get(..10).and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    match d {
        Some(d) => (chrono::Local::now().date_naive() - d).num_days(),
        None => 0,
    }
}

/// Beş kovanın adaylarını kurar.
///
/// Her adayın `reason`ı **gerçek bir metrikten** türüyor (yol haritasının şartı): uydurma
/// bir "iyileştirilmeli" cümlesi yok, hepsinde sayı var.
fn candidates(conn: &Connection, report: &OpportunityReport) -> Vec<Candidate> {
    let mut out = Vec::new();
    let states = product_states(conn);
    // Ürünün GSC tıklaması — acil maddelerinin skorunu da tıklama sürüyor.
    let clicks_of: std::collections::HashMap<&str, f64> = report
        .opportunities
        .iter()
        .map(|o| (o.sku.as_str(), o.clicks))
        .collect();

    // --- 1) ACİL: canlıda yanlış içerik ---
    for p in &states {
        // ⚠️ Yalnızca METİN alanı değişmişse ve içerik mağazaya gitmişse acil. Görsel
        // değişikliği üretilmiş metni geçersiz kılmıyor (ölçüm: 70 bayrağın 61'i görsel).
        if p.pushed_at.is_some() && queue::is_urgent_change(&p.feed_changed) {
            out.push(Candidate {
                reference: ItemRef::Product(p.sku.clone()),
                bucket: Bucket::Urgent,
                title: p.name.clone(),
                reason: format!(
                    "mağazaya gönderildikten sonra feed değişti ({}) — canlıdaki metin bayat",
                    p.feed_changed
                ),
                clicks: *clicks_of.get(p.sku.as_str()).unwrap_or(&0.0),
                page: "products".into(),
                focus_id: p.sku.clone(),
                minutes: 2,
            });
        }
        // İş yerelde bitmiş ama mağazaya hiç ulaşmamış → Faz Ö'nün merkezi kuralına göre
        // bu iş Google için HİÇ yapılmamış sayılır ve ölçülemez.
        if p.meta_done && p.details_done && p.pushed_at.is_none() {
            out.push(Candidate {
                reference: ItemRef::Product(p.sku.clone()),
                bucket: Bucket::Urgent,
                title: p.name.clone(),
                reason: "içerik hazır ama mağazaya hiç gönderilmemiş — Google bunu görmüyor".into(),
                clicks: *clicks_of.get(p.sku.as_str()).unwrap_or(&0.0),
                page: "products".into(),
                focus_id: p.sku.clone(),
                minutes: 1,
            });
        }
    }

    // --- 2) YÜKSEK KALDIRAÇ: GSC fırsatları ---
    for o in &report.opportunities {
        out.push(Candidate {
            reference: ItemRef::Product(o.sku.clone()),
            bucket: Bucket::Leverage,
            title: o.name.clone(),
            reason: format!(
                "konum {:.1}, {} gösterim ama {} tıklama — {} tıklama kaçıyor",
                o.position,
                o.impressions.round(),
                o.clicks.round(),
                o.missed_clicks.round()
            ),
            clicks: o.missed_clicks,
            page: "opportunities".into(),
            focus_id: o.sku.clone(),
            minutes: 2,
        });
    }

    // --- 3) KAÇAK TRAFİK: satışta olmayan sayfalar ---
    for e in &report.eol {
        out.push(Candidate {
            // ⚠️ Sayfa kimliği: EOL satırlarında sku YOK, ürün maddeleriyle birleştirilemez.
            reference: ItemRef::Page(e.slug.clone()),
            bucket: Bucket::Leak,
            title: e.slug.clone(),
            reason: format!(
                "{} tıklama satın alınamayan bir sayfaya gidiyor (konum {:.1})",
                e.clicks.round(),
                e.position
            ),
            clicks: e.clicks,
            page: "eol".into(),
            focus_id: e.url.clone(),
            minutes: 1,
        });
    }

    // --- 4) SONUÇ KONTROLÜ: yeterince beklemiş gönderimler ---
    let mut gorulen = std::collections::HashSet::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT sku, MAX(at) FROM work_events
         WHERE reaches_store = 1 AND sku IS NOT NULL GROUP BY sku",
    ) {
        let rows: Vec<(String, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map(|it| it.filter_map(Result::ok).collect())
            .unwrap_or_default();
        for (sku, at) in rows {
            let yas = days_since(&at);
            if yas < queue::REVIEW_AFTER_DAYS || !gorulen.insert(sku.clone()) {
                continue;
            }
            let ad = states
                .iter()
                .find(|p| p.sku == sku)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| sku.clone());
            out.push(Candidate {
                reference: ItemRef::Product(sku.clone()),
                bucket: Bucket::Review,
                title: ad,
                reason: format!("{yas} gün önce mağazaya gönderildi — sonucuna bakılabilir"),
                clicks: 0.0,
                page: "products".into(),
                focus_id: sku,
                minutes: 1,
            });
        }
    }

    // --- 5) BAKIM: düşüşte olanlar + yarışan sayfalar ---
    for d in &report.decay {
        out.push(Candidate {
            reference: ItemRef::Product(d.sku.clone()),
            bucket: Bucket::Upkeep,
            title: d.name.clone(),
            reason: format!(
                "tıklama {}→{}, konum {:.1}→{:.1}",
                d.clicks_before.round(),
                d.clicks_now.round(),
                d.position_before,
                d.position_now
            ),
            clicks: d.clicks_lost,
            page: "decay".into(),
            focus_id: d.sku.clone(),
            minutes: 2,
        });
    }
    for c in &report.cannibalization {
        out.push(Candidate {
            reference: ItemRef::Page(format!("q:{}", c.query)),
            bucket: Bucket::Upkeep,
            title: c.query.clone(),
            reason: format!(
                "{} sayfanız aynı aramada yarışıyor, {} tıklama bölünüyor",
                c.pages.len(),
                c.clicks.round()
            ),
            clicks: c.clicks,
            page: "cannibal".into(),
            focus_id: c.query.clone(),
            minutes: 3,
        });
    }

    out
}

/// Bugünün kuyruğu. **Hesaplanıyor, saklanmıyor** — bkz. `seo_core::queue` modül başlığı.
#[tauri::command]
pub fn get_today_queue(state: State<'_, AppState>) -> Result<TodayQueue, String> {
    let conn = state.conn.lock().unwrap();
    let raw = db::get_setting(&conn, "opportunity_json")?;
    let report: OpportunityReport = match raw {
        Some(j) => serde_json::from_str(&j).unwrap_or_default(),
        None => OpportunityReport::default(),
    };

    let all = candidates(&conn, &report);
    let mut bucket_counts: Vec<BucketCount> = Vec::new();
    for b in [Bucket::Urgent, Bucket::Leverage, Bucket::Leak, Bucket::Review, Bucket::Upkeep] {
        bucket_counts.push(BucketCount {
            bucket: b,
            label: b.label().into(),
            candidates: all.iter().filter(|c| c.bucket == b).count(),
        });
    }

    let gizli = dismissals(&conn);
    let kalan: Vec<Candidate> = all
        .into_iter()
        .filter(|c| {
            let (k, r) = ref_key(&c.reference);
            !gizli.iter().any(|(gk, gr)| gk == k && *gr == r)
        })
        .collect();

    // Sonuç kontrolü kovası boşsa en erken ne zaman dolacak? En yeni gönderimden değil,
    // EN ESKİ ölçülmemiş gönderimden hesaplanıyor — ilk dolacak olan o.
    let review_ready_at = if bucket_counts.iter().any(|b| b.bucket == Bucket::Review && b.candidates > 0) {
        String::new()
    } else {
        conn.query_row(
            "SELECT MIN(at) FROM work_events WHERE reaches_store = 1",
            [],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .and_then(|at| {
            at.get(..10)
                .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        })
        .map(|d| (d + chrono::Duration::days(queue::REVIEW_AFTER_DAYS)).to_string())
        .unwrap_or_default()
    };

    Ok(TodayQueue {
        items: queue::pick(kalan),
        analyzed_at: report.analyzed_at.clone(),
        hidden: gizli.len(),
        bucket_counts,
        review_ready_at,
    })
}

/// Maddeyi kuyruktan çıkarır. `until` yoksa kalıcı gizleme, varsa o tarihe kadar erteleme.
///
/// ⚠️ Kalıcı gizleme **veriyi silmiyor** — ürün kendi ekranında duruyor. Kullanıcı kararı
/// yalnızca "bunu bana her sabah gösterme" demek. Satışta olmayanlar ekranı zaten bunu
/// söylüyordu: *"bazı sayfaları bilinçli tutuyor olabilirsiniz — liste öneridir, karar sizin"*.
#[tauri::command]
pub fn dismiss_queue_item(
    state: State<'_, AppState>,
    kind: String,
    reference: String,
    until: Option<String>,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO queue_dismissals (kind, ref, until, at) VALUES (?1,?2,?3,?4)
         ON CONFLICT(kind, ref) DO UPDATE SET until = ?3, at = ?4",
        params![kind, reference, until, now_str()],
    )
    .map_err(|e| format!("Madde gizlenemedi: {e}"))?;
    Ok(())
}

/// Gizlenmiş/ertelenmiş maddelerin tamamını geri getirir.
#[tauri::command]
pub fn restore_queue_items(state: State<'_, AppState>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute("DELETE FROM queue_dismissals", [])
        .map_err(|e| format!("Geri alınamadı: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gerçek veritabanı KOPYASI üzerinde kuyruğun ne ürettiğini ölçer.
    ///
    /// `SEO_DB_COPY=/tmp/kopya.db cargo test kuyruk_real -- --ignored --nocapture`
    ///
    /// Planın **asıl doğrulaması**: kuyruk tek bir kovaya saplanıyor mu, aynı ürün birden
    /// çok kez giriyor mu, "neden" cümleleri gerçek metrik taşıyor mu.
    #[test]
    #[ignore]
    fn kuyruk_real() {
        let db = std::env::var("SEO_DB_COPY").expect("SEO_DB_COPY yok");
        let conn = Connection::open(&db).unwrap();
        seo_core::db::init(&conn).unwrap();
        let raw = seo_core::db::get_setting(&conn, "opportunity_json").unwrap().unwrap();
        let report: OpportunityReport = serde_json::from_str(&raw).unwrap();

        let all = candidates(&conn, &report);
        println!("aday havuzu: {}", all.len());
        for b in [Bucket::Urgent, Bucket::Leverage, Bucket::Leak, Bucket::Review, Bucket::Upkeep] {
            println!("  {:16} {}", b.label(), all.iter().filter(|c| c.bucket == b).count());
        }

        let q = queue::pick(all);
        println!("\nkuyruk ({} madde):", q.len());
        for (i, it) in q.iter().enumerate() {
            println!(
                "{:2}. [{:14}] {:6.1}  {:.44}\n     → {}",
                i + 1,
                it.bucket.label(),
                it.score,
                it.title,
                it.reason
            );
            if !it.also.is_empty() {
                println!("     ayrıca: {}", it.also.join(" · "));
            }
        }

        // Tekrar eden ürün olmamalı (ölçüm: 12 ürün 2+ kovada).
        let mut kimlikler: Vec<_> = q.iter().map(|i| ref_key(&i.reference)).collect();
        let onceki = kimlikler.len();
        kimlikler.sort();
        kimlikler.dedup();
        assert_eq!(onceki, kimlikler.len(), "aynı madde kuyruğa iki kez girmiş");

        // Kuyruk tek kovaya saplanmamalı.
        let mut kovalar: Vec<_> = q.iter().map(|i| i.bucket).collect();
        kovalar.dedup();
        kovalar.sort_by_key(|b| b.label());
        kovalar.dedup();
        println!("\nkuyruktaki kova sayısı: {}", kovalar.len());
        assert!(kovalar.len() >= 3, "kuyruk {} kovaya saplandı", kovalar.len());
    }
}
