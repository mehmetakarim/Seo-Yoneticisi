//! # Teklif hesabı — satır, KDV kırılımı, marj, numara, durum (Faz T)
//!
//! Teklif bugüne kadar Excel'de veya mail gövdesinde hazırlanıyordu: hangi fiyatın verildiği,
//! teklifin ne olduğu ve niye kaybedildiği hiçbir yerde kayıtlı değildi. Bu modül teklifin
//! **aritmetiğini** tutuyor; veritabanı ve ekran ona bakıyor.
//!
//! ## 🔴 Maliyet müşteriye giden çıktıya ASLA girmez
//!
//! `cost` burada var çünkü marj onsuz hesaplanamaz. Ama çıktı üreten kod bu tipi
//! görmüyor: `quote_html` ayrı bir yapı alıyor ve o yapıda maliyet **alanı yok**. Yani sızıntı
//! bir dikkat meselesi değil, derleme meselesi.
//!
//! ## Ölçümler (2026-08-10, gerçek feed — 282 ürün)
//!
//! | | |
//! |---|---|
//! | `buyingPrice` / `price1` dolu | **282 / 282** |
//! | KDV oranı | **276 ürün %20 · 6 ürün %10** |
//! | Marj medyanı | **%46,7** |
//! | 🔴 **Negatif marjlı ürün** | **7** (en düşük −%106,7) |
//! | %10 altı marj | 31 ürün |
//! | 🔴 **Katalog TEK para biriminde DEĞİL** | `currencyAbbr`: **USD 273 · EUR 8 · TL 1** |
//!
//! Son iki satır [`LOW_MARGIN_PCT`] uyarısının gerekçesi: uyarı süs değil, katalogda
//! gerçekten zararına satılan ürünler var ve teklif yazarken bunu görmek gerekiyor.
//!
//! ## 🔑 Para birimi bu modülde YOK — ve bu bilinçli
//!
//! Önce "katalog dolar" varsayılmıştı; ölçüm çürüttü (yukarıdaki son satır). Beyan güvenilir
//! çıktı: `priceTaxWithCur`'dan hesaplanan örtük kurlar beyanla birebir tutuyor
//! (USD 47,5906–47,5917 · EUR 54,9344 sabit · TL 1,0000).
//!
//! Çözüm çevrimi **satır eklenirken bir kez** yapmak: fiyat da maliyet de teklifin para
//! birimine dönüştürülüp öyle saklanıyor. Böylece bu modül tek bir birimde çalışıyor ve
//! kur mantığı aritmetiğe hiç bulaşmıyor. Yan faydası, teklifin **kayıt** olması: altı ay
//! sonra bakıldığında o günkü kur ve maliyet donmuş hâlde duruyor.

use serde::{Deserialize, Serialize};

/// Teklifin para birimi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Currency {
    Usd,
    Try,
}

impl Currency {
    pub fn code(&self) -> &'static str {
        match self {
            Currency::Usd => "USD",
            Currency::Try => "TRY",
        }
    }

    /// ⚠️ **Yalnızca TEKLİFİN para birimi için.** Teklif iki seçenekli, o yüzden bilinmeyen
    /// değer USD'ye düşüyor. Ürünün para birimi için KULLANMAYIN: katalogda üç değer var
    /// (USD 273 · EUR 8 · TL 1) ve bu fonksiyon EUR'yu USD sayar — nitekim bir kez öyle
    /// oldu ve EUR ürünler çevrilmeden geçti (test yakaladı, 2026-08-10).
    pub fn parse(s: &str) -> Currency {
        if s.eq_ignore_ascii_case("TRY") || s.eq_ignore_ascii_case("TL") {
            Currency::Try
        } else {
            Currency::Usd
        }
    }
}

/// Ürünün fiyatı zaten dolar cinsinden mi (`currencyAbbr`)?
///
/// Ayrı bir fonksiyon çünkü sorduğu soru farklı: burada "USD değilse çevir" gerekiyor,
/// [`Currency::parse`]'ta ise "TRY değilse USD say".
fn urun_dolarda(currency_abbr: &str) -> bool {
    currency_abbr.trim().eq_ignore_ascii_case("USD")
}

