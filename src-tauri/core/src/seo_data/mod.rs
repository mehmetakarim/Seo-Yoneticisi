//! SEO araştırma verisi (Faz 4+).
//!
//! MCP sunucularını (seo-research-mcp, gsc-mcp, google-news-trends) **paketlemek yerine**,
//! altlarındaki HTTP çağrılarını doğrudan Rust'ta yeniden yazıyoruz → tek self-contained
//! binary, Python/Chromium yok. Her kullanıcı kendi anahtarını Ayarlar'dan girer.
//!
//! - **Faz 4:** `ahrefs` — CapSolver Turnstile + Ahrefs free-tools (keyword ideas + difficulty).
//! - Faz 5: `gsc` — Search Console service-account (gerçek arama sorguları).
//! - Faz 6: `trends` — Google Trends trend terimleri + Ahrefs backlink/trafik.
//!
//! Orkestrasyon **graceful degrade** eder: bir kaynak hata verirse `notes`'a yazılır,
//! diğer kaynaklar denenir; kısmi sonuç yine döner. Sonuç `seo_status.research_json`'a
//! serialize edilir ve üretim (meta/details) prompt'una enjekte edilir.

pub mod ahrefs;
pub mod gsc;
// Faz 6: Google Trends related-queries — Google 429 anti-bot nedeniyle şimdilik ÇAĞRILMIYOR
// (kod korunuyor, ileride yeniden etkinleştirilebilir). Bkz. commands::research_seo.
#[allow(dead_code)]
pub mod trends;

use serde::{Deserialize, Serialize};

/// Anahtar kelime adayı — Ahrefs keyword-generator'dan.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct KeywordCand {
    pub keyword: String,
    /// 0-100 (Ahrefs difficultyLabel → sayı).
    pub difficulty: i64,
    /// Aylık arama hacmi tahmini (alt sınır).
    pub volume: i64,
    /// "idea" | "question"
    pub kind: String,
}

/// Bir anahtar kelimenin Ahrefs zorluk özeti (keyword-difficulty).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct KeywordDifficulty {
    pub keyword: String,
    pub difficulty: i64,
    pub shortage: i64,
    pub last_update: String,
}

/// GSC gerçek arama sorgusu (Faz 5'te doldurulur).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct GscQuery {
    pub query: String,
    pub clicks: f64,
    pub impressions: f64,
    pub ctr: f64,
    pub position: f64,
}

/// GSC'de bir SAYFANIN toplam performansı (fırsat analizi için, sorgu bazlı değil sayfa bazlı).
/// `search_queries` tek ürünün sorgularını getirirken bu, tek çağrıda tüm sayfaları getirir.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PageStat {
    pub page: String,
    pub clicks: f64,
    pub impressions: f64,
    pub ctr: f64,
    pub position: f64,
}

/// GSC'de bir SORGU × SAYFA satırı — fırsat analizinin sorgu düzeyine geçişi için.
///
/// `PageStat` "bu sayfa toplam ne aldı" der; bu ise "bu sayfa ŞU SORGUDA kaçıncı sırada" der.
/// Aradaki fark, "sorun var" ile "şunu yaz" arasındaki fark.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct QueryPageStat {
    pub page: String,
    pub query: String,
    pub clicks: f64,
    pub impressions: f64,
    pub ctr: f64,
    pub position: f64,
}

/// Google Trends trend terimi (Faz 6).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TrendTerm {
    pub term: String,
    pub volume: i64,
}

/// Site geneli Ahrefs backlink özeti (Faz 6) — bilgi amaçlı; ürün başına değil, alan (domain) geneli.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct DomainOverview {
    pub domain: String,
    pub domain_rating: i64,
    pub backlinks: i64,
    pub ref_domains: i64,
}

/// Araştırma çıktısı — panelde gösterilir, `research_json`'a kaydedilir,
/// üretim prompt'una enjekte edilir.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SeoInsights {
    /// Araştırmayı tetikleyen tohum kelime (kullanıcı düzenleyebilir).
    pub seed: String,
    pub target_candidates: Vec<KeywordCand>,
    pub seed_difficulty: Option<KeywordDifficulty>,
    pub gsc_queries: Vec<GscQuery>,
    pub trends: Vec<TrendTerm>,
    pub domain: Option<DomainOverview>,
    pub fetched_at: String,
    /// Kaynak bazlı durum/uyarı mesajları (şeffaflık — panelde gösterilir).
    pub notes: Vec<String>,
}

