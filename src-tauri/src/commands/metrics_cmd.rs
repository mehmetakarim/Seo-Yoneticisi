//! Ölçüm omurgasının Tauri katmanı — anlık görüntüler, olay günlüğü ve sonuç sorguları.
//!
//! Saf mantık `seo_core::metrics`'te; burada yalnızca veritabanı ve GSC erişimi var.
//!
//! ⚠️ Modül adı `metrics_cmd`: `seo_core::metrics` ile ad çakışması yaşamamak için
//! (`commands/history.rs` ↔ `seo_core::history` çakışması daha önce yaşandı, 0ah).

use super::*;
use seo_core::metrics::{self, Outcome, PageRow, Snapshot, WorkEvent};

/// Tohumlamada kaç pencere geriye gidilecek (28 günlük dilimler → ~12 ay).
const SEED_WINDOWS: usize = 12;

/// Olay yazar. **Sessiz başarısız olur**: ölçüm kaydı, asıl işi (gönderim, üretim) engellememeli.
///
/// ⚠️ `reaches_store` merkezi kural: yalnızca Google'ın göreceği değişiklikler puanlanıyor.
/// Yerel "tamamlandı" işaretleri zaman çizelgesinde bağlam olarak duruyor.
pub fn log_event(conn: &Connection, sku: &str, kind: &str, reaches_store: bool) {
    let url: Option<String> = conn
        .query_row("SELECT url FROM products WHERE sku = ?1", [sku], |r| r.get(0))
        .ok()
        .flatten();
    let _ = conn.execute(
        "INSERT INTO work_events (at, sku, url, kind, reaches_store) VALUES (?1,?2,?3,?4,?5)",
        params![now_str(), sku, url, kind, reaches_store as i64],
    );
}

fn snapshots(conn: &Connection) -> Result<Vec<Snapshot>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, captured_at, window_start, window_end, rows, clicks, impressions
             FROM metric_snapshots ORDER BY window_start",
        )
        .map_err(|e| format!("Anlık görüntüler okunamadı: {e}"))?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Snapshot {
                id: r.get(0)?,
                captured_at: r.get(1)?,
                window_start: r.get(2)?,
                window_end: r.get(3)?,
                rows: r.get(4)?,
                clicks: r.get(5)?,
                impressions: r.get(6)?,
            })
        })
        .map_err(|e| format!("Anlık görüntüler okunamadı: {e}"))?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// Ölçülebilir olaylar (mağazaya ulaşanlar), ürün bazında en yenisi.
///
/// Aynı ürüne birden çok gönderim yapılmışsa **en yenisi** ölçülüyor: eski gönderimin etkisi
/// yenisi tarafından zaten ezilmiş olur, ikisini ayrı ayrı puanlamak çift sayım olurdu.
fn latest_store_events(conn: &Connection) -> Result<Vec<WorkEvent>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, at, sku, url, kind, reaches_store FROM work_events e
             WHERE reaches_store = 1 AND sku IS NOT NULL
               AND at = (SELECT MAX(at) FROM work_events x
                         WHERE x.sku = e.sku AND x.reaches_store = 1)
             GROUP BY sku",
        )
        .map_err(|e| format!("Olaylar okunamadı: {e}"))?;
    let rows = stmt
        .query_map([], |r| {
            Ok(WorkEvent {
                id: r.get(0)?,
                at: r.get(1)?,
                sku: r.get(2)?,
                url: r.get(3)?,
                kind: r.get(4)?,
                reaches_store: r.get::<_, i64>(5)? == 1,
            })
        })
        .map_err(|e| format!("Olaylar okunamadı: {e}"))?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// `(snapshot_id, url)` → satır. Sonuç hesabı saf fonksiyon olduğu için erişim buradan veriliyor.
fn row_lookup(conn: &Connection) -> impl Fn(i64, &str) -> Option<PageRow> + '_ {
    move |sid, url| {
        conn.query_row(
            "SELECT url, clicks, impressions, position FROM metric_page_rows
             WHERE snapshot_id = ?1 AND url = ?2",
            params![sid, url],
            |r| {
                Ok(PageRow {
                    url: r.get(0)?,
                    clicks: r.get(1)?,
                    impressions: r.get(2)?,
                    position: r.get(3)?,
                })
            },
        )
        .ok()
    }
}

// ============ Komutlar ============

