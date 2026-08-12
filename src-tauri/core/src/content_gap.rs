//! İçerik açığı ve niyet uyuşmazlığı (Faz İ).
//!
//! **Sorulan soru:** Google bizi bir sorguda sıralıyor; sıralanan sayfa o sorgunun niyetini
//! karşılıyor mu?
//!
//! Bu, "içerik var mı?" sorusundan farklı ve fark önemli. Kullanıcının elindeki 50 maddelik
//! backlog "Google sıralıyor ama içerik yok" diyordu; ölçtüm (2026-08-12), 48 maddenin 40'ında
//! gerçekten yazı yoktu — **ama "yazı yok" ile "yazı yaz" aynı şey değil.** TeamViewer'da
//! 157.623 gösterim / 63 tık var ve arayan teamviewer.com'a gitmek istiyor; o gösterimi hiçbir
//! içerik kurtarmaz. Bu modülün asıl işi bu ayrımı yapmak.
//!
//! **Ölçülen sinyal.** "Niyet" doğrudan ölçülemez. Ölçülen şey, uygulamanın kurulu dili:
//! konumun getirmesi gereken tıklamayı alamamak (`opportunity::expected_ctr`). Buna sıralanan
//! sayfanın **tipi** eklenince (`page_kind`) niyet uyuşmazlığı çıkarılabilir hâle geliyor.
//!
//! ⚠️ Eşikler bu modülde **yeniden tanımlanmıyor**: `LOW_CTR_RATIO`,
//! `FIRST_PAGE_LAST_POSITION`, `MIN_IMPRESSIONS_FOR_NO_CLICK` `opportunity`'den geliyor. Aynı
//! sayıyı iki yere yazmak, projede dört kez sapmaya yol açtı.

use crate::opportunity::{
    expected_ctr, FIRST_PAGE_LAST_POSITION, LOW_CTR_RATIO, MIN_IMPRESSIONS_FOR_NO_CLICK,
};
use crate::page_kind::PageKind;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Bir sorgunun neden listede olduğu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapKind {
    /// Sayfa var, ilk sayfada, ama tipi bilgi amaçlı sorguyu karşılamıyor.
    IntentMismatch,
    /// Sorguyu anasayfa karşılıyor — konuya ait sayfa yok göstergesi.
    NoPage,
    /// Ürün sayfası sıralanıyor ama sorguyla ortak kelimesi yok (yanlış eşleşme).
    WrongMatch,
}

impl GapKind {
    pub fn label(&self) -> &'static str {
        match self {
            GapKind::IntentMismatch => "Niyet uyuşmuyor",
            GapKind::NoPage => "Sayfa yok",
            GapKind::WrongMatch => "Yanlış eşleşme",
        }
    }

    /// Ekranda gösterilecek gerekçe — "formül kullanıcıya gösterilir" şartı.
    pub fn reason(&self) -> &'static str {
        match self {
            GapKind::IntentMismatch => {
                "ilk sayfada ama tıklanmıyor: sıralanan sayfa vitrin, sorgu bilgi arıyor"
            }
            GapKind::NoPage => "sorguyu anasayfa karşılıyor, konuya ait sayfa yok",
            GapKind::WrongMatch => "sıralanan ürün sayfasının sorguyla ortak kelimesi yok",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentGap {
    pub query: String,
    /// Şu an bu sorguda sıralanan sayfa.
    pub page: String,
    pub page_kind: PageKind,
    pub clicks: f64,
    pub impressions: f64,
    pub ctr: f64,
    pub position: f64,
    /// Konumun getirmesi gereken ama alınamayan tıklama — sıralama buna göre.
    pub missed_clicks: f64,
    pub kind: GapKind,
}

/// **Kovalanmayacak** sorgu: hacim büyük ama kurtarılabilir değil.
///
/// 🔴 Bu ayrımın varlık sebebi tek bir örnek: TeamViewer. 157.623 gösterim sitenin toplamının
/// %4,9'u ve 63 tık getiriyor. Sorgu **navigasyonel** — arayan teamviewer.com'a gitmek istiyor,
/// bayi sayfasına değil. Pozisyonu 9'dan 3'e çıkarmak bu oranı değiştirmez. Liste bunu
/// ayırmasaydı ekran ilk açıldığı gün operatörü en büyük sayının peşine takardı ve o iş
/// baştan kayıptı.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Navigational {
    pub query: String,
    pub impressions: f64,
    pub clicks: f64,
    pub ctr: f64,
    pub position: f64,
    pub reason: String,
}

