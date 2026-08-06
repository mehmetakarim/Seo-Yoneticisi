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
) -> Result<IdeasoftPull, String> {
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
    // Teknik tablo IdeaSoft'un "Teknik Özellikler" sekmesinde (`extraDetails`) duruyor ve
    // XML feed'de YOK — uygulamanın bu veriyi görebildiği tek yol bu çağrı.
    let tech_text = ideasoft::tech_html_to_text(&remote.extra_details);

    let kw = remote.target_keyword.trim().to_string();
    if kw.is_empty() && tech_text.is_empty() {
        return Err(
            "IdeaSoft'ta bu ürün için hedef kelime ve teknik tablo tanımlı değil.".to_string()
        );
    }

    let conn = state.conn.lock().unwrap();
    ensure_seo_row(&conn, &sku)?;
    if !kw.is_empty() {
        conn.execute(
            "UPDATE seo_status SET target_keyword = ?2, updated_at = ?3 WHERE sku = ?1",
            params![sku, kw, now_str()],
        )
        .map_err(|e| format!("Hedef kelime kaydedilemedi: {e}"))?;
    }
    // ⚠️ Dolu kaynak metnin ÜZERİNE YAZILMAZ: orası kullanıcının elle yapıştırdığı ham veri
    // olabilir ve geri alınamaz. Boşsa dolduruluyor, doluysa korunuyor — hangisi olduğu
    // kullanıcıya AÇIKÇA söyleniyor, yoksa "getirdim" deyip getirmemiş oluruz.
    let mut tech_durum = "";
    if !tech_text.is_empty() {
        let yazilan = conn
            .execute(
                "UPDATE seo_status SET tech_source_text = ?2, updated_at = ?3
                 WHERE sku = ?1 AND COALESCE(TRIM(tech_source_text), '') = ''",
                params![sku, tech_text, now_str()],
            )
            .map_err(|e| format!("Teknik tablo kaydedilemedi: {e}"))?;
        tech_durum = if yazilan > 0 { "yazildi" } else { "korundu" };
    }

    let message = match (kw.is_empty(), tech_durum) {
        (false, "yazildi") => format!("Hedef kelime ve teknik tablo getirildi: \"{kw}\""),
        (false, "korundu") => format!(
            "Hedef kelime getirildi: \"{kw}\" · teknik tablo alanı zaten dolu olduğu için değiştirilmedi"
        ),
        (false, _) => format!("Hedef kelime getirildi: \"{kw}\" · IdeaSoft'ta teknik tablo yok"),
        (true, "yazildi") => "Teknik tablo getirildi · IdeaSoft'ta hedef kelime tanımlı değil".into(),
        (true, "korundu") => {
            "Teknik tablo alanı zaten dolu · IdeaSoft'ta hedef kelime tanımlı değil".into()
        }
        _ => "IdeaSoft'ta getirilecek bilgi bulunamadı".into(),
    };
    Ok(IdeasoftPull { detail: read_detail(&conn, &sku)?, message })
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

/// Canonical hedefi için **SATIŞTAKİ** ürünlerde arar.
///
/// Kullanıcı geri bildirimi (2026-07-30): *"Uygun halef bulunamasa bile kullanıcıya canonical
/// ayarla imkânı sunulmalı."* Yapay zekânın halef bulamaması hedefin olmadığı anlamına gelmiyor.
///
/// 🔴 **ARAMA KAYNAĞI FEED, IDEASOFT KATALOĞU DEĞİL — saha hatası (2026-07-30).**
/// İlk sürüm IdeaSoft'un tam kataloğunda (bu mağazada 10.909 ürün) arıyordu ve satıştan
/// kalkmış ürünleri de listeliyordu. Oysa bu akışın amacı tam tersi: satışta OLMAYAN ama
/// hâlâ trafik alan bir sayfayı **satıştaki** bir ürüne yönlendirmek. Ölü bir sayfayı başka
/// bir ölü sayfaya işaret ettirmek sorunu taşımak olurdu.
///
/// Feed = satıştaki ürünler (kullanıcı beyanı: *"Güncel olan ürünler xml de gelen ve sitede
/// satışta olan ürünlerdir"*). Yapay zekâ halef önerisi (`suggest_eol_successor`) zaten
/// `products` tablosunu kullanıyordu; elle seçme yolu bu kısıtı atlıyordu — aynı karara giden
/// iki yoldan biri kısıtlı, diğeri değildi.
///
/// Yan fayda: arama artık yerel, ağ çağrısı yok.
#[tauri::command]
pub fn search_live_products(
    state: State<'_, AppState>,
    term: String,
) -> Result<Vec<CatalogMatch>, String> {
    let t = term.trim();
    if t.len() < 3 {
        return Ok(Vec::new());
    }
    let like = format!("%{}%", t.to_lowercase());
    let conn = state.conn.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT sku, name, url FROM products
             WHERE url IS NOT NULL AND url <> ''
               AND (lower(name) LIKE ?1 OR lower(sku) LIKE ?1)
             ORDER BY name LIMIT 25",
        )
        .map_err(|e| format!("Ürünler okunamadı: {e}"))?;
    let rows = stmt
        .query_map([&like], |r| {
            let url: String = r.get(2)?;
            Ok(CatalogMatch {
                // Canonical hedefi URL'nin son parçası; ürün adresi feed'den geliyor.
                slug: url.trim().trim_end_matches('/').rsplit('/').next().unwrap_or("").to_lowercase(),
                // Feed'de IdeaSoft ürün kimliği yok; hedef için gerekmiyor (yalnızca slug yazılıyor).
                id: 0,
                name: r.get(1)?,
                // Feed'de olan ürün satıştadır; ayrıca durum sorgulamaya gerek yok.
                status: 1,
                stock: 0.0,
                canonical: r.get::<_, String>(0)?,
            })
        })
        .map_err(|e| format!("Ürünler okunamadı: {e}"))?
        .filter_map(Result::ok)
        .filter(|m| !m.slug.is_empty())
        .collect();
    Ok(rows)
}

