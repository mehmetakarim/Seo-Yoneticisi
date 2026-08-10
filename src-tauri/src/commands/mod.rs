use seo_core::ideasoft;
use seo_core::images::{self, ImageCheck};
use seo_core::seo_data::{self, SeoInsights};
use seo_core::validation::{
    details_badge, image_badge, meta_badge, overall_status, MetaBadge, MetaInput, OverallInput,
    OverallStatus,
};
use seo_core::{db, csv_import, feed, fingerprint, gemini, history, jsonld, opportunity, sync};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

mod assistant;
mod contacts;
mod decisions;
mod focus;
mod generation;
mod ideasoft_cmd;
mod metrics_cmd;
mod opportunities;
mod products;
mod settings;
mod today;
mod versions;

// Komut adları DEĞİŞMEDİ: `lib.rs`'teki `invoke_handler` listesi ve ön yüzdeki `invoke`
// çağrıları aynen çalışıyor. Değişen tek şey komutların hangi dosyada durduğu.
pub use assistant::*;
pub use contacts::*;
pub use decisions::*;
pub use focus::*;
pub use generation::*;
pub use ideasoft_cmd::*;
pub use metrics_cmd::*;
pub use opportunities::*;
pub use products::*;
pub use settings::*;
pub use today::*;
pub use versions::*;

pub struct AppState {
    pub conn: Mutex<Connection>,
    #[allow(dead_code)] // Faz 2/3: harici DB yolu işlemleri için saklanır
    pub db_path: PathBuf,
}

