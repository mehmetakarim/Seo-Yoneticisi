//! # Müşteri takibi — kuyruğun tek insan işi (Faz C)
//!
//! Uygulama bugüne kadar **ziyaretçiyi** görüyordu, **müşteriyi** görmüyordu. Bir ürün 955
//! gösterim alıp 36 tıklama getiriyor, o tıklamalardan biri mail atıyor — ve bu noktadan
//! sonra uygulama kör: kimin ne sorduğu, ne zaman dönüleceği hiçbir yerde yok.
//!
//! Bu modül **karar vermiyor, hatırlatıyor**: hangi kişiye bugün dönülmeli, ne kadar gecikti.
//!
//! ## 🔴 Eşik uydurulmuyor
//!
//! "X gündür temas edilmemiş müşteri" uyarısı için uygulamada kalibre edilecek **hiç veri
//! yok** — tek bir temas kaydı bile. Faz D'de stok eşiği tam bu sebeple kapsam dışı kalmıştı
//! (kod yazılır, ekranda ya boş durur ya gürültü üretir).
//!
//! Buradaki çözüm bir adım öteye gidiyor: eşik **kapalı başlıyor** ve uygulama zamanla
//! kullanıcının kendi verisinden öğreniyor — bkz. [`sessizlik_onerisi`]. Faz S'in süre
//! kalibrasyonuyla aynı desen, aynı gövde ([`crate::stats::median_sample`]).

use serde::{Deserialize, Serialize};

/// Nereden geldi — tek değerli olduğu için kişide sütun, etiket değil.
pub const CHANNELS: &[&str] = &["mail", "telefon", "instagram", "fuar", "referans", "diğer"];

/// Sessizlik önerisi için gereken en az **kişi** sayısı (her biri ≥2 temaslı).
///
/// 🔬 [`crate::queue::MIN_SAMPLES`] ile aynı gerekçe ve aynı sayı: tek bir kişinin temas
/// ritmi mağazanın ritmi değil. Ayrı bir sabit çünkü ölçtüğü şey ayrı — biri dakika, öbürü
/// gün; birinin eşiği değişirse öbürünün değişmesi gerekmez.
pub const MIN_CONTACTS: usize = 5;

/// Kuyruğa iş düşen bir kişi.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DueContact {
    pub id: i64,
    pub name: String,
    /// Firma — kuyruk başlığında ada ekleniyor ("Ahmet Yılmaz · Kurumsal BT").
    pub company: String,
    /// Sonraki adım notu; boşsa kuyruk genel bir cümle kuruyor.
    pub note: String,
    /// Kaç gün gecikti. 0 = bugün, negatif = gelecek (kuyruğa girmez).
    pub overdue_days: i64,
    /// Sessizlik dalıysa: son temastan bu yana geçen gün. 0 = bu bir sonraki-adım maddesi.
    ///
    /// ⚠️ İki dal ayrı cümle kuruyor. Ayrılmasaydı sessiz kişi *"bugün dönülecek"* derdi —
    /// çünkü sonraki adım tarihi yok, gecikme de 0 çıkıyor. Kullanıcıya söylenen sebep
    /// yanlış olurdu ve kuyruğun "her gerekçe gerçek bir ölçümden" kuralı çiğnenirdi.
    #[serde(default)]
    pub silent_days: i64,
}

impl DueContact {
    /// Kuyruk başlığı: ad + varsa firma.
    pub fn title(&self) -> String {
        if self.company.trim().is_empty() {
            self.name.clone()
        } else {
            format!("{} · {}", self.name, self.company.trim())
        }
    }

    /// Kuyruk gerekçesi — **her zaman bir tarih gerçeğine dayanıyor**, tıpkı SEO
    /// maddelerindeki gibi ("515 tıklama kaçıyor"). Uydurma bir "ilgilenilmeli" cümlesi yok.
    pub fn reason(&self) -> String {
        if self.silent_days > 0 {
            // Sessizlik: söz verilmiş bir tarih yok, ilişki soğumuş.
            return format!("{} gündür temas yok", self.silent_days);
        }
        let ne_zaman = match self.overdue_days {
            d if d <= 0 => "bugün dönülecek".to_string(),
            1 => "dünden beri bekliyor".to_string(),
            d => format!("{d} gündür bekliyor"),
        };
        let not = self.note.trim();
        if not.is_empty() {
            ne_zaman
        } else {
            format!("{ne_zaman} — {not}")
        }
    }
}

