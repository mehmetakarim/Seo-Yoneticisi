//! Teknik özellik tablosu — halüsinasyon-sıfır tasarım.
//!
//! Model yalnızca YAPILANDIRIR; `verify_traceable` kaynakta bulunmayan değeri atar.

use super::*;

/// Kanonik grup sırası — ürünler arası tutarlılık (karşılaştırma + ileride schema üretimi için).
pub const TECH_GROUPS: &[&str] = &[
    "Ürün Ailesi",
    "Performans",
    "Ekran & Görüntü",
    "Bağlantı & Ağ",
    "Kamera & Multimedya",
    "Fiziksel & Diğer",
    "Uyumluluk & Güvenlik",
    "Kutu İçeriği",
];

/// Tablo değil, liste olarak render edilen grup (anahtar-değer değil).
const LIST_GROUP: &str = "Kutu İçeriği";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TechRow {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TechGroup {
    pub group: String,
    pub rows: Vec<TechRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TechSpecsResult {
    pub groups: Vec<TechGroup>,
    /// Kaynakta doğrulanamadığı için atılan satırların etiketleri (kullanıcıya raporlanır).
    pub dropped: Vec<String>,
}

fn tech_system_prompt() -> &'static str {
    "Sen bir teknik veri yapılandırıcısın. Sana bir ürünün HAM teknik özellik metni verilir. Görevin \
     bu metni gruplanmış anahtar-değer satırlarına DÖNÜŞTÜRMEKTİR. KESİN KURALLAR: \
     1) YALNIZCA verilen metinde geçen bilgileri kullan. Metinde OLMAYAN hiçbir satır ÜRETME, \
     hiçbir değeri TAHMİN ETME, dünya bilginle TAMAMLAMA. Emin değilsen o satırı hiç yazma. \
     2) Değerleri metindeki hâliyle koru (sayılar ve birimler aynen kalsın). Sadece etiketleri \
     tutarlı ve Türkçe yaz (ör. 'CPU' → 'İşlemci', 'Memory' → 'Bellek (RAM)'). \
     3) Her satırı şu gruplardan BİRİNE ata: 'Ürün Ailesi', 'Performans', 'Ekran & Görüntü', \
     'Bağlantı & Ağ', 'Kamera & Multimedya', 'Fiziksel & Diğer', 'Uyumluluk & Güvenlik', 'Kutu İçeriği'. \
     4) Kutu içeriği maddelerinde label boş bırakılabilir, value maddenin kendisi olsun. \
     5) Pazarlama cümlesi yazma; tablo verisi üret. Yalnızca istenen JSON'u döndür."
}

async fn call_specs_model(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    prompt: &str,
) -> Result<Vec<TechGroup>, (bool, String)> {
    let url = format!("{API_BASE}/{model}:generateContent");
    let body = serde_json::json!({
        "system_instruction": { "parts": [{ "text": tech_system_prompt() }] },
        "contents": [{ "parts": [{ "text": prompt }] }],
        "generationConfig": {
            "responseMimeType": "application/json",
            "responseSchema": {
                "type": "ARRAY",
                "items": {
                    "type": "OBJECT",
                    "properties": {
                        "group": { "type": "STRING" },
                        "rows": {
                            "type": "ARRAY",
                            "items": {
                                "type": "OBJECT",
                                "properties": {
                                    "label": { "type": "STRING" },
                                    "value": { "type": "STRING" }
                                },
                                "required": ["label", "value"]
                            }
                        }
                    },
                    "required": ["group", "rows"]
                }
            },
            "temperature": 0.2
        }
    });
    let resp = client
        .post(&url)
        .query(&[("key", api_key)])
        .json(&body)
        .send()
        .await
        .map_err(|e| (false, format!("İstek gönderilemedi: {e}")))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(classify_error(status.as_u16(), &text, model));
    }
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| (false, format!("Yanıt çözümlenemedi: {e}")))?;
    let inner = v["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| (false, format!("Beklenmeyen yanıt biçimi: {}", short(&text))))?;
    serde_json::from_str::<Vec<TechGroup>>(inner)
        .map_err(|e| (false, format!("Üretilen tablo okunamadı: {e}")))
}

