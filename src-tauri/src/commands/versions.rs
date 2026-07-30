//! Sürüm geçmişi: yeniden üretimden önceki hâli saklama ve geri yükleme.
//!
//! Üç geri yükleme komutu (meta/açıklama/teknik) benzer ŞEKİLDE ama farklı SQL ve yükle
//! çalışıyor; ortak bir soyutlamaya indirilmedi — closure alan bir sarmalayıcı daha az
//! okunur olurdu.

use super::*;

/// Saklanan bir meta sürümü. Hedef kelime de içeride: o meta ona göre üretilmişti,
/// geri yüklerken ikisi birlikte dönmeli (kullanıcı kararı) yoksa çelişkili bir hâl oluşur.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct MetaVersion {
    pub at: String,
    pub title: String,
    pub descriptions: String,
    pub keywords: String,
    pub search_keywords: String,
    pub target_keyword: String,
    #[serde(default)]
    pub model: String,
}

/// Saklanan bir açıklama sürümü.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct DetailsVersion {
    pub at: String,
    pub html: String,
    #[serde(default)]
    pub model: String,
}

/// UI listesi için hafif özetler — tam sürümler payload'ı şişirmesin
/// (açıklama HTML'i ürün başına ortalama 3,7 KB).
#[derive(Debug, Serialize)]
pub struct MetaVersionMeta {
    pub at: String,
    /// Başlık, sürümü tanımanın en hızlı yolu.
    pub title: String,
    pub model: String,
}

#[derive(Debug, Serialize)]
pub struct DetailsVersionMeta {
    pub at: String,
    pub words: usize,
    pub model: String,
}

/// Saklanan bir teknik tablo sürümü (yeniden üretim öncesi anlık görüntü).
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct TechVersion {
    pub at: String,
    pub groups: Vec<gemini::TechGroup>,
    #[serde(default)]
    pub source: String,
}

/// UI listesi için hafif özet (tam sürümler payload'ı şişirmesin).
#[derive(Debug, Serialize)]
pub struct TechVersionMeta {
    pub at: String,
    pub rows: usize,
    pub groups: usize,
}

/// Önceki bir sürümü geri yükler. **Takas mantığı**: mevcut tablo geçmişin başına konur, seçilen
/// sürüm güncel olur → geri yükleme de kayıpsızdır (istenirse geri dönülebilir).
/// Eski bir meta sürümünü geri yükler.
///
/// **Takas:** geri yüklenen sürüm listeden çıkar, mevcut içerik (boş değilse) geçmişe girer —
/// böylece geri yükleme de geri alınabilir. `restore_tech_version` ile aynı semantik.
/// Hedef kelime de birlikte döner: o meta ona göre üretilmişti (kullanıcı kararı).
#[tauri::command]
pub fn restore_meta_version(
    state: State<'_, AppState>,
    sku: String,
    index: usize,
) -> Result<ProductDetail, String> {
    let conn = state.conn.lock().unwrap();
    let cur: (String, String, String, String, String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT COALESCE(draft_title,''), COALESCE(draft_descriptions,''),
                    COALESCE(draft_keywords,''), COALESCE(draft_search_keywords,''),
                    COALESCE(target_keyword,''), meta_model, meta_history_json
             FROM seo_status WHERE sku = ?1",
            [&sku],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
            other => format!("Meta okunamadı: {other}"),
        })?;

    let mut hist = history::parse::<MetaVersion>(cur.6.as_deref());
    if index >= hist.len() {
        return Err("Bu sürüm artık mevcut değil.".to_string());
    }
    let restored = hist.remove(index);

    if !cur.0.trim().is_empty() || !cur.1.trim().is_empty() {
        hist = history::push(
            hist,
            MetaVersion {
                at: now_str(),
                title: cur.0,
                descriptions: cur.1,
                keywords: cur.2,
                search_keywords: cur.3,
                target_keyword: cur.4,
                model: cur.5.unwrap_or_default(),
            },
        );
    }
    let hist_json = serde_json::to_string(&hist)
        .map_err(|e| format!("Meta geçmişi kaydedilemedi: {e}"))?;

    conn.execute(
        "UPDATE seo_status SET draft_title = ?2, draft_descriptions = ?3, draft_keywords = ?4,
                draft_search_keywords = ?5, target_keyword = ?6, meta_model = ?7,
                meta_history_json = ?8, updated_at = ?9
         WHERE sku = ?1",
        params![
            sku,
            restored.title,
            restored.descriptions,
            restored.keywords,
            restored.search_keywords,
            restored.target_keyword,
            restored.model,
            hist_json,
            now_str(),
        ],
    )
    .map_err(|e| format!("Meta geri yüklenemedi: {e}"))?;
    read_detail(&conn, &sku)
}

