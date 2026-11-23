//! Bugün kuyruğunun Tauri katmanı — girdileri toplar, `seo_core::queue`e verir.
//!
//! Saf seçim mantığı (skor, tekilleştirme, kova sınırı) `seo_core::queue`de; burada yalnızca
//! veritabanı erişimi ve **kovaların tanımı** var.
//!
//! ⚠️ **Yeni GSC çağrısı YOK.** Beş kovanın dördü mevcut `opportunity_json` önbelleğinden
//! besleniyor. Kuyruk açmak kota harcamamalı; analiz eskiyse ekran kaç gün önce çalıştığını
//! yazıyor (`analyzed_at`), kendiliğinden yenilemiyor.

use super::*;
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use seo_core::contacts::DueContact;
use seo_core::queue::{self, Bucket, Candidate, ItemRef, QueueItem};
use std::collections::{HashMap, HashSet};

/// Ön yüze giden kuyruk + kullanıcının kuyruğu yorumlaması için gereken bağlam.
#[derive(Serialize)]
pub struct TodayQueue {
    pub items: Vec<QueueItem>,
    /// Analiz ne zaman çalıştı — kuyruk bayat mı, kullanıcı görsün.
    pub analyzed_at: String,
    /// Gizlenmiş/ertelenmiş madde sayısı (geri alma düğmesi bunu gösteriyor).
    pub hidden: usize,
    /// Bugünün listesinde kaç madde "yapıldı" — ilerleme çubuğu bunu gösteriyor.
    pub done_count: usize,
    /// Kovaların o anki aday sayısı — boş kova "neden boş" diyebilsin diye.
    pub bucket_counts: Vec<BucketCount>,
    /// Ölçümü uçuşta olan iş sayısı — yapıldı, sonucu 28 gün sonra görünecek.
    ///
    /// Ekranda söyleniyor: süzgeç sessiz çalışsaydı kullanıcı kovanın neden küçüldüğünü
    /// bilemezdi. `review_ready_at` ile aynı gerekçe.
    pub in_flight: usize,
    /// Sonuç kontrolü kovası boşsa en erken ne zaman dolacağı (YYYY-AA-GG) — boşsa "".
    ///
    /// ⚠️ Ölçüldü (2026-08-07): 72 gönderimin tamamı 0–12 gün önce yapılmış, bu kova bugün
    /// BOŞ ve Eylül ortasına kadar boş kalacak. Sessizce boş bırakmak yerine tarihi söylüyoruz.
    pub review_ready_at: String,
}

#[derive(Serialize)]
pub struct BucketCount {
    pub bucket: Bucket,
    pub label: String,
    pub candidates: usize,
}

#[derive(Default, serde::Deserialize)]
#[serde(default)]
pub struct SharedTodayInputs {
    report: Option<OpportunityReport>,
    products: Vec<ProductState>,
    events: Vec<SharedTodayEvent>,
    pages: Vec<SharedTodayPage>,
    contacts: Vec<SharedTodayContact>,
    decisions: Vec<SharedTodayDecision>,
    focus_scope: Option<String>,
}

#[derive(serde::Deserialize)]
struct SharedTodayEvent {
    sku: Option<String>,
    url: Option<String>,
    at: String,
    kind: String,
    reaches_store: bool,
}

#[derive(serde::Deserialize)]
struct SharedTodayPage {
    kind: String,
    slug: String,
    remote_id: i64,
    status: i64,
}

#[derive(serde::Deserialize)]
struct SharedTodayContact {
    id: String,
    name: String,
    company: String,
    next_step_at: Option<String>,
    next_step_note: String,
    last_contact_at: Option<String>,
    archived: bool,
}

#[derive(serde::Deserialize)]
struct SharedTodayDecision {
    kind: String,
    reference: String,
    action: String,
    until_day: Option<String>,
    occurred_at: String,
    analyzed_at: Option<String>,
}

/// Kuyruktan çıkarılmış maddeler: `(kind, ref)` → gizli mi?
///
/// Erteleme süresi dolmuşsa gizli sayılmıyor; kayıt temizlenmesine gerek yok, sorgu bugünün
/// tarihine bakıyor.
fn sorgu(conn: &Connection, sql: &str, p: &[&dyn rusqlite::ToSql]) -> Vec<(String, String)> {
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map(p, |r| Ok((r.get(0)?, r.get(1)?)))
        .map(|rows| rows.filter_map(Result::ok).collect())
        .unwrap_or_default()
}

/// Kuyruktan **tamamen çıkarılmış** maddeler: kalıcı gizleme ve süresi dolmamış erteleme.
///
/// ⚠️ "Yapıldı" burada DEĞİL — o madde listede kalmaya devam ediyor (bkz. [`completed`]).
fn hidden(conn: &Connection) -> Vec<(String, String)> {
    let bugun = now_str()[..10].to_string();
    sorgu(
        conn,
        "SELECT kind, ref FROM queue_dismissals
         WHERE (until IS NULL AND done_at_analysis IS NULL) OR until > ?1",
        &[&bugun],
    )
}

/// "Yapıldı" işaretli maddeler — **bugün** ve **bu analiz** için geçerli.
///
/// 🔴 Bu maddeler kuyruktan ÇIKARILMIYOR, yerinde kalıp üstü çizili gösteriliyor. Anında
/// düşürüldüğünde yerlerine yeni aday geliyor, sayaç hep 10'da kalıyor ve gün hiç bitmiyordu
/// (saha geri bildirimi, 2026-08-08).
///
/// 🔴 **GÜN KOŞULU sonradan eklendi (saha geri bildirimi, 2026-08-10):** *"Bugün sayfasında
/// hâlâ önceki günün yapıldı olarak işaretlenmiş işleri mevcut."* İşaret yalnızca analiz
/// damgasına bağlıydı; analiz yenilenmediği sürece dünkü bitmiş işler ekranda kalıyordu.
/// Ekranın adı **Bugün** — içeriği de güne bağlı olmalı, analize değil.
///
/// Yarın ne oluyor: işaret düşüyor ama madde geri **gelmiyor** — [`in_flight`] onu
/// devralıyor (yapılan işin kanıtı 28 gün sonra görünecek). Yani iki mekanizma sırayla
/// çalışıyor: `completed` bugün, `in_flight` sonrasında.
fn completed(conn: &Connection, analyzed_at: &str) -> Vec<(String, String)> {
    let bugun = now_str()[..10].to_string();
    sorgu(
        conn,
        "SELECT kind, ref FROM queue_dismissals
         WHERE done_at_analysis = ?1 AND date(at) = ?2",
        &[&analyzed_at, &bugun],
    )
}

/// Kimlik başına **mağazaya ulaşan** son iş — `(kimlik, at)`.
///
/// ⚠️ Tek yerde: hem sonuç kontrolü kovası (28 günü DOLANLAR) hem uçuş süzgeci (28 günü
/// DOLMAYANLAR) buradan okuyor. İki ayrı sorgu olsaydı eşikler zamanla ayrışır, madde ikisinin
/// arasına düşüp kaybolabilirdi.
///
/// 🔴 **`sku IS NOT NULL` koşulu kaldırıldı (Faz İ).** İçerik gönderimlerinin sku'su yok,
/// kimlikleri URL. Koşul dururken o olaylar `work_events`'e yazılıyor ama HİÇBİR YERDE
/// okunmuyordu: sonuç kontrolü kovası onları geri getirmiyor, rozet çıkmıyordu. Yani içerik
/// işinin sonucu ölçülüyor sanılıyordu, ölçülmüyordu.
/// ⚠️ **Kimlik seçimi.** Ürün olayı `Product(sku)`; sayfa olayı `Page(slug)` — URL değil
/// **slug**, çünkü `ItemRef::Page` kaçak kovasında da slug taşıyor ve aynı kimlik uzayında
/// iki farklı biçim bulundurmak, ileride "neden eşleşmiyor" diye aranacak bir tuzak olurdu.
/// Sonuç hesabı için gereken tam adres olayın kendisinde (`work_events.url`) duruyor.
///
/// ⚠️ İçerik maddeleri `ItemRef::Query` taşıyor, yani buradaki `Page` kimliğiyle **eşleşmez**
/// ve bu bilinçli: bir sayfaya metin göndermek o sayfayla sıralanan HER sorguyu çözmez.
/// İçerik maddesinin susması "Yapıldı" işaretiyle oluyor (`queue_dismissals`, `in_flight`
/// içinde zaten okunuyor).
fn store_events(conn: &Connection) -> Vec<(ItemRef, String)> {
    let mut stmt = match conn.prepare(
        "SELECT sku, url, MAX(at) FROM work_events
         WHERE reaches_store = 1 AND COALESCE(sku, url) IS NOT NULL
         GROUP BY COALESCE(sku, url)",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map([], |r| {
        Ok((
            r.get::<_, Option<String>>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, String>(2)?,
        ))
    })
    .map(|rows| {
        rows.filter_map(Result::ok)
            .filter_map(|(sku, url, at)| match sku {
                Some(s) if !s.is_empty() => Some((ItemRef::Product(s), at)),
                _ => {
                    let u = url?;
                    let slug = seo_core::page_kind::last_segment(&u).unwrap_or(u);
                    Some((ItemRef::Page(slug), at))
                }
            })
            .collect()
    })
    .unwrap_or_default()
}

