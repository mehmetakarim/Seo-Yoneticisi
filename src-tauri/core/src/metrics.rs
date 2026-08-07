//! # Ölçüm omurgası — "yaptığımız iş işe yaradı mı?"
//!
//! Uygulama bugüne kadar **anlık fotoğraf** gösteriyordu: "51 fırsat var". Film yoktu —
//! "3 hafta önce 80'di, 20'sini kapattık, 12'si gerçekten iyileşti" denemiyordu. Sebebi
//! kodda görünüyordu: fırsat raporu tek bir `settings` anahtarına yazılıyor ve **her analiz
//! bir öncekini siliyordu**.
//!
//! Bu modül üç şeyi birbirine bağlar:
//! - **Anlık görüntü** (`Snapshot`) — belirli bir 28 günlük pencerede sayfaların GSC verisi
//! - **Olay** (`WorkEvent`) — ne yaptık, ne zaman
//! - **Sonuç** ([`outcome`]) — olaydan önceki ve sonraki pencereyi kıyaslar
//!
//! ## 🔑 Merkezi kural: ölçülen olay, mağazaya ULAŞAN olaydır
//!
//! Yerel "tamamlandı" işaretlemesi Google'ın gördüğü şeyi değiştirmiyor. İçerik ancak
//! mağazaya gidince etkili oluyor. Bu yüzden yalnızca `reaches_store` olayları puanlanıyor
//! (gönderim, canonical); "tamamlandı" işaretleri zaman çizelgesinde bağlam olarak duruyor.
//!
//! ⚠️ Bunun dürüstçe söylenmesi gereken bir yan etkisi var: içeriği elle kopyalayıp mağazaya
//! yapıştıran kullanıcı için gönderim kaydı oluşmuyor → o ürün ölçülemiyor. Arayüz bunu
//! "ölçülemiyor — mağazaya gönderim kaydı yok" diye söylüyor, sessizce boş bırakmıyor.
//!
//! ## Ölçümler (2026-08-07, gerçek mağaza)
//!
//! - **GSC 17 ay geriye veri veriyor** ve her 28 günlük pencere 1,3–2,2 sn'de geliyor. Yani
//!   geçmiş bugünden başlamak zorunda değil: [`windows`] ile geriye doğru tohumlanıyor.
//! - **Satır eşiği ölçülerek seçildi.** Son 28 günlük pencerede 5.961 satır vardı:
//!
//!   | eşik | satır | kapsanan tıklama |
//!   |---|---|---|
//!   | hepsi | 5.961 | %100 |
//!   | **tıklama > 0 VEYA gösterim ≥ 10** | **2.008 (%34)** | **%100** |
//!   | gösterim ≥ 10 | 1.642 | %88 |
//!
//!   Tek bir tıklama kaybetmeden satırların üçte ikisi eleniyor (12 anlık görüntü 8,7 MB
//!   yerine 2,9 MB). Bkz. [`kept`].

use serde::{Deserialize, Serialize};

/// Anlık görüntü penceresinin uzunluğu (gün).
///
/// 28 bilinçli: iş kuyruğu dilindeki "28 gün önce işlendi" ile aynı birim, ve tam 4 hafta
/// olduğu için hafta içi/hafta sonu dalgalanması pencereler arasında sabit kalıyor.
pub const WINDOW_DAYS: i64 = 28;

/// GSC verisi ~2–3 gün gecikmeli geliyor; en yeni pencere bu kadar geriden bitiyor.
pub const GSC_LAG_DAYS: i64 = 3;

/// Bir olayın etkisinin ölçülebilmesi için gereken en az bekleme (gün).
///
/// SEO etkisi gecikmeli: yeni içerik taranıp yeniden sıralanana kadar haftalar geçebiliyor.
/// Takip penceresi olaydan en az bu kadar sonra BAŞLAMALI, yoksa "işe yaramadı" demek
/// erken bir yargı olur.
pub const MIN_WAIT_DAYS: i64 = 21;

