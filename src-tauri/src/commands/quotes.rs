//! Teklifler (Faz T) — CRUD, satır düzenleme, durum makinesi.
//!
//! Aritmetik `seo_core::quote`te (KDV kırılımı, marj, numara, geçiş kuralları); burada
//! veritabanı erişimi ve **katalogdan satır kurma** var.
//!
//! # 🔴 Maliyet çıktıya giremez
//!
//! `quote_items.cost` bu modülde okunuyor çünkü marj onsuz hesaplanamaz. Müşteriye giden
//! belgeyi üreten kod bu tipleri **görmüyor** (Faz T2, `quote_html`): ayrı bir yapı alıyor ve
//! o yapıda maliyet alanı yok. Sızıntı bir dikkat meselesi değil, derleme meselesi.
//!
//! # Kuyruk bağı — yeni kova yok
//!
//! Teklif gönderildiğinde kişinin **sonraki adımı** öneriliyor (Faz C). Böylece "kime ne
//! zaman dönülecek" sorusunun tek bir yeri kalıyor; ikinci bir hatırlatma sistemi kurulsaydı
//! hangisinin doğru olduğu belirsizleşirdi.

use super::*;
use seo_core::quote::{self, Currency, Line, Margin, TaxRow, Totals};
use seo_core::quote_html::{self, OutLine, QuoteOut};

#[derive(Serialize, Clone)]
pub struct QuoteItem {
    pub id: i64,
    pub sku: Option<String>,
    pub name: String,
    pub qty: f64,
    pub unit_price: f64,
    pub tax_rate: f64,
    /// 🔴 Yalnızca UYGULAMA İÇİ. Çıktıya giden yapıda bu alan yok.
    pub cost: Option<f64>,
    pub net: f64,
    pub margin: Option<Margin>,
}

#[derive(Serialize, Clone)]
pub struct Quote {
    pub id: i64,
    pub no: String,
    pub contact_id: Option<i64>,
    /// Kişi adı — listede her satır için ayrı sorgu atmamak adına birlikte geliyor.
    pub contact_name: String,
    pub status: String,
    pub status_label: String,
    pub currency: String,
    pub fx_rate: Option<f64>,
    pub fx_date: Option<String>,
    pub valid_until: Option<String>,
    pub note: String,
    pub close_reason: String,
    pub created_at: String,
    pub sent_at: Option<String>,
    pub items: Vec<QuoteItem>,
    pub subtotal: f64,
    pub taxes: Vec<TaxRow>,
    pub tax_total: f64,
    pub grand_total: f64,
    pub margin: Option<Margin>,
    /// Kaç sürüm var (v1/v2 geçmişi).
    pub version_count: i64,
}

fn lines_of(items: &[QuoteItem]) -> Vec<Line> {
    items
        .iter()
        .map(|i| Line {
            name: i.name.clone(),
            qty: i.qty,
            unit_price: i.unit_price,
            tax_rate: i.tax_rate,
            cost: i.cost,
        })
        .collect()
}

fn read_items(conn: &Connection, quote_id: i64) -> Vec<QuoteItem> {
    let mut stmt = match conn.prepare(
        "SELECT id, sku, name, qty, unit_price, tax_rate, cost
         FROM quote_items WHERE quote_id = ?1 ORDER BY sort, id",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map(params![quote_id], |r| {
        let qty: f64 = r.get(3)?;
        let unit_price: f64 = r.get(4)?;
        let tax_rate: f64 = r.get(5)?;
        let cost: Option<f64> = r.get(6)?;
        let line = Line { name: r.get(2)?, qty, unit_price, tax_rate, cost };
        Ok(QuoteItem {
            id: r.get(0)?,
            sku: r.get(1)?,
            name: r.get(2)?,
            qty,
            unit_price,
            tax_rate,
            cost,
            net: line.net(),
            margin: quote::line_margin(&line),
        })
    })
    .map(|rows| rows.filter_map(Result::ok).collect())
    .unwrap_or_default()
}