/// Ölçümü **uçuşta** olan referanslar — yapıldı ama sonucu henüz görünemez.
///
/// Gerekçe ve ölçüm `seo_core::queue::drop_in_flight`te. Burada yalnızca iki kaynak var:
///
/// 1. **Mağazaya ulaşan olaylar** (ürünler) — gerçek gönderim de, "elle yapıldı" beyanı da.
/// 2. **"Yapıldı" işaretleri** (sayfalar) — EOL sayfalarının sku'su yok, olay günlüğüne
///    giremiyorlar; onlar için işaretin kendi zaman damgası tek kanıt.
///
/// 🔴 **BU ANALİZDE işaretlenenler süzgece GİRMİYOR.** Onlar [`completed`]in işi: listede
/// yerinde kalıp üstü çizili duruyorlar. Süzgece girselerdi adaylıktan düşer, yerlerine 11.
/// madde gelir ve 2026-08-08'de düzeltilen *"bu mantık ile günlük iş hiç bitmez"* hatası
/// geri dönerdi. İki mekanizma aynı işareti okuyor ama farklı zaman ölçeğinde:
/// `completed` **bu analiz** boyunca, `in_flight` **sonraki analizlerde** konuşuyor.
fn in_flight(conn: &Connection, analyzed_at: &str) -> std::collections::HashSet<ItemRef> {
    let mut out = std::collections::HashSet::new();
    for (kimlik, at) in store_events(conn) {
        if days_since(&at) < queue::REVIEW_AFTER_DAYS {
            out.insert(kimlik);
        }
    }
    let esik = (chrono::Local::now() - chrono::Duration::days(queue::REVIEW_AFTER_DAYS))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
    for (kind, r) in sorgu(
        conn,
        "SELECT kind, ref FROM queue_dismissals
         WHERE done_at_analysis IS NOT NULL AND at > ?1",
        &[&esik],
    ) {
        out.insert(ref_from(&kind, r));
    }
    for (kind, r) in completed(conn, analyzed_at) {
        out.remove(&ref_from(&kind, r));
    }
    out
}

/// Kararı verilmiş EOL sayfalarının slug'ları (Faz D).
///
/// 🔑 **`keep` de dahil.** Bilinçli tutulan sayfa bir iş değil; kuyruğun her analizde onu
/// yeniden önermesi kararı yok saymak olurdu.
fn decided_pages(conn: &Connection) -> std::collections::HashSet<String> {
    sorgu(conn, "SELECT slug, action FROM eol_decisions", &[])
        .into_iter()
        .map(|(slug, _)| slug)
        .collect()
}

pub(crate) fn ref_key(r: &ItemRef) -> (&'static str, String) {
    match r {
        ItemRef::Product(s) => ("product", s.clone()),
        ItemRef::Page(s) => ("page", s.clone()),
        ItemRef::Contact(s) => ("contact", s.clone()),
        ItemRef::Query(s) => ("query", s.clone()),
    }
}

/// `(kind, ref)` ikilisinden kimlik — [`ref_key`]in tersi.
///
/// ⚠️ Tek yerde: `queue_dismissals` üç türü de metin olarak saklıyor; üç ayrı yerde `match`
/// yazılsaydı biri "contact"ı eklemeyi unutup sessizce ürün sayardı.
fn ref_from(kind: &str, r: String) -> ItemRef {
    match kind {
        "page" => ItemRef::Page(r),
        "contact" => ItemRef::Contact(r),
        "query" => ItemRef::Query(r),
        _ => ItemRef::Product(r),
    }
}

/// Ürün başına feed/gönderim durumu — acil kovasının kaynağı.
#[derive(serde::Deserialize)]
struct ProductState {
    sku: String,
    name: String,
    feed_changed: Option<String>,
    pushed_at: Option<String>,
    meta_done: bool,
    details_done: bool,
}

fn product_states(conn: &Connection) -> Vec<ProductState> {
    let mut stmt = match conn.prepare(
        "SELECT p.sku, p.name, p.feed_changed, s.ideasoft_pushed_at,
                COALESCE(s.meta_status,''), COALESCE(s.details_status,'')
         FROM products p LEFT JOIN seo_status s ON s.sku = p.sku",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map([], |r| {
        Ok(ProductState {
            sku: r.get(0)?,
            name: r.get(1)?,
            feed_changed: r.get(2)?,
            pushed_at: r.get(3)?,
            meta_done: r.get::<_, String>(4)? == "done",
            details_done: r.get::<_, String>(5)? == "done",
        })
    })
    .map(|rows| rows.filter_map(Result::ok).collect())
    .unwrap_or_default()
}

/// Gün farkı — `at` "YYYY-AA-GGTss:dd:ss" biçiminde.
fn days_since(at: &str) -> i64 {
    calendar_date(at).map(|day| (Local::now().date_naive() - day).num_days()).unwrap_or(0)
}

fn calendar_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value.get(..10)?, "%Y-%m-%d").ok()
}

fn utc_to_local(value: &str) -> Option<NaiveDateTime> {
    let input = value.replace(' ', "T");
    if let Ok(stamp) = DateTime::parse_from_rfc3339(&input) {
        return Some(stamp.with_timezone(&Local).naive_local());
    }
    let stamp = NaiveDateTime::parse_from_str(&input, "%Y-%m-%dT%H:%M:%S%.f").ok()?;
    Some(Utc.from_utc_datetime(&stamp).with_timezone(&Local).naive_local())
}

/// Beş kovanın adaylarını kurar.
///
/// Her adayın `reason`ı **gerçek bir metrikten** türüyor (yol haritasının şartı): uydurma
/// bir "iyileştirilmeli" cümlesi yok, hepsinde sayı var.
struct CandidateData<'a> {
    products: &'a [ProductState],
    events: &'a [(ItemRef, String)],
    decided_pages: HashSet<String>,
    active_slugs: HashSet<String>,
    contacts: Vec<(String, DueContact)>,
    shared: Option<&'a SharedTodayInputs>,
}

fn local_contact_work(conn: &Connection, analyzed_at: &str) -> Vec<(String, DueContact)> {
    let mut contacts = super::due_contacts(conn);
    contacts.extend(super::silent_contacts(conn));
    // Dönüş tamamlandığında sonraki adım silinir. Günün işareti görünür kalır.
    let existing: HashSet<i64> = contacts.iter().map(|contact| contact.id).collect();
    let completed_ids: Vec<i64> = completed(conn, analyzed_at)
        .into_iter()
        .filter(|(kind, _)| kind == "contact")
        .filter_map(|(_, reference)| reference.parse().ok())
        .filter(|id| !existing.contains(id))
        .collect();
    contacts.extend(super::contacts_by_ids(conn, &completed_ids));
    contacts.into_iter().map(|contact| (contact.id.to_string(), contact)).collect()
}

fn candidates(
    conn: &Connection,
    report: &OpportunityReport,
    in_flight: &HashSet<ItemRef>,
) -> Vec<Candidate> {
    let active_slugs = conn
        .prepare("SELECT lower(slug) FROM store_pages WHERE status = 1")
        .and_then(|mut st| {
            st.query_map([], |row| row.get::<_, String>(0))
                .map(|rows| rows.filter_map(Result::ok).collect())
        })
        .unwrap_or_default();
    build_candidates(conn, report, in_flight, CandidateData {
        products: &product_states(conn),
        events: &store_events(conn),
        decided_pages: decided_pages(conn),
        active_slugs,
        contacts: local_contact_work(conn, &report.analyzed_at),
        shared: None,
    })
}