/// Sonucun anlamlı sayılabilmesi için temel penceredeki en az gösterim.
///
/// `opportunity::SD_MIN_IMPRESSIONS` ile aynı taban — iki yerde farklı eşik kullanmak
/// "burada fırsat, şurada veri yetersiz" gibi çelişkili cümleler üretirdi.
pub const MIN_IMPRESSIONS: f64 = 30.0;

/// Bir GSC anlık görüntüsünün üstverisi.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Snapshot {
    pub id: i64,
    /// Ne zaman alındı (yerel damga).
    pub captured_at: String,
    /// GSC penceresi — `YYYY-MM-DD`.
    pub window_start: String,
    pub window_end: String,
    pub rows: i64,
    pub clicks: f64,
    pub impressions: f64,
}

/// Anlık görüntüdeki tek bir sayfa satırı.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PageRow {
    pub url: String,
    pub clicks: f64,
    pub impressions: f64,
    pub position: f64,
}

/// Operasyon olayı — "ne yaptık, ne zaman".
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct WorkEvent {
    pub id: i64,
    /// `YYYY-MM-DDTHH:MM:SS` — yerel damga.
    pub at: String,
    pub sku: Option<String>,
    /// Olayın ilgili olduğu sayfa; sonuç bu adres üzerinden ölçülüyor.
    pub url: Option<String>,
    pub kind: String,
    /// Google'ın göreceği bir değişiklik mi? Yalnızca bunlar puanlanıyor.
    pub reaches_store: bool,
}

/// Bir olayın ölçüm durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeStatus {
    /// Takip penceresi henüz dolmadı.
    Measuring,
    Improved,
    Flat,
    Worse,
    /// Temel pencerede yeterli gösterim yok — sayı gürültüden ayrılamıyor.
    Insufficient,
    /// Olay mağazaya ulaşmıyor ya da karşılaştırılacak pencere yok.
    NotMeasurable,
}

impl OutcomeStatus {
    /// Arayüzdeki tek dil — rozetler her ekranda aynı kelimeyi kullanmalı.
    pub fn label(self) -> &'static str {
        match self {
            Self::Measuring => "Ölçülüyor",
            Self::Improved => "İyileşti",
            Self::Flat => "Değişmedi",
            Self::Worse => "Geriledi",
            Self::Insufficient => "Veri yetersiz",
            Self::NotMeasurable => "Ölçülemiyor",
        }
    }

    /// Mevcut rozet token'ları (`--badge-*`). Yeni renk üretilmiyor.
    pub fn tone(self) -> &'static str {
        match self {
            Self::Improved => "uygun",
            Self::Worse => "eksik",
            Self::Measuring => "bekliyor",
            _ => "tamamlandi",
        }
    }
}

/// Bir olayın ölçüm sonucu.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Outcome {
    pub status: OutcomeStatus,
    /// Kıyaslanan pencereler — arayüz "hangi tarihler" diye sorabilsin diye taşınıyor.
    pub baseline_window: Option<String>,
    pub followup_window: Option<String>,
    pub clicks_before: f64,
    pub clicks_after: f64,
    /// Konum düşmesi İYİDİR (1. sıra 9. sıradan iyi); işaret buna göre okunmalı.
    pub position_before: f64,
    pub position_after: f64,
}

impl Outcome {
    fn not_measurable() -> Self {
        Self {
            status: OutcomeStatus::NotMeasurable,
            baseline_window: None,
            followup_window: None,
            clicks_before: 0.0,
            clicks_after: 0.0,
            position_before: 0.0,
            position_after: 0.0,
        }
    }
    pub fn delta_clicks(&self) -> f64 {
        self.clicks_after - self.clicks_before
    }
}

