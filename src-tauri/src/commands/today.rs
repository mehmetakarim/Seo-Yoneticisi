//! Bugün kuyruğunun Tauri katmanı — girdileri toplar, `seo_core::queue`e verir.
//!
//! Saf seçim mantığı (skor, tekilleştirme, kova sınırı) `seo_core::queue`de; burada yalnızca
//! veritabanı erişimi ve **kovaların tanımı** var.
//!
//! ⚠️ **Yeni GSC çağrısı YOK.** Beş kovanın dördü mevcut `opportunity_json` önbelleğinden
//! besleniyor. Kuyruk açmak kota harcamamalı; analiz eskiyse ekran kaç gün önce çalıştığını
//! yazıyor (`analyzed_at`), kendiliğinden yenilemiyor.

use super::*;
use seo_core::queue::{self, Bucket, Candidate, ItemRef, QueueItem};

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

/// "Yapıldı" işaretli maddeler — yalnızca **işaretlendiği analiz** için geçerli.
///
/// 🔴 Bu maddeler kuyruktan ÇIKARILMIYOR, yerinde kalıp üstü çizili gösteriliyor. Anında
/// düşürüldüğünde yerlerine yeni aday geliyor, sayaç hep 10'da kalıyor ve gün hiç bitmiyordu
/// (saha geri bildirimi, 2026-08-08).
///
/// Analiz yenilendiğinde işaret düşer: iş işe yaradıysa madde zaten yeni raporda çıkmaz,
/// yaramadıysa geri gelmeli.
fn completed(conn: &Connection, analyzed_at: &str) -> Vec<(String, String)> {
    sorgu(
        conn,
        "SELECT kind, ref FROM queue_dismissals WHERE done_at_analysis = ?1",
        &[&analyzed_at],
    )
}