/// Genel Bakış'taki "Sonuçlar" şeridi.
#[derive(Debug, Serialize, Default)]
pub struct OutcomeSummary {
    /// Kaç anlık görüntü var ve en eskisi hangi tarihte başlıyor.
    pub snapshots: i64,
    pub oldest_window: String,
    /// Mağazaya ulaşan, ürün başına en yeni olay sayısı.
    pub measured_events: i64,
    pub improved: i64,
    pub flat: i64,
    pub worse: i64,
    pub measuring: i64,
    pub insufficient: i64,
    /// İyileşen ve gerileyenlerin net tıklama farkı.
    pub net_delta_clicks: f64,
}

#[tauri::command]
pub fn get_outcome_summary(state: State<'_, AppState>) -> Result<OutcomeSummary, String> {
    let conn = state.conn.lock().unwrap();
    let snaps = snapshots(&conn)?;
    let mut s = OutcomeSummary {
        snapshots: snaps.len() as i64,
        oldest_window: snaps.first().map(|x| x.window_start.clone()).unwrap_or_default(),
        ..Default::default()
    };
    let lookup = row_lookup(&conn);
    for ev in latest_store_events(&conn)? {
        let o = metrics::outcome(&ev, &snaps, &lookup);
        s.measured_events += 1;
        match o.status {
            metrics::OutcomeStatus::Improved => {
                s.improved += 1;
                s.net_delta_clicks += o.delta_clicks();
            }
            metrics::OutcomeStatus::Worse => {
                s.worse += 1;
                s.net_delta_clicks += o.delta_clicks();
            }
            metrics::OutcomeStatus::Flat => s.flat += 1,
            metrics::OutcomeStatus::Measuring => s.measuring += 1,
            _ => s.insufficient += 1,
        }
    }
    Ok(s)
}

/// Fırsatlar tablosundaki "Sonuç" sütunu — sku → (etiket, ton).
#[derive(Debug, Serialize)]
pub struct OutcomeBadge {
    pub sku: String,
    pub label: String,
    pub tone: String,
    /// Rozetin baloncuğu: hangi pencereler kıyaslandı, tıklama nasıl değişti.
    pub tip: String,
}

#[tauri::command]
pub fn get_outcome_badges(state: State<'_, AppState>) -> Result<Vec<OutcomeBadge>, String> {
    let conn = state.conn.lock().unwrap();
    let snaps = snapshots(&conn)?;
    let lookup = row_lookup(&conn);
    let mut out = Vec::new();
    for ev in latest_store_events(&conn)? {
        let Some(sku) = ev.sku.clone() else { continue };
        let o = metrics::outcome(&ev, &snaps, &lookup);
        out.push(OutcomeBadge {
            sku,
            label: o.status.label().to_string(),
            tone: o.status.tone().to_string(),
            tip: tip_of(&ev, &o),
        });
    }
    Ok(out)
}

fn tip_of(ev: &WorkEvent, o: &Outcome) -> String {
    let tarih = ev.at.get(..10).unwrap_or("").to_string();
    match o.status {
        metrics::OutcomeStatus::Measuring => format!(
            "{tarih} tarihinde mağazaya gönderildi · etkinin ölçülebilmesi için en az {} gün bekleniyor",
            metrics::MIN_WAIT_DAYS
        ),
        metrics::OutcomeStatus::NotMeasurable => {
            "Karşılaştırılacak dönem yok — daha eski bir anlık görüntü gerekiyor".into()
        }
        metrics::OutcomeStatus::Insufficient => {
            format!("{tarih} gönderimi · önceki dönemde gösterim çok az, sayı gürültüden ayrılamıyor")
        }
        _ => format!(
            "{tarih} gönderimi · {} → {} ({:+.0} tıklama) · {} ile {} karşılaştırıldı",
            o.clicks_before.round(),
            o.clicks_after.round(),
            o.delta_clicks(),
            o.baseline_window.clone().unwrap_or_default(),
            o.followup_window.clone().unwrap_or_default(),
        ),
    }
}

/// Ürün detayındaki zaman çizelgesi.
#[derive(Debug, Serialize)]
pub struct TimelineItem {
    pub at: String,
    pub kind: String,
    pub label: String,
    /// Yalnızca mağazaya ulaşan olaylarda dolu.
    pub outcome_label: Option<String>,
    pub outcome_tone: Option<String>,
    pub outcome_tip: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProductTimeline {
    pub items: Vec<TimelineItem>,
    /// Hiç mağazaya gönderim yoksa arayüz bunu açıkça söylüyor.
    pub has_store_event: bool,
}

/// Olay türlerinin kullanıcıya dönük adları — tek yerde.
fn kind_label(kind: &str) -> &'static str {
    match kind {
        "ideasoft_push" => "IdeaSoft'a gönderildi",
        "canonical_set" => "Canonical yazıldı",
        "meta_done" => "Meta tamamlandı",
        "details_done" => "Açıklama tamamlandı",
        "tech_done" => "Teknik tablo tamamlandı",
        "feed_ack" => "Feed değişikliği onaylandı",
        _ => "İşlem",
    }
}

