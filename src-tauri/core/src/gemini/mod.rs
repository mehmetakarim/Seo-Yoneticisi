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
mod store_page;
mod successor;
mod tech;

// Dış yüzey bölmeden ÖNCEKİYLE aynı: çağıranlar hâlâ `gemini::X` yazıyor, hangi alt
// modülde durduğunu bilmek zorunda değiller.
pub use chat::{assistant_system_prompt, chat_stream, session_title, ChatEvent, ChatMessage, CHAT_CHAIN};
pub use details::{
    generate_details, generate_details_scratch, generate_product_details, generate_team_product_details,
    has_rewritable_content, optimize_details,
};
pub use meta::{generate_meta, generate_team_meta, GeneratedMeta};
pub use store_page::{
    build_prompt as store_page_prompt, generate_store_page, generate_team_store_page,
    verify_no_invented_numbers, GeneratedStorePage, StorePageContext,
};
pub use successor::suggest_successor;
pub use tech::{assemble_tech_html, structure_team_tech_specs, structure_tech_specs, TechGroup, TechRow, TechSpecsResult, TECH_GROUPS};

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
///
/// ⚠️ **Bu liste artık yalnızca VARSAYILAN.** Gerçek zincir `settings`'ten okunuyor
/// (`gemini_model_chain`) — model emekliye ayrıldığında kullanıcı yeni sürüm beklemeden
/// listeyi düzeltebilsin diye. Buradaki liste ayar boşken veya bozukken devreye giriyor:
/// taze kurulumda ve "ayarı sildim" durumunda uygulamanın yine de üretebilmesi gerekiyor.
/// Aynı ders 0ao'da: *kodda yalnızca varsayılan, gerçek değer `settings`'te.*
pub const DEFAULT_MODEL_CHAIN: &[&str] = &[
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

pub struct WorkspaceGemini {
    endpoint: reqwest::Url,
    workspace: String,
    publishable_key: String,
    access_token: String,
}

impl WorkspaceGemini {
    pub fn new(
        project: &str,
        workspace: String,
        publishable_key: String,
        access_token: String,
    ) -> Result<Self, String> {
        use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};

        let url = reqwest::Url::parse(project).map_err(|_| "Supabase proje adresi geçersiz.")?;
        let hosted_project = url.host_str()
            .and_then(|host| host.strip_suffix(".supabase.co"))
            .is_some_and(|reference| reference.len() == 20 && reference.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
        if !hosted_project || url.scheme() != "https" || !url.username().is_empty()
            || url.password().is_some() || url.port().is_some() || url.path() != "/"
            || url.query().is_some() || url.fragment().is_some() {
            return Err("Ortak Gemini için bağlantılı Supabase projesinin standart HTTPS adresi gerekli.".into());
        }
        let valid_workspace = workspace.len() == 36 && workspace.bytes().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        });
        let valid_publishable_key = publishable_key.strip_prefix("sb_publishable_").is_some_and(|key| {
            !key.is_empty() && key.len() <= 256 && key.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
        });
        if !valid_workspace || !valid_publishable_key || access_token.len() > 8192 {
            return Err("Ortak Gemini oturum bilgileri geçersiz.".into());
        }

        // The issuer is a routing guard. Supabase verifies the real session signature and membership.
        let invalid_session = |_| "Supabase oturumu bu projeyle eşleşmiyor veya süresi dolmuş.".to_string();
        let header = decode_header(&access_token).map_err(invalid_session)?;
        let mut validation = Validation::new(header.alg);
        validation.set_issuer(&[format!("{}/auth/v1", url.origin().ascii_serialization())]);
        validation.set_audience(&["authenticated"]);
        validation.set_required_spec_claims(&["iss", "aud", "sub", "exp"]);
        validation.leeway = 0;
        validation.insecure_disable_signature_validation();
        decode::<serde_json::Value>(&access_token, &DecodingKey::from_secret(b""), &validation)
            .map_err(invalid_session)?;

        let endpoint = url.join("/functions/v1/team-gemini")
            .map_err(|_| "Supabase işlev adresi geçersiz.")?;
        Ok(Self { endpoint, workspace, publishable_key, access_token })
    }
}

