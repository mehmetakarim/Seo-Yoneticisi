//! Yapay Zekâ Asistanı — akışlı sohbet.
//!
//! Uygulamanın diğer Gemini kullanımlarından üç yerde ayrılıyor:
//!
//! 1. **Akışlı** (`streamGenerateContent?alt=sse`). Diğerleri tek yanıt bekliyor; sohbette
//!    kullanıcı 5-15 saniye boş ekrana bakamaz.
//! 2. **Kendi model zinciri var** (aşağıdaki `CHAT_CHAIN`) — normal `MODEL_CHAIN` değil.
//! 3. **`responseSchema` YOK.** Serbest metin isteniyor; şema dayatmak sohbeti bozardı.
//!
//! ⚠️ **En kritik davranış: `"thought": true` parçaları filtrelenir.**
//! Ölçüm (2026-07-30, `gemma-4-31b-it`, tek cümlelik bir soru): akışta **19 düşünce
//! parçasına karşılık 3 cevap parçası** geldi. Filtre olmasaydı kullanıcı Türkçe cevap
//! yerine modelin İngilizce iç muhakemesini görürdü — yani naif bir akış uygulaması
//! tamamen bozuk görünürdü.
//!
//! İkincil sonuç: akışın çoğu atıldığı için "token token yazılıyor" hissi zayıf. Bu yüzden
//! `on_event` düşünce parçalarını da (içeriksiz olarak) bildiriyor: arayüz o sırada
//! "düşünüyor…" gösterebiliyor — kör bir döner ikondan farkı, modelin gerçekten çalıştığını
//! biliyor olmamız.

use super::*;
use futures_util::StreamExt;

/// Sohbetin model zinciri — normal `MODEL_CHAIN`'in **tersi mantıkla** kurulu.
///
/// ⚠️ Sohbet, uygulamanın asıl işiyle (meta/açıklama/teknik üretimi) **aynı kotayı
/// paylaşamaz**. `flash` modellerinin günlük limiti 20; birkaç sohbet turu o kotayı bitirir
/// ve kullanıcı asıl işini yapamaz hâle gelir. Bu yüzden en geniş havuz (`gemma-4-31b-it`,
/// 14.400 istek/gün) BAŞA alındı; flash-lite'lar yalnızca yedek.
///
/// Gemma'nın üslubu Gemini'lerden sapabilir ama sohbette bu bir kusur değil; üretimde
/// olduğu gibi biçim garantisi aranmıyor.
pub const CHAT_CHAIN: &[&str] = &[
    "gemma-4-31b-it",        // 30/dk, 14.400/gün — sohbetin doğal evi
    "gemini-3.5-flash-lite", // 15/dk, 500/gün
    "gemini-3.1-flash-lite",
    "gemini-flash-latest",   // takma ad: liste bayatlasa da canlı kalır
];

/// Asistanın uyacağı sınır — **saf metin, ağsız, test edilebilir.**
///
/// Tauri katmanında değil burada: projenin kuralı "iş mantığı `seo-core`'da". Prompt bir
/// davranış sözleşmesi ve testleri de burada duruyor.
///
/// Üç kural da halüsinasyona karşı:
/// veriye bağlı kal, bilmediğini söyle, sayı uydurma.
pub fn assistant_system_prompt(context: &str) -> String {
    format!(
        "Sen bu SEO yönetim uygulamasının içinde çalışan bir analiz asistanısın. \
Kullanıcı bir e-ticaret kataloğunun SEO'sunu yönetiyor ve Google Search Console verisine bakıyor.\n\n\
KURALLAR:\n\
1. YALNIZCA aşağıdaki VERİ bölümüne dayan. Orada olmayan bir sayfa, sorgu veya sayı hakkında konuşma.\n\
2. Sorulan şey veride yoksa açıkça \"bu veride yok\" de. Tahmin yürütme.\n\
3. Sayı UYDURMA. Verideki sayıları aynen kullan; kendin hesap yapacaksan nasıl hesapladığını söyle.\n\
4. Türkçe, kısa ve somut yaz. Genel SEO tavsiyesi değil, BU veriye dayalı öneri ver.\n\
5. Bir işlem yapamazsın (canonical yazma, içerik üretme gibi); kullanıcıya hangi ekrandan \
yapabileceğini söyle.\n\n\
Biçim: düz metin. Vurgu için **kalın**, listeler için satır başına \"- \" kullanabilirsin.\n\n\
=== VERİ ===\n{context}"
    )
}