#[tauri::command]
pub fn get_product_timeline(
    state: State<'_, AppState>,
    sku: String,
) -> Result<ProductTimeline, String> {
    let conn = state.conn.lock().unwrap();
    let snaps = snapshots(&conn)?;
    let lookup = row_lookup(&conn);

    let mut stmt = conn
        .prepare(
            "SELECT id, at, sku, url, kind, reaches_store FROM work_events
             WHERE sku = ?1 ORDER BY at DESC LIMIT 40",
        )
        .map_err(|e| format!("Zaman çizelgesi okunamadı: {e}"))?;
    let events: Vec<WorkEvent> = stmt
        .query_map([&sku], |r| {
            Ok(WorkEvent {
                id: r.get(0)?,
                at: r.get(1)?,
                sku: r.get(2)?,
                url: r.get(3)?,
                kind: r.get(4)?,
                reaches_store: r.get::<_, i64>(5)? == 1,
            })
        })
        .map_err(|e| format!("Zaman çizelgesi okunamadı: {e}"))?
        .filter_map(Result::ok)
        .collect();

    let has_store_event = events.iter().any(|e| e.reaches_store);
    // Sonuç yalnızca EN YENİ mağaza olayında gösteriliyor: eskisinin etkisi yenisi
    // tarafından ezilmiş olur, her satıra rozet basmak çift sayım izlenimi verir.
    let newest_store = events.iter().find(|e| e.reaches_store).map(|e| e.id);

    let items = events
        .iter()
        .map(|e| {
            let goster = e.reaches_store && Some(e.id) == newest_store;
            let o = goster.then(|| metrics::outcome(e, &snaps, &lookup));
            TimelineItem {
                at: e.at.clone(),
                kind: e.kind.clone(),
                label: kind_label(&e.kind).to_string(),
                outcome_label: o.as_ref().map(|x| x.status.label().to_string()),
                outcome_tone: o.as_ref().map(|x| x.status.tone().to_string()),
                outcome_tip: o.as_ref().map(|x| tip_of(e, x)),
            }
        })
        .collect();

    Ok(ProductTimeline { items, has_store_event })
}

/// Tohumlama + geriye dönük olay dolumu sonucu.
#[derive(Debug, Serialize, Default)]
pub struct SeedResult {
    pub snapshots_added: i64,
    pub rows_written: i64,
    pub events_backfilled: i64,
    pub skipped_existing: i64,
}

