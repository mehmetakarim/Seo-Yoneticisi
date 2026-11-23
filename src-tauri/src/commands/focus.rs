//! Odak seansı — kuyruğu sürdürülebilir bir tempoda tüketmek (Faz S).
//!
//! # 🚫 Bu bir oyunlaştırma değil
//!
//! Yol haritasının açık yasağı, koda da yazılıyor: **XP yok · lig yok · seri (streak) cezası
//! yok · konfeti yok · "Harikasın! 🔥" yok.** Seans özeti sakin bir bilanço: kaç iş, ne kadar
//! sürdü, hangi kovalar. Ertesi sabah yine açılacak bir ekranda coşku yorar.
//!
//! # 🔑 Seansın asıl ürünü: gerçek süre ölçümü
//!
//! Kuyruktaki dakikalar bugüne kadar **elle yazılmış tahminlerdi** (`today.rs`'te 1–3 dk) ve
//! kodda da ekranda da "tahmin, ölçüm değil" diye işaretliydi. Bu modül duvar saati süresini
//! ölçüyor — düşünme ve düzenleme dahil, çünkü asıl bilinmeyen o.
//!
//! ⚠️ Neden `work_events` yetmiyordu (ölçüldü, 2026-08-08): olay günlüğü yalnızca uç noktaları
//! yakalıyor. Aynı ürün için `meta_done → ideasoft_push` farkı **0,6 dakika** çıktı — bu işin
//! süresi değil, gönderim işleminin süresi. Kullanıcının düşünme süresi hiçbir yerde yoktu.
//!
//! # ⚠️ Aynı anda tek kilitli iş
//!
//! Projenin "toplu değil, gerektiğinde ve tek tek" kuralının seans karşılığı.

use super::*;
use seo_core::queue;

/// Seans ve mola süresinin varsayılanı (dakika) — yol haritasındaki 25/5.
///
/// ⚠️ Ölçüm (2026-08-08) bugünün kuyruğunun tamamını **16 dakika** tahmin ediyor, yani 25
/// dakika bugün için uzun. Yine de varsayılan eğilip bükülmedi: o 16 dakika zaten uydurma
/// tahminlerden geliyor ve bu modül onları ölçüyle değiştirecek. Süreler gerçekleşince
/// varsayılan yeniden değerlendirilmeli.
const DEFAULT_WORK: i64 = 25;
const DEFAULT_BREAK: i64 = 5;

/// Ön yüze giden seans durumu. Seans yoksa `session_id` `None`.
#[derive(Serialize)]
pub struct FocusState {
    pub session_id: Option<i64>,
    pub started_at: String,
    pub planned_minutes: i64,
    pub break_minutes: i64,
    /// O an kilitli iş — yoksa kuyruk tükenmiş demektir.
    pub locked: Option<LockedItem>,
    /// Bu seansta bitirilen iş sayısı (özet ve çubuk için).
    pub done_count: i64,
    /// Bu seansta atlanan iş sayısı.
    pub skipped_count: i64,
}

#[derive(Serialize, Clone)]
pub struct LockedItem {
    pub item_id: i64,
    pub kind: String,
    pub reference: String,
    pub bucket: String,
    #[serde(flatten)]
    context: LockedContext,
    /// Bu iş ne zaman kilitlendi — çubuk "bu işte 4 dakikadır" diyebilsin diye.
    pub started_at: String,
}

#[derive(Clone, Default, Serialize, serde::Deserialize)]
struct LockedContext {
    title: String,
    reason: String,
    page: String,
    focus_id: String,
}

impl From<&queue::QueueItem> for LockedContext {
    fn from(item: &queue::QueueItem) -> Self {
        Self {
            title: item.title.clone(),
            reason: item.reason.clone(),
            page: item.page.clone(),
            focus_id: item.focus_id.clone(),
        }
    }
}

/// Seans özeti — **sakin bir bilanço**, kutlama değil.
#[derive(Serialize)]
pub struct FocusSummary {
    pub done_count: i64,
    pub skipped_count: i64,
    /// Seansın toplam süresi (dakika).
    pub minutes: f64,
    /// Neden bitti: `queue_empty` | `time_up` | `stopped`.
    pub ended_reason: String,
    /// Bitirilen işlerin kovaları ve adetleri.
    pub buckets: Vec<(String, i64)>,
}