fn build_candidates(
    conn: &Connection,
    report: &OpportunityReport,
    in_flight: &HashSet<ItemRef>,
    input: CandidateData<'_>,
) -> Vec<Candidate> {
    let mut out = Vec::new();
    // 🔑 Faz S'in ürünü: kova başına ÖLÇÜLMÜŞ süre. Yeterli örnek yoksa (kova başına 5)
    // aşağıdaki elle yazılmış tahminler kullanılmaya devam ediyor.
    let olculen = match input.shared {
        Some(shared) => shared.focus_scope.as_deref()
            .map(|scope| super::focus::calibration_for_scope(conn, scope))
            .unwrap_or_default(),
        None => super::calibration(conn),
    };
    // Ölçülmüş süre varsa onu, yoksa elle yazılan tahmini döndürür.
    let sure = |b: Bucket, tahmin: u32| -> (u32, bool) {
        match olculen.get(&format!("{b:?}").to_lowercase()) {
            Some(dk) => (*dk, true),
            None => (tahmin, false),
        }
    };
    let states = input.products;
    let active_products: HashSet<_> = states.iter().map(|product| product.sku.as_str()).collect();
    let product_visible = |sku: &str| input.shared.is_none() || active_products.contains(sku);
    // 🔑 Mağazaya ULAŞMIŞ sayılan ürünler: gerçek gönderim (`ideasoft_pushed_at`) **veya**
    // kullanıcının "yapıldı" beyanı (`manual_done`, reaches_store = 1). İkincisi olmadan,
    // içeriği elle mağazaya yapıştıran kullanıcının işi hiç bitmiyordu — madde her gün
    // "hiç gönderilmemiş" diye geri geliyordu (saha hatası, 2026-08-10).
    // ⚠️ Yalnızca ÜRÜN olayları: bu küme acil kovasında "bu ürün hiç gönderilmiş mi?"
    // sorusunu cevaplıyor. Sayfa olayları buraya karışsaydı bir kategori sayfasının
    // gönderimi, adı benzeyen bir ürünü "gönderilmiş" sayabilirdi.
    let magazaya_ulasan: HashSet<&str> = input.events.iter()
        .filter_map(|(k, _)| match k {
            ItemRef::Product(s) => Some(s.as_str()),
            _ => None,
        })
        .collect();
    // Ürünün GSC tıklaması — acil maddelerinin skorunu da tıklama sürüyor.
    let clicks_of: std::collections::HashMap<&str, f64> = report
        .opportunities
        .iter()
        .map(|o| (o.sku.as_str(), o.clicks))
        .collect();

    // --- 1) ACİL: canlıda yanlış içerik ---
    for p in states {
        let reference = ItemRef::Product(p.sku.clone());
        // ⚠️ Yalnızca METİN alanı değişmişse ve içerik mağazaya gitmişse acil. Görsel
        // değişikliği üretilmiş metni geçersiz kılmıyor (ölçüm: 70 bayrağın 61'i görsel).
        let changed = p.feed_changed.as_deref().unwrap_or_default();
        if p.pushed_at.is_some() && queue::is_urgent_change(changed) {
            out.push(Candidate {
                reference: reference.clone(),
                bucket: Bucket::Urgent,
                title: p.name.clone(),
                reason: format!(
                    "mağazaya gönderildikten sonra feed değişti ({}) — canlıdaki metin bayat",
                    changed
                ),
                clicks: *clicks_of.get(p.sku.as_str()).unwrap_or(&0.0),
                page: "products".into(),
                focus_id: p.sku.clone(),
                minutes: sure(Bucket::Urgent, 2).0,
                minutes_measured: sure(Bucket::Urgent, 2).1,
            });
        }
        // İş yerelde bitmiş ama mağazaya hiç ulaşmamış → Faz Ö'nün merkezi kuralına göre
        // bu iş Google için HİÇ yapılmamış sayılır ve ölçülemez.
        // Ekip kararının olayı önbelleğe henüz gelmemiş olabilir.
        let awaiting_event = input.shared.is_some() && in_flight.contains(&reference);
        if p.meta_done && p.details_done && p.pushed_at.is_none()
            && !magazaya_ulasan.contains(p.sku.as_str())
            && !awaiting_event
        {
            out.push(Candidate {
                reference,
                bucket: Bucket::Urgent,
                title: p.name.clone(),
                reason: "içerik hazır ama mağazaya hiç gönderilmemiş — Google bunu görmüyor".into(),
                clicks: *clicks_of.get(p.sku.as_str()).unwrap_or(&0.0),
                page: "products".into(),
                focus_id: p.sku.clone(),
                minutes: sure(Bucket::Urgent, 1).0,
                minutes_measured: sure(Bucket::Urgent, 1).1,
            });
        }
    }

    // --- 2) YÜKSEK KALDIRAÇ: GSC fırsatları ---
    for o in report.opportunities.iter().filter(|item| product_visible(&item.sku)) {
        out.push(Candidate {
            reference: ItemRef::Product(o.sku.clone()),
            bucket: Bucket::Leverage,
            title: o.name.clone(),
            reason: format!(
                "konum {:.1}, {} gösterim ama {} tıklama — {} tıklama kaçıyor",
                o.position,
                o.impressions.round(),
                o.clicks.round(),
                o.missed_clicks.round()
            ),
            clicks: o.missed_clicks,
            page: "opportunities".into(),
            focus_id: o.sku.clone(),
            minutes: sure(Bucket::Leverage, 2).0,
            minutes_measured: sure(Bucket::Leverage, 2).1,
        });
    }

    // --- 3) KAÇAK TRAFİK: satışta olmayan sayfalar ---
    // ⚠️ Kararı verilmiş sayfalar dışarıda (Faz D): 301'i panelde tanımladıysanız ya da
    // sayfayı bilinçli tutuyorsanız bu artık bir iş değil. Uygulama 301'i doğrulayamıyor,
    // tek kanıt sizin kararınız.
    for e in report.eol.iter().filter(|e| !input.decided_pages.contains(&e.slug)) {
        out.push(Candidate {
            // ⚠️ Sayfa kimliği: EOL satırlarında sku YOK, ürün maddeleriyle birleştirilemez.
            reference: ItemRef::Page(e.slug.clone()),
            bucket: Bucket::Leak,
            title: e.slug.clone(),
            reason: format!(
                "{} tıklama satın alınamayan bir sayfaya gidiyor (konum {:.1})",
                e.clicks.round(),
                e.position
            ),
            clicks: e.clicks,
            page: "eol".into(),
            focus_id: e.url.clone(),
            minutes: sure(Bucket::Leak, 1).0,
            minutes_measured: sure(Bucket::Leak, 1).1,
        });
    }

    // --- 4) SONUÇ KONTROLÜ: yeterince beklemiş gönderimler ---
    // 🔑 Uçuş süzgecinin diğer ucu: 28 günü DOLMAYAN madde diğer kovalarda susuyor, dolan
    // madde burada geri geliyor. Aynı sorgudan okuyorlar (bkz. `store_events`).
    let mut gorulen = std::collections::HashSet::new();
    for (kimlik, at) in input.events {
        let yas = days_since(at);
        if yas < queue::REVIEW_AFTER_DAYS || !gorulen.insert(kimlik.clone()) {
            continue;
        }
        let anahtar = kimlik.key().to_string();
        // Ürün ise ürün ekranı, sayfa ise içerik açığı ekranı — sonucu orada okunuyor.
        let urun = matches!(kimlik, ItemRef::Product(_));
        let ad = if urun {
            states
                .iter()
                .find(|p| p.sku == anahtar)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| anahtar.clone())
        } else {
            anahtar.clone()
        };
        let target = if urun {
            ("products".to_owned(), anahtar.clone())
        } else if let Some(shared) = input.shared {
            let Some(target) = store_page_target(&shared.pages, &anahtar) else { continue };
            target
        } else {
            ("contentgap".to_owned(), anahtar.clone())
        };
        let reason = if !urun && input.shared.is_some() {
            format!("{yas} gün önce mağazada işlem kaydedildi · Sonucu kontrol edin")
        } else {
            format!("{yas} gün önce mağazaya gönderildi — sonucuna bakılabilir")
        };
        out.push(Candidate {
            reference: kimlik.clone(),
            bucket: Bucket::Review,
            title: ad,
            reason,
            clicks: 0.0,
            page: target.0,
            focus_id: target.1,
            minutes: sure(Bucket::Review, 1).0,
            minutes_measured: sure(Bucket::Review, 1).1,
        });
    }

    // --- 5) BAKIM: düşüşte olanlar + yarışan sayfalar ---
    for d in report.decay.iter().filter(|item| product_visible(&item.sku)) {
        out.push(Candidate {
            reference: ItemRef::Product(d.sku.clone()),
            bucket: Bucket::Upkeep,
            title: d.name.clone(),
            reason: format!(
                "tıklama {}→{}, konum {:.1}→{:.1}",
                d.clicks_before.round(),
                d.clicks_now.round(),
                d.position_before,
                d.position_now
            ),
            clicks: d.clicks_lost,
            page: "decay".into(),
            focus_id: d.sku.clone(),
            minutes: sure(Bucket::Upkeep, 2).0,
            minutes_measured: sure(Bucket::Upkeep, 2).1,
        });
    }
    for c in &report.cannibalization {
        out.push(Candidate {
            reference: ItemRef::Page(format!("q:{}", c.query)),
            bucket: Bucket::Upkeep,
            title: c.query.clone(),
            reason: format!(
                "{} sayfanız aynı aramada yarışıyor, {} tıklama bölünüyor",
                c.pages.len(),
                c.clicks.round()
            ),
            clicks: c.clicks,
            page: "cannibal".into(),
            focus_id: c.query.clone(),
            minutes: sure(Bucket::Upkeep, 3).0,
            minutes_measured: sure(Bucket::Upkeep, 3).1,
        });
    }

    // --- 6) MÜŞTERİ: sonraki adımı gelmiş kişiler (Faz C) ---
    // 🔑 Kuyruğun tek insan işi ve tek "bozulabilir" işi: dönülmezse iş rakibe gider, oysa
    // bir sayfa bir gün beklemekle hiçbir şey kaybetmez. Skoru bu yüzden gecikme sürüyor.
    for (id, k) in input.contacts {
        out.push(Candidate {
            reference: ItemRef::Contact(id.clone()),
            bucket: Bucket::Contact,
            title: k.title(),
            reason: k.reason(),
            // ⚠️ `clicks` alanı bu kovada GECİKME GÜNÜ taşıyor (bkz. `queue::score`).
            clicks: k.overdue_days as f64,
            page: "contacts".into(),
            focus_id: id,
            minutes: sure(Bucket::Contact, 5).0,
            minutes_measured: sure(Bucket::Contact, 5).1,
        });
    }

    // ===== İçerik açığı (Faz İ) =====
    // ⚠️ Yalnızca **eylemi olan** maddeler kuyruğa giriyor: sıralanan sayfanın envanterde
    // karşılığı olmalı ki panel açılabilsin. "Sayfa yok" kovasındaki maddeler bu yüzden
    // kuyruğa GİRMİYOR — uygulama yeni sayfa açamıyor ve eylemsiz madde, kullanıcıyı
    // "Yapıldı" demekten başka bir şey yapamayacağı bir işe yönlendirir (`Leak`'te bu
    // bedel yaşandı, ağırlığı o yüzden 0,6).
    for g in &report.content_gaps {
        let slug = seo_core::page_kind::last_segment(&g.page).unwrap_or_default();
        if slug.is_empty() || !input.active_slugs.contains(&slug) {
            continue;
        }
        out.push(Candidate {
            reference: ItemRef::Query(g.query.clone()),
            bucket: Bucket::Content,
            title: g.query.clone(),
            reason: format!(
                "{} · {} sayfası sıralanıyor, {:.0} gösterimde konum {:.1}",
                g.kind.reason(),
                g.page_kind.label().to_lowercase(),
                g.impressions,
                g.position
            ),
            clicks: g.missed_clicks,
            page: "contentgap".into(),
            // ⚠️ Ekrandaki satır kimliği `query + page` — `ContentGapPage` böyle üretiyor.
            // Ayrışırsa odaklama sessizce çalışmaz (0b3'te yaşanan kaydırma hatasının sınıfı).
            focus_id: format!("{}{}", g.query, g.page),
            minutes: sure(Bucket::Content, 12).0,
            minutes_measured: sure(Bucket::Content, 12).1,
        });
    }

    // ⚠️ Süzgeç EN SONDA ve `pick`ten ÖNCE: kova sayaçları da süzülmüş listeyi saymalı.
    // "Kaçak trafikte 2.100 aday var" derken ölçümü uçuşta olanları sayarsak kova kendi
    // gerçeğini abartır — sayaç boş kovanın neden boş olduğunu açıklamak için var.
    queue::drop_in_flight(out, in_flight)
}

