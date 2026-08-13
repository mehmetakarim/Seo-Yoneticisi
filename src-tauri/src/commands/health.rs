//! Açılış kontrolü — neyin taze, neyin bayat, neyin bozuk olduğu tek yerde.
//!
//! **Neden var.** Aynı bakım işleri dört ayrı ekrana dağılmıştı: feed senkronu Ürünler'de,
//! GSC yenileme Genel Bakış'ta, model listesi ve IdeaSoft token testi Ayarlar'da. Kullanıcı
//! her açılışta dört ekranı gezmek zorundaydı.
//!
//! 🔴 **Kontrol var, çalıştırma YOK — ve bu bilinçli bir sınır.** Ucuz kontroller (token,
//! model listesi) gerçekten koşuyor; pahalı olanların yalnızca **tazeliği okunuyor**.
//! Gerekçe iki katmanlı:
//!   1. GSC analizi bir **ölçüm olayı**: `query_rows`'u ve fırsat raporunu yeniden yazıyor,
//!      anlık görüntü ekleyebiliyor. Her açılışta koşarsa ölçtüğümüz şeyi ölçüm aracıyla
//!      değiştirmiş oluruz. Günde beş açılış beş analiz demek.
//!   2. Ölçülen maliyetler farklı: token ~1 sn / 1 istek, GSC analizi 3,8 sn çekim +
//!      30 bin satır yazma. İkisini aynı otomatiğe bağlamak, ucuz olanı pahalıya bağlamak.
//!
//! Kullanıcı kararı (2026-08-13): *"kontrol et, çalıştırma"*.

use super::*;

/// Bir kontrol satırı. `state`: 'ok' | 'stale' | 'error' | 'off'.
///
/// ⚠️ `off` ayrı bir durum: modül kapalıysa (IdeaSoft bağlanmamış, GSC kurulmamış) bu bir
/// **hata değil**. Kırmızı göstermek, kullanmadığı bir şey için kullanıcıyı endişelendirir.
#[derive(Serialize)]
pub struct HealthCheck {
    pub key: String,
    pub label: String,
    pub state: String,
    pub detail: String,
    /// Bu satırı düzeltecek eylemin anahtarı — ekran düğmeyi buna göre çiziyor.
    /// Boşsa yapılacak bir şey yok.
    pub action: String,
}

/// Bir tarihten bu yana geçen gün. Biçim bozuksa `None`.
///
/// ⚠️ `now_str()` offsetsiz yerel saat yazıyor; `to_rfc3339()` beklemek gün kaymasına yol
/// açıyor (daha önce bir testi düşürdü). İlk 10 karakter yeterli ve biçimden bağımsız.
fn gun_gecti(at: &str) -> Option<i64> {
    let g = chrono::NaiveDate::parse_from_str(at.get(..10)?, "%Y-%m-%d").ok()?;
    Some((chrono::Local::now().date_naive() - g).num_days())
}

/// Tazelik eşikleri — **gerekçeli varsayım**, ölçüm değil.
///
/// ⚠️ Bunlar "ne kadar sonra bayat sayılır" sorusunun cevabı ve ölçülmüş bir dağılıma
/// dayanmıyorlar. Feed günde birkaç kez değişebiliyor (tedarikçi güncellemesi), GSC verisi
/// 3 gün gecikmeli geldiği için günlük analiz anlamsız. Kullanım verisi birikince
/// gözden geçirilmeli.
const FEED_STALE_DAYS: i64 = 3;
const ANALYSIS_STALE_DAYS: i64 = 7;
const INVENTORY_STALE_DAYS: i64 = 14;