/// Halef adaylarını sıralamak için katalog: `(sku, ad, url)`.
///
/// ⚠️ Ortak: hem halef önerisi (`opportunities.rs`) hem 301 CSV'si (`decisions.rs`) aynı
/// listeyi okuyor. İki yerde iki sorgu olsaydı biri değişince adaylar ayrışırdı.
pub(crate) fn live_catalog(conn: &Connection) -> Vec<(String, String, String)> {
    let mut stmt = match conn
        .prepare("SELECT sku, name, url FROM products WHERE url IS NOT NULL AND url <> ''")
    {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map(|rows| rows.filter_map(Result::ok).collect())
        .unwrap_or_default()
}

fn now_str() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

/// IdeaSoft'tan "Getir" sonucu: güncel ürün + ne yapıldığının kullanıcıya dönük özeti.
///
/// Mesaj arka uçta kuruluyor çünkü "yazıldı mı, korundu mu, hiç yok muydu" ayrımını yalnızca
/// burası biliyor. Ön yüz tahmin ederse yanlış söyler.
#[derive(Debug, Serialize)]
pub struct IdeasoftPull {
    pub detail: ProductDetail,
    pub message: String,
}

/// Onaylanan hâl ile şu anki feed verisi arasındaki tek alanlık fark.
#[derive(Debug, Serialize)]
pub struct FeedFieldDiff {
    /// `seo_core::fingerprint::FIELDS` adlarından biri.
    pub field: String,
    /// Onay anındaki değer. Açıklama alanında HTML etiketleri ayıklanmış hâli —
    /// kullanıcı işaretlemeye değil metne bakıyor.
    pub old: String,
    pub new: String,
}

/// "Ne değişti?" cevabı.
#[derive(Debug, Serialize)]
pub struct FeedDiff {
    /// Onay anındaki kayıt var mı? Özellikten ÖNCE onaylanmış ürünlerde yok: o zaman
    /// yalnızca alan adları biliniyor, önceki değerler geri getirilemez.
    pub has_snapshot: bool,
    /// Değişen alan adları — kayıt olmasa da her zaman dolu.
    pub changed_fields: Vec<String>,
    pub fields: Vec<FeedFieldDiff>,
    /// Görseller ayrı: metin değil, küçük resim olarak gösteriliyor.
    pub images_old: Vec<String>,
    pub images_new: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ProductRow {
    pub sku: String,
    pub name: String,
    pub brand: Option<String>,
    pub img_url: Option<String>,
    pub meta_badge: MetaBadge,
    pub details_badge: MetaBadge,
    pub overall: OverallStatus,
    pub meta_done: bool,
    pub details_done: bool,
    pub tech_done: bool,
    pub image_count: usize,
    /// Doluysa: kullanıcı bu ürünü "tamamlandı" işaretledikten SONRA feed verisi değişti.
    /// İçeriği değişen alanların adı ("ad, açıklama"). Bkz. [`mark_reviewed`].
    pub feed_changed: Option<String>,
    /// SEO sağlık skoru 0–100 (Faz D). ⚠️ `overall`ın YERİNE değil, yanına: `overall`
    /// filtrelere ve kuyruğa bağlı, değiştirmek beş ekranı birden etkilerdi.
    pub health: u32,
    /// Skoru düşüren bileşenler — baloncukta "neden 60?" sorusunu cevaplıyor.
    pub health_missing: Vec<seo_core::health::Missing>,
}

#[derive(Debug, Serialize)]
pub struct ProductDetail {
    pub sku: String,
    pub name: String,
    pub brand: Option<String>,
    pub main_category: Option<String>,
    pub category: Option<String>,
    pub quantity: Option<i64>,
    pub url: Option<String>,
    pub img_url: Option<String>,
    pub title: Option<String>,
    pub descriptions: Option<String>,
    pub keywords: Option<String>,
    pub search_keywords: Option<String>,
    pub details: Option<String>,
    pub meta_status: String,
    pub details_status: String,
    pub target_keyword: Option<String>,
    pub draft_title: Option<String>,
    pub draft_descriptions: Option<String>,
    pub draft_keywords: Option<String>,
    pub draft_search_keywords: Option<String>,
    pub draft_details: Option<String>,
    pub badge: MetaBadge,
    pub details_badge: MetaBadge,
    pub overall: OverallStatus,
    // Faz 7: galeri görselleri + skoru
    pub gallery: Vec<String>,
    pub image_count: usize,
    pub image_badge: MetaBadge,
    pub image_check: Option<Vec<ImageCheck>>,
    // Faz 8: teknik özellik tablosu
    pub tech_source_text: Option<String>,
    pub tech_specs: Option<Vec<gemini::TechGroup>>,
    pub tech_status: String,
    pub tech_badge: MetaBadge,
    /// Önceki sürümlerin hafif özeti (en yeni başta).
    pub tech_history: Vec<TechVersionMeta>,
    // Faz 9: IdeaSoft
    pub ideasoft_pushed_at: Option<String>,
    /// Doluysa: onaydan sonra feed verisi değişti; değişen alanların adı yazılı.
    pub feed_changed: Option<String>,
    /// IdeaSoft'un kendi SEO kural skoru (yalnızca liste ucunda dolu gelir, cache'lenir).
    pub ideasoft_seo_rule: Option<i64>,
    /// İçeriği hangi Gemini modelinin ürettiği. Zincir kotaya takıldıkça alt modellere
    /// düşüyor; kullanıcı bunu görüp limitler yenilendiğinde yeniden üretmeye karar verebilir.
    pub meta_model: Option<String>,
    pub details_model: Option<String>,
    pub tech_model: Option<String>,
    /// Yeniden üretimden önceki hâller (en yeni başta) — hafif özet.
    pub meta_history: Vec<MetaVersionMeta>,
    pub details_history: Vec<DetailsVersionMeta>,
}

#[derive(Debug, Serialize)]
pub struct Settings {
    pub feed_url: String,
    pub gemini_api_key: String,
    /// Faz 4: Ahrefs free-tools captcha'sını çözmek için CapSolver anahtarı.
    pub capsolver_api_key: String,
    /// Faz 4: SEO araştırma ülke kodu (Ahrefs/Trends), varsayılan "tr".
    pub seo_country: String,
    /// Faz 5: GSC mülkü (ör. `sc-domain:kurumsalit.com` veya `https://site/`).
    pub gsc_site_url: String,
    /// Faz 5: yüklü service-account'un e-postası (yalnızca gösterim; private key sızmaz).
    /// Boş → GSC yapılandırılmamış.
    pub gsc_client_email: String,
    /// Faz 9: IdeaSoft modülü (boşsa modül kapalı, kopyala-yapıştır akışı sürer).
    pub ideasoft_domain: String,
    pub ideasoft_token: String,
    pub ideasoft_active: bool,
    pub theme: Option<String>,
    pub last_backup_at: Option<String>,
}

struct RowData {
    sku: String,
    name: String,
    brand: Option<String>,
    img_url: Option<String>,
    title: Option<String>,
    descriptions: Option<String>,
    details: Option<String>,
    meta_status: String,
    details_status: String,
    target_keyword: Option<String>,
    draft_title: Option<String>,
    draft_descriptions: Option<String>,
    draft_details: Option<String>,
    tech_status: String,
    tech_specs_json: Option<String>,
    image_count: usize,
    /// Bkz. [`feed_change_note`] — rozet değil, uyarı metni.
    feed_changed: Option<String>,
    /// İçerik mağazaya ulaştı mı (Faz D sağlık skorunun bileşeni).
    pushed: bool,
    /// Görsel kontrolünde sorunlu bulunan görsel sayısı.
    image_problems: usize,
}

/// Meta rozeti — taslak varsa taslak (NULL değilse) yoksa feed değeri üzerinden.
fn meta_badge_of(r: &RowData) -> MetaBadge {
    let title = r.draft_title.as_deref().unwrap_or(r.title.as_deref().unwrap_or(""));
    let desc = r
        .draft_descriptions
        .as_deref()
        .unwrap_or(r.descriptions.as_deref().unwrap_or(""));
    meta_badge(&MetaInput {
        title,
        descriptions: desc,
        target_keyword: r.target_keyword.as_deref().unwrap_or(""),
        meta_done: r.meta_status == "done",
    })
}

/// Details rozeti — taslak varsa taslak yoksa feed details üzerinden.
fn details_badge_of(r: &RowData) -> MetaBadge {
    let html = r.draft_details.as_deref().unwrap_or(r.details.as_deref().unwrap_or(""));
    details_badge(html, r.target_keyword.as_deref().unwrap_or(""), r.details_status == "done")
}

fn read_detail(conn: &Connection, sku: &str) -> Result<ProductDetail, String> {
    conn.query_row(
        "SELECT p.sku, p.name, p.brand, p.main_category, p.category, p.quantity, p.url,
                p.img_url, p.title, p.descriptions, p.keywords, p.search_keywords, p.details,
                COALESCE(s.meta_status,'pending'), COALESCE(s.details_status,'pending'),
                s.target_keyword, s.draft_title, s.draft_descriptions, s.draft_search_keywords,
                s.draft_details, s.draft_keywords, p.picture2, p.picture3, p.picture4, s.image_check_json,
                s.tech_source_text, s.tech_specs_json, COALESCE(s.tech_status,'pending'),
                s.tech_history_json, s.ideasoft_pushed_at, s.ideasoft_seo_rule,
                s.meta_model, s.details_model, s.tech_model,
                s.meta_history_json, s.details_history_json,
                p.feed_fp, s.reviewed_fp, p.feed_changed
         FROM products p LEFT JOIN seo_status s ON s.sku = p.sku
         WHERE p.sku = ?1",
        [&sku],
        |row| {
            let img_url: Option<String> = row.get(7)?;
            let feed_changed = feed_change_note(row.get(36)?, row.get(37)?, row.get(38)?);
            let draft_keywords: Option<String> = row.get(20)?;
            let picture2: Option<String> = row.get(21)?;
            let picture3: Option<String> = row.get(22)?;
            let picture4: Option<String> = row.get(23)?;
            let check_json: Option<String> = row.get(24)?;
            let tech_source_text: Option<String> = row.get(25)?;
            let tech_specs: Option<Vec<gemini::TechGroup>> = row
                .get::<_, Option<String>>(26)?
                .as_deref()
                .and_then(|j| serde_json::from_str(j).ok());
            let tech_status: String = row.get(27)?;
            let tech_history: Vec<TechVersionMeta> =
                history::parse::<TechVersion>(row.get::<_, Option<String>>(28)?.as_deref())
                .into_iter()
                .map(|v| TechVersionMeta {
                    at: v.at,
                    rows: v.groups.iter().map(|g| g.rows.len()).sum(),
                    groups: v.groups.len(),
                })
                .collect();
            let meta_history: Vec<MetaVersionMeta> =
                history::parse::<MetaVersion>(row.get::<_, Option<String>>(34)?.as_deref())
                    .into_iter()
                    .map(|v| MetaVersionMeta { at: v.at, title: v.title, model: v.model })
                    .collect();
            let details_history: Vec<DetailsVersionMeta> =
                history::parse::<DetailsVersion>(row.get::<_, Option<String>>(35)?.as_deref())
                    .into_iter()
                    .map(|v| DetailsVersionMeta {
                        at: v.at,
                        words: seo_core::validation::word_count(&v.html),
                        model: v.model,
                    })
                    .collect();
            let gallery: Vec<String> = [img_url.clone(), picture2, picture3, picture4]
                .into_iter()
                .filter_map(|u| u.filter(|s| !s.trim().is_empty()))
                .collect();
            let image_check: Option<Vec<ImageCheck>> = check_json
                .as_deref()
                .and_then(|j| serde_json::from_str(j).ok());
            Ok(ProductDetail {
                sku: row.get(0)?,
                name: row.get(1)?,
                brand: row.get(2)?,
                main_category: row.get(3)?,
                category: row.get(4)?,
                quantity: row.get(5)?,
                url: row.get(6)?,
                img_url,
                title: row.get(8)?,
                descriptions: row.get(9)?,
                keywords: row.get(10)?,
                search_keywords: row.get(11)?,
                details: row.get(12)?,
                meta_status: row.get(13)?,
                details_status: row.get(14)?,
                target_keyword: row.get(15)?,
                draft_title: row.get(16)?,
                draft_descriptions: row.get(17)?,
                draft_search_keywords: row.get(18)?,
                draft_details: row.get(19)?,
                draft_keywords,
                badge: MetaBadge::Eksik,       // aşağıda hesaplanır
                details_badge: MetaBadge::Eksik, // aşağıda hesaplanır
                overall: OverallStatus::Eksik,   // aşağıda hesaplanır
                image_count: gallery.len(),
                image_badge: MetaBadge::Eksik, // aşağıda hesaplanır
                gallery,
                image_check,
                tech_badge: if tech_status == "done" {
                    MetaBadge::Tamamlandi
                } else if tech_specs.as_ref().map_or(false, |g| !g.is_empty()) {
                    MetaBadge::Uygun
                } else {
                    MetaBadge::Eksik
                },
                tech_source_text,
                tech_specs,
                tech_status,
                tech_history,
                ideasoft_pushed_at: row.get(29)?,
                feed_changed,
                ideasoft_seo_rule: row.get(30)?,
                meta_model: row.get(31)?,
                details_model: row.get(32)?,
                tech_model: row.get(33)?,
                meta_history,
                details_history,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
        other => format!("Ürün okunamadı: {other}"),
    })
    .map(|mut d| {
        let kw = d.target_keyword.as_deref().unwrap_or("");
        d.badge = meta_badge(&MetaInput {
            title: d.draft_title.as_deref().unwrap_or(d.title.as_deref().unwrap_or("")),
            descriptions: d
                .draft_descriptions
                .as_deref()
                .unwrap_or(d.descriptions.as_deref().unwrap_or("")),
            target_keyword: kw,
            meta_done: d.meta_status == "done",
        });
        let details_html = d.draft_details.as_deref().unwrap_or(d.details.as_deref().unwrap_or(""));
        d.details_badge = details_badge(details_html, kw, d.details_status == "done");
        d.overall = overall_status(&OverallInput {
            meta: d.badge,
            details: d.details_badge,
            meta_done: d.meta_status == "done",
            details_done: d.details_status == "done",
            tech_done: d.tech_status == "done",
            has_tech: d.tech_specs.as_ref().map_or(false, |g| !g.is_empty()),
            image_count: d.image_count,
        });
        // Görsel skoru: sayı + (varsa) cache'lenmiş boyut sonucu.
        let all_dims_ok = d.image_check.as_ref().map(|c| !c.is_empty() && c.iter().all(|x| x.ok));
        d.image_badge = image_badge(d.image_count, all_dims_ok);
        d
    })
}

/// Ürünü "bu hâliyle gözden geçirildi" olarak damgalar.
///
/// Kullanıcı meta/açıklama/teknik tablodan birini **tamamlandı** işaretlediğinde çağrılır:
/// o andaki feed parmak izi saklanır. Sonraki senkronda feed değişirse iki iz ayrışır ve
/// ürün "feed verisi değişti, gözden geçir" olarak işaretlenir (bkz. core/src/fingerprint.rs).
///
/// ⚠️ Yalnızca "tamamlandı"da çağrılıyor, üretimde değil: üretmek "onayladım" demek değil.
/// Bayrağın anlamı *"onayladıktan SONRA kaynak veri değişti"*.
/// Ürünün feed verisi, kullanıcının onayından sonra değişti mi?
///
/// Üç koşul birden: damga var (yani kullanıcı bir kez onaylamış), damga güncel izle uyuşmuyor
/// ve elimizde hangi alanların değiştiği yazıyor. Damga yoksa bayrak YOK — henüz onaylanmamış
/// ürün için "değişti" demek anlamsız, o zaten "bekliyor" durumunda.
fn feed_change_note(
    feed_fp: Option<String>,
    reviewed_fp: Option<String>,
    changed: Option<String>,
) -> Option<String> {
    let reviewed = reviewed_fp?;
    let current = feed_fp?;
    if reviewed == current {
        return None;
    }
    // Alan listesi bir sebeple boşsa da bayrak kalkmalı: iz ayrışması tek başına yeterli kanıt.
    Some(changed.unwrap_or_else(|| "feed verisi".into()))
}

fn mark_reviewed(conn: &Connection, sku: &str) -> Result<(), String> {
    // Parmak izi "değişti mi?", bu kayıt "NE değişti?" sorusunu cevaplıyor. İz geri
    // döndürülemez bir özet; onaylanan değerler saklanmazsa kullanıcıya yalnızca alan ADI
    // gösterilebiliyor ("görseller değişti") — hangi görselin gittiğini göremiyor.
    let facts = db::read_feed_facts(conn, sku);
    let snapshot = facts.as_ref().and_then(|f| serde_json::to_string(f).ok());
    conn.execute(
        "UPDATE seo_status SET reviewed_fp = (SELECT feed_fp FROM products WHERE sku = ?1),
                               reviewed_facts_json = ?2
         WHERE sku = ?1",
        params![sku, snapshot],
    )
    .map_err(|e| format!("Gözden geçirme damgası yazılamadı: {e}"))?;
    // Damgalandığına göre kullanıcı değişikliği görmüş sayılır; not temizlenir.
    conn.execute("UPDATE products SET feed_changed = NULL WHERE sku = ?1", [sku])
        .map_err(|e| format!("Değişiklik notu temizlenemedi: {e}"))?;
    Ok(())
}

fn ensure_seo_row(conn: &Connection, sku: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO seo_status (sku, meta_status, details_status, updated_at)
         VALUES (?1, 'pending', 'pending', ?2)
         ON CONFLICT(sku) DO NOTHING",
        params![sku, now_str()],
    )
    .map_err(|e| format!("SEO durumu oluşturulamadı: {e}"))?;
    Ok(())
}

/// Üretimden önceki meta hâlini geçmişe iter.
///
/// `Ok(None)` → geçmiş DEĞİŞMEMELİ (mevcut içerik boş, ya da yeni üretim eskisiyle birebir aynı).
/// Aynı sonucu veren yeniden üretimi kaydetmek geçmişi çöple doldurur ve gerçek eski hâlleri
/// `history::MAX` sınırından erken düşürürdü.
fn snapshot_meta(
    conn: &Connection,
    sku: &str,
    fresh: &gemini::GeneratedMeta,
) -> Result<Option<String>, String> {
    let cur: (String, String, String, String, String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT COALESCE(draft_title,''), COALESCE(draft_descriptions,''),
                    COALESCE(draft_keywords,''), COALESCE(draft_search_keywords,''),
                    COALESCE(target_keyword,''), meta_model, meta_history_json
             FROM seo_status WHERE sku = ?1",
            [sku],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        )
        .map_err(|e| format!("Mevcut meta okunamadı: {e}"))?;

    // İlk üretim: saklanacak bir şey yok
    if cur.0.trim().is_empty() && cur.1.trim().is_empty() {
        return Ok(None);
    }
    // Sonuç aynıysa sürüm açma
    if cur.0.trim() == fresh.title.trim() && cur.1.trim() == fresh.descriptions.trim() {
        return Ok(None);
    }

    let hist = history::push(
        history::parse::<MetaVersion>(cur.6.as_deref()),
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
    serde_json::to_string(&hist)
        .map(Some)
        .map_err(|e| format!("Meta geçmişi kaydedilemedi: {e}"))
}

/// Üretimden önceki açıklama hâlini geçmişe iter. Kurallar `snapshot_meta` ile aynı.
fn snapshot_details(conn: &Connection, sku: &str, fresh: &str) -> Result<Option<String>, String> {
    let (cur_html, cur_model, hist_json): (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT COALESCE(draft_details,''), details_model, details_history_json
             FROM seo_status WHERE sku = ?1",
            [sku],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|e| format!("Mevcut açıklama okunamadı: {e}"))?;

    if cur_html.trim().is_empty() || cur_html.trim() == fresh.trim() {
        return Ok(None);
    }
    let hist = history::push(
        history::parse::<DetailsVersion>(hist_json.as_deref()),
        DetailsVersion {
            at: now_str(),
            html: cur_html,
            model: cur_model.unwrap_or_default(),
        },
    );
    serde_json::to_string(&hist)
        .map(Some)
        .map_err(|e| format!("Açıklama geçmişi kaydedilemedi: {e}"))
}

/// `research_json` metnini SeoInsights'e çözer; bozuk/boşsa None.
fn parse_insights(json: Option<&str>) -> Option<SeoInsights> {
    let s = json?.trim();
    if s.is_empty() {
        return None;
    }
    serde_json::from_str::<SeoInsights>(s).ok()
}

/// Üretim komutlarının ortak önsözü: Gemini anahtarı + ürünün bağlam alanları.
///
/// Neden var: `generate_meta`, `generate_details` ve `structure_tech_specs` aynı beş alanı
/// (ad/marka/kategori/ana kategori/araştırma) ve aynı anahtar okumasını **üç kez** kopyalıyordu.
/// Kopyalanan mantık bu projede zaten bir kez üretimi durdurdu: 2026-07-28'de hata
/// sınıflandırması dört yere kopyalanmıştı ve dördü de yanlıştı (bkz. `gemini::classify_error`).
/// Yeni bir grounding alanı eklendiğinde tek yer değişsin diye burada toplandı.
///
/// ⚠️ **Sahiplik:** `gemini::ProductContext` ödünç alınmış alanlar (`&str`) tutuyor, bu yüzden
/// veri burada **sahipli** durur ve `as_context()` çağrı yerinde ödünç verir.
struct CtxParts {
    key: String,
    name: String,
    brand: Option<String>,
    category: Option<String>,
    main_category: Option<String>,
    insights: Option<SeoInsights>,
}

impl CtxParts {
    /// `target_keyword`: yalnızca meta üretiminde dolu — açıklama kendi `keyword` argümanını,
    /// teknik tablo ise hiç kelime kullanmıyor.
    ///
    /// `with_insights`: teknik tabloda bilinçli olarak **kapalı** — o tablo ürünün kendi teknik
    /// verisinden çıkar, SEO araştırması (pazarlama verisi) karıştırılırsa uydurma özellik riski
    /// doğar. Karar burada görünür kalsın diye gizli bir varsayılan değil, açık bir parametre.
    fn as_context<'a>(
        &'a self,
        target_keyword: Option<&'a str>,
        with_insights: bool,
    ) -> gemini::ProductContext<'a> {
        gemini::ProductContext {
            name: &self.name,
            brand: self.brand.as_deref(),
            category: self.category.as_deref(),
            main_category: self.main_category.as_deref(),
            target_keyword: target_keyword.map(str::trim).filter(|k| !k.is_empty()),
            insights: if with_insights {
                self.insights.as_ref().filter(|i| i.has_data())
            } else {
                None
            },
        }
    }
}

/// Bağlam alanlarını tek sorguda okur. Çağıran, ürüne özel ek alanları (galeri, mevcut açıklama,
/// teknik kaynak metni) **kendi** sorgusuyla alır — ortaklaşan yalnızca bu beş alan.
fn ctx_parts(conn: &Connection, sku: &str) -> Result<CtxParts, String> {
    let key = db::get_setting(conn, "gemini_api_key")?.unwrap_or_default();
    let (name, brand, category, main_category, research_json) = conn
        .query_row(
            "SELECT p.name, p.brand, p.category, p.main_category, s.research_json
             FROM products p LEFT JOIN seo_status s ON s.sku = p.sku
             WHERE p.sku = ?1",
            [sku],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => format!("Ürün bulunamadı: {sku}"),
            other => format!("Ürün okunamadı: {other}"),
        })?;
    Ok(CtxParts {
        key,
        name,
        brand,
        category,
        main_category,
        insights: parse_insights(research_json.as_deref()),
    })
}

/// Ürün adından tohum kelime türetir (ilk `n` anlamlı sözcük).
fn first_words(name: &str, n: usize) -> String {
    name.split_whitespace().take(n).collect::<Vec<_>>().join(" ")
}

/// URL'den alan adını (www'suz) çıkarır.
fn host_of(url: &str) -> Option<String> {
    reqwest::Url::parse(url)
        .ok()?
        .host_str()
        .map(|h| h.trim_start_matches("www.").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ver(at: &str) -> TechVersion {
        TechVersion {
            at: at.into(),
            groups: vec![gemini::TechGroup {
                group: "Performans".into(),
                rows: vec![gemini::TechRow { label: "İşlemci".into(), value: "i7".into() }],
            }],
            source: format!("kaynak {at}"),
        }
    }

    #[test]
    fn push_history_keeps_newest_first_and_caps() {
        let mut h: Vec<TechVersion> = Vec::new();
        for i in 1..=7 {
            h = history::push(h, ver(&format!("v{i}")));
        }
        // En yeni başta, üst sınır aşılmaz
        assert_eq!(h.len(), history::MAX);
        assert_eq!(h[0].at, "v7");
        assert_eq!(h[history::MAX - 1].at, "v3"); // v1, v2 düştü
    }

    #[test]
    fn parse_history_tolerates_missing_and_broken() {
        assert!(history::parse::<TechVersion>(None).is_empty());
        assert!(history::parse::<TechVersion>(Some("")).is_empty());
        assert!(history::parse::<TechVersion>(Some("  ")).is_empty());
        assert!(history::parse::<TechVersion>(Some("{bozuk json")).is_empty());
        let json = serde_json::to_string(&vec![ver("v1")]).unwrap();
        assert_eq!(history::parse::<TechVersion>(Some(&json)).len(), 1);
    }

    #[test]
    fn history_roundtrip_preserves_source_and_rows() {
        let json = serde_json::to_string(&vec![ver("2026-07-25T10:00:00")]).unwrap();
        let back = history::parse::<TechVersion>(Some(&json));
        assert_eq!(back[0].source, "kaynak 2026-07-25T10:00:00");
        assert_eq!(back[0].groups[0].rows[0].value, "i7");
    }
}
