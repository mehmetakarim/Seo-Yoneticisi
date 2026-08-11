//! # Teklif belgesi — müşteriye giden HTML (Faz T)
//!
//! Yol haritasının kararı: **entegrasyon yok**. Belge panoya kopyalanıp mail'e yapıştırılıyor,
//! ya da tarayıcıdan yazdırılıp PDF'e döküyor. Yeni bağımlılık yok.
//!
//! ## 🔴 Maliyet buraya GİREMEZ — ve bu bir dikkat meselesi değil
//!
//! Bu fazın en tehlikeli tek şeyi, maliyet fiyatının müşteriye giden bir belgede görünmesi.
//! Engel yapısal: [`QuoteOut`] ve [`OutLine`] yapılarında **maliyet ve marj alanı yok**.
//! [`render`] yalnızca bu yapıları alıyor, yani göremediği bir şeyi basamıyor.
//!
//! `commands::quotes::Quote` tipinde maliyet var (marj ekranda gösteriliyor); dönüşüm
//! bilinçli olarak **kayıplı**. Bir gün buraya maliyet eklenmek istenirse önce yapıyı
//! değiştirmek gerekir — yani karar görünür olur, kazara olmaz.
//!
//! ## Neden kendi HTML'imizi yazıyoruz
//!
//! Şablon motoru eklemek bir bağımlılık; belge tek ve sabit. `jsonld.rs` ve
//! `ideasoft::tech_table_html` aynı deseni izliyor: saf fonksiyon, metin çıktısı, testlerle
//! sabitlenmiş.

use serde::{Deserialize, Serialize};

/// Belgede görünen satır. ⚠️ Maliyet ve marj **yok** — modül başlığındaki gerekçe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutLine {
    pub name: String,
    pub qty: f64,
    pub unit_price: f64,
    pub tax_rate: f64,
    pub net: f64,
}

/// Belgenin tamamı. ⚠️ Maliyet ve marj **yok**.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteOut {
    pub no: String,
    /// Teklif tarihi (YYYY-AA-GG).
    pub date: String,
    pub valid_until: String,
    pub currency: String,
    /// Satıcı adı — Ayarlar'dan. Boşsa başlık yazılmıyor (uygulama kişiselleştirilmemiş).
    pub seller: String,
    /// Müşteri adı + firma.
    pub buyer: String,
    /// "1 USD = 47,59 TL · 10.08.2026" — yalnızca kur girilmişse.
    pub fx_note: String,
    pub lines: Vec<OutLine>,
    pub subtotal: f64,
    /// (oran, matrah, tutar) üçlüleri.
    pub taxes: Vec<(f64, f64, f64)>,
    pub grand_total: f64,
    pub note: String,
    /// Ayarlardan gelen sabit dipnot (ödeme koşulu, teslim vb.).
    pub footer: String,
}

/// HTML kaçışı. Ürün adlarında `&`, `<`, tırnak geçebiliyor.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// Türkçe biçimli sayı: binlik nokta, ondalık virgül, iki hane.
///
/// ⚠️ `Intl` yok (burası Rust); biçim elle kuruluyor ve testle sabitleniyor. Ekran tarafı
/// `Intl.NumberFormat("tr-TR")` kullanıyor — ikisi aynı sonucu vermeli.
pub fn tr_num(v: f64) -> String {
    let neg = v < 0.0;
    let yuvarlak = (v.abs() * 100.0).round() / 100.0;
    let tam = yuvarlak.trunc() as i64;
    let kurus = ((yuvarlak - tam as f64) * 100.0).round() as i64;

    let mut basamak = String::new();
    for (i, c) in tam.to_string().chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            basamak.push('.');
        }
        basamak.push(c);
    }
    let tam_str: String = basamak.chars().rev().collect();
    format!("{}{tam_str},{kurus:02}", if neg { "-" } else { "" })
}

/// Miktar: tam sayıysa ondalık yazılmıyor ("2" değil "2,00").
fn tr_qty(v: f64) -> String {
    if (v - v.round()).abs() < 0.001 {
        format!("{}", v.round() as i64)
    } else {
        tr_num(v)
    }
}

