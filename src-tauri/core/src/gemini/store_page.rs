//! Kategori · marka · blog sayfaları için meta + tanıtım metni üretimi (Faz İ4).
//!
//! **Neden ürün hattı doğrudan kullanılamıyor.** `gemini::meta` bir ÜRÜNÜ anlatıyor: adı,
//! markası, kategorisi belli tek bir nesne. Kategori sayfası bir nesne değil, bir küme —
//! "Güvenlik Duvarı" sayfasının konusu içindeki ürünler ve o kümeyi arayan sorgular.
//! Bağlam farklı olduğu için prompt da farklı; ama uzunluk kuralları ve kırpma
//! `validation`'dan **paylaşılıyor**, kopyalanmıyor.
//!
//! 🔴 **Halüsinasyon kalkanı burada ürün tarafındakinden ZAYIF ve bunu saklamıyoruz.**
//! Teknik tabloda her sayı kaynak metne karşı doğrulanıyor (`tech::verify_traceable`);
//! burada doğrulanacak bir kaynak metin yok — sayfanın kendisi zaten boş, üretmemizin sebebi
//! o. Alınan önlemler:
//!   1. Bağlam **ölçülmüş veriden** geliyor: kategorideki gerçek ürünler ve o sayfanın
//!      GSC'de gerçekten aldığı sorgular. Model boşluğa yazmıyor.
//!   2. Prompt sayısal iddiayı **yasaklıyor** (fiyat, adet, hız, garanti süresi).
//!   3. `verify_no_invented_numbers` çıktıdaki her sayıyı bağlamda arıyor; bulunmayan sayı
//!      varsa metin **reddediliyor**, sessizce düzeltilmiyor.
//!   4. Çıktı taslak; operatör okumadan gönderilmiyor (akış tek tek, onaylı).
//!
//! ⚠️ Bu tam doğrulama DEĞİL: "Fortinet güvenlik duvarları kurumsal ağlarda kullanılır"
//! gibi sayısız bir cümle yanlış olsa da yakalanmaz. Kabul edilen sınır.

use super::*;
use crate::validation::{grapheme_len, DESC_MAX, DESC_MIN, TITLE_MAX, TITLE_MIN};

/// Üretim için bağlam — hepsi ölçülmüş veriden gelir, hiçbiri uydurulmaz.
pub struct StorePageContext<'a> {
    /// 'category' | 'brand' | 'blog'
    pub kind: &'a str,
    pub name: &'a str,
    /// Bu sayfanın GSC'de gerçekten aldığı sorgular (en çok gösterim alan birkaçı).
    pub queries: &'a [String],
    /// Kategori/markadaki gerçek ürün adları (birkaç örnek).
    pub products: &'a [String],
    /// Sayfada zaten duran tanıtım metni — varsa yeniden yazılır, yoksa sıfırdan.
    pub existing: Option<&'a str>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct GeneratedStorePage {
    pub page_title: String,
    pub meta_description: String,
    pub target_keyword: String,
    /// Sayfanın üstünde duracak tanıtım metni (düz paragraf, HTML değil).
    ///
    /// ⚠️ HTML istenmiyor: IdeaSoft `showcaseContent`'i HTML kabul ediyor ama modelden
    /// etiket istemek, kapanmayan etiketle sayfa düzenini bozma riski demek. Paragraflar
    /// gönderim anında sarmalanıyor.
    pub showcase: String,
}