/// En az konum: bunun üstündeki (daha kötü) sıralamalarda "niyet uyuşmuyor" denmez.
///
/// 🔴 Ölçümle eklendi: poz 1–2'de beklenen TO eğrisi çok yüksek olduğu için orada
/// **her şey** eksik görünüyordu. Gerçek veride *"masa standı"* (poz 1,3) ve
/// *"ipad air pembe"* (poz 1,4) böyle listeye girdi — oysa ürün sorgusuna ürün sayfası
/// çıkması isabettir, açık değil.
const MIN_POSITION_FOR_MISMATCH: f64 = 3.0;

/// Türkçe **bilgi arama** belirteçleri.
///
/// 🔴 Sorgunun niyeti ölçülmeden yalnızca sayfa tipine bakmak, ilk sürümün en büyük
/// kusuruydu: ürün sorguları ürün sayfasıyla karşılandığı hâlde "uyuşmazlık" sayıldı.
/// Bu liste bir vekil; kesin değil ama yanlış yöne iş üretmesini engelliyor.
const BILGI_BELIRTECLERI: &[&str] = &[
    "nedir", "nasil", "ne", "ise", "yarar", "farki", "farklari", "hangi", "mi", "midir",
    "kurulumu", "ayarlari", "anlami", "avantajlari", "dezavantajlari", "onerisi", "rehber",
    "karsilastirma", "vs", "alinir", "kullanimi", "cesitleri", "turleri",
];

/// Navigasyonel sayılmak için TO'nun beklenenin bu oranının altında olması gerekir.
///
/// `LOW_CTR_RATIO`'dan (0,5) çok daha katı, çünkü iddia daha güçlü: "bu iş yapılamaz" demek,
/// "bu iş zor" demekten farklı. TeamViewer ölçümü %0,04 TO / poz 9,72 — beklenenin ~%1'i.
const NAV_CTR_RATIO: f64 = 0.1;

/// Sorguyu kelimelere ayırır (Türkçe harfler sadeleştirilir).
fn tokens(s: &str) -> Vec<String> {
    let mut t = String::with_capacity(s.len());
    for c in s.to_lowercase().chars() {
        t.push(match c {
            'ı' => 'i',
            'ş' => 's',
            'ğ' => 'g',
            'ü' => 'u',
            'ö' => 'o',
            'ç' => 'c',
            c => c,
        });
    }
    t.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_string())
        .collect()
}

/// Sorgu tek bir markanın (ya da **kendi sitemizin**) adından mı oluşuyor?
///
/// ⚠️ Liste **koda gömülü değil**: markalar katalogdan (`products.brand`), site adı GSC
/// adresinden türetiliyor — uygulama kişiselleştirilmemiş. Ergotron da bir marka ama TO'su iyi
/// (%6,54 ölçüldü), bu yüzden tek başına marka olmak yetmiyor; TO koşulu da aranıyor.
///
/// 🔴 **Kendi adımız buraya ölçümle eklendi.** Gerçek veride *kurumsalit* ve *kurumsal it*
/// poz 1,0'da "niyet uyuşmuyor" diye işaretlendi. Kendi adımızı arayan bizi bulmuş — bu bir
/// açık değil, kuyruğa girmemeli.
fn is_brand_only(query: &str, brands: &HashSet<String>) -> bool {
    let t = tokens(query);
    if t.is_empty() {
        return false;
    }
    // ⚠️ Karşılaştırma İKİ TARAFTA da boşluksuzlaştırılıyor. Sebebi ölçüldü: *kurumsal it*
    // (304 gösterim) ile *kurumsalit* (398) aynı şeyi arıyor. Tek tarafı normalleştirmek
    // yetmiyor — testi düşürdü: katalogdaki marka "bambu lab" boşluklu, sorgu "bambulab"
    // boşluksuz gelebiliyor.
    let bosluklu = t.join(" ");
    if brands.contains(&bosluklu) {
        return true;
    }
    let bosluksuz = t.join("");
    brands.iter().any(|b| b.replace(' ', "") == bosluksuz)
}