/// Bugünün kuyruğu. **Hesaplanıyor, saklanmıyor** — bkz. `seo_core::queue` modül başlığı.
#[tauri::command]
pub fn get_today_queue(state: State<'_, AppState>, shared: Option<SharedTodayInputs>) -> Result<TodayQueue, String> {
    let conn = state.conn.lock().unwrap();
    match shared {
        Some(input) => Ok(build_shared_today_queue(&conn, &input)),
        None => build_today_queue(&conn),
    }
}

fn store_page_target(pages: &[SharedTodayPage], slug: &str) -> Option<(String, String)> {
    let mut matches = pages.iter().filter(|page| page.status == 1 && page.slug.to_lowercase() == slug);
    let page = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    let screen = match page.kind.as_str() {
        "category" => "categories",
        "brand" => "brands",
        "blog" => "blogs",
        _ => return None,
    };
    Some((screen.into(), page.remote_id.to_string()))
}

type DecisionKeys = HashSet<ItemRef>;

fn shared_decisions(input: &SharedTodayInputs, analyzed_at: &str, now: NaiveDateTime)
    -> (DecisionKeys, DecisionKeys, DecisionKeys) {
    let mut hidden = HashSet::new();
    let mut done = HashSet::new();
    let mut in_flight = HashSet::new();
    let today = now.date();
    for decision in &input.decisions {
        let reference = ref_from(&decision.kind, decision.reference.clone());
        match decision.action.as_str() {
            "dismiss" => { hidden.insert(reference); }
            "snooze" if decision.until_day.as_deref().and_then(calendar_date).is_some_and(|until| until > today) => {
                hidden.insert(reference);
            }
            "done" => {
                let Some(at) = utc_to_local(&decision.occurred_at) else { continue };
                if at.date() == today && decision.analyzed_at.as_deref().unwrap_or_default() == analyzed_at {
                    done.insert(reference);
                } else if at > now - Duration::days(queue::REVIEW_AFTER_DAYS) {
                    in_flight.insert(reference);
                }
            }
            _ => {}
        }
    }
    (hidden, done, in_flight)
}

fn shared_contact_work(
    conn: &Connection,
    contacts: &[SharedTodayContact],
    done: &DecisionKeys,
    today: NaiveDate,
) -> Vec<(String, DueContact)> {
    let silence_limit = super::setting_i64(conn, "contact_silence_days", 0);
    contacts.iter().filter(|contact| !contact.archived).filter_map(|contact| {
        let days = |date: NaiveDate| (today - date).num_days();
        let next_step = contact.next_step_at.as_deref().and_then(calendar_date);
        let overdue = next_step.map(days);
        let silence = contact.last_contact_at.as_deref().and_then(utc_to_local)
            .map(|stamp| days(stamp.date())).filter(|days| *days >= silence_limit && silence_limit > 0);
        let marked = done.contains(&ItemRef::Contact(contact.id.clone()));
        let (overdue_days, silent_days) = match (overdue, silence) {
            (Some(days), _) if days >= 0 || marked => (days.max(0), 0),
            (None, Some(days)) => (0, days),
            _ if marked => (0, 0),
            _ => return None,
        };
        let due = DueContact {
            id: 0, // UUID ekip kimliği dışarıda taşınır. DueContact yalnızca metinleri biçimler.
            name: contact.name.clone(),
            company: contact.company.clone(),
            note: if silent_days == 0 { contact.next_step_note.clone() } else { String::new() },
            overdue_days,
            silent_days,
        };
        Some((contact.id.clone(), due))
    }).collect()
}

fn shared_store_events(
    input: &SharedTodayInputs,
    done: &DecisionKeys,
    today: NaiveDate,
) -> Vec<(ItemRef, String)> {
    let products: HashSet<_> = input.products.iter().map(|product| product.sku.as_str()).collect();
    let mut latest = HashMap::<ItemRef, NaiveDateTime>::new();
    for event in input.events.iter().filter(|event| event.reaches_store) {
        let Some(at) = utc_to_local(&event.at) else { continue };
        let reference = match &event.sku {
            Some(sku) if products.contains(sku.as_str()) => ItemRef::Product(sku.clone()),
            Some(_) => continue,
            None => {
                let Some(slug) = event.url.as_deref().and_then(page_kind::last_segment) else { continue };
                if store_page_target(&input.pages, &slug).is_none() {
                    continue;
                }
                ItemRef::Page(slug)
            }
        };
        // Aynı aksiyonun yazdığı olay, bugün tamamlanan maddeyi kuyruktan erken düşürmesin.
        if event.kind == "manual_done" && at.date() == today && done.contains(&reference) {
            continue;
        }
        latest.entry(reference).and_modify(|previous| *previous = (*previous).max(at)).or_insert(at);
    }
    let mut events: Vec<_> = latest.into_iter().map(|(reference, at)| (reference, at.to_string())).collect();
    events.sort_by_cached_key(|(reference, _)| ref_key(reference));
    events
}

pub(super) fn build_shared_today_queue(conn: &Connection, shared: &SharedTodayInputs) -> TodayQueue {
    let now = Local::now().naive_local();
    let empty_report = OpportunityReport::default();
    let report = shared.report.as_ref().unwrap_or(&empty_report);
    let (hidden, done, mut in_flight) = shared_decisions(shared, &report.analyzed_at, now);
    let events = shared_store_events(shared, &done, now.date());
    in_flight.extend(events.iter()
        .filter(|(reference, at)| days_since(at) < queue::REVIEW_AFTER_DAYS && !done.contains(reference))
        .map(|(reference, _)| reference.clone()));
    let candidates_data = CandidateData {
        products: &shared.products,
        events: &events,
        decided_pages: HashSet::new(),
        active_slugs: shared.pages.iter().filter(|page| page.status == 1).map(|page| page.slug.to_lowercase()).collect(),
        contacts: shared_contact_work(conn, &shared.contacts, &done, now.date()),
        shared: Some(shared),
    };
    let all = build_candidates(conn, report, &in_flight, candidates_data);
    let oldest = events.iter().map(|(_, at)| at.as_str()).min();
    assemble_today_queue(&report.analyzed_at, all, in_flight.len(), hidden, done, oldest)
}

