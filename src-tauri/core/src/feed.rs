use serde::Deserialize;

/// Feed'deki tek bir <product> kaydı. Alanların çoğu CDATA gelir;
/// quick-xml bunları düz metin olarak çözer, biz de hepsini trim ederiz
/// (özellikle quantityStatus "[ var ]" baş/son boşluklu gelir).
#[derive(Debug, Deserialize)]
pub struct FeedProduct {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub sku: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default, rename = "productBrand")]
    pub product_brand: Option<String>,
    #[serde(default, rename = "searchKeywords")]
    pub search_keywords: Option<String>,
    #[serde(default, rename = "mainCategory")]
    pub main_category: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    /// Dolar **alış** (maliyet) fiyatı. 🔴 Müşteriye giden hiçbir çıktıda görünmez —
    /// bkz. `quote_html::QuoteOut` (yapıda maliyet alanı yok).
    #[serde(default, rename = "buyingPrice")]
    pub buying_price: Option<String>,
    /// Dolar **satış** fiyatı — teklif satırının varsayılan birim fiyatı.
    #[serde(default)]
    pub price1: Option<String>,
    /// KDV oranı (%). Ölçüldü (2026-08-10): 282 üründe 276'sı 20, 6'sı 10 — katalogda
    /// **iki oran** var, o yüzden tek bir ayara indirgenmiyor.
    #[serde(default)]
    pub tax: Option<String>,
    /// 🔴 Ürünün fiyat para birimi (`USD` · `EUR` · `TL`). Ölçüldü (2026-08-10):
    /// **USD 273 · EUR 8 · TL 1** — "katalog dolar" varsayımı yanlıştı.
    #[serde(default, rename = "currencyAbbr")]
    pub currency_abbr: Option<String>,
    /// Mağazanın **TL** satış fiyatı, KDV **dahil**. Her ürün için mağazanın kendi çevrimini
    /// içeriyor; TL teklifte kur sormaya gerek bırakmıyor (bkz. `quote::catalog_line`).
    #[serde(default, rename = "priceTaxWithCur")]
    pub price_tax_with_cur: Option<String>,
    #[serde(default)]
    pub quantity: Option<String>,
    #[serde(default, rename = "quantityStatus")]
    pub quantity_status: Option<String>,
    #[serde(default, rename = "imgUrl")]
    pub img_url: Option<String>,
    #[serde(default, rename = "picture2Path")]
    pub picture2: Option<String>,
    #[serde(default, rename = "picture3Path")]
    pub picture3: Option<String>,
    #[serde(default, rename = "picture4Path")]
    pub picture4: Option<String>,
    #[serde(default)]
    pub details: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub keywords: Option<String>,
    #[serde(default)]
    pub descriptions: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FeedRoot {
    #[serde(default, rename = "product")]
    products: Vec<FeedProduct>,
}

impl FeedProduct {
    fn trimmed(mut self) -> Self {
        fn t(v: &mut Option<String>) {
            if let Some(s) = v {
                let trimmed = s.trim();
                if trimmed.len() != s.len() {
                    *s = trimmed.to_string();
                }
            }
        }
        t(&mut self.id);
        t(&mut self.sku);
        t(&mut self.name);
        t(&mut self.status);
        t(&mut self.product_brand);
        t(&mut self.search_keywords);
        t(&mut self.main_category);
        t(&mut self.category);
        t(&mut self.buying_price);
        t(&mut self.price1);
        t(&mut self.tax);
        t(&mut self.currency_abbr);
        t(&mut self.price_tax_with_cur);
        t(&mut self.quantity);
        t(&mut self.quantity_status);
        t(&mut self.img_url);
        t(&mut self.picture2);
        t(&mut self.picture3);
        t(&mut self.picture4);
        t(&mut self.details);
        t(&mut self.url);
        t(&mut self.title);
        t(&mut self.keywords);
        t(&mut self.descriptions);
        self
    }

    pub fn quantity_i64(&self) -> Option<i64> {
        self.quantity.as_deref().and_then(|q| q.trim().parse().ok())
    }