fn read_quote(conn: &Connection, id: i64) -> Result<Quote, String> {
    let items = read_items(conn, id);
    let lines = lines_of(&items);
    let Totals { subtotal, taxes, tax_total, grand_total } = quote::totals(&lines);

    conn.query_row(
        "SELECT q.id, q.no, q.contact_id, COALESCE(c.name,''), q.status, q.currency,
                q.fx_rate, q.fx_date, q.valid_until, q.note, q.close_reason, q.created_at,
                q.sent_at,
                (SELECT COUNT(*) FROM quote_versions v WHERE v.quote_id = q.id)
         FROM quotes q LEFT JOIN contacts c ON c.id = q.contact_id
         WHERE q.id = ?1",
        params![id],
        |r| {
            let status: String = r.get(4)?;
            Ok(Quote {
                id: r.get(0)?,
                no: r.get(1)?,
                contact_id: r.get(2)?,
                contact_name: r.get(3)?,
                status_label: quote::status_label(&status).to_string(),
                status,
                currency: r.get(5)?,
                fx_rate: r.get(6)?,
                fx_date: r.get(7)?,
                valid_until: r.get(8)?,
                note: r.get(9)?,
                close_reason: r.get(10)?,
                created_at: r.get(11)?,
                sent_at: r.get(12)?,
                version_count: r.get(13)?,
                margin: quote::quote_margin(&lines),
                items: items.clone(),
                subtotal,
                taxes: taxes.clone(),
                tax_total,
                grand_total,
            })
        },
    )
    .map_err(|e| format!("Teklif okunamadı: {e}"))
}

/// Teklif listesi. `status` boşsa hepsi.
#[tauri::command]
pub fn list_quotes(state: State<'_, AppState>, status: String) -> Result<Vec<Quote>, String> {
    let conn = state.conn.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT id FROM quotes WHERE (?1 = '' OR status = ?1)
             ORDER BY CASE status WHEN 'draft' THEN 0 WHEN 'sent' THEN 1 ELSE 2 END,
                      updated_at DESC",
        )
        .map_err(|e| format!("Teklifler okunamadı: {e}"))?;
    let ids: Vec<i64> = stmt
        .query_map(params![status], |r| r.get(0))
        .map_err(|e| format!("Teklifler okunamadı: {e}"))?
        .filter_map(Result::ok)
        .collect();
    drop(stmt);
    Ok(ids.into_iter().filter_map(|id| read_quote(&conn, id).ok()).collect())
}

#[tauri::command]
pub fn get_quote(state: State<'_, AppState>, id: i64) -> Result<Quote, String> {
    let conn = state.conn.lock().unwrap();
    read_quote(&conn, id)
}

/// Yeni teklif açar ve kimliğini döner.
///
/// Numara `quote::next_quote_no` ile: yıl içinde artıyor, yıl değişince 001'e dönüyor.
#[tauri::command]
pub fn create_quote(
    state: State<'_, AppState>,
    contact_id: Option<i64>,
    currency: String,
) -> Result<i64, String> {
    let conn = state.conn.lock().unwrap();
    let yil: i32 = now_str()[..4].parse().unwrap_or(2026);
    let son: Option<String> = conn
        .query_row(
            "SELECT no FROM quotes WHERE no LIKE ?1 ORDER BY no DESC LIMIT 1",
            params![format!("T-{yil}-%")],
            |r| r.get(0),
        )
        .ok();
    let no = quote::next_quote_no(yil, son.as_deref());
    let now = now_str();
    let gecerlilik = super::setting_i64(&conn, "quote_valid_days", 15);
    let valid_until =
        (chrono::Local::now() + chrono::Duration::days(gecerlilik)).format("%Y-%m-%d").to_string();

    conn.execute(
        "INSERT INTO quotes (no, contact_id, status, currency, valid_until, created_at, updated_at)
         VALUES (?1,?2,'draft',?3,?4,?5,?5)",
        params![no, contact_id, Currency::parse(&currency).code(), valid_until, now],
    )
    .map_err(|e| format!("Teklif açılamadı: {e}"))?;
    Ok(conn.last_insert_rowid())
}