/// Ürün başına **mağazaya ulaşan** son iş — `(sku, at)`.
///
/// ⚠️ Tek yerde: hem sonuç kontrolü kovası (28 günü DOLANLAR) hem uçuş süzgeci (28 günü
/// DOLMAYANLAR) buradan okuyor. İki ayrı sorgu olsaydı eşikler zamanla ayrışır, madde ikisinin
/// arasına düşüp kaybolabilirdi.
fn store_events(conn: &Connection) -> Vec<(String, String)> {
    sorgu(
        conn,
        "SELECT sku, MAX(at) FROM work_events
         WHERE reaches_store = 1 AND sku IS NOT NULL GROUP BY sku",
        &[],
    )
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
    for (sku, at) in store_events(conn) {
        if days_since(&at) < queue::REVIEW_AFTER_DAYS {
            out.insert(ItemRef::Product(sku));
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
        _ => ItemRef::Product(r),
    }
}

/// Ürün başına feed/gönderim durumu — acil kovasının kaynağı.
struct ProductState {
    sku: String,
    name: String,
    feed_changed: String,
    pushed_at: Option<String>,
    meta_done: bool,
    details_done: bool,
}

fn product_states(conn: &Connection) -> Vec<ProductState> {
    let mut stmt = match conn.prepare(
        "SELECT p.sku, p.name, COALESCE(p.feed_changed,''), s.ideasoft_pushed_at,
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
    let d = at.get(..10).and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    match d {
        Some(d) => (chrono::Local::now().date_naive() - d).num_days(),
        None => 0,
    }
}

/// Beş kovanın adaylarını kurar.
///
/// Her adayın `reason`ı **gerçek bir metrikten** türüyor (yol haritasının şartı): uydurma
/// bir "iyileştirilmeli" cümlesi yok, hepsinde sayı var.
fn candidates(
    conn: &Connection,
    report: &OpportunityReport,
    in_flight: &std::collections::HashSet<ItemRef>,
) -> Vec<Candidate> {
    let mut out = Vec::new();
    // 🔑 Faz S'in ürünü: kova başına ÖLÇÜLMÜŞ süre. Yeterli örnek yoksa (kova başına 5)
    // aşağıdaki elle yazılmış tahminler kullanılmaya devam ediyor.
    let olculen = super::calibration(conn);
    // Ölçülmüş süre varsa onu, yoksa elle yazılan tahmini döndürür.
    let sure = |b: Bucket, tahmin: u32| -> (u32, bool) {
        match olculen.get(&format!("{b:?}").to_lowercase()) {
            Some(dk) => (*dk, true),
            None => (tahmin, false),
        }
    };
    let states = product_states(conn);
    // Ürünün GSC tıklaması — acil maddelerinin skorunu da tıklama sürüyor.
    let clicks_of: std::collections::HashMap<&str, f64> = report
        .opportunities
        .iter()
        .map(|o| (o.sku.as_str(), o.clicks))
        .collect();

    // --- 1) ACİL: canlıda yanlış içerik ---
    for p in &states {
        // ⚠️ Yalnızca METİN alanı değişmişse ve içerik mağazaya gitmişse acil. Görsel
        // değişikliği üretilmiş metni geçersiz kılmıyor (ölçüm: 70 bayrağın 61'i görsel).
        if p.pushed_at.is_some() && queue::is_urgent_change(&p.feed_changed) {
            out.push(Candidate {
                reference: ItemRef::Product(p.sku.clone()),
                bucket: Bucket::Urgent,
                title: p.name.clone(),
                reason: format!(
                    "mağazaya gönderildikten sonra feed değişti ({}) — canlıdaki metin bayat",
                    p.feed_changed
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
        if p.meta_done && p.details_done && p.pushed_at.is_none() {
            out.push(Candidate {
                reference: ItemRef::Product(p.sku.clone()),
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
    for o in &report.opportunities {
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
    let kararli = decided_pages(conn);
    for e in report.eol.iter().filter(|e| !kararli.contains(&e.slug)) {
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
    for (sku, at) in store_events(conn) {
        let yas = days_since(&at);
        if yas < queue::REVIEW_AFTER_DAYS || !gorulen.insert(sku.clone()) {
            continue;
        }
        let ad = states
            .iter()
            .find(|p| p.sku == sku)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| sku.clone());
        out.push(Candidate {
            reference: ItemRef::Product(sku.clone()),
            bucket: Bucket::Review,
            title: ad,
            reason: format!("{yas} gün önce mağazaya gönderildi — sonucuna bakılabilir"),
            clicks: 0.0,
            page: "products".into(),
            focus_id: sku,
            minutes: sure(Bucket::Review, 1).0,
            minutes_measured: sure(Bucket::Review, 1).1,
        });
    }

    // --- 5) BAKIM: düşüşte olanlar + yarışan sayfalar ---
    for d in &report.decay {
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
    let mut kisiler = super::due_contacts(conn);
    // ⚠️ Bu analizde "yapıldı" denen kişiler listeye GERİ konuyor: dönüş yapılınca sonraki
    // adım temizleniyor ve kişi adaylıktan tamamen düşerdi — yerine 11. madde gelir, 08-08'de
    // düzeltilen "gün hiç bitmiyor" hatası CRM tarafında yeniden doğardı.
    let mevcut: std::collections::HashSet<i64> = kisiler.iter().map(|k| k.id).collect();
    let yapilan_kisiler: Vec<i64> = completed(conn, &report.analyzed_at)
        .into_iter()
        .filter(|(kind, _)| kind == "contact")
        .filter_map(|(_, r)| r.parse::<i64>().ok())
        .filter(|id| !mevcut.contains(id))
        .collect();
    kisiler.extend(super::contacts_by_ids(conn, &yapilan_kisiler));

    for k in kisiler {
        out.push(Candidate {
            reference: ItemRef::Contact(k.id.to_string()),
            bucket: Bucket::Contact,
            title: k.title(),
            reason: k.reason(),
            // ⚠️ `clicks` alanı bu kovada GECİKME GÜNÜ taşıyor (bkz. `queue::score`).
            clicks: k.overdue_days as f64,
            page: "contacts".into(),
            focus_id: k.id.to_string(),
            minutes: sure(Bucket::Contact, 5).0,
            minutes_measured: sure(Bucket::Contact, 5).1,
        });
    }

    // ⚠️ Süzgeç EN SONDA ve `pick`ten ÖNCE: kova sayaçları da süzülmüş listeyi saymalı.
    // "Kaçak trafikte 2.100 aday var" derken ölçümü uçuşta olanları sayarsak kova kendi
    // gerçeğini abartır — sayaç boş kovanın neden boş olduğunu açıklamak için var.
    queue::drop_in_flight(out, in_flight)
}

/// Bugünün kuyruğu. **Hesaplanıyor, saklanmıyor** — bkz. `seo_core::queue` modül başlığı.
#[tauri::command]
pub fn get_today_queue(state: State<'_, AppState>) -> Result<TodayQueue, String> {
    let conn = state.conn.lock().unwrap();
    build_today_queue(&conn)
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
    let mut bucket_counts: Vec<BucketCount> = Vec::new();
    for b in [
        Bucket::Urgent,
        Bucket::Leverage,
        Bucket::Leak,
        Bucket::Review,
        Bucket::Upkeep,
        Bucket::Contact,
    ] {
        bucket_counts.push(BucketCount {
            bucket: b,
            label: b.label().into(),
            candidates: all.iter().filter(|c| c.bucket == b).count(),
        });
    }

    // ⚠️ İKİ AYRI liste: gizlenenler kuyruktan çıkar, "yapıldı" olanlar YERİNDE KALIR.
    // Anında düşürmek günü bitmez kılıyordu (saha geri bildirimi) — bkz. `completed`.
    let gizli = hidden(conn);
    let yapilan = completed(conn, &report.analyzed_at);
    let kalan: Vec<Candidate> = all
        .into_iter()
        .filter(|c| {
            let (k, r) = ref_key(&c.reference);
            !gizli.iter().any(|(gk, gr)| gk == k && *gr == r)
        })
        .collect();

    // Sonuç kontrolü kovası boşsa en erken ne zaman dolacak? En yeni gönderimden değil,
    // EN ESKİ ölçülmemiş gönderimden hesaplanıyor — ilk dolacak olan o.
    let review_ready_at = if bucket_counts.iter().any(|b| b.bucket == Bucket::Review && b.candidates > 0) {
        String::new()
    } else {
        conn.query_row(
            "SELECT MIN(at) FROM work_events WHERE reaches_store = 1",
            [],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .and_then(|at| {
            at.get(..10)
                .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        })
        .map(|d| (d + chrono::Duration::days(queue::REVIEW_AFTER_DAYS)).to_string())
        .unwrap_or_default()
    };

    let mut items = queue::pick(kalan);
    for it in &mut items {
        let (k, r) = ref_key(&it.reference);
        it.done = yapilan.iter().any(|(dk, dr)| dk == k && *dr == r);
    }
    let done_count = items.iter().filter(|i| i.done).count();

    Ok(TodayQueue {
        items,
        analyzed_at: report.analyzed_at.clone(),
        hidden: gizli.len(),
        in_flight: ucus.len(),
        done_count,
        bucket_counts,
        review_ready_at,
    })
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
        super::log_event(&conn, &reference, "manual_done", true);
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
}