/// Kuyruğu kurar — **bağlantıyı kilitlemeden**.
///
/// ⚠️ Ayrı bir fonksiyon çünkü odak seansı da (Faz S) aynı kuyruğu okuyor ve kilidi kendisi
/// tutuyor. İki yerde iki farklı kuyruk kurulumu olsaydı seansın kilitlediği iş ile ekrandaki
/// liste ayrışabilirdi.
pub fn build_today_queue(conn: &Connection) -> Result<TodayQueue, String> {
    let raw = db::get_setting(conn, "opportunity_json")?;
    let report: OpportunityReport = match raw {
        Some(j) => serde_json::from_str(&j).unwrap_or_default(),
        None => OpportunityReport::default(),
    };

    let ucus = in_flight(conn, &report.analyzed_at);
    let all = candidates(conn, &report, &ucus);
    let oldest: Option<String> = conn.query_row(
        "SELECT MIN(at) FROM work_events WHERE reaches_store = 1",
        [],
        |row| row.get(0),
    ).ok().flatten();
    let keys = |decisions: Vec<(String, String)>| decisions.into_iter().map(|(kind, r)| ref_from(&kind, r)).collect();
    Ok(assemble_today_queue(
        &report.analyzed_at,
        all,
        ucus.len(),
        keys(hidden(conn)),
        keys(completed(conn, &report.analyzed_at)),
        oldest.as_deref(),
    ))
}

fn assemble_today_queue(
    analyzed_at: &str,
    all: Vec<Candidate>,
    in_flight: usize,
    hidden: DecisionKeys,
    done: DecisionKeys,
    oldest_store_event: Option<&str>,
) -> TodayQueue {
    let mut bucket_counts: Vec<BucketCount> = Vec::new();
    for b in [
        Bucket::Urgent,
        Bucket::Leverage,
        Bucket::Leak,
        Bucket::Review,
        Bucket::Upkeep,
        Bucket::Contact,
        Bucket::Content,
    ] {
        bucket_counts.push(BucketCount {
            bucket: b,
            label: b.label().into(),
            candidates: all.iter().filter(|c| c.bucket == b).count(),
        });
    }

    // ⚠️ İKİ AYRI liste: gizlenenler kuyruktan çıkar, "yapıldı" olanlar YERİNDE KALIR.
    // Anında düşürmek günü bitmez kılıyordu (saha geri bildirimi) — bkz. `completed`.
    let kalan: Vec<Candidate> = all.into_iter().filter(|c| !hidden.contains(&c.reference)).collect();

    // Sonuç kontrolü kovası boşsa en erken ne zaman dolacak? En yeni gönderimden değil,
    // EN ESKİ ölçülmemiş gönderimden hesaplanıyor — ilk dolacak olan o.
    let review_ready_at = if bucket_counts.iter().any(|b| b.bucket == Bucket::Review && b.candidates > 0) {
        String::new()
    } else {
        oldest_store_event.and_then(calendar_date)
            .map(|day| (day + Duration::days(queue::REVIEW_AFTER_DAYS)).to_string()).unwrap_or_default()
    };

    let mut items = queue::pick(kalan);
    for it in &mut items {
        it.done = done.contains(&it.reference);
    }
    let done_count = items.iter().filter(|i| i.done).count();

    TodayQueue {
        items,
        analyzed_at: analyzed_at.into(),
        hidden: hidden.len(),
        in_flight,
        done_count,
        bucket_counts,
        review_ready_at,
    }
}

/// Maddeyi kuyruktan çıkarır. `until` yoksa kalıcı gizleme, varsa o tarihe kadar erteleme.
///
/// ⚠️ Kalıcı gizleme **veriyi silmiyor** — ürün kendi ekranında duruyor. Kullanıcı kararı
/// yalnızca "bunu bana her sabah gösterme" demek. Satışta olmayanlar ekranı zaten bunu
/// söylüyordu: *"bazı sayfaları bilinçli tutuyor olabilirsiniz — liste öneridir, karar sizin"*.
#[tauri::command]
pub fn dismiss_queue_item(
    state: State<'_, AppState>,
    kind: String,
    reference: String,
    until: Option<String>,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO queue_dismissals (kind, ref, until, at, done_at_analysis)
         VALUES (?1,?2,?3,?4,NULL)
         ON CONFLICT(kind, ref) DO UPDATE SET until = ?3, at = ?4, done_at_analysis = NULL",
        params![kind, reference, until, now_str()],
    )
    .map_err(|e| format!("Madde gizlenemedi: {e}"))?;
    Ok(())
}

/// Kuyruk maddesini **yapıldı** olarak işaretler.
///
/// İki şey birden yapıyor ve ikincisi asıl değerli olan:
///
/// 1. Maddeyi kuyruktan çıkarır — ama **sonraki analize kadar** (`done_at_analysis`).
///    Kalıcı gizleme değil: iş işe yaramadıysa madde geri gelmeli.
/// 2. 🔑 **Ölçüm olayı yazar.** Faz Ö'nün dürüstçe itiraf ettiği boşluk buydu: *"içeriği elle
///    kopyalayıp mağazaya yapıştıran kullanıcı için olay oluşmuyor → o ürün ölçülemiyor"*.
///    Kullanıcının yapamadığı işleri (301 yönlendirme IdeaSoft panelinden tanımlanıyor,
///    uygulama yapamıyor) artık ölçüme sokabiliyoruz.
///
/// ⚠️ `reaches_store = 1` ama `kind = "manual_done"`: bu bir **beyan**, uygulamanın
/// doğruladığı bir gönderim değil. Zaman çizelgesi ikisini ayrı etiketliyor
/// ("Elle yapıldı olarak işaretlendi" ↔ "IdeaSoft'a gönderildi") — kullanıcı 3 hafta sonra
/// sonuca bakarken neye dayandığını bilmeli.
///
/// Yalnızca ÜRÜN maddeleri ölçülebiliyor: olay günlüğü sku'ya bağlı, satışta olmayan
/// sayfaların sku'su yok. Sayfa maddelerinde madde yine kuyruktan çıkıyor, olay yazılmıyor.
#[tauri::command]
pub fn complete_queue_item(
    state: State<'_, AppState>,
    kind: String,
    reference: String,
    record_store_change: Option<bool>,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let analyzed_at: String = db::get_setting(&conn, "opportunity_json")?
        .and_then(|j| serde_json::from_str::<OpportunityReport>(&j).ok())
        .map(|r| r.analyzed_at)
        .unwrap_or_default();

    conn.execute(
        "INSERT INTO queue_dismissals (kind, ref, until, at, done_at_analysis)
         VALUES (?1,?2,NULL,?3,?4)
         ON CONFLICT(kind, ref) DO UPDATE SET until = NULL, at = ?3, done_at_analysis = ?4",
        params![kind, reference, now_str(), analyzed_at],
    )
    .map_err(|e| format!("Madde işaretlenemedi: {e}"))?;

    if kind == "product" {
        if record_store_change.unwrap_or(true) {
            super::log_event(&conn, &reference, "manual_done", true);
        }
        // 🔴 Feed bayrağı da kapanıyor (saha hatası, 2026-08-10). Acil kovası canlı veriden
        // besleniyor ve uçuş süzgecinden MUAF — haklı olarak: feed yarın yine değişirse madde
        // yeniden çıkmalı. Ama "yapıldı" bayrağı temizlemeyince madde her gün geri geliyordu.
        //
        // ⚠️ Uçuş süzgeciyle aynı fikir değil, TERSİ: orada kanıt beklendiği için madde
        // susturuluyor; burada kanıtı uygulamanın kendisi tutuyor, o yüzden VERİ düzeltiliyor.
        // Faz C'de aynı ilke: dönüş yapılınca kişinin sonraki adımı temizleniyor.
        let bayrak: Option<String> = conn
            .query_row("SELECT feed_changed FROM products WHERE sku = ?1", [&reference], |r| r.get(0))
            .ok()
            .flatten();
        if bayrak.is_some() {
            super::mark_reviewed(&conn, &reference)?;
        }
    }
    // ⚠️ Müşteri maddesi `work_events`e YAZILMIYOR: o günlük sku-anahtarlı ve `reaches_store`
    // eksenli, bir telefon görüşmesi oraya ait değil. CRM'in kendi günlüğü `contact_events`.
    // Dönüş yapıldığı için sonraki adım da temizleniyor — madde kuyruktan gizlenerek değil,
    // VERİSİ DÜZELDİĞİ için düşüyor.
    if kind == "contact" {
        if let Ok(id) = reference.parse::<i64>() {
            super::complete_contact_followup(&conn, id)?;
        }
    }
    Ok(())
}

