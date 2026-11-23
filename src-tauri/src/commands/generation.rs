//! Üretim komutları: meta, açıklama, teknik tablo, görsel kontrolü, SEO araştırması.
//!
//! Ortak önsöz (`ctx_parts`) üst modülde — üç üretim yolu da aynı bağlamı kullanıyor.

use super::*;

/// Faz 2: Gemini ile meta üretir, sonucu taslak alanlarına + hedef kelimeye yazar.
/// Not: SQLite kilidi await'lerin ötesine taşınmaz (Send güvenliği için bloklarda tutulur).
#[tauri::command]
pub async fn generate_meta(state: State<'_, AppState>, sku: String) -> Result<ProductDetail, String> {
    let (parts, target_keyword, zincir) = {
        let conn = state.conn.lock().unwrap();
        let parts = ctx_parts(&conn, &sku)?;
        let zincir = uretim_zinciri(&conn);
        let kw: String = conn
            .query_row(
                "SELECT COALESCE(target_keyword,'') FROM seo_status WHERE sku = ?1",
                [&sku],
                |r| r.get(0),
            )
            .unwrap_or_default();
        (parts, kw, zincir)
    };

    let ctx = parts.as_context(Some(&target_keyword), true);
    let toplayici = CallToplayici::default();
    let kanal = toplayici.kanal();
    let chain = gemini::ChainCtx { models: zincir, log: Some(&kanal) };
    let sonuc = gemini::generate_meta(&parts.key, &ctx, &chain).await;

    // ⚠️ Kayıt HATA DURUMUNDA DA yazılıyor: başarısız istekler de kotadan düşüyor ve
    // "limit darboğaz mı?" sorusunun cevabı zaten büyük ölçüde onlarda (429'lar).
    {
        let conn = state.conn.lock().unwrap();
        toplayici.yaz(&conn, "meta");
    }
    let produced = sonuc?;
    let (meta, model) = (produced.value, produced.model);

    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;

    // Yeniden üretimden ÖNCEKİ hâli geçmişe al — elle düzeltilmiş bir başlık geri dönüşsüz
    // kaybolmasın. Boşsa (ilk üretim) veya sonuç aynıysa kayıt açma.
    let history_json = snapshot_meta(&conn, &sku, &meta)?;

    conn.execute(
        "UPDATE seo_status SET target_keyword = ?2, draft_title = ?3, draft_descriptions = ?4,
                draft_keywords = ?5, draft_search_keywords = ?6, updated_at = ?7, meta_model = ?8,
                meta_history_json = COALESCE(?9, meta_history_json)
         WHERE sku = ?1",
        params![
            sku,
            meta.target_keyword.trim(),
            meta.title.trim(),
            meta.descriptions.trim(),
            meta.keywords.trim(),
            meta.search_keywords.trim(),
            now_str(),
            model,
            history_json,
        ],
    )
    .map_err(|e| format!("Üretilen meta kaydedilemedi: {e}"))?;
    read_detail(&conn, &sku)
}

#[derive(serde::Deserialize, Default)]
pub struct SharedProductMetaDraft {
    target_keyword: Option<String>,
    draft_title: Option<String>,
    draft_descriptions: Option<String>,
    draft_keywords: Option<String>,
    draft_search_keywords: Option<String>,
    meta_model: Option<String>,
    meta_history_json: Option<serde_json::Value>,
    research_json: Option<serde_json::Value>,
}

impl SharedProductMetaDraft {
    fn previous(&self) -> Option<MetaVersion> {
        let title = self.draft_title.as_deref().unwrap_or_default();
        let descriptions = self.draft_descriptions.as_deref().unwrap_or_default();
        let keywords = self.draft_keywords.as_deref().unwrap_or_default();
        let search_keywords = self.draft_search_keywords.as_deref().unwrap_or_default();
        if [title, descriptions, keywords, search_keywords].iter().all(|value| value.trim().is_empty()) {
            return None;
        }
        Some(MetaVersion {
            at: now_str(),
            title: title.into(),
            descriptions: descriptions.into(),
            keywords: keywords.into(),
            search_keywords: search_keywords.into(),
            target_keyword: self.target_keyword.clone().unwrap_or_default(),
            model: self.meta_model.clone().unwrap_or_default(),
        })
    }

    fn insights(&self) -> Option<SeoInsights> {
        match self.research_json.as_ref()? {
            serde_json::Value::String(json) => parse_insights(Some(json)),
            value => serde_json::from_value(value.clone()).ok(),
        }
    }

    fn take_history(&mut self) -> Vec<MetaVersion> {
        match self.meta_history_json.take() {
            Some(serde_json::Value::String(json)) => history::parse(Some(&json)),
            Some(value) => serde_json::from_value(value).unwrap_or_default(),
            None => Vec::new(),
        }
    }
}