/// ⚠️ `pub(crate)`: Faz C'nin sessizlik eşiği de bunu okuyor. Kopyalanmadı — `.filter(> 0)`
/// davranışı orada da doğru: eşikte **0 = kapalı** ve fallback de 0.
pub(crate) fn setting_i64(conn: &Connection, key: &str, fallback: i64) -> i64 {
    db::get_setting(conn, key)
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(fallback)
}

fn session_scope(scope: Option<&str>) -> Result<&str, String> {
    match scope {
        None => Ok(""),
        Some(value) if value.trim().is_empty() || value.len() > 512 => {
            Err("Odak seansının çalışma alanı geçersiz.".into())
        }
        Some(value) => Ok(value),
    }
}

fn session_queue(
    conn: &Connection,
    scope: &str,
    shared: Option<Vec<queue::QueueItem>>,
) -> Result<Vec<queue::QueueItem>, String> {
    if !scope.is_empty() {
        return Ok(shared.unwrap_or_default());
    }
    if shared.is_some() {
        return Err("Ortak odak kuyruğu için çalışma alanı gerekli.".into());
    }
    Ok(super::build_today_queue(conn)?.items)
}

fn open_session(conn: &Connection, scope: &str) -> Option<i64> {
    conn.query_row(
        "SELECT id FROM focus_sessions WHERE scope = ?1 AND ended_at IS NULL
         ORDER BY id DESC LIMIT 1",
        [scope],
        |r| r.get(0),
    )
    .ok()
}

/// Açık seansın kilitli maddesi (henüz sonuçlanmamış satır).
fn open_item(conn: &Connection, session_id: i64) -> Option<LockedItem> {
    conn.query_row(
        "SELECT id, kind, ref, bucket, started_at, queue_item FROM focus_session_items
         WHERE session_id = ?1 AND outcome IS NULL ORDER BY id DESC LIMIT 1",
        [session_id],
        |r| {
            Ok(LockedItem {
                item_id: r.get(0)?,
                kind: r.get(1)?,
                reference: r.get(2)?,
                bucket: r.get(3)?,
                started_at: r.get(4)?,
                context: r.get::<_, Option<String>>(5)?
                    .and_then(|value| serde_json::from_str(&value).ok())
                    .unwrap_or_default(),
            })
        },
    )
    .ok()
}

/// Kuyruktan, bu seansta **henüz dokunulmamış** ilk işi seçer.
///
/// ⚠️ "Yapıldı" işaretli maddeler kuyrukta duruyor (Faz K: gün bitebilsin diye) ama seansa
/// tekrar kilitlenmemeli. Atlananlar da bu seans içinde tekrar gelmemeli — ama kalıcı bir
/// karar yazılmıyor, yarın yine kuyrukta olacaklar.
fn next_item<'a>(
    conn: &Connection,
    session_id: i64,
    items: &'a [queue::QueueItem],
) -> Option<&'a queue::QueueItem> {
    let mut dokunulan: Vec<(String, String)> = Vec::new();
    if let Ok(mut stmt) =
        conn.prepare("SELECT kind, ref FROM focus_session_items WHERE session_id = ?1")
    {
        if let Ok(rows) = stmt.query_map([session_id], |r| Ok((r.get(0)?, r.get(1)?))) {
            dokunulan = rows.filter_map(Result::ok).collect();
        }
    }

    items.iter().find(|it| {
        if it.done {
            return false;
        }
        let (k, r) = super::ref_key(&it.reference);
        !dokunulan.iter().any(|(dk, dr)| dk == k && dr == &r)
    })
}

/// Kilitli maddenin ekran bilgilerini (başlık, sebep, hedef) kuyruktan tazeler.
///
/// Ortak madde listeden çıkarsa kilitlendiği andaki ekran bilgileri kullanılır.
fn enrich(items: &[queue::QueueItem], mut it: LockedItem) -> LockedItem {
    if let Some(item) = items.iter().find(|item| {
        let (kind, reference) = super::ref_key(&item.reference);
        kind == it.kind && reference == it.reference
    }) {
        it.context = LockedContext::from(item);
    }
    if it.context.title.is_empty() {
        it.context.title = it.reference.clone();
    }
    it
}