/// **Halüsinasyon kalkanı.** Değerdeki her sayı dizisi kaynak metinde birebir geçmelidir; geçmiyorsa
/// satır atılır. (Sayısız değerler — "IPS", "Var" gibi — doğrulanamaz, prompt kısıtına güvenilir.)
fn verify_traceable(groups: Vec<TechGroup>, source: &str) -> TechSpecsResult {
    let num_re = regex::Regex::new(r"\d+(?:[.,]\d+)?").unwrap();
    let src = source.to_lowercase();
    // Ondalık ayırıcı farkını tolere et (4.90 ↔ 4,90)
    let src_alt = src.replace(',', ".");
    let mut dropped = Vec::new();
    let mut kept = Vec::new();

    for g in groups {
        let mut rows = Vec::new();
        for r in g.rows {
            let label = r.label.trim().to_string();
            let value = r.value.trim().to_string();
            if value.is_empty() {
                continue;
            }
            let ok = num_re.find_iter(&value).all(|m| {
                let n = m.as_str();
                let n_alt = n.replace(',', ".");
                src.contains(&n.to_lowercase()) || src_alt.contains(&n_alt.to_lowercase())
            });
            if ok {
                rows.push(TechRow { label, value });
            } else {
                dropped.push(if label.is_empty() { value } else { label });
            }
        }
        if !rows.is_empty() {
            kept.push(TechGroup { group: g.group.trim().to_string(), rows });
        }
    }

    // Kanonik sıraya diz; listede olmayan gruplar sona (girdiği sırayla).
    kept.sort_by_key(|g| {
        TECH_GROUPS
            .iter()
            .position(|c| c.eq_ignore_ascii_case(&g.group))
            .unwrap_or(usize::MAX)
    });
    TechSpecsResult { groups: kept, dropped }
}

/// Ham teknik metni gruplu anahtar-değer yapısına çevirir ve kaynağa karşı doğrular.
pub async fn structure_tech_specs(
    api_key: &str,
    ctx: &ProductContext<'_>,
    source_text: &str,
) -> Result<Produced<TechSpecsResult>, String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("Gemini API anahtarı ayarlı değil. Ayarlar'dan ekleyin.".to_string());
    }
    let src = source_text.trim();
    if src.is_empty() {
        return Err("Önce üreticinin teknik özellik metnini yapıştırın.".to_string());
    }

    let prompt = format!(
        "Ürün adı: {}\nMarka: {}\nKategori: {}\n\nHAM TEKNİK METİN (yalnızca buradaki bilgileri kullan):\n{}",
        ctx.name,
        ctx.brand.unwrap_or("-"),
        ctx.category.unwrap_or("-"),
        src,
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;

    let mut last_err = String::from("Bilinmeyen hata");
    for model in MODEL_CHAIN.iter() {
        match call_specs_model(&client, key, model, &prompt).await {
            Ok(groups) => {
                let result = verify_traceable(groups, src);
                if result.groups.is_empty() {
                    return Err(
                        "Metinden doğrulanabilir teknik özellik çıkarılamadı. Daha ayrıntılı bir \
                         teknik metin yapıştırmayı deneyin."
                            .to_string(),
                    );
                }
                return Ok(Produced { value: result, model });
            }
            Err((try_next, msg)) => {
                last_err = msg;
                if !try_next {
                    return Err(last_err);
                }
            }
        }
    }
    Err(format!("Tüm modeller denendi, tablo üretilemedi. Son hata: {last_err}"))
}

