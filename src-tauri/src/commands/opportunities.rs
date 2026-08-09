//! Google Search Console fırsat analizi ve EOL halef önerisi.
//!
//! ⚠️ `OpportunityReport` DB'ye serialize ediliyor → yeni alanlar `#[serde(default)]`
//! taşımalı, yoksa eski önbelleği olan kurulumlarda ekran düşer (v0.5.9 saha hatası).

use super::*;

#[derive(Serialize, serde::Deserialize)]
pub struct InvisibleProduct {
    pub sku: String,
    pub name: String,
    pub url: String,
}

/// ⚠️ **Deserialize + `serde(default)` zorunlu.** Bu yapı önbelleğe (`opportunity_json`)
/// yazılıyor ve sonraki sürümlerde yeni alanlar ekleniyor. Önbellek eski sürümden kalmışsa
/// yeni alanlar JSON'da YOKTUR; varsayılan verilmezse ya çözümleme başarısız olur ya da
/// ön yüze eksik nesne gider ve arayüz çöker.
///
/// v0.5.8'de tam bu oldu: `eol` alanı eklendi, eski önbellekte yoktu, ön yüz
/// `report.eol.length` deyince Fırsatlar ekranı bomboş açıldı. Alan eklerken burayı unutma.
#[derive(Serialize, serde::Deserialize)]
#[serde(default)]
pub struct OpportunityReport {
    pub analyzed_at: String,
    pub days: i64,
    /// Kaçırılan tıklamaya göre azalan sıralı.
    pub opportunities: Vec<opportunity::Opportunity>,
    /// GSC'de hiç satırı olmayan ürünler — farklı bir iş (indeksleme/görünürlük),
    /// meta üretimiyle çözülmediği için ayrı listede.
    pub invisible: Vec<InvisibleProduct>,
    pub total_products: usize,
    /// GSC verisiyle eşleşen ürün sayısı — eşleşme düşükse sorun URL biçimindedir.
    pub matched: usize,
    /// **Satışta olmayan ama trafik alan sayfalar** (en çok tıklama alan başta).
    /// Ölçüm: bu sitede ürün trafiğinin %69'u buraya gidiyor.
    pub eol: Vec<opportunity::EolPage>,
    /// EOL sayfaların toplam tıklaması — fırsat listesiyle kıyaslanabilsin diye.
    pub eol_clicks: f64,
    /// 4–20. sıradaki sorgular — "ne yazmalıyım" katmanı. Kaçırılan tıklamaya göre sıralı.
    pub striking: Vec<opportunity::QueryOpportunity>,
    /// Aynı sorguda yarışan kendi sayfalarımız. Tespit var, birleştirme kararı operatörde.
    pub cannibalization: Vec<opportunity::Cannibalization>,
    /// Önceki döneme göre gerileyen sayfalar — kaybedilen tıklamaya göre sıralı.
    pub decay: Vec<opportunity::Decay>,
}

impl Default for OpportunityReport {
    fn default() -> Self {
        Self {
            analyzed_at: String::new(),
            days: 0,
            opportunities: Vec::new(),
            invisible: Vec::new(),
            total_products: 0,
            matched: 0,
            eol: Vec::new(),
            eol_clicks: 0.0,
            striking: Vec::new(),
            cannibalization: Vec::new(),
            decay: Vec::new(),
        }
    }
}

/// GSC'nin döndürdüğü URL ile feed'deki URL arasındaki zararsız farkları törpüler
/// (sondaki `/`, büyük/küçük harf). Aksi halde tek karakterlik fark yüzünden ürün
/// "Google'da görünmüyor" gibi raporlanırdı.
fn norm_url(u: &str) -> String {
    u.trim().trim_end_matches('/').to_lowercase()
}

const OPPORTUNITY_DAYS: i64 = 90;

