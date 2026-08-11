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

/// Belgeyi üretir. Stil **satır içi**: mail istemcileri `<style>` bloğunu sıklıkla atıyor.
pub fn render(q: &QuoteOut) -> String {
    let mut h = String::with_capacity(4096);
    let cizgi = "border-bottom:1px solid #e5e5e7;";
    let sag = "text-align:right;";

    h.push_str(
        "<div style=\"font-family:-apple-system,Segoe UI,Roboto,Helvetica,Arial,sans-serif;\
         color:#1d1d1f;font-size:13px;line-height:1.5;max-width:720px;\">",
    );

    // Başlık
    h.push_str("<div style=\"display:flex;justify-content:space-between;align-items:flex-start;\
                margin-bottom:18px;\"><div>");
    if !q.seller.trim().is_empty() {
        h.push_str(&format!(
            "<div style=\"font-size:17px;font-weight:640;\">{}</div>",
            esc(&q.seller)
        ));
    }
    h.push_str(&format!(
        "<div style=\"color:#6e6e73;font-size:12px;margin-top:2px;\">Teklif {}</div>",
        esc(&q.no)
    ));
    h.push_str("</div><div style=\"text-align:right;color:#6e6e73;font-size:12px;\">");
    h.push_str(&format!("<div>Tarih: {}</div>", esc(&q.date)));
    if !q.valid_until.trim().is_empty() {
        h.push_str(&format!("<div>Geçerlilik: {}</div>", esc(&q.valid_until)));
    }
    h.push_str("</div></div>");

    if !q.buyer.trim().is_empty() {
        h.push_str(&format!(
            "<div style=\"margin-bottom:14px;\"><span style=\"color:#6e6e73;font-size:12px;\">\
             Sayın</span><div style=\"font-weight:600;\">{}</div></div>",
            esc(&q.buyer)
        ));
    }

    // Satırlar
    h.push_str("<table style=\"width:100%;border-collapse:collapse;\"><thead><tr>");
    for (baslik, hiza) in [("Kalem", ""), ("Adet", sag), ("Birim", sag), ("KDV", sag), ("Tutar", sag)]
    {
        h.push_str(&format!(
            "<th style=\"{hiza}{cizgi}padding:6px 8px;font-size:11px;color:#6e6e73;\
             font-weight:600;\">{baslik}</th>"
        ));
    }
    h.push_str("</tr></thead><tbody>");
    for l in &q.lines {
        h.push_str(&format!(
            "<tr>\
             <td style=\"{cizgi}padding:7px 8px;\">{}</td>\
             <td style=\"{sag}{cizgi}padding:7px 8px;\">{}</td>\
             <td style=\"{sag}{cizgi}padding:7px 8px;\">{}</td>\
             <td style=\"{sag}{cizgi}padding:7px 8px;\">%{}</td>\
             <td style=\"{sag}{cizgi}padding:7px 8px;font-weight:600;\">{}</td>\
             </tr>",
            esc(&l.name),
            tr_qty(l.qty),
            tr_num(l.unit_price),
            tr_qty(l.tax_rate),
            tr_num(l.net),
        ));
    }
    h.push_str("</tbody></table>");

    // Toplamlar
    h.push_str(
        "<table style=\"margin-left:auto;margin-top:12px;border-collapse:collapse;\
         font-size:12.5px;\">",
    );
    let satir = |ad: &str, deger: String, kalin: bool| {
        format!(
            "<tr><td style=\"padding:3px 12px 3px 0;color:#6e6e73;\">{ad}</td>\
             <td style=\"{sag}padding:3px 0;{}\">{deger}</td></tr>",
            if kalin { "font-weight:660;font-size:14px;color:#1d1d1f;" } else { "" }
        )
    };
    h.push_str(&satir("Ara toplam", format!("{} {}", tr_num(q.subtotal), esc(&q.currency)), false));
    for (oran, matrah, tutar) in &q.taxes {
        h.push_str(&satir(
            &format!("KDV %{} ({} üzerinden)", tr_qty(*oran), tr_num(*matrah)),
            tr_num(*tutar),
            false,
        ));
    }
    h.push_str(&satir(
        "Genel toplam",
        format!("{} {}", tr_num(q.grand_total), esc(&q.currency)),
        true,
    ));
    h.push_str("</table>");

    if !q.fx_note.trim().is_empty() {
        h.push_str(&format!(
            "<div style=\"margin-top:10px;color:#6e6e73;font-size:11.5px;\">{}</div>",
            esc(&q.fx_note)
        ));
    }
    for metin in [&q.note, &q.footer] {
        if !metin.trim().is_empty() {
            h.push_str(&format!(
                "<div style=\"margin-top:12px;white-space:pre-wrap;font-size:12px;\">{}</div>",
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
    #[test]
    fn iki_kdv_orani_ayri_satirda() {
        let mut q = ornek();
        q.taxes = vec![(10.0, 50.0, 5.0), (20.0, 2597.0, 519.4)];
        let html = render(&q);
        assert!(html.contains("KDV %10 (50,00 üzerinden)"));
        assert!(html.contains("KDV %20 (2.597,00 üzerinden)"));
    }
}