fn valid_workspace_model(model: &str) -> bool {
    let suffix = model.strip_prefix("gemini-").or_else(|| model.strip_prefix("gemma-"));
    suffix.is_some_and(|suffix| {
        let bytes = suffix.as_bytes();
        let alphanumeric = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
        bytes.len() <= 77
            && bytes.first().copied().is_some_and(alphanumeric)
            && bytes.iter().copied().enumerate().all(|(index, byte)| {
                if byte != b'.' {
                    return alphanumeric(byte) || byte == b'-';
                }
                index > 0 && alphanumeric(bytes[index - 1])
                    && bytes.get(index + 1).copied().is_some_and(alphanumeric)
            })
    })
}

pub(crate) async fn post_workspace_generate(
    client: &reqwest::Client,
    connection: &WorkspaceGemini,
    model: &str,
    body: &serde_json::Value,
    chain: &ChainCtx<'_>,
) -> Result<String, (bool, String)> {
    if !valid_workspace_model(model) {
        return Err((true, format!("{model} ortak Gemini için geçerli bir model adı değil.")));
    }
    let response = client.post(connection.endpoint.clone())
        .header("apikey", &connection.publishable_key)
        .bearer_auth(&connection.access_token)
        .json(&serde_json::json!({ "workspace": connection.workspace, "operation": "generate", "model": model, "request": body }))
        .send().await.map_err(|_| {
            chain.kaydet(model, 0, false);
            (false, "Ortak Gemini servisine ulaşılamadı. Bağlantıyı kontrol edip yeniden deneyin.".to_string())
        })?;
    let status = response.status();
    if status.is_success() {
        chain.kaydet(model, status.as_u16(), true);
        let data: serde_json::Value = response.json().await
            .map_err(|_| (false, "Ortak Gemini yanıtı okunamadı.".to_string()))?;
        return data["candidates"][0]["content"]["parts"][0]["text"].as_str().map(str::to_owned)
            .ok_or_else(|| (false, "Ortak Gemini metin yanıtı üretmedi.".to_string()));
    }

    #[derive(Deserialize)]
    struct FunctionError {
        error: String,
        upstream_status: Option<u16>,
        retry_with_another_model: Option<bool>,
    }
    let failure = response.json::<FunctionError>().await.ok();
    let upstream = failure.as_ref().and_then(|e| e.upstream_status).unwrap_or(0);
    chain.kaydet(model, upstream, false);
    let retry = failure.as_ref().and_then(|e| e.retry_with_another_model).unwrap_or(false);
    let message = match failure.as_ref().map(|e| e.error.as_str()) {
        Some("sign_in_required") => "Ekip oturumu doğrulanamadı. Sistem → Ekip'ten oturumu kontrol edin.".into(),
        Some("integration_access_denied") => "Ortak Gemini üretimi için çalışma alanında düzenleme yetkisi gerekli.".into(),
        Some("gemini_not_configured" | "invalid_configuration") => "Bu çalışma alanı için ortak Gemini bağlantısı hazır değil.".into(),
        Some("gemini_credentials_rejected") => "Gemini sunucudaki ortak anahtarı reddetti.".into(),
        Some("gemini_model_not_allowed") => format!("{model} ortak çalışma alanında etkin değil."),
        Some("gemini_model_unavailable") => format!("{model} Gemini'de artık kullanılabilir değil."),
        Some("gemini_rate_limited") => format!("{model} Gemini kota sınırına ulaştı."),
        Some("gemini_prompt_blocked") => "Gemini bu taslağın üretimini engelledi.".into(),
        Some("gemini_request_rejected") => format!("{model} taslak üretme isteğini kabul etmedi."),
        Some("request_too_large") => "Bu sayfanın üretim bağlamı çok büyük.".into(),
        _ if status.as_u16() == 401 => "Ekip oturumu doğrulanamadı. Sistem → Ekip'ten oturumu kontrol edin.".into(),
        _ if status.as_u16() == 404 => "Ortak Gemini servisi bu Supabase projesinde henüz kurulu değil.".into(),
        _ => format!("Ortak Gemini isteği tamamlanamadı (HTTP {}).", status.as_u16()),
    };
    Err((retry, message))
}

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
    /// ⚠️ `&'static str` DEĞİL: zincir artık ayarlardan geliyor, model adı derleme
    /// zamanında bilinmiyor.
    pub model: String,
}