/// Eski bir açıklama sürümünü geri yükler (takas semantiği `restore_meta_version` ile aynı).
#[tauri::command]
pub fn restore_details_version(
    state: State<'_, AppState>,
    sku: String,
    index: usize,
) -> Result<ProductDetail, String> {
    let conn = state.conn.lock().unwrap();
    let (cur_html, cur_model, hist_json): (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT COALESCE(draft_details,''), details_model, details_history_json
             FROM seo_status WHERE sku = ?1",
            [&sku],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
            other => format!("Açıklama okunamadı: {other}"),
        })?;

    let mut hist = history::parse::<DetailsVersion>(hist_json.as_deref());
    if index >= hist.len() {
        return Err("Bu sürüm artık mevcut değil.".to_string());
    }
    let restored = hist.remove(index);

    if !cur_html.trim().is_empty() {
        hist = history::push(
            hist,
            DetailsVersion {
                at: now_str(),
                html: cur_html,
                model: cur_model.unwrap_or_default(),
            },
        );
    }
    let new_hist = serde_json::to_string(&hist)
        .map_err(|e| format!("Açıklama geçmişi kaydedilemedi: {e}"))?;

    conn.execute(
        "UPDATE seo_status SET draft_details = ?2, details_model = ?3,
                details_history_json = ?4, updated_at = ?5
         WHERE sku = ?1",
        params![sku, restored.html, restored.model, new_hist, now_str()],
    )
    .map_err(|e| format!("Açıklama geri yüklenemedi: {e}"))?;
    read_detail(&conn, &sku)
}

#[tauri::command]
pub fn restore_tech_version(
    state: State<'_, AppState>,
    sku: String,
    index: usize,
) -> Result<ProductDetail, String> {
    let conn = state.conn.lock().unwrap();
    let (cur_json, hist_json, cur_source): (Option<String>, Option<String>, String) = conn
        .query_row(
            "SELECT tech_specs_json, tech_history_json, COALESCE(tech_source_text,'')
             FROM seo_status WHERE sku = ?1",
            [&sku],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
            other => format!("Teknik tablo okunamadı: {other}"),
        })?;

    let mut hist = history::parse(hist_json.as_deref());
    if index >= hist.len() {
        return Err("Bu sürüm artık mevcut değil.".to_string());
    }
    let restored = hist.remove(index);

    // Mevcut tablo boş değilse geçmişe geri koy (takas)
    let cur_groups: Vec<gemini::TechGroup> = cur_json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();
    if !cur_groups.is_empty() {
        hist = history::push(
            hist,
            TechVersion { at: now_str(), groups: cur_groups, source: cur_source },
        );
    }

    let specs_json = serde_json::to_string(&restored.groups)
        .map_err(|e| format!("Serialize edilemedi: {e}"))?;
    let hist_out = serde_json::to_string(&hist).map_err(|e| format!("Serialize edilemedi: {e}"))?;
    conn.execute(
        "UPDATE seo_status SET tech_specs_json = ?2, tech_history_json = ?3,
                tech_source_text = ?4, updated_at = ?5
         WHERE sku = ?1",
        params![sku, specs_json, hist_out, restored.source, now_str()],
    )
    .map_err(|e| format!("Sürüm geri yüklenemedi: {e}"))?;
    read_detail(&conn, &sku)
}