/// Tek bir maddenin kuyruktan çıkarılma kararını geri alır ("geri al").
///
/// ⚠️ Yazılan ölçüm olayı SİLİNMİYOR: `manual_done` olayı olmuş bir şeyin kaydı, kullanıcı
/// işareti geri alsa da o an gerçekten bir iş yapıldığı bilgisi zaman çizelgesinde kalmalı.
/// Geri alınan tek şey maddenin kuyruktaki görünümü.
#[tauri::command]
pub fn restore_queue_item(
    state: State<'_, AppState>,
    kind: String,
    reference: String,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute(
        "DELETE FROM queue_dismissals WHERE kind = ?1 AND ref = ?2",
        params![kind, reference],
    )
    .map_err(|e| format!("Geri alınamadı: {e}"))?;
    Ok(())
}

/// Gizlenmiş/ertelenmiş maddelerin tamamını geri getirir.
#[tauri::command]
pub fn restore_queue_items(state: State<'_, AppState>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute("DELETE FROM queue_dismissals", [])
        .map_err(|e| format!("Geri alınamadı: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_team_without_data_is_not_filled_from_legacy_sqlite() {
        let conn = Connection::open_in_memory().unwrap();
        db::init(&conn).unwrap();
        conn.execute(
            "INSERT INTO products (sku, name) VALUES ('LEGACY','Yerel ürün')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO seo_status (sku, meta_status, details_status) VALUES ('LEGACY','done','done')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO queue_dismissals (kind, ref, at) VALUES ('page','old-page',?1)",
            [now_str()],
        ).unwrap();
        let standalone = build_today_queue(&conn).unwrap();
        assert_eq!((standalone.items.len(), standalone.hidden), (1, 1));

        let shared = build_shared_today_queue(&conn, &SharedTodayInputs::default());
        assert!(shared.items.is_empty());
        assert_eq!((shared.hidden, shared.done_count, shared.in_flight), (0, 0, 0));
    }

    #[test]
    fn team_queue_uses_uuid_followups_shared_decisions_and_catalog_review_links() {
        let conn = Connection::open_in_memory().unwrap();
        db::init(&conn).unwrap();
        let now = Utc::now();
        let today = now.with_timezone(&Local).date_naive();
        let review_at = (now - Duration::days(queue::REVIEW_AFTER_DAYS + 1)).to_rfc3339();
        let report = "2026-11-21T10:20:30";
        let due_id = "00000000-0000-0000-0000-000000000111";
        let done_id = "00000000-0000-0000-0000-000000000222";
        let future_id = "00000000-0000-0000-0000-000000000333";
        let scoped = serde_json::from_value(json!({
            "report": { "analyzed_at": report, "eol": [
                { "slug": "snoozed", "url": "https://example.com/urun/snoozed", "clicks": 100, "impressions": 1000, "position": 2 },
                { "slug": "restored", "url": "https://example.com/urun/restored", "clicks": 100, "impressions": 1000, "position": 2 }
            ] },
            "products": [
                { "sku": "PUBLISHED", "name": "Ürün", "meta_done": true, "details_done": true },
                { "sku": "OFFLINE_DONE", "name": "Çevrimdışı tamamlandı", "meta_done": true, "details_done": true }
            ],
            "events": [
                { "sku": "PUBLISHED", "url": "https://example.com/urun/urun", "at": now.to_rfc3339(), "kind": "manual_done", "reaches_store": true },
                { "url": "https://example.com/kategori/kuyruk", "at": review_at, "kind": "store_page_publish", "reaches_store": true },
                { "url": "https://example.com/kategori/ambiguous", "at": review_at, "kind": "store_page_publish", "reaches_store": true },
                { "sku": "REMOVED", "url": "https://example.com/urun/removed", "at": review_at, "kind": "ideasoft_push", "reaches_store": true }
            ],
            "pages": [
                { "kind": "category", "slug": "kuyruk", "remote_id": 123, "status": 1 },
                { "kind": "category", "slug": "ambiguous", "remote_id": 124, "status": 1 },
                { "kind": "brand", "slug": "ambiguous", "remote_id": 125, "status": 1 }
            ],
            "contacts": [
                { "id": due_id, "name": "Dönülecek", "company": "Firma", "next_step_at": today.to_string(), "next_step_note": "Teklif", "archived": false },
                { "id": done_id, "name": "Tamamlandı", "company": "", "next_step_note": "", "archived": false },
                { "id": future_id, "name": "İleride", "company": "", "next_step_at": (today + Duration::days(1)).to_string(), "next_step_note": "", "archived": false }
            ],
            "decisions": [
                { "kind": "product", "reference": "PUBLISHED", "action": "done", "occurred_at": now.to_rfc3339(), "analyzed_at": report },
                { "kind": "product", "reference": "OFFLINE_DONE", "action": "done", "occurred_at": (now - Duration::days(1)).to_rfc3339(), "analyzed_at": report },
                { "kind": "contact", "reference": done_id, "action": "done", "occurred_at": now.naive_utc().to_string(), "analyzed_at": report },
                { "kind": "page", "reference": "snoozed", "action": "snooze", "occurred_at": now.to_rfc3339(), "until_day": (today + Duration::days(1)).to_string() },
                { "kind": "page", "reference": "restored", "action": "restore", "occurred_at": now.to_rfc3339() }
            ]
        })).unwrap();

        let queue = build_shared_today_queue(&conn, &scoped);
        let item = |key| queue.items.iter().find(|item| item.reference.key() == key);
        assert_eq!((queue.hidden, queue.done_count), (1, 2));
        assert_eq!(item(due_id).unwrap().focus_id, due_id);
        assert!(item(done_id).unwrap().done);
        assert!(item("PUBLISHED").unwrap().done);
        let review = item("kuyruk").unwrap();
        assert_eq!(review.bucket, Bucket::Review);
        assert_eq!((review.page.as_str(), review.focus_id.as_str()), ("categories", "123"));
        assert!(item("restored").is_some());
        for omitted in [future_id, "snoozed", "REMOVED", "ambiguous", "OFFLINE_DONE"] {
            assert!(item(omitted).is_none(), "{omitted}");
        }
    }

    /// Üç çıkarma biçiminin **ayrıştığını** sabitler.
    ///
    /// ⚠️ Asıl korunan davranış sonuncusu: **"yapıldı" kalıcı değil.** Analiz yenilendiğinde
    /// işaret düşer; iş gerçekten işe yaradıysa madde zaten yeni raporda çıkmaz, yaramadıysa
    /// geri gelir. Kalıcı gizleseydik çözülmemiş bir iş sessizce kaybolurdu.
    #[test]
    fn yapildi_isareti_sonraki_analizde_dusuyor() {
        let conn = Connection::open_in_memory().unwrap();
        seo_core::db::init(&conn).unwrap();
        let bugun = now_str();
        let yarin = (chrono::Local::now().date_naive() + chrono::Duration::days(1)).to_string();

        conn.execute(
            "INSERT INTO queue_dismissals (kind, ref, until, at, done_at_analysis)
             VALUES ('product','KALICI',NULL,?1,NULL),
                    ('product','ERTELI',?2,?1,NULL),
                    ('product','YAPILDI',NULL,?1,'2026-08-07T21:27:10')",
            params![bugun, yarin],
        )
        .unwrap();

        // Gizlenenler: kalıcı + ertelenmiş. ⚠️ "Yapıldı" burada OLMAMALI — o madde listede
        // kalıp üstü çizili gösteriliyor (gün bitebilsin diye).
        let g: Vec<String> = hidden(&conn).into_iter().map(|(_, r)| r).collect();
        assert_eq!(g.len(), 2, "gizlenenler yanlış: {g:?}");
        assert!(g.iter().any(|r| r == "KALICI") && g.iter().any(|r| r == "ERTELI"));
        assert!(
            !g.iter().any(|r| r == "YAPILDI"),
            "'yapıldı' kuyruktan çıkarılmış — listede kalmalıydı"
        );

        // "Yapıldı" yalnızca İŞARETLENDİĞİ analiz için geçerli.
        assert_eq!(completed(&conn, "2026-08-07T21:27:10").len(), 1);
        assert_eq!(
            completed(&conn, "2026-08-20T10:00:00").len(),
            0,
            "yeni analizde 'yapıldı' işareti düşmeliydi"
        );
    }

    /// 🔴 Saha hatası (2026-08-10, ikinci tur): *"daha önce yapıldı olarak işaretlediğim
    /// işlerin bazıları yeniden listeleniyor."* Uçuş süzgeci **acil kovasını kapsamıyor**
    /// (haklı olarak: feed yarın yine değişirse madde çıkmalı), ama "yapıldı" acil koşulunu
    /// da kaldırmıyordu — madde her gün geri geliyordu.
    ///
    /// Ölçüm (kullanıcının veritabanı): 9 işaretli üründen **3'ü** acil koşulunu hâlâ
    /// sağlıyordu — 2'sinde feed bayrağı duruyordu, 1'i "hiç gönderilmemiş" görünüyordu
    /// (oysa `manual_done` beyanı vardı).
    #[test]
    fn yapildi_acil_kosulunu_da_kaldiriyor() {
        let conn = Connection::open_in_memory().unwrap();
        seo_core::db::init(&conn).unwrap();
        conn.execute(
            "INSERT INTO products (sku, name, url, feed_changed, feed_fp)
             VALUES ('BAYRAKLI','Feed değişti','https://x/a','açıklama','fp1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO seo_status (sku, meta_status, details_status, ideasoft_pushed_at)
             VALUES ('BAYRAKLI','done','done','2026-08-01T10:00:00')",
            [],
        )
        .unwrap();

        let bos = OpportunityReport::default();
        let acil = |c: &Connection| {
            candidates(c, &bos, &Default::default())
                .iter()
                .filter(|x| x.bucket == Bucket::Urgent)
                .count()
        };
        assert_eq!(acil(&conn), 1, "feed bayrağı duruyor, madde acil");

        // "Yapıldı" → bayrak temizleniyor, koşul ortadan kalkıyor.
        super::super::mark_reviewed(&conn, "BAYRAKLI").unwrap();
        assert_eq!(acil(&conn), 0, "yapıldı dendikten sonra madde geri gelmemeli");
    }

    /// 🔑 "Hiç gönderilmemiş" koşulu kullanıcının **beyanını** da saymalı: içeriği elle
    /// mağazaya yapıştıran kullanıcının `ideasoft_pushed_at`i hiç dolmuyor.
    #[test]
    fn elle_yapildi_beyani_magazaya_ulasmis_sayiliyor() {
        let conn = Connection::open_in_memory().unwrap();
        seo_core::db::init(&conn).unwrap();
        conn.execute(
            "INSERT INTO products (sku, name, url) VALUES ('ELLE','Elle gönderilen','https://x/b')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO seo_status (sku, meta_status, details_status)
             VALUES ('ELLE','done','done')",
            [],
        )
        .unwrap();

        let bos = OpportunityReport::default();
        let acil = |c: &Connection| {
            candidates(c, &bos, &Default::default())
                .iter()
                .filter(|x| x.bucket == Bucket::Urgent)
                .count()
        };
        assert_eq!(acil(&conn), 1, "içerik hazır ama mağazaya ulaşmamış");

        super::super::log_event(&conn, "ELLE", "manual_done", true);
        assert_eq!(acil(&conn), 0, "kullanıcı beyanı gönderim yerine geçmeli");
    }

    /// 🔴 Saha hatası (2026-08-10): *"Bugün sayfasında hâlâ önceki günün yapıldı olarak
    /// işaretlenmiş işleri mevcut."* İşaret analize bağlıydı; analiz yenilenmediği sürece
    /// dünkü bitmiş işler ekranda kalıyordu.
    ///
    /// Test iki ucu birden tutuyor: bugünkü işaret **geçerli**, dünkü **değil** — ve dünkü
    /// madde geri gelmiyor, uçuş süzgeci devralıyor.
    #[test]
    fn yapildi_isareti_yalnizca_bugun_gecerli() {
        let conn = Connection::open_in_memory().unwrap();
        seo_core::db::init(&conn).unwrap();
        let analiz = "2026-08-09T18:11:07";
        let dun = (chrono::Local::now() - chrono::Duration::days(1))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();

        conn.execute(
            "INSERT INTO queue_dismissals (kind, ref, until, at, done_at_analysis)
             VALUES ('product','BUGUN',NULL,?1,?3), ('product','DUN',NULL,?2,?3)",
            params![now_str(), dun, analiz],
        )
        .unwrap();

        let bugunku: Vec<String> = completed(&conn, analiz).into_iter().map(|(_, r)| r).collect();
        assert_eq!(bugunku, vec!["BUGUN"], "dünkü işaret bugünün listesinde durmamalı");

        // ⚠️ Dünkü madde geri GELMİYOR: uçuş süzgeci onu devralıyor.
        let uctakiler = in_flight(&conn, analiz);
        assert!(uctakiler.contains(&ItemRef::Product("DUN".into())));
        assert!(
            !uctakiler.contains(&ItemRef::Product("BUGUN".into())),
            "bugün işaretlenen madde listede kalmalı (üstü çizili)"
        );
    }

    /// 🔴 Saha hatası (2026-08-09): yapılan iş, analiz yenilenince kuyruğa geri geliyordu.
    ///
    /// Test iki zaman ölçeğinin **ayrı** kaldığını sabitliyor — bu ikisi karışırsa iki ayrı
    /// hata geri gelir: bu analizde işaretlenen madde düşerse gün bitmez (08-08 hatası),
    /// önceki analizde işaretlenen madde susmazsa yapılan iş geri gelir (08-09 hatası).
    #[test]
    fn ucus_suzgeci_bu_analizi_degil_oncekileri_susturuyor() {
        let conn = Connection::open_in_memory().unwrap();
        seo_core::db::init(&conn).unwrap();
        let simdi = now_str();
        let bu_analiz = "2026-08-09T18:11:07";

        conn.execute(
            "INSERT INTO queue_dismissals (kind, ref, until, at, done_at_analysis)
             VALUES ('product','BU_ANALIZDE',NULL,?1,?2),
                    ('page','ONCEKI_ANALIZDE',NULL,?1,'2026-08-07T21:27:10')",
            params![simdi, bu_analiz],
        )
        .unwrap();

        let uctakiler = in_flight(&conn, bu_analiz);
        assert!(
            uctakiler.contains(&ItemRef::Page("ONCEKI_ANALIZDE".into())),
            "önceki analizde yapılan iş susturulmalıydı"
        );
        assert!(
            !uctakiler.contains(&ItemRef::Product("BU_ANALIZDE".into())),
            "bu analizde işaretlenen madde listede kalmalı (üstü çizili) — düşerse gün bitmez"
        );
    }

    /// Mağazaya ulaşan iş 28 günü doldurunca susmayı bırakıp sonuç kontrolüne geçiyor.
    ///
    /// ⚠️ İki uç aynı sorgudan (`store_events`) okuyor; eşikler ayrışırsa madde ikisinin
    /// arasına düşüp tamamen kaybolur.
    #[test]
    fn ucus_suresi_dolunca_madde_sonuc_kontrolune_geciyor() {
        let conn = Connection::open_in_memory().unwrap();
        seo_core::db::init(&conn).unwrap();
        let gun = |n: i64| {
            (chrono::Local::now() - chrono::Duration::days(n))
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string()
        };
        conn.execute(
            "INSERT INTO work_events (at, sku, kind, reaches_store)
             VALUES (?1,'TAZE','manual_done',1), (?2,'ESKI','ideasoft_push',1)",
            params![gun(3), gun(queue::REVIEW_AFTER_DAYS + 1)],
        )
        .unwrap();

        let uctakiler = in_flight(&conn, "analiz");
        assert!(uctakiler.contains(&ItemRef::Product("TAZE".into())), "3 günlük iş uçuşta");
        assert!(
            !uctakiler.contains(&ItemRef::Product("ESKI".into())),
            "29 günlük iş artık uçuşta değil — sonuç kontrolü kovası onu getirecek"
        );

        let adaylar = candidates(&conn, &OpportunityReport::default(), &uctakiler);
        let sonuc: Vec<&str> = adaylar
            .iter()
            .filter(|c| c.bucket == Bucket::Review)
            .map(|c| c.reference.key())
            .collect();
        assert_eq!(sonuc, vec!["ESKI"], "yalnızca süresi dolan madde sonuç kontrolünde");
    }

    /// 🔴 08-08 hatasının CRM'de tekrarlamaması: "yapıldı" denen kişi sonraki adımı
    /// temizlediği için adaylıktan düşerdi, yerine 11. madde gelirdi.
    #[test]
    fn yapildi_denen_musteri_gunun_listesinde_kaliyor() {
        let conn = Connection::open_in_memory().unwrap();
        seo_core::db::init(&conn).unwrap();
        let bu_analiz = "2026-08-09T18:11:07";
        conn.execute(
            "INSERT INTO contacts (id, name, company, created_at, updated_at)
             VALUES (7,'Ahmet Yılmaz','Kurumsal BT',?1,?1)",
            params![now_str()],
        )
        .unwrap();
        // Sonraki adımı YOK (dönüş yapılınca temizlendi) ama bu analizde yapıldı işaretli.
        conn.execute(
            "INSERT INTO queue_dismissals (kind, ref, until, at, done_at_analysis)
             VALUES ('contact','7',NULL,?1,?2)",
            params![now_str(), bu_analiz],
        )
        .unwrap();

        let mut report = OpportunityReport::default();
        report.analyzed_at = bu_analiz.into();
        let adaylar = candidates(&conn, &report, &Default::default());
        let musteri: Vec<&str> = adaylar
            .iter()
            .filter(|c| c.bucket == Bucket::Contact)
            .map(|c| c.title.as_str())
            .collect();
        assert_eq!(musteri, vec!["Ahmet Yılmaz · Kurumsal BT"]);
    }

    /// Kararı verilmiş EOL sayfası kaçak kovasında iş çıkarmıyor (Faz D karar deposu).
    #[test]
    fn karari_verilmis_eol_sayfasi_kacak_kovasindan_cikiyor() {
        let conn = Connection::open_in_memory().unwrap();
        seo_core::db::init(&conn).unwrap();
        conn.execute(
            "INSERT INTO eol_decisions (slug, url, action, source, decided_at)
             VALUES ('yonlendirildi','u','redirect_301','manual',?1),
                    ('bilincli-tutuluyor','u','keep','manual',?1)",
            params![now_str()],
        )
        .unwrap();

        let mut report = OpportunityReport::default();
        for slug in ["yonlendirildi", "bilincli-tutuluyor", "karar-yok"] {
            report.eol.push(seo_core::opportunity::EolPage {
                slug: slug.into(),
                url: format!("https://x/{slug}"),
                clicks: 100.0,
                impressions: 1000.0,
                position: 5.0,
            });
        }

        let adaylar = candidates(&conn, &report, &Default::default());
        let kalan: Vec<&str> = adaylar
            .iter()
            .filter(|c| c.bucket == Bucket::Leak)
            .map(|c| c.reference.key())
            .collect();
        assert_eq!(
            kalan,
            vec!["karar-yok"],
            "301'i de bilinçli tutmayı da karar sayıyoruz — ikisi de iş değil"
        );
    }

    /// Gerçek veritabanı KOPYASI üzerinde kuyruğun ne ürettiğini ölçer.
    ///
    /// `SEO_DB_COPY=/tmp/kopya.db cargo test kuyruk_real -- --ignored --nocapture`
    ///
    /// Planın **asıl doğrulaması**: kuyruk tek bir kovaya saplanıyor mu, aynı ürün birden
    /// çok kez giriyor mu, "neden" cümleleri gerçek metrik taşıyor mu.
    #[test]
    #[ignore]
    fn kuyruk_real() {
        let db = std::env::var("SEO_DB_COPY").expect("SEO_DB_COPY yok");
        let conn = Connection::open(&db).unwrap();
        seo_core::db::init(&conn).unwrap();
        let raw = seo_core::db::get_setting(&conn, "opportunity_json").unwrap().unwrap();
        let report: OpportunityReport = serde_json::from_str(&raw).unwrap();

        let all = candidates(&conn, &report, &in_flight(&conn, &report.analyzed_at));
        println!("aday havuzu: {}", all.len());
        for b in
            [Bucket::Urgent, Bucket::Leverage, Bucket::Leak, Bucket::Review, Bucket::Upkeep, Bucket::Contact]
        {
            println!("  {:16} {}", b.label(), all.iter().filter(|c| c.bucket == b).count());
        }

        let q = queue::pick(all);
        println!("\nkuyruk ({} madde):", q.len());
        for (i, it) in q.iter().enumerate() {
            println!(
                "{:2}. [{:14}] {:6.1}  {:.44}\n     → {}",
                i + 1,
                it.bucket.label(),
                it.score,
                it.title,
                it.reason
            );
            if !it.also.is_empty() {
                println!("     ayrıca: {}", it.also.join(" · "));
            }
        }

        // Tekrar eden ürün olmamalı (ölçüm: 12 ürün 2+ kovada).
        let mut kimlikler: Vec<_> = q.iter().map(|i| ref_key(&i.reference)).collect();
        let onceki = kimlikler.len();
        kimlikler.sort();
        kimlikler.dedup();
        assert_eq!(onceki, kimlikler.len(), "aynı madde kuyruğa iki kez girmiş");

        // Kuyruk tek kovaya saplanmamalı.
        let mut kovalar: Vec<_> = q.iter().map(|i| i.bucket).collect();
        kovalar.dedup();
        kovalar.sort_by_key(|b| b.label());
        kovalar.dedup();
        println!("\nkuyruktaki kova sayısı: {}", kovalar.len());
        assert!(kovalar.len() >= 3, "kuyruk {} kovaya saplandı", kovalar.len());
    }

    // ===================== İçerik gönderimlerinin ölçümü (Faz İ, v0.19.1) =====================

    fn bos_db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        seo_core::db::init(&c).unwrap();
        c
    }

    fn olay(c: &Connection, sku: Option<&str>, url: &str, kind: &str, gun_once: i64) {
        c.execute(
            "INSERT INTO work_events (at, sku, url, kind, reaches_store)
             VALUES (datetime('now', ?1), ?2, ?3, ?4, 1)",
            rusqlite::params![format!("-{gun_once} days"), sku, url, kind],
        )
        .unwrap();
    }

    /// 🔴 Asıl düzeltme: sku'suz (içerik) olaylar artık okunuyor. Koşul dururken bu olaylar
    /// yazılıyor ama hiçbir yerde okunmuyordu — sonuç kontrolü kovası geri getirmiyor,
    /// rozet çıkmıyordu. Yani ölçülüyor sanılan bir şey ölçülmüyordu.
    #[test]
    fn icerik_olayi_kimlige_cozuluyor() {
        let c = bos_db();
        olay(&c, Some("SKU.1"), "https://m.com/urun/a", "ideasoft_push", 40);
        olay(&c, None, "https://m.com/kategori/access-point", "store_page_push", 40);

        let ev = store_events(&c);
        assert_eq!(ev.len(), 2, "ürün ve sayfa olayının ikisi de okunmalı");
        assert!(ev.iter().any(|(k, _)| *k == ItemRef::Product("SKU.1".into())));
        // ⚠️ Sayfa kimliği SLUG: `ItemRef::Page` kaçak kovasında da slug taşıyor, aynı
        // uzayda iki biçim bulundurmak sonradan aranacak bir tuzak olurdu.
        assert!(
            ev.iter().any(|(k, _)| *k == ItemRef::Page("access-point".into())),
            "sayfa olayı slug kimliğine çözülmeli: {ev:?}"
        );
    }

    /// 28 günü DOLMAYAN içerik gönderimi uçuşta sayılmalı; DOLAN sayılmamalı.
    /// İki uç aynı sorgudan okuyor — ayrışırlarsa madde ikisinin arasına düşüp kaybolur.
    #[test]
    fn icerik_gonderimi_ucus_suzgecine_giriyor() {
        let c = bos_db();
        olay(&c, None, "https://m.com/kategori/yeni", "store_page_push", 3);
        olay(&c, None, "https://m.com/kategori/eski", "store_page_push", 40);

        let ucusta = in_flight(&c, "2026-08-13T00:00:00");
        assert!(
            ucusta.contains(&ItemRef::Page("yeni".into())),
            "3 gün önceki gönderim uçuşta olmalı"
        );
        assert!(
            !ucusta.contains(&ItemRef::Page("eski".into())),
            "40 gün önceki gönderim uçuştan çıkmış olmalı"
        );
    }

    /// 🔴 Regresyon koruması: aynı sorgu iki kovayı besliyor ve ürün davranışı DEĞİŞMEMELİ.
    #[test]
    fn urun_olaylarinin_davranisi_degismedi() {
        let c = bos_db();
        olay(&c, Some("SKU.A"), "https://m.com/urun/a", "ideasoft_push", 5);
        olay(&c, Some("SKU.B"), "https://m.com/urun/b", "manual_done", 40);

        let ucusta = in_flight(&c, "2026-08-13T00:00:00");
        assert!(ucusta.contains(&ItemRef::Product("SKU.A".into())), "yeni gönderim uçuşta");
        assert!(!ucusta.contains(&ItemRef::Product("SKU.B".into())), "eski gönderim uçuşta değil");

        // En son olay kazanmalı: aynı sku için birden çok gönderim varsa yaş sonuncudan.
        olay(&c, Some("SKU.B"), "https://m.com/urun/b", "ideasoft_push", 1);
        let ucusta2 = in_flight(&c, "2026-08-13T00:00:00");
        assert!(
            ucusta2.contains(&ItemRef::Product("SKU.B".into())),
            "yeni gönderim eski olayı gölgelemeli"
        );
    }
}
