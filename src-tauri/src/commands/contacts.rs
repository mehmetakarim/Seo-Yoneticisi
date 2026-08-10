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
//! olduğu için bu "eklemeyerek" sağlanıyor. ⚠️ Projede JS test koşucusu olmadığı için garanti
//! bir test değil, **dev kipinde açılışta patlayan bir kilit**: `YASAKLI_KAYNAKLAR`.
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
    /// İlgi etiketleri. ⚠️ Listede satır başına ayrı sorgu YOK: `group_concat` ile tek
    /// sorguda geliyor (300 kişide 300 sorgu, açılışta hissedilir bir gecikme olurdu).
    pub tags: Vec<String>,
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
        tags: r
            .get::<_, Option<String>>(12)?
            .unwrap_or_default()
            .split(',')
            .filter(|t| !t.is_empty())
            .map(|t| t.to_string())
            .collect(),
    })
}

const SELECT_CONTACT: &str = "SELECT c.id, c.name, c.company, c.email, c.phone, c.channel,
        c.note, c.last_contact_at, c.next_step_at, c.next_step_note, c.archived,
        (SELECT COUNT(*) FROM contact_events e WHERE e.contact_id = c.id),
        (SELECT group_concat(t.tag) FROM contact_tags t WHERE t.contact_id = c.id)
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
    channel: String,
    tag: String,
) -> Result<Vec<Contact>, String> {
    let conn = state.conn.lock().unwrap();
    let like = format!("%{}%", search.trim().to_lowercase());
    let sql = format!(
        "{SELECT_CONTACT}
         WHERE (?1 = 0 OR c.archived = 0)
           AND (?2 = '%%' OR lower(c.name) LIKE ?2 OR lower(c.company) LIKE ?2
                OR lower(c.email) LIKE ?2 OR c.phone LIKE ?2)
           AND (?3 = '' OR c.channel = ?3)
           AND (?4 = '' OR EXISTS
                (SELECT 1 FROM contact_tags t WHERE t.contact_id = c.id AND t.tag = ?4))
         ORDER BY c.next_step_at IS NULL, c.next_step_at, c.name"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| format!("Kişiler okunamadı: {e}"))?;
    let rows = stmt
        .query_map(params![include_archived as i64, like, channel, tag], |r| row_to_contact(r))
        .map_err(|e| format!("Kişiler okunamadı: {e}"))?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// Kullanılan bütün etiketler — süzgeç listesi ve giriş kutusundaki öneriler.
///
/// ⚠️ Sabit etiket listesi YOK: uygulama kişiselleştirilmemiş, bir mağazanın ilgi etiketleri
/// ("sunucu", "3D yazıcı") diğerininkine benzemez. Öneriler kullanıcının kendi verisinden.
#[tauri::command]
pub fn list_contact_tags(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let conn = state.conn.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT tag, COUNT(*) n FROM contact_tags GROUP BY tag ORDER BY n DESC, tag")
        .map_err(|e| format!("Etiketler okunamadı: {e}"))?;
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| format!("Etiketler okunamadı: {e}"))?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// Kişinin etiketlerini **tamamen değiştirir** (ekle/çıkar ayrı komut değil).
///
/// Tek komut olmasının sebebi: ekran zaten tam listeyi tutuyor; "ekle" ve "çıkar" ayrı
/// olsaydı ekranla veritabanı arasında kısmi başarısızlıkta ayrışma olurdu.
#[tauri::command]
pub fn set_contact_tags(
    state: State<'_, AppState>,
    contact_id: i64,
    tags: Vec<String>,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute("DELETE FROM contact_tags WHERE contact_id = ?1", params![contact_id])
        .map_err(|e| format!("Etiketler yazılamadı: {e}"))?;
    for t in tags {
        let t = t.trim();
        if t.is_empty() {
            continue;
        }
        // Aynı etiket iki kez gelirse PK çakışır; sorun değil, yok sayılıyor.
        let _ = conn.execute(
            "INSERT OR IGNORE INTO contact_tags (contact_id, tag) VALUES (?1,?2)",
            params![contact_id, t],
        );
    }
    Ok(())
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
        COALESCE(julianday(?1) - julianday(date(next_step_at)), 0),
        COALESCE(julianday(?1) - julianday(date(last_contact_at)), 0)
     FROM contacts";

fn map_due(r: &rusqlite::Row) -> rusqlite::Result<DueContact> {
    Ok(DueContact {
        id: r.get(0)?,
        name: r.get(1)?,
        company: r.get(2)?,
        note: r.get(3)?,
        overdue_days: r.get::<_, f64>(4)?.round() as i64,
        // Sessizlik dalı bunu 6. sütundan dolduruyor; sonraki-adım dalında 0 kalıyor
        // (`map_sessiz` ile ayrılıyor, çünkü aynı satır iki farklı cümle kurabiliyor).
        silent_days: 0,
    })
}

/// Sessizlik dalının satır eşlemesi — son temastan bu yana geçen günü taşıyor.
fn map_sessiz(r: &rusqlite::Row) -> rusqlite::Result<DueContact> {
    let mut k = map_due(r)?;
    k.silent_days = r.get::<_, f64>(5)?.round() as i64;
    k
        .note
        .clear(); // sessizlikte "sonraki adım notu" yok; boş bırakılmazsa yanlış bağlam verir
    Ok(k)
}

fn oku(
    conn: &Connection,
    sql: &str,
    p: &[&dyn rusqlite::ToSql],
    map: fn(&rusqlite::Row) -> rusqlite::Result<DueContact>,
) -> Vec<DueContact> {
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map(p, map)
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
        map_due,
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
    oku(conn, &format!("{DUE_SELECT} WHERE id IN ({})", liste.join(",")), &[&bugun], map_due)
}

// ---------------------------------------------------------------------------
// "Bu ürünle ilgilendi" — SEO tarafıyla CRM'i birleştiren tek yer
// ---------------------------------------------------------------------------

/// Kişinin ilgilendiği ürün (kartta) ya da ürünle ilgilenen kişi (ürün detayında).
#[derive(Serialize, Clone)]
pub struct ContactProduct {
    pub sku: String,
    pub name: String,
    /// Ürün detayında kullanılıyor; kişi kartında boş.
    pub contact_id: i64,
    pub at: String,
}

/// Kişinin ilgilendiği ürünler.
#[tauri::command]
pub fn get_contact_products(
    state: State<'_, AppState>,
    contact_id: i64,
) -> Result<Vec<ContactProduct>, String> {
    let conn = state.conn.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT cp.sku, COALESCE(p.name, cp.sku), cp.contact_id, cp.at
             FROM contact_products cp LEFT JOIN products p ON p.sku = cp.sku
             WHERE cp.contact_id = ?1 ORDER BY cp.at DESC",
        )
        .map_err(|e| format!("Ürün bağları okunamadı: {e}"))?;
    let rows = stmt
        .query_map(params![contact_id], |r| {
            Ok(ContactProduct { sku: r.get(0)?, name: r.get(1)?, contact_id: r.get(2)?, at: r.get(3)? })
        })
        .map_err(|e| format!("Ürün bağları okunamadı: {e}"))?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// Bu ürünle ilgilenen kişiler — ürün detayında gösteriliyor.
///
/// ⚠️ **Kopya kayıt yok:** iki yön de `contact_products`ı sorguluyor. Ürün tarafına ayrı bir
/// tablo/sayaç konsaydı biri güncellenip diğeri unutulabilirdi.
#[tauri::command]
pub fn contacts_of_product(
    state: State<'_, AppState>,
    sku: String,
) -> Result<Vec<ContactProduct>, String> {
    let conn = state.conn.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT cp.sku, c.name || CASE WHEN c.company = '' THEN '' ELSE ' · ' || c.company END,
                    cp.contact_id, cp.at
             FROM contact_products cp JOIN contacts c ON c.id = cp.contact_id
             WHERE cp.sku = ?1 AND c.archived = 0 ORDER BY cp.at DESC",
        )
        .map_err(|e| format!("İlgilenenler okunamadı: {e}"))?;
    let rows = stmt
        .query_map(params![sku], |r| {
            Ok(ContactProduct { sku: r.get(0)?, name: r.get(1)?, contact_id: r.get(2)?, at: r.get(3)? })
        })
        .map_err(|e| format!("İlgilenenler okunamadı: {e}"))?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

#[tauri::command]
pub fn link_contact_product(
    state: State<'_, AppState>,
    contact_id: i64,
    sku: String,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO contact_products (contact_id, sku, at) VALUES (?1,?2,?3)",
        params![contact_id, sku, now_str()],
    )
    .map_err(|e| format!("Ürün bağlanamadı: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn unlink_contact_product(
    state: State<'_, AppState>,
    contact_id: i64,
    sku: String,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute(
        "DELETE FROM contact_products WHERE contact_id = ?1 AND sku = ?2",
        params![contact_id, sku],
    )
    .map_err(|e| format!("Bağ kaldırılamadı: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// CSV içe aktarma
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct CsvPreview {
    pub headers: Vec<String>,
    /// Yalnızca ilk [`csv_import::PREVIEW_ROWS`] satır.
    pub rows: Vec<Vec<String>>,
    pub total_rows: usize,
    pub delimiter: String,
    /// Uygulamanın tahmini — `FIELDS` sırasıyla sütun indeksleri.
    pub mapping: Vec<Option<usize>>,
    /// Alan anahtarı + okunur adı; ekran eşleştirme satırlarını buradan çiziyor.
    pub fields: Vec<(String, String)>,
}

#[derive(Serialize)]
pub struct ImportSummary {
    pub added: usize,
    pub updated: usize,
    pub skipped: usize,
    /// Atlama sebepleri, ekranda dürüstçe söyleniyor ("2 satırda ad boştu").
    pub skip_reason: String,
}

fn read_utf8(path: &str) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Dosya okunamadı: {e}"))?;
    // ⚠️ Türkçe Excel sıklıkla Windows-1254 yazıyor. Sessizce bozuk karakter üretmek yerine
    // ne yapılacağını söylüyoruz — kullanıcı "CSV UTF-8" ile yeniden kaydedip dönebilir.
    String::from_utf8(bytes).map_err(|_| {
        "Dosya UTF-8 değil. Excel'de \"Farklı Kaydet → CSV UTF-8\" seçip tekrar deneyin.".to_string()
    })
}

/// Dosyayı okur, başlıkları ve ilk satırları döner. **Hiçbir şey yazmaz.**
#[tauri::command]
pub fn preview_contact_csv(path: String) -> Result<CsvPreview, String> {
    let text = read_utf8(&path)?;
    let p = csv_import::parse(&text)?;
    let mapping = csv_import::guess_mapping(&p.headers);
    Ok(CsvPreview {
        rows: p.rows.iter().take(csv_import::PREVIEW_ROWS).cloned().collect(),
        total_rows: p.rows.len(),
        headers: p.headers,
        delimiter: p.delimiter,
        mapping,
        fields: csv_import::FIELDS.iter().map(|(k, l)| (k.to_string(), l.to_string())).collect(),
    })
}

/// Eşleştirmeye göre kişileri yazar.
///
/// 🔑 **Tekilleştirme e-posta → telefon sırasıyla.** Eşleşen kayıt GÜNCELLENİYOR: aynı listeyi
/// ikinci kez aktarmak kişileri ikizlemesin. Boş e-posta/telefon eşleşme sayılmıyor —
/// yoksa alanı boş olan bütün satırlar tek kişiye çökerdi.
#[tauri::command]
pub fn import_contacts_csv(
    state: State<'_, AppState>,
    path: String,
    mapping: Vec<Option<usize>>,
) -> Result<ImportSummary, String> {
    let text = read_utf8(&path)?;
    let p = csv_import::parse(&text)?;
    let conn = state.conn.lock().unwrap();
    let now = now_str();
    let (mut added, mut updated, mut adsiz) = (0usize, 0usize, 0usize);

    for row in &p.rows {
        let al = |f: &str| csv_import::field_of(row, &mapping, f);
        let name = al("name");
        if name.is_empty() {
            // Adsız kişi listede kimliksiz bir satır olurdu; sessizce atlanmıyor, sayılıyor.
            adsiz += 1;
            continue;
        }
        let (email, phone) = (al("email"), al("phone"));
        let mevcut: Option<i64> = conn
            .query_row(
                "SELECT id FROM contacts
                 WHERE (?1 <> '' AND lower(email) = lower(?1)) OR (?2 <> '' AND phone = ?2)
                 LIMIT 1",
                params![email, phone],
                |r| r.get(0),
            )
            .ok();

        match mevcut {
            Some(id) => {
                // ⚠️ Yalnızca DOLU alanlar yazılıyor: CSV'de boş bırakılan bir sütun,
                // uygulamada elle girilmiş bilgiyi silmemeli.
                conn.execute(
                    "UPDATE contacts SET
                       name = ?2,
                       company = CASE WHEN ?3 <> '' THEN ?3 ELSE company END,
                       email   = CASE WHEN ?4 <> '' THEN ?4 ELSE email END,
                       phone   = CASE WHEN ?5 <> '' THEN ?5 ELSE phone END,
                       channel = CASE WHEN ?6 <> '' THEN ?6 ELSE channel END,
                       note    = CASE WHEN ?7 <> '' THEN ?7 ELSE note END,
                       updated_at = ?8
                     WHERE id = ?1",
                    params![id, name, al("company"), email, phone, al("channel"), al("note"), now],
                )
                .map_err(|e| format!("Kişi güncellenemedi: {e}"))?;
                updated += 1;
            }
            None => {
                conn.execute(
                    "INSERT INTO contacts (name, company, email, phone, channel, note,
                        created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?7)",
                    params![name, al("company"), email, phone, al("channel"), al("note"), now],
                )
                .map_err(|e| format!("Kişi eklenemedi: {e}"))?;
                added += 1;
            }
        }
    }

    Ok(ImportSummary {
        added,
        updated,
        skipped: adsiz,
        skip_reason: if adsiz > 0 { format!("{adsiz} satırda ad boştu") } else { String::new() },
    })
}

// ---------------------------------------------------------------------------
// Sessizlik eşiği — kapalı doğar, veriden öğrenir
// ---------------------------------------------------------------------------

const SILENCE_KEY: &str = "contact_silence_days";

#[derive(Serialize)]
pub struct SilenceState {
    /// 0 = kapalı.
    pub days: i64,
    /// Veriden çıkan öneri; yetersiz veride `None` (sayı UYDURULMUYOR).
    pub suggestion: Option<u32>,
    /// Öneriye kaç kişinin temas aralığı katıldı — kullanıcı neye dayandığını görsün.
    pub sample_contacts: usize,
}

/// Kişi başına ortalama temas aralığı (gün) — öneri bundan çıkıyor.
///
/// ⚠️ Yalnızca **≥2 teması olan** kişiler: tek temaslı kişide "aralık" diye bir şey yok.
fn contact_intervals(conn: &Connection) -> Vec<f64> {
    let mut stmt = match conn.prepare(
        "SELECT (julianday(MAX(at)) - julianday(MIN(at))) / (COUNT(*) - 1)
         FROM contact_events GROUP BY contact_id HAVING COUNT(*) > 1",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map([], |r| r.get::<_, f64>(0))
        .map(|rows| rows.filter_map(Result::ok).collect())
        .unwrap_or_default()
}

#[tauri::command]
pub fn get_silence_state(state: State<'_, AppState>) -> Result<SilenceState, String> {
    let conn = state.conn.lock().unwrap();
    let araliklar = contact_intervals(&conn);
    Ok(SilenceState {
        days: super::setting_i64(&conn, SILENCE_KEY, 0),
        suggestion: seo_core::contacts::sessizlik_onerisi(&araliklar),
        sample_contacts: araliklar.len(),
    })
}

/// Eşiği yazar. `0` = kapalı.
///
/// ⚠️ Öneri **kendiliğinden yazılmıyor**; bu komut yalnızca kullanıcı onayıyla çağrılıyor.
/// Faz D'de stok eşiği "veri yokken eşik uydurma" gerekçesiyle elenmişti; buradaki fark,
/// eşiğin kullanıcının kendi verisinden çıkması ve yine de onun kararı olması.
#[tauri::command]
pub fn set_silence_days(state: State<'_, AppState>, days: i64) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    db::set_setting(&conn, SILENCE_KEY, &days.clamp(0, 365).to_string())?;
    Ok(())
}

/// Eşiği aşan, **sonraki adımı olmayan** kişiler — kuyruğun sessizlik dalı.
///
/// ⚠️ Sonraki adımı olan kişi buraya girmiyor: zaten tarih verilmiş, ikinci kez hatırlatmak
/// gürültü olurdu. Hiç teması olmayan kişi de girmiyor — "sessizlik" bir ilişkinin
/// soğuması demek, henüz başlamamış bir ilişkinin değil.
pub(crate) fn silent_contacts(conn: &Connection) -> Vec<DueContact> {
    let days = super::setting_i64(conn, SILENCE_KEY, 0);
    if days <= 0 {
        return Vec::new();
    }
    let bugun = now_str()[..10].to_string();
    oku(
        conn,
        &format!(
            "{DUE_SELECT} WHERE archived = 0 AND next_step_at IS NULL
             AND last_contact_at IS NOT NULL
             AND julianday(?1) - julianday(date(last_contact_at)) >= {days}"
        ),
        &[&bugun],
        map_sessiz,
    )
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

    /// 🔴 Eşik KAPALI doğuyor: veri yokken eşik uydurmak Faz D'de elenen kalemin aynısı.
    #[test]
    fn sessizlik_esigi_kapaliyken_kimse_kuyruga_girmiyor() {
        let conn = db();
        let id = ekle(&conn, "Soğumuş", None);
        conn.execute(
            "UPDATE contacts SET last_contact_at = ?2 WHERE id = ?1",
            params![id, (chrono::Local::now() - chrono::Duration::days(90)).to_rfc3339()],
        )
        .unwrap();

        assert!(silent_contacts(&conn).is_empty(), "eşik kapalıyken sessiz kişi çıkmamalı");

        seo_core::db::set_setting(&conn, "contact_silence_days", "30").unwrap();
        let s = silent_contacts(&conn);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].reason(), "90 gündür temas yok", "cümle sessizlik dalından gelmeli");
    }

    /// ⚠️ Sonraki adımı olan kişi eşikten etkilenmiyor — çift hatırlatma gürültüdür.
    #[test]
    fn sonraki_adimi_olan_kisi_sessizlik_dalina_girmiyor() {
        let conn = db();
        seo_core::db::set_setting(&conn, "contact_silence_days", "30").unwrap();
        let id = ekle(&conn, "Sözü var", Some(&gun(20)));
        conn.execute(
            "UPDATE contacts SET last_contact_at = ?2 WHERE id = ?1",
            params![id, (chrono::Local::now() - chrono::Duration::days(90)).to_rfc3339()],
        )
        .unwrap();

        assert!(silent_contacts(&conn).is_empty(), "randevusu olan kişi ikinci kez çağrılmamalı");
        assert!(due_contacts(&conn).is_empty(), "randevu 20 gün sonra, bugünün işi değil");
    }

    /// Gerçek bir Türkçe Excel dosyasıyla uçtan uca ölçüm.
    ///
    /// `SEO_CSV=/tmp/musteriler.csv cargo test csv_import_real -- --ignored --nocapture`
    ///
    /// ⚠️ Ad `csv_real` DEĞİL: Faz D'nin `decisions::csv_real` testi de var ve `cargo test
    /// csv_real` ikisini birden çalıştırıp SEO_DB_COPY olmadan başarısız oluyordu.
    ///
    /// Fazın bitiş şartlarından biri: dosya **açılıp** aktarıldı ve sayılar yazıldı.
    #[test]
    #[ignore]
    fn csv_import_real() {
        let yol = std::env::var("SEO_CSV").expect("SEO_CSV yok");
        let metin = std::fs::read_to_string(&yol).unwrap();
        let p = seo_core::csv_import::parse(&metin).unwrap();
        let esleme = seo_core::csv_import::guess_mapping(&p.headers);

        println!("ayraç: {:?} · başlık: {:?}", p.delimiter, p.headers);
        println!("satır: {}", p.rows.len());
        println!("tahmin: {esleme:?}");

        let conn = db();
        let now = now_str();
        let (mut eklendi, mut guncellendi, mut atlandi) = (0, 0, 0);
        for row in &p.rows {
            let al = |f: &str| seo_core::csv_import::field_of(row, &esleme, f);
            let (ad, eposta) = (al("name"), al("email"));
            if ad.is_empty() {
                atlandi += 1;
                continue;
            }
            let mevcut: Option<i64> = conn
                .query_row(
                    "SELECT id FROM contacts WHERE ?1 <> '' AND lower(email) = lower(?1)",
                    params![eposta],
                    |r| r.get(0),
                )
                .ok();
            if let Some(id) = mevcut {
                conn.execute(
                    "UPDATE contacts SET company = ?2 WHERE id = ?1",
                    params![id, al("company")],
                )
                .unwrap();
                guncellendi += 1;
            } else {
                conn.execute(
                    "INSERT INTO contacts (name, company, email, phone, created_at, updated_at)
                     VALUES (?1,?2,?3,?4,?5,?5)",
                    params![ad, al("company"), eposta, al("phone"), now],
                )
                .unwrap();
                eklendi += 1;
            }
        }
        println!("{eklendi} eklendi · {guncellendi} güncellendi · {atlandi} atlandı");

        let toplam: i64 =
            conn.query_row("SELECT COUNT(*) FROM contacts", [], |r| r.get(0)).unwrap();
        println!("veritabanındaki kişi: {toplam}");
        for (ad, firma) in conn
            .prepare("SELECT name, company FROM contacts ORDER BY id")
            .unwrap()
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .unwrap()
            .filter_map(Result::ok)
        {
            println!("  {ad} — {firma}");
        }
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
