//! Müşteri kayıtları ve temas geçmişi (Faz C) — CRM'in Tauri katmanı.
//!
//! Saf mantık `seo_core::contacts`te (gecikme cümlesi, sessizlik önerisi); burada yalnızca
//! veritabanı erişimi var.
//!
//! # 🔴 Kişisel veri sınırı
//!
//! Bu modülle birlikte uygulama ilk kez **kişisel veri** tutuyor. Kural: bu tablolar
//! asistanın bağlam kaynaklarına **girmiyor** (`src/assistantSources.ts`). Asistan bağlamı
//! Gemini'ye gönderiliyor; müşteri adı, telefonu ve notu oraya gitmez. Kayıt tablosu opt-in
//! olduğu için bu "eklemeyerek" sağlanıyor — ve bir test bunu sabitliyor ki sonradan
//! "eksik kalmış" diye eklenmesin.
//!
//! # Kuyruk bağı
//!
//! Kişinin `next_step_at` tarihi geldiğinde madde Bugün kuyruğuna düşer ([`due_contacts`]).
//! "Yapıldı" denince [`complete_contact_followup`] çalışır: temas yazılır, `last_contact_at`
//! güncellenir ve **sonraki adım temizlenir** — böylece madde kuyruktan kendiliğinden düşer,
//! ayrıca gizlemeye gerek kalmaz.

use super::*;
use seo_core::contacts::DueContact;

#[derive(Serialize, Clone)]
pub struct Contact {
    pub id: i64,
    pub name: String,
    pub company: String,
    pub email: String,
    pub phone: String,
    pub channel: String,
    pub note: String,
    pub last_contact_at: Option<String>,
    pub next_step_at: Option<String>,
    pub next_step_note: String,
    pub archived: bool,
    /// Kaç temas kaydı var — listede "3 temas" diye görünüyor.
    pub event_count: i64,
}

#[derive(Serialize, Clone)]
pub struct ContactEvent {
    pub id: i64,
    pub at: String,
    /// `call` | `email` | `meeting` | `note` | `followup_done`
    pub kind: String,
    pub note: String,
}

fn row_to_contact(r: &rusqlite::Row) -> rusqlite::Result<Contact> {
    Ok(Contact {
        id: r.get(0)?,
        name: r.get(1)?,
        company: r.get(2)?,
        email: r.get(3)?,
        phone: r.get(4)?,
        channel: r.get(5)?,
        note: r.get(6)?,
        last_contact_at: r.get(7)?,
        next_step_at: r.get(8)?,
        next_step_note: r.get(9)?,
        archived: r.get::<_, i64>(10)? != 0,
        event_count: r.get(11)?,
    })
}

const SELECT_CONTACT: &str = "SELECT c.id, c.name, c.company, c.email, c.phone, c.channel,
        c.note, c.last_contact_at, c.next_step_at, c.next_step_note, c.archived,
        (SELECT COUNT(*) FROM contact_events e WHERE e.contact_id = c.id)
     FROM contacts c";

