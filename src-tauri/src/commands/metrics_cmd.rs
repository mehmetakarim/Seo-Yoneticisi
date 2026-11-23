//! Ölçüm omurgasının Tauri katmanı — anlık görüntüler, olay günlüğü ve sonuç sorguları.
//!
//! Saf mantık `seo_core::metrics`'te; burada yalnızca veritabanı ve GSC erişimi var.
//!
//! ⚠️ Modül adı `metrics_cmd`: `seo_core::metrics` ile ad çakışması yaşamamak için
//! (`commands/history.rs` ↔ `seo_core::history` çakışması daha önce yaşandı, 0ah).

use super::*;
use seo_core::metrics::{self, Outcome, PageRow, Snapshot, WorkEvent};
use std::collections::{HashMap, HashSet};

#[derive(serde::Deserialize)]
pub struct SharedOutcomeInputs {
    snapshots: Vec<Snapshot>,
    events: Vec<WorkEvent>,
    page_rows: Vec<SharedPageRow>,
}

#[derive(serde::Deserialize)]
struct SharedPageRow {
    snapshot_id: i64,
    #[serde(flatten)]
    row: PageRow,
}

impl SharedOutcomeInputs {
    fn with_outcomes<T>(mut self, sku: Option<&str>, compute: impl FnOnce(&[Snapshot], Vec<WorkEvent>, &dyn Fn(i64, &str) -> Option<PageRow>) -> T) -> T {
        self.snapshots.sort_by(|a, b| (&a.window_start, &a.window_end).cmp(&(&b.window_start, &b.window_end)));
        self.events.sort_by(|a, b| (&b.at, b.id).cmp(&(&a.at, a.id)));
        let mut seen = HashSet::new();
        let events = if let Some(sku) = sku {
            self.events.into_iter().filter(|event| event.sku.as_deref() == Some(sku)).take(40).collect()
        } else {
            self.events.into_iter().filter(|event| event.reaches_store &&
                event.sku.as_ref().or(event.url.as_ref()).is_some_and(|key| seen.insert(key.clone()))).collect()
        };
        let mut by_snapshot: HashMap<i64, HashMap<String, PageRow>> = HashMap::new();
        for entry in self.page_rows {
            by_snapshot.entry(entry.snapshot_id).or_default().insert(entry.row.url.clone(), entry.row);
        }
        compute(&self.snapshots, events, &|id, url| by_snapshot.get(&id)?.get(url).cloned())
    }
}

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
            // ⚠️ `sku IS NOT NULL` koşulu KALDIRILDI (Faz İ): içerik gönderimlerinin
            // sku'su yok, kimlikleri URL. Koşul dururken o olaylar yazılıyor ama hiçbir
            // yerde okunmuyordu — rozet çıkmıyor, sonuç kontrolü kovası geri getirmiyordu.
            // Gruplama `COALESCE(sku, url)`: ürün için sku, sayfa için adres.
            "SELECT id, at, sku, url, kind, reaches_store FROM work_events e
             WHERE reaches_store = 1 AND COALESCE(e.sku, e.url) IS NOT NULL
               AND at = (SELECT MAX(at) FROM work_events x
                         WHERE COALESCE(x.sku, x.url) = COALESCE(e.sku, e.url)
                           AND x.reaches_store = 1)
             GROUP BY COALESCE(sku, url)",
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
pub fn get_outcome_summary(state: State<'_, AppState>, shared: Option<SharedOutcomeInputs>) -> Result<OutcomeSummary, String> {
    if let Some(shared) = shared { return Ok(shared.with_outcomes(None, outcome_summary)); }
    let conn = state.conn.lock().unwrap();
    let snaps = snapshots(&conn)?;
    let lookup = row_lookup(&conn);
    Ok(outcome_summary(&snaps, latest_store_events(&conn)?, &lookup))
}