/// Geçmişi GSC'den tohumlar ve gönderim olaylarını geriye dönük doldurur.
///
/// 🔬 **Ölçüldü (2026-08-07):** GSC 17 ay geriye veri veriyor, 28 günlük pencere 1,3–2,2 sn.
/// 12 pencere ≈ 25 saniye. Satır eşiği (`metrics::kept`) satırların %34'ünü tutuyor ama
/// tıklamaların %100'ünü kapsıyor.
///
/// **Idempotent:** var olan pencere atlanıyor, var olan olay tekrar yazılmıyor. Kullanıcı
/// yeniden çalıştırırsa veri ikiye katlanmaz.
#[tauri::command]
pub async fn seed_metric_history(state: State<'_, AppState>) -> Result<SeedResult, String> {
    let (gsc_json, gsc_site) = {
        let conn = state.conn.lock().unwrap();
        (
            db::get_setting(&conn, "gsc_service_account_json")?.unwrap_or_default(),
            db::get_setting(&conn, "gsc_site_url")?.unwrap_or_default(),
        )
    };
    if gsc_json.trim().is_empty() || gsc_site.trim().is_empty() {
        return Err("Google Search Console bağlantısı kurulmamış.".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;

    let mut res = SeedResult::default();
    for (start, end) in metrics::windows(chrono::Local::now().date_naive(), SEED_WINDOWS) {
        {
            let conn = state.conn.lock().unwrap();
            let var: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM metric_snapshots WHERE window_start=?1 AND window_end=?2",
                    params![start, end],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            if var > 0 {
                res.skipped_existing += 1;
                continue;
            }
        }
        let stats =
            seo_core::seo_data::gsc::page_stats_range(&client, &gsc_json, gsc_site.trim(), &start, &end, 25_000)
                .await?;
        let conn = state.conn.lock().unwrap();
        let n = write_snapshot(&conn, &start, &end, &stats)?;
        res.snapshots_added += 1;
        res.rows_written += n;
    }

    let conn = state.conn.lock().unwrap();
    res.events_backfilled = backfill_push_events(&conn)?;
    Ok(res)
}

/// Bir pencereyi yazar; eşikten geçen satır sayısını döndürür.
fn write_snapshot(
    conn: &Connection,
    start: &str,
    end: &str,
    stats: &[seo_core::seo_data::PageStat],
) -> Result<i64, String> {
    let tutulan: Vec<&seo_core::seo_data::PageStat> =
        stats.iter().filter(|s| metrics::kept(s.clicks, s.impressions)).collect();
    let clicks: f64 = tutulan.iter().map(|s| s.clicks).sum();
    let imps: f64 = tutulan.iter().map(|s| s.impressions).sum();

    conn.execute(
        "INSERT INTO metric_snapshots (captured_at, window_start, window_end, rows, clicks, impressions)
         VALUES (?1,?2,?3,?4,?5,?6)",
        params![now_str(), start, end, tutulan.len() as i64, clicks, imps],
    )
    .map_err(|e| format!("Anlık görüntü yazılamadı: {e}"))?;
    let sid = conn.last_insert_rowid();

    for s in &tutulan {
        // `sku` eşleşiyorsa yazılıyor: ürün bazlı sorgular böylece tek indeksle çalışıyor.
        let _ = conn.execute(
            "INSERT OR IGNORE INTO metric_page_rows (snapshot_id, url, sku, clicks, impressions, position)
             VALUES (?1, ?2, (SELECT sku FROM products WHERE url = ?2), ?3, ?4, ?5)",
            params![sid, s.page, s.clicks, s.impressions, s.position],
        );
    }
    Ok(tutulan.len() as i64)
}

/// `ideasoft_pushed_at` alanından geriye dönük gönderim olayı üretir.
///
/// 🔴 **Neden yalnızca gönderimler:** ölçüldü (2026-08-07) — `*_history_json` toplam 4 girdi
/// taşıyor ve `seo_status.updated_at` tek/değişken bir alan ("en son bir şey değişti" der).
/// Gerçek olay damgası taşıyan tek alan bu. Zaten SEO açısından anlamlı olan da bu: içerik
/// mağazada yayına çıktığı an.
fn backfill_push_events(conn: &Connection) -> Result<i64, String> {
    let n = conn
        .execute(
            "INSERT INTO work_events (at, sku, url, kind, reaches_store, payload_json)
             SELECT s.ideasoft_pushed_at, s.sku, p.url, 'ideasoft_push', 1, '{\"backfill\":true}'
             FROM seo_status s JOIN products p ON p.sku = s.sku
             WHERE COALESCE(s.ideasoft_pushed_at,'') <> ''
               AND NOT EXISTS (
                 SELECT 1 FROM work_events w
                 WHERE w.sku = s.sku AND w.kind = 'ideasoft_push' AND w.at = s.ideasoft_pushed_at
               )",
            [],
        )
        .map_err(|e| format!("Geçmiş gönderimler yazılamadı: {e}"))?;
    Ok(n as i64)
}

/// Analiz sonunda çağrılır: son anlık görüntü yeterince eskiyse yenisini alır.
///
/// Hata YUTULUYOR: ölçüm kaydı alınamadı diye fırsat analizi başarısız sayılmamalı.
pub async fn snapshot_if_due(
    state: &State<'_, AppState>,
    gsc_json: &str,
    gsc_site: &str,
    client: &reqwest::Client,
) {
    let son_bitis: Option<String> = {
        let conn = state.conn.lock().unwrap();
        conn.query_row("SELECT MAX(window_end) FROM metric_snapshots", [], |r| r.get(0))
            .ok()
            .flatten()
    };
    let bugun = chrono::Local::now().date_naive();
    // Pencereler döşeniyor: sıradaki ancak TAMAMEN geçmişse alınıyor.
    let Some((start, end)) = metrics::next_window(son_bitis.as_deref(), bugun) else { return };
    {
        let conn = state.conn.lock().unwrap();
        let var: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM metric_snapshots WHERE window_start=?1 AND window_end=?2",
                params![start, end],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if var > 0 {
            return;
        }
    }
    if let Ok(stats) =
        seo_core::seo_data::gsc::page_stats_range(client, gsc_json, gsc_site, &start, &end, 25_000).await
    {
        let conn = state.conn.lock().unwrap();
        let _ = write_snapshot(&conn, &start, &end, &stats);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gerçek veritabanı KOPYASI + canlı GSC üzerinde uçtan uca ölçüm.
    ///
    /// `SEO_DB_COPY=/tmp/kopya.db cargo test seed_real -- --ignored --nocapture`
    ///
    /// Üç şeyi ölçer ve kararları buna göre veririz:
    /// 1. Tohumlama maliyeti — süre, satır, veritabanı büyümesi
    /// 2. Geriye dönük dolumun kaç olay ürettiği ve kaçının ölçülebildiği
    /// 3. **Eşik kalibrasyonu** — tıklama deltalarının gerçek dağılımı
    #[tokio::test]
    #[ignore]
    async fn seed_real() {
        let db = std::env::var("SEO_DB_COPY").expect("SEO_DB_COPY yok");
        let conn = Connection::open(&db).unwrap();
        seo_core::db::init(&conn).unwrap();
        let sa = seo_core::db::get_setting(&conn, "gsc_service_account_json").unwrap().unwrap();
        let site = seo_core::db::get_setting(&conn, "gsc_site_url").unwrap().unwrap();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap();

        let boyut = |p: &str| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
        let once = boyut(&db);
        let t0 = std::time::Instant::now();
        let mut toplam_satir = 0i64;
        for (start, end) in metrics::windows(chrono::Local::now().date_naive(), SEED_WINDOWS) {
            let var: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM metric_snapshots WHERE window_start=?1 AND window_end=?2",
                    params![start, end],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            if var > 0 {
                continue;
            }
            let stats = seo_core::seo_data::gsc::page_stats_range(
                &client, &sa, site.trim(), &start, &end, 25_000,
            )
            .await
            .expect("GSC");
            let n = write_snapshot(&conn, &start, &end, &stats).unwrap();
            toplam_satir += n;
            println!("  {start} → {end}: {} satırdan {n} tutuldu", stats.len());
        }
        let sure = t0.elapsed();
        println!(
            "\nTOHUMLAMA: {} pencere · {toplam_satir} satır · {:.1} sn · DB {:.1} → {:.1} MB",
            SEED_WINDOWS,
            sure.as_secs_f64(),
            once as f64 / 1048576.0,
            boyut(&db) as f64 / 1048576.0
        );

        let n = backfill_push_events(&conn).unwrap();
        println!("GERİYE DÖNÜK DOLUM: {n} gönderim olayı");

        // --- Eşik kalibrasyonu: gerçek deltaların dağılımı
        let snaps = snapshots(&conn).unwrap();
        let lookup = row_lookup(&conn);
        let mut olculdu = Vec::new();
        let (mut olcum_bekliyor, mut yetersiz, mut olculemez) = (0, 0, 0);
        for ev in latest_store_events(&conn).unwrap() {
            let o = metrics::outcome(&ev, &snaps, &lookup);
            match o.status {
                metrics::OutcomeStatus::Measuring => olcum_bekliyor += 1,
                metrics::OutcomeStatus::Insufficient => yetersiz += 1,
                metrics::OutcomeStatus::NotMeasurable => olculemez += 1,
                _ => olculdu.push((
                    ev.sku.clone().unwrap_or_default(),
                    o.clicks_before,
                    o.clicks_after,
                    o.delta_clicks(),
                    o.status,
                )),
            }
        }
        println!(
            "\nDURUM: {} ölçüldü · {olcum_bekliyor} ölçülüyor · {yetersiz} veri yetersiz · {olculemez} ölçülemez",
            olculdu.len()
        );
        olculdu.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap());
        println!("\n{:<32} {:>8} {:>8} {:>8}  durum", "sku", "önce", "sonra", "delta");
        for (sku, b, a, d, st) in &olculdu {
            println!("{sku:<32} {b:>8.0} {a:>8.0} {d:>+8.0}  {}", st.label());
        }
        let net: f64 = olculdu.iter().map(|x| x.3).sum();
        println!("\nNET DELTA: {net:+.0} tıklama");
        assert!(!snaps.is_empty(), "hiç anlık görüntü yazılmadı");
    }
}
