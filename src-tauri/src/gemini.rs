//! Faz 2/3 iskeleti. Gemini çağrıları burada toplanacak:
//! - Faz 2: tek çağrı, JSON structured output → hedef kelime + title + descriptions
//!   + keywords + searchKeywords üretimi, kural fail'inde tek retry ("kısalt/uzat").
//! - Faz 3: details HTML üretimi; <img src> listesi üretimden önce çıkarılır,
//!   dönen HTML'de bozulan src'ler orijinalleriyle geri yazılır.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratedMeta {
    pub target_keyword: String,
    pub title: String,
    pub descriptions: String,
    pub keywords: String,
    pub search_keywords: String,
}

pub async fn generate_meta(
    _api_key: &str,
    _product_name: &str,
    _brand: Option<&str>,
    _category: Option<&str>,
) -> Result<GeneratedMeta, String> {
    Err("Gemini ile meta üretimi Faz 2'de aktif olacak.".to_string())
}

pub async fn generate_details(
    _api_key: &str,
    _details_html: &str,
    _target_keyword: &str,
) -> Result<String, String> {
    Err("Açıklama üretimi Faz 3'te aktif olacak.".to_string())
}

/// Ayarlardaki "Bağlantıyı test et" — Faz 1'de yalnızca biçim kontrolü yapar.
pub fn check_key_format(key: &str) -> Result<String, String> {
    let k = key.trim();
    if k.len() < 20 {
        return Err("Anahtar çok kısa görünüyor.".to_string());
    }
    Ok("Anahtar biçimi geçerli · gerçek bağlantı testi Faz 2'de aktif olacak.".to_string())
}