/// Belgeyi üretir.
///
/// ## Neden bu biçim — üç kısıt birden
///
/// 1. **Stil satır içi.** Mail istemcileri `<style>` bloğunu sıklıkla atıyor.
/// 2. **Flexbox YOK, düzen tablolarla.** Outlook flex'i yok sayıyor ve başlık üst üste
///    biner. Saha geri bildirimi (2026-08-11) tabloların mailde **çok geniş** düştüğünü
///    gösterdi; genişlik 720 → **560 px** ve tablo `width:100%` yerine sabit genişliğin
///    içinde duruyor.
/// 3. **Uygulamanın dili.** Ağır kutu çizgileri yerine saç teli ayırıcılar, sayılar
///    `tabular-nums`, ikincil metin soluk. *"Boşluk da bir özelliktir"* — belge bir fatura
///    taklidi değil, sakin bir mektup.
pub fn render(q: &QuoteOut) -> String {
    let mut h = String::with_capacity(4096);
    // Tek yerde tutulan ölçüler; belge boyunca aynı ritim.
    let govde = "font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,Helvetica,Arial,\
                 sans-serif;color:#1d1d1f;font-size:12.5px;line-height:1.55;";
    let soluk = "color:#86868b;";
    let tel = "border-bottom:1px solid #f0f0f2;";
    let sag = "text-align:right;";
    let sayi = "font-variant-numeric:tabular-nums;";

    h.push_str(&format!(
        "<div style=\"{govde}max-width:560px;margin:0;padding:0;\">"
    ));

    // --- Başlık: iki hücreli tablo (flex Outlook'ta çalışmıyor) ---
    h.push_str("<table style=\"width:100%;border-collapse:collapse;\"><tr>");
    h.push_str("<td style=\"vertical-align:top;padding:0;\">");
    if !q.seller.trim().is_empty() {
        h.push_str(&format!(
            "<div style=\"font-size:15px;font-weight:650;letter-spacing:-0.01em;\">{}</div>",
            esc(&q.seller)
        ));
    }
    h.push_str(&format!(
        "<div style=\"{soluk}font-size:11.5px;margin-top:1px;\">Teklif {}</div>",
        esc(&q.no)
    ));
    h.push_str("</td>");
    h.push_str(&format!(
        "<td style=\"vertical-align:top;padding:0;{sag}{soluk}font-size:11.5px;\">"
    ));
    h.push_str(&format!("<div>{}</div>", esc(&q.date)));
    if !q.valid_until.trim().is_empty() {
        h.push_str(&format!("<div>Geçerlilik {}</div>", esc(&q.valid_until)));
    }
    h.push_str("</td></tr></table>");

    if !q.buyer.trim().is_empty() {
        h.push_str(&format!(
            "<div style=\"margin-top:18px;\"><span style=\"{soluk}font-size:11.5px;\">Sayın</span>\
             <div style=\"font-weight:600;\">{}</div></div>",
            esc(&q.buyer)
        ));
    }

    // --- Kalemler ---
    h.push_str(&format!(
        "<table style=\"width:100%;border-collapse:collapse;margin-top:16px;\"><thead><tr>"
    ));
    // ⚠️ Başlıklar küçük ve harf aralıklı: tablo "form" değil "liste" gibi okunsun.
    let bas = format!(
        "padding:0 0 6px;border-bottom:1px solid #d2d2d7;font-size:10px;font-weight:600;\
         letter-spacing:0.04em;text-transform:uppercase;{soluk}"
    );
    h.push_str(&format!("<th style=\"text-align:left;{bas}\">Kalem</th>"));
    for baslik in ["Adet", "Birim", "KDV", "Tutar"] {
        h.push_str(&format!("<th style=\"{sag}{bas}padding-left:10px;\">{baslik}</th>"));
    }
    h.push_str("</tr></thead><tbody>");
    for l in &q.lines {
        h.push_str(&format!(
            "<tr>\
             <td style=\"{tel}padding:8px 0;\">{}</td>\
             <td style=\"{sag}{tel}{sayi}padding:8px 0 8px 10px;{soluk}\">{}</td>\
             <td style=\"{sag}{tel}{sayi}padding:8px 0 8px 10px;{soluk}\">{}</td>\
             <td style=\"{sag}{tel}{sayi}padding:8px 0 8px 10px;{soluk}\">%{}</td>\
             <td style=\"{sag}{tel}{sayi}padding:8px 0 8px 10px;font-weight:600;\">{}</td>\
             </tr>",
            esc(&l.name),
            tr_qty(l.qty),
            tr_num(l.unit_price),
            tr_qty(l.tax_rate),
            tr_num(l.net),
        ));
    }
    h.push_str("</tbody></table>");

    // --- Toplamlar: sağa yaslı, yalnızca genel toplamın üstünde çizgi ---
    h.push_str(&format!(
        "<table style=\"border-collapse:collapse;margin:14px 0 0 auto;font-size:12px;\">"
    ));
    let satir = |ad: String, deger: String, genel: bool| {
        let ust = if genel { "border-top:1px solid #d2d2d7;padding-top:8px;" } else { "" };
        format!(
            "<tr><td style=\"padding:3px 18px 3px 0;{ust}{}\">{ad}</td>\
             <td style=\"{sag}{sayi}padding:3px 0;{ust}{}\">{deger}</td></tr>",
            if genel { "" } else { soluk },
            if genel { "font-weight:660;font-size:14px;" } else { "" }
        )
    };
    h.push_str(&satir(
        "Ara toplam".into(),
        format!("{} {}", tr_num(q.subtotal), esc(&q.currency)),
        false,
    ));
    for (oran, matrah, tutar) in &q.taxes {
        h.push_str(&satir(
            format!("KDV %{} · {}", tr_qty(*oran), tr_num(*matrah)),
            tr_num(*tutar),
            false,
        ));
    }
    h.push_str(&satir(
        "Genel toplam".into(),
        format!("{} {}", tr_num(q.grand_total), esc(&q.currency)),
        true,
    ));
    h.push_str("</table>");

    if !q.fx_note.trim().is_empty() {
        h.push_str(&format!(
            "<div style=\"{sag}{soluk}font-size:11px;margin-top:6px;\">{}</div>",
            esc(&q.fx_note)
        ));
    }
    for (i, metin) in [&q.note, &q.footer].iter().enumerate() {
        if !metin.trim().is_empty() {
            let ust = if i == 0 { "margin-top:20px;" } else { "margin-top:10px;" };
            h.push_str(&format!(
                "<div style=\"{ust}white-space:pre-wrap;font-size:11.5px;{}\">{}</div>",
                if i == 1 { soluk } else { "" },
                esc(metin.trim())
            ));
        }
    }
    h.push_str("</div>");
    h
}