/// Sonraki adım tarihi bugüne geldi mi?
///
/// ⚠️ Gelecek tarihli adım kuyruğa **girmiyor**: "3 hafta sonra ara" notu bugünün listesini
/// kirletmemeli. Bugün ve geçmiş girer.
pub fn is_due(overdue_days: i64) -> bool {
    overdue_days >= 0
}

/// Gecikmeye göre sessizlik önerisi: temas aralıklarının medyanı, yukarı 5'in katına.
///
/// Kullanıcıya cümle olarak dönüyor ("ortalama 24 günde bir dönüyorsunuz — 30 gün diyelim
/// mi?"), sessizce ayara YAZILMIYOR. Eşiği açmak kullanıcının kararı.
///
/// `None`: henüz yeterli veri yok, öneri yapılmıyor. Bu bir hata hâli değil — Faz S'te
/// öğrenildiği gibi "veri yok" geçerli bir sonuç ve öyle karşılanmalı.
pub fn sessizlik_onerisi(gun_araliklari: &[f64]) -> Option<u32> {
    let medyan = crate::stats::median_sample(gun_araliklari, MIN_CONTACTS)?;
    // 5'in katına yuvarlanıyor: "27 gün" diye bir öneri sahte bir kesinlik taşır, veri o
    // kadar hassas değil. En az 7 — günlük temas bir "sessizlik eşiği" değildir.
    let yuvarlak = ((medyan / 5.0).ceil() * 5.0).max(7.0);
    Some(yuvarlak as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kisi(overdue: i64, note: &str) -> DueContact {
        DueContact {
            id: 1,
            name: "Ahmet Yılmaz".into(),
            company: "Kurumsal BT".into(),
            note: note.into(),
            overdue_days: overdue,
            silent_days: 0,
        }
    }

    #[test]
    fn baslik_firmayi_ekliyor_bos_firmada_eklemiyor() {
        assert_eq!(kisi(0, "").title(), "Ahmet Yılmaz · Kurumsal BT");
        let mut k = kisi(0, "");
        k.company = "   ".into();
        assert_eq!(k.title(), "Ahmet Yılmaz");
    }

    /// Gerekçe cümlesi yol haritasının şartı: **gerçek bir veriden** türemeli.
    #[test]
    fn gerekce_her_zaman_tarih_gercegi_tasiyor() {
        assert_eq!(kisi(0, "").reason(), "bugün dönülecek");
        assert_eq!(kisi(1, "").reason(), "dünden beri bekliyor");
        assert_eq!(kisi(12, "").reason(), "12 gündür bekliyor");
        assert_eq!(kisi(3, "fiyat verilecek").reason(), "3 gündür bekliyor — fiyat verilecek");
    }

    /// ⚠️ "3 hafta sonra ara" bugünün listesini kirletmemeli.
    #[test]
    fn gelecek_tarihli_adim_kuyruga_girmiyor() {
        assert!(is_due(0), "bugünü kapsıyor");
        assert!(is_due(9));
        assert!(!is_due(-1), "yarın için verilen söz bugün iş değil");
    }

    /// 🔴 İki dal ayrılmasaydı sessiz kişi "bugün dönülecek" derdi: sonraki adım tarihi yok,
    /// gecikme 0 çıkıyor. Kuyruğun her gerekçesi gerçek bir ölçümü söylemeli.
    #[test]
    fn sessiz_kisi_kendi_cumlesini_kuruyor() {
        let mut k = kisi(0, "");
        k.silent_days = 41;
        assert_eq!(k.reason(), "41 gündür temas yok");
        // Sonraki adım maddesi etkilenmiyor.
        assert_eq!(kisi(0, "").reason(), "bugün dönülecek");
    }

    #[test]
    fn sessizlik_onerisi_yetersiz_veride_sayi_uydurmuyor() {
        assert_eq!(sessizlik_onerisi(&[10.0, 20.0, 30.0, 40.0]), None, "4 kişi < MIN_CONTACTS");
        assert_eq!(sessizlik_onerisi(&[]), None);
    }

    #[test]
    fn sessizlik_onerisi_medyani_bese_yuvarliyor() {
        // Medyan 24 → 25. Uç değer (bir yıl unutulan kişi) öneriyi kaçırmamalı.
        assert_eq!(sessizlik_onerisi(&[8.0, 20.0, 24.0, 30.0, 365.0]), Some(25));
        // ⚠️ Taban 7: her gün konuşulan bir liste "1 gün sessizlik" önerisi üretmemeli.
        assert_eq!(sessizlik_onerisi(&[1.0, 1.0, 2.0, 2.0, 3.0]), Some(7));
    }
}