/// Sohbet turundaki tek bir mesaj.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// `"user"` veya `"model"` — Gemini'nin beklediği adlandırma.
    pub role: String,
    pub text: String,
}

/// Akış sırasında çağırana bildirilen olaylar.
#[derive(Debug, Clone, PartialEq)]
pub enum ChatEvent<'a> {
    /// Model düşünüyor (içerik yok). Arayüz gösterge çizebilir.
    Thinking,
    /// Cevabın bir parçası.
    Chunk(&'a str),
}

/// Bir SSE satırından metin parçalarını çıkarır — **ağsız, test edilebilir.**
///
/// Dönen değer: `(cevap_parçaları, düşünce_parçası_var_mı)`.
///
/// ⚠️ `thought: true` olan parçalar cevaba KARIŞTIRILMAZ. Bu fonksiyonun tek işi bu ayrımı
/// doğru yapmak; testleri de bunu sabitliyor.
fn parse_sse_line(line: &str) -> Option<(Vec<String>, bool)> {
    let payload = line.strip_prefix("data: ")?.trim();
    if payload.is_empty() || payload == "[DONE]" {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(payload).ok()?;
    let parts = v
        .get("candidates")?
        .as_array()?
        .first()?
        .get("content")?
        .get("parts")?
        .as_array()?;

    let mut out = Vec::new();
    let mut thinking = false;
    for p in parts {
        let text = p.get("text").and_then(|t| t.as_str()).unwrap_or("");
        if p.get("thought").and_then(|t| t.as_bool()).unwrap_or(false) {
            thinking = true;
        } else if !text.is_empty() {
            out.push(text.to_string());
        }
    }
    Some((out, thinking))
}

fn build_body(system: &str, history: &[ChatMessage]) -> serde_json::Value {
    serde_json::json!({
        "system_instruction": { "parts": [{ "text": system }] },
        "contents": history
            .iter()
            .map(|m| serde_json::json!({ "role": m.role, "parts": [{ "text": m.text }] }))
            .collect::<Vec<_>>(),
    })
}

/// Sohbet turunu akışlı çalıştırır. Tam cevabı döndürür; parçalar `on_event` ile bildirilir.
///
/// Zincir davranışı üretim komutlarıyla aynı: bir model kota/emeklilik nedeniyle düşerse
/// sıradakine geçilir (`classify_error`). ⚠️ Ama **akış başladıktan sonra** kopan bir
/// bağlantıda zincire dönülmez — kullanıcı yarım bir cevap görmüşken baştan başlamak
/// kafa karıştırıcı olurdu; hata olduğu gibi bildirilir.
pub async fn chat_stream<F>(
    api_key: &str,
    system: &str,
    history: &[ChatMessage],
    mut on_event: F,
) -> Result<Produced<String>, String>
where
    F: FnMut(ChatEvent<'_>),
{
    if api_key.trim().is_empty() {
        return Err("Gemini API anahtarı girilmemiş. Ayarlar'dan ekleyin.".into());
    }
    if history.is_empty() {
        return Err("Gönderilecek mesaj yok.".into());
    }

    let client = reqwest::Client::new();
    let body = build_body(system, history);
    let mut last_err = String::new();

    for model in CHAT_CHAIN {
        let url = format!("{API_BASE}/{model}:streamGenerateContent?alt=sse&key={api_key}");
        let resp = match client.post(&url).json(&body).send().await {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("Gemini'ye ulaşılamadı: {e}");
                continue;
            }
        };

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            let (retry, msg) = classify_error(status.as_u16(), &text, model);
            last_err = msg;
            if retry {
                continue;
            }
            return Err(last_err);
        }

        // Akış: gövde parçalar hâlinde geliyor ve bir SSE satırı iki parçaya BÖLÜNEBİLİR.
        // Bu yüzden tampon tutuluyor; yalnızca tam satırlar işleniyor.
        let mut stream = resp.bytes_stream();
        let mut buf = String::new();
        let mut full = String::new();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| format!("{model} akışı kesildi: {e}"))?;
            buf.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(nl) = buf.find('\n') {
                let line: String = buf.drain(..=nl).collect();
                if let Some((texts, thinking)) = parse_sse_line(&line) {
                    if thinking && texts.is_empty() {
                        on_event(ChatEvent::Thinking);
                    }
                    for t in texts {
                        full.push_str(&t);
                        on_event(ChatEvent::Chunk(&t));
                    }
                }
            }
        }

        if full.trim().is_empty() {
            // Model yalnızca düşünce üretip sustu — sıradakini denemek mantıklı.
            last_err = format!("{model} boş cevap döndürdü.");
            continue;
        }
        return Ok(Produced { value: full, model });
    }

    Err(if last_err.is_empty() {
        "Asistan yanıt veremedi.".to_string()
    } else {
        format!("Asistan yanıt veremedi. Son hata: {last_err}")
    })
}

