//! # Fırsat analizi — hangi ürüne emek harcamak en çok getirir?
//!
//! Operatör 262 ürün arasında elle seçim yapıyordu. Google Search Console zaten "hangi sayfa
//! gösterim alıyor ama tıklanmıyor", "hangisi 2. sayfada takılmış" verisini tutuyor; bu modül
//! o veriyi sıralanabilir bir işe çeviriyor.
//!
//! **Tasarım kararı: soyut "puan" yerine somut sayı.** Çıktı `kaçırılan tıklama` — yani
//! "bu sayfa, konumunun normalde getirmesi gereken tıklamanın kaçını alamıyor". Operatör
//! 87 puanlık bir ürünle 62 puanlığı kıyaslayamaz ama "ayda 140 tıklama kaçırıyor" ile
//! "12 tıklama kaçırıyor"u kıyaslayabilir. Sayının nereden geldiği de açıklanabilir olmalı;
//! bu yüzden her satır ayrıca bir **sebep etiketi** taşıyor.
//!
//! Bu modül saf mantıktır (ağ/DB yok) → hızlı test edilir, `cargo test -p seo-core`.

use serde::{Deserialize, Serialize};

/// "Gösterim var ama tıklama yok" demek için gereken en az gösterim.
/// Altındaki sayılar istatistiksel gürültü: 3 gösterim / 0 tıklama bir şey ifade etmez.
const MIN_IMPRESSIONS_FOR_NO_CLICK: f64 = 50.0;

/// Google'ın ilk sayfası ~10 sonuç. 11+ = ikinci sayfa.
const FIRST_PAGE_LAST_POSITION: f64 = 10.0;
/// 20'den sonrası "ikinci sayfa fırsatı" sayılmaz — oraya çıkmak küçük bir iyileştirme değil.
const SECOND_PAGE_LAST_POSITION: f64 = 20.0;

/// Düşük CTR eşiği: beklenenin bu oranının altındaysa meta çekmiyordur.
const LOW_CTR_RATIO: f64 = 0.5;

/// Listeye girmek için gereken en az kaçırılan tıklama — altı gürültü.
const MIN_MISSED_CLICKS: f64 = 1.0;

/// Bir sayfanın konumuna göre **beklenen** tıklama oranı.
///
/// Sektör ortalaması bir eğri; kesin değil, sıralama için yeterli. Mutlak doğruluk gerekmiyor:
/// amaç ürünleri birbirine göre sıralamak, tıklama sayısı tahmin etmek değil. Konum ondalıklı
/// gelir (GSC ortalama verir), bu yüzden aralık bazlı.
pub fn expected_ctr(position: f64) -> f64 {
    match position {
        p if p < 1.5 => 0.280,
        p if p < 2.5 => 0.150,
        p if p < 3.5 => 0.110,
        p if p < 4.5 => 0.080,
        p if p < 5.5 => 0.070,
        p if p < 6.5 => 0.050,
        p if p < 7.5 => 0.040,
        p if p < 8.5 => 0.033,
        p if p < 9.5 => 0.028,
        p if p <= FIRST_PAGE_LAST_POSITION => 0.025,
        p if p <= SECOND_PAGE_LAST_POSITION => 0.010,
        _ => 0.005,
    }
}

/// Neden bu ürün listede? Sıralamayı belirlemez — operatöre *ne yapması gerektiğini* söyler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// Pozisyon 11–20: ilk sayfaya çıkmak için küçük bir itme yeterli.
    SecondPage,
    /// Gösterim alıyor ama hiç tıklanmıyor — başlık/açıklama çekmiyor.
    NoClicks,
    /// İlk sayfada ama CTR beklenenin çok altında — meta çalışması.
    LowCtr,
}

