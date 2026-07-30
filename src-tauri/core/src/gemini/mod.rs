//! Gemini meta üretimi (Faz 2).
//!
//! - Model **kademeli fallback**: biri kota/limit (429) verirse sıradaki denenir.
//! - JSON structured output (responseSchema) → 5 alan.
//! - Üretim sonrası kural fail ederse **tek retry** (kısalt/uzat yönergesiyle).
//!
//! Faz 3: `generate_details` gerçek implementasyona kavuşacak (yapı korunur, img-src güvenliği).

use crate::seo_data::SeoInsights;
use crate::validation::{grapheme_len, MetaBadge, MetaInput};
use serde::{Deserialize, Serialize};

mod chat;
mod details;
mod meta;
mod successor;
mod tech;

// Dış yüzey bölmeden ÖNCEKİYLE aynı: çağıranlar hâlâ `gemini::X` yazıyor, hangi alt
// modülde durduğunu bilmek zorunda değiller.
pub use chat::{assistant_system_prompt, chat_stream, ChatEvent, ChatMessage, CHAT_CHAIN};
pub use details::{generate_details, generate_details_scratch, has_rewritable_content, optimize_details};
pub use meta::{generate_meta, GeneratedMeta};
pub use successor::suggest_successor;
pub use tech::{assemble_tech_html, structure_tech_specs, TechGroup, TechRow, TechSpecsResult, TECH_GROUPS};

/// Kota dolduğunda soldan sağa denenecek modeller.
/// Kota dolduğunda (veya model emekliye ayrıldığında) soldan sağa denenecek modeller.
///
/// **Sıralama mantığı:** en yeni/yetenekli önce, sonra aynı neslin `lite` sürümü, sonra bir
/// önceki nesil. Her modelin ücretsiz katmanda **ayrı kota havuzu** var; bu yüzden zincirde
/// nesil çeşitliliği olması, bir modelin limiti dolduğunda üretimin durmamasını sağlıyor.
///
/// **Kurallar:**
/// - `-preview` ekli modeller KULLANILMAZ. Habersiz emekliye ayrılıyorlar; 2026-07 sonunda
///   `gemini-1.5-flash` tam olarak böyle kaybolup üretimi tamamen durdurmuştu.
/// - Zincirin sonundaki `gemini-flash-latest` bir **takma ad**: Google onu güncel modele
///   yönlendirir. Bu liste yıllar sonra bayatlasa bile son halka canlı kalır.
/// - Yeni model eklemeden önce gerçekten çalıştığı doğrulanmalı: uygulamanın istek biçimi
///   `system_instruction` + `responseSchema` kullanıyor ve her model ikisini birden
///   desteklemiyor.
///
/// **Günlük limitler çok farklı** (2026-07-28, ücretsiz katman — konsoldan doğrulandı):
/// normal `flash` modelleri günde yalnızca **20** istek, `flash-lite` sürümleri **500**,
/// Gemma ise **14.400**. Sıralama bu yüzden "kalite azalan, havuz büyüyen": kıt ama iyi
/// olanlar önce harcanır, arkada gittikçe genişleyen emniyet ağı durur.
///
/// Son doğrulama 2026-07-28: hepsi 200 döndü ve `responseSchema`'ya uydu.
pub const MODEL_CHAIN: &[&str] = &[
    // ── kaliteli ama kıt: 5 istek/dk, 20 istek/gün ────────────────────────
    "gemini-3.6-flash",
    "gemini-3.5-flash",
    "gemini-2.5-flash",
    // ── hafif sürümler: 15 istek/dk, 500 istek/gün (25× daha geniş havuz) ──
    "gemini-3.5-flash-lite",
    "gemini-3.1-flash-lite",
    "gemini-2.5-flash-lite",
    // ── takma ad: liste bayatlarsa da canlı kalsın diye ───────────────────
    "gemini-flash-latest",
    // ── son çare: 30 istek/dk, 14.400 istek/gün. Farklı model ailesi olduğu
    //    için üslup Gemini'lerden sapabilir; buraya ancak yukarıdakilerin
    //    tamamı tükendiğinde düşülür — üretimin hiç durmaması için.
    "gemma-4-31b-it",
];

/// HTTP hatasını sınıflandırır: **zincirdeki sıradaki modele geçilmeli mi**, ve kullanıcıya
/// ne yazılmalı.
///
/// `true` → "bu model şu an olmaz, başkası olabilir". `false` → modelden bağımsız kesin hata,
/// zinciri denemenin anlamı yok.
///
/// ⚠️ **404 bilinçli olarak `true`.** Uzun süre kota dışındaki her şey gibi `false` sayılıyordu;
/// `gemini-1.5-flash` emekliye ayrıldığında zincir o modele gelince kırıldı ve üretim tamamen
/// durdu — oysa "bu model artık yok" tam da sıradakini denemek için sebep.
fn classify_error(code: u16, text: &str, model: &str) -> (bool, String) {
    // Anahtar/yetki hataları her modelde aynı sonucu verir → zinciri gezmek zaman kaybı.
    if code == 400 && text.contains("API_KEY_INVALID") {
        return (false, "Gemini API anahtarı geçersiz.".to_string());
    }
    if code == 403 {
        return (
            false,
            format!("Gemini erişimi reddedildi (HTTP 403): {}", short(text)),
        );
    }
    match code {
        429 | 503 => (true, format!("{model} kotası/limiti doldu (HTTP {code}).")),
        500 => (true, format!("{model} geçici sunucu hatası (HTTP 500).")),
        404 => (
            true,
            format!("{model} artık mevcut değil (HTTP 404) — emekliye ayrılmış olabilir."),
        ),
        // Sınıflandırılmamış hatalar (ör. 400): zinciri denemeye devam et. Bir modelin
        // isteğimizi reddetmesi (system_instruction/responseSchema desteklememesi) 400
        // veriyor ve bu tam olarak modele özgü bir durum — burada durmak, düzelttiğimiz
        // hatanın aynısını üretirdi. Bedeli: gerçekten bozuk bir istekte zincir boşuna
        // gezilir; bunu telafi etmek için çağıran, son hatayı mesaja ekliyor.
        _ => (
            true,
            format!("Gemini hatası (HTTP {code}): {}", short(text)),
        ),
    }
}