#[tauri::command]
pub async fn analyze_opportunities(
    state: State<'_, AppState>,
) -> Result<OpportunityReport, String> {
    let (gsc_json, gsc_site, products) = {
        let conn = state.conn.lock().unwrap();
        let gsc_json = db::get_setting(&conn, "gsc_service_account_json")?.unwrap_or_default();
        let gsc_site = db::get_setting(&conn, "gsc_site_url")?.unwrap_or_default();
        // Kategori/marka ve SEO iş durumu da alınır — fırsat listesinde "hiç dokunulmamış" ile
        // "çalışılmış ama hâlâ sorunlu" ayrımı için. Tek sorgu, ek ağ çağrısı yok.
        let mut stmt = conn
            .prepare(
                "SELECT p.sku, p.name, p.url, COALESCE(p.category,''), COALESCE(p.brand,''),
                        COALESCE(s.meta_status,'pending'), COALESCE(s.details_status,'pending')
                 FROM products p LEFT JOIN seo_status s ON s.sku = p.sku
                 WHERE p.url IS NOT NULL AND p.url <> ''",
            )
            .map_err(|e| format!("Ürünler okunamadı: {e}"))?;
        let rows: Vec<(String, String, String, String, String, String, String)> = stmt
            .query_map([], |r| {
                Ok((
                    r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?,
                ))
            })
            .map_err(|e| format!("Ürünler okunamadı: {e}"))?
            .filter_map(|r| r.ok())
            .collect();
        (gsc_json, gsc_site, rows)
    };

    // Yapılandırılmamışsa sessizce boş liste DÖNME — kullanıcı "fırsat yok" sanır.
    if gsc_json.trim().is_empty() || gsc_site.trim().is_empty() {
        return Err(
            "Google Search Console bağlantısı kurulmamış. Ayarlar'dan service-account \
             dosyasını yükleyip mülk adresini girin."
                .to_string(),
        );
    }
    if products.is_empty() {
        return Err("Önce ürünleri senkronize edin.".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {e}"))?;

    // Tek çağrı: sayfa boyutunda tüm site. Ürün başına istek atmak 262 çağrı olurdu.
    let stats = seo_data::gsc::page_stats(
        &client,
        &gsc_json,
        gsc_site.trim(),
        OPPORTUNITY_DAYS,
        25_000,
    )
    .await?;

    let by_url: std::collections::HashMap<String, &seo_data::PageStat> =
        stats.iter().map(|s| (norm_url(&s.page), s)).collect();
    // Yol öneki türetmek için ham (normalize edilmemiş) ürün URL'leri
    let product_urls: Vec<String> = products.iter().map(|p| p.2.clone()).collect();
    // Sorgu analizi için TÜM ürünler — sayfa düzeyinde sorunsuz görünen bir ürün de
    // belirli bir sorguda 12. sırada olabilir.
    let product_index: std::collections::HashMap<String, (String, String)> = products
        .iter()
        .map(|p| (norm_url(&p.2), (p.0.clone(), p.1.clone())))
        .collect();
    let path_prefix = opportunity::common_path_prefix(&product_urls);

    let total_products = products.len();
    let mut opportunities = Vec::new();
    let mut invisible = Vec::new();
    let mut matched = 0usize;

    for (sku, name, url, category, brand, meta_status, details_status) in products {
        match by_url.get(&norm_url(&url)) {
            Some(st) => {
                matched += 1;
                if let Some((reason, missed)) =
                    opportunity::classify(st.clicks, st.impressions, st.ctr, st.position)
                {
                    opportunities.push(opportunity::Opportunity {
                        sku,
                        name,
                        url,
                        clicks: st.clicks,
                        impressions: st.impressions,
                        ctr: st.ctr,
                        position: st.position,
                        missed_clicks: missed,
                        reason,
                        category,
                        brand,
                        meta_status,
                        details_status,
                    });
                }
            }
            None => invisible.push(InvisibleProduct { sku, name, url }),
        }
    }

    opportunity::sort_by_impact(&mut opportunities);

    // Satışta olmayan ama trafik alan sayfalar. `stats` zaten tüm siteyi içeriyor →
    // EK API ÇAĞRISI YOK. Ürün yolu, ürünlerin kendi URL'lerinden türetiliyor ki
    // blog/kategori sayfaları "satışta olmayan ürün" sanılmasın.
    // DİKKAT: `by_url` GSC sayfalarını tutuyor, kataloğu değil. "Satışta" kümesi
    // ÜRÜNLERDEN kurulmalı — yoksa her sayfa "satışta" sayılır ve EOL listesi hep boş çıkar.
    let live: std::collections::HashSet<String> =
        product_urls.iter().map(|u| norm_url(u)).collect();
    let page_tuples: Vec<(String, f64, f64, f64)> = stats
        .iter()
        .map(|s| (s.page.clone(), s.clicks, s.impressions, s.position))
        .collect();
    let eol = opportunity::find_eol(&page_tuples, &live, path_prefix.as_deref());
    let eol_clicks = eol.iter().map(|e| e.clicks).sum();

    // Sorgu düzeyi veri: ikinci bir GSC çağrısı (ölçüm: 24.204 satır, ~2,5 sn).
    // Yol öneki ürünlerden türetiliyor; blog/kategori sayfaları hiç gelmesin diye.
    // Bu çağrı başarısız olursa analizin GERİ KALANI YİNE DÖNSÜN — sorgu katmanı
    // ek bilgidir, onun yokluğu tüm raporu kaybettirmemeli.
    let (striking, cannibalization) = match seo_data::gsc::query_page_stats(
        &client,
        &gsc_json,
        gsc_site.trim(),
        OPPORTUNITY_DAYS,
        path_prefix.as_deref(),
        60_000,
    )
    .await
    {
        Ok(qp) => {
            let by_page: std::collections::HashMap<String, (String, String)> = opportunities
                .iter()
                .map(|o| (norm_url(&o.url), (o.sku.clone(), o.name.clone())))
                .chain(
                    // Fırsat listesinde olmayan ürünler de sorgu analizine girsin:
                    // bir ürün sayfa düzeyinde sorunsuz görünüp belirli bir sorguda
                    // 12. sırada olabilir.
                    product_index.iter().map(|(u, v)| (u.clone(), v.clone())),
                )
                .collect();
            let rows: Vec<(String, String, f64, f64, f64, f64)> = qp
                .iter()
                .map(|r| {
                    (
                        norm_url(&r.page),
                        r.query.clone(),
                        r.clicks,
                        r.impressions,
                        r.ctr,
                        r.position,
                    )
                })
                .collect();
            (
                opportunity::striking_distance(&rows, &by_page),
                opportunity::cannibalization(&rows, &by_page),
            )
        }
        Err(_) => (Vec::new(), Vec::new()),
    };

    // Trend: önceki 90 günü de çek ve karşılaştır. Ayrı bir GSC çağrısı; başarısız olursa
    // raporun geri kalanı yine dönsün — trend ek bilgidir.
    let decay = match seo_data::gsc::page_stats_offset(
        &client,
        &gsc_json,
        gsc_site.trim(),
        OPPORTUNITY_DAYS,
        OPPORTUNITY_DAYS,
        25_000,
    )
    .await
    {
        Ok(prev) => {
            let to_tuples = |v: &[seo_data::PageStat]| -> Vec<(String, f64, f64, f64)> {
                v.iter()
                    .map(|s| (norm_url(&s.page), s.clicks, s.impressions, s.position))
                    .collect()
            };
            opportunity::find_decay(&to_tuples(&stats), &to_tuples(&prev), &product_index)
        }
        Err(_) => Vec::new(),
    };

    let report = OpportunityReport {
        analyzed_at: now_str(),
        days: OPPORTUNITY_DAYS,
        opportunities,
        invisible,
        total_products,
        matched,
        eol,
        eol_clicks,
        striking,
        cannibalization,
        decay,
    };

    // Önbelleğe al: GSC verisi günlük değişir, her sayfa açılışında API'ye gitmeye gerek yok.
    // ⚠️ Bu önbellek ÜZERİNE YAZILIYOR — geçmiş burada tutulmuyor, `metric_snapshots`'ta.
    if let Ok(json) = serde_json::to_string(&report) {
        let conn = state.conn.lock().unwrap();
        let _ = db::set_setting(&conn, "opportunity_json", &json);
    }

    // Ölçüm omurgası: son anlık görüntü ≥7 günse yenisini al. Hata yutuluyor — ölçüm
    // kaydı alınamadı diye fırsat analizi başarısız sayılmamalı.
    super::snapshot_if_due(&state, &gsc_json, gsc_site.trim(), &client).await;

    Ok(report)
}

#[derive(Serialize)]
pub struct SuccessorSuggestion {
    /// Model bir halef seçtiyse dolu; "halef yok" dediyse boş.
    pub sku: Option<String>,
    pub name: Option<String>,
    pub url: Option<String>,
    pub reason: String,
    pub model: String,
    /// Deterministik adaylar — model seçmese de operatör kendi bakabilsin.
    pub candidates: Vec<opportunity::SuccessorCandidate>,
}

/// Satışta olmayan bir sayfa için halef ürün önerisi.
///
/// **İstek üzerine, tek sayfa için** çalışır — 1.073 EOL sayfanın tamamı için model çağırmak
/// günlük kotayı (flash modellerde 20/gün) anında tüketirdi. Trafik zaten en tepedeki
/// sayfalarda yoğunlaşıyor; operatör oradan başlar.
#[tauri::command]
pub async fn suggest_eol_successor(
    state: State<'_, AppState>,
    url: String,
) -> Result<SuccessorSuggestion, String> {
    let (api_key, catalog) = {
        let conn = state.conn.lock().unwrap();
        let key = db::get_setting(&conn, "gemini_api_key")?.unwrap_or_default();
        (key, super::live_catalog(&conn))
    };

    // Kod adayları daraltır (262 → 5), model karar verir. Bkz. successor_candidates dokümanı.
    let candidates = opportunity::successor_candidates(&url, &catalog, 5);
    if candidates.is_empty() {
        return Ok(SuccessorSuggestion {
            sku: None,
            name: None,
            url: None,
            reason: "Katalogda benzer bir ürün bulunamadı.".into(),
            model: String::new(),
            candidates,
        });
    }

    let pairs: Vec<(String, String)> = candidates
        .iter()
        .map(|c| (c.sku.clone(), c.name.clone()))
        .collect();
    let produced = gemini::suggest_successor(&api_key, &url, &pairs).await?;

    let (sku, name, url_out, reason) = match produced.value {
        Some((sku, reason)) => {
            let found = catalog.iter().find(|(s, _, _)| s == &sku);
            match found {
                Some((s, n, u)) => (Some(s.clone()), Some(n.clone()), Some(u.clone()), reason),
                // Şemadan geçmiş ama katalogda yoksa halef yok say.
                None => (None, None, None, "Uygun bir halef bulunamadı.".into()),
            }
        }
        None => (
            None,
            None,
            None,
            "Uygun bir halef bulunamadı — kategori sayfasına yönlendirmeyi değerlendirin.".into(),
        ),
    };

    Ok(SuccessorSuggestion {
        sku,
        name,
        url: url_out,
        reason,
        model: produced.model.to_string(),
        candidates,
    })
}

/// Önbellekteki son analiz (API'ye gitmeden). Hiç çalıştırılmadıysa `None`.
#[tauri::command]
pub fn get_opportunity_cache(
    state: State<'_, AppState>,
) -> Result<Option<OpportunityReport>, String> {
    let conn = state.conn.lock().unwrap();
    let raw = db::get_setting(&conn, "opportunity_json")?;
    // Ham `serde_json::Value` DÖNDÜRME: eski sürümden kalan önbellekte yeni alanlar
    // bulunmaz ve ön yüz eksik nesneyle çöker (v0.5.8'de yaşandı). Yapıdan geçirince
    // `serde(default)` devreye girer, ön yüz her zaman tam bir nesne alır.
    Ok(raw.and_then(|j| serde_json::from_str::<OpportunityReport>(&j).ok()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regresyon (v0.5.8): rapora `eol` alanı eklendi, eski önbellekte yoktu ve ön yüz
    /// `report.eol.length` deyince Fırsatlar ekranı bomboş açıldı. Önbellek artık yapıdan
    /// geçiyor; eksik alanlar varsayılanla doluyor. Rapora alan eklerken bu test korur.
    #[test]
    fn old_opportunity_cache_still_parses() {
        // v0.5.6 biçimi: eol / eol_clicks YOK, fırsat satırında bağlam alanları YOK
        let old = r#"{
            "analyzed_at":"2026-07-28T10:00:00","days":90,"total_products":262,"matched":260,
            "invisible":[],
            "opportunities":[{"sku":"X","name":"Ürün","url":"u","clicks":1.0,"impressions":100.0,
                              "ctr":0.01,"position":8.0,"missed_clicks":2.3,"reason":"low_ctr"}]
        }"#;
        let r: OpportunityReport =
            serde_json::from_str(old).expect("eski önbellek çözümlenebilmeli");
        assert_eq!(r.opportunities.len(), 1);
        assert!(r.eol.is_empty(), "eksik alan varsayılana düşmeli, çözümleme kırılmamalı");
        assert_eq!(r.eol_clicks, 0.0);
        assert_eq!(r.matched, 260);
    }
}
