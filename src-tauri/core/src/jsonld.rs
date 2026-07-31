//! # Schema.org `Product` JSON-LD çıktısı
//!
//! ## Ölçüm önce yapıldı: IdeaSoft ZATEN yapılandırılmış veri basıyor (2026-08-01)
//!
//! Canlı ürün sayfası (`kurumsalit.com/urun/dell-pro-e2725hm…`) ham HTML'i incelendi:
//!
//! | | durum |
//! |---|---|
//! | `application/ld+json` bloğu | **0 tane** |
//! | microdata `schema.org/Product` | **var** — `display:none` bir div içinde |
//! | microdata alanları | name · description · sku · brand · category · image · url |
//! | microdata `Offer` | priceCurrency · price · **availability** — canlı ve doğru |
//!
//! Bu ölçüm özelliğin kapsamını belirledi: **temel alanları tekrar etmenin değeri yok**,
//! mağaza altyapısı onları zaten doğru basıyor. Bizim eklediğimiz tek gerçek şey
//! **`additionalProperty`** — yani üretilen teknik özellik tablosu. IdeaSoft'un mikroverisinde
//! ürün özelliği YOK; teknik tablo mağazada yalnızca HTML olarak duruyor, yani arama motoru
//! ve yapay zekâ tarafında yapılandırılmış veri olarak okunmuyor. Kazanç burada.
//!
//! ## ⚠️ Bilinçli olarak DIŞARIDA bırakılanlar
//!
//! - **`offers` / `price` / `availability`** — feed'de fiyat alanı **yok** (`FeedProduct`'a
//!   bakın), stok da anlık değişiyor. Üretim anında yakalanmış bir fiyat sayfadaki canlı
//!   mikroveriyle çelişir; Google bu uyuşmazlığı **hata olarak** raporlar. Yanlış fiyat,
//!   fiyat olmamasından kötüdür. Canlı veriyi zaten mağaza altyapısı basıyor.
//! - **`aggregateRating` / `review`** — elimizde puan verisi yok. Uydurmak hem Google'ın
//!   yapılandırılmış veri politikasının açık ihlali hem de kullanıcının *"halüsinasyon riskini
//!   göze alamam"* kısıtının tam karşıtı.
//! - **`gtin` / `mpn`** — feed'de yok. Boş string basmak "alan var ama değeri yok" demek
//!   olurdu; Google boş değeri hata sayıyor. Alan yoksa hiç yazılmıyor.
//!
//! ## Kural: boş alan hiç yazılmaz
//!
//! `"description": ""` gibi bir çıktı geçerli JSON ama geçersiz yapılandırılmış veri.
//! Bütün alanlar `push_str_field` üzerinden geçiyor: boşsa anahtar hiç oluşmuyor.

use crate::gemini::TechGroup;
use serde_json::{json, Map, Value};

/// JSON-LD üretimi için gereken ürün verisi. Hepsi elimizde OLAN alanlar —
/// modül başlığındaki "dışarıda bırakılanlar" listesi bilinçli olarak burada da yok.
#[derive(Debug, Default, Clone)]
pub struct ProductFacts {
    pub name: String,
    pub sku: String,
    pub brand: String,
    pub category: String,
    /// Meta açıklama (taslak varsa taslak). Sayfadaki açıklamayla aynı kaynak olmalı.
    pub description: String,
    pub url: String,
    /// Galeri: ana görsel + picture2..4.
    pub images: Vec<String>,
}

/// Protokolsüz (`//host/…`) adresleri mutlak hâle getirir.
///
/// ⚠️ Bu mağazanın feed'inde 254/254 görsel `https://` ile başlıyor — yani burada gerek YOK.
/// Yine de duruyor: uygulama **her IdeaSoft mağazası** için, ve canlı sayfanın HTML'inde
/// görseller `//www…` biçiminde basılıyor. Schema.org mutlak URL istiyor; protokolsüz adres
/// sessizce geçersiz veri üretirdi.
fn absolute(u: &str) -> String {
    let u = u.trim();
    if let Some(rest) = u.strip_prefix("//") {
        format!("https://{rest}")
    } else {
        u.to_string()
    }
}

/// Boş olmayan metin alanını ekler; boşsa anahtarı hiç oluşturmaz.
fn push_str_field(map: &mut Map<String, Value>, key: &str, value: &str) {
    let v = value.trim();
    if !v.is_empty() {
        map.insert(key.to_string(), Value::String(v.to_string()));
    }
}

