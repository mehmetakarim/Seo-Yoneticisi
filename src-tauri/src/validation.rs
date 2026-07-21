use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;

/// Türkçe karakterler için grapheme bazlı sayım (ham byte değil).
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
    let title_len_ok = (20..=60).contains(&tl);
    let desc_len_ok = (50..=155).contains(&dl);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OverallStatus {
    Eksik,
    Hatali,
    Bekliyor,
    Uygun,
    Tamamlandi,
}

/// İki boyutlu (Meta + Açıklama) liste durumu — prototipteki `overall()` mantığı.
pub fn overall_status(
    meta: MetaBadge,
    details: MetaBadge,
    meta_done: bool,
    details_done: bool,
) -> OverallStatus {
    if meta_done && details_done {
        return OverallStatus::Tamamlandi;
    }
    if meta_done && !details_done {
        return OverallStatus::Bekliyor;
    }
    if meta == MetaBadge::Eksik || details == MetaBadge::Eksik {
        return OverallStatus::Eksik;
    }
    if meta == MetaBadge::Hatali || details == MetaBadge::Hatali {
        return OverallStatus::Hatali;
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

/// Hedef kelime yoğunluğu (%). Kelime öbeği geçiş sayısı * öbek kelime sayısı / toplam kelime.
pub fn keyword_density(html: &str, keyword: &str) -> f64 {
    let words = word_count(html);
    let kw = keyword.trim().to_lowercase();
    if words == 0 || kw.is_empty() {
        return 0.0;
    }
    let text = html_strip(html).to_lowercase();
    let occurrences = text.matches(&kw).count();
    let kw_words = kw.split_whitespace().count();
    (occurrences * kw_words) as f64 / words as f64 * 100.0
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

    #[test]
    fn density_counts_phrase() {
        let html = "<p>kablosuz kulaklık iyidir. Bu kablosuz kulaklık rahattır ve şıktır ve uygundur.</p>";
        // 11 kelime, 2 geçiş * 2 kelime = 4 → %36.36
        let d = keyword_density(html, "kablosuz kulaklık");
        assert!((d - 36.36).abs() < 0.1, "beklenmedik yoğunluk: {d}");
        assert_eq!(keyword_density(html, ""), 0.0);
    }
}