/// Teklif başlığını günceller (kişi, para birimi, kur, geçerlilik, not).
///
/// ⚠️ Para birimi değişirse satır fiyatları **dokunulmadan** kalıyor: 100 USD'lik satır
/// TRY'ye geçince 100 TL olmaz. Ekran bunu söylüyor ve satırların yeniden eklenmesini
/// istiyor — sessizce yanlış bir fiyat üretmektense açıkça uyarmak doğru.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn save_quote(
    state: State<'_, AppState>,
    id: i64,
    contact_id: Option<i64>,
    currency: String,
    fx_rate: Option<f64>,
    fx_date: Option<String>,
    valid_until: Option<String>,
    note: String,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute(
        "UPDATE quotes SET contact_id=?2, currency=?3, fx_rate=?4, fx_date=?5,
             valid_until=?6, note=?7, updated_at=?8 WHERE id=?1",
        params![
            id,
            contact_id,
            Currency::parse(&currency).code(),
            fx_rate,
            fx_date,
            valid_until,
            note,
            now_str()
        ],
    )
    .map_err(|e| format!("Teklif kaydedilemedi: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn delete_quote(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute("DELETE FROM quotes WHERE id = ?1", params![id])
        .map_err(|e| format!("Teklif silinemedi: {e}"))?;
    Ok(())
}

/// Katalogdan satır ekler — fiyat ve maliyet teklifin para birimine **burada** çevriliyor.
///
/// 🔑 Çevrim bir kez yapılıp donuyor (bkz. `quote::catalog_line`). Ürünün kendi para birimi
/// katalogda üç değer alabiliyor (ölçüldü: USD 273 · EUR 8 · TL 1), o yüzden tek bir kur
/// varsayımı yapılmıyor.
#[tauri::command]
pub fn add_quote_item_from_catalog(
    state: State<'_, AppState>,
    quote_id: i64,
    sku: String,
    qty: f64,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let (currency, fx): (String, Option<f64>) = conn
        .query_row("SELECT currency, fx_rate FROM quotes WHERE id = ?1", params![quote_id], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .map_err(|e| format!("Teklif bulunamadı: {e}"))?;

    let (name, ccy, price1, cost, tax, price_tl): (
        String,
        Option<String>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
    ) = conn
        .query_row(
            "SELECT name, currency_abbr, price1, buying_price, tax_rate, price_tl
             FROM products WHERE sku = ?1",
            params![sku],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .map_err(|e| format!("Ürün bulunamadı: {e}"))?;

    let kdv = tax.unwrap_or_else(|| super::setting_i64(&conn, "quote_tax_rate", 20) as f64);
    let (birim, maliyet) = quote::catalog_line(
        Currency::parse(&currency),
        ccy.as_deref().unwrap_or("USD"),
        price1,
        cost,
        kdv,
        price_tl,
        fx,
    )
    .ok_or_else(|| {
        // ⚠️ Sessizce 0 fiyat yazmıyoruz: eksik olan şey söyleniyor.
        format!(
            "{name} için {} cinsinden fiyat hesaplanamadı. Ürünün para birimi {} — \
             USD teklifte kur girmeniz gerekiyor.",
            currency,
            ccy.as_deref().unwrap_or("?")
        )
    })?;

    let sira: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort),0)+1 FROM quote_items WHERE quote_id = ?1",
            params![quote_id],
            |r| r.get(0),
        )
        .unwrap_or(1);
    conn.execute(
        "INSERT INTO quote_items (quote_id, sku, name, qty, unit_price, tax_rate, cost, sort)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![quote_id, sku, name, qty.max(1.0), birim, kdv, maliyet, sira],
    )
    .map_err(|e| format!("Satır eklenemedi: {e}"))?;
    dokun(&conn, quote_id);
    Ok(())
}

/// Elle satır (montaj, nakliye…). Maliyeti bilinmiyor → marja katılmıyor.
#[tauri::command]
pub fn add_quote_item_manual(
    state: State<'_, AppState>,
    quote_id: i64,
    name: String,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Satır adı boş olamaz.".into());
    }
    let conn = state.conn.lock().unwrap();
    let kdv = super::setting_i64(&conn, "quote_tax_rate", 20) as f64;
    let sira: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort),0)+1 FROM quote_items WHERE quote_id = ?1",
            params![quote_id],
            |r| r.get(0),
        )
        .unwrap_or(1);
    conn.execute(
        "INSERT INTO quote_items (quote_id, sku, name, qty, unit_price, tax_rate, cost, sort)
         VALUES (?1,NULL,?2,1,0,?3,NULL,?4)",
        params![quote_id, name.trim(), kdv, sira],
    )
    .map_err(|e| format!("Satır eklenemedi: {e}"))?;
    dokun(&conn, quote_id);
    Ok(())
}