fn outcome_summary(snaps: &[Snapshot], events: Vec<WorkEvent>, lookup: &dyn Fn(i64, &str) -> Option<PageRow>) -> OutcomeSummary {
    let mut s = OutcomeSummary {
        snapshots: snaps.len() as i64,
        oldest_window: snaps.first().map(|x| x.window_start.clone()).unwrap_or_default(),
        ..Default::default()
    };
    for ev in events {
        let o = metrics::outcome(&ev, snaps, &lookup);
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
    s
}

/// Fırsatlar tablosundaki "Sonuç" sütunu — sku → (etiket, ton).
#[derive(Debug, Serialize)]
pub struct OutcomeBadge {
    /// Ürün rozetlerinde SKU; içerik rozetlerinde boş (kimlik `url`).
    pub sku: String,
    /// Sayfa adresi — içerik rozetleri bununla eşleşiyor.
    pub url: String,
    pub label: String,
    pub tone: String,
    /// Rozetin baloncuğu: hangi pencereler kıyaslandı, tıklama nasıl değişti.
    pub tip: String,
}

#[tauri::command]
pub fn get_outcome_badges(state: State<'_, AppState>, shared: Option<SharedOutcomeInputs>) -> Result<Vec<OutcomeBadge>, String> {
    if let Some(shared) = shared { return Ok(shared.with_outcomes(None, outcome_badges)); }
    let conn = state.conn.lock().unwrap();
    let snaps = snapshots(&conn)?;
    let lookup = row_lookup(&conn);
    Ok(outcome_badges(&snaps, latest_store_events(&conn)?, &lookup))
}

fn outcome_badges(snaps: &[Snapshot], events: Vec<WorkEvent>, lookup: &dyn Fn(i64, &str) -> Option<PageRow>) -> Vec<OutcomeBadge> {
    let mut out = Vec::new();
    for ev in events {
        // ⚠️ Artık sku'suz olay ATLANMIYOR: içerik gönderimleri URL ile kimlikleniyor.
        // `metrics::outcome` zaten URL anahtarlı çalışıyordu — eksik olan yalnızca buydu.
        let o = metrics::outcome(&ev, snaps, &lookup);
        out.push(OutcomeBadge {
            sku: ev.sku.clone().unwrap_or_default(),
            url: ev.url.clone().unwrap_or_default(),
            label: o.status.label().to_string(),
            tone: o.status.tone().to_string(),
            tip: tip_of(&ev, &o),
        });
    }
    out
}

fn tip_of(ev: &WorkEvent, o: &Outcome) -> String {
    let tarih = ev.at.get(..10).unwrap_or("").to_string();
    let (action, completed) = if ev.kind == "store_page_publish" {
        ("yayını", "mağazada yayımlandı")
    } else {
        ("gönderimi", "mağazaya gönderildi")
    };
    match o.status {
        metrics::OutcomeStatus::Measuring => format!(
            "{tarih} tarihinde {completed} · etkinin ölçülebilmesi için en az {} gün bekleniyor",
            metrics::MIN_WAIT_DAYS
        ),
        metrics::OutcomeStatus::NotMeasurable => {
            "Karşılaştırılacak dönem yok — daha eski bir anlık görüntü gerekiyor".into()
        }
        metrics::OutcomeStatus::Insufficient => {
            format!("{tarih} {action} · önceki dönemde gösterim çok az, sayı gürültüden ayrılamıyor")
        }
        _ => format!(
            "{tarih} {action} · {} → {} ({:+.0} tıklama) · {} ile {} karşılaştırıldı",
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
        // ⚠️ "elle" kelimesi bilinçli: uygulamanın DOĞRULADIĞI bir gönderim değil, kullanıcının
        // beyanı. Zaman çizelgesinde ikisi karışmamalı.
        "manual_done" => "Elle yapıldı olarak işaretlendi",
        _ => "İşlem",
    }
}

#[tauri::command]
pub fn get_product_timeline(
    state: State<'_, AppState>,
    sku: String,
    shared: Option<SharedOutcomeInputs>,
) -> Result<ProductTimeline, String> {
    if let Some(shared) = shared { return Ok(shared.with_outcomes(Some(&sku), product_timeline)); }
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

    Ok(product_timeline(&snaps, events, &lookup))
}

fn product_timeline(snaps: &[Snapshot], events: Vec<WorkEvent>, lookup: &dyn Fn(i64, &str) -> Option<PageRow>) -> ProductTimeline {
    let has_store_event = events.iter().any(|e| e.reaches_store);
    // Sonuç yalnızca EN YENİ mağaza olayında gösteriliyor: eskisinin etkisi yenisi
    // tarafından ezilmiş olur, her satıra rozet basmak çift sayım izlenimi verir.
    let newest_store = events.iter().find(|e| e.reaches_store).map(|e| e.id);

    let items = events
        .iter()
        .map(|e| {
            let goster = e.reaches_store && Some(e.id) == newest_store;
            let o = goster.then(|| metrics::outcome(e, snaps, &lookup));
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

    ProductTimeline { items, has_store_event }
}

#[test]
fn shared_history_uses_latest_store_event_and_keeps_product_timeline() {
    let events = vec![
        WorkEvent { id: 2, at: "2026-11-02T12:00:00".into(), sku: Some("A".into()), reaches_store: true, ..Default::default() },
        WorkEvent { id: 1, at: "2026-11-01T12:00:00".into(), sku: Some("A".into()), reaches_store: true, ..Default::default() },
        WorkEvent { id: 3, at: "2026-11-03T12:00:00".into(), sku: Some("B".into()), ..Default::default() },
    ];
    let input = |events| SharedOutcomeInputs { snapshots: vec![], events, page_rows: vec![] };
    let summary = input(events.clone()).with_outcomes(None, outcome_summary);
    assert_eq!(summary.measured_events, 1);
    let timeline = input(events).with_outcomes(Some("A"), product_timeline);
    assert!(timeline.has_store_event);
    assert_eq!(timeline.items.len(), 2);
    assert_eq!(timeline.items[0].at, "2026-11-02T12:00:00");
    assert!(timeline.items[0].outcome_label.is_some());
    assert!(timeline.items[1].outcome_label.is_none());
}

#[derive(Serialize)]
pub struct QueryComparison {
    event_day: String,
    before: Option<QueryPeriod>,
    after: Option<QueryPeriod>,
}

#[derive(Serialize)]
struct QueryPeriod {
    start: String,
    end: String,
    days: usize,
    clicks: f64,
    impressions: f64,
    position: Option<f64>,
    ctr: Option<f64>,
}

fn summarize_query_period(
    rows: &[seo_core::seo_data::gsc::DailyQueryRow],
    start: chrono::NaiveDate,
    end: chrono::NaiveDate,
) -> QueryPeriod {
    let start = start.to_string();
    let end = end.to_string();
    let (mut days, mut clicks, mut impressions, mut weighted_position) = (0, 0.0, 0.0, 0.0);
    for row in rows.iter().filter(|row| row.day >= start && row.day <= end && row.impressions > 0.0) {
        days += 1;
        clicks += row.clicks;
        impressions += row.impressions;
        weighted_position += row.position * row.impressions;
    }
    QueryPeriod {
        start,
        end,
        days,
        clicks,
        impressions,
        position: (impressions > 0.0).then(|| weighted_position / impressions),
        ctr: (impressions > 0.0).then(|| clicks / impressions),
    }
}

#[cfg(test)]
#[test]
fn query_period_weights_position_and_excludes_other_days() {
    use seo_core::seo_data::gsc::DailyQueryRow;

    let rows = [
        DailyQueryRow { day: "2026-11-01".into(), clicks: 2.0, impressions: 10.0, position: 2.0 },
        DailyQueryRow { day: "2026-11-07".into(), clicks: 8.0, impressions: 90.0, position: 12.0 },
        DailyQueryRow { day: "2026-11-08".into(), clicks: 10.0, impressions: 100.0, position: 1.0 },
    ];
    let day = |n| chrono::NaiveDate::from_ymd_opt(2026, 11, n).unwrap();
    let before = summarize_query_period(&rows, day(1), day(7));
    assert_eq!(before.days, 2);
    assert_eq!(before.clicks, 10.0);
    assert_eq!(before.position, Some(11.0));
    assert_eq!(before.ctr, Some(0.1));
    let empty = summarize_query_period(&rows, day(9), day(15));
    assert_eq!(empty.days, 0);
    assert_eq!(empty.position, None);
    assert_eq!(empty.ctr, None);
}

#[tauri::command]
pub async fn compare_product_query(
    state: State<'_, AppState>,
    sku: String,
    query: String,
) -> Result<QueryComparison, String> {
    use chrono::{Days, NaiveDate};
    use rusqlite::OptionalExtension;
    use seo_core::seo_data::gsc;

    let query = query.trim();
    if query.is_empty() {
        return Err("Ölçülecek sorguyu girin.".into());
    }
    let (credentials, property, event_at, page) = {
        let conn = state.conn.lock().unwrap();
        let event: Option<(String, Option<String>)> = conn.query_row(
            "SELECT at, url FROM work_events WHERE sku = ?1 AND reaches_store = 1
             ORDER BY at DESC, id DESC LIMIT 1",
            [&sku],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional().map_err(|e| e.to_string())?;
        let (event_at, page) = event
            .ok_or("Bu ürün için mağazaya yansıyan işlem kaydı yok.")?;
        let page = page.filter(|url| !url.trim().is_empty()).ok_or("İşlem kaydında sayfa adresi yok.")?;
        (
            db::get_setting(&conn, "gsc_service_account_json")?.unwrap_or_default(),
            db::get_setting(&conn, "gsc_site_url")?.unwrap_or_default().trim().to_owned(),
            event_at,
            page,
        )
    };
    let event_day = event_at.get(..10)
        .and_then(|day| NaiveDate::parse_from_str(day, "%Y-%m-%d").ok())
        .ok_or("İşlemin tarihi okunamadı.")?;
    let start = event_day - Days::new(7);
    let end = event_day + Days::new(7);
    // GSC'nin son günleri kesinleştirmesine zaman tanı.
    if chrono::Utc::now().date_naive() < end + Days::new(4) {
        return Ok(QueryComparison { event_day: event_day.to_string(), before: None, after: None });
    }
    if credentials.trim().is_empty() || property.is_empty() {
        return Err("Ölçüm için Ayarlar'dan Google Search Console bağlantısını kurun.".into());
    }

    let day_key = event_day.to_string();
    let key = [property.as_str(), page.as_str(), query, day_key.as_str()];
    let cached: Option<String> = {
        let conn = state.conn.lock().unwrap();
        conn.query_row(
            "SELECT daily_rows FROM gsc_query_comparisons
             WHERE property = ?1 AND page = ?2 AND query = ?3 AND event_day = ?4",
            key,
            |row| row.get(0),
        ).optional().map_err(|e| e.to_string())?
    };
    let rows = if let Some(cached) = cached {
        serde_json::from_str(&cached).map_err(|e| e.to_string())?
    } else {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build().map_err(|e| e.to_string())?;
        let rows = gsc::daily_query_rows(&client, &credentials, &property, &page, query, start, end).await?;
        if !rows.is_empty() {
            let encoded = serde_json::to_string(&rows).map_err(|e| e.to_string())?;
            state.conn.lock().unwrap().execute(
                "INSERT OR IGNORE INTO gsc_query_comparisons (property, page, query, event_day, daily_rows)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![property, page, query, day_key, encoded],
            ).map_err(|e| e.to_string())?;
        }
        rows
    };

    Ok(summarize_comparison(event_day, &rows))
}

fn summarize_comparison(
    event_day: chrono::NaiveDate,
    rows: &[seo_core::seo_data::gsc::DailyQueryRow],
) -> QueryComparison {
    use chrono::Days;
    QueryComparison {
        event_day: event_day.to_string(),
        before: Some(summarize_query_period(rows, event_day - Days::new(7), event_day - Days::new(1))),
        after: Some(summarize_query_period(rows, event_day + Days::new(1), event_day + Days::new(7))),
    }
}

#[tauri::command]
pub fn summarize_team_query(
    event_day: String,
    rows: Vec<seo_core::seo_data::gsc::DailyQueryRow>,
) -> Result<QueryComparison, String> {
    let day = chrono::NaiveDate::parse_from_str(&event_day, "%Y-%m-%d")
        .map_err(|_| "Ortak işlemin tarihi okunamadı.")?;
    let window = chrono::Days::new(7);
    if day.checked_sub_days(window).is_none() || day.checked_add_days(window).is_none() {
        return Err("Ortak işlemin tarihi okunamadı.".into());
    }
    Ok(summarize_comparison(day, &rows))
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
pub async fn seed_metric_history(state: State<'_, AppState>, measurement_scope: Option<String>) -> Result<SeedResult, String> {
    if measurement_scope.is_some() {
        return Err("Ekip geçmişini ortak GSC üzerinden alın.".into());
    }
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
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let captured_at = now_str();
    let inserted = tx.execute(
        "INSERT INTO metric_snapshots (captured_at, window_start, window_end, rows, clicks, impressions)
         VALUES (?1,?2,?3,0,0,0) ON CONFLICT(window_start, window_end) DO NOTHING",
        params![captured_at, start, end],
    )
    .map_err(|e| format!("Anlık görüntü yazılamadı: {e}"))?;
    if inserted == 0 { return Ok(0); }
    let sid = tx.last_insert_rowid();

    for s in &tutulan {
        // `sku` eşleşiyorsa yazılıyor: ürün bazlı sorgular böylece tek indeksle çalışıyor.
        tx.execute(
            "INSERT OR IGNORE INTO metric_page_rows (snapshot_id, url, sku, clicks, impressions, position)
             VALUES (?1, ?2, (SELECT sku FROM products WHERE url = ?2), ?3, ?4, ?5)",
            params![sid, s.page, s.clicks, s.impressions, s.position],
        ).map_err(|e| e.to_string())?;
    }
    let (row_count, clicks, imps): (i64, f64, f64) = tx.query_row(
        "SELECT count(*), coalesce(sum(clicks),0), coalesce(sum(impressions),0) FROM metric_page_rows WHERE snapshot_id=?1",
        [sid], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|e| e.to_string())?;
    tx.execute("UPDATE metric_snapshots SET rows=?2, clicks=?3, impressions=?4 WHERE id=?1", params![sid, row_count, clicks, imps])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(row_count)
}

#[tauri::command]
pub fn team_metric_outbox(state: State<'_, AppState>, scope: String, acknowledge: Option<i64>) -> Result<serde_json::Value, String> {
    use rusqlite::OptionalExtension;
    let conn = state.conn.lock().unwrap();
    if let Some(id) = acknowledge {
        conn.execute("DELETE FROM team_metric_outbox WHERE scope=?1 AND id=?2", params![scope,id]).map_err(|e| e.to_string())?;
        return Ok(serde_json::Value::Null);
    }
    let row: Option<(i64, String, String)> = conn.query_row(
        "SELECT id, snapshot, page_rows FROM team_metric_outbox WHERE scope=?1 ORDER BY id LIMIT 1",
        [scope], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).optional().map_err(|e| e.to_string())?;
    let Some((id, snapshot, rows)) = row else { return Ok(serde_json::Value::Null); };
    let snapshot: serde_json::Value = serde_json::from_str(&snapshot).map_err(|e| e.to_string())?;
    let rows: serde_json::Value = serde_json::from_str(&rows).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "id": id, "snapshot": snapshot, "page_rows": rows }))
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
    let last: Option<String> = state.conn.lock().unwrap()
        .query_row("SELECT MAX(window_end) FROM metric_snapshots", [], |r| r.get(0))
        .ok().flatten();
    let Some((start, end)) = metrics::next_window(last.as_deref(), chrono::Local::now().date_naive()) else { return };
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

#[tauri::command]
pub fn gsc_snapshot_window(
    state: State<'_, AppState>,
    measurement_scope: String,
    shared_window_end: Option<String>,
) -> Result<Option<(String, String)>, String> {
    let conn = state.conn.lock().unwrap();
    let queued_end: Option<String> = conn.query_row(
        "SELECT MAX(json_extract(snapshot,'$.window_end')) FROM team_metric_outbox WHERE scope=?1",
        [measurement_scope],
        |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    let last = queued_end.as_deref().into_iter().chain(shared_window_end.as_deref()).max();
    Ok(metrics::next_window(last, chrono::Utc::now().date_naive()))
}

#[tauri::command]
pub fn team_gsc_seed_windows(shared_window_end: Option<String>) -> Vec<(String, String)> {
    seed_windows(chrono::Utc::now().date_naive(), shared_window_end.as_deref())
}

fn seed_windows(today: chrono::NaiveDate, shared_window_end: Option<&str>) -> Vec<(String, String)> {
    use chrono::{Duration, NaiveDate};
    let lag = Duration::days(metrics::GSC_LAG_DAYS);
    let latest = today - lag;
    let aligned = shared_window_end
        .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
        .filter(|end| *end <= latest)
        .map(|end| {
            let window = metrics::WINDOW_DAYS + 1;
            let advance = ((latest - end).num_days() / window) * window;
            end + Duration::days(advance)
        })
        .unwrap_or(latest);
    metrics::windows(aligned + lag, SEED_WINDOWS)
}

#[derive(serde::Deserialize)]
pub struct SharedSnapshot {
    pub start: String,
    pub end: String,
    pub rows: Vec<seo_core::seo_data::PageStat>,
}

#[tauri::command]
pub fn queue_team_gsc_snapshot(state: State<'_, AppState>, scope: String, snapshot: SharedSnapshot) -> Result<bool, String> {
    queue_shared_snapshot(&state, &scope, snapshot)
}

pub fn queue_shared_snapshot(state: &State<'_, AppState>, scope: &str, snapshot: SharedSnapshot) -> Result<bool, String> {
    stage_shared_snapshot(&state.conn.lock().unwrap(), scope, snapshot)
}

fn stage_shared_snapshot(conn: &Connection, scope: &str, snapshot: SharedSnapshot) -> Result<bool, String> {
    use chrono::{Days, Duration, NaiveDate, Utc};
    let start = NaiveDate::parse_from_str(&snapshot.start, "%Y-%m-%d").map_err(|e| e.to_string())?;
    let end = NaiveDate::parse_from_str(&snapshot.end, "%Y-%m-%d").map_err(|e| e.to_string())?;
    let today = Utc::now().date_naive();
    if scope.is_empty() || scope.len() > 512 || start.checked_add_days(Days::new(metrics::WINDOW_DAYS as u64)) != Some(end)
        || end > today - Duration::days(metrics::GSC_LAG_DAYS) || start < today - Duration::days(540) || snapshot.rows.len() > 25_000 {
        return Err("Geçersiz GSC ölçüm penceresi.".into());
    }

    let mut seen = HashSet::new();
    let mut rows = Vec::new();
    for row in &snapshot.rows {
        if row.page.trim().is_empty() || [row.clicks, row.impressions, row.position].iter().any(|value| !value.is_finite() || *value < 0.0) {
            return Err("Ortak GSC satırı geçersiz.".into());
        }
        if metrics::kept(row.clicks, row.impressions) && seen.insert(row.page.as_str()) {
            rows.push(serde_json::json!({ "url": row.page, "clicks": row.clicks,
                "impressions": row.impressions, "position": row.position }));
        }
    }
    let head = serde_json::json!({
        "window_start": snapshot.start,
        "window_end": snapshot.end,
        "captured_at": Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        "row_count": rows.len(),
    });
    let inserted = conn.execute(
        "INSERT INTO team_metric_outbox(scope, snapshot, page_rows)
         SELECT ?1, ?2, ?3 WHERE NOT EXISTS (
           SELECT 1 FROM team_metric_outbox WHERE scope=?1
             AND json_extract(snapshot,'$.window_start') = ?4 AND json_extract(snapshot,'$.window_end') = ?5
         )",
        params![scope, head.to_string(), serde_json::Value::Array(rows).to_string(), snapshot.start, snapshot.end],
    ).map_err(|e| e.to_string())?;
    Ok(inserted > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn team_snapshots_dedupe_by_scope_without_writing_legacy_history() {
        let conn = Connection::open_in_memory().unwrap();
        db::init(&conn).unwrap();
        let stat = seo_core::seo_data::PageStat {
            page: "https://example.test/product".into(), clicks: 3.0, impressions: 20.0, ctr: 0.15, position: 6.0,
        };
        let (start, end) = metrics::windows(chrono::Utc::now().date_naive(), 1).remove(0);
        let snapshot = || SharedSnapshot { start: start.clone(), end: end.clone(), rows: vec![stat.clone(), stat.clone()] };
        assert!(stage_shared_snapshot(&conn, "team-a", snapshot()).unwrap());
        assert!(!stage_shared_snapshot(&conn, "team-a", snapshot()).unwrap());
        assert!(stage_shared_snapshot(&conn, "team-b", snapshot()).unwrap());
        let (queued, count, clicks): (i64, i64, f64) = conn.query_row(
            "SELECT count(*), json_array_length(page_rows), json_extract(page_rows,'$[0].clicks') FROM team_metric_outbox WHERE scope='team-a'",
            [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).unwrap();
        assert_eq!((queued, count, clicks), (1, 1, 3.0));
        let local: i64 = conn.query_row("SELECT count(*) FROM metric_snapshots", [], |r| r.get(0)).unwrap();
        assert_eq!(local, 0);
        assert_eq!(write_snapshot(&conn, &start, &end, &[stat]).unwrap(), 1);
    }

    #[test]
    fn seed_windows_extend_the_existing_cadence_without_overlapping() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 11, 22).unwrap();
        let windows = seed_windows(today, Some("2026-09-15"));
        assert_eq!(windows.len(), SEED_WINDOWS);
        assert_eq!(windows[0], ("2026-10-15".into(), "2026-11-12".into()));
        assert_eq!(windows[1], ("2026-09-16".into(), "2026-10-14".into()));
        assert_eq!(windows[2].1, "2026-09-15");
    }

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