#[tauri::command]
pub async fn generate_shared_product_meta(
    state: State<'_, AppState>,
    product: SharedProductGeminiContext,
    draft: SharedProductMetaDraft,
    session: SharedProductGeminiSession,
) -> Result<serde_json::Value, String> {
    let workspace = gemini::WorkspaceGemini::new(
        &session.project, session.workspace, session.publishable_key, session.access_token,
    )?;
    let insights = draft.insights();
    let context = gemini::ProductContext {
        name: &product.name,
        brand: product.brand.as_deref(),
        category: product.category.as_deref(),
        main_category: product.main_category.as_deref(),
        target_keyword: draft.target_keyword.as_deref(),
        insights: insights.as_ref(),
    };
    let models = uretim_zinciri(&state.conn.lock().unwrap());
    let calls = CallToplayici::default();
    let log = calls.kanal();
    let chain = gemini::ChainCtx { models, log: Some(&log) };
    let output = gemini::generate_team_meta(&workspace, &context, &chain).await;
    calls.yaz(&state.conn.lock().unwrap(), "meta");
    Ok(shared_generated_meta_changes(draft, output?))
}

#[derive(serde::Serialize)]
pub struct TeamGeminiMetaPreview {
    model: String,
    title: String,
    description: String,
}

#[tauri::command]
pub async fn preview_team_gemini_meta(
    state: State<'_, AppState>,
    session: SharedProductGeminiSession,
) -> Result<TeamGeminiMetaPreview, String> {
    let workspace = gemini::WorkspaceGemini::new(
        &session.project,
        session.workspace,
        session.publishable_key,
        session.access_token,
    )?;
    let context = gemini::ProductContext {
        name: "Kablo Düzenleyici",
        brand: None,
        category: Some("Masaüstü aksesuarları"),
        main_category: None,
        target_keyword: Some("kablo düzenleyici"),
        insights: None,
    };
    let models = uretim_zinciri(&state.conn.lock().unwrap());
    let chain = gemini::ChainCtx { models, log: None };
    let output = gemini::generate_team_meta(&workspace, &context, &chain).await?;
    Ok(TeamGeminiMetaPreview {
        model: output.model,
        title: output.value.title.trim().to_owned(),
        description: output.value.descriptions.trim().to_owned(),
    })
}

fn shared_generated_meta_changes(
    mut draft: SharedProductMetaDraft,
    output: gemini::Produced<gemini::GeneratedMeta>,
) -> serde_json::Value {
    let fresh = output.value;
    let mut changes = serde_json::json!({
        "draft_title": fresh.title.trim(),
        "draft_descriptions": fresh.descriptions.trim(),
        "draft_keywords": fresh.keywords.trim(),
        "draft_search_keywords": fresh.search_keywords.trim(),
        "target_keyword": fresh.target_keyword.trim(),
        "meta_model": output.model,
    });
    if let Some(previous) = draft.previous().filter(|old| {
        old.title.trim() != fresh.title.trim() || old.descriptions.trim() != fresh.descriptions.trim()
            || old.keywords.trim() != fresh.keywords.trim() || old.search_keywords.trim() != fresh.search_keywords.trim()
            || old.target_keyword.trim() != fresh.target_keyword.trim()
    }) {
        let history = history::push(draft.take_history(), previous);
        changes["meta_history_json"] = serde_json::json!(history);
    }
    changes
}

#[tauri::command]
pub fn restore_shared_product_meta(
    mut draft: SharedProductMetaDraft,
    index: usize,
) -> Result<serde_json::Value, String> {
    let mut history = draft.take_history();
    if index >= history.len() {
        return Err("Bu sürüm artık mevcut değil.".to_string());
    }
    let restored = history.remove(index);
    if let Some(previous) = draft.previous() {
        history = history::push(history, previous);
    }
    history.truncate(history::MAX);
    Ok(serde_json::json!({
        "draft_title": restored.title,
        "draft_descriptions": restored.descriptions,
        "draft_keywords": restored.keywords,
        "draft_search_keywords": restored.search_keywords,
        "target_keyword": restored.target_keyword,
        "meta_model": restored.model,
        "meta_history_json": history,
    }))
}

#[cfg(test)]
mod shared_meta_tests {
    use super::*;
    use serde_json::json;

    fn generated(title: &str, desc: &str, search: &str, kw: &str) -> gemini::Produced<gemini::GeneratedMeta> {
        gemini::Produced {
            value: gemini::GeneratedMeta {
                title: title.into(),
                descriptions: desc.into(),
                keywords: "meta kelimeleri".into(),
                search_keywords: search.into(),
                target_keyword: kw.into(),
            },
            model: "gemini-new".into(),
        }
    }