#[cfg(test)]
mod tests {
    //! Akış ayrıştırma testleri — asıl korunan davranış düşünce/cevap ayrımı.
    use super::*;

    /// ⚠️ Bu testin koruduğu şey: kullanıcı modelin iç muhakemesini GÖRMEMELİ.
    /// Gerçek ölçümde (gemma-4-31b-it) akışın %86'sı düşünce parçasıydı.
    #[test]
    fn dusunce_parcalari_cevaba_karismaz() {
        let dusunce = r#"data: {"candidates":[{"content":{"parts":[{"text":"Role: SEO assistant. Constraint:","thought":true}],"role":"model"}}]}"#;
        let (texts, thinking) = parse_sse_line(dusunce).expect("ayrıştırılmalı");
        assert!(texts.is_empty(), "düşünce metni cevaba sızdı: {texts:?}");
        assert!(thinking, "düşünce olduğu bildirilmeli");

        let cevap = r#"data: {"candidates":[{"content":{"parts":[{"text":"urun/abc sayfası 36 tıklama aldı."}],"role":"model"}}]}"#;
        let (texts, thinking) = parse_sse_line(cevap).expect("ayrıştırılmalı");
        assert_eq!(texts, vec!["urun/abc sayfası 36 tıklama aldı."]);
        assert!(!thinking);
    }

    #[test]
    fn ayni_olayda_hem_dusunce_hem_cevap_olabilir() {
        // Tek `data:` satırında iki parça gelirse cevap kısmı kaybolmamalı.
        let line = r#"data: {"candidates":[{"content":{"parts":[{"text":"düşünüyorum","thought":true},{"text":"Cevap: 12"}],"role":"model"}}]}"#;
        let (texts, thinking) = parse_sse_line(line).unwrap();
        assert_eq!(texts, vec!["Cevap: 12"]);
        assert!(thinking);
    }

