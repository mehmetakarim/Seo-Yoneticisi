//! # Ürün sağlık skoru — "bu ürünün SEO'su ne durumda?" tek sayıda
//!
//! ## ⚠️ Bu, `validation::overall_status`ın yerine geçmiyor
//!
//! `overall_status` bir **durum kovası** döndürüyor (Eksik · Hatalı · Bekliyor · Uygun ·
//! Tamamlandı) ve uygulamanın her yerinde filtre, sayaç ve kuyruk ona bağlı. Değiştirmek beş
//! ekranı birden etkilerdi. Skor onun **yanına** geliyor, yerine değil.
//!
//! Skorun iki katkısı var:
//!
//! 1. **Dereceli.** "Eksik" bir üründe neyin eksik olduğu kovadan anlaşılmıyor; 45/100 ile
//!    80/100 arasındaki fark iş planı için anlamlı.
//! 2. 🔑 **Mağazaya ulaştı mı — `overall` buna HİÇ bakmıyor.** Oysa Faz Ö'nün merkezi kuralı
//!    bu: yerel "tamamlandı" işareti Google'ın gördüğünü değiştirmiyor.
//!
//! ## 🔬 Ölçüm (2026-08-08, gerçek mağaza — 279 ürün)
//!
//! | Durum | Adet |
//! |---|---|
//! | meta + açıklama + teknik üçü de tam | 54 |
//! | **üçü tam AMA mağazaya hiç gönderilmemiş** | **1** |
//! | mağazaya gönderilmiş ama içerik eksik | 19 |
//! | görsel kontrolünde sorun bulunan | 12 |
//!
//! O 1 ürün `overall`a göre "Tamamlandı" görünüyor ama Google için hiç yapılmamış. Skorun
//! varlık sebebi tam olarak bu satır.

use serde::Serialize;

/// Bileşen ağırlıkları — toplamı 100. **Tek yerde**, ekran da buradan okuyor.
///
/// Ağırlık sırası "ne kadar çok arama görünürlüğü taşıyor" sorusuna göre: meta ve açıklama
/// doğrudan sonuç sayfasında görünüyor, teknik tablo içerik derinliği, görsel bir ön koşul.
/// Gönderim düşük ağırlıkta değil — 15 puan, tek başına "tamamlandı"yı 85'te tutmaya yetiyor
/// ki ekranda fark edilsin.
pub const W_META: u32 = 25;
pub const W_DETAILS: u32 = 25;
pub const W_TECH: u32 = 20;
pub const W_IMAGES: u32 = 15;
pub const W_PUSHED: u32 = 15;

/// Üretim için gereken en az galeri görseli (`validation` ile aynı eşik).
pub const MIN_IMAGES: usize = 3;

/// Skorun girdisi — hepsi zaten veritabanında olan alanlar.
#[derive(Debug, Clone, Copy, Default)]
pub struct HealthInput {
    pub meta_done: bool,
    pub details_done: bool,
    pub tech_done: bool,
    pub image_count: usize,
    /// Görsel kontrolünde **sorunlu** bulunan görsel sayısı (kare değil / çok küçük).
    pub image_problems: usize,
    /// İçerik mağazaya ulaştı mı — Faz Ö'nün merkezi kuralı.
    pub pushed: bool,
}

/// Eksik bir bileşen: puanı ve kullanıcıya dönük sebebi.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Missing {
    pub label: String,
    pub points: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Health {
    /// 0–100.
    pub score: u32,
    /// Eksik bileşenler — baloncukta "neden 60?" sorusunu cevaplıyor.
    pub missing: Vec<Missing>,
}