/// Anlık görüntüye YAZILACAK satır mı?
///
/// ⚠️ Eşik ölçümle seçildi (modül başlığındaki tabloya bakın): tıklaması olan her satır
/// zaten giriyor, gösterimi 10'un altındaki tıklamasız satırlar eleniyor. Kapsanan tıklama
/// **%100** — yani bu filtre hiçbir gerçek kazancı gizlemiyor, yalnızca kuyruk gürültüsünü
/// atıyor.
pub fn kept(clicks: f64, impressions: f64) -> bool {
    clicks > 0.0 || impressions >= 10.0
}

/// Tohumlama pencereleri: en yeniden eskiye doğru `count` adet ardışık 28 günlük dilim.
///
/// `today` dışarıdan veriliyor ki test edilebilsin (sistem saatine bağlı test kırılgan olur).
/// Dönen çiftler `(start, end)`, ikisi de dahil biçimde `YYYY-MM-DD`.
pub fn windows(today: chrono::NaiveDate, count: usize) -> Vec<(String, String)> {
    let mut out = Vec::with_capacity(count);
    let mut end = today - chrono::Duration::days(GSC_LAG_DAYS);
    for _ in 0..count {
        let start = end - chrono::Duration::days(WINDOW_DAYS);
        out.push((start.format("%Y-%m-%d").to_string(), end.format("%Y-%m-%d").to_string()));
        // Pencereler ÇAKIŞMAMALI: aynı günü iki pencerede saymak sahte trend üretir.
        end = start - chrono::Duration::days(1);
    }
    out
}

/// Sıradaki anlık görüntü penceresi — pencereler **döşenir, çakışmaz**.
///
/// 🔬 **Ölçüm bu tasarımı dayattı (2026-08-07).** İlk tasarım "son görüntü ≥7 günse yenisini
/// al" diyordu; 28 günlük pencerelerle bu, birbiriyle **%75 çakışan** pencereler üretiyor ve
/// yılda ~18 MB yazıyordu. Döşenmiş pencerelerde yıllık maliyet **~5 MB** ve iki pencereyi
/// kıyaslamak elma-elma oluyor (ortak gün yok).
///
/// `last_end` son yazılan pencerenin bitişi. `None` ise tohumlama yapılmamış demektir.
/// Sıradaki pencere ancak **tamamen geçmişse** döner; yarım pencere ölçüm değil, yanılgıdır.
pub fn next_window(
    last_end: Option<&str>,
    today: chrono::NaiveDate,
) -> Option<(String, String)> {
    let son_gun = today - chrono::Duration::days(GSC_LAG_DAYS);
    let Some(last) = last_end else {
        // Hiç görüntü yok: en yeni pencereyi al (tohumlama ayrıca 12 pencere yazıyor).
        return windows(today, 1).into_iter().next();
    };
    let Some(prev_end) = chrono::NaiveDate::parse_from_str(last, "%Y-%m-%d").ok() else {
        return windows(today, 1).into_iter().next();
    };
    let start = prev_end + chrono::Duration::days(1);
    let end = start + chrono::Duration::days(WINDOW_DAYS);
    // Pencere henüz dolmadıysa bekle — eksik günlerle alınan görüntü sahte düşüş üretir.
    (end <= son_gun)
        .then(|| (start.format("%Y-%m-%d").to_string(), end.format("%Y-%m-%d").to_string()))
}