fn tazelik(
    key: &str,
    label: &str,
    at: Option<String>,
    esik: i64,
    action: &str,
    hic_yok: &str,
) -> HealthCheck {
    let (state, detail) = match at.as_deref().and_then(gun_gecti) {
        None => ("stale".to_string(), hic_yok.to_string()),
        Some(g) if g > esik => (
            "stale".to_string(),
            format!("{g} gün önce — {esik} günden eski"),
        ),
        Some(0) => ("ok".to_string(), "bugün".to_string()),
        Some(g) => ("ok".to_string(), format!("{g} gün önce")),
    };
    HealthCheck {
        key: key.into(),
        label: label.into(),
        state,
        detail,
        action: action.into(),
    }
}

/// Yalnızca **okunan** kontroller — ağ isteği yok, anında döner.
///
/// Ayrı bir komut olmasının sebebi: ekran önce bunları çizip sonra ağ kontrollerini
/// beklerken kullanıcı boş ekrana bakmasın.
#[tauri::command]
pub fn local_health(state: State<'_, AppState>) -> Result<Vec<HealthCheck>, String> {
    let conn = state.conn.lock().unwrap();
    let ayar = |k: &str| db::get_setting(&conn, k).ok().flatten().unwrap_or_default();

    let mut out = Vec::new();

    // Feed senkronu
    let son_sync: Option<String> = conn
        .query_row("SELECT MAX(run_at) FROM sync_log", [], |r| r.get(0))
        .ok()
        .flatten();
    out.push(tazelik(
        "feed",
        "Ürün feed'i",
        son_sync,
        FEED_STALE_DAYS,
        "sync_feed",
        "hiç senkronlanmadı",
    ));

    // GSC analizi — raporun kendi damgasından.
    let analiz = db::get_setting(&conn, "opportunity_json")
        .ok()
        .flatten()
        .and_then(|j| serde_json::from_str::<serde_json::Value>(&j).ok())
        .and_then(|v| v["analyzed_at"].as_str().map(String::from));
    let mut c = tazelik(
        "analysis",
        "GSC analizi",
        analiz,
        ANALYSIS_STALE_DAYS,
        "run_analysis",
        "hiç çalıştırılmadı",
    );
    if ayar("gsc_service_account_json").trim().is_empty() {
        c.state = "off".into();
        c.detail = "GSC bağlanmamış".into();
        c.action = String::new();
    }
    out.push(c);

    // Sayfa envanteri (Faz İ) — IdeaSoft'a bağlı.
    let env: Option<String> = conn
        .query_row("SELECT MAX(fetched_at) FROM store_pages", [], |r| r.get(0))
        .ok()
        .flatten();
    let mut c = tazelik(
        "inventory",
        "Sayfa envanteri",
        env,
        INVENTORY_STALE_DAYS,
        "sync_store_pages",
        "hiç çekilmedi",
    );
    if ayar("ideasoft_token").trim().is_empty() {
        c.state = "off".into();
        c.detail = "IdeaSoft bağlanmamış".into();
        c.action = String::new();
    }
    out.push(c);

    Ok(out)
}