    #[test]
    fn shared_meta_keeps_the_previous_fields_and_restores_the_exact_version() {
        let archived = json!({
            "at": "2026-11-19", "title": "Arşiv", "descriptions": "Arşiv açıklaması",
            "keywords": "Arşiv", "search_keywords": "Eski arama", "target_keyword": "Eski hedef"
        });
        let draft = serde_json::from_value(json!({
            "draft_title": "Düzenlenmiş başlık", "draft_descriptions": "Düzenlenmiş açıklama",
            "draft_keywords": "meta kelimeleri", "draft_search_keywords": "eski arama",
            "target_keyword": "onaylı kelime", "meta_model": "gemini-old",
            "meta_history_json": vec![archived; 5]
        })).unwrap();
        let output = generated("Düzenlenmiş başlık", "Düzenlenmiş açıklama", "yeni arama", "onaylı kelime");
        let changes = shared_generated_meta_changes(draft, output);
        assert_eq!(changes["draft_search_keywords"], "yeni arama");
        let history = changes["meta_history_json"].as_array().unwrap();
        assert_eq!(history.len(), 5);
        assert_eq!(history[0]["search_keywords"], "eski arama");
        assert_eq!(history[0]["model"], "gemini-old");

        let restored = restore_shared_product_meta(serde_json::from_value(changes).unwrap(), 0).unwrap();
        assert_eq!(restored["draft_search_keywords"], "eski arama");
        assert_eq!(restored["target_keyword"], "onaylı kelime");
        assert_eq!(restored["meta_model"], "gemini-old");
        assert_eq!(restored["meta_history_json"][0]["model"], "gemini-new");

        let old = restore_shared_product_meta(serde_json::from_value(restored).unwrap(), 1).unwrap();
        assert_eq!(old["target_keyword"], "Eski hedef");
        assert_eq!(old["meta_model"], "");
    }

    #[test]
    fn unchanged_meta_and_first_generation_do_not_fill_version_history() {
        let output = generated(" Başlık ", " Açıklama ", " arama ", " hedef ");
        let first = shared_generated_meta_changes(SharedProductMetaDraft::default(), output);
        assert!(first.get("meta_history_json").is_none());
        assert_eq!(first["target_keyword"], "hedef");

        let output = generated("Başlık", "Açıklama", "arama", "hedef");
        let again = shared_generated_meta_changes(serde_json::from_value(first).unwrap(), output);
        assert!(again.get("meta_history_json").is_none());
    }
}

/// Faz 3: details HTML'ini yapıyı koruyarak yeniden üretir, taslağa yazar.
#[tauri::command]
pub async fn generate_details(
    state: State<'_, AppState>,
    sku: String,
) -> Result<ProductDetail, String> {
    // Ortak bağlam `ctx_parts`'tan; galeri ve mevcut açıklama yalnızca bu komuta özel.
    let (parts, details_html, keyword, gallery, zincir) = {
        let conn = state.conn.lock().unwrap();
        let parts = ctx_parts(&conn, &sku)?;
        let zincir = uretim_zinciri(&conn);
        let own = conn
            .query_row(
                "SELECT COALESCE(s.draft_details, p.details), COALESCE(s.target_keyword,''),
                        p.img_url, p.picture2, p.picture3, p.picture4
                 FROM products p LEFT JOIN seo_status s ON s.sku = p.sku
                 WHERE p.sku = ?1",
                [&sku],
                |r| {
                    let gallery: Vec<String> = [
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, Option<String>>(3)?,
                        r.get::<_, Option<String>>(4)?,
                        r.get::<_, Option<String>>(5)?,
                    ]
                    .into_iter()
                    .filter_map(|u| u.filter(|s| !s.trim().is_empty()))
                    .collect();
                    Ok((
                        r.get::<_, Option<String>>(0)?.unwrap_or_default(),
                        r.get::<_, String>(1)?,
                        gallery,
                    ))
                },
            )
            .map_err(|e| format!("Ürün okunamadı: {e}"))?;
        (parts, own.0, own.1, own.2, zincir)
    };

    require_description_gallery(&gallery)?;

    // `target_keyword: None` — açıklama akışı kelimeyi ayrı `keyword` argümanıyla alıyor.
    let ctx = parts.as_context(None, true);
    let api_key = &parts.key;
    let toplayici = CallToplayici::default();
    let kanal = toplayici.kanal();
    let chain = gemini::ChainCtx { models: zincir, log: Some(&kanal) };
    let generated = gemini::generate_product_details(api_key, &ctx, &details_html, &gallery, &keyword, &chain).await;

    {
        let conn = state.conn.lock().unwrap();
        toplayici.yaz(&conn, "details");
    }
    let gemini::Produced { value: new_html, model } = generated?;

    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    let history_json = snapshot_details(&conn, &sku, &new_html)?;
    conn.execute(
        "UPDATE seo_status SET draft_details = ?2, updated_at = ?3, details_model = ?4,
                details_history_json = COALESCE(?5, details_history_json)
         WHERE sku = ?1",
        params![sku, new_html, now_str(), model, history_json],
    )
    .map_err(|e| format!("Üretilen açıklama kaydedilemedi: {e}"))?;
    read_detail(&conn, &sku)
}

fn require_description_gallery(gallery: &[String]) -> Result<(), String> {
    if gallery.len() < 3 {
        return Err(format!(
            "En az 3 ürün görseli gerekli — şu an {}/4. Ürüne görsel ekleyin.",
            gallery.len()
        ));
    }
    Ok(())
}

#[derive(serde::Deserialize)]
pub struct SharedProductDetailsContent {
    html: Option<String>,
    gallery: Vec<String>,
}

#[derive(serde::Deserialize)]
pub struct SharedProductDetailsDraft {
    draft_details: Option<String>,
    details_status: Option<String>,
    target_keyword: Option<String>,
    details_model: Option<String>,
    details_history_json: Option<Vec<DetailsVersion>>,
}

