//! IdeaSoft entegrasyonu: bağlantı, gönderim, katalog senkronu, canonical yazma.
//!
//! ⚠️ Canonical akışı bilinçli olarak TOPLU DEĞİL: `apply_canonical` tek slug çifti alıyor.

use super::*;

#[derive(Debug, Serialize)]
pub struct IdeasoftPreview {
    pub id: i64,
    pub remote: ideasoft::RemoteProduct,
    /// Gönderilecek değerler (yalnızca seçilen parçalar).
    pub local: serde_json::Value,
}

/// Ayar + yerel içerikleri toplar; `parts` için gönderilecek `LocalContent` üretir.
fn ideasoft_local(conn: &Connection, sku: &str) -> Result<(String, String, ideasoft::LocalContent), String> {
    let domain = db::get_setting(conn, "ideasoft_domain")?.unwrap_or_default();
    let token = db::get_setting(conn, "ideasoft_token")?.unwrap_or_default();
    if domain.trim().is_empty() || token.trim().is_empty() {
        return Err("IdeaSoft bağlantısı ayarlı değil. Ayarlar'dan domain ve token girin.".to_string());
    }
    let d = read_detail(conn, sku)?;
    let tech_html = d
        .tech_specs
        .as_ref()
        .filter(|g| !g.is_empty())
        .map(|g| gemini::assemble_tech_html(g))
        .unwrap_or_default();
    let local = ideasoft::LocalContent {
        page_title: d.draft_title.clone().or(d.title.clone()).unwrap_or_default(),
        meta_description: d.draft_descriptions.clone().or(d.descriptions.clone()).unwrap_or_default(),
        // Üretilen anahtar kelimeler (draft) önceliklidir; yoksa feed'deki, o da yoksa arama kelimeleri
        // — böylece metaKeywords boş kalmaz (saha testi bulgusu).
        meta_keywords: d
            .draft_keywords
            .clone()
            .or(d.keywords.clone())
            .or(d.draft_search_keywords.clone())
            .unwrap_or_default(),
        search_keywords: d
            .draft_search_keywords
            .clone()
            .or(d.search_keywords.clone())
            .unwrap_or_default(),
        target_keyword: d.target_keyword.clone().unwrap_or_default(),
        details_html: d.draft_details.clone().or(d.details.clone()).unwrap_or_default(),
        tech_html,
    };
    Ok((domain, token, local))
}

/// sku → IdeaSoft id (önce cache, yoksa arama; bulununca cache'lenir).
async fn ideasoft_id_for(
    state: &State<'_, AppState>,
    sku: &str,
    domain: &str,
    token: &str,
) -> Result<i64, String> {
    let cached: Option<i64> = {
        let conn = state.conn.lock().unwrap();
        conn.query_row(
            "SELECT ideasoft_product_id FROM seo_status WHERE sku = ?1",
            [sku],
            |r| r.get(0),
        )
        .unwrap_or(None)
    };
    if let Some(id) = cached.filter(|v| *v > 0) {
        return Ok(id);
    }
    let r = ideasoft::resolve(domain, token, sku)
        .await?
        .ok_or_else(|| format!("Bu sku IdeaSoft'ta bulunamadı: {sku}"))?;
    {
        let conn = state.conn.lock().unwrap();
        ensure_seo_row(&conn, sku)?;
        conn.execute(
            "UPDATE seo_status SET ideasoft_product_id = ?2, ideasoft_seo_rule = ?3 WHERE sku = ?1",
            params![sku, r.id, r.seo_rule_count],
        )
        .map_err(|e| format!("IdeaSoft id kaydedilemedi: {e}"))?;
    }
    Ok(r.id)
}

#[tauri::command]
pub async fn test_ideasoft(state: State<'_, AppState>) -> Result<String, String> {
    let (domain, token) = {
        let conn = state.conn.lock().unwrap();
        (
            db::get_setting(&conn, "ideasoft_domain")?.unwrap_or_default(),
            db::get_setting(&conn, "ideasoft_token")?.unwrap_or_default(),
        )
    };
    ideasoft::test_connection(&domain, &token).await
}