/// Yapılandırılmış spec'lerden **semantik** HTML üretir (model devrede değil, tamamen deterministik).
/// Grup başlığı `<caption>`, satır etiketi `<th scope="row">`, genişlik `<colgroup>` ile —
/// `<thead>`+boş `<th>` ve Bootstrap `col-*` grid sınıfı KULLANILMAZ. "Kutu İçeriği" liste olur.
pub fn assemble_tech_html(groups: &[TechGroup]) -> String {
    let mut out = String::with_capacity(1024);
    for g in groups {
        if g.rows.is_empty() {
            continue;
        }
        if g.group.eq_ignore_ascii_case(LIST_GROUP) {
            out.push_str(&format!("<h3>{}</h3><ul>", esc(&g.group)));
            for r in &g.rows {
                let item = if r.label.trim().is_empty() {
                    r.value.clone()
                } else {
                    format!("{}: {}", r.label, r.value)
                };
                out.push_str(&format!("<li>{}</li>", esc(&item)));
            }
            out.push_str("</ul>");
            continue;
        }
        out.push_str(&format!(
            r#"<table class="table teknik-tablo"><caption>{}</caption><colgroup><col class="tt-etiket" /><col class="tt-deger" /></colgroup><tbody>"#,
            esc(&g.group)
        ));
        for r in &g.rows {
            out.push_str(&format!(
                r#"<tr><th scope="row">{}</th><td>{}</td></tr>"#,
                esc(&r.label),
                esc(&r.value)
            ));
        }
        out.push_str("</tbody></table>");
    }
    out
}

#[cfg(test)]
mod tests {
    //! Teknik tablo testleri.
    use super::*;

    fn tg(group: &str, rows: &[(&str, &str)]) -> TechGroup {
        TechGroup {
            group: group.into(),
            rows: rows
                .iter()
                .map(|(l, v)| TechRow { label: (*l).into(), value: (*v).into() })
                .collect(),
        }
    }

    #[test]
    fn verify_traceable_drops_invented_numbers() {
        let source = "İşlemci: Intel Core i7-13620H, 24 MB cache. Bellek: 16 GB DDR5 5200 MHz. Panel: IPS";
        let groups = vec![tg(
            "Performans",
            &[
                ("İşlemci", "Intel Core i7-13620H (24 MB cache)"), // kaynakta var
                ("Bellek (RAM)", "16 GB DDR5 5200 MHz"),           // kaynakta var
                ("Parlaklık", "300 nit"),                          // UYDURMA → atılmalı
                ("Panel Tipi", "IPS"),                             // sayısız → geçer
            ],
        )];
        let r = verify_traceable(groups, source);
        let labels: Vec<&str> = r.groups[0].rows.iter().map(|x| x.label.as_str()).collect();
        assert_eq!(labels, vec!["İşlemci", "Bellek (RAM)", "Panel Tipi"]);
        assert_eq!(r.dropped, vec!["Parlaklık"]);
    }

    #[test]
    fn verify_traceable_tolerates_decimal_separator_and_orders_groups() {
        let source = "Ağırlık 7,5 kg. Kategori: All-in-One";
        let groups = vec![
            tg("Fiziksel & Diğer", &[("Ağırlık", "7.5 kg")]), // 7,5 ↔ 7.5
            tg("Ürün Ailesi", &[("Kategori", "All-in-One")]),
        ];
        let r = verify_traceable(groups, source);
        // Kanonik sıra: Ürün Ailesi, Fiziksel & Diğer
        assert_eq!(r.groups[0].group, "Ürün Ailesi");
        assert_eq!(r.groups[1].group, "Fiziksel & Diğer");
        assert!(r.dropped.is_empty());
    }

    #[test]
    fn verify_traceable_drops_empty_group_entirely() {
        let groups = vec![tg("Performans", &[("Parlaklık", "999 nit")])];
        let r = verify_traceable(groups, "hiç sayı yok burada");
        assert!(r.groups.is_empty());
        assert_eq!(r.dropped.len(), 1);
    }

