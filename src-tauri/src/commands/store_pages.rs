//! Mağaza sayfası (kategori · marka · blog) meta ve tanıtım metni akışı — Faz İ4.
//!
//! Ürün akışının aynı iskeleti: **üret → gözden geçir → gönder**, üçü ayrı adım ve arada
//! operatör var. Toplu üretim yok (kullanıcı kısıtı: *"halüsinasyon riskini göze alamam"*).
//!
//! 🔴 Buradaki çıktı ürün meta'sından **daha az doğrulanabilir** — kategori tanıtım metninin
//! karşılaştırılacağı bir kaynak yok (bkz. `gemini::store_page` modül başlığı). Bu yüzden
//! gönderim öncesi farkı göstermek burada ürün akışındakinden daha da önemli.

use super::*;

/// Ekrana giden mağaza sayfası: mağazadaki hâli + üretilmiş taslak + ölçüm.
#[derive(Serialize)]
pub struct StorePageDetail {
    pub kind: String,
    pub remote_id: i64,
    pub slug: String,
    pub name: String,
    // Mağazadaki mevcut hâl
    pub page_title: String,
    pub meta_description: String,
    pub target_keyword: String,
    pub showcase_content: Option<String>,
    // Üretilmiş taslak (yoksa boş)
    pub draft_page_title: String,
    pub draft_meta_description: String,
    pub draft_target_keyword: String,
    pub draft_showcase: String,
    pub draft_model: String,
    pub draft_at: String,
    pub pushed_at: String,
    /// Son analizde bu sayfanın aldığı sorgular — üretimin bağlamı ve ekranda gerekçe.
    pub queries: Vec<String>,
    pub impressions: f64,
    pub clicks: f64,
}

/// Bir mağaza sayfasının GSC'de aldığı sorgular (en çok gösterim alanlar önce).
///
/// ⚠️ `query_rows` yalnızca son çekimi tutuyor; envanter senkronu değil **analiz**
/// doldurduğu için analiz koşulmadıysa liste boş gelir. Boş bağlamla üretim yine çalışır
/// ama zayıf olur — ekran bunu söylemeli.
fn queries_of(conn: &Connection, slug: &str, limit: usize) -> (Vec<String>, f64, f64) {
    let desen = format!("%/{}", slug.to_lowercase());
    let mut st = match conn.prepare(
        "SELECT query, impressions, clicks FROM query_rows
         WHERE lower(page) LIKE ?1 OR lower(page) LIKE ?1 || '/'
         ORDER BY impressions DESC LIMIT ?2",
    ) {
        Ok(s) => s,
        Err(_) => return (Vec::new(), 0.0, 0.0),
    };
    let rows: Vec<(String, f64, f64)> = st
        .query_map(params![desen, limit as i64], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .map(|it| it.filter_map(|x| x.ok()).collect())
        .unwrap_or_default();
    let gos = rows.iter().map(|r| r.1).sum();
    let tik = rows.iter().map(|r| r.2).sum();
    (rows.into_iter().map(|r| r.0).collect(), gos, tik)
}

/// Kategorideki/markadaki gerçek ürün adları — üretimin ikinci bağlam kaynağı.
///
/// 🔴 Uydurmaya karşı en güçlü koruma bu: model "bu sayfada neler var" sorusunu tahmin
/// etmiyor, katalogdan okuyor. Blogda karşılığı yok (bir yazı ürün kümesi değil) → boş.
fn products_of(conn: &Connection, kind: &str, name: &str, limit: usize) -> Vec<String> {
    let (sql, p) = match kind {
        "category" => (
            "SELECT name FROM products WHERE lower(COALESCE(category,'')) = lower(?1)
                OR lower(COALESCE(main_category,'')) = lower(?1) LIMIT ?2",
            name,
        ),
        "brand" => (
            "SELECT name FROM products WHERE lower(COALESCE(brand,'')) = lower(?1) LIMIT ?2",
            name,
        ),
        _ => return Vec::new(),
    };
    conn.prepare(sql)
        .and_then(|mut st| {
            st.query_map(params![p, limit as i64], |r| r.get::<_, String>(0))
                .map(|it| it.filter_map(|x| x.ok()).collect())
        })
        .unwrap_or_default()
}

fn read_detail(conn: &Connection, kind: &str, id: i64) -> Result<StorePageDetail, String> {
    let (slug, name, pt, md, tk, sc, dpt, dmd, dtk, dsc, dm, da, pa): (
        String, String, String, String, String, Option<String>,
        Option<String>, Option<String>, Option<String>, Option<String>,
        Option<String>, Option<String>, Option<String>,
    ) = conn
        .query_row(
            "SELECT slug, name, page_title, meta_description, target_keyword, showcase_content,
                    draft_page_title, draft_meta_description, draft_target_keyword,
                    draft_showcase, draft_model, draft_at, pushed_at
             FROM store_pages WHERE kind = ?1 AND remote_id = ?2",
            params![kind, id],
            |r| {
                Ok((
                    r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?,
                    r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?, r.get(11)?, r.get(12)?,
                ))
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                "Sayfa envanterde yok. Ayarlar'dan envanteri çekin.".to_string()
            }
            other => format!("Sayfa okunamadı: {other}"),
        })?;

    let (queries, impressions, clicks) = queries_of(conn, &slug, 12);
    Ok(StorePageDetail {
        kind: kind.to_string(),
        remote_id: id,
        slug,
        name,
        page_title: pt,
        meta_description: md,
        target_keyword: tk,
        showcase_content: sc,
        draft_page_title: dpt.unwrap_or_default(),
        draft_meta_description: dmd.unwrap_or_default(),
        draft_target_keyword: dtk.unwrap_or_default(),
        draft_showcase: dsc.unwrap_or_default(),
        draft_model: dm.unwrap_or_default(),
        draft_at: da.unwrap_or_default(),
        pushed_at: pa.unwrap_or_default(),
        queries,
        impressions,
        clicks,
    })
}