fn system_prompt(kind: &str) -> String {
    let ne = match kind {
        "category" => "bir ürün kategorisi sayfası",
        "brand" => "bir marka sayfası",
        _ => "bir bilgi içeriği sayfası",
    };
    format!(
        "Sen bir Türkçe e-ticaret SEO editörüsün. {ne} için meta ve kısa tanıtım metni \
         yazacaksın.\n\
         KURALLAR:\n\
         1) 🔴 SAYI UYDURMA. Fiyat, adet, hız, kapasite, garanti süresi, yüzde YAZMA — \
            sana verilmeyen hiçbir sayı metinde geçmesin.\n\
         2) Marka/model adı uydurma; yalnızca sana verilen adları kullan.\n\
         3) Abartı ve pazarlama klişesi yok (\"en iyi\", \"lider\", \"eşsiz\"). \
            Ne olduğunu ve kime uygun olduğunu anlat.\n\
         4) Başlık {TITLE_MIN}-{TITLE_MAX} karakter, açıklama {DESC_MIN}-{DESC_MAX} karakter.\n\
         5) Tanıtım metni 2 kısa paragraf, toplam 300-600 karakter. Düz metin, HTML yok.\n\
         6) Hedef kelime, aramalarda en çok geçen ifadeden seçilir ve başlıkta geçer.\n\
         Yalnızca istenen JSON'u döndür."
    )
}

fn response_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "OBJECT",
        "properties": {
            "page_title": { "type": "STRING" },
            "meta_description": { "type": "STRING" },
            "target_keyword": { "type": "STRING" },
            "showcase": { "type": "STRING" }
        },
        "required": ["page_title", "meta_description", "target_keyword", "showcase"]
    })
}

/// Prompt gövdesi — bağlamın tamamı burada, model başka hiçbir şey bilmiyor.
pub fn build_prompt(ctx: &StorePageContext<'_>) -> String {
    let mut p = format!("Sayfa adı: {}\n", ctx.name);
    if !ctx.queries.is_empty() {
        p.push_str(&format!(
            "Bu sayfanın Google'da aldığı aramalar (gerçek veri): {}\n",
            ctx.queries.join(" · ")
        ));
    }
    if !ctx.products.is_empty() {
        p.push_str(&format!(
            "Bu sayfadaki ürünlerden örnekler: {}\n",
            ctx.products.join(" · ")
        ));
    }
    match ctx.existing {
        Some(m) if !m.trim().is_empty() => {
            p.push_str(&format!("Sayfada şu an duran metin (iyileştir):\n{}\n", m.trim()));
        }
        _ => p.push_str("Sayfada şu an tanıtım metni yok; sıfırdan yaz.\n"),
    }
    p
}

/// Bağlamda geçmeyen sayı var mı?
///
/// 🔴 Bu, halüsinasyon kalkanının **ölçülebilir** kısmı. Model "40 Gbps" ya da "5 yıl
/// garanti" gibi bir sayı uydurursa metin reddediliyor. Sessizce silmek yerine reddetmek
/// bilinçli: silinen sayı cümleyi bozar ve kullanıcı neyin değiştiğini bilmez.
///
/// ⚠️ Bunun yakalayamadığı şey: sayısız yanlış iddia. Kabul edilen sınır (modül başlığı).
pub fn verify_no_invented_numbers(text: &str, context: &str) -> Result<(), String> {
    let num = regex::Regex::new(r"\d+(?:[.,]\d+)?").unwrap();
    let kaynak: Vec<String> = num.find_iter(context).map(|m| m.as_str().to_string()).collect();
    for m in num.find_iter(text) {
        let s = m.as_str();
        // Tek haneli sayılar ("2 paragraf", "3 farklı") sayısal İDDİA değil, sayma sözü.
        // Ölçüt: iki haneden küçükse ve kaynakta yoksa da kabul — yoksa "iki" yerine "2"
        // yazan her cümle reddedilirdi.
        if s.len() < 2 {
            continue;
        }
        if !kaynak.iter().any(|k| k == s) {
            return Err(format!(
                "Üretilen metinde kaynakta olmayan bir sayı var: \"{s}\". \
                 Sayısal iddia doğrulanamadığı için metin kabul edilmedi."
            ));
        }
    }
    Ok(())
}