const API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

pub struct ProductContext<'a> {
    pub name: &'a str,
    pub brand: Option<&'a str>,
    pub category: Option<&'a str>,
    pub main_category: Option<&'a str>,
    /// Faz 4: kullanıcının araştırma panelinde onayladığı hedef kelime (meta üretiminde
    /// "türet" adımını devre dışı bırakır). None → mevcut davranış (model türetir).
    pub target_keyword: Option<&'a str>,
    /// Faz 4: gerçek SEO verisi (Ahrefs/GSC/Trends). None → prompt'a hiçbir şey eklenmez.
    pub insights: Option<&'a SeoInsights>,
}

/// Üretim sonucu + **hangi modelin ürettiği**.
///
/// Zincir kotaya takıldıkça alttaki modellere düşüyor; kullanıcının hangi modelde olduğunu
/// görmesi, "devam mı edeyim yoksa limitler yenilensin mi bekleyeyim" kararını verebilmesi
/// için gerekli — günlük limitler modeller arasında 25× fark ediyor.
pub struct Produced<T> {
    pub value: T,
    pub model: &'static str,
}

fn short(s: &str) -> String {
    let t = s.trim();
    if t.len() > 240 {
        format!("{}…", &t[..240])
    } else {
        t.to_string()
    }
}

/// HTML özel karakterlerini attribute/metin için kaçırır (alt metni güvenliği).
fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Ayarlardaki "Bağlantıyı test et" — hafif bir modelle gerçek bağlantı denemesi.
pub async fn test_key(api_key: &str) -> Result<String, String> {
    let key = api_key.trim();
    if key.len() < 20 {
        return Err("Anahtar çok kısa görünüyor.".to_string());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;
    // models listesini çekerek anahtarı doğrula (üretim tüketmez)
    let url = format!("{API_BASE}?key={key}");
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Gemini'ye ulaşılamadı: {e}"))?;
    if resp.status().is_success() {
        Ok("Anahtar geçerli · Gemini bağlantısı doğrulandı.".to_string())
    } else if resp.status().as_u16() == 400 || resp.status().as_u16() == 403 {
        Err("Anahtar reddedildi · geçersiz veya yetkisiz.".to_string())
    } else {
        Err(format!("Beklenmeyen yanıt: HTTP {}", resp.status().as_u16()))
    }
}

#[cfg(test)]

#[cfg(test)]
mod tests {
    //! Zincir ve anahtar doğrulama testleri — modüller arası ortak davranış.
    use super::*;

    /// Regresyon: 2026-07-28'de `gemini-1.5-flash` emekliye ayrılınca üretim tamamen durdu.
    /// Sebep, 404'ün "kota değil" sayılıp zinciri kırmasıydı — oysa modelin yokluğu tam da
    /// sıradakini denemek için sebep. Bu test o kararı sabitler.
    #[test]
    fn retired_model_advances_the_chain() {
        let (try_next, msg) = classify_error(404, "{\"error\":{\"code\":404}}", "gemini-1.5-flash");
        assert!(try_next, "404 sıradaki modele geçmeli, zinciri kırmamalı");
        assert!(msg.contains("gemini-1.5-flash"), "mesaj modeli adlandırmalı: {msg}");

        // Kota ve geçici sunucu hataları da zinciri sürdürür
        for code in [429, 503, 500] {
            assert!(classify_error(code, "", "m").0, "HTTP {code} devam etmeli");
        }

        // Anahtar/yetki hataları modelden bağımsız → zinciri gezmek anlamsız
        let (try_next, msg) = classify_error(400, "{\"error\":\"API_KEY_INVALID\"}", "m");
        assert!(!try_next, "geçersiz anahtarda zincir denenmemeli");
        assert!(msg.contains("geçersiz"));
        assert!(!classify_error(403, "yasak", "m").0, "403'te zincir denenmemeli");
    }

    /// Zincirde `-preview` model bulunmamalı: habersiz emekliye ayrılıp üretimi durduruyorlar.
    /// Son halka bir takma ad olmalı ki liste bayatlasa bile canlı kalsın.
    #[test]
    fn model_chain_is_production_safe() {
        assert!(!MODEL_CHAIN.is_empty());
        for m in MODEL_CHAIN {
            assert!(!m.contains("preview"), "zincirde preview model var: {m}");
        }
        assert!(
            MODEL_CHAIN.iter().any(|m| m.contains("latest")),
            "zincirde bir 'latest' takma adı olmalı — liste bayatlasa da canlı kalsın"
        );
        // Havuzu en geniş model en sonda dursun: yukarıdakilerin tamamı tükenirse
        // üretim yine de durmasın.
        assert_eq!(
            *MODEL_CHAIN.last().unwrap(),
            "gemma-4-31b-it",
            "son çare, günlük limiti en yüksek model olmalı"
        );
        // Emekli modeller geri sızmasın
        for dead in ["gemini-1.5-flash", "gemini-1.5-pro", "gemini-1.0-pro"] {
            assert!(!MODEL_CHAIN.contains(&dead), "emekli model zincirde: {dead}");
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_key_real() {
        let key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY ayarlı değil");
        let msg = test_key(&key).await.expect("anahtar testi başarısız");
        println!("{msg}");
    }
}