    #[test]
    fn assemble_tech_html_is_semantic() {
        let groups = vec![
            tg("Performans", &[("İşlemci", "Intel® Core™ i7-13620H"), ("Bellek (RAM)", "16 GB")]),
            tg("Kutu İçeriği", &[("", "Güç adaptörü"), ("", "Kullanım kılavuzu")]),
        ];
        let html = assemble_tech_html(&groups);
        // Semantik gereklilikler
        assert!(html.contains("<caption>Performans</caption>"));
        assert!(html.contains(r#"<th scope="row">İşlemci</th>"#));
        assert!(html.contains("<colgroup>"));
        // Yanlış desenler OLMAMALI
        assert!(!html.contains("<thead"));
        assert!(!html.contains("col-4"));
        assert!(!html.contains("col-8"));
        // Kutu İçeriği tablo değil liste
        assert!(html.contains("<h3>Kutu İçeriği</h3><ul>"));
        assert!(html.contains("<li>Güç adaptörü</li>"));
        // Kutu içeriği için tablo üretilmemeli → yalnızca 1 tablo
        assert_eq!(html.matches("<table").count(), 1);
    }

    #[test]
    fn assemble_tech_html_escapes_and_skips_empty() {
        let groups = vec![
            tg("Uyumluluk & Güvenlik", &[("Güvenlik", "TPM 2.0 & <Kensington>")]),
            TechGroup { group: "Performans".into(), rows: vec![] }, // boş grup atlanır
        ];
        let html = assemble_tech_html(&groups);
        assert!(html.contains("Uyumluluk &amp; Güvenlik"));
        assert!(html.contains("TPM 2.0 &amp; &lt;Kensington&gt;"));
        assert_eq!(html.matches("<table").count(), 1);
    }

    /// **Halüsinasyon kalkanı canlı testi.** Modelin iyi bildiği bir ürüne KASITLI OLARAK eksik bir
    /// teknik metin verilir (parlaklık/ağırlık/renk gamı yok). Çıktıdaki hiçbir sayı kaynakta
    /// olmadan görünmemelidir.
    /// `GEMINI_API_KEY=... cargo test tech_specs_real -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn tech_specs_real() {
        let key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY ayarlı değil");
        let ctx = ProductContext {
            name: "Lenovo ThinkCentre Neo 50a Gen 5 12SCA078TR i7-13620H 16G 512G DOS 27''",
            brand: Some("Lenovo"),
            category: Some("All In One Bilgisayar"),
            main_category: Some("Bilgisayar"),
            target_keyword: None,
            insights: None,
        };
        // Bilinçli olarak EKSİK: parlaklık, renk gamı, ağırlık, boyut, hoparlör gücü yok.
        let source = "Processor: Intel Core i7-13620H (10 cores, 16 threads, up to 4.90 GHz)\n\
                      Memory: 16 GB DDR5 5200MHz\n\
                      Storage: 512 GB PCIe 4.0 NVMe M.2 SSD\n\
                      Display: 27 inch, 1920x1080, IPS\n\
                      Operating System: FreeDOS\n\
                      Ethernet: RJ45 Gigabit\n\
                      In the box: keyboard, mouse, power adapter";
        let produced = structure_tech_specs(&key, &ctx, source).await.expect("yapılandırma başarısız");
        println!("model: {}", produced.model);
        let res = produced.value;
        println!("--- GRUPLAR ---");
        for g in &res.groups {
            println!("[{}]", g.group);
            for r in &g.rows {
                println!("   {} = {}", r.label, r.value);
            }
        }
        println!("--- ATILAN: {:?}", res.dropped);

        // GARANTİ: çıktıdaki her sayı kaynakta geçmeli
        let num_re = regex::Regex::new(r"\d+(?:[.,]\d+)?").unwrap();
        let src = source.to_lowercase();
        let src_alt = src.replace(',', ".");
        for g in &res.groups {
            for r in &g.rows {
                for m in num_re.find_iter(&r.value) {
                    let n = m.as_str().to_lowercase();
                    let n_alt = n.replace(',', ".");
                    assert!(
                        src.contains(&n) || src_alt.contains(&n_alt),
                        "UYDURMA SAYI sızdı: '{}' = '{}' (sayı: {})",
                        r.label,
                        r.value,
                        n
                    );
                }
            }
        }
        assert!(!res.groups.is_empty());
        println!("✅ Hiçbir uydurma sayı çıktıya sızmadı.");
    }
}