/// Gemini'ye giden **tek bir istek**. Sayaç bunu sayıyor.
///
/// Neden var: ücretsiz katmanda flash modellerin günlük limiti 20 ve kullanıcı bunu ancak
/// üretim alt modele düştüğünde fark ediyordu. Kapasitenin görünmesi gerekiyor.
///
/// ⚠️ Burada **kalan hak hesaplanmıyor**, yalnızca yapılan istek bildiriliyor. Kullanıcı
/// kararı (2026-08-12): *"20 hakkın var, 14 kullandın dersek bir beklenti oluşur"* — oysa
/// sayabildiğimiz tek şey bu uygulamanın gönderdikleri; aynı anahtar başka yerde
/// kullanılırsa sayı eksik kalır. Eksik sayıyı "kalan hak" diye sunmak yanlış güven verir.
pub struct CallRecord<'a> {
    pub model: &'a str,
    /// HTTP durum kodu. **0 = istek hiç gitmedi** (ağ hatası) — kotadan düşmez, ama
    /// "denendi mi?" sorusunun cevabı olduğu için yine kaydedilir.
    pub http_code: u16,
    pub ok: bool,
}

/// Kayıt kanalı. `seo-core` veritabanı bilmez; kaydı Tauri katmanı yazar.
pub type CallLog<'a> = &'a (dyn Fn(CallRecord<'_>) + Send + Sync);

/// Bir üretimin **hangi modelleri hangi sırayla deneyeceği** + isteklerin nereye yazılacağı.
///
/// Tek yapıda duruyorlar çünkü ikisi de aynı yerden geliyor (ayarlar + veritabanı) ve
/// yedi giriş noktasının hepsine birlikte gidiyorlar; ayrı iki parametre olsalardı
/// birini eklemeyi unutmak sessizce eksik sayıma yol açardı.
pub struct ChainCtx<'a> {
    pub models: Vec<String>,
    pub log: Option<CallLog<'a>>,
}

impl<'a> ChainCtx<'a> {
    /// Ayar metninden zincir kurar; boş/bozuksa `fallback`'e düşer.
    ///
    /// Biçim bilinçli olarak gevşek: satır satır **veya** virgülle. Kullanıcı listeyi elle
    /// düzeltirken hangi ayırıcıyı kullandığını düşünmek zorunda kalmamalı.
    ///
    /// ⚠️ Boş listeye ASLA düşülmez. Zincir boşsa üretim "tüm modeller denendi" diyerek
    /// hiç istek göndermeden başarısız olurdu — kullanıcı sebebini anlayamazdı.
    pub fn from_setting(raw: &str, fallback: &[&str]) -> Self {
        let mut models: Vec<String> = Vec::new();
        for parca in raw.split(['\n', ',']) {
            let m = parca.trim();
            if m.is_empty() || models.iter().any(|x| x == m) {
                continue;
            }
            models.push(m.to_string());
        }
        if models.is_empty() {
            models = fallback.iter().map(|m| m.to_string()).collect();
        }
        Self { models, log: None }
    }

    /// Ayarsız varsayılan üretim zinciri — testler ve ayar okunamadığı durumlar için.
    pub fn defaults() -> Self {
        Self::from_setting("", DEFAULT_MODEL_CHAIN)
    }

    /// Ayarsız varsayılan **sohbet** zinciri. Ayrı olması bilinçli (bkz. `CHAT_CHAIN`).
    pub fn chat_defaults() -> Self {
        Self::from_setting("", CHAT_CHAIN)
    }

    pub fn with_log(mut self, log: CallLog<'a>) -> Self {
        self.log = Some(log);
        self
    }

    pub(crate) fn kaydet(&self, model: &str, http_code: u16, ok: bool) {
        if let Some(f) = self.log {
            f(CallRecord { model, http_code, ok });
        }
    }
}

/// **Gemini'ye giden tek kapı** (akış tabanlı sohbet hariç).
///
/// 🔴 Neden tek kapı: bu istek dört ayrı dosyada birebir kopyalanmıştı (meta, açıklama,
/// teknik tablo, halef). Sayaç kopyalardan birini atlarsa **sessizce** yanlış sayar — ve
/// projenin tekrar eden dersi tam olarak bu: *kopyalanan mantık zamanla sapar, paylaşılır.*
///
/// Yalnızca ortak olan kısım burada: istek gönderme, durum kodu, hata sınıflandırma, kayıt
/// ve yanıtın içindeki metni çıkarma. Gövde kurma ve o metnin çözümlenmesi çağıranda kalıyor
/// — şemaları farklı.
async fn post_generate(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    body: &serde_json::Value,
    chain: &ChainCtx<'_>,
) -> Result<String, (bool, String)> {
    let url = format!("{API_BASE}/{model}:generateContent");
    let resp = match client.post(&url).query(&[("key", api_key)]).json(body).send().await {
        Ok(r) => r,
        Err(e) => {
            // İstek hiç gitmedi: kotadan düşmez ama denendi. `http_code = 0`.
            chain.kaydet(model, 0, false);
            return Err((false, format!("İstek gönderilemedi: {e}")));
        }
    };
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    chain.kaydet(model, status.as_u16(), status.is_success());

    if !status.is_success() {
        return Err(classify_error(status.as_u16(), &text, model));
    }
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| (false, format!("Yanıt çözümlenemedi: {e}")))?;
    v["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| (false, format!("Beklenmeyen yanıt biçimi: {}", short(&text))))
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

/// Google'ın **şu an sunduğu** modelleri listeler.
///
/// 🔴 Bu fonksiyonun varlık sebebi: model listesi kodda sabit durduğu sürece bayatlıyor ve
/// bayatlaması üretimi durduruyor (2026-07-28: `gemini-1.5-flash` emekli oldu, üretim tamamen
/// durdu, düzeltmek yeni sürüm gerektirdi). Liste canlıdan gelirse emekli model kullanıcının
/// karşısına hiç çıkmaz.
///
/// ⚠️ Yalnızca `generateContent` destekleyenler dönüyor: uç nokta gömme (embedding) ve
/// görüntü modellerini de listeliyor, onlar bu uygulamada bir işe yaramaz.
///
/// ⚠️ **Bu liste "çalışır" garantisi vermez.** Uygulamanın isteği `system_instruction` +
/// `responseSchema` kullanıyor ve her model ikisini birden desteklemiyor; uç nokta bunu
/// söylemiyor. Emin olmanın tek yolu `probe_model` — gerçek istek biçimiyle bir deneme.
pub async fn list_models(api_key: &str) -> Result<Vec<String>, String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("Gemini API anahtarı ayarlı değil. Ayarlar'dan ekleyin.".to_string());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;

    let mut out: Vec<String> = Vec::new();
    let mut sayfa: Option<String> = None;
    // Uç nokta sayfalı; tek sayfa okumak listeyi sessizce eksik gösterirdi.
    loop {
        let mut req = client.get(API_BASE).query(&[("key", key), ("pageSize", "200")]);
        if let Some(t) = &sayfa {
            req = req.query(&[("pageToken", t.as_str())]);
        }
        let resp = req.send().await.map_err(|e| format!("Gemini'ye ulaşılamadı: {e}"))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(match status.as_u16() {
                400 | 403 => "Anahtar reddedildi · geçersiz veya yetkisiz.".to_string(),
                c => format!("Model listesi alınamadı (HTTP {c}): {}", short(&text)),
            });
        }
        let v: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("Yanıt çözümlenemedi: {e}"))?;
        for m in v["models"].as_array().unwrap_or(&Vec::new()) {
            let destekli = m["supportedGenerationMethods"]
                .as_array()
                .map(|a| a.iter().any(|x| x.as_str() == Some("generateContent")))
                .unwrap_or(false);
            if !destekli {
                continue;
            }
            // "models/gemini-3.6-flash" → "gemini-3.6-flash"; zincirde bu biçim kullanılıyor.
            if let Some(ad) = m["name"].as_str().and_then(|n| n.strip_prefix("models/")) {
                out.push(ad.to_string());
            }
        }
        match v["nextPageToken"].as_str() {
            Some(t) if !t.is_empty() => sayfa = Some(t.to_string()),
            _ => break,
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// Tek bir modeli **uygulamanın gerçek istek biçimiyle** dener.
///
/// `list_models` "bu model var" der; bu fonksiyon "bu model BİZİM isteğimizi kaldırıyor" der.
/// İkisi farklı sorular — aradaki fark `system_instruction` + `responseSchema` desteği.
///
/// ⚠️ Bu deneme kotadan düşen **gerçek bir istek**; sayaca da yazılıyor. Bilerek en küçük
/// istek: tek kelimelik çıktı isteyen bir şema.
///
/// ⚠️ `chain` burada **zincir olarak kullanılmıyor** — denenen model ayrı parametre. Yalnızca
/// kayıt kanalını taşıyor; deneme de kotadan düşen bir istek olduğu için sayaca girmeli.
pub async fn probe_model(
    api_key: &str,
    model: &str,
    chain: &ChainCtx<'_>,
) -> Result<String, String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("Gemini API anahtarı ayarlı değil.".to_string());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;

    let body = serde_json::json!({
        "system_instruction": { "parts": [{ "text": "Yalnızca istenen JSON'u döndür." }] },
        "contents": [{ "parts": [{ "text": "\"tamam\" kelimesini döndür." }] }],
        "generationConfig": {
            "responseMimeType": "application/json",
            "responseSchema": {
                "type": "OBJECT",
                "properties": { "ok": { "type": "STRING" } },
                "required": ["ok"]
            },
            "temperature": 0.0
        }
    });

    match post_generate(&client, key, model, &body, chain).await {
        Ok(inner) => {
            // Şemaya uyan bir JSON dönmeli; dönmüyorsa model bu biçimi kaldırmıyor demektir.
            serde_json::from_str::<serde_json::Value>(&inner)
                .map(|_| format!("{model} çalışıyor · istek biçimimizi destekliyor."))
                .map_err(|_| format!("{model} yanıt verdi ama beklenen JSON biçiminde değil."))
        }
        Err((_, msg)) => Err(msg),
    }
}

#[cfg(test)]
mod tests {
    //! Zincir ve anahtar doğrulama testleri — modüller arası ortak davranış.
    use super::*;

    #[test]
    fn workspace_gemini_binds_the_session_to_the_hosted_project() {
        let project = "https://aaaaaaaaaaaaaaaaaaaa.supabase.co";
        let workspace = "11111111-1111-4111-8111-111111111111";
        let claims = serde_json::json!({
            "iss": format!("{project}/auth/v1"), "aud": "authenticated", "sub": workspace,
            "exp": jsonwebtoken::get_current_timestamp() + 120,
        });
        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(), &claims,
            &jsonwebtoken::EncodingKey::from_secret(b"synthetic-test-signing-key"),
        ).unwrap();
        let connect = |target: &str| WorkspaceGemini::new(target, workspace.into(), "sb_publishable_test".into(), token.clone());
        let connection = connect(project).unwrap();
        assert_eq!(connection.endpoint.as_str(), format!("{project}/functions/v1/team-gemini"));
        assert!(connect("https://bbbbbbbbbbbbbbbbbbbb.supabase.co").is_err());
        assert!(connect("https://aaaaaaaaaaaaaaaaaaaa.supabase.co.example.com").is_err());
        assert!(connect("https://aaaaaaaaaaaaaaaaaaaa.supabase.co:444").is_err());
        assert!(connect("http://aaaaaaaaaaaaaaaaaaaa.supabase.co").is_err());
    }

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
        assert!(!DEFAULT_MODEL_CHAIN.is_empty());
        for m in DEFAULT_MODEL_CHAIN {
            assert!(!m.contains("preview"), "zincirde preview model var: {m}");
        }
        assert!(
            DEFAULT_MODEL_CHAIN.iter().any(|m| m.contains("latest")),
            "zincirde bir 'latest' takma adı olmalı — liste bayatlasa da canlı kalsın"
        );
        // Havuzu en geniş model en sonda dursun: yukarıdakilerin tamamı tükenirse
        // üretim yine de durmasın.
        assert_eq!(
            *DEFAULT_MODEL_CHAIN.last().unwrap(),
            "gemma-4-31b-it",
            "son çare, günlük limiti en yüksek model olmalı"
        );
        // Emekli modeller geri sızmasın
        for dead in ["gemini-1.5-flash", "gemini-1.5-pro", "gemini-1.0-pro"] {
            assert!(!DEFAULT_MODEL_CHAIN.contains(&dead), "emekli model zincirde: {dead}");
        }
    }

    #[test]
    fn workspace_model_names_accept_release_versions_and_reject_path_parts() {
        for model in DEFAULT_MODEL_CHAIN {
            assert!(valid_workspace_model(model), "Ortak Gemini model adı reddedildi: {model}");
        }
        for model in [
            "gemini-.", "gemini-2..5-flash", "gemini-2.-5-flash", "gemini-2.5-flash.",
            "gemini-flash/other", "gemini-flash%2fother", "gemini-flash?key=y",
        ] {
            assert!(!valid_workspace_model(model), "Geçersiz model adı kabul edildi: {model}");
        }
    }

    /// Ayar okuma: iki ayırıcı, kırpma, tekrar eleme.
    #[test]
    fn ayardan_zincir_kuruluyor() {
        let c = ChainCtx::from_setting(" a , b \n c \n\n b ", DEFAULT_MODEL_CHAIN);
        assert_eq!(c.models, ["a", "b", "c"], "kırpılmalı, tekrar elenmeli, iki ayırıcı da geçerli");
    }

    /// 🔴 Zincir ASLA boş kalmaz. Boş kalsaydı üretim tek bir istek bile göndermeden
    /// "tüm modeller denendi" derdi — kullanıcı sebebini anlayamazdı.
    #[test]
    fn bos_veya_bozuk_ayar_varsayilana_duser() {
        for bozuk in ["", "   ", ",,,", "\n \n", " , \n ,"] {
            let c = ChainCtx::from_setting(bozuk, DEFAULT_MODEL_CHAIN);
            assert_eq!(
                c.models.len(),
                DEFAULT_MODEL_CHAIN.len(),
                "bozuk ayar ({bozuk:?}) varsayılana düşmeli"
            );
        }
        // Sohbet zinciri ayrı: üretimin 20/gün kotasını yiyemez (bkz. CHAT_CHAIN).
        assert_ne!(
            ChainCtx::defaults().models,
            ChainCtx::chat_defaults().models,
            "sohbet ve üretim zincirleri ayrı kalmalı"
        );
    }

    /// Sayaç, zincir kaç modele düşerse o kadar istek görmeli — "bir üretim = bir istek"
    /// varsayımı sayıyı olduğundan düşük gösterirdi.
    #[test]
    fn kayit_kanali_her_istegi_bildiriyor() {
        use std::sync::Mutex;
        let kayitlar: Mutex<Vec<(String, u16, bool)>> = Mutex::new(Vec::new());
        let f = |r: CallRecord<'_>| {
            kayitlar.lock().unwrap().push((r.model.to_string(), r.http_code, r.ok));
        };
        let c = ChainCtx::from_setting("a,b", DEFAULT_MODEL_CHAIN).with_log(&f);
        c.kaydet("a", 429, false);
        c.kaydet("b", 200, true);
        let v = kayitlar.lock().unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0], ("a".to_string(), 429, false));
        assert_eq!(v[1], ("b".to_string(), 200, true));
    }

    /// Kayıt kanalı YOKKEN de çalışmalı: testler ve kayıt istemeyen yollar için.
    #[test]
    fn kayit_kanali_olmadan_da_calisiyor() {
        ChainCtx::defaults().kaydet("a", 200, true);
    }

    #[tokio::test]
    #[ignore]
    async fn test_key_real() {
        let key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY ayarlı değil");
        let msg = test_key(&key).await.expect("anahtar testi başarısız");
        println!("{msg}");
    }
}