fn sayac(conn: &Connection, session_id: i64, outcome: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM focus_session_items WHERE session_id = ?1 AND outcome = ?2",
        params![session_id, outcome],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

fn durum(conn: &Connection, scope: &str, items: &[queue::QueueItem]) -> FocusState {
    let Some(id) = open_session(conn, scope) else {
        return FocusState {
            session_id: None,
            started_at: String::new(),
            planned_minutes: setting_i64(conn, "focus_work_minutes", DEFAULT_WORK),
            break_minutes: setting_i64(conn, "focus_break_minutes", DEFAULT_BREAK),
            locked: None,
            done_count: 0,
            skipped_count: 0,
        };
    };
    let (started_at, planned, brk): (String, i64, i64) = conn
        .query_row(
            "SELECT started_at, planned_minutes, break_minutes FROM focus_sessions WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap_or_else(|_| (now_str(), DEFAULT_WORK, DEFAULT_BREAK));

    FocusState {
        session_id: Some(id),
        started_at,
        planned_minutes: planned,
        break_minutes: brk,
        locked: open_item(conn, id).map(|it| enrich(items, it)),
        done_count: sayac(conn, id, "done"),
        skipped_count: sayac(conn, id, "skipped"),
    }
}

/// Yeni bir seans başlatır ve ilk işi kilitler.
///
/// 🔴 **Kilitlenecek iş yoksa seans AÇILMAZ** (saha hatası, 2026-08-08). İlk sürüm önce kaydı
/// açıyor, sonra kilitleyecek iş bulamayınca hemen kapatıyordu: kullanıcı "Odak seansı başlat"
/// deyince karşısına "Seans bitti · 0 iş · 0 dk" modali çıkıyordu — bu bir başarısızlık gibi
/// okunuyor, oysa günün işi bitmiş demek. Üstelik her denemede sıfır saniyelik bir seans kaydı
/// birikiyordu (kullanıcının veritabanında 8 tane bulundu).
///
/// Artık `session_id` `None` dönüyor ve ekran bunu sakin bir bilgiyle karşılıyor.
#[tauri::command]
pub fn start_focus_session(
    state: State<'_, AppState>,
    queue_scope: Option<String>,
    queue_items: Option<Vec<queue::QueueItem>>,
) -> Result<FocusState, String> {
    let conn = state.conn.lock().unwrap();
    let scope = session_scope(queue_scope.as_deref())?;
    let items = session_queue(&conn, scope, queue_items)?;
    start_session(&conn, scope, &items)
}

fn start_session(conn: &Connection, scope: &str, items: &[queue::QueueItem]) -> Result<FocusState, String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    if open_session(&tx, scope).is_some() || next_item(&tx, 0, items).is_none() {
        return Ok(durum(&tx, scope, items));
    }
    let work = setting_i64(&tx, "focus_work_minutes", DEFAULT_WORK);
    let brk = setting_i64(&tx, "focus_break_minutes", DEFAULT_BREAK);
    tx.execute(
        "INSERT INTO focus_sessions (scope, started_at, planned_minutes, break_minutes)
         VALUES (?1, ?2, ?3, ?4)",
        params![scope, now_str(), work, brk],
    )
    .map_err(|e| format!("Seans başlatılamadı: {e}"))?;
    let id = tx.last_insert_rowid();
    lock_next(&tx, id, scope, items)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(durum(conn, scope, items))
}

/// Kuyrukta seansa kilitlenebilecek bir iş var mı — ekran düğmeyi buna göre açıyor.
#[tauri::command]
pub fn has_lockable_item(
    state: State<'_, AppState>,
    queue_scope: Option<String>,
    queue_items: Option<Vec<queue::QueueItem>>,
) -> Result<bool, String> {
    let conn = state.conn.lock().unwrap();
    let scope = session_scope(queue_scope.as_deref())?;
    let items = session_queue(&conn, scope, queue_items)?;
    Ok(next_item(&conn, 0, &items).is_some())
}

/// Sıradaki işi kilitler. Kuyruk tükenmişse hiçbir şey yapmaz (çağıran seansı bitirir).
fn lock_next(
    conn: &Connection,
    session_id: i64,
    scope: &str,
    items: &[queue::QueueItem],
) -> Result<bool, String> {
    let Some(item) = next_item(conn, session_id, items) else { return Ok(false) };
    let (kind, reference) = super::ref_key(&item.reference);
    let bucket = format!("{:?}", item.bucket).to_lowercase();
    let context = (!scope.is_empty())
        .then(|| serde_json::to_string(&LockedContext::from(item)))
        .transpose()
        .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO focus_session_items (session_id, kind, ref, bucket, started_at, queue_item)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![session_id, kind, reference, bucket, now_str(), context],
    ).map_err(|e| format!("İş kilitlenemedi: {e}"))?;
    Ok(true)
}