/// Bir olayın sonucunu hesaplar.
///
/// ⚠️ **Saklanmıyor, istendiğinde hesaplanıyor.** Saklanan bir sonuç yeni anlık görüntü
/// gelince bayatlar ve geçersiz kılma mantığı gerektirir; hesap zaten ucuz.
///
/// `snapshots` **eskiden yeniye sıralı** olmalı. `row_of` bir (snapshot_id, url) çifti için
/// satırı döndürür — veritabanı erişimini çağırana bırakmak bu fonksiyonu saf tutuyor.
pub fn outcome(
    event: &WorkEvent,
    snapshots: &[Snapshot],
    row_of: impl Fn(i64, &str) -> Option<PageRow>,
) -> Outcome {
    if !event.reaches_store {
        return Outcome::not_measurable();
    }
    let (Some(url), Some(at)) = (event.url.as_deref(), event.at.get(..10)) else {
        return Outcome::not_measurable();
    };
    let Ok(event_day) = chrono::NaiveDate::parse_from_str(at, "%Y-%m-%d") else {
        return Outcome::not_measurable();
    };
    let gun = |s: &str| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok();

    // Temel: olaydan ÖNCE biten son pencere. Olay gününü içeren pencere kullanılamaz —
    // içinde hem öncesi hem sonrası var, ikisi birbirini götürür.
    let baseline = snapshots
        .iter()
        .filter(|s| gun(&s.window_end).is_some_and(|e| e < event_day))
        .next_back();

    // Takip: olaydan en az MIN_WAIT_DAYS sonra BAŞLAYAN ilk pencere.
    let followup = snapshots
        .iter()
        .find(|s| {
            gun(&s.window_start)
                .is_some_and(|b| (b - event_day).num_days() >= MIN_WAIT_DAYS)
        });

    let Some(base) = baseline else { return Outcome::not_measurable() };
    let before = row_of(base.id, url).unwrap_or_default();

    let Some(next) = followup else {
        // Ölçüm süreci işliyor ama sonuç için erken.
        return Outcome {
            status: OutcomeStatus::Measuring,
            baseline_window: Some(pencere(base)),
            followup_window: None,
            clicks_before: before.clicks,
            clicks_after: 0.0,
            position_before: before.position,
            position_after: 0.0,
        };
    };
    let after = row_of(next.id, url).unwrap_or_default();

    // ⚠️ Az gösterimli sayfada ±2 tıklama gürültüdür; "iyileşti" demek yanıltıcı olur.
    let status = if before.impressions < MIN_IMPRESSIONS {
        OutcomeStatus::Insufficient
    } else {
        classify(before.clicks, after.clicks)
    };

    Outcome {
        status,
        baseline_window: Some(pencere(base)),
        followup_window: Some(pencere(next)),
        clicks_before: before.clicks,
        clicks_after: after.clicks,
        position_before: before.position,
        position_after: after.position,
    }
}

fn pencere(s: &Snapshot) -> String {
    format!("{} → {}", s.window_start, s.window_end)
}