/// Uzunluk kurallarını sınar; ihlal sayısını döndürür (0 = kurallara uygun).
pub fn violations(g: &GeneratedStorePage) -> u32 {
    let mut n = 0;
    if !(TITLE_MIN..=TITLE_MAX).contains(&grapheme_len(g.page_title.trim())) {
        n += 1;
    }
    if !(DESC_MIN..=DESC_MAX).contains(&grapheme_len(g.meta_description.trim())) {
        n += 1;
    }
    if g.target_keyword.trim().is_empty() {
        n += 1;
    }
    // Hedef kelime başlıkta geçmeli — ürün tarafındaki kuralın aynısı.
    let kw = g.target_keyword.trim().to_lowercase();
    if !kw.is_empty() && !g.page_title.to_lowercase().contains(&kw) {
        n += 1;
    }
    if g.showcase.trim().chars().count() < 120 {
        n += 1;
    }
    n
}

async fn call_model(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    prompt: &str,
    kind: &str,
    chain: &ChainCtx<'_>,
) -> Result<GeneratedStorePage, (bool, String)> {
    let body = serde_json::json!({
        "system_instruction": { "parts": [{ "text": system_prompt(kind) }] },
        "contents": [{ "parts": [{ "text": prompt }] }],
        "generationConfig": {
            "responseMimeType": "application/json",
            "responseSchema": response_schema(),
            "temperature": 0.6
        }
    });
    let inner = post_generate(client, api_key, model, &body, chain).await?;
    serde_json::from_str::<GeneratedStorePage>(&inner)
        .map_err(|e| (false, format!("Üretilen JSON okunamadı: {e}")))
}