/// Katalog satırının teklif para birimine dönüştürülmüş fiyat + maliyeti.
///
/// 🔑 Çevrim burada, **satır eklenirken bir kez** yapılıyor (bkz. modül başlığı). Kaynaklar:
///
/// | Teklif | Ürün | Kaynak |
/// |---|---|---|
/// | TRY | hepsi | `priceTaxWithCur ÷ (1+KDV)` — mağazanın kendi TL fiyatı, **kur gerekmiyor** |
/// | USD | USD | `price1` — birebir |
/// | USD | EUR/TL | TL fiyatı ÷ kullanıcının USD/TRY kuru |
///
/// ⚠️ Üçüncü satır tek kur istiyor, N kur değil: `priceTaxWithCur` her ürün için mağazanın
/// **kendi** çevrimini zaten içeriyor, üzerine yalnızca USD/TRY gerekiyor.
///
/// `None` dönmesi "fiyat bilinmiyor" demek — çağıran kullanıcıdan istiyor, 0 uydurmuyor.
pub fn catalog_line(
    quote_currency: Currency,
    product_currency: &str,
    price1: Option<f64>,
    cost: Option<f64>,
    tax_rate: f64,
    price_tax_with_cur: Option<f64>,
    usd_try: Option<f64>,
) -> Option<(f64, Option<f64>)> {
    // Ürünün kendi biriminde fiyat/maliyet oranı: maliyet çevrimi fiyatınkiyle AYNI yoldan
    // gitmeli, yoksa marj kur farkından bozulur.
    let oran = |hedef: f64| -> Option<f64> {
        let p = price1.filter(|p| *p > 0.0)?;
        Some(hedef / p)
    };
    let tl_net = price_tax_with_cur.map(|t| round2(t / (1.0 + tax_rate / 100.0)));

    let birim = match quote_currency {
        Currency::Try => tl_net?,
        Currency::Usd => {
            if urun_dolarda(product_currency) {
                price1?
            } else {
                let k = usd_try.filter(|r| *r > 0.0)?;
                round2(tl_net? / k)
            }
        }
    };
    let maliyet = cost.and_then(|c| oran(birim).map(|k| round2(c * k)));
    Some((round2(birim), maliyet))
}

/// Altında marjın "düşük" sayıldığı yüzde.
///
/// 🔬 Ölçümle seçildi: katalogda 31 ürün bu eşiğin altında, 8'i %5'in altında ve 7'si
/// negatif. %10 uyarıyı anlamlı kılacak kadar dar, gürültü yapmayacak kadar geniş.
pub const LOW_MARGIN_PCT: f64 = 10.0;

/// Teklif satırı. `sku` yoksa elle eklenmiş kalem (montaj, nakliye).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line {
    pub name: String,
    pub qty: f64,
    /// **Teklifin para biriminde** birim fiyat — pazarlık edilen fiyat neyse o.
    pub unit_price: f64,
    /// KDV yüzdesi. Katalog satırında ürünün kendi oranı, elle satırda ayardaki varsayılan.
    pub tax_rate: f64,
    /// 🔴 Maliyet — **teklifin para biriminde** (satır eklenirken çevrildi, bkz.
    /// [`catalog_line`]). Elle satırda `None`.
    pub cost: Option<f64>,
}

/// Kuruş yuvarlaması — **tek yerde**.
///
/// ⚠️ Her satırda ayrı ayrı yuvarlanıyor, sonra toplanıyor. Tersi (toplayıp sonunda
/// yuvarlamak) müşterinin elle topladığında farklı bir sayı bulmasına yol açar: belgede
/// görünen satır tutarları neyse, toplam da onların toplamı olmalı.
pub fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

impl Line {
    /// Satırın KDV hariç tutarı.
    pub fn net(&self) -> f64 {
        round2(self.qty * self.unit_price)
    }
}