/// Tıklama değişimini sınıflandırır.
///
/// **İKİ koşul birden** aranıyor: oransal değişim ve mutlak fark. Yalnızca oran bakılsaydı
/// 1 → 2 tıklama "%100 iyileşme" olurdu; yalnızca mutlak fark bakılsaydı 400 → 403 tıklama
/// "iyileşme" sayılırdı. İkisi birlikte gürültüyü eliyor.
///
/// ⚠️ Eşikler gerçek veri üzerinde kalibre edilecek (65 gönderimin delta dağılımı).
fn classify(before: f64, after: f64) -> OutcomeStatus {
    const REL: f64 = 0.20;
    const ABS: f64 = 3.0;
    let fark = after - before;
    let oran = if before > 0.0 { fark / before } else { 1.0 };
    if fark >= ABS && oran >= REL {
        OutcomeStatus::Improved
    } else if fark <= -ABS && oran <= -REL {
        OutcomeStatus::Worse
    } else {
        OutcomeStatus::Flat
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gun(s: &str) -> chrono::NaiveDate {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn snap(id: i64, start: &str, end: &str) -> Snapshot {
        Snapshot {
            id,
            captured_at: format!("{end}T12:00:00"),
            window_start: start.into(),
            window_end: end.into(),
            ..Default::default()
        }
    }

    fn olay(at: &str) -> WorkEvent {
        WorkEvent {
            id: 1,
            at: at.into(),
            sku: Some("A".into()),
            url: Some("https://x/urun/a".into()),
            kind: "ideasoft_push".into(),
            reaches_store: true,
        }
    }

    /// 🔴 Pencereler ÇAKIŞMAMALI: aynı günü iki pencerede saymak sahte trend üretir.
    #[test]
    fn pencereler_ardisik_ve_cakismiyor() {
        let w = windows(gun("2026-08-07"), 3);
        assert_eq!(w[0], ("2026-07-07".into(), "2026-08-04".into())); // 3 gün GSC gecikmesi
        assert_eq!(w[1], ("2026-06-08".into(), "2026-07-06".into()));
        assert_eq!(w[2], ("2026-05-10".into(), "2026-06-07".into()));
        for i in 0..w.len() - 1 {
            assert!(w[i + 1].1 < w[i].0, "pencereler çakışıyor: {:?}", (&w[i], &w[i + 1]));
        }
    }

    /// 🔬 Ölçülen eşik: tıklaması olan her satır girer, gösterimi 10'un altındaki
    /// tıklamasız satırlar elenir. Gerçek veride kapsanan tıklama %100'dü.
    #[test]
    fn satir_esigi_tiklamayi_asla_elemez() {
        assert!(kept(1.0, 0.0), "tıklaması olan satır elendi");
        assert!(kept(0.0, 10.0));
        assert!(!kept(0.0, 9.0));
        assert!(!kept(0.0, 0.0));
    }

    #[test]
    fn takip_penceresi_dolmadiysa_olculuyor() {
        // Olay 2026-08-01; tek pencere Temmuz'da bitiyor → takip yok.
        let snaps = vec![snap(1, "2026-06-30", "2026-07-28")];
        let o = outcome(&olay("2026-08-01T10:00:00"), &snaps, |_, _| {
            Some(PageRow { clicks: 40.0, impressions: 900.0, position: 6.0, ..Default::default() })
        });
        assert_eq!(o.status, OutcomeStatus::Measuring);
        assert!(o.followup_window.is_none());
    }

    /// ⚠️ Takip penceresi olaydan en az 21 gün SONRA başlamalı. Hemen ertesi gün başlayan
    /// pencere "işe yaramadı" dedirtirdi — SEO etkisi gecikmeli.
    #[test]
    fn cok_erken_baslayan_pencere_takip_sayilmaz() {
        let snaps = vec![
            snap(1, "2026-06-01", "2026-06-29"),
            snap(2, "2026-07-02", "2026-07-30"), // olaydan 2 gün sonra → SAYILMAZ
        ];
        let o = outcome(&olay("2026-06-30T09:00:00"), &snaps, |_, _| {
            Some(PageRow { clicks: 10.0, impressions: 500.0, position: 8.0, ..Default::default() })
        });
        assert_eq!(o.status, OutcomeStatus::Measuring);
    }

    #[test]
    fn iyilesme_ve_gerileme_siniflandirmasi() {
        let snaps = vec![snap(1, "2026-05-01", "2026-05-29"), snap(2, "2026-06-25", "2026-07-23")];
        let ev = olay("2026-06-01T09:00:00");

        let o = outcome(&ev, &snaps, |id, _| {
            Some(if id == 1 {
                PageRow { clicks: 20.0, impressions: 900.0, position: 8.0, ..Default::default() }
            } else {
                PageRow { clicks: 35.0, impressions: 1000.0, position: 5.0, ..Default::default() }
            })
        });
        assert_eq!(o.status, OutcomeStatus::Improved);
        assert_eq!(o.delta_clicks(), 15.0);

        let o2 = outcome(&ev, &snaps, |id, _| {
            Some(if id == 1 {
                PageRow { clicks: 40.0, impressions: 900.0, position: 5.0, ..Default::default() }
            } else {
                PageRow { clicks: 12.0, impressions: 700.0, position: 11.0, ..Default::default() }
            })
        });
        assert_eq!(o2.status, OutcomeStatus::Worse);
    }

    /// 🔴 İki koşul birden aranmasının sebebi: yalnızca ORAN bakılsaydı 1→2 tıklama
    /// "%100 iyileşme", yalnızca MUTLAK bakılsaydı 400→403 "iyileşme" sayılırdı.
    #[test]
    fn kucuk_sayilarda_gurultu_iyilesme_sayilmaz() {
        let snaps = vec![snap(1, "2026-05-01", "2026-05-29"), snap(2, "2026-06-25", "2026-07-23")];
        let ev = olay("2026-06-01T09:00:00");
        // 1 → 2 tıklama: oran %100 ama mutlak fark 1 → değişmedi
        let o = outcome(&ev, &snaps, |id, _| {
            Some(if id == 1 {
                PageRow { clicks: 1.0, impressions: 400.0, position: 9.0, ..Default::default() }
            } else {
                PageRow { clicks: 2.0, impressions: 420.0, position: 9.0, ..Default::default() }
            })
        });
        assert_eq!(o.status, OutcomeStatus::Flat);

        // 400 → 403: mutlak fark 3 ama oran %0,75 → değişmedi
        let o2 = outcome(&ev, &snaps, |id, _| {
            Some(if id == 1 {
                PageRow { clicks: 400.0, impressions: 9000.0, position: 3.0, ..Default::default() }
            } else {
                PageRow { clicks: 403.0, impressions: 9100.0, position: 3.0, ..Default::default() }
            })
        });
        assert_eq!(o2.status, OutcomeStatus::Flat);
    }

    /// ⚠️ Az gösterimli sayfada birkaç tıklama gürültüdür; "iyileşti" demek yanıltıcı olur.
    #[test]
    fn az_gosterimde_veri_yetersiz() {
        let snaps = vec![snap(1, "2026-05-01", "2026-05-29"), snap(2, "2026-06-25", "2026-07-23")];
        let o = outcome(&olay("2026-06-01T09:00:00"), &snaps, |id, _| {
            Some(if id == 1 {
                PageRow { clicks: 0.0, impressions: 12.0, position: 20.0, ..Default::default() }
            } else {
                PageRow { clicks: 5.0, impressions: 60.0, position: 14.0, ..Default::default() }
            })
        });
        assert_eq!(o.status, OutcomeStatus::Insufficient);
    }

    /// 🔑 Merkezi kural: yerel "tamamlandı" Google'ın gördüğünü değiştirmiyor → puanlanmaz.
    #[test]
    fn magazaya_ulasmayan_olay_olculmez() {
        let snaps = vec![snap(1, "2026-05-01", "2026-05-29"), snap(2, "2026-06-25", "2026-07-23")];
        let mut ev = olay("2026-06-01T09:00:00");
        ev.kind = "meta_done".into();
        ev.reaches_store = false;
        let o = outcome(&ev, &snaps, |_, _| Some(PageRow::default()));
        assert_eq!(o.status, OutcomeStatus::NotMeasurable);
    }

    /// 🔴 Pencereler DÖŞENİR: sıradaki, bir öncekinin bittiği günün ertesinde başlar.
    /// Çakışan pencereler hem yılda ~18 MB yazıyordu hem de ortak günler yüzünden
    /// karşılaştırmayı bozuyordu (ölçüldü, 2026-08-07).
    #[test]
    fn siradaki_pencere_dosenir_ve_dolmadan_alinmaz() {
        let bugun = gun("2026-09-15");
        // Son pencere 2026-08-04'te bitmiş → sıradaki 08-05 … 09-02, ve dolmuş.
        assert_eq!(
            next_window(Some("2026-08-04"), bugun),
            Some(("2026-08-05".into(), "2026-09-02".into()))
        );
        // Aynı son pencere ama bugün 08-20: sıradaki pencere HENÜZ DOLMADI.
        assert_eq!(next_window(Some("2026-08-04"), gun("2026-08-20")), None);
        // Hiç görüntü yoksa en yeni pencere.
        assert!(next_window(None, bugun).is_some());
    }
}