/// Kişi listesi. `search` boşsa hepsi; arşivlenenler yalnızca `include_archived` ile.
///
/// ⚠️ Sıralama **sonraki adım tarihine göre**: en yakın iş üstte. Tarihi olmayan kişiler
/// (`NULL`) sona düşüyor — liste "bugün kime dönmeliyim" sorusuna göre diziliyor, alfabeye
/// göre değil.
#[tauri::command]
pub fn list_contacts(
    state: State<'_, AppState>,
    search: String,
    include_archived: bool,
) -> Result<Vec<Contact>, String> {
    let conn = state.conn.lock().unwrap();
    let like = format!("%{}%", search.trim().to_lowercase());
    let sql = format!(
        "{SELECT_CONTACT}
         WHERE (?1 = 0 OR c.archived = 0)
           AND (?2 = '%%' OR lower(c.name) LIKE ?2 OR lower(c.company) LIKE ?2
                OR lower(c.email) LIKE ?2 OR c.phone LIKE ?2)
         ORDER BY c.next_step_at IS NULL, c.next_step_at, c.name"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| format!("Kişiler okunamadı: {e}"))?;
    let rows = stmt
        .query_map(params![include_archived as i64, like], |r| row_to_contact(r))
        .map_err(|e| format!("Kişiler okunamadı: {e}"))?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

#[tauri::command]
pub fn get_contact(state: State<'_, AppState>, id: i64) -> Result<Contact, String> {
    let conn = state.conn.lock().unwrap();
    conn.query_row(&format!("{SELECT_CONTACT} WHERE c.id = ?1"), params![id], |r| {
        row_to_contact(r)
    })
    .map_err(|e| format!("Kişi bulunamadı: {e}"))
}

/// Kişi ekler ya da günceller (`id` varsa günceller). Yeni kişinin kimliğini döner.
///
/// ⚠️ `last_contact_at` buradan yazılmıyor: o **temas eklendiğinde** güncelleniyor
/// ([`add_contact_event`]). Kartı düzenlemek "temas ettim" demek değil.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn save_contact(
    state: State<'_, AppState>,
    id: Option<i64>,
    name: String,
    company: String,
    email: String,
    phone: String,
    channel: String,
    note: String,
    next_step_at: Option<String>,
    next_step_note: String,
) -> Result<i64, String> {
    if name.trim().is_empty() {
        return Err("Kişi adı boş olamaz.".into());
    }
    let conn = state.conn.lock().unwrap();
    let now = now_str();
    match id {
        Some(id) => {
            conn.execute(
                "UPDATE contacts SET name=?2, company=?3, email=?4, phone=?5, channel=?6,
                    note=?7, next_step_at=?8, next_step_note=?9, updated_at=?10 WHERE id=?1",
                params![
                    id,
                    name.trim(),
                    company.trim(),
                    email.trim(),
                    phone.trim(),
                    channel,
                    note,
                    next_step_at,
                    next_step_note,
                    now
                ],
            )
            .map_err(|e| format!("Kişi kaydedilemedi: {e}"))?;
            Ok(id)
        }
        None => {
            conn.execute(
                "INSERT INTO contacts (name, company, email, phone, channel, note,
                    next_step_at, next_step_note, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9)",
                params![
                    name.trim(),
                    company.trim(),
                    email.trim(),
                    phone.trim(),
                    channel,
                    note,
                    next_step_at,
                    next_step_note,
                    now
                ],
            )
            .map_err(|e| format!("Kişi kaydedilemedi: {e}"))?;
            Ok(conn.last_insert_rowid())
        }
    }
}