impl SharedProductDetailsDraft {
    fn previous(&self) -> Option<DetailsVersion> {
        let html = self.draft_details.as_ref().filter(|html| !html.trim().is_empty())?;
        Some(DetailsVersion {
            at: now_str(),
            html: html.clone(),
            model: self.details_model.clone().unwrap_or_default(),
        })
    }
}

#[tauri::command]
pub async fn generate_shared_details(
    state: State<'_, AppState>,
    product: SharedProductGeminiContext,
    content: SharedProductDetailsContent,
    draft: SharedProductDetailsDraft,
    session: SharedProductGeminiSession,
) -> Result<serde_json::Value, String> {
    if draft.details_status.as_deref() == Some("done") {
        return Err("Bu ürünün açıklaması zaten tamamlandı işaretli.".into());
    }
    let gallery: Vec<_> = content.gallery.into_iter().filter(|url| !url.trim().is_empty()).collect();
    require_description_gallery(&gallery)?;
    let connection = gemini::WorkspaceGemini::new(
        &session.project, session.workspace, session.publishable_key, session.access_token,
    )?;
    let context = gemini::ProductContext {
        name: &product.name,
        brand: product.brand.as_deref(),
        category: product.category.as_deref(),
        main_category: product.main_category.as_deref(),
        target_keyword: None,
        insights: None,
    };
    let source = draft.draft_details.as_deref().or(content.html.as_deref()).unwrap_or_default();
    let keyword = draft.target_keyword.as_deref().unwrap_or_default();
    let models = uretim_zinciri(&state.conn.lock().unwrap());
    let calls = CallToplayici::default();
    let log = calls.kanal();
    let chain = gemini::ChainCtx { models, log: Some(&log) };
    let generated = gemini::generate_team_product_details(&connection, &context, source, &gallery, keyword, &chain).await;
    calls.yaz(&state.conn.lock().unwrap(), "details");
    Ok(generated_details_changes(draft, generated?))
}

fn generated_details_changes(
    mut draft: SharedProductDetailsDraft,
    generated: gemini::Produced<String>,
) -> serde_json::Value {
    let previous = draft.previous().filter(|old| old.html.trim() != generated.value.trim());
    let mut changes = serde_json::json!({
        "draft_details": generated.value,
        "details_model": generated.model,
    });
    if let Some(previous) = previous {
        let snapshots = history::push(draft.details_history_json.take().unwrap_or_default(), previous);
        changes["details_history_json"] = serde_json::json!(snapshots);
    }
    changes
}

#[tauri::command]
pub fn restore_shared_details_version(
    mut draft: SharedProductDetailsDraft,
    index: usize,
) -> Result<serde_json::Value, String> {
    let mut snapshots = draft.details_history_json.take().unwrap_or_default();
    if index >= snapshots.len() {
        return Err("Bu sürüm artık mevcut değil.".into());
    }
    let restored = snapshots.remove(index);
    if let Some(previous) = draft.previous() {
        snapshots = history::push(snapshots, previous);
    }
    snapshots.truncate(history::MAX);
    Ok(serde_json::json!({
        "draft_details": restored.html,
        "details_model": restored.model,
        "details_history_json": snapshots,
    }))
}

#[cfg(test)]
mod shared_details_tests {
    use super::*;
    use serde_json::json;

    fn generated(html: &str) -> gemini::Produced<String> {
        gemini::Produced { value: html.into(), model: "gemini-new".into() }
    }

    #[test]
    fn generation_snapshots_only_changed_drafts_and_keeps_five_versions() {
        let empty = serde_json::from_value(json!({})).unwrap();
        assert_eq!(generated_details_changes(empty, generated("<p>Yeni</p>")), json!({
            "draft_details": "<p>Yeni</p>", "details_model": "gemini-new",
        }));

        let current = json!({ "draft_details": " <p>Taslak</p> ", "details_model": "gemini-old",
            "details_history_json": (1..=5).map(|i| json!({"at": "2026-11-18 09:00", "html": i.to_string()})).collect::<Vec<_>>(),
        });
        let unchanged = generated_details_changes(serde_json::from_value(current.clone()).unwrap(), generated("<p>Taslak</p>"));
        assert!(unchanged.get("details_history_json").is_none());

        let changes = generated_details_changes(serde_json::from_value(current).unwrap(), generated("<p>Yeni</p>"));
        let versions = changes["details_history_json"].as_array().unwrap();
        assert_eq!(versions.len(), history::MAX);
        assert_eq!(versions[0]["html"], " <p>Taslak</p> ");
        assert_eq!(versions[0]["model"], "gemini-old");
        assert_eq!(versions[4]["html"], "4");
    }

    #[test]
    fn restoring_legacy_version_swaps_current_draft_without_changing_other_fields() {
        let draft = json!({ "draft_details": "<p>Güncel</p>", "details_model": "gemini-current",
            "details_status": "done", "target_keyword": "ürün",
            "details_history_json": [
                { "at": "2026-11-18 09:00", "html": "<p>İlk</p>" },
                { "at": "2026-11-17 09:00", "html": "<p>Önceki</p>", "model": null },
            ],
        });
        assert!(restore_shared_details_version(serde_json::from_value(draft.clone()).unwrap(), 2).is_err());
        let restored = restore_shared_details_version(serde_json::from_value(draft).unwrap(), 1).unwrap();
        assert_eq!(restored.as_object().unwrap().len(), 3);
        assert_eq!(restored["draft_details"], "<p>Önceki</p>");
        assert_eq!(restored["details_model"], "");
        assert_eq!(restored["details_history_json"][0]["html"], "<p>Güncel</p>");
        assert_eq!(restored["details_history_json"][0]["model"], "gemini-current");
        assert_eq!(restored["details_history_json"][1]["html"], "<p>İlk</p>");
    }
}