/// Kilitli işi sonuçlandırır ve sıradakini kilitler.
///
/// `outcome`:
/// - `done` → çağıran ortak veya yerel kuyruk kararını önce kaydeder
/// - `skipped` → **yalnızca bu seans için**; kuyrukta kalır, kalıcı karar yazılmaz
/// - `dismissed` → çağıran zaten `dismiss_queue_item`i çalıştırdı, burada sadece kaydediliyor
#[tauri::command]
pub fn resolve_focus_item(
    state: State<'_, AppState>,
    outcome: String,
    queue_scope: Option<String>,
    queue_items: Option<Vec<queue::QueueItem>>,
    expected_session_id: Option<i64>,
    expected_item_id: Option<i64>,
) -> Result<FocusState, String> {
    let conn = state.conn.lock().unwrap();
    let scope = session_scope(queue_scope.as_deref())?;
    let items = session_queue(&conn, scope, queue_items)?;
    resolve_item(&conn, scope, &items, &outcome, expected_session_id, expected_item_id)
}

fn resolve_item(
    conn: &Connection,
    scope: &str,
    items: &[queue::QueueItem],
    outcome: &str,
    expected_session_id: Option<i64>,
    expected_item_id: Option<i64>,
) -> Result<FocusState, String> {
    if !matches!(outcome, "done" | "skipped" | "dismissed") {
        return Err("Geçersiz odak seansı işlemi.".into());
    }
    if !scope.is_empty() && (expected_session_id.is_none() || expected_item_id.is_none()) {
        return Err("Odak seansını yenileyin.".into());
    }
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let Some(id) = open_session(&tx, scope) else {
        return Ok(durum(&tx, scope, items));
    };
    if expected_session_id.is_some_and(|expected| expected != id) {
        return Err("Odak seansı değişti. Seansı yenileyin.".into());
    }
    let Some(item) = open_item(&tx, id) else {
        return Ok(durum(&tx, scope, items));
    };
    if expected_item_id.is_some_and(|expected| expected != item.item_id) {
        return Err("Odak seansındaki iş değişti.".into());
    }
    tx.execute(
        "UPDATE focus_session_items SET ended_at = ?1, outcome = ?2
         WHERE id = ?3 AND outcome IS NULL",
        params![now_str(), outcome, item.item_id],
    )
    .map_err(|e| format!("İş sonuçlandırılamadı: {e}"))?;

    // Kuyruk tükendiyse seans erken biter — Faz K'nin "gün bitebilmeli" ilkesiyle aynı çizgi.
    if !lock_next(&tx, id, scope, items)? {
        end_session(&tx, id, "queue_empty")?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(durum(conn, scope, items))
}

fn end_session(conn: &Connection, id: i64, reason: &str) -> Result<(), String> {
    // Açık kalan madde varsa "abandoned": ölçüme girmemeli.
    conn.execute(
        "UPDATE focus_session_items SET ended_at = ?1, outcome = 'abandoned'
         WHERE session_id = ?2 AND outcome IS NULL",
        params![now_str(), id],
    ).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE focus_sessions SET ended_at = ?1, ended_reason = ?2 WHERE id = ?3",
        params![now_str(), reason, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// Seansı bitirir ve özetini döndürür.
#[tauri::command]
pub fn end_focus_session(
    state: State<'_, AppState>,
    reason: String,
    queue_scope: Option<String>,
    expected_session_id: Option<i64>,
) -> Result<Option<FocusSummary>, String> {
    let conn = state.conn.lock().unwrap();
    stop_session(&conn, session_scope(queue_scope.as_deref())?, &reason, expected_session_id)
}

fn stop_session(
    conn: &Connection,
    scope: &str,
    reason: &str,
    expected_session_id: Option<i64>,
) -> Result<Option<FocusSummary>, String> {
    use rusqlite::OptionalExtension;

    if !scope.is_empty() && expected_session_id.is_none() {
        return Err("Odak seansını yenileyin.".into());
    }
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let Some(id) = expected_session_id.or_else(|| open_session(&tx, scope)) else { return Ok(None) };
    let ended: Option<Option<String>> = tx.query_row(
        "SELECT ended_at FROM focus_sessions WHERE id = ?1 AND scope = ?2",
        params![id, scope],
        |row| row.get(0),
    ).optional().map_err(|e| e.to_string())?;
    let Some(ended) = ended else { return Ok(None) };
    if ended.is_none() {
        end_session(&tx, id, reason)?;
    }
    let result = summary(&tx, id);
    tx.commit().map_err(|e| e.to_string())?;
    Ok(Some(result))
}

fn summary(conn: &Connection, id: i64) -> FocusSummary {
    let (started, ended, reason): (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT started_at, ended_at, ended_reason FROM focus_sessions WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap_or_else(|_| (now_str(), None, None));

    let dakika = ended
        .as_deref()
        .and_then(|e| dakika_farki(&started, e))
        .unwrap_or(0.0);

    let mut buckets = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT bucket, COUNT(*) FROM focus_session_items
         WHERE session_id = ?1 AND outcome = 'done' GROUP BY bucket ORDER BY 2 DESC",
    ) {
        if let Ok(rows) = stmt.query_map([id], |r| Ok((r.get(0)?, r.get(1)?))) {
            buckets = rows.filter_map(Result::ok).collect();
        }
    }

    FocusSummary {
        done_count: sayac(conn, id, "done"),
        skipped_count: sayac(conn, id, "skipped"),
        minutes: (dakika * 10.0).round() / 10.0,
        ended_reason: reason.unwrap_or_default(),
        buckets,
    }
}

/// İki zaman damgası arasındaki dakika farkı.
fn dakika_farki(a: &str, b: &str) -> Option<f64> {
    let f = |s: &str| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok();
    let (x, y) = (f(a)?, f(b)?);
    Some((y - x).num_seconds() as f64 / 60.0)
}

#[tauri::command]
pub fn get_focus_state(
    state: State<'_, AppState>,
    queue_scope: Option<String>,
    queue_items: Option<Vec<queue::QueueItem>>,
) -> Result<FocusState, String> {
    let conn = state.conn.lock().unwrap();
    let scope = session_scope(queue_scope.as_deref())?;
    if open_session(&conn, scope).is_none() {
        return Ok(durum(&conn, scope, &[]));
    }
    let items = session_queue(&conn, scope, queue_items)?;
    Ok(durum(&conn, scope, &items))
}

/// Kova başına ölçülmüş süre (dakika) — yeterli örnek yoksa o kova listede yok.
///
/// Bu, fazın asıl çıktısı: `today.rs` süreyi yazarken önce buraya bakıyor.
pub fn calibration(conn: &Connection) -> std::collections::HashMap<String, u32> {
    calibration_for_scope(conn, "")
}

pub(super) fn calibration_for_scope(conn: &Connection, scope: &str) -> std::collections::HashMap<String, u32> {
    let mut out = std::collections::HashMap::new();
    let Ok(mut stmt) = conn.prepare(
        // ⚠️ YALNIZCA 'done': atlanan iş süre bilgisi taşımıyor, ortalamayı aşağı çeker.
        "SELECT i.bucket, i.started_at, i.ended_at FROM focus_session_items i
         JOIN focus_sessions s ON s.id = i.session_id
         WHERE s.scope = ?1 AND i.outcome = 'done' AND i.ended_at IS NOT NULL",
    ) else {
        return out;
    };
    let mut ornekler: std::collections::HashMap<String, Vec<f64>> = std::collections::HashMap::new();
    if let Ok(rows) = stmt.query_map([scope], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
    }) {
        for (bucket, a, b) in rows.filter_map(Result::ok) {
            if let Some(dk) = dakika_farki(&a, &b) {
                ornekler.entry(bucket).or_default().push(dk);
            }
        }
    }
    for (bucket, v) in ornekler {
        if let Some(dk) = queue::calibrated_minutes(&v) {
            out.insert(bucket, dk);
        }
    }
    out
}

/// Ayarlar ekranı için: kova başına kaç ölçüm birikti ve medyanı ne.
#[derive(Serialize)]
pub struct CalibrationRow {
    pub bucket: String,
    pub samples: i64,
    /// Yeterli örnek yoksa `None` — ekran "henüz ölçülmedi" der.
    pub minutes: Option<u32>,
}

#[tauri::command]
pub fn get_focus_calibration(
    state: State<'_, AppState>,
    queue_scope: Option<String>,
) -> Result<Vec<CalibrationRow>, String> {
    let conn = state.conn.lock().unwrap();
    let scope = session_scope(queue_scope.as_deref())?;
    let olculen = calibration_for_scope(&conn, scope);
    let mut out = Vec::new();
    let mut stmt = conn
        .prepare(
            "SELECT i.bucket, COUNT(*) FROM focus_session_items i
             JOIN focus_sessions s ON s.id = i.session_id
             WHERE s.scope = ?1 AND i.outcome = 'done' AND i.ended_at IS NOT NULL
             GROUP BY i.bucket ORDER BY 1",
        )
        .map_err(|e| format!("Kalibrasyon okunamadı: {e}"))?;
    let rows = stmt
        .query_map([scope], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
        .map_err(|e| format!("Kalibrasyon okunamadı: {e}"))?;
    for (bucket, samples) in rows.filter_map(Result::ok) {
        let minutes = olculen.get(&bucket).copied();
        out.push(CalibrationRow { bucket, samples, minutes });
    }
    Ok(out)
}

/// Seans ve mola süresini kaydeder (Ayarlar).
///
/// ⚠️ Değerler makul aralığa sıkıştırılıyor: 5–90 dk çalışma, 1–30 dk mola. Sıfır ya da
/// negatif bir süre sayacı anlamsız kılardı.
#[tauri::command]
pub fn set_focus_durations(
    state: State<'_, AppState>,
    work: i64,
    brk: i64,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    db::set_setting(&conn, "focus_work_minutes", &work.clamp(5, 90).to_string())?;
    db::set_setting(&conn, "focus_break_minutes", &brk.clamp(1, 30).to_string())?;
    Ok(())
}

/// Açılışta yarım kalmış seansı kapatır.
///
/// ⚠️ Uygulama seans açıkken kapatıldıysa o seansın süresi ölçüm değil: kullanıcı işi
/// bırakmış olabilir, bilgisayarı uyutmuş olabilir. Açık madde `abandoned` işaretleniyor ve
/// kalibrasyona hiç girmiyor.
pub fn close_stale_session(conn: &Connection) {
    let Ok(tx) = conn.unchecked_transaction() else { return };
    let now = now_str();
    let closed_items = tx.execute(
        "UPDATE focus_session_items SET ended_at = ?1, outcome = 'abandoned'
         WHERE outcome IS NULL AND session_id IN (SELECT id FROM focus_sessions WHERE ended_at IS NULL)",
        [&now],
    );
    let closed_sessions = tx.execute(
        "UPDATE focus_sessions SET ended_at = ?1, ended_reason = 'stopped' WHERE ended_at IS NULL",
        [&now],
    );
    if closed_items.is_err() || closed_sessions.is_err() {
        return;
    }
    // Hiç maddesi olmayan KAPANMIŞ seanslar: v0.12.0 sonrası bir kusurdan kalan artıklar
    // (kilitlenecek iş yokken de seans açılıyordu). Ölçüm taşımıyorlar, temizleniyor.
    let _ = tx.execute(
        "DELETE FROM focus_sessions WHERE scope = '' AND ended_at IS NOT NULL AND id NOT IN
           (SELECT DISTINCT session_id FROM focus_session_items)",
        [],
    );
    let _ = tx.commit();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shared_item(sku: &str, done: bool) -> queue::QueueItem {
        queue::QueueItem {
            reference: queue::ItemRef::Product(sku.into()),
            bucket: queue::Bucket::Urgent,
            title: format!("{sku} ürünü"),
            reason: "SEO kontrolü".into(),
            clicks: 1.0,
            score: 1.0,
            page: "products".into(),
            focus_id: sku.into(),
            minutes: 2,
            minutes_measured: false,
            also: Vec::new(),
            done,
        }
    }

    #[test]
    fn ortak_odak_yerel_kuyruga_dusmez_ve_degisikligi_kapsamda_tutar() {
        let conn = Connection::open_in_memory().unwrap();
        db::init(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO products(sku, name) VALUES ('LOCAL', 'Yerel ürün');
             INSERT INTO seo_status(sku, meta_status, details_status) VALUES ('LOCAL', 'done', 'done');",
        ).unwrap();

        let standalone = session_queue(&conn, "", None).unwrap();
        assert_eq!(start_session(&conn, "", &standalone).unwrap().locked.unwrap().reference, "LOCAL");
        let empty = session_queue(&conn, "team-a:user-1", None).unwrap();
        assert!(start_session(&conn, "team-a:user-1", &empty).unwrap().session_id.is_none());
        let items = vec![shared_item("DONE", true), shared_item("A", false), shared_item("B", false)];
        let first = start_session(&conn, "team-a:user-1", &items).unwrap();
        let id = first.session_id.unwrap();
        let lock = first.locked.unwrap();
        assert_eq!(lock.reference, "A");
        assert!(durum(&conn, "team-a:user-2", &items).session_id.is_none());
        assert!(durum(&conn, "team-b:user-1", &items).session_id.is_none());
        assert!(stop_session(&conn, "team-a:user-2", "stopped", Some(id)).unwrap().is_none());
        assert_eq!(durum(&conn, "team-a:user-1", &[]).locked.unwrap().context.title, "A ürünü");

        let updated = resolve_item(&conn, "team-a:user-1", &items, "done", Some(id), Some(lock.item_id)).unwrap();
        assert_eq!(updated.done_count, 1);
        let next = updated.locked.unwrap();
        assert_eq!(next.reference, "B");
        assert!(resolve_item(&conn, "team-a:user-1", &items, "done", Some(id), Some(lock.item_id)).is_err());
        assert_eq!(durum(&conn, "team-a:user-1", &items).locked.unwrap().item_id, next.item_id);
        assert!(resolve_item(&conn, "team-a:user-1", &items, "skipped", Some(id), Some(next.item_id)).unwrap().session_id.is_none());
        let summary = stop_session(&conn, "team-a:user-1", "stopped", Some(id)).unwrap().unwrap();
        assert_eq!((summary.done_count, summary.skipped_count), (1, 1));
        assert_eq!(summary.ended_reason, "queue_empty");
        assert_eq!(durum(&conn, "", &standalone).locked.unwrap().reference, "LOCAL");
    }

    #[test]
    fn eski_odak_kayitlari_yerel_kapsamda_kalir() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE focus_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT, started_at TEXT NOT NULL, ended_at TEXT,
                planned_minutes INTEGER NOT NULL, break_minutes INTEGER NOT NULL, ended_reason TEXT
             );
             CREATE TABLE focus_session_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT, session_id INTEGER NOT NULL, kind TEXT NOT NULL,
                ref TEXT NOT NULL, bucket TEXT NOT NULL, started_at TEXT NOT NULL, ended_at TEXT, outcome TEXT
             );
             INSERT INTO focus_sessions(started_at, planned_minutes, break_minutes)
             VALUES ('2026-11-20T10:00:00', 25, 5);
             INSERT INTO focus_session_items(session_id,kind,ref,bucket,started_at)
             VALUES (1,'product','OLD','urgent','2026-11-20T10:01:00');",
        ).unwrap();
        db::init(&conn).unwrap();
        db::init(&conn).unwrap();
        assert_eq!(durum(&conn, "", &[]).locked.unwrap().reference, "OLD");
        assert!(durum(&conn, "team-a:user-1", &[]).session_id.is_none());
    }

    /// Bir seansı taklit eder: `n` iş `dk` dakika sürmüş gibi kaydedilir.
    fn seans(conn: &Connection, bucket: &str, dakikalar: &[i64], outcome: &str) {
        conn.execute(
            "INSERT INTO focus_sessions (started_at, planned_minutes, break_minutes, ended_at)
             VALUES ('2026-08-08T09:00:00', 25, 5, '2026-08-08T09:30:00')",
            [],
        )
        .unwrap();
        let sid = conn.last_insert_rowid();
        for (i, dk) in dakikalar.iter().enumerate() {
            let bas = format!("2026-08-08T10:{:02}:00", i * 2);
            let bit = format!("2026-08-08T10:{:02}:00", i * 2 + *dk as usize);
            conn.execute(
                "INSERT INTO focus_session_items
                   (session_id, kind, ref, bucket, started_at, ended_at, outcome)
                 VALUES (?1,'product',?2,?3,?4,?5,?6)",
                params![sid, format!("SKU{i}"), bucket, bas, bit, outcome],
            )
            .unwrap();
        }
    }

    /// 🔑 **Fazın bitiş şartı:** seans geçmişinden gerçek süre hesaplanıyor ve kuyruk
    /// tahminlerinin yerini alıyor.
    #[test]
    fn olcum_yeterli_orneke_ulasinca_tahmini_devraliyor() {
        let conn = Connection::open_in_memory().unwrap();
        seo_core::db::init(&conn).unwrap();

        // 4 ölçüm: eşiğin altında, tahmin yerinde kalmalı.
        seans(&conn, "upkeep", &[6, 7, 8, 9], "done");
        assert!(
            calibration(&conn).get("upkeep").is_none(),
            "4 örnekle ölçüm devralmamalıydı"
        );

        // 5'e çıkınca devralıyor. Medyan(6,7,7,8,9) = 7.
        seans(&conn, "upkeep", &[7], "done");
        assert_eq!(calibration(&conn).get("upkeep"), Some(&7));
    }

    /// ⚠️ Atlanan ve yarıda kalan işler ölçüme GİRMEMELİ: "ne kadar sürdüğü" bilgisi
    /// taşımıyorlar, listeye girerlerse süreyi olduğundan kısa gösterirler.
    #[test]
    fn atlanan_ve_yarim_kalan_isler_olcume_girmiyor() {
        let conn = Connection::open_in_memory().unwrap();
        seo_core::db::init(&conn).unwrap();

        seans(&conn, "leverage", &[1, 1, 1, 1, 1], "skipped");
        seans(&conn, "leverage", &[1, 1, 1, 1, 1], "abandoned");
        assert!(
            calibration(&conn).get("leverage").is_none(),
            "atlanan/yarım işler ölçüm sayılmış"
        );

        // Aynı kovada gerçekten bitirilmiş 5 iş → ölçüm oluşuyor ve KISA değil.
        seans(&conn, "leverage", &[9, 9, 9, 9, 9], "done");
        assert_eq!(calibration(&conn).get("leverage"), Some(&9));
    }

    /// 🔴 Kilitlenecek iş yokken seans AÇILMAMALI (saha hatası, 2026-08-08).
    ///
    /// Kullanıcı günün 10 işinin 10'unu bitirmişti; "Odak seansı başlat" deyince karşısına
    /// "Seans bitti · 0 iş · 0 dk" modali çıkıyordu — başarısızlık gibi okunuyor. Üstelik her
    /// denemede sıfır saniyelik bir kayıt birikiyordu (veritabanında 8 tane bulundu).
    #[test]
    fn kilitlenecek_is_yokken_seans_kaydi_olusmuyor() {
        let conn = Connection::open_in_memory().unwrap();
        seo_core::db::init(&conn).unwrap();
        // Kuyruk boş (analiz yok) → kilitlenecek iş yok.
        let items = session_queue(&conn, "", None).unwrap();
        assert!(next_item(&conn, 0, &items).is_none());

        // Boş seans kaydı temizliği: elle bir artık bırakıp kalktığını doğrula.
        conn.execute(
            "INSERT INTO focus_sessions (started_at, ended_at, planned_minutes, break_minutes,
                                         ended_reason)
             VALUES ('2026-08-08T13:47:29','2026-08-08T13:47:29',25,5,'queue_empty')",
            [],
        )
        .unwrap();
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM focus_sessions", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        close_stale_session(&conn);
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM focus_sessions", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0,
            "maddesi olmayan seans artığı temizlenmedi"
        );
    }

    /// Uygulama seans açıkken kapatılırsa: seans kapanır, açık madde `abandoned` olur.
    #[test]
    fn yarim_seans_acilista_kapaniyor_ve_olcume_girmiyor() {
        let conn = Connection::open_in_memory().unwrap();
        seo_core::db::init(&conn).unwrap();
        conn.execute(
            "INSERT INTO focus_sessions (started_at, planned_minutes, break_minutes)
             VALUES ('2026-08-08T09:00:00', 25, 5)",
            [],
        )
        .unwrap();
        let sid = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO focus_session_items (session_id, kind, ref, bucket, started_at)
             VALUES (?1,'product','SKU','urgent','2026-08-08T09:01:00')",
            [sid],
        )
        .unwrap();

        assert!(open_session(&conn, "").is_some());
        close_stale_session(&conn);
        assert!(open_session(&conn, "").is_none(), "yarım seans kapanmadı");

        let outcome: String = conn
            .query_row("SELECT outcome FROM focus_session_items WHERE session_id = ?1", [sid], |r| r.get(0))
            .unwrap();
        assert_eq!(outcome, "abandoned");
        assert!(calibration(&conn).is_empty(), "yarım seans ölçüme girmiş");
    }
}