/// Arşivler ya da arşivden çıkarır.
///
/// ⚠️ Silmek yerine arşiv: geçmiş temaslar bir kayıt, kişi listeden çıksa da durmalı.
/// `eol_decisions`'taki `keep` kararıyla aynı fikir — kullanıcının kararı veriyi silmez.
#[tauri::command]
pub fn archive_contact(state: State<'_, AppState>, id: i64, archived: bool) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute(
        "UPDATE contacts SET archived = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, archived as i64, now_str()],
    )
    .map_err(|e| format!("Arşivlenemedi: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn get_contact_events(
    state: State<'_, AppState>,
    contact_id: i64,
) -> Result<Vec<ContactEvent>, String> {
    let conn = state.conn.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT id, at, kind, note FROM contact_events
             WHERE contact_id = ?1 ORDER BY at DESC, id DESC",
        )
        .map_err(|e| format!("Temaslar okunamadı: {e}"))?;
    let rows = stmt
        .query_map(params![contact_id], |r| {
            Ok(ContactEvent { id: r.get(0)?, at: r.get(1)?, kind: r.get(2)?, note: r.get(3)? })
        })
        .map_err(|e| format!("Temaslar okunamadı: {e}"))?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// Temas kaydeder ve kişinin `last_contact_at` alanını günceller.
///
/// `next_step_at`/`next_step_note` verilirse sonraki adım da aynı işlemde yazılıyor:
/// "aradım, iki hafta sonra tekrar" tek adımda olmalı, yoksa kullanıcı ikinci adımı unutur
/// ve kişi sessizce düşer.
#[tauri::command]
pub fn add_contact_event(
    state: State<'_, AppState>,
    contact_id: i64,
    kind: String,
    note: String,
    next_step_at: Option<String>,
    next_step_note: Option<String>,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    write_event(&conn, contact_id, &kind, &note, next_step_at, next_step_note)
}

/// Ortak yazma yolu — komut da, kuyruğun "yapıldı" dalı da buradan geçiyor.
///
/// ⚠️ Tek yerde: iki ayrı yazma olsaydı biri `last_contact_at`i güncellemeyi unutabilir,
/// kişi "hiç temas edilmemiş" görünmeye devam ederdi.
pub(crate) fn write_event(
    conn: &Connection,
    contact_id: i64,
    kind: &str,
    note: &str,
    next_step_at: Option<String>,
    next_step_note: Option<String>,
) -> Result<(), String> {
    let now = now_str();
    conn.execute(
        "INSERT INTO contact_events (contact_id, at, kind, note) VALUES (?1,?2,?3,?4)",
        params![contact_id, now, kind, note],
    )
    .map_err(|e| format!("Temas kaydedilemedi: {e}"))?;
    conn.execute(
        "UPDATE contacts SET last_contact_at = ?2, next_step_at = ?3,
             next_step_note = ?4, updated_at = ?2 WHERE id = ?1",
        params![contact_id, now, next_step_at, next_step_note.unwrap_or_default()],
    )
    .map_err(|e| format!("Kişi güncellenemedi: {e}"))?;
    Ok(())
}

/// Kuyruğun "yapıldı" dalı: dönüş yapıldı, sonraki adım **temizlendi**.
///
/// 🔑 Madde kuyruktan `queue_dismissals` ile değil, **verinin kendisi değiştiği için**
/// düşüyor: sonraki adım tarihi yoksa kişi aday bile olmuyor. Faz D'nin karar deposuyla aynı
/// fikir — gizleme bir yama, veriyi düzeltmek çözüm.
pub(crate) fn complete_contact_followup(conn: &Connection, contact_id: i64) -> Result<(), String> {
    write_event(conn, contact_id, "followup_done", "", None, None)
}

/// Kuyruk maddesi için gereken alanlar. `?1` = bugünün tarihi.
///
/// ⚠️ Sonraki adımı olmayan kişide gecikme **0**: [`contacts_by_ids`] "bugün yapıldı"
/// maddelerini geri okurken tarih zaten temizlenmiş oluyor.
const DUE_SELECT: &str = "SELECT id, name, company, next_step_note,
        COALESCE(julianday(?1) - julianday(date(next_step_at)), 0)
     FROM contacts";

fn map_due(r: &rusqlite::Row) -> rusqlite::Result<DueContact> {
    Ok(DueContact {
        id: r.get(0)?,
        name: r.get(1)?,
        company: r.get(2)?,
        note: r.get(3)?,
        overdue_days: r.get::<_, f64>(4)?.round() as i64,
    })
}

fn oku(conn: &Connection, sql: &str, p: &[&dyn rusqlite::ToSql]) -> Vec<DueContact> {
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map(p, map_due)
        .map(|rows| rows.filter_map(Result::ok).collect())
        .unwrap_or_default()
}

/// Sonraki adımı bugüne gelmiş kişiler — Bugün kuyruğunun müşteri kovası.
///
/// ⚠️ Arşivlenenler dışarıda. `date(next_step_at) <= date(bugün)`: saat bilgisi taşımıyoruz,
/// "bugün dönülecek" bütün günü kapsar.
pub(crate) fn due_contacts(conn: &Connection) -> Vec<DueContact> {
    let bugun = now_str()[..10].to_string();
    oku(
        conn,
        &format!(
            "{DUE_SELECT} WHERE archived = 0 AND next_step_at IS NOT NULL
             AND date(next_step_at) <= ?1"
        ),
        &[&bugun],
    )
}

/// Kimliğe göre kişiler — sonraki adımı **olmasa da** döner.
///
/// 🔴 Neden var: kuyrukta "yapıldı" denen müşteri maddesi sonraki adımı temizlediği için
/// adaylıktan tamamen düşerdi; yerine 11. madde gelir ve 2026-08-08'de düzeltilen *"gün hiç
/// bitmiyor"* hatası CRM tarafında yeniden doğardı. Bu fonksiyon o maddeleri günün listesine
/// geri koyuyor — üstü çizili, ilerlemeye sayılarak.
pub(crate) fn contacts_by_ids(conn: &Connection, ids: &[i64]) -> Vec<DueContact> {
    if ids.is_empty() {
        return Vec::new();
    }
    let bugun = now_str()[..10].to_string();
    let liste: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
    // Kimlikler veritabanından gelen i64'ler; metne çevrilmeleri güvenli.
    oku(conn, &format!("{DUE_SELECT} WHERE id IN ({})", liste.join(",")), &[&bugun])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        seo_core::db::init(&conn).unwrap();
        conn
    }

    fn ekle(conn: &Connection, ad: &str, next: Option<&str>) -> i64 {
        conn.execute(
            "INSERT INTO contacts (name, company, next_step_at, next_step_note,
                created_at, updated_at) VALUES (?1,'Kurumsal BT',?2,'fiyat verilecek',?3,?3)",
            params![ad, next, now_str()],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn gun(n: i64) -> String {
        (chrono::Local::now().date_naive() + chrono::Duration::days(n)).to_string()
    }

    /// ⚠️ "3 hafta sonra ara" bugünün listesini kirletmemeli; geciken kişi ise gecikmesiyle
    /// birlikte gelmeli.
    #[test]
    fn yalnizca_zamani_gelmis_kisiler_kuyruga_giriyor() {
        let conn = db();
        ekle(&conn, "Bugün", Some(&gun(0)));
        ekle(&conn, "Geciken", Some(&gun(-4)));
        ekle(&conn, "Gelecek", Some(&gun(3)));
        ekle(&conn, "Tarihsiz", None);

        let mut d = due_contacts(&conn);
        d.sort_by_key(|c| c.name.clone());
        let adlar: Vec<&str> = d.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(adlar, vec!["Bugün", "Geciken"]);
        let geciken = d.iter().find(|c| c.name == "Geciken").unwrap();
        assert_eq!(geciken.overdue_days, 4);
        assert_eq!(geciken.reason(), "4 gündür bekliyor — fiyat verilecek");
    }

    #[test]
    fn arsivlenen_kisi_kuyruktan_cikiyor() {
        let conn = db();
        let id = ekle(&conn, "Arşivlik", Some(&gun(-1)));
        assert_eq!(due_contacts(&conn).len(), 1);
        conn.execute("UPDATE contacts SET archived = 1 WHERE id = ?1", params![id]).unwrap();
        assert!(due_contacts(&conn).is_empty());
    }

    /// 🔑 "Yapıldı" maddeyi gizleyerek değil, **veriyi düzelterek** düşürüyor.
    #[test]
    fn yapildi_temas_yaziyor_ve_sonraki_adimi_temizliyor() {
        let conn = db();
        let id = ekle(&conn, "Ahmet", Some(&gun(-2)));
        complete_contact_followup(&conn, id).unwrap();

        assert!(due_contacts(&conn).is_empty(), "sonraki adım kalkınca kuyruktan düştü");
        let (kind, son): (String, Option<String>) = conn
            .query_row(
                "SELECT e.kind, c.last_contact_at FROM contact_events e
                 JOIN contacts c ON c.id = e.contact_id WHERE e.contact_id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(kind, "followup_done");
        assert!(son.is_some(), "son temas tarihi güncellenmeliydi");
    }

    /// Temas + yeni randevu tek adımda: ikiye bölünse kullanıcı ikincisini unutur.
    #[test]
    fn temas_ve_yeni_randevu_ayni_islemde_yaziliyor() {
        let conn = db();
        let id = ekle(&conn, "Ayşe", None);
        write_event(&conn, id, "call", "fiyat sordu", Some(gun(5)), Some("teklif gönder".into()))
            .unwrap();

        let (next, not): (Option<String>, String) = conn
            .query_row("SELECT next_step_at, next_step_note FROM contacts WHERE id=?1", [id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(next, Some(gun(5)));
        assert_eq!(not, "teklif gönder");
        assert!(due_contacts(&conn).is_empty(), "5 gün sonrası bugünün işi değil");
    }
}
