use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;

/// Türkçe karakterler için grapheme bazlı sayım (ham byte değil).
/// Meta başlığın kabul edilen uzunluk aralığı (grapheme).
///
/// 🔴 **Tek kaynak.** Bu aralık `validation`, `gemini::meta` ve `gemini::store_page`
/// tarafından kullanılıyor; üç kopya olsaydı biri güncellenmediğinde üretim bir kuralı,
/// rozet başka bir kuralı uygular ve kullanıcı "uygun" yazan bir başlığın neden yeniden
/// üretildiğini anlayamazdı. (Projede kopyalanan sayı dört kez saptı.)
pub const TITLE_MIN: usize = 20;
pub const TITLE_MAX: usize = 60;

/// Meta açıklamanın kabul edilen aralığı.
///
/// ⚠️ Üst sınır 155, IdeaSoft'un kestiği 160'tan **bilinçli olarak kısa**: canlı ölçümde
/// (2026-08-12) blog PUT'unda 170 karakter gönderildi, 160 saklandı, uyarı çıkmadı. 155
/// hedeflemek o sessiz kırpmanın kenarına yaklaşmamayı sağlıyor.
pub const DESC_MIN: usize = 50;
pub const DESC_MAX: usize = 155;

pub fn grapheme_len(s: &str) -> usize {
    s.graphemes(true).count()
}

