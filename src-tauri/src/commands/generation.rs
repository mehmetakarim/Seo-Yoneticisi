//! Üretim komutları: meta, açıklama, teknik tablo, görsel kontrolü, SEO araştırması.
//!
//! Ortak önsöz (`ctx_parts`) üst modülde — üç üretim yolu da aynı bağlamı kullanıyor.

use super::*;

/// Faz 2: Gemini ile meta üretir, sonucu taslak alanlarına + hedef kelimeye yazar.
/// Not: SQLite kilidi await'lerin ötesine taşınmaz (Send güvenliği için bloklarda tutulur).
#[tauri::command]
pub async fn generate_meta(state: State<'_, AppState>, sku: String) -> Result<ProductDetail, String> {
    let (parts, target_keyword) = {
        let conn = state.conn.lock().unwrap();
        let parts = ctx_parts(&conn, &sku)?;
        let kw: String = conn
            .query_row(
                "SELECT COALESCE(target_keyword,'') FROM seo_status WHERE sku = ?1",
                [&sku],
                |r| r.get(0),
            )
            .unwrap_or_default();
        (parts, kw)
    };

    let ctx = parts.as_context(Some(&target_keyword), true);
    let produced = gemini::generate_meta(&parts.key, &ctx).await?;
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

/// Faz 3: details HTML'ini yapıyı koruyarak yeniden üretir, taslağa yazar.
#[tauri::command]
pub async fn generate_details(
    state: State<'_, AppState>,
    sku: String,
) -> Result<ProductDetail, String> {
    // Ortak bağlam `ctx_parts`'tan; galeri ve mevcut açıklama yalnızca bu komuta özel.
    let (parts, details_html, keyword, gallery) = {
        let conn = state.conn.lock().unwrap();
        let parts = ctx_parts(&conn, &sku)?;
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
        (parts, own.0, own.1, own.2)
    };

    // Görsel kapısı: en az 3 galeri görseli (backend savunma; UI de engeller).
    if gallery.len() < 3 {
        return Err(format!(
            "En az 3 ürün görseli gerekli — şu an {}/4. Ürüne görsel ekleyin.",
            gallery.len()
        ));
    }

    // `target_keyword: None` — açıklama akışı kelimeyi ayrı `keyword` argümanıyla alıyor.
    let ctx = parts.as_context(None, true);
    let api_key = &parts.key;
    // Açıklama akışı:
    //  1) İçerik yok / yeniden yazılabilir metin yok → sıfırdan semantik HTML (galeri görselleri).
    //  2) Düzenli yapı → OPTIMIZE: metin iyileştirilir + yapı semantikleştirilir + anlamlı alt eklenir.
    //  3) Düzensiz yapı → eski güvenli yol (yapıyı aynen koruyarak yalnızca metni yeniden yaz).
    let (new_html, model) = if details_html.trim().is_empty()
        || !gemini::has_rewritable_content(&details_html)
    {
        let p = gemini::generate_details_scratch(api_key, &ctx, &gallery, &keyword).await?;
        (p.value, p.model)
    } else {
        let opt = gemini::optimize_details(api_key, &ctx, &details_html, &keyword).await?;
        match opt.value {
            Some(html) => (html, opt.model),
            // Yapı beklenmedik → yapı-koruyan eski yol. Modeli o çağrıdan al.
            None => {
                let p = gemini::generate_details(api_key, &ctx, &details_html, &keyword).await?;
                (p.value, p.model)
            }
        }
    };

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
    let (parts, source, prev_specs, prev_hist) = {
        let conn = state.conn.lock().unwrap();
        let parts = ctx_parts(&conn, &sku)?;
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
        (parts, own.0, own.1, own.2)
    };

    // `with_insights: false` — teknik tablo pazarlama verisi değil; SEO araştırması
    // karıştırılırsa modelin olmayan özellik uydurma riski doğar.
    let ctx = parts.as_context(None, false);
    let produced = gemini::structure_tech_specs(&parts.key, &ctx, &source).await?;
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
            TechVersion { at: now_str(), groups: old_groups, source: source.clone() },
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