    #[test]
    fn veri_olmayan_satirlar_atlanir() {
        assert!(parse_sse_line("").is_none());
        assert!(parse_sse_line("\n").is_none());
        assert!(parse_sse_line(": keep-alive").is_none());
        assert!(parse_sse_line("data: [DONE]").is_none());
        assert!(parse_sse_line("data: {bozuk").is_none());
        // Beklenmeyen ama geçerli JSON: çökmemeli.
        assert!(parse_sse_line(r#"data: {"promptFeedback":{}}"#).is_none());
    }

    #[test]
    fn govde_rolleri_ve_sistem_talimatini_tasir() {
        let body = build_body(
            "Yalnızca verilen veriye dayan.",
            &[
                ChatMessage { role: "user".into(), text: "merhaba".into() },
                ChatMessage { role: "model".into(), text: "buyurun".into() },
            ],
        );
        assert_eq!(body["system_instruction"]["parts"][0]["text"], "Yalnızca verilen veriye dayan.");
        assert_eq!(body["contents"][0]["role"], "user");
        assert_eq!(body["contents"][1]["parts"][0]["text"], "buyurun");
        // Sohbette şema DAYATILMAZ — serbest metin isteniyor.
        assert!(body.get("generationConfig").is_none());
    }

    /// ⚠️ Bu testin koruduğu şey projenin baştan beri koyduğu sınır: asistan ölçüm yapmaz,
    /// yorum yapar. Kurallar prompt'tan düşerse halüsinasyon kapısı açılır.
    #[test]
    fn sistem_talimati_halusinasyon_sinirini_kuruyor() {
        let p = assistant_system_prompt("sayfa=urun/abc tiklama=36");
        assert!(p.contains("YALNIZCA"), "veriye bağlı kalma kuralı yok");
        assert!(p.contains("bu veride yok"), "bilmediğini söyleme kuralı yok");
        assert!(p.contains("UYDURMA"), "sayı uydurmama kuralı yok");
        assert!(p.contains("Bir işlem yapamazsın"), "yazma yetkisi olmadığı belirtilmemiş");
        // Bağlam prompt'un İÇİNDE olmalı; ayrı mesaj olarak gönderilirse model onu
        // kullanıcı isteği sanıp yönergeymiş gibi davranabilir.
        assert!(p.ends_with("sayfa=urun/abc tiklama=36"));
    }

    #[test]
    fn sohbet_zinciri_genis_havuzla_basliyor() {
        // ⚠️ Regresyon koruması: zincir başı flash olursa uygulamanın asıl işi olan
        // üretimin günlük kotası (20) sohbetle tükenir.
        assert_eq!(CHAT_CHAIN[0], "gemma-4-31b-it");
        assert!(
            !CHAT_CHAIN.iter().any(|m| m.contains("preview")),
            "-preview modeller habersiz kaybolur, zincire konmaz",
        );
    }

    /// ⚠️ Halüsinasyon sınırının CANLI testi: veride OLMAYAN bir şey sorulunca model
    /// uydurmamalı, bilmediğini söylemeli. Prompt'un tek başına doğru yazılmış olması
    /// yetmez — modelin ona uyduğu görülmeli.
    /// `GEMINI_API_KEY=... cargo test -p seo-core veride_olmayani_uydurmuyor -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn veride_olmayani_uydurmuyor() {
        let key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY ayarlı değil");
        let ctx = "FIRSATLAR:\n- Ergotron WorkFit-T [DGR.AKS.33-397] gösterim=474 tıklama=2 konum=4.1";
        let out = chat_stream(
            &key,
            &assistant_system_prompt(ctx),
            &[ChatMessage {
                role: "user".into(),
                text: "Geçen yılın toplam cirosu ne kadardı?".into(),
            }],
            |_| {},
        )
        .await
        .expect("akış başarısız");
        println!("CEVAP={}", out.value);
        let v = out.value.to_lowercase();
        assert!(
            v.contains("veride yok") || v.contains("bilgi yok") || v.contains("bulunmuyor")
                || v.contains("yer almıyor") || v.contains("mevcut değil"),
            "model ciro verisi olmadığını söylemedi: {}",
            out.value
        );
        // Ciro rakamı uydurmamalı — TL/₺ geçen bir tutar cevapta olmamalı.
        assert!(!v.contains("₺"), "para birimi uydurdu: {}", out.value);
    }

    /// Gerçek akış: düşünce/cevap oranını ve Türkçe yanıtı raporlar.
    /// `GEMINI_API_KEY=... cargo test -p seo-core chat_stream_real -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn chat_stream_real() {
        let key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY ayarlı değil");
        let mut dusunce = 0usize;
        let mut parca = 0usize;
        let out = chat_stream(
            &key,
            "Sen bir SEO analiz asistanısın. Yalnızca verilen veriye dayan. Türkçe yanıt ver.",
            &[ChatMessage {
                role: "user".into(),
                text: "Veri: sayfa=urun/abc, gosterim=1204, tiklama=36, konum=18.4. \
                       Tek cümleyle durumu özetle."
                    .into(),
            }],
            |e| match e {
                ChatEvent::Thinking => dusunce += 1,
                ChatEvent::Chunk(_) => parca += 1,
            },
        )
        .await
        .expect("akış başarısız");
        println!("MODEL={} DUSUNCE={dusunce} CEVAP_PARCA={parca}", out.model);
        println!("CEVAP={}", out.value);
        assert!(!out.value.trim().is_empty());
        assert!(
            !out.value.to_lowercase().contains("constraint:"),
            "düşünce zinciri cevaba sızmış: {}",
            out.value
        );
    }
}