/// Teknik tablo satırlarını `PropertyValue` dizisine çevirir.
///
/// Gruplar düzleştiriliyor: schema.org'da özellik grubu diye bir kavram yok, `additionalProperty`
/// düz bir liste. Grup adı satır adına eklenmiyor — "Bağlantı → HDMI" gibi birleşik etiketler
/// arama motoru tarafında okunabilir bir özellik adı olmaz.
///
/// ⚠️ Üst düzey alanı **birebir tekrar eden** satırlar atılıyor. Gerçek çıktıda ölçüldü:
/// teknik tablonun "Ürün Ailesi" grubu ad/marka/kategoriyi tekrar ediyordu ve bunlar zaten
/// `name`/`brand`/`category` olarak yazılıyor. Aynı bilgiyi iki kez söylemek çıktıyı
/// şişirmekten başka bir şey yapmıyor. Ölçüt **tam eşitlik**: benzer değerler korunuyor,
/// yanlışlıkla gerçek bir özellik atılmasın.
fn properties(specs: &[TechGroup], facts: &ProductFacts) -> Vec<Value> {
    let ust: Vec<String> = [&facts.name, &facts.brand, &facts.category]
        .iter()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    specs
        .iter()
        .flat_map(|g| g.rows.iter())
        .filter(|r| !r.label.trim().is_empty() && !r.value.trim().is_empty())
        .filter(|r| !ust.contains(&r.value.trim().to_lowercase()))
        .map(|r| json!({
            "@type": "PropertyValue",
            "name": r.label.trim(),
            "value": r.value.trim(),
        }))
        .collect()
}

/// `Product` düğümünü kurar. Elde yeterli veri yoksa `None` döner.
///
/// Asgari koşul **ad + url**: adsız bir ürün düğümü hiçbir şey ifade etmiyor, url ise düğümü
/// sayfadaki varlıkla eşleştiren tek bağ (aynı ürünün iki ayrı kaydı sanılmasın diye).
pub fn build(facts: &ProductFacts, specs: &[TechGroup]) -> Option<Value> {
    if facts.name.trim().is_empty() || facts.url.trim().is_empty() {
        return None;
    }
    let mut m = Map::new();
    m.insert("@context".into(), Value::String("https://schema.org".into()));
    m.insert("@type".into(), Value::String("Product".into()));
    push_str_field(&mut m, "name", &facts.name);
    push_str_field(&mut m, "sku", &facts.sku);
    push_str_field(&mut m, "description", &facts.description);
    push_str_field(&mut m, "category", &facts.category);
    push_str_field(&mut m, "url", &facts.url);

    if !facts.brand.trim().is_empty() {
        m.insert("brand".into(), json!({ "@type": "Brand", "name": facts.brand.trim() }));
    }

    let images: Vec<Value> = facts
        .images
        .iter()
        .map(|s| absolute(s))
        .filter(|s| !s.is_empty())
        .map(Value::String)
        .collect();
    if !images.is_empty() {
        m.insert("image".into(), Value::Array(images));
    }

    let props = properties(specs, facts);
    if !props.is_empty() {
        m.insert("additionalProperty".into(), Value::Array(props));
    }

    Some(Value::Object(m))
}