/// Bir KDV oranının kırılımı.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TaxRow {
    pub rate: f64,
    pub base: f64,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Totals {
    pub subtotal: f64,
    /// Orana göre gruplanmış KDV — katalogda iki oran olduğu için tek satır yetmiyor.
    pub taxes: Vec<TaxRow>,
    pub tax_total: f64,
    pub grand_total: f64,
}

/// Ara toplam, oran kırılımlı KDV ve genel toplam.
pub fn totals(lines: &[Line]) -> Totals {
    let subtotal = round2(lines.iter().map(Line::net).sum());

    // Oranlar gruplanıyor; sıra sabit (küçükten büyüğe) ki belge her üretimde aynı çıksın.
    let mut oranlar: Vec<f64> = lines.iter().map(|l| l.tax_rate).collect();
    oranlar.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    oranlar.dedup_by(|a, b| (*a - *b).abs() < f64::EPSILON);

    let taxes: Vec<TaxRow> = oranlar
        .into_iter()
        .filter(|r| *r > 0.0)
        .map(|rate| {
            let base = round2(
                lines
                    .iter()
                    .filter(|l| (l.tax_rate - rate).abs() < f64::EPSILON)
                    .map(Line::net)
                    .sum(),
            );
            TaxRow { rate, base, amount: round2(base * rate / 100.0) }
        })
        .filter(|t| t.base != 0.0)
        .collect();

    let tax_total = round2(taxes.iter().map(|t| t.amount).sum());
    Totals { subtotal, taxes, tax_total, grand_total: round2(subtotal + tax_total) }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MarginState {
    Ok,
    Low,
    Negative,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Margin {
    /// Kâr — **teklifin para biriminde** (fiyat ve maliyet aynı birimde).
    pub amount: f64,
    /// Satış üzerinden yüzde.
    pub pct: f64,
    pub state: MarginState,
}

impl Margin {
    fn new(satis: f64, maliyet: f64) -> Margin {
        let amount = round2(satis - maliyet);
        let pct = if satis > 0.0 { amount / satis * 100.0 } else { 0.0 };
        let state = if pct < 0.0 {
            MarginState::Negative
        } else if pct < LOW_MARGIN_PCT {
            MarginState::Low
        } else {
            MarginState::Ok
        };
        Margin { amount, pct: (pct * 10.0).round() / 10.0, state }
    }
}

/// Tek satırın marjı. Maliyeti bilinmeyen (elle) satırda `None`.
///
/// ⚠️ Kur hesabı YOK: fiyat da maliyet de satır eklenirken aynı birime çevrildi.
pub fn line_margin(l: &Line) -> Option<Margin> {
    let cost = l.cost?;
    Some(Margin::new(l.net(), round2(cost * l.qty)))
}

/// Teklifin toplam marjı.
///
/// ⚠️ **Yalnızca maliyeti bilinen satırlar** hesaba katılıyor; elle satırlar (nakliye,
/// montaj) marjı olduğundan düşük göstermesin diye dışarıda. Hiç maliyetli satır yoksa
/// `None` — "marj %0" demek yanlış olurdu.
pub fn quote_margin(lines: &[Line]) -> Option<Margin> {
    let maliyetli: Vec<&Line> = lines.iter().filter(|l| l.cost.is_some()).collect();
    if maliyetli.is_empty() {
        return None;
    }
    let satis = round2(maliyetli.iter().map(|l| l.net()).sum());
    let maliyet = round2(maliyetli.iter().map(|l| l.cost.unwrap_or(0.0) * l.qty).sum::<f64>());
    Some(Margin::new(satis, maliyet))
}

/// Teklif numarası: `T-<yıl>-<sıra>`.
///
/// Sıra **yıl içinde** artıyor; yıl değişince 001'e dönüyor. `last_no` geçen numaralardan
/// en büyüğü (aynı yıla ait değilse yok sayılıyor).
pub fn next_quote_no(year: i32, last_no: Option<&str>) -> String {
    let onek = format!("T-{year}-");
    let sira = last_no
        .filter(|n| n.starts_with(&onek))
        .and_then(|n| n[onek.len()..].parse::<u32>().ok())
        .unwrap_or(0)
        + 1;
    format!("{onek}{sira:03}")
}

/// Teklif durumları. `draft → sent → won | lost | expired`.
pub const STATUSES: &[&str] = &["draft", "sent", "won", "lost", "expired"];

/// Bu geçiş yapılabilir mi?
///
/// ⚠️ Kapanmış teklif (kazanıldı/kaybedildi) yeniden açılmıyor: teklif bir **kayıt**, sonucu
/// değiştirilecek bir taslak değil. Yanlış kapatıldıysa yeni sürüm/yeni teklif açılır.
pub fn can_transition(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("draft", "sent")
            | ("sent", "won")
            | ("sent", "lost")
            | ("sent", "expired")
            | ("sent", "draft")   // revizyon için taslağa geri alma
            | ("expired", "sent") // süresi dolan teklif yenilenip yeniden gönderilebilir
    )
}

pub fn status_label(s: &str) -> &'static str {
    match s {
        "draft" => "Taslak",
        "sent" => "Gönderildi",
        "won" => "Kazanıldı",
        "lost" => "Kaybedildi",
        "expired" => "Süresi doldu",
        _ => "—",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn satir(qty: f64, fiyat: f64, kdv: f64, maliyet: Option<f64>) -> Line {
        Line {
            name: "Ürün".into(),
            qty,
            unit_price: fiyat,
            tax_rate: kdv,
            cost: maliyet,
        }
    }

    /// 🔑 Katalogda İKİ oran var (ölçüldü: 276 ürün %20, 6 ürün %10) — tek satırlık KDV
    /// gösterimi yanlış olurdu.
    #[test]
    fn kdv_orana_gore_kirilyor() {
        let t = totals(&[
            satir(2.0, 100.0, 20.0, None),
            satir(1.0, 50.0, 10.0, None),
            satir(3.0, 10.0, 20.0, None),
        ]);
        assert_eq!(t.subtotal, 280.0);
        assert_eq!(t.taxes.len(), 2, "iki oran iki satır");
        assert_eq!(t.taxes[0], TaxRow { rate: 10.0, base: 50.0, amount: 5.0 });
        assert_eq!(t.taxes[1], TaxRow { rate: 20.0, base: 230.0, amount: 46.0 });
        assert_eq!(t.tax_total, 51.0);
        assert_eq!(t.grand_total, 331.0);
    }

    /// ⚠️ Belgede görünen satır tutarları neyse, toplam da onların toplamı olmalı — müşteri
    /// elle topladığında farklı bir sayı bulmamalı.
    #[test]
    fn kurus_yuvarlamasi_toplami_bozmuyor() {
        let t = totals(&[satir(3.0, 33.333, 20.0, None), satir(3.0, 33.333, 20.0, None)]);
        // Satır tutarı 99,999 → 100,00; iki satır 200,00.
        assert_eq!(t.subtotal, 200.0);
        assert_eq!(t.tax_total, 40.0);
        assert_eq!(t.grand_total, 240.0);
    }

    #[test]
    fn kdv_sifirsa_satir_acilmiyor() {
        let t = totals(&[satir(1.0, 100.0, 0.0, None)]);
        assert!(t.taxes.is_empty());
        assert_eq!(t.grand_total, 100.0);
    }

    /// 🔴 Katalogda 7 ürün zararına: uyarı süs değil.
    #[test]
    fn negatif_marj_isaretleniyor() {
        let m = line_margin(&satir(1.0, 100.0, 20.0, Some(207.0))).unwrap();
        assert_eq!(m.state, MarginState::Negative);
        assert_eq!(m.amount, -107.0);
        assert!((m.pct - -107.0).abs() < 0.05);

        let dusuk = line_margin(&satir(1.0, 100.0, 20.0, Some(95.0))).unwrap();
        assert_eq!(dusuk.state, MarginState::Low, "%5 marj düşük sayılmalı");

        let iyi = line_margin(&satir(1.0, 749.0, 20.0, Some(600.0))).unwrap();
        assert_eq!(iyi.state, MarginState::Ok, "gerçek katalog satırı: 600 → 749");
    }

    /// Elle satırlar (nakliye, montaj) marjı olduğundan düşük göstermemeli.
    #[test]
    fn maliyetsiz_satirlar_marja_katilmiyor() {
        let lines = [satir(1.0, 749.0, 20.0, Some(600.0)), satir(1.0, 500.0, 20.0, None)];
        let m = quote_margin(&lines).unwrap();
        assert_eq!(m.amount, 149.0, "yalnızca maliyeti bilinen satır");

        // Hiç maliyetli satır yoksa marj yok — "%0" demek yanlış olurdu.
        assert!(quote_margin(&[satir(1.0, 500.0, 20.0, None)]).is_none());
    }

    /// 🔴 Ölçüm: katalog tek para biriminde değil (USD 273 · EUR 8 · TL 1). Gerçek
    /// satırlarla üç yol da sınanıyor.
    #[test]
    fn katalog_satiri_teklif_para_birimine_ceviriliyor() {
        // USD teklif + USD ürün → price1 birebir (gerçek satır: 600 → 749).
        let (p, c) = catalog_line(Currency::Usd, "USD", Some(749.0), Some(600.0), 20.0,
                                  Some(42774.88), None).unwrap();
        assert_eq!((p, c), (749.0, Some(600.0)));

        // TRY teklif → mağazanın kendi TL fiyatı, KUR GEREKMİYOR (42774,88 ÷ 1,20).
        let (p, c) = catalog_line(Currency::Try, "USD", Some(749.0), Some(600.0), 20.0,
                                  Some(42774.88), None).unwrap();
        assert_eq!(p, 35645.73);
        // Maliyet fiyatla AYNI oranda çevrildi → marj yüzdesi korunuyor.
        let marj = line_margin(&Line { name: "x".into(), qty: 1.0, unit_price: p,
                                       tax_rate: 20.0, cost: c }).unwrap();
        assert!((marj.pct - 19.9).abs() < 0.2, "USD'deki %19,9 marj TL'de de aynı kalmalı");

        // USD teklif + EUR ürün → TL fiyatı üzerinden tek kurla (8 ürünün yolu).
        let (p, _) = catalog_line(Currency::Usd, "EUR", Some(133.58), Some(100.0), 20.0,
                                  Some(8805.76), Some(47.5911)).unwrap();
        assert!((p - 154.2).abs() < 0.2, "EUR ürün USD teklifte doğru çevrilmeli, {p}");

        // ⚠️ Kur yoksa fiyat UYDURULMUYOR.
        assert!(catalog_line(Currency::Usd, "EUR", Some(133.58), Some(100.0), 20.0,
                             Some(8805.76), None).is_none());
    }

    #[test]
    fn teklif_numarasi_yil_basinda_bire_donuyor() {
        assert_eq!(next_quote_no(2026, None), "T-2026-001");
        assert_eq!(next_quote_no(2026, Some("T-2026-007")), "T-2026-008");
        assert_eq!(next_quote_no(2026, Some("T-2026-099")), "T-2026-100");
        // Geçen yılın numarası sırayı taşımıyor.
        assert_eq!(next_quote_no(2027, Some("T-2026-042")), "T-2027-001");
    }

    /// ⚠️ Kapanmış teklif yeniden açılmıyor: teklif bir kayıt, sonucu düzeltilecek bir
    /// taslak değil.
    #[test]
    fn kapanmis_teklif_yeniden_acilmiyor() {
        assert!(can_transition("draft", "sent"));
        assert!(can_transition("sent", "won"));
        assert!(can_transition("sent", "draft"), "revizyon için geri alma");
        assert!(!can_transition("won", "sent"));
        assert!(!can_transition("lost", "draft"));
        assert!(!can_transition("draft", "won"), "gönderilmeden kazanılamaz");
    }
}