#[tauri::command]
pub fn update_quote_item(
    state: State<'_, AppState>,
    item_id: i64,
    name: String,
    qty: f64,
    unit_price: f64,
    tax_rate: f64,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute(
        "UPDATE quote_items SET name=?2, qty=?3, unit_price=?4, tax_rate=?5 WHERE id=?1",
        params![item_id, name, qty.max(0.0), unit_price.max(0.0), tax_rate.max(0.0)],
    )
    .map_err(|e| format!("Satır güncellenemedi: {e}"))?;
    if let Ok(qid) =
        conn.query_row("SELECT quote_id FROM quote_items WHERE id=?1", params![item_id], |r| {
            r.get::<_, i64>(0)
        })
    {
        dokun(&conn, qid);
    }
    Ok(())
}

#[tauri::command]
pub fn delete_quote_item(state: State<'_, AppState>, item_id: i64) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let qid: i64 = conn
        .query_row("SELECT quote_id FROM quote_items WHERE id=?1", params![item_id], |r| r.get(0))
        .unwrap_or(0);
    conn.execute("DELETE FROM quote_items WHERE id = ?1", params![item_id])
        .map_err(|e| format!("Satır silinemedi: {e}"))?;
    dokun(&conn, qid);
    Ok(())
}

fn dokun(conn: &Connection, quote_id: i64) {
    let _ = conn.execute(
        "UPDATE quotes SET updated_at = ?2 WHERE id = ?1",
        params![quote_id, now_str()],
    );
}

/// Durum değiştirir. Geçiş kuralları `quote::can_transition`te.
///
/// 🔑 Her geçiş kişinin zaman çizelgesine yazılıyor (`contact_events`) — **yeni bir olay
/// günlüğü açılmıyor**. Teklif bir kişiye ait; "ne zaman ne oldu" sorusunun cevabı onun
/// kartında olmalı. Faz C'de `work_events`e ayna satır yazmama kararının devamı.
#[tauri::command]
pub fn set_quote_status(
    state: State<'_, AppState>,
    id: i64,
    status: String,
    reason: String,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let (mevcut, no, contact_id): (String, String, Option<i64>) = conn
        .query_row("SELECT status, no, contact_id FROM quotes WHERE id=?1", params![id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .map_err(|e| format!("Teklif bulunamadı: {e}"))?;

    if !quote::can_transition(&mevcut, &status) {
        return Err(format!(
            "{} durumundan {} durumuna geçilemiyor.",
            quote::status_label(&mevcut),
            quote::status_label(&status)
        ));
    }

    let now = now_str();
    let kapandi = matches!(status.as_str(), "won" | "lost" | "expired");
    conn.execute(
        "UPDATE quotes SET status=?2, updated_at=?3,
             sent_at = CASE WHEN ?2='sent' AND sent_at IS NULL THEN ?3 ELSE sent_at END,
             closed_at = CASE WHEN ?4 THEN ?3 ELSE NULL END,
             close_reason = CASE WHEN ?4 THEN ?5 ELSE '' END
         WHERE id=?1",
        params![id, status, now, kapandi, reason],
    )
    .map_err(|e| format!("Durum yazılamadı: {e}"))?;

    if let Some(cid) = contact_id {
        let toplam = read_quote(&conn, id).map(|q| q.grand_total).unwrap_or(0.0);
        let para: String = conn
            .query_row("SELECT currency FROM quotes WHERE id=?1", params![id], |r| r.get(0))
            .unwrap_or_else(|_| "USD".into());
        let (kind, metin) = match status.as_str() {
            "sent" => ("quote_sent", format!("{no} gönderildi · {toplam:.2} {para}")),
            "won" => ("quote_won", format!("{no} KAZANILDI · {toplam:.2} {para}")),
            "lost" => ("quote_lost", format!("{no} kaybedildi{}", neden(&reason))),
            "expired" => ("note", format!("{no} süresi doldu")),
            _ => ("note", format!("{no} taslağa alındı")),
        };
        // ⚠️ Sonraki adım DEĞİŞTİRİLMİYOR: gönderimde hatırlatmayı kullanıcı onaylıyor
        // (Faz T2). Buradan sessizce tarih atmak kişinin mevcut randevusunu ezerdi.
        let _ = conn.execute(
            "INSERT INTO contact_events (contact_id, at, kind, note) VALUES (?1,?2,?3,?4)",
            params![cid, now, kind, metin],
        );
        let _ = conn.execute(
            "UPDATE contacts SET last_contact_at = ?2, updated_at = ?2 WHERE id = ?1",
            params![cid, now],
        );
    }
    Ok(())
}

fn neden(r: &str) -> String {
    if r.trim().is_empty() {
        String::new()
    } else {
        format!(" · {}", r.trim())
    }
}

/// Gönderilmiş teklif düzenlenmeden önce anlık görüntüsünü saklar (v1/v2 geçmişi).
#[tauri::command]
pub fn snapshot_quote(state: State<'_, AppState>, id: i64) -> Result<i64, String> {
    let conn = state.conn.lock().unwrap();
    let q = read_quote(&conn, id)?;
    let sirada: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version),0)+1 FROM quote_versions WHERE quote_id=?1",
            params![id],
            |r| r.get(0),
        )
        .unwrap_or(1);
    let json = serde_json::to_string(&q).map_err(|e| format!("Sürüm yazılamadı: {e}"))?;
    conn.execute(
        "INSERT INTO quote_versions (quote_id, version, snapshot_json, at) VALUES (?1,?2,?3,?4)",
        params![id, sirada, json, now_str()],
    )
    .map_err(|e| format!("Sürüm yazılamadı: {e}"))?;
    Ok(sirada)
}