/// GSC adresinden site adını çıkarır: `https://www.kurumsalit.com/` → `["kurumsalit"]`.
///
/// 🔴 **Bölme noktası TAHMİN EDİLMİYOR.** İlk sürüm "kurumsalit"i her yerden bölüp
/// `kurumsa lit`, `kurums alit`, `kurum salit` gibi çöp belirteçler üretiyordu. Boşluklu
/// arama (*kurumsal it*, 304 gösterim) `is_brand_only` içinde sorgunun boşlukları atılarak
/// karşılanıyor — bir kural, tahmin yok.
///
/// Koda hiçbir mağaza adı yazılmıyor; ad adresten geliyor.
pub fn site_brand_tokens(site_url: &str) -> Vec<String> {
    let host = site_url
        .split("://")
        .last()
        .unwrap_or("")
        .split('/')
        .next()
        .unwrap_or("")
        .to_lowercase();
    let ad = host
        .trim_start_matches("www.")
        .split('.')
        .next()
        .unwrap_or("")
        .to_string();
    if ad.is_empty() {
        return Vec::new();
    }
    vec![ad]
}

/// Sorgu bilgi arıyor mu?
///
/// İki yol: açık bir belirteç (*nedir*, *nasıl*…) **veya** jenerik olması — yani içinde
/// harf+rakam karışık bir model kodu geçmemesi (`k2`, `g6`, `21sr006rtx`). Model kodu geçen
/// sorgu bir ürünü arıyor, bilgi değil.
pub fn is_informational(query: &str) -> bool {
    let t = tokens(query);
    if t.iter().any(|w| BILGI_BELIRTECLERI.contains(&w.as_str())) {
        return true;
    }
    let model_kodu_var = t.iter().any(|w| {
        w.chars().any(|c| c.is_ascii_digit()) && w.chars().any(|c| c.is_ascii_alphabetic())
    });
    !model_kodu_var && t.len() <= 3
}

