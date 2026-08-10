//! # CSV içe aktarma — mevcut müşteri listesini uygulamaya taşımak (Faz C)
//!
//! Kişileri tek tek elle girmek gerçekçi değil; CRM boş kalırsa Faz C'nin tamamı boşa gider.
//!
//! ## ⚠️ Neden hazır bir crate değil
//!
//! `csv` crate'i işi yapardı ama buradaki asıl zorluk ayrıştırma değil, **Excel'in Türkçe
//! yerel ayarı**: dosya `;` ile ayrılıyor, başında BOM oluyor ve sütun adları her mağazada
//! farklı. Bunların hepsi yine elle yazılacaktı. 120 satır kod, bir bağımlılıktan ucuz.
//!
//! ## 🔴 Sabit sütun şeması YOK
//!
//! Uygulama kişiselleştirilmemiş: hangi mağazanın hangi CSV'yi vereceği bilinmiyor. Bu yüzden
//! ayrıştırıcı **başlıkları okur**, [`guess_mapping`] bir tahmin üretir ve son sözü kullanıcı
//! söyler. Tahmini "akıllı" olmaya çalışmıyor; yanıldığında düzeltmesi bir tıklama.

use serde::Serialize;

/// Aday ayraçlar — Türkçe Excel `;`, İngilizce `,`, pano yapıştırması `\t`.
const DELIMITERS: [char; 3] = [';', ',', '\t'];

/// Önizlemede gösterilen satır sayısı.
///
/// ⚠️ Önizleme **zorunlu adım**: 300 satırlık yanlış eşleşmiş bir aktarım geri alınamaz.
/// Beş satırın maliyeti bir ekran, faydası bir felaketin önlenmesi.
pub const PREVIEW_ROWS: usize = 5;

/// Uygulamanın doldurabileceği alanlar. Eşleştirme ekranı bu listeyi gösteriyor.
pub const FIELDS: &[(&str, &str)] = &[
    ("name", "Ad soyad"),
    ("company", "Firma"),
    ("email", "E-posta"),
    ("phone", "Telefon"),
    ("channel", "Kanal"),
    ("note", "Not"),
];

/// Ayrıştırılmış dosya: başlıklar + satırlar.
#[derive(Debug, Serialize, PartialEq)]
pub struct Parsed {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// Sezilen ayraç — ekranda söyleniyor ("noktalı virgülle ayrılmış").
    pub delimiter: String,
}

/// Metni CSV olarak ayrıştırır.
///
/// Ayraç **başlık satırından** seziliyor: veri satırlarında bir not alanı virgül taşıyabilir,
/// başlık satırı ise sade olur.
pub fn parse(text: &str) -> Result<Parsed, String> {
    // BOM ilk sütun adını bozar: "\u{feff}ad" hiçbir eşleşmeye uymaz ve kullanıcı sebebini
    // ekranda göremez (görünmez karakter).
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let ilk_satir = text.lines().next().unwrap_or("").to_string();
    if ilk_satir.trim().is_empty() {
        return Err("Dosya boş görünüyor.".into());
    }

    let d = DELIMITERS
        .iter()
        .copied()
        .max_by_key(|c| ilk_satir.matches(*c).count())
        .filter(|c| ilk_satir.contains(*c))
        // Tek sütunlu dosya da geçerli (yalnızca e-posta listesi olabilir).
        .unwrap_or(';');

    let mut satirlar = split_rows(text, d);
    if satirlar.is_empty() {
        return Err("Dosya boş görünüyor.".into());
    }
    let headers: Vec<String> = satirlar.remove(0).into_iter().map(|h| h.trim().to_string()).collect();
    // Tamamen boş satırlar atılıyor: Excel dosya sonuna sıklıkla bir tane bırakıyor.
    satirlar.retain(|r| r.iter().any(|h| !h.trim().is_empty()));

    Ok(Parsed { headers, rows: satirlar, delimiter: d.to_string() })
}

/// RFC 4180 tarzı ayrıştırma: tırnaklı alan, alan içinde ayraç/satır sonu, `""` kaçışı.
fn split_rows(text: &str, d: char) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut alan = String::new();
    let mut tirnakta = false;
    let mut it = text.chars().peekable();

    while let Some(c) = it.next() {
        match c {
            '"' if tirnakta && it.peek() == Some(&'"') => {
                // `""` → tek tırnak (alan içinde tırnak kaçışı).
                alan.push('"');
                it.next();
            }
            '"' => tirnakta = !tirnakta,
            _ if c == d && !tirnakta => row.push(std::mem::take(&mut alan)),
            '\r' if !tirnakta => {} // CRLF: \r yutuluyor, satırı \n bitiriyor.
            '\n' if !tirnakta => {
                row.push(std::mem::take(&mut alan));
                rows.push(std::mem::take(&mut row));
            }
            _ => alan.push(c),
        }
    }
    if !alan.is_empty() || !row.is_empty() {
        row.push(alan);
        rows.push(row);
    }
    rows
}