/// Faz 7: galeri görsellerinin 1:1 + çözünürlük kontrolü (async, `?revision` parmak iziyle cache'li).
#[tauri::command]
pub async fn check_images(state: State<'_, AppState>, sku: String) -> Result<Vec<ImageCheck>, String> {
    let (gallery, cached_json, cached_fp) = {
        let conn = state.conn.lock().unwrap();
        conn.query_row(
            "SELECT p.img_url, p.picture2, p.picture3, p.picture4, s.image_check_json, s.image_check_fp
             FROM products p LEFT JOIN seo_status s ON s.sku = p.sku
             WHERE p.sku = ?1",
            [&sku],
            |r| {
                let g: Vec<String> = [
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ]
                .into_iter()
                .filter_map(|u| u.filter(|s| !s.trim().is_empty()))
                .collect();
                Ok((g, r.get::<_, Option<String>>(4)?, r.get::<_, Option<String>>(5)?))
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
            other => format!("Ürün okunamadı: {other}"),
        })?
    };

    if gallery.is_empty() {
        return Ok(Vec::new());
    }
    let fp = gallery.join("|");
    // Görsel URL'leri (revision dahil) değişmemişse cache'i döndür
    if cached_fp.as_deref() == Some(fp.as_str()) {
        if let Some(cached) = cached_json.as_deref().and_then(|j| serde_json::from_str::<Vec<ImageCheck>>(j).ok())
        {
            return Ok(cached);
        }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;
    let checks = images::check_dimensions(&client, &gallery).await;
    let json = serde_json::to_string(&checks).unwrap_or_default();
    {
        let conn = state.conn.lock().unwrap();
        ensure_seo_row(&conn, &sku)?;
        conn.execute(
            "UPDATE seo_status SET image_check_json = ?2, image_check_fp = ?3, updated_at = ?4 WHERE sku = ?1",
            params![sku, json, fp, now_str()],
        )
        .map_err(|e| format!("Görsel kontrolü kaydedilemedi: {e}"))?;
    }
    Ok(checks)
}

// ---- Faz 9: IdeaSoft gönderim modülü (opsiyonel) ----

/// Kullanıcının yapıştırdığı ham teknik metni saklar (debounce'lu kayıt).
#[tauri::command]
pub fn save_tech_source(state: State<'_, AppState>, sku: String, text: String) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    conn.execute(
        "UPDATE seo_status SET tech_source_text = ?2, updated_at = ?3 WHERE sku = ?1",
        params![sku, text, now_str()],
    )
    .map_err(|e| format!("Teknik metin kaydedilemedi: {e}"))?;
    Ok(())
}

