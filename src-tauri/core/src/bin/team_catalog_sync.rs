//! Server-side publication of a complete IdeaSoft catalog.

use reqwest::{redirect::Policy, Client, Url};
use seo_core::feed::{self, FeedProduct};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashSet, env, process::ExitCode, time::Duration};

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let action = match env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => "",
        [flag] if flag == "--apply" => "--apply",
        [flag] if flag == "--snapshot" => "--snapshot",
        _ => return Err("Kullanım: team_catalog_sync [--apply | --snapshot]".into()),
    };
    let feed_url = https_url(&required("CATALOG_FEED_URL")?, "CATALOG_FEED_URL")?;
    let minimum = required("CATALOG_MIN_PRODUCTS")?
        .parse::<usize>()
        .ok()
        .filter(|minimum| *minimum > 0)
        .ok_or("CATALOG_MIN_PRODUCTS pozitif bir tam sayı olmalı.")?;
    let target = if action == "--apply" {
        Some(Target::from_env()?)
    } else {
        None
    };
    // Take the baseline before downloading the feed so overlapping jobs cannot publish stale data.
    let revision = match &target {
        Some(target) => target.revision().await?,
        None => None,
    };
    let products = feed::fetch_and_parse(feed_url.as_str())
        .await
        .map_err(|_| "IdeaSoft feed’i indirilemedi veya XML olarak okunamadı.")?;
    let products = prepare(products, minimum)?;
    let count = products.len();
    if let Some(target) = target {
        let revision = target.publish(revision, products).await?;
        println!("{count} ürün yayımlandı. Katalog sürümü {revision}.");
    } else if action == "--snapshot" {
        println!("{}", json!(products));
    } else {
        println!("{count} ürün doğrulandı. Önizleme tamamlandı.");
    }
    Ok(())
}

fn required(name: &str) -> Result<String, String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{name} yapılandırması eksik."))
}

fn https_url(raw: &str, name: &str) -> Result<Url, String> {
    let url = Url::parse(raw).map_err(|_| format!("{name} geçerli bir HTTPS adresi olmalı."))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(format!("{name} geçerli bir HTTPS adresi olmalı."));
    }
    Ok(url)
}

fn prepare(products: Vec<FeedProduct>, minimum: usize) -> Result<Vec<Value>, String> {
    let mut seen = HashSet::new();
    let mut prepared = Vec::with_capacity(products.len());
    for product in &products {
        let sku = product.sku.as_deref().filter(|sku| !sku.is_empty());
        let name = product.name.as_deref().filter(|name| !name.is_empty());
        let (Some(sku), Some(name)) = (sku, name) else {
            return Err("Feed’de SKU veya adı eksik ürün var.".into());
        };
        if !seen.insert(sku) {
            return Err("Feed’de yinelenen SKU var.".into());
        }
        prepared.push(json!({ "sku": sku, "name": name, "facts": {
            "id": product.id, "status": product.status, "brand": product.product_brand,
            "main_category": product.main_category, "category": product.category,
            "quantity": product.quantity_i64(), "quantity_status": product.quantity_status,
            "url": product.url, "img_url": product.img_url, "picture2": product.picture2,
            "picture3": product.picture3, "picture4": product.picture4,
            "title": product.title, "descriptions": product.descriptions,
            "keywords": product.keywords, "search_keywords": product.search_keywords,
            "details": product.details, "currency_abbr": product.currency_abbr,
            "price1": product.price1_f64(), "buying_price": product.buying_price_f64(),
            "tax_rate": product.tax_f64(), "price_tl": product.price_tl_f64(),
        }}));
    }
    if prepared.len() < minimum {
        return Err(format!(
            "Feed’de {} ürün var. Güvenli yayın eşiği {minimum}.",
            prepared.len()
        ));
    }
    Ok(prepared)
}

struct Target {
    client: Client,
    project: Url,
    credentials: Credentials,
}

enum Credentials {
    Publisher {
        publishable_key: String,
        token: String,
    },
    Administrator {
        key: String,
        workspace: String,
    },
}