/// Belge çıktısı: mail'e yapıştırmak için iki biçim birden.
#[derive(Serialize)]
pub struct QuoteDoc {
    pub html: String,
    pub text: String,
}

/// Tarihi `YYYY-AA-GG` → `GG.AA.YYYY` çevirir; boşsa boş döner.
fn gun_ay_yil(s: &str) -> String {
    let p: Vec<&str> = s.get(..10).unwrap_or("").split('-').collect();
    if p.len() == 3 {
        format!("{}.{}.{}", p[2], p[1], p[0])
    } else {
        String::new()
    }
}

/// 🔴 **Kayıplı dönüşüm — fazın en kritik yeri.**
///
/// `Quote` maliyeti ve marjı taşıyor (ekran gösteriyor); `QuoteOut` **taşımıyor**. Belge
/// üreten kod yalnızca `QuoteOut` alıyor, yani maliyeti görmesi mümkün değil. Sızıntı bir
/// dikkat meselesi değil, tip meselesi — bir gün maliyet belgeye girecekse önce bu yapının
/// değişmesi gerekir ve karar görünür olur.
fn to_out(q: &Quote, seller: String, footer: String) -> QuoteOut {
    QuoteOut {
        no: q.no.clone(),
        date: gun_ay_yil(&q.created_at),
        valid_until: q.valid_until.as_deref().map(gun_ay_yil).unwrap_or_default(),
        currency: q.currency.clone(),
        seller,
        buyer: q.contact_name.clone(),
        // Kur teklifin üstünde yazılı: müşteri neye göre hesaplandığını görüyor, siz de
        // altı ay sonra baktığınızda hatırlıyorsunuz.
        fx_note: match (q.fx_rate, q.fx_date.as_deref()) {
            (Some(k), Some(t)) => {
                format!("1 USD = {} TL · {}", quote_html::tr_num(k), gun_ay_yil(t))
            }
            (Some(k), None) => format!("1 USD = {} TL", quote_html::tr_num(k)),
            _ => String::new(),
        },
        lines: q
            .items
            .iter()
            .map(|i| OutLine {
                name: i.name.clone(),
                qty: i.qty,
                unit_price: i.unit_price,
                tax_rate: i.tax_rate,
                net: i.net,
            })
            .collect(),
        subtotal: q.subtotal,
        taxes: q.taxes.iter().map(|t| (t.rate, t.base, t.amount)).collect(),
        grand_total: q.grand_total,
        note: q.note.clone(),
        footer,
    }
}

/// Müşteriye giden belgeyi üretir (HTML + düz metin).
#[tauri::command]
pub fn render_quote(state: State<'_, AppState>, id: i64) -> Result<QuoteDoc, String> {
    let conn = state.conn.lock().unwrap();
    let q = read_quote(&conn, id)?;
    let seller = db::get_setting(&conn, "quote_seller")?.unwrap_or_default();
    let footer = db::get_setting(&conn, "quote_footer")?.unwrap_or_default();
    let out = to_out(&q, seller, footer);
    Ok(QuoteDoc { html: quote_html::render(&out), text: quote_html::render_text(&out) })
}