impl Reason {
    /// Arayüzde gösterilecek Türkçe etiket.
    pub fn label(self) -> &'static str {
        match self {
            Reason::SecondPage => "İkinci sayfa",
            Reason::NoClicks => "Gösterim var, tıklama yok",
            Reason::LowCtr => "Düşük CTR",
        }
    }
}

/// Fırsat listesindeki tek satır.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Opportunity {
    pub sku: String,
    pub name: String,
    pub url: String,
    pub clicks: f64,
    pub impressions: f64,
    pub ctr: f64,
    pub position: f64,
    /// Konumunun getirmesi gereken tıklamanın kaçını alamıyor (sıralama buna göre).
    pub missed_clicks: f64,
    pub reason: Reason,

    // --- Bağlam alanları (GSC'den değil, kendi kataloğumuzdan) ---
    // Hepsi `serde(default)`: eski `opportunity_json` önbelleği bu alanları taşımıyor.
    // Default olmasaydı eski önbellek çözümlenemez, kullanıcı sebepsiz boş ekran görürdü.
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub brand: String,
    /// "pending" | "done" — ürünün meta durumu.
    #[serde(default)]
    pub meta_status: String,
    #[serde(default)]
    pub details_status: String,
}

/// Fırsat listesindeki ürünün SEO iş durumu.
///
/// Ayrım önemli: ölçüme göre 60 fırsatın 49'u hiç dokunulmamış, 11'i çalışılmış ama hâlâ
/// sorunlu. Bunlar **farklı işler** — ilki "içerik üret", ikincisi "ürettiğim neden işe
/// yaramadı?". Aynı listede karışık durmaları operatörü yanlış yönlendiriyordu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkState {
    /// Ne meta ne açıklama üretilmiş.
    Untouched,
    /// Biri üretilmiş.
    Partial,
    /// İkisi de üretilmiş — ama sayfa hâlâ fırsat listesinde, yani sonuç alınamamış.
    Worked,
}

pub fn work_state(meta_status: &str, details_status: &str) -> WorkState {
    match (meta_status == "done", details_status == "done") {
        (true, true) => WorkState::Worked,
        (false, false) => WorkState::Untouched,
        _ => WorkState::Partial,
    }
}

// ====================== Sorgu düzeyi analizler ======================
//
// Sayfa düzeyi "bu sayfa sorunlu" der; sorgu düzeyi "şu kelimede 12. sıradasın" der.
// Aradaki fark, operatörün NE YAZACAĞINI bilmesi.
//
// Referans: kullanıcının paylaştığı QueryLoom kural dosyası. Aralıklar oradan alındı,
// eşikler bu katalogda ölçülerek seçildi.

/// Striking distance aralığı — QueryLoom: "positions 4–20 … may move with focused content".
/// 1-3 zaten iyi konumda; 20+ için "küçük bir itme" yetmez.
const SD_MIN_POSITION: f64 = 4.0;
const SD_MAX_POSITION: f64 = 20.0;
/// Bu gösterimin altındaki sorgular istatistiksel gürültü.
const SD_MIN_IMPRESSIONS: f64 = 30.0;

/// Kanibalizasyon: bir sorguda en iyi sayfamızın payı bunun ÜSTÜNDEYSE sorun yok —
/// Google zaten hangi sayfayı tercih ettiğine karar vermiş demektir.
/// QueryLoom: "No single URL has a clear dominant share".
const CANNIBAL_DOMINANT_SHARE: f64 = 0.70;
/// Kanibalizasyon sayılması için sorgunun en az bu kadar gösterim alması gerekir.
const CANNIBAL_MIN_IMPRESSIONS: f64 = 30.0;

/// Bir ürün sayfasının BELİRLİ BİR SORGUDAKİ fırsatı.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueryOpportunity {
    pub sku: String,
    pub name: String,
    pub query: String,
    pub clicks: f64,
    pub impressions: f64,
    pub ctr: f64,
    pub position: f64,
    pub missed_clicks: f64,
}

