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
    pub kind: String,
    pub reference: String,
    pub bucket: String,
    pub title: String,
    pub reason: String,
    pub page: String,
    pub focus_id: String,
    /// Bu iş ne zaman kilitlendi — çubuk "bu işte 4 dakikadır" diyebilsin diye.
    pub started_at: String,
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

fn setting_i64(conn: &Connection, key: &str, fallback: i64) -> i64 {
    db::get_setting(conn, key)
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(fallback)
}

/// Açık (bitmemiş) seansın kimliği.
fn open_session(conn: &Connection) -> Option<i64> {
    conn.query_row(
        "SELECT id FROM focus_sessions WHERE ended_at IS NULL ORDER BY id DESC LIMIT 1",
        [],
        |r| r.get(0),
    )
    .ok()
}

/// Açık seansın kilitli maddesi (henüz sonuçlanmamış satır).
fn open_item(conn: &Connection, session_id: i64) -> Option<LockedItem> {
    conn.query_row(
        "SELECT kind, ref, bucket, started_at FROM focus_session_items
         WHERE session_id = ?1 AND outcome IS NULL ORDER BY id DESC LIMIT 1",
        [session_id],
        |r| {
            Ok(LockedItem {
                kind: r.get(0)?,
                reference: r.get(1)?,
                bucket: r.get(2)?,
                title: String::new(),
                reason: String::new(),
                page: String::new(),
                focus_id: String::new(),
                started_at: r.get(3)?,
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
fn next_item(conn: &Connection, session_id: i64) -> Option<LockedItem> {
    let kuyruk = super::build_today_queue(conn).ok()?;
    let mut dokunulan: Vec<(String, String)> = Vec::new();
    if let Ok(mut stmt) =
        conn.prepare("SELECT kind, ref FROM focus_session_items WHERE session_id = ?1")
    {
        if let Ok(rows) = stmt.query_map([session_id], |r| Ok((r.get(0)?, r.get(1)?))) {
            dokunulan = rows.filter_map(Result::ok).collect();
        }
    }

    let it = kuyruk.items.iter().find(|it| {
        if it.done {
            return false;
        }
        let (k, r) = super::ref_key(&it.reference);
        !dokunulan.iter().any(|(dk, dr)| dk == k && dr == &r)
    })?;
    let (kind, reference) = super::ref_key(&it.reference);
    Some(LockedItem {
        kind: kind.to_string(),
        reference,
        bucket: format!("{:?}", it.bucket).to_lowercase(),
        title: it.title.clone(),
        reason: it.reason.clone(),
        page: it.page.clone(),
        focus_id: it.focus_id.clone(),
        started_at: now_str(),
    })
}

/// Kilitli maddenin ekran bilgilerini (başlık, sebep, hedef) kuyruktan tazeler.
///
/// Veritabanı yalnızca kimliği tutuyor; başlık ve gerekçe kuyruğun kendisinden geliyor ki
/// iki yerde saklanıp ayrışmasın.
fn enrich(conn: &Connection, mut it: LockedItem) -> LockedItem {
    if let Ok(kuyruk) = super::build_today_queue(conn) {
        if let Some(q) = kuyruk.items.iter().find(|q| {
            let (k, r) = super::ref_key(&q.reference);
            k == it.kind && r == it.reference
        }) {
            it.title = q.title.clone();
            it.reason = q.reason.clone();
            it.page = q.page.clone();
            it.focus_id = q.focus_id.clone();
        }
    }
    if it.title.is_empty() {
        it.title = it.reference.clone();
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

fn durum(conn: &Connection) -> FocusState {
    let Some(id) = open_session(conn) else {
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
        locked: open_item(conn, id).map(|it| enrich(conn, it)),
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
pub fn start_focus_session(state: State<'_, AppState>) -> Result<FocusState, String> {
    let conn = state.conn.lock().unwrap();
    if open_session(&conn).is_some() {
        return Ok(durum(&conn));
    }
    // ⚠️ ÖNCE bak, sonra kaydet.
    if next_item(&conn, 0).is_none() {
        return Ok(durum(&conn));
    }
    let work = setting_i64(&conn, "focus_work_minutes", DEFAULT_WORK);
    let brk = setting_i64(&conn, "focus_break_minutes", DEFAULT_BREAK);
    conn.execute(
        "INSERT INTO focus_sessions (started_at, planned_minutes, break_minutes)
         VALUES (?1, ?2, ?3)",
        params![now_str(), work, brk],
    )
    .map_err(|e| format!("Seans başlatılamadı: {e}"))?;
    let id = conn.last_insert_rowid();
    lock_next(&conn, id);
    Ok(durum(&conn))
}

/// Kuyrukta seansa kilitlenebilecek bir iş var mı — ekran düğmeyi buna göre açıyor.
#[tauri::command]
pub fn has_lockable_item(state: State<'_, AppState>) -> Result<bool, String> {
    let conn = state.conn.lock().unwrap();
    Ok(next_item(&conn, 0).is_some())
}

/// Sıradaki işi kilitler. Kuyruk tükenmişse hiçbir şey yapmaz (çağıran seansı bitirir).
fn lock_next(conn: &Connection, session_id: i64) -> bool {
    let Some(it) = next_item(conn, session_id) else { return false };
    let _ = conn.execute(
        "INSERT INTO focus_session_items (session_id, kind, ref, bucket, started_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![session_id, it.kind, it.reference, it.bucket, now_str()],
    );
    true
}

/// Kilitli işi sonuçlandırır ve sıradakini kilitler.
///
/// `outcome`:
/// - `done` → Faz K'nin `complete_queue_item`i çağrılır (yeni bir "tamamlandı" kavramı YOK)
/// - `skipped` → **yalnızca bu seans için**; kuyrukta kalır, kalıcı karar yazılmaz
/// - `dismissed` → çağıran zaten `dismiss_queue_item`i çalıştırdı, burada sadece kaydediliyor
#[tauri::command]
pub fn resolve_focus_item(
    state: State<'_, AppState>,
    outcome: String,
) -> Result<FocusState, String> {
    let conn = state.conn.lock().unwrap();
    let Some(id) = open_session(&conn) else {
        return Ok(durum(&conn));
    };
    conn.execute(
        "UPDATE focus_session_items SET ended_at = ?1, outcome = ?2
         WHERE session_id = ?3 AND outcome IS NULL",
        params![now_str(), outcome, id],
    )
    .map_err(|e| format!("İş sonuçlandırılamadı: {e}"))?;

    // Kuyruk tükendiyse seans erken biter — Faz K'nin "gün bitebilmeli" ilkesiyle aynı çizgi.
    if !lock_next(&conn, id) {
        end_session(&conn, id, "queue_empty");
    }
    Ok(durum(&conn))
}

fn end_session(conn: &Connection, id: i64, reason: &str) {
    // Açık kalan madde varsa "abandoned": ölçüme girmemeli.
    let _ = conn.execute(
        "UPDATE focus_session_items SET ended_at = ?1, outcome = 'abandoned'
         WHERE session_id = ?2 AND outcome IS NULL",
        params![now_str(), id],
    );
    let _ = conn.execute(
        "UPDATE focus_sessions SET ended_at = ?1, ended_reason = ?2 WHERE id = ?3",
        params![now_str(), reason, id],
    );
}

/// Seansı bitirir ve özetini döndürür.
#[tauri::command]
pub fn end_focus_session(
    state: State<'_, AppState>,
    reason: String,
) -> Result<Option<FocusSummary>, String> {
    let conn = state.conn.lock().unwrap();
    let Some(id) = open_session(&conn) else { return Ok(None) };
    end_session(&conn, id, &reason);
    Ok(Some(summary(&conn, id)))
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
pub fn get_focus_state(state: State<'_, AppState>) -> Result<FocusState, String> {
    let conn = state.conn.lock().unwrap();
    Ok(durum(&conn))
}

/// Kova başına ölçülmüş süre (dakika) — yeterli örnek yoksa o kova listede yok.
///
/// Bu, fazın asıl çıktısı: `today.rs` süreyi yazarken önce buraya bakıyor.
pub fn calibration(conn: &Connection) -> std::collections::HashMap<String, u32> {
    let mut out = std::collections::HashMap::new();
    let Ok(mut stmt) = conn.prepare(
        // ⚠️ YALNIZCA 'done': atlanan iş süre bilgisi taşımıyor, ortalamayı aşağı çeker.
        "SELECT bucket, started_at, ended_at FROM focus_session_items
         WHERE outcome = 'done' AND ended_at IS NOT NULL",
    ) else {
        return out;
    };
    let mut ornekler: std::collections::HashMap<String, Vec<f64>> = std::collections::HashMap::new();
    if let Ok(rows) = stmt.query_map([], |r| {
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
pub fn get_focus_calibration(state: State<'_, AppState>) -> Result<Vec<CalibrationRow>, String> {
    let conn = state.conn.lock().unwrap();
    let olculen = calibration(&conn);
    let mut out = Vec::new();
    let mut stmt = conn
        .prepare(
            "SELECT bucket, COUNT(*) FROM focus_session_items
             WHERE outcome = 'done' AND ended_at IS NOT NULL GROUP BY bucket ORDER BY 1",
        )
        .map_err(|e| format!("Kalibrasyon okunamadı: {e}"))?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
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
    if let Some(id) = open_session(conn) {
        end_session(conn, id, "stopped");
    }
    // Hiç maddesi olmayan KAPANMIŞ seanslar: v0.12.0 sonrası bir kusurdan kalan artıklar
    // (kilitlenecek iş yokken de seans açılıyordu). Ölçüm taşımıyorlar, temizleniyor.
    let _ = conn.execute(
        "DELETE FROM focus_sessions WHERE ended_at IS NOT NULL AND id NOT IN
           (SELECT DISTINCT session_id FROM focus_session_items)",
        [],
    );
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(next_item(&conn, 0).is_none());

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

        assert!(open_session(&conn).is_some());
        close_stale_session(&conn);
        assert!(open_session(&conn).is_none(), "yarım seans kapanmadı");

        let outcome: String = conn
            .query_row("SELECT outcome FROM focus_session_items WHERE session_id = ?1", [sid], |r| r.get(0))
            .unwrap();
        assert_eq!(outcome, "abandoned");
        assert!(calibration(&conn).is_empty(), "yarım seans ölçüme girmiş");
    }
}