/// Mağaza sayfası için meta + tanıtım metni üretir.
///
/// ⚠️ Kural ihlali varsa **tek retry** (ürün hattındaki desenin aynısı), sonra ikisinin
/// iyisi seçiliyor. Uydurma sayı varsa retry değil **hata**: kural ihlali biçim sorunu,
/// uydurma sayı güven sorunu.
pub async fn generate_store_page(
    api_key: &str,
    ctx: &StorePageContext<'_>,
    chain: &ChainCtx<'_>,
) -> Result<Produced<GeneratedStorePage>, String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("Gemini API anahtarı ayarlı değil. Ayarlar'dan ekleyin.".to_string());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;

    let prompt = build_prompt(ctx);
    let mut last_err = String::from("Bilinmeyen hata");

    for model in &chain.models {
        match call_model(&client, key, model, &prompt, ctx.kind, chain).await {
            Ok(ilk) => {
                let best = if violations(&ilk) == 0 {
                    ilk
                } else {
                    let düzelt = format!(
                        "{prompt}\nÖnceki denemede uzunluk/kural ihlali vardı. \
                         Başlık {TITLE_MIN}-{TITLE_MAX}, açıklama {DESC_MIN}-{DESC_MAX} \
                         karakter olacak ve hedef kelime başlıkta geçecek."
                    );
                    match call_model(&client, key, model, &düzelt, ctx.kind, chain).await {
                        Ok(ikinci) if violations(&ikinci) <= violations(&ilk) => ikinci,
                        _ => ilk,
                    }
                };
                // 🔴 Uydurma sayı → hata. Zincirde sonraki modeli denemek anlamsız:
                // sorun modelin yeteneği değil, çıktının güvenilmezliği.
                let hepsi = format!(
                    "{} {} {}",
                    best.page_title, best.meta_description, best.showcase
                );
                verify_no_invented_numbers(&hepsi, &prompt)?;
                return Ok(Produced { value: best, model: model.clone() });
            }
            Err((try_next, msg)) => {
                last_err = msg;
                if !try_next {
                    return Err(last_err);
                }
            }
        }
    }
    Err(format!("Tüm modeller denendi, üretim başarısız. Son hata: {last_err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx<'a>(existing: Option<&'a str>) -> StorePageContext<'a> {
        StorePageContext {
            kind: "category",
            name: "Güvenlik Duvarı (Firewall)",
            queries: &[],
            products: &[],
            existing,
        }
    }

    #[test]
    fn prompt_baglami_tasiyor_ve_bos_alanlari_atliyor() {
        let sorgular = ["firewall cihazı".to_string(), "firewall markaları".to_string()];
        let urunler = ["FortiGate 100F".to_string()];
        let c = StorePageContext {
            kind: "category",
            name: "Güvenlik Duvarı (Firewall)",
            queries: &sorgular,
            products: &urunler,
            existing: None,
        };
        let p = build_prompt(&c);
        assert!(p.contains("Güvenlik Duvarı"));
        assert!(p.contains("firewall cihazı"), "gerçek sorgular bağlama girmeli");
        assert!(p.contains("FortiGate 100F"), "gerçek ürünler bağlama girmeli");
        assert!(p.contains("sıfırdan yaz"), "metin yoksa açıkça söylenmeli");

        // Mevcut metin varsa "iyileştir" deniyor — sıfırdan yazmak var olanı çöpe atardı.
        let p2 = build_prompt(&ctx(Some("Kurumsal ağ güvenliği çözümleri.")));
        assert!(p2.contains("iyileştir"));
        assert!(p2.contains("Kurumsal ağ güvenliği"));
    }

    /// 🔴 Halüsinasyon kalkanının ölçülebilir kısmı: kaynakta olmayan sayı reddedilir.
    #[test]
    fn uydurma_sayi_reddediliyor() {
        let kaynak = "Sayfa adı: Güvenlik Duvarı\nÜrünler: FortiGate 100F";
        // 100F kaynakta var → kabul
        assert!(verify_no_invented_numbers("FortiGate 100F modelleri", kaynak).is_ok());
        // 40 Gbps kaynakta YOK → red
        let e = verify_no_invented_numbers("40 Gbps hızında çalışır", kaynak).unwrap_err();
        assert!(e.contains("40"), "hangi sayı olduğu söylenmeli: {e}");
        // Tek hane sayma sözü sayılıyor, iddia değil
        assert!(verify_no_invented_numbers("3 farklı seçenek", kaynak).is_ok());
    }

    #[test]
    fn uzunluk_kurallari_paylasilan_sabitlerden() {
        let mut g = GeneratedStorePage {
            page_title: "Güvenlik Duvarı (Firewall) Modelleri ve Fiyatları".into(),
            meta_description:
                "Kurumsal ağınız için güvenlik duvarı modellerini inceleyin; hangi cihazın \
                 hangi ölçekteki işletmeye uygun olduğunu anlatıyoruz."
                    .into(),
            target_keyword: "güvenlik duvarı".into(),
            showcase: "x".repeat(300),
        };
        assert_eq!(violations(&g), 0, "kurallara uygun çıktı ihlal üretmemeli");

        // Başlıkta hedef kelime yok → ihlal
        g.target_keyword = "yönlendirici".into();
        assert_eq!(violations(&g), 1);

        // Çok kısa tanıtım → ihlal
        g.target_keyword = "güvenlik duvarı".into();
        g.showcase = "kısa".into();
        assert_eq!(violations(&g), 1);

        // Sınırların paylaşılan sabitlerden geldiği: başlığı taşır
        g.showcase = "x".repeat(300);
        g.page_title = "x".repeat(TITLE_MAX + 5);
        assert!(violations(&g) >= 1);
    }

    /// Sayısal iddia yasağı prompt'ta AÇIKÇA yazılı olmalı — kalkanın ilk katmanı bu.
    #[test]
    fn prompt_sayi_yasagini_soyluyor() {
        let p = system_prompt("category");
        assert!(p.contains("SAYI UYDURMA"));
        assert!(p.contains(&TITLE_MAX.to_string()), "uzunluk kuralı prompt'a girmeli");
        assert!(system_prompt("brand").contains("marka sayfası"));
        assert!(system_prompt("blog").contains("bilgi içeriği"));
    }
}