/// Ürünün sağlık skoru ve eksiklerinin dökümü.
///
/// ⚠️ Görsel bileşeni **kısmi** puan alabiliyor: 3 görseli olan ama birinde boyut sorunu olan
/// ürün, hiç görseli olmayanla aynı sayılmamalı. Diğer bileşenler ikili (yapıldı/yapılmadı).
pub fn evaluate(i: &HealthInput) -> Health {
    let mut score = 0;
    let mut missing = Vec::new();

    let bilesen = |ok: bool, points: u32, label: &str, s: &mut u32, m: &mut Vec<Missing>| {
        if ok {
            *s += points;
        } else {
            m.push(Missing { label: label.into(), points });
        }
    };

    bilesen(i.meta_done, W_META, "meta", &mut score, &mut missing);
    bilesen(i.details_done, W_DETAILS, "açıklama", &mut score, &mut missing);
    bilesen(i.tech_done, W_TECH, "teknik tablo", &mut score, &mut missing);

    // Görsel: yeterli sayı + sorunsuz. Sorunlu görseller oranınca puan kırılıyor.
    if i.image_count < MIN_IMAGES {
        missing.push(Missing {
            label: format!("görsel ({}/{} adet)", i.image_count, MIN_IMAGES),
            points: W_IMAGES,
        });
    } else if i.image_problems > 0 {
        let sorunsuz = i.image_count.saturating_sub(i.image_problems);
        let kismi = (W_IMAGES as usize * sorunsuz / i.image_count) as u32;
        score += kismi;
        missing.push(Missing {
            label: format!("{} görselde boyut sorunu", i.image_problems),
            points: W_IMAGES - kismi,
        });
    } else {
        score += W_IMAGES;
    }

    // 🔑 Yerel "tamamlandı" Google'ın gördüğünü değiştirmiyor (Faz Ö'nün merkezi kuralı).
    bilesen(i.pushed, W_PUSHED, "mağazaya gönderim", &mut score, &mut missing);

    Health { score, missing }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tam() -> HealthInput {
        HealthInput {
            meta_done: true,
            details_done: true,
            tech_done: true,
            image_count: 4,
            image_problems: 0,
            pushed: true,
        }
    }

    #[test]
    fn agirliklarin_toplami_yuz() {
        assert_eq!(W_META + W_DETAILS + W_TECH + W_IMAGES + W_PUSHED, 100);
        assert_eq!(evaluate(&tam()).score, 100);
        assert!(evaluate(&tam()).missing.is_empty());
    }

    /// 🔑 Skorun varlık sebebi: `overall_status` bu ürüne "Tamamlandı" diyor ama içerik
    /// mağazaya hiç ulaşmamış, yani Google için hiç yapılmamış. Ölçümde 1 ürün böyleydi.
    #[test]
    fn ucu_tam_ama_gonderilmemis_urun_yuz_almiyor() {
        let mut i = tam();
        i.pushed = false;
        let h = evaluate(&i);
        assert_eq!(h.score, 85);
        assert_eq!(h.missing.len(), 1);
        assert_eq!(h.missing[0].label, "mağazaya gönderim");
    }

    #[test]
    fn gorsel_sorunu_puani_kismi_kiriyor() {
        // 4 görselin 1'i sorunlu → 15 puanın 11'i alınıyor, kalan 4 eksik olarak listeleniyor.
        let mut i = tam();
        i.image_problems = 1;
        let h = evaluate(&i);
        assert_eq!(h.score, 96);
        assert_eq!(h.missing[0].points, 4);

        // ⚠️ Görseli yetersiz olan ürün, sorunlu görseli olanla AYNI sayılmamalı.
        let mut j = tam();
        j.image_count = 2;
        assert_eq!(evaluate(&j).score, 85);
    }

    #[test]
    fn hicbir_sey_yapilmamis_urun_sifir() {
        let h = evaluate(&HealthInput::default());
        assert_eq!(h.score, 0);
        assert_eq!(h.missing.len(), 5, "beş bileşenin beşi de eksik listelenmeli");
    }

    #[test]
    fn eksikler_okunur_ve_puanlariyla_geliyor() {
        let mut i = tam();
        i.tech_done = false;
        i.meta_done = false;
        let h = evaluate(&i);
        assert_eq!(h.score, 55);
        let etiketler: Vec<&str> = h.missing.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(etiketler, vec!["meta", "teknik tablo"]);
        assert_eq!(h.missing.iter().map(|m| m.points).sum::<u32>(), 45);
    }
}