/// Ham metni gruplu spec'lere çevirir (kaynağa karşı doğrulanır) ve saklar.
#[tauri::command]
pub async fn structure_tech_specs(
    state: State<'_, AppState>,
    sku: String,
) -> Result<gemini::TechSpecsResult, String> {
    let (parts, source, prev_specs, prev_hist, zincir) = {
        let conn = state.conn.lock().unwrap();
        let parts = ctx_parts(&conn, &sku)?;
        let zincir = uretim_zinciri(&conn);
        let own = conn
            .query_row(
                "SELECT COALESCE(tech_source_text,''), tech_specs_json, tech_history_json
                 FROM seo_status WHERE sku = ?1",
                [&sku],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .unwrap_or_default();
        (parts, own.0, own.1, own.2, zincir)
    };

    // `with_insights: false` — teknik tablo pazarlama verisi değil; SEO araştırması
    // karıştırılırsa modelin olmayan özellik uydurma riski doğar.
    let ctx = parts.as_context(None, false);
    let toplayici = CallToplayici::default();
    let kanal = toplayici.kanal();
    let chain = gemini::ChainCtx { models: zincir, log: Some(&kanal) };
    let sonuc = gemini::structure_tech_specs(&parts.key, &ctx, &source, &chain).await;
    {
        let conn = state.conn.lock().unwrap();
        toplayici.yaz(&conn, "tech");
    }
    let produced = sonuc?;
    let (result, model) = (produced.value, produced.model);

    let json = serde_json::to_string(&result.groups)
        .map_err(|e| format!("Teknik tablo serialize edilemedi: {e}"))?;

    // Yeniden üretim: eski tabloyu kaybetmeden geçmişe al (bkz. core/src/history.rs).
    let old_groups: Vec<gemini::TechGroup> = prev_specs
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();
    let history_json = if old_groups.is_empty() {
        None
    } else {
        let hist = history::push(
            history::parse(prev_hist.as_deref()),
            TechVersion { at: now_str(), groups: old_groups, source: source.clone(), model: None },
        );
        serde_json::to_string(&hist).ok()
    };

    {
        let conn = state.conn.lock().unwrap();
        ensure_seo_row(&conn, &sku)?;
        match &history_json {
            Some(h) => conn.execute(
                "UPDATE seo_status SET tech_specs_json = ?2, tech_history_json = ?3, updated_at = ?4,
                        tech_model = ?5
                 WHERE sku = ?1",
                params![sku, json, h, now_str(), model],
            ),
            None => conn.execute(
                "UPDATE seo_status SET tech_specs_json = ?2, updated_at = ?3, tech_model = ?4
                 WHERE sku = ?1",
                params![sku, json, now_str(), model],
            ),
        }
        .map_err(|e| format!("Teknik tablo kaydedilemedi: {e}"))?;
    }
    Ok(result)
}

#[derive(serde::Deserialize)]
pub struct SharedProductTechDraft {
    tech_source_text: Option<String>,
    tech_table_source_text: Option<String>,
    tech_specs_json: Option<Vec<gemini::TechGroup>>,
    tech_history_json: Option<Vec<TechVersion>>,
    tech_model: Option<String>,
}

impl SharedProductTechDraft {
    fn previous(&self) -> Option<TechVersion> {
        let groups = self.tech_specs_json.as_ref().filter(|groups| !groups.is_empty())?;
        Some(TechVersion {
            at: now_str(),
            groups: groups.clone(),
            source: self.tech_table_source_text.clone()
                .or_else(|| self.tech_source_text.clone()).unwrap_or_default(),
            model: self.tech_model.clone(),
        })
    }
}

#[derive(serde::Deserialize)]
pub struct SharedProductGeminiContext {
    name: String,
    brand: Option<String>,
    category: Option<String>,
    main_category: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct SharedProductGeminiSession {
    project: String,
    workspace: String,
    publishable_key: String,
    access_token: String,
}

#[derive(Serialize)]
pub struct SharedProductTechOutput {
    result: gemini::TechSpecsResult,
    changes: serde_json::Value,
}

/// Ekip üretimi doğrulanmış taslak verisini işler. Sonuç normal ortak kayıt kuyruğuna gider.
#[tauri::command]
pub async fn structure_shared_tech_specs(
    state: State<'_, AppState>,
    product: SharedProductGeminiContext,
    draft: SharedProductTechDraft,
    session: SharedProductGeminiSession,
) -> Result<SharedProductTechOutput, String> {
    let connection = gemini::WorkspaceGemini::new(
        &session.project, session.workspace, session.publishable_key, session.access_token,
    )?;
    let context = gemini::ProductContext {
        name: &product.name,
        brand: product.brand.as_deref(),
        category: product.category.as_deref(),
        main_category: product.main_category.as_deref(),
        target_keyword: None,
        insights: None,
    };
    let models = uretim_zinciri(&state.conn.lock().unwrap());
    let calls = CallToplayici::default();
    let log = calls.kanal();
    let chain = gemini::ChainCtx { models, log: Some(&log) };
    let generated = gemini::structure_team_tech_specs(
        &connection, &context, draft.tech_source_text.as_deref().unwrap_or_default(), &chain,
    ).await;
    calls.yaz(&state.conn.lock().unwrap(), "tech");
    let produced = generated?;
    let changes = generated_tech_changes(draft, &produced.value.groups, produced.model);
    Ok(SharedProductTechOutput { result: produced.value, changes })
}

fn generated_tech_changes(
    mut draft: SharedProductTechDraft,
    groups: &[gemini::TechGroup],
    model: String,
) -> serde_json::Value {
    let mut changes = serde_json::json!({
        "tech_specs_json": groups,
        "tech_model": model,
        "tech_table_source_text": draft.tech_source_text.as_deref().unwrap_or_default(),
    });
    if let Some(previous) = draft.previous() {
        let snapshots = history::push(draft.tech_history_json.take().unwrap_or_default(), previous);
        changes["tech_history_json"] = serde_json::json!(snapshots);
    }
    changes
}

#[tauri::command]
pub fn restore_shared_tech_version(
    mut draft: SharedProductTechDraft,
    index: usize,
) -> Result<serde_json::Value, String> {
    let mut snapshots = draft.tech_history_json.take().unwrap_or_default();
    if index >= snapshots.len() {
        return Err("Bu sürüm artık mevcut değil.".into());
    }
    let restored = snapshots.remove(index);
    if let Some(previous) = draft.previous() {
        snapshots = history::push(snapshots, previous);
    }
    snapshots.truncate(history::MAX);
    Ok(serde_json::json!({
        "tech_specs_json": restored.groups,
        "tech_source_text": restored.source,
        "tech_table_source_text": restored.source,
        "tech_model": restored.model,
        "tech_history_json": snapshots,
    }))
}

#[tauri::command]
pub fn shared_tech_table_html(groups: Vec<gemini::TechGroup>) -> Result<String, String> {
    if groups.is_empty() {
        return Err("Önce teknik tabloyu yapılandırın.".into());
    }
    Ok(gemini::assemble_tech_html(&groups))
}

#[cfg(test)]
mod shared_tech_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn generation_and_restore_keep_the_source_and_model_of_each_table() {
        let old = json!([{"group": "Eski", "rows": [{"label": "Çözünürlük", "value": "10"}]}]);
        let new = json!([{"group": "Yeni", "rows": [{"label": "Çözünürlük", "value": "12"}]}]);
        let legacy = json!({"at": "2026-11-19", "groups": old, "source": "Arşiv kaynağı"});
        let current: SharedProductTechDraft = serde_json::from_value(json!({
            "tech_source_text": "Yeni kaynak",
            "tech_table_source_text": "Eski kaynak",
            "tech_model": "gemini-old",
            "tech_specs_json": old,
            "tech_history_json": vec![legacy; 5]
        })).unwrap();
        let groups = serde_json::from_value::<Vec<gemini::TechGroup>>(new.clone()).unwrap();
        let mut generated = generated_tech_changes(current, &groups, "gemini-new".into());
        assert_eq!(generated["tech_model"], "gemini-new");
        assert_eq!(generated["tech_specs_json"], new);
        assert_eq!(generated["tech_history_json"].as_array().unwrap().len(), 5);
        let previous = &generated["tech_history_json"][0];
        assert_eq!(previous["source"], "Eski kaynak");
        assert_eq!(previous["model"], "gemini-old");
        generated["tech_source_text"] = json!("Henüz tabloya uygulanmamış kaynak");

        let restored = restore_shared_tech_version(serde_json::from_value(generated).unwrap(), 0).unwrap();
        assert_eq!(restored["tech_source_text"], "Eski kaynak");
        assert_eq!(restored["tech_model"], "gemini-old");
        assert_eq!(restored["tech_specs_json"], old);
        assert_eq!(restored["tech_history_json"][0]["source"], "Yeni kaynak");
        assert_eq!(restored["tech_history_json"][0]["model"], "gemini-new");

        let archived = restore_shared_tech_version(serde_json::from_value(restored).unwrap(), 1).unwrap();
        assert_eq!(archived["tech_source_text"], "Arşiv kaynağı");
        assert!(archived["tech_model"].is_null());
    }
}

/// Kullanıcının elle düzenlediği tablo (doğruluk kaynağı kullanıcıdır).
#[tauri::command]
pub fn save_tech_specs(
    state: State<'_, AppState>,
    sku: String,
    specs: Vec<gemini::TechGroup>,
) -> Result<(), String> {
    let json = serde_json::to_string(&specs).map_err(|e| format!("Serialize edilemedi: {e}"))?;
    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    conn.execute(
        "UPDATE seo_status SET tech_specs_json = ?2, updated_at = ?3 WHERE sku = ?1",
        params![sku, json, now_str()],
    )
    .map_err(|e| format!("Teknik tablo kaydedilemedi: {e}"))?;
    Ok(())
}

/// IdeaSoft'a yapıştırılacak semantik HTML (deterministik, model devrede değil).
#[tauri::command]
pub fn tech_table_html(state: State<'_, AppState>, sku: String) -> Result<String, String> {
    let conn = state.conn.lock().unwrap();
    let json: Option<String> = conn
        .query_row("SELECT tech_specs_json FROM seo_status WHERE sku = ?1", [&sku], |r| r.get(0))
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
            other => format!("Teknik tablo okunamadı: {other}"),
        })?;
    let groups: Vec<gemini::TechGroup> = json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();
    if groups.is_empty() {
        return Err("Önce teknik tabloyu yapılandırın.".to_string());
    }
    Ok(gemini::assemble_tech_html(&groups))
}