/// Aynı sorguda yarışan kendi sayfalarımızdan biri.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CannibalPage {
    pub sku: String,
    pub name: String,
    pub clicks: f64,
    pub impressions: f64,
    pub position: f64,
}

/// **Kanibalizasyon:** bir sorguda birden çok ürün sayfamız görünüyor ve hiçbiri baskın değil.
///
/// QueryLoom otomatik birleştirme ÖNERMİYOR, elle inceleme diyor — aynı temkin korunuyor:
/// burada yalnızca tespit var, "şunları birleştir" kararı operatörün.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Cannibalization {
    pub query: String,
    pub clicks: f64,
    pub impressions: f64,
    /// En iyi konumdaki başta.
    pub pages: Vec<CannibalPage>,
}

/// Striking distance: 4–20. sıradaki sorgular. Küçük bir iyileştirme ilk sıralara taşıyabilir.
///
/// `rows`: (sayfa, sorgu, tıklama, gösterim, ctr, pozisyon) — normalize edilmiş sayfa URL'si.
/// `by_page`: normalize URL → (sku, ad). Katalogda olmayan sayfalar atlanır (onlar EOL analizi).
pub fn striking_distance(
    rows: &[(String, String, f64, f64, f64, f64)],
    by_page: &std::collections::HashMap<String, (String, String)>,
) -> Vec<QueryOpportunity> {
    let mut out: Vec<QueryOpportunity> = rows
        .iter()
        .filter(|(_, _, _, imps, _, pos)| {
            *pos >= SD_MIN_POSITION && *pos <= SD_MAX_POSITION && *imps >= SD_MIN_IMPRESSIONS
        })
        .filter_map(|(page, query, clicks, imps, ctr, pos)| {
            let (sku, name) = by_page.get(page)?;
            Some(QueryOpportunity {
                sku: sku.clone(),
                name: name.clone(),
                query: query.clone(),
                clicks: *clicks,
                impressions: *imps,
                ctr: *ctr,
                position: *pos,
                missed_clicks: (imps * (expected_ctr(*pos) - ctr)).max(0.0),
            })
        })
        .collect();
    out.sort_by(|a, b| {
        b.missed_clicks
            .partial_cmp(&a.missed_clicks)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

/// Kanibalizasyon tespiti. Yalnızca **kendi ürün sayfalarımız** sayılır.
pub fn cannibalization(
    rows: &[(String, String, f64, f64, f64, f64)],
    by_page: &std::collections::HashMap<String, (String, String)>,
) -> Vec<Cannibalization> {
    use std::collections::HashMap;
    let mut by_query: HashMap<&str, Vec<(&str, &(String, String), f64, f64, f64)>> = HashMap::new();
    for (page, query, clicks, imps, _ctr, pos) in rows {
        if let Some(info) = by_page.get(page) {
            by_query
                .entry(query.as_str())
                .or_default()
                .push((page.as_str(), info, *clicks, *imps, *pos));
        }
    }

    let mut out: Vec<Cannibalization> = by_query
        .into_iter()
        .filter_map(|(query, mut entries)| {
            if entries.len() < 2 {
                return None; // tek sayfa = kanibalizasyon değil
            }
            let total_clicks: f64 = entries.iter().map(|e| e.2).sum();
            let total_imps: f64 = entries.iter().map(|e| e.3).sum();
            if total_imps < CANNIBAL_MIN_IMPRESSIONS {
                return None;
            }
            // Baskınlık: tıklama varsa tıklama payına, yoksa gösterim payına bak.
            let (best, total) = if total_clicks > 0.0 {
                (entries.iter().map(|e| e.2).fold(0.0, f64::max), total_clicks)
            } else {
                (entries.iter().map(|e| e.3).fold(0.0, f64::max), total_imps)
            };
            if total > 0.0 && best / total >= CANNIBAL_DOMINANT_SHARE {
                return None; // Google zaten bir sayfayı seçmiş — sorun yok
            }
            // En iyi konumdaki başta
            entries.sort_by(|a, b| a.4.partial_cmp(&b.4).unwrap_or(std::cmp::Ordering::Equal));
            Some(Cannibalization {
                query: query.to_string(),
                clicks: total_clicks,
                impressions: total_imps,
                pages: entries
                    .into_iter()
                    .map(|(_, (sku, name), clicks, imps, pos)| CannibalPage {
                        sku: sku.clone(),
                        name: name.clone(),
                        clicks,
                        impressions: imps,
                        position: pos,
                    })
                    .collect(),
            })
        })
        .collect();
    // En çok gösterim alan çakışma en önemlisi
    out.sort_by(|a, b| {
        b.impressions
            .partial_cmp(&a.impressions)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

/// EOL sayfanın listeye girmesi için gereken en az tıklama.
/// Ölçüm (2026-07-29): 4.523 EOL sayfanın yalnızca 967'si en az 1 tıklama alıyor —
/// gerisi tamamen gürültü. Tıklama alanlar toplam kaybın %100'ünü kapsıyor.
const EOL_MIN_CLICKS: f64 = 1.0;
/// Tıklama almasa da bu kadar gösterim alan sayfa da sorunludur (görünüyor ama tıklanmıyor).
const EOL_MIN_IMPRESSIONS: f64 = 100.0;

/// **Satışta olmayan ama trafik alan sayfa.**
///
/// Feed'de olmayan bir ürün URL'si = katalogdan çıkmış ama Google'da hâlâ sıralanan sayfa.
/// Müşteri geliyor, ürünü satın alamıyor. Ölçüm (2026-07-29, kurumsalit.com): ürün
/// trafiğinin **%69'u** bu sayfalara gidiyor — 3.840 tıklama.
///
/// Bu analizi QueryLoom gibi araçlar yapamaz: onlar yalnızca GSC'yi görür, hangi ürünün
/// satışta olduğunu bilmez. Biz kataloğu da biliyoruz.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EolPage {
    pub url: String,
    /// URL'nin son parçası — listede okunabilir bir ad olarak gösterilir.
    pub slug: String,
    pub clicks: f64,
    pub impressions: f64,
    pub position: f64,
}

/// GSC'de görünen ama katalogda olmayan ürün sayfalarını bulur.
///
/// `live_urls` normalize edilmiş (küçük harf, sondaki `/` atılmış) olmalı — çağıran
/// `norm_url` uygular. `path_prefix` verilirse yalnızca o yoldaki sayfalar değerlendirilir;
/// aksi halde blog ve kategori sayfaları da "satışta olmayan ürün" sanılırdı.
pub fn find_eol(
    pages: &[(String, f64, f64, f64)],
    live_urls: &std::collections::HashSet<String>,
    path_prefix: Option<&str>,
) -> Vec<EolPage> {
    let mut out: Vec<EolPage> = pages
        .iter()
        .filter(|(url, clicks, imps, _)| {
            let u = url.trim().trim_end_matches('/').to_lowercase();
            // Ürün yolunda mı? (yoksa blog/kategori sayfaları listeye sızar)
            if let Some(pfx) = path_prefix {
                if !u.contains(pfx) {
                    return false;
                }
            }
            // Katalogda var mı? Varsa EOL değil.
            if live_urls.contains(&u) {
                return false;
            }
            *clicks >= EOL_MIN_CLICKS || *imps >= EOL_MIN_IMPRESSIONS
        })
        .map(|(url, clicks, imps, pos)| EolPage {
            slug: url
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or("")
                .to_string(),
            url: url.clone(),
            clicks: *clicks,
            impressions: *imps,
            position: *pos,
        })
        .collect();
    // En çok trafik kaybeden başa
    out.sort_by(|a, b| {
        b.clicks
            .partial_cmp(&a.clicks)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.impressions.partial_cmp(&a.impressions).unwrap_or(std::cmp::Ordering::Equal))
    });
    out
}

/// Ürün URL'lerinden ortak yol önekini türetir (ör. `/urun/`).
///
/// **Neden gerekli:** sorgu × sayfa verisi çok hacimli. Bu sitede GSC'de 8087 sayfa var ama
/// bunların çoğu blog/kategori ve **EOL olmuş eski nesil ürünler**; sorgu kırılımıyla on
/// binlerce satır olur. GSC'ye `page contains <önek>` filtresi vererek yalnızca ürün
/// sayfalarını istiyoruz.
///
/// **Sabit yazılmıyor**, ürünlerin kendi URL'lerinden türetiliyor — uygulama global kullanım
/// için geliştiriliyor ve her mağazanın yol yapısı farklı olabilir.
///
/// Ortak bir ilk segment yoksa `None` döner → çağıran filtresiz çeker ve istemcide eler.
pub fn common_path_prefix(urls: &[String]) -> Option<String> {
    let first_segment = |u: &str| -> Option<String> {
        // "https://host/urun/xyz" → "urun"
        let after_scheme = u.split("://").nth(1)?;
        let path = after_scheme.split_once('/')?.1;
        let seg = path.split('/').find(|s| !s.is_empty())?;
        Some(seg.to_lowercase())
    };
    let mut it = urls.iter().filter_map(|u| first_segment(u));
    let first = it.next()?;
    if it.all(|s| s == first) {
        Some(format!("/{first}/"))
    } else {
        None
    }
}

/// Bir sayfanın ölçümlerinden fırsat çıkar. Fırsat yoksa `None`.
///
/// **Sıra önemli ve konum önce geliyor.** İkinci sayfadaki bir sayfanın tıklama almaması
/// zaten beklenen davranıştır; ona "tıklama yok" demek meta sorunu varmış gibi yanıltır ve
/// operatörü yanlış işe yönlendirir — asıl sorun konumdur. Bu yüzden önce konum bakılır,
/// tıklama/CTR yorumları yalnızca **ilk sayfadaki** sayfalar için yapılır.
pub fn classify(clicks: f64, impressions: f64, ctr: f64, position: f64) -> Option<(Reason, f64)> {
    if impressions <= 0.0 {
        return None;
    }
    let expected = expected_ctr(position);
    // Beklenenden İYİ performans gösteren sayfa fırsat değildir → negatife düşmesin.
    let missed = (impressions * (expected - ctr)).max(0.0);

    let on_first_page = position <= FIRST_PAGE_LAST_POSITION;
    let reason = if !on_first_page {
        if position <= SECOND_PAGE_LAST_POSITION {
            Reason::SecondPage
        } else {
            // 20+ sıra: buradan ilk sayfaya çıkmak "küçük bir iyileştirme" değil.
            return None;
        }
    } else if clicks == 0.0 && impressions >= MIN_IMPRESSIONS_FOR_NO_CLICK {
        Reason::NoClicks
    } else if ctr < expected * LOW_CTR_RATIO {
        Reason::LowCtr
    } else {
        return None;
    };

    if missed < MIN_MISSED_CLICKS {
        return None;
    }
    Some((reason, missed))
}

/// Fırsatları kaçırılan tıklamaya göre azalan sırala (en yüksek getirili iş başta).
pub fn sort_by_impact(list: &mut [Opportunity]) {
    list.sort_by(|a, b| {
        b.missed_clicks
            .partial_cmp(&a.missed_clicks)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_ctr_decreases_with_position() {
        let mut prev = f64::MAX;
        for p in [1.0, 2.0, 3.0, 5.0, 8.0, 10.0, 15.0, 30.0] {
            let c = expected_ctr(p);
            assert!(c < prev, "pozisyon {p} için CTR artmış olamaz ({c} >= {prev})");
            prev = c;
        }
    }

    #[test]
    fn second_page_boundary_is_exact() {
        // 10 = ilk sayfa (SecondPage DEĞİL), 11 = ikinci sayfa, 21 = artık fırsat değil
        assert_eq!(classify(5.0, 1000.0, 0.005, 10.0).unwrap().0, Reason::LowCtr);
        assert_eq!(classify(5.0, 1000.0, 0.005, 11.0).unwrap().0, Reason::SecondPage);
        assert_eq!(classify(5.0, 1000.0, 0.005, 20.0).unwrap().0, Reason::SecondPage);
        assert!(classify(5.0, 1000.0, 0.004, 21.0).is_none(), "21. sıra fırsat sayılmamalı");
    }

    /// Regresyon: ikinci sayfadaki sıfır tıklama "Tıklama yok" ETİKETLENMEMELİ.
    /// 2. sayfada tıklama zaten beklenmez; o etiket meta sorunu varmış gibi yanıltıp
    /// operatörü yanlış işe yönlendirirdi. (Gerçek veride yakalandı: TP-Link CPE510,
    /// konum 12.7, 636 gösterim, 0 tıklama → doğru etiket "İkinci sayfa".)
    #[test]
    fn zero_clicks_on_page_two_is_a_position_problem() {
        let (reason, _) = classify(0.0, 636.0, 0.0, 12.7).unwrap();
        assert_eq!(
            reason,
            Reason::SecondPage,
            "2. sayfada tıklama alamamak konum sorunudur, meta sorunu değil"
        );
    }

    #[test]
    fn no_clicks_needs_enough_impressions() {
        // 3 gösterim / 0 tıklama istatistiksel gürültü — fırsat değil
        assert!(classify(0.0, 3.0, 0.0, 8.0).is_none());
        // eşiğin üstü + anlamlı kayıp → fırsat
        let (reason, missed) = classify(0.0, 500.0, 0.0, 8.0).unwrap();
        assert_eq!(reason, Reason::NoClicks);
        assert!(missed > 15.0, "500 gösterim, 8. sıra, 0 tıklama → kayıp büyük olmalı: {missed}");
    }

    #[test]
    fn better_than_expected_is_not_an_opportunity() {
        // 1. sırada beklenen %28; %40 alıyorsa kaçırılan yok → negatife düşmemeli
        assert!(classify(400.0, 1000.0, 0.40, 1.0).is_none());
        // doğrudan hesap da negatif vermemeli
        let expected = expected_ctr(1.0);
        assert!((1000.0 * (expected - 0.40_f64)).max(0.0) == 0.0);
    }

    #[test]
    fn zero_impressions_is_not_an_opportunity() {
        assert!(classify(0.0, 0.0, 0.0, 0.0).is_none());
    }

    #[test]
    fn tiny_losses_are_filtered_out() {
        // 20 gösterim, 9. sıra, beklenene yakın CTR → kayıp 1 tıklamanın altında
        assert!(classify(0.0, 20.0, 0.02, 9.0).is_none());
    }

    fn urls(v: &[&str]) -> std::collections::HashSet<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn cat(pairs: &[(&str, &str, &str)]) -> std::collections::HashMap<String, (String, String)> {
        pairs
            .iter()
            .map(|(u, sku, n)| (u.to_string(), (sku.to_string(), n.to_string())))
            .collect()
    }

    #[test]
    fn striking_distance_respects_position_range() {
        let by_page = cat(&[("p1", "SKU1", "Ürün")]);
        let mk = |pos: f64| vec![("p1".into(), "sorgu".into(), 0.0, 500.0, 0.0, pos)];
        // 4-20 arası → fırsat
        assert_eq!(striking_distance(&mk(4.0), &by_page).len(), 1);
        assert_eq!(striking_distance(&mk(20.0), &by_page).len(), 1);
        // dışı → değil (1-3 zaten iyi, 20+ küçük itmeyle çıkılmaz)
        assert!(striking_distance(&mk(3.9), &by_page).is_empty());
        assert!(striking_distance(&mk(20.1), &by_page).is_empty());
    }

    #[test]
    fn striking_distance_skips_pages_outside_catalog() {
        // Katalogda olmayan sayfa buraya değil, EOL analizine ait.
        let by_page = cat(&[("p1", "SKU1", "Ürün")]);
        let rows = vec![("bilinmeyen".to_string(), "s".to_string(), 0.0, 900.0, 0.0, 8.0)];
        assert!(striking_distance(&rows, &by_page).is_empty());
    }

    #[test]
    fn cannibalization_needs_two_pages_without_a_dominant_one() {
        let by_page = cat(&[("p1", "A", "Ürün A"), ("p2", "B", "Ürün B")]);

        // Tek sayfa → kanibalizasyon değil
        let one = vec![("p1".to_string(), "q".to_string(), 10.0, 200.0, 0.05, 3.0)];
        assert!(cannibalization(&one, &by_page).is_empty());

        // İki sayfa ama biri baskın (%90 tıklama) → Google zaten seçmiş, sorun yok
        let dominant = vec![
            ("p1".to_string(), "q".to_string(), 90.0, 500.0, 0.18, 2.0),
            ("p2".to_string(), "q".to_string(), 10.0, 300.0, 0.03, 9.0),
        ];
        assert!(cannibalization(&dominant, &by_page).is_empty());

        // İki sayfa, pay dengeli → kanibalizasyon
        let split = vec![
            ("p1".to_string(), "q".to_string(), 50.0, 500.0, 0.10, 6.0),
            ("p2".to_string(), "q".to_string(), 45.0, 400.0, 0.11, 4.0),
        ];
        let c = cannibalization(&split, &by_page);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].pages.len(), 2);
        // En iyi konumdaki başta olmalı (4.0 < 6.0)
        assert_eq!(c[0].pages[0].sku, "B");
    }

    #[test]
    fn cannibalization_uses_impression_share_when_no_clicks() {
        // Hiç tıklama yoksa tıklama payı hesaplanamaz → gösterim payına düşmeli
        let by_page = cat(&[("p1", "A", "A"), ("p2", "B", "B")]);
        let rows = vec![
            ("p1".to_string(), "q".to_string(), 0.0, 500.0, 0.0, 12.0),
            ("p2".to_string(), "q".to_string(), 0.0, 450.0, 0.0, 14.0),
        ];
        assert_eq!(cannibalization(&rows, &by_page).len(), 1, "dengeli gösterim payı = çakışma");

        // Gösterimde baskınlık varsa işaretlenmemeli
        let dom = vec![
            ("p1".to_string(), "q".to_string(), 0.0, 950.0, 0.0, 3.0),
            ("p2".to_string(), "q".to_string(), 0.0, 50.0, 0.0, 30.0),
        ];
        assert!(cannibalization(&dom, &by_page).is_empty());
    }

    #[test]
    fn eol_excludes_pages_that_are_still_in_catalog() {
        let live = urls(&["https://x.com/urun/canli"]);
        let pages = vec![
            ("https://x.com/urun/canli".to_string(), 500.0, 9000.0, 2.0),
            ("https://x.com/urun/emekli".to_string(), 50.0, 900.0, 3.0),
        ];
        let eol = find_eol(&pages, &live, Some("/urun/"));
        assert_eq!(eol.len(), 1, "katalogdaki sayfa EOL sayılmamalı");
        assert_eq!(eol[0].slug, "emekli");
    }

    #[test]
    fn eol_ignores_non_product_paths() {
        // Blog/kategori sayfaları "satışta olmayan ürün" sanılmamalı — yanlış pozitif üretir.
        let live = urls(&[]);
        let pages = vec![
            ("https://x.com/blog/yazi".to_string(), 900.0, 9000.0, 1.0),
            ("https://x.com/urun/emekli".to_string(), 5.0, 100.0, 8.0),
        ];
        let eol = find_eol(&pages, &live, Some("/urun/"));
        assert_eq!(eol.len(), 1);
        assert_eq!(eol[0].slug, "emekli");
    }

    #[test]
    fn eol_filters_noise_and_sorts_by_clicks() {
        let live = urls(&[]);
        let pages = vec![
            ("https://x.com/urun/az".to_string(), 0.0, 5.0, 40.0),      // gürültü
            ("https://x.com/urun/orta".to_string(), 3.0, 200.0, 9.0),
            ("https://x.com/urun/cok".to_string(), 90.0, 1500.0, 4.0),
            ("https://x.com/urun/goster".to_string(), 0.0, 400.0, 15.0), // tıklama yok ama görünüyor
        ];
        let eol = find_eol(&pages, &live, Some("/urun/"));
        assert_eq!(
            eol.iter().map(|e| e.slug.as_str()).collect::<Vec<_>>(),
            ["cok", "orta", "goster"],
            "gürültü elenmeli, en çok tıklama başta olmalı"
        );
    }

    #[test]
    fn derives_product_path_prefix() {
        let urls = vec![
            "https://www.kurumsalit.com/urun/lenovo-thinkpad".to_string(),
            "https://www.kurumsalit.com/urun/dell-monitor".to_string(),
        ];
        assert_eq!(common_path_prefix(&urls).as_deref(), Some("/urun/"));
    }

    #[test]
    fn no_prefix_when_paths_differ() {
        // Ortak segment yoksa filtre uygulanmamalı — yanlış filtre veriyi sessizce yok ederdi.
        let urls = vec![
            "https://x.com/urun/a".to_string(),
            "https://x.com/product/b".to_string(),
        ];
        assert!(common_path_prefix(&urls).is_none());
        assert!(common_path_prefix(&[]).is_none());
    }

    #[test]
    fn work_state_maps_three_cases() {
        assert_eq!(work_state("pending", "pending"), WorkState::Untouched);
        assert_eq!(work_state("done", "done"), WorkState::Worked);
        assert_eq!(work_state("done", "pending"), WorkState::Partial);
        assert_eq!(work_state("pending", "done"), WorkState::Partial);
        // Beklenmeyen değerler "done" değildir → dokunulmamış sayılır
        assert_eq!(work_state("", ""), WorkState::Untouched);
    }

    /// Eski önbellek (yeni bağlam alanları yokken yazılmış) çözümlenebilmeli — aksi halde
    /// kullanıcı analiz kaybolmuş gibi boş ekran görür.
    #[test]
    fn old_cache_without_context_fields_still_parses() {
        let old = r#"{"sku":"X","name":"Ürün","url":"u","clicks":1.0,"impressions":100.0,
                      "ctr":0.01,"position":8.0,"missed_clicks":2.3,"reason":"low_ctr"}"#;
        let o: Opportunity = serde_json::from_str(old).expect("eski önbellek çözümlenmeli");
        assert_eq!(o.sku, "X");
        assert_eq!(o.category, "");
        assert_eq!(o.meta_status, "");
    }

    #[test]
    fn sorting_puts_biggest_loss_first() {
        let mk = |sku: &str, missed: f64| Opportunity {
            sku: sku.into(), name: "x".into(), url: "u".into(),
            clicks: 0.0, impressions: 0.0, ctr: 0.0, position: 0.0,
            missed_clicks: missed, reason: Reason::NoClicks,
            category: String::new(), brand: String::new(),
            meta_status: String::new(), details_status: String::new(),
        };
        let mut v = vec![mk("a", 5.0), mk("b", 120.0), mk("c", 40.0)];
        sort_by_impact(&mut v);
        assert_eq!(
            v.iter().map(|o| o.sku.as_str()).collect::<Vec<_>>(),
            ["b", "c", "a"]
        );
    }
}