/// Gönderim öncesi fark önizlemesi — uzaktaki mevcut değerler + gönderilecek gövde.
#[tauri::command]
pub async fn ideasoft_preview(
    state: State<'_, AppState>,
    sku: String,
    parts: Vec<String>,
) -> Result<IdeasoftPreview, String> {
    let (domain, token, local) = {
        let conn = state.conn.lock().unwrap();
        ideasoft_local(&conn, &sku)?
    };
    let id = ideasoft_id_for(&state, &sku, &domain, &token).await?;
    let remote = ideasoft::fetch_product(&domain, &token, id).await?;
    Ok(IdeasoftPreview { id, remote, local: ideasoft::build_payload(&parts, &local) })
}

/// IdeaSoft'taki hedef kelimeyi çeker ve yerel alana yazar (boş başlangıç sorununu çözer).
#[tauri::command]
pub async fn ideasoft_pull_keyword(
    state: State<'_, AppState>,
    sku: String,
) -> Result<ProductDetail, String> {
    let (domain, token) = {
        let conn = state.conn.lock().unwrap();
        let d = db::get_setting(&conn, "ideasoft_domain")?.unwrap_or_default();
        let t = db::get_setting(&conn, "ideasoft_token")?.unwrap_or_default();
        if d.trim().is_empty() || t.trim().is_empty() {
            return Err("IdeaSoft bağlantısı ayarlı değil.".to_string());
        }
        (d, t)
    };
    let id = ideasoft_id_for(&state, &sku, &domain, &token).await?;
    let remote = ideasoft::fetch_product(&domain, &token, id).await?;
    let kw = remote.target_keyword.trim().to_string();
    if kw.is_empty() {
        return Err("IdeaSoft'ta bu ürün için hedef kelime tanımlı değil.".to_string());
    }
    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    conn.execute(
        "UPDATE seo_status SET target_keyword = ?2, updated_at = ?3 WHERE sku = ?1",
        params![sku, kw, now_str()],
    )
    .map_err(|e| format!("Hedef kelime kaydedilemedi: {e}"))?;
    read_detail(&conn, &sku)
}

/// Seçilen parçaları IdeaSoft'a yazar. `parts` ∈ meta | keyword | details | tech.
#[tauri::command]
pub async fn ideasoft_push(
    state: State<'_, AppState>,
    sku: String,
    parts: Vec<String>,
) -> Result<ProductDetail, String> {
    let (domain, token, local) = {
        let conn = state.conn.lock().unwrap();
        ideasoft_local(&conn, &sku)?
    };
    let payload = ideasoft::build_payload(&parts, &local);
    if payload.as_object().map_or(true, |o| o.is_empty()) {
        return Err("Gönderilecek içerik yok — önce üretim yapın.".to_string());
    }
    let id = ideasoft_id_for(&state, &sku, &domain, &token).await?;
    // IdeaSoft `detail.details`'in null olmasına izin vermiyor → eksik alt alanları uzaktakiyle doldur
    // (dokunulmayan taraf aynen korunur).
    let mut payload = payload;
    if payload.get("detail").is_some() {
        let remote = ideasoft::fetch_product(&domain, &token, id).await?;
        ideasoft::fill_detail_from_remote(&mut payload, &remote);
    }
    ideasoft::push_product(&domain, &token, id, &payload).await?;

    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    conn.execute(
        "UPDATE seo_status SET ideasoft_pushed_at = ?2, updated_at = ?2 WHERE sku = ?1",
        params![sku, now_str()],
    )
    .map_err(|e| format!("Gönderim zamanı kaydedilemedi: {e}"))?;
    read_detail(&conn, &sku)
}

// ---- Faz 8: teknik özellik tablosu ----

/// En fazla saklanan önceki sürüm sayısı.

#[derive(Serialize)]
pub struct CatalogSyncResult {
    pub fetched: usize,
    pub synced_at: String,
    /// EOL listesindeki kaç sayfa katalogda eşleşti.
    pub matched_eol: usize,
}

