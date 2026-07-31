//! Ürün listesi, detay okuma ve durum işaretleme komutları.

use super::*;

#[tauri::command]
pub async fn sync_feed(state: State<'_, AppState>) -> Result<sync::SyncSummary, String> {
    let url = {
        let conn = state.conn.lock().unwrap();
        db::feed_url(&conn)?
    };
    // Feed adresi artık varsayılana düşmüyor (bkz. db::feed_url) → boşsa kullanıcıya ne
    // yapması gerektiği söylenmeli. Aksi halde `fetch_and_parse` teknik bir URL hatası döner.
    if url.trim().is_empty() {
        return Err(
            "Feed adresi ayarlı değil. Ayarlar'dan girin veya kurulum sihirbazını çalıştırın."
                .to_string(),
        );
    }
    let items = feed::fetch_and_parse(&url).await?;
    let mut conn = state.conn.lock().unwrap();
    sync::sync_products(&mut conn, items)
}

#[tauri::command]
pub fn get_last_sync(state: State<'_, AppState>) -> Result<Option<sync::SyncSummary>, String> {
    let conn = state.conn.lock().unwrap();
    sync::last_sync(&conn)
}

#[tauri::command]
pub fn list_products(
    state: State<'_, AppState>,
    filter: Option<String>,
    search: Option<String>,
) -> Result<Vec<ProductRow>, String> {
    let conn = state.conn.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT p.sku, p.name, p.brand, p.img_url, p.title, p.descriptions, p.details,
                    COALESCE(s.meta_status,'pending'), COALESCE(s.details_status,'pending'),
                    s.target_keyword, s.draft_title, s.draft_descriptions, s.draft_details,
                    COALESCE(s.tech_status,'pending'), s.tech_specs_json,
                    p.picture2, p.picture3, p.picture4,
                    p.feed_fp, s.reviewed_fp, p.feed_changed
             FROM products p LEFT JOIN seo_status s ON s.sku = p.sku
             ORDER BY p.name COLLATE NOCASE",
        )
        .map_err(|e| format!("Ürün listesi hazırlanamadı: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(RowData {
                sku: row.get(0)?,
                name: row.get(1)?,
                brand: row.get(2)?,
                img_url: row.get(3)?,
                title: row.get(4)?,
                descriptions: row.get(5)?,
                details: row.get(6)?,
                meta_status: row.get(7)?,
                details_status: row.get(8)?,
                target_keyword: row.get(9)?,
                draft_title: row.get(10)?,
                draft_descriptions: row.get(11)?,
                draft_details: row.get(12)?,
                tech_status: row.get(13)?,
                tech_specs_json: row.get(14)?,
                image_count: [
                    row.get::<_, Option<String>>(3)?,   // img_url
                    row.get::<_, Option<String>>(15)?,  // picture2
                    row.get::<_, Option<String>>(16)?,  // picture3
                    row.get::<_, Option<String>>(17)?,  // picture4
                ]
                .into_iter()
                .filter(|u| u.as_deref().map_or(false, |x| !x.trim().is_empty()))
                .count(),
                feed_changed: feed_change_note(row.get(18)?, row.get(19)?, row.get(20)?),
            })
        })
        .map_err(|e| format!("Ürün listesi okunamadı: {e}"))?
        .filter_map(Result::ok);

    let q = search.unwrap_or_default().trim().to_lowercase();
    let f = filter.unwrap_or_default();
    let mut out = Vec::new();
    for r in rows {
        if !q.is_empty()
            && !r.name.to_lowercase().contains(&q)
            && !r.sku.to_lowercase().contains(&q)
        {
            continue;
        }
        let meta = meta_badge_of(&r);
        let details = details_badge_of(&r);
        let meta_done = r.meta_status == "done";
        let details_done = r.details_status == "done";
        let has_tech = r
            .tech_specs_json
            .as_deref()
            .map(str::trim)
            .map_or(false, |j| !j.is_empty() && j != "[]");
        let overall = overall_status(&OverallInput {
            meta,
            details,
            meta_done,
            details_done,
            tech_done: r.tech_status == "done",
            has_tech,
            image_count: r.image_count,
        });
        let keep = match f.as_str() {
            "" | "hepsi" => true, // filtre yok: tamamlananlar dahil her şey
            "tumu" => overall != OverallStatus::Tamamlandi,
            "eksik" => overall == OverallStatus::Eksik,
            "hatali" => overall == OverallStatus::Hatali,
            "bekliyor" => overall == OverallStatus::Bekliyor,
            "uygun" => overall == OverallStatus::Uygun,
            "tamamlandi" => overall == OverallStatus::Tamamlandi,
            // Onaydan sonra kaynak verisi değişenler — "sessizce bayatlayan" ürünler.
            "degisti" => r.feed_changed.is_some(),
            other => return Err(format!("Bilinmeyen filtre: {other}")),
        };
        if keep {
            out.push(ProductRow {
                sku: r.sku,
                name: r.name,
                brand: r.brand,
                img_url: r.img_url,
                meta_badge: meta,
                details_badge: details,
                overall,
                meta_done,
                details_done,
                tech_done: r.tech_status == "done",
                image_count: r.image_count,
                feed_changed: r.feed_changed,
            });
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn get_product(state: State<'_, AppState>, sku: String) -> Result<ProductDetail, String> {
    let conn = state.conn.lock().unwrap();
    read_detail(&conn, &sku)
}

#[tauri::command]
pub fn set_target_keyword(state: State<'_, AppState>, sku: String, kw: String) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    conn.execute(
        "UPDATE seo_status SET target_keyword = ?2, updated_at = ?3 WHERE sku = ?1",
        params![sku, kw.trim(), now_str()],
    )
    .map_err(|e| format!("Hedef kelime kaydedilemedi: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn save_meta_draft(
    state: State<'_, AppState>,
    sku: String,
    title: String,
    descriptions: String,
    search_keywords: String,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    conn.execute(
        "UPDATE seo_status SET draft_title = ?2, draft_descriptions = ?3,
                draft_search_keywords = ?4, updated_at = ?5
         WHERE sku = ?1",
        params![sku, title, descriptions, search_keywords, now_str()],
    )
    .map_err(|e| format!("Taslak kaydedilemedi: {e}"))?;
    Ok(())
}

/// Ürünün Schema.org `Product` JSON-LD çıktısı — sayfaya yapıştırılmaya hazır script etiketi.
///
/// Veri **üretilmiyor**, elde olandan derleniyor: ad/sku/marka/kategori feed'den, açıklama
/// meta alanından (taslak öncelikli — sayfaya çıkacak olan o), görseller galeriden, özellikler
/// teknik tablodan. Model çağrısı yok, dolayısıyla halüsinasyon yüzeyi de yok.
///
/// ⚠️ Fiyat/stok/puan bilinçli olarak DIŞARIDA — gerekçesi `seo_core::jsonld` başlığında.
#[tauri::command]
pub fn get_jsonld(state: State<'_, AppState>, sku: String) -> Result<String, String> {
    let conn = state.conn.lock().unwrap();
    let d = read_detail(&conn, &sku)?;
    let facts = jsonld::ProductFacts {
        name: d.name,
        sku: d.sku,
        brand: d.brand.unwrap_or_default(),
        // Alt kategori daha belirleyici; yoksa ana kategoriye düşülüyor.
        category: d.category.or(d.main_category).unwrap_or_default(),
        // Taslak varsa taslak: yayımlandığında sayfada duracak olan metin o.
        description: d.draft_descriptions.or(d.descriptions).unwrap_or_default(),
        url: d.url.unwrap_or_default(),
        images: d.gallery,
    };
    let specs = d.tech_specs.unwrap_or_default();
    jsonld::build(&facts, &specs)
        .map(|n| jsonld::render_script(&n))
        .ok_or_else(|| {
            "JSON-LD üretilemedi: ürünün adı veya sayfa adresi yok. Feed'i senkronlayın."
                .to_string()
        })
}

/// "Baktım, içerik hâlâ doğru" — bayrağı düşürür, içeriğe DOKUNMAZ.
///
/// Feed değiştiğinde her zaman yeniden üretim gerekmiyor: bazen değişen alan zaten
/// üretilmiş metni etkilemiyor. Bu düğme olmasaydı kullanıcının bayraktan kurtulmak için
/// "tamamlandı"yı kapatıp açması gerekirdi — durum alanını yalan söylemeye zorlayan bir çözüm.
#[tauri::command]
pub fn mark_feed_reviewed(state: State<'_, AppState>, sku: String) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    mark_reviewed(&conn, &sku)
}

#[tauri::command]
pub fn mark_meta_done(state: State<'_, AppState>, sku: String) -> Result<String, String> {
    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    let current: String = conn
        .query_row("SELECT meta_status FROM seo_status WHERE sku = ?1", [&sku], |r| r.get(0))
        .map_err(|e| format!("SEO durumu okunamadı: {e}"))?;
    let next = if current == "done" { "pending" } else { "done" };
    conn.execute(
        "UPDATE seo_status SET meta_status = ?2, updated_at = ?3 WHERE sku = ?1",
        params![sku, next, now_str()],
    )
    .map_err(|e| format!("SEO durumu güncellenemedi: {e}"))?;
    if next == "done" {
        mark_reviewed(&conn, &sku)?;
    }
    Ok(next.to_string())
}

#[tauri::command]
pub fn mark_details_done(state: State<'_, AppState>, sku: String) -> Result<String, String> {
    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    let current: String = conn
        .query_row("SELECT details_status FROM seo_status WHERE sku = ?1", [&sku], |r| r.get(0))
        .map_err(|e| format!("SEO durumu okunamadı: {e}"))?;
    let next = if current == "done" { "pending" } else { "done" };
    conn.execute(
        "UPDATE seo_status SET details_status = ?2, updated_at = ?3 WHERE sku = ?1",
        params![sku, next, now_str()],
    )
    .map_err(|e| format!("SEO durumu güncellenemedi: {e}"))?;
    if next == "done" {
        mark_reviewed(&conn, &sku)?;
    }
    Ok(next.to_string())
}