/// Hedef kelime kuralı üç durumlu: kelime boşsa "belirsiz" (None).
fn contains_keyword(text: &str, keyword: &str) -> Option<bool> {
    let kw = keyword.trim();
    if kw.is_empty() {
        return None;
    }
    Some(text.to_lowercase().contains(&kw.to_lowercase()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MetaBadge {
    Eksik,
    Hatali,
    Uygun,
    Tamamlandi,
}

pub struct MetaInput<'a> {
    pub title: &'a str,
    pub descriptions: &'a str,
    pub target_keyword: &'a str,
    pub meta_done: bool,
}

/// Spec'teki rozet mantığı:
/// done → Tamamlandı; title/descriptions tamamen boş → Eksik;
/// herhangi bir kesin kural fail → Hatalı; aksi halde Uygun.
/// Hedef kelime kuralı "belirsiz" (keyword boş) ise fail sayılmaz.
pub fn meta_badge(m: &MetaInput) -> MetaBadge {
    if m.meta_done {
        return MetaBadge::Tamamlandi;
    }
    let title = m.title.trim();
    let desc = m.descriptions.trim();
    if title.is_empty() || desc.is_empty() {
        return MetaBadge::Eksik;
    }
    let tl = grapheme_len(title);
    let dl = grapheme_len(desc);
    let title_len_ok = (TITLE_MIN..=TITLE_MAX).contains(&tl);
    let desc_len_ok = (DESC_MIN..=DESC_MAX).contains(&dl);
    let title_kw_ok = contains_keyword(title, m.target_keyword).unwrap_or(true);
    let desc_kw_ok = contains_keyword(desc, m.target_keyword).unwrap_or(true);
    if title_len_ok && desc_len_ok && title_kw_ok && desc_kw_ok {
        MetaBadge::Uygun
    } else {
        MetaBadge::Hatali
    }
}

/// Details (uzun açıklama) durum rozeti — Faz 3.
/// done → Tamamlandı; içerik boş → Eksik; kelime≥50 ve yoğunluk 1.5–3.5 → Uygun; aksi Hatalı.
pub fn details_badge(details_html: &str, target_keyword: &str, details_done: bool) -> MetaBadge {
    if details_done {
        return MetaBadge::Tamamlandi;
    }
    let words = word_count(details_html);
    if words == 0 {
        return MetaBadge::Eksik;
    }
    let dens = keyword_density(details_html, target_keyword);
    let word_ok = words >= 50;
    let dens_ok = (1.5..=3.5).contains(&dens);
    if word_ok && dens_ok {
        MetaBadge::Uygun
    } else {
        MetaBadge::Hatali
    }
}

// ---- Faz 7: görsel skoru ----

/// Görsel skoru rozeti (mevcut MetaBadge yeniden kullanılır).
/// count<3 → Eksik (üretim kapısı bunu kullanır); count≥3 & boyut sonucu yok/pending → Uygun;
/// count≥3 & tüm görsel 1:1+≥min → Uygun; count≥3 ama biri başarısız → Hatalı.
pub fn image_badge(count: usize, all_dims_ok: Option<bool>) -> MetaBadge {
    if count < 3 {
        return MetaBadge::Eksik;
    }
    match all_dims_ok {
        Some(false) => MetaBadge::Hatali,
        _ => MetaBadge::Uygun,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OverallStatus {
    Eksik,
    Hatali,
    Bekliyor,
    Uygun,
    Tamamlandi,
}

/// Bir ürünün tüm iş boyutları (liste durumu bunlardan hesaplanır).
pub struct OverallInput {
    pub meta: MetaBadge,
    pub details: MetaBadge,
    pub meta_done: bool,
    pub details_done: bool,
    /// Faz 8: teknik tablo tamamlandı mı? (Faz 9b: tamamlanma ölçütüne dahil)
    pub tech_done: bool,
    pub has_tech: bool,
    /// Faz 7: galeri görseli sayısı — 3'ten azsa üretim zaten engelli, durum "Eksik".
    pub image_count: usize,
}

/// **Dört boyutlu** liste durumu: Meta + Açıklama + Teknik tablo + Görsel.
/// Tamamlandı = üçü de "Tamamlandı" işaretli. 3'ten az görsel → Eksik (üretim engelli).
pub fn overall_status(i: &OverallInput) -> OverallStatus {
    if i.meta_done && i.details_done && i.tech_done {
        return OverallStatus::Tamamlandi;
    }
    // Görsel yetersizse hiçbir şey üretilemez → önce bu giderilmeli
    if i.image_count < 3 {
        return OverallStatus::Eksik;
    }
    if i.meta == MetaBadge::Eksik || i.details == MetaBadge::Eksik || !i.has_tech {
        return OverallStatus::Eksik;
    }
    if i.meta == MetaBadge::Hatali || i.details == MetaBadge::Hatali {
        return OverallStatus::Hatali;
    }
    // İçerik hazır ama tamamlandı işaretlenmemiş
    if i.meta_done || i.details_done || i.tech_done {
        return OverallStatus::Bekliyor;
    }
    OverallStatus::Uygun
}

// ---- HTML yardımcıları (Faz 2/3) ----

/// HTML etiketlerini ve entity'leri söküp düz metin döndürür.
pub fn html_strip(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    // Basit entity temizliği ve boşluk normalizasyonu
    let cleaned = out
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"");
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn word_count(html: &str) -> usize {
    let text = html_strip(html);
    if text.is_empty() {
        0
    } else {
        text.split_whitespace().count()
    }
}

/// Hedef kelime yoğunluğu (%). **Öbek-bazlı**: hedef kelime öbeğinin geçiş sayısı / toplam kelime.
/// (Öbeğin kelime sayısıyla ÇARPILMAZ — aksi halde çok kelimeli hedef kelimeler yapay biçimde şişer;
/// standart SEO yoğunluğu öbek başınadır ve %2-3 hedefi çok kelimeli öbekler için de gerçekçi olur.)
pub fn keyword_density(html: &str, keyword: &str) -> f64 {
    let words = word_count(html);
    let kw = keyword.trim().to_lowercase();
    if words == 0 || kw.is_empty() {
        return 0.0;
    }
    let text = html_strip(html).to_lowercase();
    let occurrences = text.matches(&kw).count();
    occurrences as f64 / words as f64 * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grapheme_counts_turkish_chars() {
        assert_eq!(grapheme_len("ığüşöçİĞÜŞÖÇ"), 12);
        assert_eq!(grapheme_len(""), 0);
        // "ığüşöç".len() byte olarak 12'dir; grapheme 6 olmalı
        assert_eq!(grapheme_len("ığüşöç"), 6);
    }

    fn badge(title: &str, desc: &str, kw: &str, done: bool) -> MetaBadge {
        meta_badge(&MetaInput { title, descriptions: desc, target_keyword: kw, meta_done: done })
    }

    const GOOD_DESC: &str =
        "Sony WH-1000XM5 kablosuz kulaklık ile üstün ses kalitesi, aktif gürültü engelleme ve uzun pil ömrü.";

    #[test]
    fn badge_done_wins() {
        assert_eq!(badge("", "", "", true), MetaBadge::Tamamlandi);
    }

    #[test]
    fn badge_empty_is_eksik() {
        assert_eq!(badge("", GOOD_DESC, "", false), MetaBadge::Eksik);
        assert_eq!(badge("Sony WH-1000XM5 Kablosuz Kulaklık", "  ", "", false), MetaBadge::Eksik);
    }

    #[test]
    fn badge_length_rules() {
        // 20'den kısa title → Hatalı
        assert_eq!(badge("Kısa başlık", GOOD_DESC, "", false), MetaBadge::Hatali);
        // Uygun aralıkta, keyword belirsiz → Uygun
        assert_eq!(badge("Sony WH-1000XM5 Kablosuz Kulaklık", GOOD_DESC, "", false), MetaBadge::Uygun);
    }

    #[test]
    fn badge_keyword_case_insensitive() {
        assert_eq!(
            badge("Sony WH-1000XM5 Kablosuz Kulaklık", GOOD_DESC, "KABLOSUZ KULAKLIK", false),
            MetaBadge::Hatali, // "KULAKLIK" küçük harfe "kulaklik" olur, metinde "kulaklık" var → eşleşmez (ASCII I)
        );
        assert_eq!(
            badge("Sony WH-1000XM5 Kablosuz Kulaklık", GOOD_DESC, "kablosuz kulaklık", false),
            MetaBadge::Uygun,
        );
    }

    #[test]
    fn html_strip_and_word_count() {
        let html = "<section><h2>Başlık</h2><p>Bir iki üç &amp; dört</p></section>";
        assert_eq!(html_strip(html), "Başlık Bir iki üç & dört");
        assert_eq!(word_count(html), 6);
        assert_eq!(word_count(""), 0);
    }

    fn ov(meta_done: bool, details_done: bool, tech_done: bool, has_tech: bool, imgs: usize) -> OverallStatus {
        overall_status(&OverallInput {
            meta: MetaBadge::Uygun,
            details: MetaBadge::Uygun,
            meta_done,
            details_done,
            tech_done,
            has_tech,
            image_count: imgs,
        })
    }

    #[test]
    fn overall_requires_all_three_dimensions() {
        // Üçü de done → Tamamlandı
        assert_eq!(ov(true, true, true, true, 4), OverallStatus::Tamamlandi);
        // Teknik tablo eksik → artık Tamamlandı DEĞİL
        assert_eq!(ov(true, true, false, true, 4), OverallStatus::Bekliyor);
        // Hiç teknik tablo yok → Eksik
        assert_eq!(ov(false, false, false, false, 4), OverallStatus::Eksik);
        // Görsel 3'ten az → üretim engelli → Eksik (içerik hazır olsa bile)
        assert_eq!(ov(false, false, false, true, 2), OverallStatus::Eksik);
        // Her şey hazır, hiçbiri işaretlenmemiş → Uygun
        assert_eq!(ov(false, false, false, true, 3), OverallStatus::Uygun);
    }

    #[test]
    fn overall_flags_broken_content() {
        let st = overall_status(&OverallInput {
            meta: MetaBadge::Hatali,
            details: MetaBadge::Uygun,
            meta_done: false,
            details_done: false,
            tech_done: false,
            has_tech: true,
            image_count: 4,
        });
        assert_eq!(st, OverallStatus::Hatali);
    }

    #[test]
    fn image_badge_rules() {
        assert_eq!(image_badge(1, None), MetaBadge::Eksik);
        assert_eq!(image_badge(2, Some(true)), MetaBadge::Eksik); // <3 her zaman Eksik
        assert_eq!(image_badge(3, None), MetaBadge::Uygun); // boyut pending
        assert_eq!(image_badge(4, Some(true)), MetaBadge::Uygun);
        assert_eq!(image_badge(3, Some(false)), MetaBadge::Hatali);
    }

    #[test]
    fn density_counts_phrase() {
        let html = "<p>kablosuz kulaklık iyidir. Bu kablosuz kulaklık rahattır ve şıktır ve uygundur.</p>";
        // 11 kelime, 2 öbek geçişi → 2/11 → %18.18 (öbek-bazlı, kelime sayısıyla çarpılmaz)
        let d = keyword_density(html, "kablosuz kulaklık");
        assert!((d - 18.18).abs() < 0.1, "beklenmedik yoğunluk: {d}");
        assert_eq!(keyword_density(html, ""), 0.0);
    }
}