/// IdeaSoft kataloğunu (tüm ürünler) çekip yerel tabloya yazar.
///
/// **Neden:** XML feed bilinçli olarak sınırlı — bu mağazada 10.909 üründen 262'si.
/// Feed dışı sayfalar Google'dan ciddi trafik alıyor (ölçüm: ürün trafiğinin %69'u) ama
/// uygulama onları hiç görmüyordu. Katalog bir kez çekilince EOL sayfalar slug ile
/// eşleştirilip durumları (aktif mi, stok var mı, canonical tanımlı mı) gösterilebiliyor.
///
/// ⚠️ Birkaç dakika sürer: ~110 istek, 40 istek/dk sınırına saygıyla. Elle tetiklenir.
#[tauri::command]
pub async fn sync_ideasoft_catalog(
    state: State<'_, AppState>,
) -> Result<CatalogSyncResult, String> {
    let (domain, token) = {
        let conn = state.conn.lock().unwrap();
        (
            db::get_setting(&conn, "ideasoft_domain")?.unwrap_or_default(),
            db::get_setting(&conn, "ideasoft_token")?.unwrap_or_default(),
        )
    };
    if domain.trim().is_empty() || token.trim().is_empty() {
        return Err("IdeaSoft bağlantısı kurulmamış. Ayarlar'dan alan adı ve token girin.".into());
    }

    let items = ideasoft::fetch_catalog(&domain, &token, 50_000, |_done, _total| {}).await?;
    let now = now_str();

    let mut conn = state.conn.lock().unwrap();
    let tx = conn
        .transaction()
        .map_err(|e| format!("İşlem başlatılamadı: {e}"))?;
    // Tam yenileme: silinen ürünler tabloda kalmasın.
    tx.execute("DELETE FROM ideasoft_catalog", [])
        .map_err(|e| format!("Katalog temizlenemedi: {e}"))?;
    {
        let mut stmt = tx
            .prepare(
                "INSERT OR REPLACE INTO ideasoft_catalog
                 (slug, id, name, status, stock, canonical, synced_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .map_err(|e| format!("Katalog yazılamadı: {e}"))?;
        for it in &items {
            if it.slug.is_empty() {
                continue;
            }
            let _ = stmt.execute(params![
                it.slug,
                it.id,
                it.name,
                it.status,
                it.stock,
                it.canonical,
                now
            ]);
        }
    }
    tx.commit().map_err(|e| format!("Katalog kaydedilemedi: {e}"))?;

    // Önbellekteki EOL listesiyle kaçı eşleşiyor?
    let matched_eol = match db::get_setting(&conn, "opportunity_json")? {
        Some(j) => match serde_json::from_str::<OpportunityReport>(&j) {
            Ok(r) => r
                .eol
                .iter()
                .filter(|e| {
                    conn.query_row(
                        "SELECT 1 FROM ideasoft_catalog WHERE slug = ?1",
                        [&e.slug.to_lowercase()],
                        |_| Ok(()),
                    )
                    .is_ok()
                })
                .count(),
            Err(_) => 0,
        },
        None => 0,
    };

    Ok(CatalogSyncResult {
        fetched: items.len(),
        synced_at: now,
        matched_eol,
    })
}

/// EOL sayfanın IdeaSoft'taki karşılığı (katalog senkronu yapılmışsa).
#[derive(Serialize)]
pub struct CatalogMatch {
    pub slug: String,
    pub id: i64,
    pub name: String,
    pub status: i64,
    pub stock: f64,
    pub canonical: String,
}

/// Verilen slug'ları yerel katalog tablosunda arar — ağ çağrısı YOK.
#[tauri::command]
pub fn lookup_catalog(
    state: State<'_, AppState>,
    slugs: Vec<String>,
) -> Result<Vec<CatalogMatch>, String> {
    let conn = state.conn.lock().unwrap();
    let mut out = Vec::new();
    for slug in slugs {
        let s = slug.to_lowercase();
        let row = conn.query_row(
            "SELECT slug, id, name, status, stock, COALESCE(canonical,'')
             FROM ideasoft_catalog WHERE slug = ?1",
            [&s],
            |r| {
                Ok(CatalogMatch {
                    slug: r.get(0)?,
                    id: r.get(1)?,
                    name: r.get(2)?,
                    status: r.get(3)?,
                    stock: r.get(4)?,
                    canonical: r.get(5)?,
                })
            },
        );
        if let Ok(m) = row {
            out.push(m);
        }
    }
    Ok(out)
}

/// IdeaSoft bağlantı ayarları — kilidi await'ten önce bırakmak için ayrı okunur.
fn ideasoft_creds(state: &State<'_, AppState>) -> Result<(String, String), String> {
    let conn = state.conn.lock().unwrap();
    let d = db::get_setting(&conn, "ideasoft_domain")?.unwrap_or_default();
    let t = db::get_setting(&conn, "ideasoft_token")?.unwrap_or_default();
    if d.trim().is_empty() || t.trim().is_empty() {
        return Err("IdeaSoft bağlantısı kurulmamış. Ayarlar'dan alan adı ve token girin.".into());
    }
    Ok((d, t))
}

/// Slug → (id, ad). Önce yerel katalog (ağsız, bedava), yoksa IdeaSoft aramasıyla tek satır çözer.
///
/// ⚠️ **Saha hatası (v0.6.4):** bu çözümleme eskiden YALNIZCA yerel `ideasoft_catalog` tablosuna
/// bakıyordu. Tablo boşken kullanıcı "Canonical ayarla"ya bastığında
/// *"Bu sayfa IdeaSoft kataloğunda bulunamadı, önce katalog senkronunu çalıştırın"* uyarısı
/// alıyordu — yani tek bir satırı yazmak için ~7 dakikalık tam senkron ön koşuldu. Ölçüm bunun
/// gereksiz olduğunu gösterdi: `resolve_slug` satır başına **1,44 istek** ile 25/25 çözüyor.
/// Senkron artık yalnızca bir hızlandırma; ön koşul değil.
async fn resolve_slug_cached(
    state: &State<'_, AppState>,
    domain: &str,
    token: &str,
    slug: &str,
) -> Result<(i64, String), String> {
    let key = slug.trim().trim_matches('/').to_lowercase();
    if key.is_empty() {
        return Err("Ürün adresi boş.".into());
    }
    // 1) Yerel katalog
    {
        let conn = state.conn.lock().unwrap();
        if let Ok(hit) = conn.query_row(
            "SELECT id, name FROM ideasoft_catalog WHERE slug = ?1",
            [&key],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
        ) {
            return Ok(hit);
        }
    }
    // 2) IdeaSoft araması
    let found = ideasoft::resolve_slug(domain, token, &key).await?.ok_or_else(|| {
        format!("\"{key}\" IdeaSoft'ta bulunamadı. Ürün silinmiş olabilir; bu durumda canonical yazılamaz.")
    })?;
    // Bir sonraki çağrı bedava olsun.
    {
        let conn = state.conn.lock().unwrap();
        let _ = conn.execute(
            "INSERT OR REPLACE INTO ideasoft_catalog
             (slug, id, name, status, stock, canonical, synced_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                found.slug,
                found.id,
                found.name,
                found.status,
                found.stock,
                found.canonical,
                now_str()
            ],
        );
    }
    Ok((found.id, found.name))
}

/// Serbest metinle ürün arar — canonical hedefini **elle** seçmek için.
///
/// Kullanıcı geri bildirimi (2026-07-30): *"Uygun halef bulunamasa bile kullanıcıya canonical
/// ayarla imkânı sunulmalı."* Yapay zekânın halef bulamaması hedefin olmadığı anlamına gelmiyor;
/// karar zaten operatörde.
#[tauri::command]
pub async fn search_catalog(
    state: State<'_, AppState>,
    term: String,
) -> Result<Vec<CatalogMatch>, String> {
    let (domain, token) = ideasoft_creds(&state)?;
    let items = ideasoft::search_products(&domain, &token, &term, 25).await?;
    Ok(items
        .into_iter()
        .map(|i| CatalogMatch {
            slug: i.slug,
            id: i.id,
            name: i.name,
            status: i.status,
            stock: i.stock,
            canonical: i.canonical,
        })
        .collect())
}

#[derive(Serialize)]
pub struct CanonicalPreview {
    pub product_id: i64,
    pub product_name: String,
    /// Canonical'ın işaret edeceği ürünün adı — kullanıcı doğru hedefi seçtiğini görsün.
    pub target_name: String,
    /// Şu an tanımlı canonical (boşsa yok).
    pub current: String,
    /// Yazılacak değer (`urun/<slug>` biçiminde).
    pub proposed: String,
    /// Kayıt yoksa oluşturulacak — kullanıcıya ne olacağı söylensin.
    pub will_create: bool,
}

/// Canonical yazmadan ÖNCE ne olacağını gösterir. Yazma yapmaz.
///
/// Faz 9'daki gönderim modalinin aynı deseni: önce fark, sonra onay.
#[tauri::command]
pub async fn preview_canonical(
    state: State<'_, AppState>,
    eol_slug: String,
    target_slug: String,
) -> Result<CanonicalPreview, String> {
    let (domain, token) = ideasoft_creds(&state)?;
    let (product_id, product_name) = resolve_slug_cached(&state, &domain, &token, &eol_slug).await?;
    // ⚠️ HEDEF de doğrulanır. Var olmayan bir sayfaya canonical yazmak, hiç yazmamaktan kötüdür:
    // Google'a "asıl sayfa şu" der ve o sayfa 404'tür. Bulunamazsa yazma adımına geçilmez.
    let (_, target_name) = resolve_slug_cached(&state, &domain, &token, &target_slug).await?;

    let cur = ideasoft::get_seo_setting(&domain, &token, product_id).await?;
    Ok(CanonicalPreview {
        product_id,
        product_name,
        target_name,
        current: cur.canonical,
        proposed: format!("urun/{}", target_slug.trim().trim_matches('/').to_lowercase()),
        will_create: cur.setting_id.is_none(),
    })
}

/// Canonical'ı yazar. **Canlı mağazayı değiştirir.**
///
/// ⚠️ Kullanıcı kararı: **toplu değil, gerektiğinde ve tek tek, açık onayla.** Bu komut
/// yalnızca `preview_canonical` gösterildikten ve kullanıcı onayladıktan sonra çağrılmalı.
/// Bilinçli olarak liste almıyor — imza toplu kullanımı zorlaştırıyor.
///
/// Canonical bir 301 DEĞİLDİR: ziyaretçi yine eski sayfaya düşer, yalnızca Google'a
/// "asıl sayfa şu" sinyali gider. Arayüz bunu söylemeli.
#[tauri::command]
pub async fn apply_canonical(
    state: State<'_, AppState>,
    eol_slug: String,
    target_slug: String,
) -> Result<String, String> {
    let (domain, token) = ideasoft_creds(&state)?;
    let (product_id, _) = resolve_slug_cached(&state, &domain, &token, &eol_slug).await?;
    // Önizlemede doğrulanmış olsa da burada tekrar bakılıyor: bu komut canlı mağazayı değiştiriyor
    // ve var olmayan bir hedefe canonical yazmamalı. Önizleme sonrası önbellekten geldiği için
    // ek istek yok.
    resolve_slug_cached(&state, &domain, &token, &target_slug).await?;

    // Mevcut kaydı oku: index/follow korunmalı, yalnızca canonical değişmeli.
    let cur = ideasoft::get_seo_setting(&domain, &token, product_id).await?;
    let target = format!("urun/{}", target_slug.trim().trim_matches('/').to_lowercase());
    ideasoft::set_canonical(&domain, &token, product_id, &target, &cur).await?;

    // Yerel katalogda da güncelle ki arayüz yeniden senkron beklemeden doğruyu göstersin.
    {
        let conn = state.conn.lock().unwrap();
        let _ = conn.execute(
            "UPDATE ideasoft_catalog SET canonical = ?2 WHERE slug = ?1",
            params![eol_slug.to_lowercase(), target],
        );
    }
    Ok(target)
}
