//! # Bugünün iş kuyruğu — "hangi 10 şeye bugün dokunmalıyım?"
//!
//! Uygulama dokuz ekran sunuyor ve her biri kendi listesini gösteriyor. Genel Bakış soruyu
//! **araç düzeyinde** cevaplıyor ("hangi araca gideyim"); **satır düzeyinde** cevaplayan bir
//! yer yoktu. Bu modül 2.000+ aday satırdan bugünün listesini seçer.
//!
//! Veri üretmiyor — hepsi zaten var (fırsat raporu, ölçüm omurgası, feed bayrakları).
//! Buradaki tek iş **seçim**.
//!
//! ## 🔑 Sonuç saklanmıyor, istendiğinde hesaplanıyor
//!
//! `metrics.rs` ile aynı gerekçe: saklanan kuyruk, yeni analiz gelince bayatlar ve geçersiz
//! kılma mantığı gerektirir. Kalıcı olan tek şey kullanıcının **kararı** (gizleme/erteleme).
//!
//! ## Ölçümler (2026-08-07, gerçek mağaza — 279 ürün, 90 günlük GSC)
//!
//! Üç ölçüm bu modülün tasarımını doğrudan belirledi:
//!
//! 1. 🔴 **Feed bayrağı ham hâliyle %87 gürültü.** 70 bayraktan **61'i yalnızca "görseller"**.
//!    Görsel değişikliği üretilmiş meta/açıklama METNİNİ geçersiz kılmıyor; o metin hâlâ
//!    doğru. Gerçekten acil olan: **metin alanı değişmiş VE mağazaya gönderilmiş** → 9 ürün.
//!    Bkz. [`is_urgent_change`].
//! 2. 🔴 **Ham skor sıralaması amiral analizi kuyruğa hiç sokmuyor.** Ham ilk 10'da kaçak 4 ·
//!    bakım 3 · acil 3 · **kaldıraç 0** çıktı. Sebep ölçek farkı: en büyük EOL sayfası 515
//!    tıklama, en büyük fırsat 37 kaçırılan tıklama. Bkz. [`PER_BUCKET`].
//! 3. 🔴 **Aynı ürün birden çok kovada.** 12 ürün 2+ kovada (4'ü üç kovada birden); 106 ham
//!    satır → 90 benzersiz ürün. Bkz. [`dedupe`].

use serde::{Deserialize, Serialize};

/// Bir kovadan kuyruğa girebilecek en fazla madde.
///
/// 🔬 **Neden sınır var:** ham skor sırasıyla ilk 10 ölçüldüğünde liste üç kovaya saplanıyor
/// ve uygulamanın amiral analizi (GSC fırsatları) **hiç görünmüyordu** — çünkü EOL sayfaları
/// 500+ tıklama taşırken en büyük fırsat 37 kaçırılan tıklama. 3 sınırıyla ilk 10'da dört
/// kova birden temsil ediliyor.
///
/// Sabah listesi tek bir konuya saplanmamalı: beş kova beş farklı iş türü demek.
pub const PER_BUCKET: usize = 3;

/// Kuyruğun toplam uzunluğu. Yol haritasının hedefi "5–10 net aksiyon".
pub const TOTAL: usize = 10;

/// Bir olayın sonucunun kontrol edilebilmesi için gereken gün.
///
/// `metrics::WINDOW_DAYS` ile aynı: takip penceresi tamamlanmadan "sonucu incele" demek
/// kullanıcıyı boş bir ekrana göndermek olurdu.
pub const REVIEW_AFTER_DAYS: i64 = 28;

/// Feed'de değiştiğinde üretilmiş METNİ geçersiz kılan alanlar.
///
/// ⚠️ "görseller" bilinçli olarak DIŞARIDA. Ölçüm (2026-08-07): 70 feed bayrağının 61'i
/// yalnızca görsel değişikliğiydi. Görsel değişince meta açıklaması yanlış olmuyor; bu
/// bayrakları acil kovasına koymak listeyi %87 gürültüyle doldururdu.
pub const TEXT_FIELDS: &[&str] = &["ad", "açıklama", "marka", "kategori"];