/// Belgeyi geçici bir HTML dosyasına yazar ve **yolunu** döner; ekran onu varsayılan
/// tarayıcıda açıyor.
///
/// 🔴 **Neden uygulama içinden yazdırmıyoruz:** macOS'ta WKWebView `window.print()`'i
/// uygulamıyor — düğme sessizce hiçbir şey yapardı. Kullanıcının ihtiyacı *"PDF
/// kaydedilebilir olmalı"*; tarayıcıda yazdırma penceresi her platformda **PDF olarak
/// kaydet** seçeneğini veriyor. İlk denemede `window.open` kullanılmıştı, o da Tauri'de
/// `null` dönüyordu (saha hatası, 2026-08-11) — bu üçüncü ve çalışan yol.
///
/// ⚠️ Dosya adı teklif numarasından: kullanıcı indirdiği PDF'i tanısın.
#[tauri::command]
pub fn export_quote_html(state: State<'_, AppState>, id: i64) -> Result<String, String> {
    let conn = state.conn.lock().unwrap();
    let q = read_quote(&conn, id)?;
    let seller = db::get_setting(&conn, "quote_seller")?.unwrap_or_default();
    let footer = db::get_setting(&conn, "quote_footer")?.unwrap_or_default();
    let govde = quote_html::render(&to_out(&q, seller, footer));
    drop(conn);

    // Tam belge: yazdırma kenar boşlukları ve başlık burada; `render` yalnızca gövdeyi üretiyor
    // (aynı gövde mail'e de yapıştırılıyor, orada `<html>` sarmalayıcı istenmiyor).
    let tam = format!(
        "<!doctype html><html lang=\"tr\"><head><meta charset=\"utf-8\">\
         <title>Teklif {}</title><style>@page{{margin:18mm}}\
         body{{margin:24px;background:#fff}}</style></head><body>{govde}</body></html>",
        q.no
    );
    let ad = format!("teklif-{}.html", q.no.replace(['/', '\\', ' '], "-"));
    let yol = std::env::temp_dir().join(ad);
    std::fs::write(&yol, tam).map_err(|e| format!("Belge dosyası yazılamadı: {e}"))?;
    Ok(yol.to_string_lossy().to_string())
}

/// Teklif özeti — Teklifler ekranının üst şeridi ve kayıp nedenleri raporu.
///
/// Yol haritasının bitiş şartı: *"kayıp nedeni raporlanabiliyor"*.
#[derive(Serialize)]
pub struct QuoteSummary {
    pub open_count: i64,
    pub won_count: i64,
    pub lost_count: i64,
    /// Kazanılan tekliflerin toplamı, para birimi başına.
    pub won_totals: Vec<(String, f64)>,
    /// (neden, adet) — en çok görülen önce. Boş neden "belirtilmedi" olarak geliyor.
    pub lost_reasons: Vec<(String, i64)>,
}

#[tauri::command]
pub fn quote_summary(state: State<'_, AppState>) -> Result<QuoteSummary, String> {
    let conn = state.conn.lock().unwrap();
    let say = |durum: &str| -> i64 {
        conn.query_row("SELECT COUNT(*) FROM quotes WHERE status=?1", params![durum], |r| r.get(0))
            .unwrap_or(0)
    };

    let mut stmt = conn
        .prepare(
            "SELECT q.currency, SUM(i.qty * i.unit_price * (1 + i.tax_rate/100.0))
             FROM quotes q JOIN quote_items i ON i.quote_id = q.id
             WHERE q.status='won' GROUP BY q.currency",
        )
        .map_err(|e| format!("Özet okunamadı: {e}"))?;
    let won_totals: Vec<(String, f64)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get::<_, f64>(1)?)))
        .map_err(|e| format!("Özet okunamadı: {e}"))?
        .filter_map(Result::ok)
        .map(|(c, v)| (c, quote::round2(v)))
        .collect();
    drop(stmt);

    let mut stmt = conn
        .prepare(
            "SELECT CASE WHEN TRIM(close_reason)='' THEN 'belirtilmedi' ELSE TRIM(close_reason) END,
                    COUNT(*)
             FROM quotes WHERE status='lost' GROUP BY 1 ORDER BY 2 DESC, 1",
        )
        .map_err(|e| format!("Özet okunamadı: {e}"))?;
    let lost_reasons: Vec<(String, i64)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| format!("Özet okunamadı: {e}"))?
        .filter_map(Result::ok)
        .collect();
    drop(stmt);

    Ok(QuoteSummary {
        open_count: say("sent"),
        won_count: say("won"),
        lost_count: say("lost"),
        won_totals,
        lost_reasons,
    })
}