impl SeoInsights {
    /// Herhangi bir gerçek veri geldi mi? (prompt'a enjeksiyonun anlamlı olması için)
    pub fn has_data(&self) -> bool {
        !self.target_candidates.is_empty()
            || self.seed_difficulty.is_some()
            || !self.gsc_queries.is_empty()
            || !self.trends.is_empty()
            || self.domain.is_some()
    }

    /// Üretim prompt'una eklenecek "Gerçek arama verileri" bloğu.
    /// Veri yoksa boş string döner (mevcut davranış korunur).
    pub fn prompt_block(&self) -> String {
        if !self.has_data() {
            return String::new();
        }
        let mut b = String::from("\n\nGERÇEK ARAMA VERİLERİ (bu verilere dayan, uydurma):");

        if !self.gsc_queries.is_empty() {
            b.push_str("\n- Bu ürün sayfasının Google'da aldığı gerçek sorgular (öncelikli): ");
            let top: Vec<String> = self
                .gsc_queries
                .iter()
                .take(10)
                .map(|q| format!("{} ({} gösterim)", q.query, q.impressions as i64))
                .collect();
            b.push_str(&top.join(", "));
        }

        if !self.target_candidates.is_empty() {
            b.push_str("\n- Ahrefs anahtar kelime fikirleri (zorluk/hacim): ");
            let top: Vec<String> = self
                .target_candidates
                .iter()
                .take(12)
                .map(|k| format!("{} (zorluk {}, hacim {})", k.keyword, k.difficulty, k.volume))
                .collect();
            b.push_str(&top.join(", "));
        }

        if let Some(d) = &self.seed_difficulty {
            b.push_str(&format!(
                "\n- '{}' zorluğu: {} (rekabet)",
                d.keyword, d.difficulty
            ));
        }

        if !self.trends.is_empty() {
            b.push_str("\n- Hedef kelimeye ilgili trend aramaları (Google Trends): ");
            let top: Vec<String> = self.trends.iter().take(8).map(|t| t.term.clone()).collect();
            b.push_str(&top.join(", "));
        }

        b.push_str(
            "\nMümkünse yüksek hacimli + düşük zorluklu ve gerçek sorgularla örtüşen ifadeleri tercih et.",
        );
        b
    }
}

/// Feed'den JSON parse'ında `["Ok", {...}]` sarmalını açar; başka biçimlerde None.
pub(crate) fn unwrap_ok(v: &serde_json::Value) -> Option<&serde_json::Value> {
    let arr = v.as_array()?;
    if arr.len() >= 2 && arr[0].as_str() == Some("Ok") {
        Some(&arr[1])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_block_empty_when_no_data() {
        let ins = SeoInsights::default();
        assert!(!ins.has_data());
        assert_eq!(ins.prompt_block(), "");
    }

    #[test]
    fn prompt_block_includes_sources() {
        let ins = SeoInsights {
            seed: "all in one bilgisayar".into(),
            target_candidates: vec![KeywordCand {
                keyword: "all in one pc".into(),
                difficulty: 15,
                volume: 1000,
                kind: "idea".into(),
            }],
            seed_difficulty: Some(KeywordDifficulty {
                keyword: "all in one bilgisayar".into(),
                difficulty: 22,
                shortage: 0,
                last_update: "2026-07-01".into(),
            }),
            gsc_queries: vec![GscQuery {
                query: "lenovo all in one".into(),
                clicks: 3.0,
                impressions: 120.0,
                ctr: 0.025,
                position: 8.4,
            }],
            trends: vec![TrendTerm { term: "yeni lenovo".into(), volume: 5000 }],
            domain: None,
            fetched_at: "2026-07-22T00:00:00".into(),
            notes: vec![],
        };
        assert!(ins.has_data());
        let block = ins.prompt_block();
        assert!(block.contains("all in one pc"));
        assert!(block.contains("lenovo all in one"));
        assert!(block.contains("zorluk 15"));
        assert!(block.contains("yeni lenovo"));
    }

    #[test]
    fn unwrap_ok_extracts_second_element() {
        let v = serde_json::json!(["Ok", { "difficulty": 5 }]);
        assert_eq!(unwrap_ok(&v).unwrap()["difficulty"], 5);
        let bad = serde_json::json!(["Error", "nope"]);
        assert!(unwrap_ok(&bad).is_none());
        let notarr = serde_json::json!({ "a": 1 });
        assert!(unwrap_ok(&notarr).is_none());
    }
}