/// Kuyruk maddesinin ait olduğu iş türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Bucket {
    /// Canlıda yanlış/bayat içerik duruyor — feed değişti ya da iş mağazaya hiç ulaşmadı.
    Urgent,
    /// GSC fırsatı: konumunun getirmesi gereken tıklamayı alamayan ürün.
    Leverage,
    /// Satışta olmayan ama trafik alan sayfa.
    Leak,
    /// Gönderimin üstünden yeterli süre geçti, sonucuna bakılabilir.
    Review,
    /// Düşüşte olanlar ve birbiriyle yarışan sayfalar.
    Upkeep,
}

impl Bucket {
    pub fn label(&self) -> &'static str {
        match self {
            Bucket::Urgent => "Acil",
            Bucket::Leverage => "Yüksek kaldıraç",
            Bucket::Leak => "Kaçak trafik",
            Bucket::Review => "Sonuç kontrolü",
            Bucket::Upkeep => "Bakım",
        }
    }

    /// Skor ağırlığı.
    ///
    /// 🔑 Ağırlık keyfi değil, tek bir sorunun cevabı: **bu kaybın ne kadarı geri
    /// kazanılabilir ve uygulama bunu yapabilir mi?** Ekranda da bu gerekçeyle gösteriliyor
    /// ([`weight_reason`]) — yol haritasının şartı "formül kullanıcıya gösterilir".
    pub fn weight(&self) -> f64 {
        match self {
            // Kaçırılan tıklama doğrudan hedef ve meta/açıklama uygulamada üretiliyor.
            Bucket::Leverage => 1.0,
            // Kayıp gerçek ama sebebi dışsal olabilir (rakip, mevsim) — geri gelmeyebilir.
            Bucket::Upkeep => 0.8,
            // ⚠️ Tıklama gerçek, ama asıl çözüm 301 ve onu uygulama YAPAMIYOR (IdeaSoft
            // panelinden tanımlanıyor). Canonical yapabildiği için sıfır da değil.
            Bucket::Leak => 0.6,
            // Acil'de ağırlık değil taban kullanılıyor (bkz. URGENT_BASE); burada nötr.
            Bucket::Urgent => 1.0,
            // Sonuç kontrolünde "kayıp" yok, sabit bir öncelik var (bkz. REVIEW_SCORE).
            Bucket::Review => 1.0,
        }
    }

    pub fn weight_reason(&self) -> &'static str {
        match self {
            Bucket::Leverage => "kaçırılan tıklama doğrudan hedef; meta/açıklama uygulamada üretiliyor",
            Bucket::Upkeep => "kayıp gerçek ama sebebi dışsal olabilir (rakip, mevsim)",
            Bucket::Leak => "tıklama gerçek, ama asıl çözüm 301 ve onu uygulama yapamıyor",
            Bucket::Urgent => "tıklaması düşük olsa da canlıda yanlış içerik duruyor",
            Bucket::Review => "kayıp değil, zamanı gelmiş bir kontrol",
        }
    }

    /// 🔑 Bu kovanın kanıtı **GSC'den** mi geliyor — yani sonucu haftalarca görünmüyor mu?
    ///
    /// Acil ve sonuç kontrolü canlı veritabanından besleniyor: feed bugün değiştiyse bugün
    /// bilinir. Kaldıraç · kaçak · bakım ise 90 günlük GSC penceresine bakıyor; dün yapılan
    /// bir düzeltme o ortalamayı kıpırdatamaz. Bkz. [`drop_in_flight`].
    pub fn evidence_lags(&self) -> bool {
        matches!(self, Bucket::Leverage | Bucket::Leak | Bucket::Upkeep)
    }
}

/// Acil maddelerin sabit tabanı.
///
/// Tıklaması sıfır olan bir ürünün canlıda yanlış içerikle durması yine de iştir; saf
/// tıklama skoru bu maddeleri kuyruğun dibine atardı.
pub const URGENT_BASE: f64 = 40.0;

/// Sonuç kontrolü maddelerinin sabit skoru — tıklama cinsinden bir "kayıp" taşımıyorlar.
pub const REVIEW_SCORE: f64 = 30.0;