    /// Fiyat alanları sayıya çevriliyor; boş ya da bozuk değer `None` (0 DEĞİL).
    ///
    /// ⚠️ 0'a düşürmek marj hesabını sessizce bozardı: fiyatı bilinmeyen ürün ile bedava
    /// ürün aynı şey değil. Teklif ekranı `None` gördüğünde fiyatı kullanıcıdan istiyor.
    fn sayi(v: &Option<String>) -> Option<f64> {
        v.as_deref()
            .map(|s| s.trim().replace(',', "."))
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|f| f.is_finite() && *f >= 0.0)
    }

    pub fn buying_price_f64(&self) -> Option<f64> {
        Self::sayi(&self.buying_price)
    }
    pub fn price1_f64(&self) -> Option<f64> {
        Self::sayi(&self.price1)
    }
    pub fn tax_f64(&self) -> Option<f64> {
        Self::sayi(&self.tax)
    }
    /// TL fiyatı; feed'de "42.774,88 TL" gibi biçimli gelebildiği için harfler ayıklanıyor.
    pub fn price_tl_f64(&self) -> Option<f64> {
        let ham = self.price_tax_with_cur.as_deref()?;
        let temiz: String = ham.chars().filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',').collect();
        // Binlik ayıracı belirsizliği: son ayıraç ondalık kabul ediliyor.
        let nokta = temiz.rfind('.');
        let virgul = temiz.rfind(',');
        let s = match (nokta, virgul) {
            (Some(n), Some(v)) if v > n => temiz.replace('.', "").replace(',', "."),
            (Some(_), Some(_)) => temiz.replace(',', ""),
            (None, Some(_)) => temiz.replace(',', "."),
            _ => temiz,
        };
        s.parse::<f64>().ok().filter(|f| f.is_finite() && *f >= 0.0)
    }
}

pub fn parse(xml: &str) -> Result<Vec<FeedProduct>, String> {
    let root: FeedRoot = quick_xml::de::from_str(xml)
        .map_err(|e| format!("Feed XML çözümlenemedi: {e}"))?;
    Ok(root.products.into_iter().map(FeedProduct::trimmed).collect())
}

pub async fn fetch(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Feed'e ulaşılamadı: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Feed sunucusu hata döndürdü: HTTP {}", resp.status().as_u16()));
    }
    resp.text()
        .await
        .map_err(|e| format!("Feed içeriği okunamadı: {e}"))
}

pub async fn fetch_and_parse(url: &str) -> Result<Vec<FeedProduct>, String> {
    let xml = fetch(url).await?;
    parse(&xml)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cdata_and_trims() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
        <products>
          <product>
            <id>1</id>
            <sku><![CDATA[ ABC-123 ]]></sku>
            <name><![CDATA[Sony Kulaklık]]></name>
            <status>1</status>
            <productBrand><![CDATA[Sony]]></productBrand>
            <searchKeywords><![CDATA[]]></searchKeywords>
            <mainCategory><![CDATA[Elektronik]]></mainCategory>
            <category><![CDATA[Kulaklık]]></category>
            <quantity>12</quantity>
            <quantityStatus><![CDATA[ var ]]></quantityStatus>
            <imgUrl><![CDATA[https://x/y.jpg]]></imgUrl>
            <details><![CDATA[<section><h2>Başlık</h2><p>Metin</p></section>]]></details>
            <url><![CDATA[https://x/p]]></url>
            <title><![CDATA[Sony Kulaklık]]></title>
            <keywords><![CDATA[kulaklık]]></keywords>
            <descriptions><![CDATA[Sony Kulaklık]]></descriptions>
          </product>
        </products>"#;
        let items = parse(xml).unwrap();
        assert_eq!(items.len(), 1);
        let p = &items[0];
        assert_eq!(p.sku.as_deref(), Some("ABC-123"));
        assert_eq!(p.quantity_status.as_deref(), Some("var"));
        assert_eq!(p.quantity_i64(), Some(12));
        assert!(p.details.as_deref().unwrap().contains("<h2>"));
    }

    #[test]
    fn parses_empty_and_missing_fields() {
        let xml = "<products><product><sku>X</sku><name>Ürün</name></product></products>";
        let items = parse(xml).unwrap();
        assert_eq!(items[0].sku.as_deref(), Some("X"));
        assert!(items[0].title.is_none());
    }

    #[test]
    fn parses_empty_feed() {
        let items = parse("<products></products>").unwrap();
        assert!(items.is_empty());
    }
}