#[tauri::command]
pub fn mark_tech_done(state: State<'_, AppState>, sku: String) -> Result<String, String> {
    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    let current: String = conn
        .query_row(
            "SELECT COALESCE(tech_status,'pending') FROM seo_status WHERE sku = ?1",
            [&sku],
            |r| r.get(0),
        )
        .map_err(|e| format!("Durum okunamadı: {e}"))?;
    let next = if current == "done" { "pending" } else { "done" };
    conn.execute(
        "UPDATE seo_status SET tech_status = ?2, updated_at = ?3 WHERE sku = ?1",
        params![sku, next, now_str()],
    )
    .map_err(|e| format!("Durum güncellenemedi: {e}"))?;
    if next == "done" {
        super::mark_reviewed(&conn, &sku)?;
        // Yerel işaretleme — ölçülmez, zaman çizelgesinde bağlam (bkz. metrics_cmd::log_event).
        super::log_event(&conn, &sku, "tech_done", false);
    }
    Ok(next.to_string())
}

/// Faz 4: Kontrollü SEO araştırması — Ahrefs (keyword ideas + difficulty).
/// Tohum kelime: verilen `seed` → yoksa onaylı hedef kelime → kategori → ürün adının ilk 4 sözcüğü.
/// Sonuç `seo_status.research_json`'a kaydedilir ve panele döner. GSC/Trends Faz 5/6'da eklenir.
#[tauri::command]
pub async fn research_seo(
    state: State<'_, AppState>,
    sku: String,
    seed: Option<String>,
) -> Result<SeoInsights, String> {
    let (name, category, url, target_kw, capsolver_key, country, gsc_json, gsc_site) = {
        let conn = state.conn.lock().unwrap();
        let capsolver_key = db::get_setting(&conn, "capsolver_api_key")?.unwrap_or_default();
        let country = db::get_setting(&conn, "seo_country")?.unwrap_or_else(|| "tr".to_string());
        let gsc_json = db::get_setting(&conn, "gsc_service_account_json")?.unwrap_or_default();
        let gsc_site = db::get_setting(&conn, "gsc_site_url")?.unwrap_or_default();
        let (name, category, url, target_kw) = conn
            .query_row(
                "SELECT p.name, p.category, p.url, COALESCE(s.target_keyword,'')
                 FROM products p LEFT JOIN seo_status s ON s.sku = p.sku
                 WHERE p.sku = ?1",
                [&sku],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
                other => format!("Ürün okunamadı: {other}"),
            })?;
        (name, category, url, target_kw, capsolver_key, country, gsc_json, gsc_site)
    };

    let has_capsolver = !capsolver_key.trim().is_empty();
    let has_gsc = !gsc_json.trim().is_empty() && !gsc_site.trim().is_empty();
    if !has_capsolver && !has_gsc {
        return Err(
            "Araştırma için Ayarlar'dan CapSolver anahtarı ve/veya GSC service-account + mülk ekleyin."
                .to_string(),
        );
    }

    // Tohum kelime seçimi (kontrollü: kullanıcı panelde düzenleyebilir)
    let seed = seed
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            let t = target_kw.trim();
            if t.is_empty() { None } else { Some(t.to_string()) }
        })
        .or_else(|| category.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(String::from))
        .unwrap_or_else(|| first_words(&name, 4));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .cookie_store(true) // Google Trends explore için NID çerezi gerekir
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;

    let mut ins = SeoInsights {
        seed: seed.clone(),
        fetched_at: now_str(),
        ..Default::default()
    };

    let domain = url.as_deref().and_then(host_of);

    // Ahrefs (CapSolver varsa): keyword ideas + difficulty + domain overview — hepsi eşzamanlı.
    if has_capsolver {
        let overview_fut = async {
            match &domain {
                Some(d) => Some(seo_data::ahrefs::backlinks_overview(&client, &capsolver_key, d).await),
                None => None,
            }
        };
        let (ideas_res, kd_res, ov_res) = tokio::join!(
            seo_data::ahrefs::keyword_ideas(&client, &capsolver_key, &seed, &country),
            seo_data::ahrefs::keyword_difficulty(&client, &capsolver_key, &seed, &country),
            overview_fut,
        );
        match ideas_res {
            Ok(mut cands) => {
                cands.sort_by(|a, b| b.volume.cmp(&a.volume));
                ins.target_candidates = cands;
            }
            Err(e) => ins.notes.push(format!("Anahtar kelime fikirleri alınamadı: {e}")),
        }
        match kd_res {
            Ok(d) => ins.seed_difficulty = Some(d),
            Err(e) => ins.notes.push(format!("Zorluk verisi alınamadı: {e}")),
        }
        if let Some(ov) = ov_res {
            match ov {
                Ok(d) => ins.domain = Some(d),
                Err(e) => ins.notes.push(format!("Alan (backlink) özeti alınamadı: {e}")),
            }
        }
    }

    // Google Trends — hedef kelimeye ilgili sorgular (explore→relatedsearches) DEVRE DIŞI:
    // Google'ın anti-bot koruması API'yi HTTP 429 ile blokluyor (tarayıcı consent çerezi gerekiyor).
    // Kod `seo_data::trends`'te korunuyor; keyword-relevant ihtiyaç Ahrefs fikirleri + GSC sorgularıyla
    // zaten karşılanıyor. İleride güvenilir bir yol bulunursa yeniden etkinleştirilebilir.

    // GSC gerçek sorgular (SA + mülk varsa ve üründe URL varsa).
    if has_gsc {
        match url.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
            Some(page) => {
                match seo_data::gsc::search_queries(&client, &gsc_json, gsc_site.trim(), page, 90, 25)
                    .await
                {
                    Ok(q) => ins.gsc_queries = q,
                    Err(e) => ins.notes.push(format!("GSC sorguları alınamadı: {e}")),
                }
            }
            None => ins.notes.push("Bu üründe URL yok, GSC sorguları atlandı.".to_string()),
        }
    }

    if !ins.has_data() {
        let detail = ins.notes.join(" ");
        return Err(if detail.is_empty() {
            "Araştırma verisi alınamadı.".to_string()
        } else {
            detail
        });
    }

    // Sonucu kaydet (üretim prompt'ları buradan okur)
    let json = serde_json::to_string(&ins).map_err(|e| format!("Araştırma serialize edilemedi: {e}"))?;
    {
        let conn = state.conn.lock().unwrap();
        ensure_seo_row(&conn, &sku)?;
        conn.execute(
            "UPDATE seo_status SET research_json = ?2, updated_at = ?3 WHERE sku = ?1",
            params![sku, json, now_str()],
        )
        .map_err(|e| format!("Araştırma kaydedilemedi: {e}"))?;
    }
    Ok(ins)
}

// ---- Fırsat analizi: GSC verisiyle "önce hangi ürüne bakmalıyım?" ----