/// Sayfaya yapıştırılacak hâli: `<script type="application/ld+json">…</script>`.
///
/// ⚠️ `</script>` kaçışı zorunlu: teknik özellik değerinde bu dizi geçerse tarayıcı script'i
/// erkenden kapatır ve **sayfanın geri kalanı bozulur**. JSON kaçışı (`<\/script>`) hem geçerli
/// JSON hem de tarayıcı için güvenli.
pub fn render_script(node: &Value) -> String {
    let body = serde_json::to_string_pretty(node)
        .unwrap_or_default()
        .replace("</", "<\\/");
    format!("<script type=\"application/ld+json\">\n{body}\n</script>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gemini::TechRow;

    fn facts() -> ProductFacts {
        ProductFacts {
            name: "Dell Pro E2725HM 27'' Monitör".into(),
            sku: "MON.DEL.E2725HM".into(),
            brand: "Dell".into(),
            category: "Monitör".into(),
            description: "27 inç, 100 Hz, 5 ms IPS ofis monitörü.".into(),
            url: "https://ornek.com/urun/dell-pro-e2725hm".into(),
            images: vec!["https://ornek.com/a.jpg".into(), "https://ornek.com/b.jpg".into()],
        }
    }

    fn specs() -> Vec<TechGroup> {
        vec![TechGroup {
            group: "Ekran".into(),
            rows: vec![
                TechRow { label: "Panel".into(), value: "IPS".into() },
                TechRow { label: "Yenileme Hızı".into(), value: "100 Hz".into() },
            ],
        }]
    }

    /// Gerçek veritabanı kopyası üzerinde çıktı ölçümü — kaç ürün JSON-LD üretebiliyor,
    /// kaç özellik taşıyor, çıktı ne kadar büyük.
    ///
    /// `SEO_DB_COPY=/tmp/kopya.db cargo test jsonld_real -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn jsonld_real() {
        let db = std::env::var("SEO_DB_COPY").expect("SEO_DB_COPY yok");
        let conn = rusqlite::Connection::open(&db).unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT p.name, p.sku, p.brand, COALESCE(p.category, p.main_category),
                        COALESCE(s.draft_descriptions, p.descriptions, ''), COALESCE(p.url,''),
                        COALESCE(p.img_url,''), COALESCE(p.picture2,''), COALESCE(p.picture3,''),
                        COALESCE(p.picture4,''), COALESCE(s.tech_specs_json,'')
                 FROM products p LEFT JOIN seo_status s ON s.sku = p.sku",
            )
            .unwrap();
        let rows: Vec<(ProductFacts, Vec<TechGroup>)> = stmt
            .query_map([], |r| {
                let g = |i: usize| -> rusqlite::Result<String> {
                    Ok(r.get::<_, Option<String>>(i)?.unwrap_or_default())
                };
                let specs: Vec<TechGroup> = serde_json::from_str(&g(10)?).unwrap_or_default();
                Ok((
                    ProductFacts {
                        name: g(0)?,
                        sku: g(1)?,
                        brand: g(2)?,
                        category: g(3)?,
                        description: g(4)?,
                        url: g(5)?,
                        images: vec![g(6)?, g(7)?, g(8)?, g(9)?]
                            .into_iter()
                            .filter(|s| !s.is_empty())
                            .collect(),
                    },
                    specs,
                ))
            })
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        let (mut uretilen, mut ozellikli, mut toplam_ozellik, mut en_buyuk) = (0, 0, 0usize, 0usize);
        let mut ornek = String::new();
        for (f, specs) in &rows {
            let Some(node) = build(f, specs) else { continue };
            uretilen += 1;
            let script = render_script(&node);
            en_buyuk = en_buyuk.max(script.len());
            if let Some(a) = node.get("additionalProperty").and_then(|v| v.as_array()) {
                ozellikli += 1;
                toplam_ozellik += a.len();
                if ornek.is_empty() && a.len() > 20 {
                    ornek = script;
                }
            }
            // Ölçümün asıl amacı: gerçek veride hiçbir alan boş string olarak çıkmasın.
            for (k, v) in node.as_object().unwrap() {
                if let Some(s) = v.as_str() {
                    assert!(!s.trim().is_empty(), "boş alan gerçek veride çıktı: {k}");
                }
            }
        }
        println!(
            "{} üründen {uretilen} tanesi JSON-LD üretti · {ozellikli} tanesinde özellik var \
             ({toplam_ozellik} satır) · en büyük çıktı {en_buyuk} bayt",
            rows.len()
        );
        std::fs::write("/tmp/ornek-jsonld.html", &ornek).ok();
        println!("örnek çıktı: /tmp/ornek-jsonld.html ({} bayt)", ornek.len());
        assert!(uretilen > 0, "hiçbir ürün için JSON-LD üretilemedi");
    }

    #[test]
    fn temel_alanlar_ve_ozellikler_yazilir() {
        let n = build(&facts(), &specs()).expect("düğüm kurulmadı");
        assert_eq!(n["@type"], "Product");
        assert_eq!(n["sku"], "MON.DEL.E2725HM");
        assert_eq!(n["brand"]["name"], "Dell");
        assert_eq!(n["image"].as_array().unwrap().len(), 2);
        let props = n["additionalProperty"].as_array().unwrap();
        assert_eq!(props.len(), 2);
        assert_eq!(props[0]["@type"], "PropertyValue");
        assert_eq!(props[0]["name"], "Panel");
        assert_eq!(props[0]["value"], "IPS");
    }

    /// 🔴 Ölçümün dayattığı kural: fiyat ve stok ASLA yazılmaz. Feed'de fiyat yok, stok anlık
    /// değişiyor; yazılsaydı sayfadaki canlı mikroveriyle çelişir ve Google hata raporlardı.
    #[test]
    fn fiyat_stok_ve_puan_asla_yazilmaz() {
        let n = build(&facts(), &specs()).unwrap();
        let s = n.to_string();
        for yasak in ["offers", "price", "availability", "aggregateRating", "review", "gtin"] {
            assert!(!s.contains(yasak), "yasak alan çıktıya girdi: {yasak}");
        }
    }

    /// ⚠️ Boş string geçerli JSON ama geçersiz yapılandırılmış veri — Google hata sayıyor.
    #[test]
    fn bos_alan_anahtar_olusturmaz() {
        let mut f = facts();
        f.brand = "".into();
        f.description = "   ".into();
        f.category = "".into();
        f.images.clear();
        let n = build(&f, &[]).unwrap();
        let m = n.as_object().unwrap();
        for k in ["brand", "description", "category", "image", "additionalProperty"] {
            assert!(!m.contains_key(k), "boş alan anahtarı yazıldı: {k}");
        }
        assert_eq!(m["name"], "Dell Pro E2725HM 27'' Monitör");
    }

    #[test]
    fn ad_veya_url_yoksa_dugum_kurulmaz() {
        let mut f = facts();
        f.url = "".into();
        assert!(build(&f, &specs()).is_none(), "url'siz düğüm üretildi");
        let mut f2 = facts();
        f2.name = "  ".into();
        assert!(build(&f2, &specs()).is_none(), "adsız düğüm üretildi");
    }

    #[test]
    fn protokolsuz_gorsel_mutlak_hale_gelir() {
        let mut f = facts();
        f.images = vec!["//cdn.ornek.com/x.jpg".into()];
        let n = build(&f, &[]).unwrap();
        assert_eq!(n["image"][0], "https://cdn.ornek.com/x.jpg");
    }

    /// 🔬 Gerçek çıktıda görüldü: teknik tablonun ilk üç satırı ad/marka/kategoriyi
    /// tekrar ediyordu — üst düzeyde zaten yazılan bilgi.
    #[test]
    fn ust_duzey_alani_tekrar_eden_satirlar_atlanir() {
        let f = facts();
        let s = vec![TechGroup {
            group: "Ürün Ailesi".into(),
            rows: vec![
                TechRow { label: "Ürün Adı".into(), value: f.name.clone() },
                TechRow { label: "Marka".into(), value: "dell".into() }, // büyük/küçük harf farkı
                TechRow { label: "Kategori".into(), value: f.category.clone() },
                TechRow { label: "Panel".into(), value: "IPS".into() },
                // Benzer ama aynı değil → KORUNMALI, ölçüt tam eşitlik.
                TechRow { label: "Seri".into(), value: "Dell Pro".into() },
            ],
        }];
        let n = build(&f, &s).unwrap();
        let props = n["additionalProperty"].as_array().unwrap();
        let adlar: Vec<&str> = props.iter().map(|p| p["name"].as_str().unwrap()).collect();
        assert_eq!(adlar, vec!["Panel", "Seri"], "tekrar ayıklama yanlış çalıştı");
    }

    #[test]
    fn bos_ozellik_satirlari_atlanir() {
        let s = vec![TechGroup {
            group: "Genel".into(),
            rows: vec![
                TechRow { label: "Panel".into(), value: "IPS".into() },
                TechRow { label: "".into(), value: "değer".into() },
                TechRow { label: "Etiket".into(), value: "  ".into() },
            ],
        }];
        let n = build(&facts(), &s).unwrap();
        assert_eq!(n["additionalProperty"].as_array().unwrap().len(), 1);
    }

    /// 🔴 Kaçış olmasaydı özellik değerindeki `</script>` script'i erkenden kapatır ve
    /// sayfanın geri kalanı bozulurdu — sessiz değil, görünür bir hasar.
    #[test]
    fn script_kapanisi_kacirilir() {
        let s = vec![TechGroup {
            group: "Genel".into(),
            rows: vec![TechRow {
                label: "Not".into(),
                value: "</script><img src=x>".into(),
            }],
        }];
        let out = render_script(&build(&facts(), &s).unwrap());
        assert_eq!(out.matches("</script>").count(), 1, "gövdede kapanış etiketi kaldı");
        assert!(out.contains("<\\/script>"), "kaçış uygulanmadı");
        assert!(out.starts_with("<script type=\"application/ld+json\">"));
    }
}