/// Navigasyonel sorguları ayırır. Dönen ikinci liste **kuyruğa girmez**.
///
/// ⚠️ Bu bir **vekil ölçüt**, niyet ölçümü değil: "katalogdaki bir markanın adı + çok yüksek
/// hacim + beklenenin onda birinden az TO". Yanılabilir; bu yüzden ekranda ayrı bölümde,
/// gerekçesiyle gösteriliyor ve kullanıcı görüp itiraz edebiliyor. Sessizce atmak yanlış olurdu.
pub fn split_navigational(
    rows: &[(String, String, f64, f64, f64)],
    brands: &HashSet<String>,
    site_tokens: &HashSet<String>,
) -> (Vec<(String, String, f64, f64, f64)>, Vec<Navigational>) {
    let mut kalan = Vec::new();
    let mut nav: Vec<Navigational> = Vec::new();
    for r in rows {
        let (query, clicks, imps, pos) = (&r.1, r.2, r.3, r.4);
        let ctr = if imps > 0.0 { clicks / imps } else { 0.0 };
        let bekleyen = expected_ctr(pos);
        // 🔴 İKİ AYRI KURAL, çünkü iki ayrı durum. İlk sürümde tek kural vardı ve testi
        // düşürdü: kendi marka sorgumuz İYİ TO alıyor (%7,5 ölçüldü), o yüzden "TO çok
        // düşük" koşuluna hiç takılmıyordu.
        //
        // (a) **Kendi adımız** → koşulsuz ayrılır. Trafik zaten bize geliyor; yazılacak
        //     içerik yok, TO'nun iyi ya da kötü olması bunu değiştirmiyor.
        // (b) **Başka bir üreticinin markası** → yalnızca TO beklenenin onda birinden azsa.
        //     Ergotron da marka ama TO'su iyi (%6,54) ve o gerçek bir kazanç; ayırmak yanlış.
        //
        // ⚠️ Hacim koşulu paylaşılan gürültü eşiği (50); daha önce 5.000'di ve BELGEDEN
        // alınmıştı — 90 günlük gerçek veride hiç ateşlenmedi (bkz. brain.md 0b9).
        let kendi = is_brand_only(query, site_tokens);
        let uretici = !kendi
            && imps >= MIN_IMPRESSIONS_FOR_NO_CLICK
            && ctr < bekleyen * NAV_CTR_RATIO
            && is_brand_only(query, brands);

        if kendi || uretici {
            nav.push(Navigational {
                query: query.clone(),
                impressions: imps,
                clicks,
                ctr,
                position: pos,
                reason: if kendi {
                    "kendi markamız arandı — trafik zaten bize geliyor, yazılacak içerik yok"
                        .to_string()
                } else {
                    format!(
                        "başka bir üreticinin marka adı arandı ve konum (poz {pos:.1}) uygun \
                         olduğu hâlde TO beklenenin %{:.0}'ı — arayan üreticinin sitesine \
                         gidiyor, içerikle kurtarılamaz",
                        if bekleyen > 0.0 { ctr / bekleyen * 100.0 } else { 0.0 }
                    )
                },
            });
        } else {
            kalan.push(r.clone());
        }
    }
    // ⚠️ Sorgu düzeyinde tekilleştir: hüküm ("bu sorgu kovalanmaz") **sorguya** ait, sayfaya
    // değil. Aynı sorguya birden çok sayfamız sıralanabiliyor ve gerçek veride
    // *kurumsalit* dört ayrı satırda çıktı; listede dört kez görünmesi yalnızca gürültü.
    nav.sort_by(|a, b| {
        b.impressions.partial_cmp(&a.impressions).unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut gorulen: HashSet<String> = HashSet::new();
    nav.retain(|n| gorulen.insert(n.query.to_lowercase()));
    (kalan, nav)
}

/// İçerik açıklarını çıkarır.
///
/// `rows`: `(sayfa, sorgu, tık, gösterim, konum)` — navigasyonel ayıklanmış olmalı.
/// `kind_of`: sayfa → tip (bkz. `page_kind::classify`).
/// `product_name_of`: ürün sayfasının adı; yanlış eşleşme kontrolü için. `None` → ürün değil
/// ya da katalogda yok.
pub fn find_gaps(
    rows: &[(String, String, f64, f64, f64)],
    kind_of: &dyn Fn(&str) -> PageKind,
    product_name_of: &dyn Fn(&str) -> Option<String>,
) -> Vec<ContentGap> {
    let mut out: Vec<ContentGap> = Vec::new();

    for (page, query, clicks, imps, pos) in rows {
        // Gürültü eşiği ürün analiziyle aynı: az gösterimde TO bir şey ifade etmez.
        if *imps < MIN_IMPRESSIONS_FOR_NO_CLICK {
            continue;
        }
        let ctr = if *imps > 0.0 { clicks / imps } else { 0.0 };
        let bekleyen = expected_ctr(*pos);
        let kind = kind_of(page);

        // ⚠️ Etiketlenmemiş segment hakkında hüküm verilmiyor — kullanıcı o segmenti
        // tanımlamadıysa "niyeti karşılamıyor" demek uydurma olur.
        let bilgiyi_karsilar = match kind.serves_informational() {
            Some(v) => v,
            None => continue,
        };

        let dusuk_to = ctr < bekleyen * LOW_CTR_RATIO;
        // Poz 3–10 bandı: ilk sayfada ama tepede değil. Tepe, marka/SERP bölgesi.
        let bant = *pos >= MIN_POSITION_FOR_MISMATCH && *pos <= FIRST_PAGE_LAST_POSITION;

        let tur = if kind == PageKind::Home {
            // Anasayfa sıralanıyor → konuya ait sayfa yok. Bu, düşük TO gerektirmiyor:
            // ölçümde tam tersi görüldü (SKU kodları anasayfayla sıralanıp %10,45 TO alıyor)
            // ve sayfa yokluğu yine gerçek.
            GapKind::NoPage
        } else if kind == PageKind::Product
            && product_name_of(page)
                .map(|ad| {
                    let urun = tokens(&ad);
                    // Sorgunun HİÇBİR anlamlı kelimesi ürün adında yok → yanlış eşleşme.
                    // Kısa kelimeler (2 harf) atılmıyor: model kodları (`k2`, `g6`, `a1`)
                    // tam olarak orada ve atıldıklarında eşleştirme yanlış pozitif üretiyor
                    // (bu hata bir kez yapıldı, bkz. brain.md 0b8).
                    !tokens(query).iter().any(|t| urun.contains(t))
                })
                .unwrap_or(false)
        {
            GapKind::WrongMatch
        } else if bant && dusuk_to && !bilgiyi_karsilar && is_informational(query) {
            GapKind::IntentMismatch
        } else {
            continue;
        };

        out.push(ContentGap {
            query: query.clone(),
            page: page.clone(),
            page_kind: kind,
            clicks: *clicks,
            impressions: *imps,
            ctr,
            position: *pos,
            missed_clicks: (imps * (bekleyen - ctr)).max(0.0),
            kind: tur,
        });
    }

    out.sort_by(|a, b| {
        b.missed_clicks
            .partial_cmp(&a.missed_clicks)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn markalar() -> HashSet<String> {
        ["teamviewer", "ergotron", "lenovo"].iter().map(|s| s.to_string()).collect()
    }

    fn tipler() -> HashMap<String, PageKind> {
        [
            ("https://m.com/marka/teamviewer".to_string(), PageKind::Brand),
            ("https://m.com/kategori/access-point".to_string(), PageKind::Category),
            ("https://m.com/blog/icerik/x".to_string(), PageKind::Blog),
            ("https://m.com/".to_string(), PageKind::Home),
            ("https://m.com/urun/hp-elitebook-840".to_string(), PageKind::Product),
            ("https://m.com/destek/sss".to_string(), PageKind::Other),
        ]
        .into_iter()
        .collect()
    }

    fn kind_fn(m: HashMap<String, PageKind>) -> impl Fn(&str) -> PageKind {
        move |u: &str| m.get(u).copied().unwrap_or(PageKind::Other)
    }

    /// 🔴 Bu testin varlık sebebi tek bir saha vakası: TeamViewer 157.623 gösterim / 63 tık.
    /// Liste onu ayırmazsa ekran operatörü kurtarılamaz bir hacmin peşine takar.
    #[test]
    fn navigasyonel_sorgu_listeden_ayriliyor() {
        let rows = vec![
            // TeamViewer: marka adı + büyük hacim + neredeyse sıfır TO
            ("https://m.com/marka/teamviewer".into(), "teamviewer".into(), 63.0, 157_623.0, 9.72),
            // Ergotron da marka AMA TO'su iyi → navigasyonel değil, listede kalmalı
            ("https://m.com/marka/ergotron".into(), "ergotron".into(), 97.0, 1_491.0, 5.61),
            // Marka değil, jenerik terim → kalmalı
            ("https://m.com/kategori/access-point".into(), "access point".into(), 84.0, 32_816.0, 6.07),
        ];
        let (kalan, nav) = split_navigational(&rows, &markalar(), &HashSet::new());
        assert_eq!(nav.len(), 1, "yalnızca teamviewer ayrılmalı");
        assert_eq!(nav[0].query, "teamviewer");
        assert!(nav[0].reason.contains("üreticinin sitesine"), "gerekçe gösterilmeli");
        assert_eq!(kalan.len(), 2, "ergotron ve access point listede kalmalı");
        assert!(kalan.iter().any(|r| r.1 == "access point"));
    }

    /// Jenerik bir terim yüksek hacimli ve düşük TO'lu olsa da navigasyonel DEĞİL:
    /// "firewall" için içerik yazmak gerçek bir iş (denetim raporu da öyle diyor).
    #[test]
    fn jenerik_terim_navigasyonel_sayilmiyor() {
        let rows = vec![(
            "https://m.com/kategori/guvenlik-duvari-firewall".into(),
            "firewall".into(),
            67.0,
            13_117.0,
            9.44,
        )];
        let (kalan, nav) = split_navigational(&rows, &markalar(), &HashSet::new());
        assert!(nav.is_empty(), "firewall markanın adı değil");
        assert_eq!(kalan.len(), 1);
    }

    #[test]
    fn niyet_uyusmazligi_vitrin_sayfasinda_bulunuyor() {
        // Kategori sayfası ilk sayfada ama TO beklenenin çok altında.
        let rows = vec![(
            "https://m.com/kategori/access-point".into(),
            "access point nedir".into(),
            84.0,
            32_816.0,
            6.07,
        )];
        let g = find_gaps(&rows, &kind_fn(tipler()), &|_| None);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].kind, GapKind::IntentMismatch);
        assert_eq!(g[0].page_kind, PageKind::Category);
        assert!(g[0].missed_clicks > 0.0);
    }

    /// Blog sayfası bilgi sorgusunu KARŞILIYOR — TO düşük olsa bile niyet uyuşmazlığı değil.
    /// (Sorun başka olabilir: başlık, AI Overview. Ama bu kova o iş için değil.)
    #[test]
    fn blog_sayfasi_niyet_uyusmazligi_uretmiyor() {
        let rows = vec![(
            "https://m.com/blog/icerik/x".into(),
            "lenovo boot tusu".into(),
            1_755.0,
            223_179.0,
            5.18,
        )];
        assert!(find_gaps(&rows, &kind_fn(tipler()), &|_| None).is_empty());
    }

    #[test]
    fn anasayfa_sayfa_yok_demek_ve_yuksek_to_engel_degil() {
        // Ölçüm: SKU kodları anasayfayla sıralanıp %10,45 TO alıyor. Sayfa yokluğu gerçek.
        let rows =
            vec![("https://m.com/".into(), "d30n5et".into(), 18.0, 172.0, 9.0)];
        let g = find_gaps(&rows, &kind_fn(tipler()), &|_| None);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].kind, GapKind::NoPage);
    }

    #[test]
    fn yanlis_eslesme_ortak_kelime_yoksa() {
        let rows = vec![(
            "https://m.com/urun/hp-elitebook-840".into(),
            "hp sunucu".into(),
            5.0,
            848.0,
            4.5,
        )];
        // "hp" ortak → yanlış eşleşme SAYILMAZ (ortak kelime var).
        let g = find_gaps(&rows, &kind_fn(tipler()), &|_| Some("HP EliteBook 840 G5".into()));
        assert!(
            g.iter().all(|x| x.kind != GapKind::WrongMatch),
            "ortak kelime varken yanlış eşleşme denmemeli"
        );

        // Hiç ortak kelime yok → yanlış eşleşme.
        let rows2 = vec![(
            "https://m.com/urun/hp-elitebook-840".into(),
            "aruba modem".into(),
            2.0,
            761.0,
            6.0,
        )];
        let g2 = find_gaps(&rows2, &kind_fn(tipler()), &|_| Some("HP EliteBook 840 G5".into()));
        assert_eq!(g2.len(), 1);
        assert_eq!(g2[0].kind, GapKind::WrongMatch);
    }

    /// 🔴 Etiketlenmemiş segment iş düşürmüyor. Kullanıcı `/destek/`i tanımlamadıysa onun
    /// hakkında hüküm vermiyoruz.
    #[test]
    fn etiketsiz_segment_is_dusurmuyor() {
        let rows = vec![(
            "https://m.com/destek/sss".into(),
            "garanti sorgulama".into(),
            1.0,
            4_000.0,
            4.0,
        )];
        assert!(find_gaps(&rows, &kind_fn(tipler()), &|_| None).is_empty());
    }

    /// Az gösterimli satır gürültü — ürün analiziyle aynı eşik.
    #[test]
    fn gurultu_esigi_altindaki_satir_girmiyor() {
        let rows =
            vec![("https://m.com/kategori/access-point".into(), "x".into(), 0.0, 12.0, 3.0)];
        assert!(find_gaps(&rows, &kind_fn(tipler()), &|_| None).is_empty());
    }

    /// 🔴 **Düzeltme 1'in testi.** Eşik 5.000'di ve BELGEDEN alınmıştı; TeamViewer'ın 90
    /// günlük gerçeği 452 gösterim / poz 14,7. Eşik düşmeseydi navigasyonel tespit hiç
    /// ateşlenmezdi — gerçek veride tam olarak bu oldu (brain.md 0b9).
    #[test]
    fn navigasyonel_dusuk_hacimde_de_yakalaniyor() {
        let rows = vec![(
            "https://m.com/marka/teamviewer".into(),
            "teamviewer".into(),
            0.0,
            255.0,
            21.2,
        )];
        let (kalan, nav) = split_navigational(&rows, &markalar(), &HashSet::new());
        assert_eq!(nav.len(), 1, "90 günlük gerçek hacimde de yakalanmalı");
        assert!(kalan.is_empty());
    }

    /// 🔴 **Düzeltme 2'nin testi.** Kendi adımızı arayan bizi bulmuş; açık değil.
    #[test]
    fn kendi_site_adimiz_navigasyonel() {
        // Tek belirteç yeter: boşluklu arama `is_brand_only` içinde normalleştirmeyle
        // karşılanıyor (bölme noktası tahmin edilmiyor).
        let t = site_brand_tokens("https://www.kurumsalit.com/");
        assert_eq!(t, vec!["kurumsalit".to_string()]);

        let site: HashSet<String> = t.into_iter().collect();
        let rows = vec![
            ("https://m.com/kategori/x".into(), "kurumsalit".into(), 30.0, 398.0, 1.0),
            ("https://m.com/kategori/x".into(), "kurumsal it".into(), 20.0, 304.0, 1.1),
        ];
        let (kalan, nav) = split_navigational(&rows, &markalar(), &site);
        assert_eq!(nav.len(), 2, "iki biçim de ayrılmalı");
        assert!(kalan.is_empty());
    }

    /// 🔴 Bölme noktası tahmin edilmiyor: ilk sürüm `kurumsa lit`, `kurums alit` gibi çöp
    /// belirteçler üretiyordu. Tek belirteç döner; boşluklu arama sorgu normalleştirmesiyle
    /// karşılanır.
    #[test]
    fn site_adi_tek_belirtec_cop_uretmiyor() {
        assert_eq!(
            site_brand_tokens("https://www.kurumsalit.com/"),
            vec!["kurumsalit".to_string()]
        );
        assert_eq!(site_brand_tokens("http://abc.com"), vec!["abc".to_string()]);
        assert!(site_brand_tokens("").is_empty());
    }

    /// Boşluklu/boşluksuz aynı sayılıyor — hem kendi adımız hem boşluklu markalar için.
    #[test]
    fn boslukli_ve_bosluksuz_bicim_ayni() {
        let m: HashSet<String> = ["kurumsalit", "bambu lab"].iter().map(|s| s.to_string()).collect();
        assert!(is_brand_only("kurumsal it", &m), "boşluklu sorgu boşluksuz markaya");
        assert!(is_brand_only("bambulab", &m), "boşluksuz sorgu boşluklu markaya");
        assert!(!is_brand_only("bambu lab a1", &m), "model kodu eklenince marka sorgusu değil");
    }

    /// 🔴 **Düzeltme 3'ün testi — en büyük kusur.** İlk sürüm yalnızca sayfa tipine bakıyordu
    /// ve ürün sorguları ürün sayfasıyla karşılandığı hâlde "uyuşmazlık" sayılıyordu.
    #[test]
    fn urun_sorgusu_urun_sayfasinda_acik_degil() {
        // Gerçek veriden: poz 1,3'te ürün sorgusu → isabet, açık değil.
        let rows = vec![(
            "https://m.com/urun/hp-elitebook-840".into(),
            "masa standi".into(),
            2.0,
            158.0,
            1.3,
        )];
        assert!(
            find_gaps(&rows, &kind_fn(tipler()), &|_| Some("masa standı ergotron".into()))
                .is_empty(),
            "ürün sorgusu + ürün sayfası + tepe konum → açık değil"
        );
        // Model kodu içeren sorgu da ürün arıyor, bilgi değil.
        assert!(!is_informational("21sr006rtx"));
        assert!(!is_informational("creality k2 combo"));
        // Bilgi belirteçleri
        assert!(is_informational("access point nedir"));
        assert!(is_informational("firewall ile utm farki"));
        assert!(is_informational("access point"), "jenerik kısa terim");
        // Uzun ve model kodsuz ama jenerik sayılmayacak kadar özgül
        assert!(!is_informational("hp elitebook 840 g5 sarj adaptoru fiyati"));
    }

    /// Konum tabanı: poz 1–2'de beklenen TO eğrisi çok yüksek, orada her şey "eksik" görünür.
    #[test]
    fn tepe_konumda_uyusmazlik_denmiyor() {
        let rows = vec![(
            "https://m.com/kategori/access-point".into(),
            "access point nedir".into(),
            5.0,
            500.0,
            1.5,
        )];
        assert!(
            find_gaps(&rows, &kind_fn(tipler()), &|_| None).is_empty(),
            "poz 1,5 tepe bölgesi — marka/SERP etkisi, içerik açığı değil"
        );
    }

    /// 🔬 **Gerçek veri ölçümü.** Sentetik testler mantığın tutarlı olduğunu söyler; bu test
    /// çıktının ANLAMLI olup olmadığını söyler. Python'da yeniden yazmak yerine gerçek kod
    /// koşturuluyor — paralel bir uygulama, sapacak bir kopyadır.
    ///
    /// `QP_TSV=/…/qp.tsv SEO_DB=/…/kopya.db cargo test gercek_veri -- --ignored --nocapture`
    ///
    /// TSV biçimi `qp_volume --nocapture` çıktısındaki `ORNEK` satırları:
    /// `ORNEK<TAB>sayfa<TAB>sorgu<TAB>tık<TAB>gösterim<TAB>konum`
    #[test]
    #[ignore]
    fn gercek_veri() {
        use crate::page_kind;
        let tsv = std::env::var("QP_TSV").expect("QP_TSV yok");
        let db = std::env::var("SEO_DB").expect("SEO_DB yok");
        let conn = rusqlite::Connection::open(&db).unwrap();

        // Katalog: ürün segmenti, ürün adları ve markalar — hepsi ÖLÇÜMDEN, gömülü değil.
        let urls: Vec<String> = conn
            .prepare("SELECT url FROM products WHERE url IS NOT NULL AND url <> ''")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .filter_map(|x| x.ok())
            .collect();
        let urun_seg = crate::opportunity::common_path_prefix(&urls)
            .map(|p| p.trim_matches('/').to_string());
        let ad_of: HashMap<String, String> = conn
            .prepare("SELECT lower(url), name FROM products WHERE url IS NOT NULL")
            .unwrap()
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .unwrap()
            .filter_map(|x| x.ok())
            .collect();
        let markalar: HashSet<String> = conn
            .prepare("SELECT DISTINCT lower(brand) FROM products WHERE brand IS NOT NULL AND brand <> ''")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .filter_map(|x| x.ok())
            .collect();

        let mut rows: Vec<(String, String, f64, f64, f64)> = Vec::new();
        for l in std::fs::read_to_string(&tsv).unwrap().lines() {
            let p: Vec<&str> = l.split('\t').collect();
            if p.len() < 6 || p[0] != "ORNEK" {
                continue;
            }
            rows.push((
                p[1].to_string(),
                p[2].to_string(),
                p[3].parse().unwrap_or(0.0),
                p[4].parse().unwrap_or(0.0),
                p[5].parse().unwrap_or(0.0),
            ));
        }
        println!("okunan satır: {}  · marka: {}  · ürün segmenti: {:?}",
                 rows.len(), markalar.len(), urun_seg);

        // Segment etiketleri: kullanıcı normalde Ayarlar'dan verir; ölçümde veriden türetiyoruz.
        let sayfa_urls: Vec<String> = rows.iter().map(|r| r.0.clone()).collect();
        let mut etiket: HashMap<String, page_kind::PageKind> = HashMap::new();
        for (seg, n) in page_kind::unlabeled_segments(&sayfa_urls, urun_seg.as_deref(), &etiket) {
            println!("  segment {seg:22} {n:>6} sayfa");
        }
        for (seg, k) in [
            ("kategori", page_kind::PageKind::Category),
            ("marka", page_kind::PageKind::Brand),
            ("blog", page_kind::PageKind::Blog),
        ] {
            etiket.insert(seg.to_string(), k);
        }

        let site: HashSet<String> = site_brand_tokens(
            &std::env::var("GSC_SITE").unwrap_or_default(),
        )
        .into_iter()
        .collect();
        println!("kendi marka belirteçleri: {site:?}");
        let (kalan, nav) = split_navigational(&rows, &markalar, &site);
        println!("\nNAVİGASYONEL (kuyruğa girmiyor): {} sorgu", nav.len());
        for n in nav.iter().take(8) {
            println!("  {:34} gös {:>9.0}  tık {:>5.0}  TO %{:.2}",
                     n.query, n.impressions, n.clicks, n.ctr * 100.0);
        }

        let et = etiket.clone();
        let us = urun_seg.clone();
        let kind_of = move |u: &str| page_kind::classify(u, us.as_deref(), &et);
        let name_of = move |u: &str| ad_of.get(&u.to_lowercase()).cloned();
        let gaps = find_gaps(&kalan, &kind_of, &name_of);

        let mut sayim: HashMap<&str, (usize, f64)> = HashMap::new();
        for g in &gaps {
            let e = sayim.entry(g.kind.label()).or_insert((0, 0.0));
            e.0 += 1;
            e.1 += g.missed_clicks;
        }
        println!("\nİÇERİK AÇIĞI: {} madde", gaps.len());
        for (k, (n, m)) in &sayim {
            println!("  {k:18} {n:>5} madde · kaçırılan tık {m:>8.0}");
        }
        println!("\n--- en büyük 20 ---");
        for g in gaps.iter().take(20) {
            println!("  {:12} {:34} gös {:>8.0} poz {:>5.1} kaçan {:>6.0}  [{}]",
                     g.kind.label(), g.query, g.impressions, g.position,
                     g.missed_clicks, g.page_kind.label());
        }

        assert!(
            !nav.iter().any(|n| gaps.iter().any(|g| g.query == n.query)),
            "navigasyonel sorgu açık listesine sızmamalı"
        );
    }

    /// Kısa model kodları atılmamalı — bu hata bir kez yapıldı (brain.md 0b8).
    #[test]
    fn kisa_model_kodlari_token_olarak_korunuyor() {
        assert!(tokens("creality k2 combo").contains(&"k2".to_string()));
        assert!(tokens("bambu lab a1").contains(&"a1".to_string()));
        assert!(tokens("HP Pavilion G6").contains(&"g6".to_string()));
        // Türkçe sadeleştirme
        assert!(tokens("yazıcı çözümü").contains(&"yazici".to_string()));
    }
}