/// Ağ gerektiren **ucuz** kontroller: token geçerli mi, modeller yanıt veriyor mu.
///
/// ⚠️ Burada "modeller yanıt veriyor mu" = liste ucu erişilebilir mi. Zincirdeki her modeli
/// tek tek denemek (`probe_gemini_model`) kotadan düşen gerçek istekler demek; açılışta
/// yapılacak iş değil.
#[tauri::command]
pub async fn remote_health(state: State<'_, AppState>) -> Result<Vec<HealthCheck>, String> {
    let (gemini_key, is_domain, is_token) = {
        let conn = state.conn.lock().unwrap();
        (
            db::get_setting(&conn, "gemini_api_key")?.unwrap_or_default(),
            db::get_setting(&conn, "ideasoft_domain")?.unwrap_or_default(),
            db::get_setting(&conn, "ideasoft_token")?.unwrap_or_default(),
        )
    };
    let mut out = Vec::new();

    out.push(if gemini_key.trim().is_empty() {
        HealthCheck {
            key: "gemini".into(),
            label: "Yapay zekâ bağlantısı".into(),
            state: "off".into(),
            detail: "anahtar girilmemiş".into(),
            action: String::new(),
        }
    } else {
        match gemini::list_models(&gemini_key).await {
            Ok(m) => HealthCheck {
                key: "gemini".into(),
                label: "Yapay zekâ bağlantısı".into(),
                state: "ok".into(),
                detail: format!("{} model erişilebilir", m.len()),
                action: String::new(),
            },
            Err(e) => HealthCheck {
                key: "gemini".into(),
                label: "Yapay zekâ bağlantısı".into(),
                state: "error".into(),
                detail: e,
                action: "open_settings".into(),
            },
        }
    });

    out.push(if is_domain.trim().is_empty() || is_token.trim().is_empty() {
        HealthCheck {
            key: "ideasoft".into(),
            label: "IdeaSoft bağlantısı".into(),
            state: "off".into(),
            detail: "bağlanmamış (opsiyonel)".into(),
            action: String::new(),
        }
    } else {
        match ideasoft::test_connection(&is_domain, &is_token).await {
            Ok(msg) => HealthCheck {
                key: "ideasoft".into(),
                label: "IdeaSoft bağlantısı".into(),
                state: "ok".into(),
                detail: msg,
                action: String::new(),
            },
            // 🔴 Token GÜNLÜK yenileniyor (Ayarlar'daki not). Açılışta yakalanacak en olası
            // hata bu ve şimdiye kadar ancak bir gönderim başarısız olunca fark ediliyordu.
            Err(e) => HealthCheck {
                key: "ideasoft".into(),
                label: "IdeaSoft bağlantısı".into(),
                state: "error".into(),
                detail: e,
                action: "open_settings".into(),
            },
        }
    });

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gun_once(n: i64) -> String {
        (chrono::Local::now() - chrono::Duration::days(n))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string()
    }

    #[test]
    fn tazelik_esige_gore_karar_veriyor() {
        let t = |g: i64| tazelik("k", "L", Some(gun_once(g)), 3, "a", "hiç").state;
        assert_eq!(t(0), "ok");
        assert_eq!(t(3), "ok", "eşik dahil");
        assert_eq!(t(4), "stale");
    }

    /// Hiç çalışmamış olan **bayat** sayılıyor, hata değil: kullanıcı henüz yapmamış olabilir.
    #[test]
    fn hic_calismamis_bayat_sayiliyor() {
        let c = tazelik("k", "L", None, 3, "a", "hiç senkronlanmadı");
        assert_eq!(c.state, "stale");
        assert_eq!(c.detail, "hiç senkronlanmadı");
        assert!(!c.action.is_empty(), "yapılacak eylem sunulmalı");
    }

    /// ⚠️ Bozuk tarih biçimi çökertmemeli; bayat sayılıp eylem sunulmalı.
    #[test]
    fn bozuk_tarih_cokertmiyor() {
        assert_eq!(tazelik("k", "L", Some("bozuk".into()), 3, "a", "hiç").state, "stale");
        assert_eq!(tazelik("k", "L", Some(String::new()), 3, "a", "hiç").state, "stale");
        assert_eq!(gun_gecti("2026-13-45"), None);
        // ⚠️ Offsetsiz yerel saat (now_str biçimi) okunabilmeli — daha önce bir testi
        // düşüren gün kayması bu biçimden geliyordu.
        assert_eq!(gun_gecti(&gun_once(0)), Some(0));
    }

    /// "Bugün" ile "1 gün önce" farklı yazılmalı — ekranda "0 gün önce" garip durur.
    #[test]
    fn bugun_ayri_yaziliyor() {
        assert_eq!(tazelik("k", "L", Some(gun_once(0)), 3, "a", "h").detail, "bugün");
        assert_eq!(tazelik("k", "L", Some(gun_once(2)), 3, "a", "h").detail, "2 gün önce");
    }
}