/// Verilen slug SATIŞTAKİ bir ürüne mi ait? Canonical yazmadan önceki son kapı.
///
/// ⚠️ Arayüz zaten yalnızca satıştaki ürünleri listeliyor; bu kontrol ikinci savunma hattı.
/// Yapay zekâ önerisi de feed'den geldiği için iki yol da buradan geçiyor.
fn live_product_name(conn: &Connection, slug: &str) -> Option<String> {
    let s = slug.trim().trim_matches('/').to_lowercase();
    if s.is_empty() {
        return None;
    }
    // ⚠️ Karşılaştırma SQL'de `LIKE` ile YAPILMIYOR: slug'da geçen `_` karakteri LIKE'ta
    // joker olur ve yanlış ürünü eşleştirebilirdi. Feed birkaç yüz satır — tam tarama bedava.
    let mut stmt = conn
        .prepare("SELECT name, url FROM products WHERE url IS NOT NULL AND url <> ''")
        .ok()?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .ok()?;
    for (name, url) in rows.filter_map(Result::ok) {
        let last = url.trim().trim_end_matches('/').rsplit('/').next().unwrap_or("");
        if last.to_lowercase() == s {
            return Some(name);
        }
    }
    None
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
    // 🔴 HEDEF SATIŞTA OLMALI. Bu akışın amacı satışta olmayan bir sayfayı satıştaki bir ürüne
    // yönlendirmek; hedef de feed dışıysa ölü sayfayı başka bir ölü sayfaya işaret ettirmiş
    // oluruz (saha hatası, 2026-07-30). Arayüz zaten yalnızca satıştakileri listeliyor —
    // bu ikinci savunma hattı.
    let target_name = {
        let conn = state.conn.lock().unwrap();
        live_product_name(&conn, &target_slug)
    }
    .ok_or_else(|| {
        format!(
            "\"{}\" satıştaki ürünler arasında değil. Canonical yalnızca feed'deki (satıştaki) \
bir ürüne verilebilir; aksi halde ziyaretçiyi yine satın alınamayan bir sayfaya göndeririz.",
            target_slug.trim().trim_matches('/')
        )
    })?;

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
    // Önizlemede doğrulanmış olsa da burada TEKRAR bakılıyor: bu komut canlı mağazayı
    // değiştiriyor ve hedef satıştaki bir ürün değilse yazılmamalı. Yerel sorgu, ek istek yok.
    {
        let conn = state.conn.lock().unwrap();
        if live_product_name(&conn, &target_slug).is_none() {
            return Err(format!(
                "\"{}\" satıştaki ürünler arasında değil — canonical yazılmadı.",
                target_slug.trim().trim_matches('/')
            ));
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    fn db_with(urls: &[(&str, &str)]) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        seo_core::db::init(&conn).unwrap();
        for (sku, url) in urls {
            conn.execute(
                "INSERT INTO products (sku, name, url) VALUES (?1, ?2, ?3)",
                params![sku, format!("Ürün {sku}"), url],
            )
            .unwrap();
        }
        conn
    }

    /// 🔴 Bu testin koruduğu şey saha hatasının kendisi: canonical hedefi SATIŞTAKİ
    /// (feed'deki) bir ürün olmalı. Feed dışı bir slug kabul edilirse ölü sayfayı başka bir
    /// ölü sayfaya işaret ettirmiş oluruz.
    #[test]
    fn hedef_yalnizca_satistaki_urun_olabilir() {
        let conn = db_with(&[
            ("A1", "https://magaza.com/urun/satistaki-urun"),
            ("A2", "https://magaza.com/urun/ikinci-urun/"),
        ]);
        assert_eq!(live_product_name(&conn, "satistaki-urun").as_deref(), Some("Ürün A1"));
        // Sondaki eğik çizgi ve büyük harf eşleşmeyi bozmamalı.
        assert_eq!(live_product_name(&conn, "/İKİNCİ-URUN/".to_lowercase().as_str()).as_deref(), None);
        assert_eq!(live_product_name(&conn, "ikinci-urun").as_deref(), Some("Ürün A2"));
        // Feed'de olmayan (satıştan kalkmış) ürün → hedef olamaz.
        assert_eq!(live_product_name(&conn, "asus-zenbook-17-fold"), None);
        assert_eq!(live_product_name(&conn, ""), None);
    }

    /// ⚠️ `LIKE` ile yapılsaydı slug'daki `_` joker olur ve YANLIŞ ürünü eşleştirirdi.
    #[test]
    fn alt_cizgi_joker_gibi_davranmamali() {
        let conn = db_with(&[("B1", "https://magaza.com/urun/abc-x-def")]);
        // `abc_x_def` LIKE kalıbı olarak `abc-x-def`e uyardı; burada uymamalı.
        assert_eq!(live_product_name(&conn, "abc_x_def"), None);
        assert_eq!(live_product_name(&conn, "abc-x-def").as_deref(), Some("Ürün B1"));
    }
}