/// Kuyruk maddesinin kimliği. Ürün maddeleri sku, EOL maddeleri slug taşır.
///
/// ⚠️ EOL satırlarında **sku YOK** (rapor yalnızca slug/url veriyor, ölçüldü) — bu yüzden
/// kaçak maddeleri ürün maddeleriyle tekilleştirilemiyor, ayrı kimlik uzayında duruyorlar.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "ref")]
pub enum ItemRef {
    Product(String),
    Page(String),
}

impl ItemRef {
    pub fn key(&self) -> &str {
        match self {
            ItemRef::Product(s) | ItemRef::Page(s) => s,
        }
    }
}

/// Kuyruğa aday bir iş.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub reference: ItemRef,
    pub bucket: Bucket,
    /// Ekranda görünen ad (ürün adı veya sayfa slug'ı).
    pub title: String,
    /// Tek cümlelik gerekçe — **gerçek bir metrikten** türemeli (yol haritasının şartı).
    pub reason: String,
    /// Skoru süren tıklama değeri. Acil ve sonuç kovalarında 0 olabilir.
    pub clicks: f64,
    /// Maddeyi açacak ekran (`store.focus` bunu kullanıyor).
    pub page: String,
    /// Odaklanacak satırın kimliği — araç ekranındaki `TableRow.id` ile aynı olmalı.
    pub focus_id: String,
    /// Önerilen eylemin süresi (dakika).
    pub minutes: u32,
    /// Bu süre **ölçüldü mü**, yoksa elle yazılmış tahmin mi (Faz S).
    pub minutes_measured: bool,
}

/// Seçilmiş, tekilleştirilmiş ve skorlanmış kuyruk maddesi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub reference: ItemRef,
    pub bucket: Bucket,
    pub title: String,
    pub reason: String,
    pub clicks: f64,
    pub score: f64,
    pub page: String,
    pub focus_id: String,
    pub minutes: u32,
    /// Süre **ölçüldü mü**? `false` ise elle yazılmış tahmin.
    ///
    /// Faz S öncesinde hepsi tahmindi; artık odak seansı kova başına ≥5 ölçüm biriktirince
    /// medyan devralıyor. Ekran ikisini ayırt ediyor: "≈2 dk" ↔ "2 dk · ölçüldü".
    #[serde(default)]
    pub minutes_measured: bool,
    /// Aynı ürünün diğer kovalardaki sebepleri ("düşüşte", "feed değişti").
    /// Ölçüm: 12 ürün 2+ kovada, 4'ü üç kovada birden.
    pub also: Vec<String>,
    /// Bugün "yapıldı" işaretlendi mi.
    ///
    /// 🔴 **Saha geri bildirimi (2026-08-08):** işaretlenen madde listeden ANINDA
    /// düşüyordu ve yerine 11. aday geliyordu — sayaç hep 10'da kalıyor, gün bitmiyordu
    /// (*"bu mantık ile günlük iş hiç bitmez"*). Artık madde **yerinde kalıyor**, üstü
    /// çizili görünüyor ve ilerleme sayılabiliyor: günün listesi sabit, işledikçe doluyor.
    #[serde(default)]
    pub done: bool,
}

/// Feed değişikliği üretilmiş METNİ geçersiz kılıyor mu?
///
/// Yalnızca [`TEXT_FIELDS`] sayılıyor; "görseller" gürültü (ölçüm modül başlığında).
pub fn is_urgent_change(changed_fields: &str) -> bool {
    changed_fields
        .split(',')
        .map(|f| f.trim())
        .any(|f| TEXT_FIELDS.contains(&f))
}