#[tauri::command]
pub fn get_store_page(
    state: State<'_, AppState>,
    kind: String,
    id: i64,
) -> Result<StorePageDetail, String> {
    let conn = state.conn.lock().unwrap();
    read_detail(&conn, &kind, id)
}

/// Eksik alanı olan ya da GSC'de görünen sayfaları listeler — çalışma listesi.
///
/// ⚠️ Sıralama **gösterime göre**: görünmeyen bir sayfanın meta'sını düzeltmek ölçülebilir
/// sonuç üretmiyor (ölçüldü: 265 marka kaydının 75'i görünüyor). En üstte iş yapacak olan.
#[tauri::command]
pub fn list_store_pages(
    state: State<'_, AppState>,
    kind: String,
) -> Result<Vec<StorePageDetail>, String> {
    let conn = state.conn.lock().unwrap();
    let ids: Vec<i64> = conn
        .prepare("SELECT remote_id FROM store_pages WHERE kind = ?1 AND status = 1")
        .and_then(|mut st| {
            st.query_map([&kind], |r| r.get::<_, i64>(0))
                .map(|it| it.filter_map(|x| x.ok()).collect())
        })
        .unwrap_or_default();
    let mut out: Vec<StorePageDetail> = ids
        .into_iter()
        .filter_map(|id| read_detail(&conn, &kind, id).ok())
        .collect();
    out.sort_by(|a, b| {
        b.impressions.partial_cmp(&a.impressions).unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(out)
}

/// Meta + tanıtım metni üretir ve **taslağa** yazar. Mağazaya dokunmaz.
#[tauri::command]
pub async fn generate_store_page(
    state: State<'_, AppState>,
    kind: String,
    id: i64,
) -> Result<StorePageDetail, String> {
    let (key, zincir, name, existing, queries, products) = {
        let conn = state.conn.lock().unwrap();
        let d = read_detail(&conn, &kind, id)?;
        (
            db::get_setting(&conn, "gemini_api_key")?.unwrap_or_default(),
            uretim_zinciri(&conn),
            d.name.clone(),
            d.showcase_content.clone(),
            d.queries.clone(),
            products_of(&conn, &kind, &d.name, 8),
        )
    };

    let ctx = gemini::StorePageContext {
        kind: &kind,
        name: &name,
        queries: &queries,
        products: &products,
        existing: existing.as_deref(),
    };
    let toplayici = CallToplayici::default();
    let kanal = toplayici.kanal();
    let chain = gemini::ChainCtx { models: zincir, log: Some(&kanal) };
    let sonuc = gemini::generate_store_page(&key, &ctx, &chain).await;

    let conn = state.conn.lock().unwrap();
    toplayici.yaz(&conn, "store_page");
    let produced = sonuc?;
    let v = produced.value;
    conn.execute(
        "UPDATE store_pages SET draft_page_title = ?3, draft_meta_description = ?4,
                draft_target_keyword = ?5, draft_showcase = ?6, draft_model = ?7, draft_at = ?8
         WHERE kind = ?1 AND remote_id = ?2",
        params![
            kind, id, v.page_title, v.meta_description, v.target_keyword, v.showcase,
            produced.model, now_str()
        ],
    )
    .map_err(|e| format!("Taslak kaydedilemedi: {e}"))?;
    read_detail(&conn, &kind, id)
}

/// Taslağı elle düzenler — operatör metni değiştirebilmeli.
#[tauri::command]
pub fn save_store_page_draft(
    state: State<'_, AppState>,
    kind: String,
    id: i64,
    page_title: String,
    meta_description: String,
    target_keyword: String,
    showcase: String,
) -> Result<StorePageDetail, String> {
    let conn = state.conn.lock().unwrap();
    conn.execute(
        "UPDATE store_pages SET draft_page_title = ?3, draft_meta_description = ?4,
                draft_target_keyword = ?5, draft_showcase = ?6, draft_at = ?7
         WHERE kind = ?1 AND remote_id = ?2",
        params![kind, id, page_title, meta_description, target_keyword, showcase, now_str()],
    )
    .map_err(|e| format!("Taslak kaydedilemedi: {e}"))?;
    read_detail(&conn, &kind, id)
}

/// Taslağı IdeaSoft'a gönderir.
///
/// ⚠️ **Kısmi PUT**, ürün gönderimindeki desenin aynısı: yalnızca dolu alanlar gidiyor.
/// Canlı doğrulandı (2026-08-12, taslak bir blog kaydında): PUT birleştirme mantığında,
/// gönderilmeyen alanlara dokunmuyor.
///
/// 🔴 Olay `reaches_store = 1` ile yazılıyor — merkezî kural: yalnızca mağazaya ulaşan iş
/// ölçülüyor. Taslak üretmek tek başına bir sonuç üretmez.
#[tauri::command]
pub async fn push_store_page(
    state: State<'_, AppState>,
    kind: String,
    id: i64,
) -> Result<StorePageDetail, String> {
    let (domain, token, d) = {
        let conn = state.conn.lock().unwrap();
        (
            db::get_setting(&conn, "ideasoft_domain")?.unwrap_or_default(),
            db::get_setting(&conn, "ideasoft_token")?.unwrap_or_default(),
            read_detail(&conn, &kind, id)?,
        )
    };
    if domain.trim().is_empty() || token.trim().is_empty() {
        return Err("IdeaSoft bağlantısı kurulmamış. Ayarlar'dan alan adı ve token girin.".into());
    }
    if d.draft_page_title.trim().is_empty() && d.draft_showcase.trim().is_empty() {
        return Err("Gönderilecek taslak yok — önce üretin.".into());
    }

    let uc = match kind.as_str() {
        "category" => "categories",
        "brand" => "brands",
        "blog" => "blogs",
        _ => return Err(format!("Bilinmeyen sayfa tipi: {kind}")),
    };
    // ⚠️ `showcaseContent` blogda YOK; göndermek 400 üretebilir. Tip başına alan seti.
    let showcase_alani = kind != "blog";
    let url = ideasoft::store_page_url(&domain, uc, id)?;
    ideasoft::put_store_page(
        &url,
        &token,
        &d.draft_page_title,
        &d.draft_meta_description,
        &d.draft_target_keyword,
        if showcase_alani { Some(&d.draft_showcase) } else { None },
    )
    .await?;

    let now = now_str();
    let conn = state.conn.lock().unwrap();
    conn.execute(
        "UPDATE store_pages SET page_title = COALESCE(NULLIF(?3,''), page_title),
                meta_description = COALESCE(NULLIF(?4,''), meta_description),
                target_keyword = COALESCE(NULLIF(?5,''), target_keyword),
                showcase_content = CASE WHEN ?6 <> '' THEN ?6 ELSE showcase_content END,
                pushed_at = ?7
         WHERE kind = ?1 AND remote_id = ?2",
        params![
            kind, id, d.draft_page_title, d.draft_meta_description, d.draft_target_keyword,
            if showcase_alani { d.draft_showcase.clone() } else { String::new() },
            now
        ],
    )
    .map_err(|e| format!("Yerel kayıt güncellenemedi: {e}"))?;

    // Ölçüm omurgasına yazılıyor: sonuç rozetleri ve "işe yaradı mı?" bunu okuyor.
    let _ = conn.execute(
        "INSERT INTO work_events (at, sku, url, kind, reaches_store, payload_json)
         VALUES (?1, NULL, ?2, 'store_page_push', 1, ?3)",
        params![
            now,
            d.slug,
            serde_json::json!({ "kind": kind, "id": id, "model": d.draft_model }).to_string()
        ],
    );
    read_detail(&conn, &kind, id)
}