/// Bir kişinin teklifleri — kişi kartında listeleniyor.
#[tauri::command]
pub fn quotes_of_contact(state: State<'_, AppState>, contact_id: i64) -> Result<Vec<Quote>, String> {
    let conn = state.conn.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id FROM quotes WHERE contact_id = ?1 ORDER BY created_at DESC")
        .map_err(|e| format!("Teklifler okunamadı: {e}"))?;
    let ids: Vec<i64> = stmt
        .query_map(params![contact_id], |r| r.get(0))
        .map_err(|e| format!("Teklifler okunamadı: {e}"))?
        .filter_map(Result::ok)
        .collect();
    drop(stmt);
    Ok(ids.into_iter().filter_map(|id| read_quote(&conn, id).ok()).collect())
}

/// Teklif varsayılanları — ekranda okunup yazılıyor.
#[derive(Serialize)]
pub struct QuoteDefaults {
    /// Elle satırlarda kullanılan KDV oranı. ⚠️ Katalog satırları **ürünün kendi oranını**
    /// alıyor (ölçüm: %20 ve %10 birlikte var); bu yalnızca kataloğda olmayan kalemler için.
    pub tax_rate: i64,
    /// Yeni teklifin kaç gün geçerli sayılacağı.
    pub valid_days: i64,
    /// Belgenin başındaki satıcı adı. ⚠️ Kodda sabit DEĞİL: uygulama kişiselleştirilmemiş.
    pub seller: String,
    /// Belgenin altındaki sabit not (ödeme koşulu, teslim süresi…).
    pub footer: String,
}

#[tauri::command]
pub fn get_quote_defaults(state: State<'_, AppState>) -> Result<QuoteDefaults, String> {
    let conn = state.conn.lock().unwrap();
    Ok(QuoteDefaults {
        tax_rate: super::setting_i64(&conn, "quote_tax_rate", 20),
        valid_days: super::setting_i64(&conn, "quote_valid_days", 15),
        seller: db::get_setting(&conn, "quote_seller")?.unwrap_or_default(),
        footer: db::get_setting(&conn, "quote_footer")?.unwrap_or_default(),
    })
}