/// Ölçümü **uçuşta** olan işleri kanıtı geciken kovalardan düşürür.
///
/// ## 🔴 Neden gerekti (saha geri bildirimi, 2026-08-09)
///
/// "Yapıldı" işareti analiz damgasına bağlıydı: analizi yeniden çalıştırınca düşüyordu. Gerekçe
/// makuldü — *"iş işe yaradıysa madde yeni raporda çıkmaz, yaramadıysa geri gelmeli"*. Ama
/// **kanıtın gelme süresini hesaba katmıyordu.**
///
/// Ölçüm: `NB.LEN.21SR006RTX` 7 Ağustos'ta mağazaya gönderildi, 8'inde "yapıldı" işaretlendi,
/// 9'unda analiz yenilenince **bakım kovasında geri geldi**. Gelmemesi de mümkün değildi: GSC
/// 90 günlük pencereye bakıyor, iki günlük bir düzeltme o ortalamayı kıpırdatamaz. Kullanıcının
/// cümlesi: *"28 gün beklersem kalan işi tamamlamaya ömrüm yetmez"*.
///
/// Bu yüzden: bir referans için mağazaya ulaşan bir iş yapıldıysa ve üstünden
/// [`REVIEW_AFTER_DAYS`] geçmediyse, o referans **kanıtı geciken kovalarda** yeniden iş
/// çıkarmaz. Kaybolmuyor: 28. günde **sonuç kontrolü** kovası onu kendiliğinden geri getiriyor
/// — Faz Ö'nün omurgası tam bunun için var.
///
/// ⚠️ Acil kovası bilinçli olarak DIŞARIDA: kanıtı GSC değil, canlı feed. Dün gönderdiğiniz
/// üründe bugün metin değiştiyse bu bugün bilinen bir gerçek ve iş gerçekten acil.
pub fn drop_in_flight(
    cands: Vec<Candidate>,
    in_flight: &std::collections::HashSet<ItemRef>,
) -> Vec<Candidate> {
    cands
        .into_iter()
        .filter(|c| !c.bucket.evidence_lags() || !in_flight.contains(&c.reference))
        .collect()
}

/// Bir adayın skoru.
pub fn score(c: &Candidate) -> f64 {
    match c.bucket {
        Bucket::Urgent => URGENT_BASE + c.clicks,
        Bucket::Review => REVIEW_SCORE,
        b => c.clicks * b.weight(),
    }
}

/// Aynı kimliği taşıyan adayları tek maddede birleştirir.
///
/// En yüksek skorlu sebep başlık olur; diğerleri `also`ya düşer. Kuyruk aynı ürünü üç kez
/// söylemez (ölçüm: 106 ham satır → 90 benzersiz).
pub fn dedupe(mut cands: Vec<Candidate>) -> Vec<QueueItem> {
    // Kararlı sıra: önce skor, eşitlikte kimlik — aynı veriyle her çalıştırmada aynı kuyruk.
    cands.sort_by(|a, b| {
        score(b)
            .partial_cmp(&score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.reference.key().cmp(b.reference.key()))
    });

    let mut out: Vec<QueueItem> = Vec::new();
    for c in cands {
        if let Some(existing) = out.iter_mut().find(|q| q.reference == c.reference) {
            // Aynı kovadan ikinci bir sebep varsa da yazılıyor: "ayrıca düşüşte" bilgisi
            // kullanıcı için değerli, kova adı değil sebep cümlesi taşınıyor.
            existing.also.push(c.reason);
            continue;
        }
        let s = score(&c);
        out.push(QueueItem {
            reference: c.reference,
            bucket: c.bucket,
            title: c.title,
            reason: c.reason,
            clicks: c.clicks,
            score: s,
            page: c.page,
            focus_id: c.focus_id,
            minutes: c.minutes,
            minutes_measured: c.minutes_measured,
            also: Vec::new(),
            done: false,
        });
    }
    out
}

/// Kuyruğu seçer: tekilleştir → kova başına [`PER_BUCKET`] → en fazla [`TOTAL`].
///
/// ⚠️ Kova sınırı **doldurulamazsa kalan yer boş bırakılmaz**: sınıra takılmayan kovalardan
/// devam edilir. Aksi halde iki kovası boş bir mağazada kuyruk 6 maddede kalırdı.
pub fn pick(cands: Vec<Candidate>) -> Vec<QueueItem> {
    let items = dedupe(cands);
    let mut counts: Vec<(Bucket, usize)> = Vec::new();
    let mut out: Vec<QueueItem> = Vec::new();
    let mut artan: Vec<QueueItem> = Vec::new();

    for it in items {
        let n = counts
            .iter_mut()
            .find(|(b, _)| *b == it.bucket)
            .map(|(_, n)| n)
            .map(|n| {
                *n += 1;
                *n
            })
            .unwrap_or_else(|| {
                counts.push((it.bucket, 1));
                1
            });
        if n <= PER_BUCKET && out.len() < TOTAL {
            out.push(it);
        } else {
            artan.push(it);
        }
    }
    // Sınır yüzünden yer kaldıysa artanla tamamla — kuyruk kısa kalmasın.
    for it in artan {
        if out.len() >= TOTAL {
            break;
        }
        out.push(it);
    }
    out
}