impl Target {
    fn from_env() -> Result<Self, String> {
        let project = https_url(&required("CATALOG_SUPABASE_URL")?, "CATALOG_SUPABASE_URL")?;
        if project.path() != "/" || project.query().is_some() {
            return Err("CATALOG_SUPABASE_URL proje kök adresi olmalı.".into());
        }
        let credentials = if env::var_os("CATALOG_PUBLISHER_TOKEN").is_some()
            || env::var_os("CATALOG_SUPABASE_PUBLISHABLE_KEY").is_some()
        {
            let publishable_key = required("CATALOG_SUPABASE_PUBLISHABLE_KEY")?;
            let token = required("CATALOG_PUBLISHER_TOKEN")?;
            if !publishable_key.starts_with("sb_publishable_") {
                return Err(
                    "CATALOG_SUPABASE_PUBLISHABLE_KEY bir sb_publishable_ anahtarı olmalı.".into(),
                );
            }
            Credentials::Publisher {
                publishable_key,
                token,
            }
        } else {
            let key = required("CATALOG_SUPABASE_SECRET_KEY")?;
            if !key.starts_with("sb_secret_") {
                return Err("CATALOG_SUPABASE_SECRET_KEY bir sb_secret_ anahtarı olmalı.".into());
            }
            Credentials::Administrator {
                key,
                workspace: required("CATALOG_WORKSPACE_ID")?,
            }
        };
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .redirect(Policy::none())
            .build()
            .map_err(|_| "Supabase istemcisi başlatılamadı.")?;
        Ok(Self {
            client,
            project,
            credentials,
        })
    }

    async fn revision(&self) -> Result<Option<i64>, String> {
        #[derive(Deserialize)]
        struct CatalogState {
            revision: Option<i64>,
        }
        let response = match &self.credentials {
            Credentials::Publisher {
                publishable_key,
                token,
            } => self
                .client
                .get(
                    self.project
                        .join("functions/v1/team-catalog-publisher")
                        .unwrap(),
                )
                .header("apikey", publishable_key)
                .header("x-catalog-publisher-token", token),
            Credentials::Administrator { key, workspace } => self
                .client
                .get(self.project.join("rest/v1/team_catalog_state").unwrap())
                .header("apikey", key)
                .query(&[
                    ("workspace_id", format!("eq.{workspace}")),
                    ("select", "revision".into()),
                ]),
        }
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|_| "Supabase katalog sürümü okunamadı.")?;
        let invalid = |_| "Supabase katalog sürümü yanıtı geçersiz.".to_string();
        match &self.credentials {
            Credentials::Publisher { .. } => Ok(response
                .json::<CatalogState>()
                .await
                .map_err(invalid)?
                .revision),
            Credentials::Administrator { .. } => Ok(response
                .json::<Vec<CatalogState>>()
                .await
                .map_err(invalid)?
                .first()
                .and_then(|row| row.revision)),
        }
    }

    async fn publish(&self, revision: Option<i64>, products: Vec<Value>) -> Result<i64, String> {
        let request = match &self.credentials {
            Credentials::Publisher { publishable_key, token } => self.client
                .post(self.project.join("functions/v1/team-catalog-publisher").unwrap())
                .header("apikey", publishable_key)
                .header("x-catalog-publisher-token", token)
                .json(&json!({ "expected_revision": revision, "products": products })),
            Credentials::Administrator { key, workspace } => self.client
                .post(self.project.join("rest/v1/rpc/team_replace_catalog").unwrap())
                .header("apikey", key)
                .json(&json!({ "target_workspace": workspace, "expected_revision": revision, "products": products })),
        };
        let response = request
            .send()
            .await
            .map_err(|_| "Yayın yanıtı alınamadı. Katalog sürümünü kontrol edin.")?;
        if !response.status().is_success() {
            return Err(format!(
                "Katalog yayımlanamadı (HTTP {}).",
                response.status().as_u16()
            ));
        }
        response
            .json::<i64>()
            .await
            .map_err(|_| "Yayın sürümü okunamadı. Katalog sürümünü kontrol edin.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_ideasoft_prices_and_trimmed_sku() {
        let items = feed::parse("<products><product><sku> A </sku><name>Sample</name><price1>12,50</price1><priceTaxWithCur>42.774,88 TL</priceTaxWithCur></product></products>").unwrap();
        let snapshot = prepare(items, 1).unwrap();
        assert_eq!(snapshot[0]["sku"], "A");
        assert_eq!(snapshot[0]["facts"]["price1"], 12.5);
        assert_eq!(snapshot[0]["facts"]["price_tl"], 42774.88);
    }

    #[test]
    fn rejects_incomplete_or_ambiguous_snapshots() {
        for (body, minimum) in [
            ("", 1),
            ("<product><sku>A</sku></product>", 1),
            ("<product><sku>A</sku><name>One</name></product>", 2),
            ("<product><sku>A</sku><name>One</name></product><product><sku> A </sku><name>Two</name></product>", 1),
        ] {
            let items = feed::parse(&format!("<products>{body}</products>")).unwrap();
            assert!(prepare(items, minimum).is_err());
        }
    }
}