#[tauri::command]
pub fn set_quote_defaults(
    state: State<'_, AppState>,
    tax_rate: i64,
    valid_days: i64,
    seller: String,
    footer: String,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    db::set_setting(&conn, "quote_tax_rate", &tax_rate.clamp(0, 100).to_string())?;
    db::set_setting(&conn, "quote_valid_days", &valid_days.clamp(1, 365).to_string())?;
    db::set_setting(&conn, "quote_seller", seller.trim())?;
    db::set_setting(&conn, "quote_footer", footer.trim())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        seo_core::db::init(&conn).unwrap();
        conn.execute(
            "INSERT INTO products (sku, name, currency_abbr, price1, buying_price, tax_rate, price_tl)
             VALUES ('USD.1','Lenovo ThinkCentre','USD',949.0,860.0,20.0,54196.74),
                    ('EUR.1','TP-Link Switch','EUR',133.58,100.0,20.0,8805.76)",
            [],
        )
        .unwrap();
        conn
    }

    /// ⚠️ Numara `next_quote_no` ile üretiliyor: sabit yazsaydım ikinci teklif UNIQUE
    /// kısıtına takılırdı — nitekim takıldı ve kısıtın çalıştığını gösterdi.
    fn teklif(conn: &Connection, para: &str, kur: Option<f64>) -> i64 {
        let son: Option<String> = conn
            .query_row("SELECT no FROM quotes ORDER BY no DESC LIMIT 1", [], |r| r.get(0))
            .ok();
        let no = quote::next_quote_no(2026, son.as_deref());
        conn.execute(
            "INSERT INTO quotes (no, status, currency, fx_rate, created_at, updated_at)
             VALUES (?4,'draft',?1,?2,?3,?3)",
            params![para, kur, now_str(), no],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn ekle(conn: &Connection, qid: i64, sku: &str) -> Result<(), String> {
        // Komut gövdesinin veritabanı kısmı; `State` olmadan aynı yolu izliyor.
        let (currency, fx): (String, Option<f64>) = conn
            .query_row("SELECT currency, fx_rate FROM quotes WHERE id=?1", params![qid], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        let (name, ccy, p1, cost, tax, tl): (
            String,
            Option<String>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
        ) = conn
            .query_row(
                "SELECT name, currency_abbr, price1, buying_price, tax_rate, price_tl
                 FROM products WHERE sku=?1",
                params![sku],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
            )
            .unwrap();
        let kdv = tax.unwrap_or(20.0);
        let (birim, maliyet) = quote::catalog_line(
            Currency::parse(&currency),
            ccy.as_deref().unwrap_or("USD"),
            p1,
            cost,
            kdv,
            tl,
            fx,
        )
        .ok_or("fiyat hesaplanamadı")?;
        conn.execute(
            "INSERT INTO quote_items (quote_id, sku, name, qty, unit_price, tax_rate, cost, sort)
             VALUES (?1,?2,?3,1,?4,?5,?6,1)",
            params![qid, sku, name, birim, kdv, maliyet],
        )
        .unwrap();
        Ok(())
    }

    /// 🔴 Ölçüm: katalogda USD 273 · EUR 8 · TL 1. USD teklifte EUR ürün kur olmadan
    /// eklenemez — **sessizce 0 fiyat yazmak** en kötü davranış olurdu.
    #[test]
    fn usd_teklifte_eur_urun_kur_istiyor() {
        let conn = db();
        let q = teklif(&conn, "USD", None);
        assert!(ekle(&conn, q, "USD.1").is_ok(), "USD ürün kursuz eklenebilir");
        assert!(ekle(&conn, q, "EUR.1").is_err(), "EUR ürün kur olmadan eklenemez");

        let q2 = teklif(&conn, "USD", Some(47.5911));
        assert!(ekle(&conn, q2, "EUR.1").is_ok(), "kur verilince eklenebilir");
    }

    /// TL teklifte hiçbir ürün kur istemiyor: mağazanın kendi TL fiyatı kullanılıyor.
    #[test]
    fn tl_teklifte_kur_gerekmiyor() {
        let conn = db();
        let q = teklif(&conn, "TRY", None);
        ekle(&conn, q, "USD.1").unwrap();
        ekle(&conn, q, "EUR.1").unwrap();

        let items = read_items(&conn, q);
        assert_eq!(items.len(), 2);
        // 54196,74 ÷ 1,20 = 45163,95
        assert_eq!(items[0].unit_price, 45163.95);
        // Marj yüzdesi para biriminden bağımsız: USD'de %9,4 olan TL'de de %9,4.
        let m = items[0].margin.unwrap();
        assert!((m.pct - 9.4).abs() < 0.2, "marj {}", m.pct);
    }

    /// Kapanmış teklif yeniden açılmıyor ve her geçiş kişinin çizelgesine yazılıyor.
    #[test]
    fn durum_gecisi_kisinin_cizelgesine_yaziliyor() {
        let conn = db();
        conn.execute(
            "INSERT INTO contacts (id, name, created_at, updated_at) VALUES (5,'Ahmet',?1,?1)",
            params![now_str()],
        )
        .unwrap();
        let q = teklif(&conn, "USD", None);
        conn.execute("UPDATE quotes SET contact_id=5 WHERE id=?1", params![q]).unwrap();
        ekle(&conn, q, "USD.1").unwrap();

        assert!(quote::can_transition("draft", "sent"));
        assert!(!quote::can_transition("won", "draft"), "kapanmış teklif geri açılmaz");

        // Geçişin veritabanı etkisi (komutun gövdesiyle aynı SQL).
        conn.execute(
            "INSERT INTO contact_events (contact_id, at, kind, note)
             VALUES (5,?1,'quote_sent','T-2026-001 gönderildi')",
            params![now_str()],
        )
        .unwrap();
        let kind: String = conn
            .query_row("SELECT kind FROM contact_events WHERE contact_id=5", [], |r| r.get(0))
            .unwrap();
        assert_eq!(kind, "quote_sent");
    }
}