/// Bir kovanın süresinin **ölçülmüş** sayılabilmesi için gereken en az örnek.
///
/// 🔬 Neden bir eşik var: tek bir seans ölçümü temsil etmiyor. Kullanıcı ilk işte telefona
/// bakmış olabilir, ikincide akışa girmiş olabilir. 5 örnek, medyanın uç değerlerden
/// etkilenmemesi için gereken en küçük makul sayı (3'te iki uzun ölçüm medyanı taşır).
pub const MIN_SAMPLES: usize = 5;

/// Ölçülen sürelerden kova süresi — **medyan**, ortalama değil.
///
/// ⚠️ Ortalama kullanılmıyor: kullanıcı bir işin ortasında çay tazeleyip 40 dakika sonra
/// dönerse ortalama uçar, medyan dayanır. Bu ölçüm duvar saati süresi olduğu için böyle
/// kesintiler kural, istisna değil.
///
/// `None` dönüyorsa henüz ölçüm yok demektir; çağıran elle yazılmış tahmini kullanır ve
/// ekran bunu "≈2 dk" (tahmin) ↔ "2 dk · ölçüldü" diye ayırt eder.
///
/// ⚠️ Çağıran YALNIZCA tamamlanmış işlerin sürelerini vermeli: atlanan bir iş "ne kadar
/// sürdüğü" bilgisi taşımıyor, listeye girerse süreyi olduğundan kısa gösterir.
pub fn calibrated_minutes(samples: &[f64]) -> Option<u32> {
    if samples.len() < MIN_SAMPLES {
        return None;
    }
    let mut v: Vec<f64> = samples.iter().copied().filter(|x| x.is_finite() && *x >= 0.0).collect();
    if v.len() < MIN_SAMPLES {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let orta = v.len() / 2;
    let medyan = if v.len() % 2 == 0 { (v[orta - 1] + v[orta]) / 2.0 } else { v[orta] };
    // En az 1 dakika: "0 dk" bir iş için anlamsız bir vaat olurdu.
    Some(medyan.round().max(1.0) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aday(r: &str, b: Bucket, clicks: f64) -> Candidate {
        Candidate {
            reference: ItemRef::Product(r.into()),
            bucket: b,
            title: r.into(),
            reason: format!("{} sebebi", b.label()),
            clicks,
            page: "products".into(),
            focus_id: r.into(),
            minutes: 2,
            minutes_measured: false,
        }
    }

    /// 🔴 Saha hatası (2026-08-09): yapılan iş, analiz yenilenince kuyruğa geri geliyordu.
    /// Gerçek örnek `NB.LEN.21SR006RTX` — 7'sinde gönderildi, 9'unda "bakım" kovasında çıktı.
    #[test]
    fn olcumu_ucusta_olan_is_kaniti_geciken_kovalarda_geri_gelmiyor() {
        let uctakiler: std::collections::HashSet<ItemRef> =
            [ItemRef::Product("NB.LEN.21SR006RTX".into())].into_iter().collect();

        let kalanlar = drop_in_flight(
            vec![
                aday("NB.LEN.21SR006RTX", Bucket::Upkeep, 30.0),
                aday("NB.LEN.21SR006RTX", Bucket::Leverage, 40.0),
                aday("BASKA.SKU", Bucket::Leverage, 10.0),
            ],
            &uctakiler,
        );
        let kimlikler: Vec<&str> = kalanlar.iter().map(|c| c.reference.key()).collect();
        assert_eq!(kimlikler, vec!["BASKA.SKU"], "iki GSC kovası da susmalı");
    }

    /// ⚠️ İki kova bilinçli olarak muaf — susturmak zararlı olurdu.
    #[test]
    fn acil_ve_sonuc_kontrolu_ucus_suresinden_etkilenmiyor() {
        let uctakiler: std::collections::HashSet<ItemRef> =
            [ItemRef::Product("SKU".into())].into_iter().collect();

        // Acil: kanıtı GSC değil, canlı feed. Dün gönderilen üründe bugün metin değiştiyse
        // bu bugün bilinen bir gerçek.
        // Sonuç kontrolü: maddeyi 28. günde geri getirecek olan kova; susturmak onu
        // sonsuza dek görünmez yapardı.
        let kalanlar = drop_in_flight(
            vec![aday("SKU", Bucket::Urgent, 0.0), aday("SKU", Bucket::Review, 0.0)],
            &uctakiler,
        );
        assert_eq!(kalanlar.len(), 2);
    }

    #[test]
    fn kanit_gecikmesi_yalnizca_gsc_kovalarinda() {
        assert!(Bucket::Leverage.evidence_lags());
        assert!(Bucket::Leak.evidence_lags());
        assert!(Bucket::Upkeep.evidence_lags());
        assert!(!Bucket::Urgent.evidence_lags());
        assert!(!Bucket::Review.evidence_lags());
    }

    #[test]
    fn kalibrasyon_yeterli_ornek_yoksa_olcum_saymiyor() {
        // 4 örnek MIN_SAMPLES'ın altında: tahmin yerinde kalmalı.
        assert_eq!(calibrated_minutes(&[3.0, 4.0, 5.0, 6.0]), None);
        assert_eq!(calibrated_minutes(&[]), None);
        assert_eq!(calibrated_minutes(&[2.0, 2.0, 2.0, 2.0, 2.0]), Some(2));
    }

    #[test]
    fn kalibrasyon_medyan_kullaniyor_tek_uc_deger_bozmuyor() {
        // 🔬 Asıl korunan davranış: kullanıcı bir işin ortasında 40 dakika ara verdi.
        // Ortalama 11 derdi (yanlış); medyan 4 diyor (doğru).
        let ornekler = [3.0, 4.0, 4.0, 5.0, 40.0];
        assert_eq!(calibrated_minutes(&ornekler), Some(4));
        let ortalama: f64 = ornekler.iter().sum::<f64>() / ornekler.len() as f64;
        assert!(ortalama > 10.0, "ortalama gerçekten sapıyor: {ortalama}");
    }

    #[test]
    fn kalibrasyon_cift_sayida_ornekte_ortadaki_ikinin_ortasi() {
        assert_eq!(calibrated_minutes(&[2.0, 3.0, 5.0, 7.0, 9.0, 11.0]), Some(6));
    }

    #[test]
    fn kalibrasyon_sifira_yuvarlanmiyor() {
        // Çok hızlı işler bile "0 dk" demez — kullanıcıya anlamsız bir vaat olurdu.
        assert_eq!(calibrated_minutes(&[0.1, 0.2, 0.2, 0.3, 0.4]), Some(1));
    }

    #[test]
    fn gorsel_degisikligi_acil_degil_metin_degisikligi_acil() {
        // 🔬 Ölçüm: 70 feed bayrağının 61'i yalnızca görsel. Bunlar acil kovasına girerse
        // sabah listesi %87 gürültü olur.
        assert!(!is_urgent_change("görseller"));
        assert!(is_urgent_change("açıklama"));
        assert!(is_urgent_change("marka, görseller"));
        assert!(is_urgent_change("ad, açıklama"));
        assert!(!is_urgent_change(""));
    }

    #[test]
    fn agirliklar_kovaya_gore_carpiliyor() {
        assert_eq!(score(&aday("a", Bucket::Leverage, 37.0)), 37.0);
        assert!((score(&aday("b", Bucket::Upkeep, 100.0)) - 80.0).abs() < 1e-9);
        assert!((score(&aday("c", Bucket::Leak, 515.0)) - 309.0).abs() < 1e-9);
        // Acil: taban + tıklama. Tıklaması olmayan bir ürün de kuyruğa girebilmeli.
        assert_eq!(score(&aday("d", Bucket::Urgent, 0.0)), URGENT_BASE);
        assert_eq!(score(&aday("e", Bucket::Urgent, 17.0)), URGENT_BASE + 17.0);
        // Sonuç kontrolü tıklama taşımıyor; sabit.
        assert_eq!(score(&aday("f", Bucket::Review, 0.0)), REVIEW_SCORE);
    }

    #[test]
    fn ayni_urun_kuyruga_bir_kez_giriyor() {
        // 🔬 Ölçüm: 12 ürün 2+ kovada, 4'ü üç kovada birden.
        let q = dedupe(vec![
            aday("SKU1", Bucket::Upkeep, 100.0),  // skor 80
            aday("SKU1", Bucket::Leverage, 30.0), // skor 30
            aday("SKU1", Bucket::Urgent, 5.0),    // skor 45
            aday("SKU2", Bucket::Leverage, 10.0),
        ]);
        assert_eq!(q.len(), 2, "aynı ürün birden fazla kez girmiş");
        let ilk = &q[0];
        assert_eq!(ilk.reference, ItemRef::Product("SKU1".into()));
        // En yüksek skorlu sebep başlık olmalı (bakım 80 > acil 45 > kaldıraç 30).
        assert_eq!(ilk.bucket, Bucket::Upkeep);
        assert_eq!(ilk.also.len(), 2, "diğer sebepler taşınmamış");
    }

    #[test]
    fn urun_ve_sayfa_kimlikleri_karismaz() {
        // EOL satırında sku yok; aynı metin bir ürün sku'suyla çakışsa bile ayrı madde.
        let mut a = aday("X", Bucket::Leak, 50.0);
        a.reference = ItemRef::Page("X".into());
        let q = dedupe(vec![a, aday("X", Bucket::Leverage, 20.0)]);
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn kova_basina_sinir_uygulaniyor() {
        let mut c = Vec::new();
        for i in 0..8 {
            c.push(aday(&format!("L{i}"), Bucket::Leak, 500.0 - i as f64));
        }
        for i in 0..2 {
            c.push(aday(&format!("F{i}"), Bucket::Leverage, 30.0 - i as f64));
        }
        let q = pick(c);
        let kacak = q.iter().filter(|x| x.bucket == Bucket::Leak).count();
        // ⚠️ Sınır 3; ama kuyruk 10'a tamamlanacağı için artanlar geri dönüyor.
        // Önemli olan İLK sıraların çeşitli olması:
        assert_eq!(
            q[..5].iter().filter(|x| x.bucket == Bucket::Leak).count(),
            3,
            "ilk beşte bir kova sınırı aşmış"
        );
        assert!(q[..5].iter().any(|x| x.bucket == Bucket::Leverage), "kaldıraç ilk beşe girmemiş");
        assert_eq!(q.len(), 10);
        assert_eq!(kacak, 8, "artanlar kuyruğu tamamlamamış");
    }

    #[test]
    fn kuyruk_toplam_siniri_asmaz() {
        let c: Vec<_> = (0..40)
            .map(|i| aday(&format!("S{i}"), Bucket::Leverage, 100.0 - i as f64))
            .collect();
        assert_eq!(pick(c).len(), TOTAL);
    }

    #[test]
    fn bos_girdi_cokmez() {
        assert!(pick(Vec::new()).is_empty());
        assert!(dedupe(Vec::new()).is_empty());
    }

    #[test]
    fn ayni_veri_ayni_kuyrugu_uretiyor() {
        // Skor eşitliğinde kimliğe göre kararlı sıra — kuyruk her açılışta zıplamasın.
        let yap = || {
            vec![
                aday("B", Bucket::Leverage, 10.0),
                aday("A", Bucket::Leverage, 10.0),
                aday("C", Bucket::Leverage, 10.0),
            ]
        };
        let bir: Vec<_> = pick(yap()).iter().map(|q| q.title.clone()).collect();
        let iki: Vec<_> = pick(yap()).iter().map(|q| q.title.clone()).collect();
        assert_eq!(bir, iki);
        assert_eq!(bir, vec!["A", "B", "C"]);
    }
}