/// Düz metin karşılığı — HTML'i kabul etmeyen mail istemcileri için.
///
/// ⚠️ Panoya **iki biçim birden** konuyor (`text/html` + `text/plain`); yapıştırılan yer
/// hangisini destekliyorsa onu alıyor.
pub fn render_text(q: &QuoteOut) -> String {
    let mut t = String::new();
    if !q.seller.trim().is_empty() {
        t.push_str(&format!("{}\n", q.seller));
    }
    t.push_str(&format!("Teklif {} · {}\n", q.no, q.date));
    if !q.valid_until.trim().is_empty() {
        t.push_str(&format!("Geçerlilik: {}\n", q.valid_until));
    }
    if !q.buyer.trim().is_empty() {
        t.push_str(&format!("Sayın {}\n", q.buyer));
    }
    t.push('\n');
    for l in &q.lines {
        t.push_str(&format!(
            "{} — {} x {} (KDV %{}) = {} {}\n",
            l.name,
            tr_qty(l.qty),
            tr_num(l.unit_price),
            tr_qty(l.tax_rate),
            tr_num(l.net),
            q.currency
        ));
    }
    t.push_str(&format!("\nAra toplam: {} {}\n", tr_num(q.subtotal), q.currency));
    for (oran, matrah, tutar) in &q.taxes {
        t.push_str(&format!(
            "KDV %{} ({} üzerinden): {}\n",
            tr_qty(*oran),
            tr_num(*matrah),
            tr_num(*tutar)
        ));
    }
    t.push_str(&format!("Genel toplam: {} {}\n", tr_num(q.grand_total), q.currency));
    if !q.fx_note.trim().is_empty() {
        t.push_str(&format!("{}\n", q.fx_note));
    }
    for metin in [&q.note, &q.footer] {
        if !metin.trim().is_empty() {
            t.push_str(&format!("\n{}\n", metin.trim()));
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gerçek katalog satırları: 600→749 ve 860→949.
    fn ornek() -> QuoteOut {
        QuoteOut {
            no: "T-2026-001".into(),
            date: "10.08.2026".into(),
            valid_until: "25.08.2026".into(),
            currency: "USD".into(),
            seller: "Kurumsal BT".into(),
            buyer: "Ahmet Yılmaz · Anadolu Yapı".into(),
            fx_note: String::new(),
            lines: vec![
                OutLine {
                    name: "Bambu Lab P2S Combo".into(),
                    qty: 1.0,
                    unit_price: 749.0,
                    tax_rate: 20.0,
                    net: 749.0,
                },
                OutLine {
                    name: "Lenovo ThinkCentre <Neo> & 50q".into(),
                    qty: 2.0,
                    unit_price: 949.0,
                    tax_rate: 20.0,
                    net: 1898.0,
                },
            ],
            subtotal: 2647.0,
            taxes: vec![(20.0, 2647.0, 529.4)],
            grand_total: 3176.4,
            note: String::new(),
            footer: String::new(),
        }
    }

    /// Etiketleri ve öznitelikleri atıp **müşterinin gördüğü** metni bırakır.
    ///
    /// ⚠️ Gerekli çünkü ham HTML'de arama yanıltıyor: ilk hâlinde test `font-weight:600`
    /// yüzünden "600 sızdı" diye patladı. Asıl soru belgede ne YAZDIĞI, biçim değerlerinde
    /// hangi rakamların geçtiği değil.
    fn gorunur_metin(html: &str) -> String {
        let mut out = String::new();
        let mut etiket_icinde = false;
        for c in html.chars() {
            match c {
                '<' => etiket_icinde = true,
                '>' => {
                    etiket_icinde = false;
                    out.push(' ');
                }
                _ if !etiket_icinde => out.push(c),
                _ => {}
            }
        }
        out
    }

    /// 🔴 **Fazın en kritik testi.** Maliyet değerleri belgeye hiçbir biçimde sızmamalı.
    ///
    /// Yapı zaten maliyet taşımıyor (derleyici engeli), bu test o güvenceyi **davranış
    /// düzeyinde** de sabitliyor: biri ileride `QuoteOut`a maliyet eklerse burası patlar.
    #[test]
    fn maliyet_belgeye_sizmiyor() {
        let q = ornek();
        let gorunen = gorunur_metin(&render(&q));
        let metin = render_text(&q);
        // Gerçek maliyetler: 600 ve 860 (ve biçimlenmiş hâlleri, toplamları).
        for yasak in ["600", "860", "1.720", "2.320", "maliyet", "marj", "kâr"] {
            assert!(!gorunen.contains(yasak), "belgede yasaklı değer görünüyor: {yasak}");
            assert!(!metin.contains(yasak), "düz metinde yasaklı değer geçiyor: {yasak}");
        }
        // Satış fiyatları ise görünmeli.
        assert!(gorunen.contains("749,00") && gorunen.contains("949,00"));
        assert!(metin.contains("749,00"));
    }

    /// Ürün adındaki `<`, `>` ve `&` belgeyi bozmamalı.
    #[test]
    fn html_kacisi_yapiliyor() {
        let html = render(&ornek());
        assert!(html.contains("Lenovo ThinkCentre &lt;Neo&gt; &amp; 50q"));
        assert!(!html.contains("<Neo>"), "ham etiket kaçmamış");
    }

    /// ⚠️ Ekran `Intl.NumberFormat("tr-TR")` kullanıyor; buradaki biçim onunla aynı olmalı.
    #[test]
    fn turkce_sayi_bicimi() {
        assert_eq!(tr_num(1234.5), "1.234,50");
        assert_eq!(tr_num(0.0), "0,00");
        assert_eq!(tr_num(42774.88), "42.774,88");
        assert_eq!(tr_num(-107.0), "-107,00");
        assert_eq!(tr_num(1_234_567.891), "1.234.567,89");
        // Adet ve KDV oranı tam sayıysa ondalık yazılmıyor.
        assert_eq!(tr_qty(2.0), "2");
        assert_eq!(tr_qty(20.0), "20");
        assert_eq!(tr_qty(1.5), "1,50");
    }

    /// Boş alanlar (satıcı, kur notu, dipnot) belgede **hiç** yer kaplamamalı.
    #[test]
    fn bos_alanlar_cizilmiyor() {
        let mut q = ornek();
        q.seller = String::new();
        q.buyer = String::new();
        let html = render(&q);
        assert!(!html.contains("Sayın"));
        assert!(html.contains("Teklif T-2026-001"));

        q.fx_note = "1 USD = 47,59 TL · 10.08.2026".into();
        assert!(render(&q).contains("1 USD = 47,59 TL"));
    }

    /// İki KDV oranı belgede ayrı satırlarda — katalogda %20 ve %10 birlikte var.
    ///
    /// ⚠️ Belgedeki yazım kısa (`KDV %10 · 50,00`), düz metindeki uzun
    /// (`KDV %10 (50,00 üzerinden)`): belge 560 px'e sığmak zorunda, düz metinde yer var.
    #[test]
    fn iki_kdv_orani_ayri_satirda() {
        let mut q = ornek();
        q.taxes = vec![(10.0, 50.0, 5.0), (20.0, 2597.0, 519.4)];
        let gorunen = gorunur_metin(&render(&q));
        assert!(gorunen.contains("KDV %10 · 50,00"), "{gorunen}");
        assert!(gorunen.contains("KDV %20 · 2.597,00"));

        let metin = render_text(&q);
        assert!(metin.contains("KDV %10 (50,00 üzerinden)"));
    }

    /// 🔧 Harness örneğini üretir — elle yazılmış bir örnek gerçek çıktıdan sapardı.
    ///
    /// `cargo test -p seo-core belge_ornegi_yaz -- --ignored --nocapture > /tmp/belge.json`
    #[test]
    #[ignore]
    fn belge_ornegi_yaz() {
        let mut q = ornek();
        q.lines[1].name = "Lenovo ThinkCentre Neo 50q G5".into();
        q.footer = "Fiyatlarımız 15 gün geçerlidir. Teslim: stoktan 2 iş günü.".into();
        let cikti = serde_json::json!({ "html": render(&q), "text": render_text(&q) });
        println!("{}", serde_json::to_string(&cikti).unwrap());
    }

    /// 🔴 Saha geri bildirimi (2026-08-11): *"mail'e yapıştırdığım tablo biraz fazla geniş."*
    ///
    /// Belge 560 px'e sabit ve düzen **tablolarla** kuruluyor — Outlook flexbox'ı yok sayıyor,
    /// başlık üst üste binerdi.
    #[test]
    fn belge_dar_ve_mail_uyumlu() {
        let html = render(&ornek());
        assert!(html.contains("max-width:560px"), "belge daraltılmalı");
        assert!(!html.contains("display:flex"), "mail istemcileri flex'i yok sayıyor");
        // Sayılar hizalı: tabular figürler olmadan sütunlar kayıyor.
        assert!(html.contains("font-variant-numeric:tabular-nums"));
    }
}