/// Başlıklardan alan tahmini: `FIELDS` anahtarı → sütun indeksi.
///
/// Tahmin bilinçli olarak **basit**: küçük harfe indirip anahtar kelime arıyor. Daha zekisi
/// (bulanık eşleşme) yanıldığında kullanıcı **neden** yanıldığını anlayamaz; buradaki hata
/// açıklanabilir ve düzeltmesi bir seçim.
pub fn guess_mapping(headers: &[String]) -> Vec<Option<usize>> {
    let ipuclari: &[(&str, &[&str])] = &[
        ("name", &["ad soyad", "adı soyadı", "isim", "ad", "name", "kişi", "yetkili"]),
        ("company", &["firma", "şirket", "company", "kurum", "ünvan", "unvan"]),
        ("email", &["e-posta", "eposta", "e-mail", "email", "mail"]),
        ("phone", &["telefon", "phone", "gsm", "cep", "tel"]),
        ("channel", &["kanal", "kaynak", "channel", "source"]),
        ("note", &["not", "note", "açıklama", "aciklama", "yorum"]),
    ];
    let kucuk: Vec<String> = headers.iter().map(|h| h.trim().to_lowercase()).collect();

    FIELDS
        .iter()
        .map(|(key, _)| {
            let adaylar = ipuclari.iter().find(|(k, _)| k == key).map(|(_, v)| *v)?;
            // Önce TAM eşleşme: "ad" ipucu, "adres" başlığını yakalamamalı.
            adaylar
                .iter()
                .find_map(|ip| kucuk.iter().position(|h| h == ip))
                .or_else(|| adaylar.iter().find_map(|ip| kucuk.iter().position(|h| h.contains(ip))))
        })
        .collect()
}

/// Bir satırdan kişi alanlarını çıkarır. `mapping` sırası [`FIELDS`] ile aynı.
pub fn field_of(row: &[String], mapping: &[Option<usize>], field: &str) -> String {
    let i = FIELDS.iter().position(|(k, _)| *k == field);
    match i.and_then(|i| mapping.get(i).copied().flatten()).and_then(|c| row.get(c)) {
        Some(v) => v.trim().to_string(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 🔴 Türkçe Excel'in varsayılanı: `;` ayraç + BOM. İkisi de sessizce her şeyi bozar.
    #[test]
    fn turkce_excel_dosyasi_okunuyor() {
        let metin = "\u{feff}Ad Soyad;Firma;E-posta\nAhmet Yılmaz;Kurumsal BT;a@b.com\n";
        let p = parse(metin).unwrap();
        assert_eq!(p.delimiter, ";");
        assert_eq!(p.headers, vec!["Ad Soyad", "Firma", "E-posta"], "BOM ilk başlığı bozmamalı");
        assert_eq!(p.rows.len(), 1);
        assert_eq!(p.rows[0][0], "Ahmet Yılmaz");
    }

    #[test]
    fn ayrac_baslik_satirindan_seziliyor() {
        // Not alanı virgül taşıyor ama dosya `;` ile ayrılmış — veri satırına bakılsaydı
        // ayraç yanlış seçilirdi.
        let metin = "ad;not\nAhmet;fiyat sordu, sonra arayacak\n";
        let p = parse(metin).unwrap();
        assert_eq!(p.delimiter, ";");
        assert_eq!(p.rows[0][1], "fiyat sordu, sonra arayacak");
        assert_eq!(parse("ad,firma\nA,B\n").unwrap().delimiter, ",");
    }

    #[test]
    fn tirnakli_alan_ayraci_ve_tirnagi_tasiyor() {
        let metin = "ad,not\n\"Yılmaz, Ahmet\",\"14\"\" monitör istedi\"\n";
        let p = parse(metin).unwrap();
        assert_eq!(p.rows[0][0], "Yılmaz, Ahmet");
        assert_eq!(p.rows[0][1], "14\" monitör istedi");
    }

    #[test]
    fn crlf_ve_sondaki_bos_satir_sorun_cikarmiyor() {
        // Excel hem CRLF yazıyor hem dosya sonuna boş satır bırakıyor.
        let p = parse("ad;firma\r\nAhmet;BT\r\n\r\n").unwrap();
        assert_eq!(p.rows.len(), 1, "boş satır kişi sayılmamalı");
        assert_eq!(p.rows[0], vec!["Ahmet", "BT"]);
    }

    #[test]
    fn bos_dosya_okunur_hata_veriyor() {
        assert!(parse("").is_err());
        assert!(parse("   \n").is_err());
    }

    /// ⚠️ "ad" ipucu "adres" başlığını YAKALAMAMALI — tam eşleşme önce deneniyor.
    #[test]
    fn tahmin_tam_eslesmeyi_once_deniyor() {
        let h: Vec<String> = ["Adres", "Ad", "Firma Ünvanı", "GSM"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let m = guess_mapping(&h);
        assert_eq!(m[0], Some(1), "ad → 'Ad', 'Adres' değil");
        assert_eq!(m[1], Some(2), "firma → 'Firma Ünvanı'");
        assert_eq!(m[2], None, "e-posta sütunu yok, uydurulmamalı");
        assert_eq!(m[3], Some(3), "telefon → 'GSM'");
    }

    #[test]
    fn eslesmeyen_alan_bos_donuyor() {
        let satir: Vec<String> = vec!["Ahmet".into(), "BT".into()];
        let m = guess_mapping(&["ad".to_string(), "firma".to_string()]);
        assert_eq!(field_of(&satir, &m, "name"), "Ahmet");
        assert_eq!(field_of(&satir, &m, "email"), "", "olmayan sütun panik değil boş");
    }
}
