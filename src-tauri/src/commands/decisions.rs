//! Satışta olmayan sayfalar için **karar deposu** ve 301 CSV çıktısı (Faz D).
//!
//! # 🔴 Neden var
//!
//! Halef önerileri bugüne kadar **hiçbir yerde saklanmıyordu** — yalnızca `store.successors`,
//! yani bellekte. Uygulama kapanınca verilen kararlar kayboluyordu. Uygulama yönlendirmeyi
//! zaten yapamıyor (IdeaSoft panelinden tanımlanıyor), dolayısıyla elde kalan tek değerli şey
//! **karar**dı ve o da uçuyordu.
//!
//! # ⚠️ Karar ≠ öneri
//!
//! Model bir hedef önerebilir ama satır ancak kullanıcı **onaylayınca** yazılır. Sebebi
//! ölçülmüş (`opportunity::successor_candidates` doc'unda, 2026-07-29): deterministik
//! eşleştirici tek başına güvenilir değil — `asus-zenbook-17-fold` için en iyi aday
//! *"Microsoft Windows 11 Pro"* çıkmıştı. **Yanlış yönlendirme, yönlendirmemekten kötüdür.**
//!
//! Bu yüzden CSV'de kararsız satırların hedef sütunu **bilerek boş** bırakılıyor; en iyi aday
//! yalnızca bilgi sütununda duruyor.

use super::*;

/// Bir sayfa için verilmiş karar.
#[derive(Serialize, Clone)]
pub struct EolDecision {
    pub slug: String,
    pub url: String,
    /// `redirect_301` | `canonical` | `keep`
    pub action: String,
    pub target_slug: Option<String>,
    pub target_sku: Option<String>,
    /// `ai` | `manual` — hedefi model mi önerdi, kullanıcı mı seçti.
    pub source: String,
    pub decided_at: String,
    pub exported_at: Option<String>,
}

/// CSV üretim özeti — ekran ne olduğunu söyleyebilsin diye.
#[derive(Serialize)]
pub struct ExportSummary {
    pub decided_rows: usize,
    pub undecided_rows: usize,
    pub bytes: usize,
    pub path: String,
}

fn row(r: &rusqlite::Row) -> rusqlite::Result<EolDecision> {
    Ok(EolDecision {
        slug: r.get(0)?,
        url: r.get(1)?,
        action: r.get(2)?,
        target_slug: r.get(3)?,
        target_sku: r.get(4)?,
        source: r.get(5)?,
        decided_at: r.get(6)?,
        exported_at: r.get(7)?,
    })
}

/// Verilmiş tüm kararlar (ekran satırları rozetlemek için kullanıyor).
#[tauri::command]
pub fn get_eol_decisions(state: State<'_, AppState>) -> Result<Vec<EolDecision>, String> {
    let conn = state.conn.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT slug, url, action, target_slug, target_sku, source, decided_at, exported_at
             FROM eol_decisions ORDER BY decided_at DESC",
        )
        .map_err(|e| format!("Kararlar okunamadı: {e}"))?;
    let rows = stmt
        .query_map([], row)
        .map_err(|e| format!("Kararlar okunamadı: {e}"))?;
    Ok(rows.filter_map(Result::ok).collect())
}

/// Karar girdisini doğrular ve saklanacak hedefi döndürür — **saf, test edilebilir**.
///
/// İki kural: bilinmeyen karar türü reddedilir · `keep` dışındaki kararlar hedefsiz olamaz.
/// `keep`te hedef **temizlenir**: "bu sayfayı bilinçli tutuyorum" bir yere yönlendirmek değil.
fn validate(
    action: &str,
    target_slug: Option<String>,
    target_sku: Option<String>,
) -> Result<(Option<String>, Option<String>), String> {
    if !matches!(action, "redirect_301" | "canonical" | "keep") {
        return Err(format!("Bilinmeyen karar türü: {action}"));
    }
    if action == "keep" {
        return Ok((None, None));
    }
    if target_slug.as_deref().unwrap_or("").trim().is_empty() {
        return Err("Hedef seçilmeden karar kaydedilemez.".into());
    }
    Ok((target_slug, target_sku))
}

/// Karar verir/günceller.
///
/// `exported_at` bilerek **sıfırlanıyor**: karar değiştiyse önceki CSV artık geçerli değil,
/// yeniden dışa aktarılmalı.
#[tauri::command]
pub fn save_eol_decision(
    state: State<'_, AppState>,
    slug: String,
    url: String,
    action: String,
    target_slug: Option<String>,
    target_sku: Option<String>,
    source: String,
) -> Result<(), String> {
    let (ts, tsku) = validate(&action, target_slug, target_sku)?;

    let conn = state.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO eol_decisions
           (slug, url, action, target_slug, target_sku, source, decided_at, exported_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,NULL)
         ON CONFLICT(slug) DO UPDATE SET
           url = ?2, action = ?3, target_slug = ?4, target_sku = ?5, source = ?6,
           decided_at = ?7, exported_at = NULL",
        params![slug, url, action, ts, tsku, source, now_str()],
    )
    .map_err(|e| format!("Karar kaydedilemedi: {e}"))?;
    Ok(())
}

/// Kararı geri alır — sayfa yeniden "kararsız" olur.
#[tauri::command]
pub fn delete_eol_decision(state: State<'_, AppState>, slug: String) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute("DELETE FROM eol_decisions WHERE slug = ?1", [slug])
        .map_err(|e| format!("Karar silinemedi: {e}"))?;
    Ok(())
}

/// CSV alanını kaçırır. Virgül, tırnak veya satır sonu varsa tırnak içine alınır.
fn csv(v: &str) -> String {
    if v.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}

/// **301 iş listesi CSV'si** — tek dosya, iki bölüm.
///
/// - Karar verilmiş satırlar: `hedef_yol` **dolu**, panele girilmeye hazır.
/// - Kararsız satırlar: `hedef_yol` **BOŞ**; en iyi aday yalnızca `aday`/`benzerlik` bilgi
///   sütunlarında. Ölçüm bu ayrımı zorunlu kılıyor (modül başlığındaki Windows 11 örneği).
///
/// ⚠️ Excel'de doldurulan hedefler uygulamaya **geri dönmüyor**. CSV bir çıktı, bir senkron
/// kanalı değil; karar deposu yalnızca uygulama içinde verilen kararları bilir.
#[tauri::command]
pub fn export_redirect_csv(
    state: State<'_, AppState>,
    path: String,
    min_clicks: f64,
) -> Result<ExportSummary, String> {
    let conn = state.conn.lock().unwrap();
    let (out, decided, undecided) = build_csv(&conn, min_clicks)?;

    std::fs::write(&path, &out).map_err(|e| format!("Dosya yazılamadı: {e}"))?;

    // Hangi kararların panele taşındığı izlensin: "bunu girdim mi?" sorusu cevaplanabilsin.
    let _ = conn.execute(
        "UPDATE eol_decisions SET exported_at = ?1 WHERE action <> 'keep'",
        [now_str()],
    );
    // Dışa aktarım bir iştir; zaman çizelgesinde görünsün. Mağazaya ULAŞMIYOR (301'i
    // kullanıcı panelde tanımlıyor), bu yüzden `reaches_store = false`.
    super::log_event(&conn, "", "redirect_csv_export", false);

    Ok(ExportSummary { decided_rows: decided, undecided_rows: undecided, bytes: out.len(), path })
}

/// CSV metnini kurar — **dosyaya yazmadan**, böylece gerçek veriyle test edilebiliyor.
///
/// Döner: `(içerik, karar_verilmiş, kararsız)`.
pub(crate) fn build_csv(
    conn: &Connection,
    min_clicks: f64,
) -> Result<(String, usize, usize), String> {
    let raw = db::get_setting(conn, "opportunity_json")?
        .ok_or("Önce Search Console analizi çalıştırılmalı.")?;
    let report: OpportunityReport =
        serde_json::from_str(&raw).map_err(|e| format!("Analiz okunamadı: {e}"))?;

    let mut stmt = conn
        .prepare(
            "SELECT slug, url, action, target_slug, target_sku, source, decided_at, exported_at
             FROM eol_decisions",
        )
        .map_err(|e| format!("Kararlar okunamadı: {e}"))?;
    let kararlar: Vec<EolDecision> = stmt
        .query_map([], row)
        .map_err(|e| format!("Kararlar okunamadı: {e}"))?
        .filter_map(Result::ok)
        .collect();

    // Aday sütunları için katalog (deterministik sıralayıcı — YALNIZCA bilgi amaçlı).
    let katalog = super::live_catalog(conn);

    let mut out = String::new();
    // Başlık öncesi açıklama: dosyayı Excel'de açan kişi kuralı bilmeli.
    out.push_str("# 301 is listesi — SEO Yoneticisi\n");
    out.push_str(
        "# Karar verilmis satirlarda hedef_yol DOLU. Kararsiz satirlarda hedef_yol BOS birakildi:\n",
    );
    out.push_str(
        "# aday sutunu yalnizca bilgi amaclidir, dogrulanmadan yonlendirme tanimlamayin.\n",
    );
    out.push_str("kaynak_yol,hedef_yol,tiklama,konum,karar,kaynak,karar_tarihi,aday,benzerlik\n");

    let mut decided = 0usize;
    let mut undecided = 0usize;

    for e in &report.eol {
        let k = kararlar.iter().find(|d| d.slug == e.slug);
        // Bilinçli tutulan sayfa iş listesine girmiyor — karar zaten "dokunma".
        if k.map(|d| d.action.as_str()) == Some("keep") {
            continue;
        }
        let karar_var = k.is_some();
        if !karar_var && e.clicks < min_clicks {
            continue;
        }

        let (hedef, karar, kaynak, tarih) = match k {
            Some(d) => (
                d.target_slug.clone().unwrap_or_default(),
                d.action.clone(),
                d.source.clone(),
                d.decided_at.get(..10).unwrap_or("").to_string(),
            ),
            None => (String::new(), String::new(), String::new(), String::new()),
        };

        // Aday sütunları YALNIZCA kararsız satırlarda: karar verilmişse gürültü olur.
        let (aday, benzerlik) = if karar_var {
            (String::new(), String::new())
        } else {
            match seo_core::opportunity::successor_candidates(&e.slug, &katalog, 1).first() {
                Some(c) => (c.name.clone(), format!("{:.2}", c.score)),
                None => (String::new(), String::new()),
            }
        };

        if karar_var {
            decided += 1;
        } else {
            undecided += 1;
        }

        out.push_str(&format!(
            "{},{},{},{:.1},{},{},{},{},{}\n",
            csv(&e.slug),
            csv(&hedef),
            e.clicks.round(),
            e.position,
            csv(&karar),
            csv(&kaynak),
            csv(&tarih),
            csv(&aday),
            benzerlik
        ));
    }

    Ok((out, decided, undecided))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 🔑 **Fazın bitiş şartı:** gerçek veriyle CSV üretilip AÇILIP kontrol ediliyor.
    ///
    /// `SEO_DB_COPY=/tmp/kopya.db cargo test csv_real -- --ignored --nocapture`
    ///
    /// Sabitlenen kural: karar verilmiş satırlarda `hedef_yol` DOLU, kararsızlarda **BOŞ**
    /// ve en iyi aday yalnızca bilgi sütununda. Otomatik doldurmanın tehlikeli olduğu
    /// ölçülmüştü (`opportunity::successor_candidates`).
    #[test]
    #[ignore]
    fn csv_real() {
        let db = std::env::var("SEO_DB_COPY").expect("SEO_DB_COPY yok");
        let conn = Connection::open(&db).unwrap();
        seo_core::db::init(&conn).unwrap();

        let (out, decided, undecided) = build_csv(&conn, 3.0).unwrap();
        println!(
            "karar verilmiş: {decided} · kararsız: {undecided} · {} bayt · {} satır",
            out.len(),
            out.lines().count()
        );
        println!("\n--- ilk 6 satır ---");
        for l in out.lines().take(6) {
            println!("{l}");
        }

        // Başlık ve açıklama satırları
        assert!(out.starts_with("# 301 is listesi"));
        assert!(out.contains("kaynak_yol,hedef_yol,tiklama,konum,karar,kaynak,karar_tarihi,aday,benzerlik"));

        // ⚠️ Kararsız satırların hedef sütunu BOŞ olmalı — CSV'nin en kritik kuralı.
        let veri: Vec<&str> = out.lines().filter(|l| !l.starts_with('#')).skip(1).collect();
        assert!(!veri.is_empty(), "gerçek veriden hiç satır çıkmadı");
        let mut bos_hedef = 0;
        for l in &veri {
            let s: Vec<&str> = l.split(',').collect();
            // karar sütunu (index 4) boşsa hedef (index 1) de boş olmalı
            if s.len() > 4 && s[4].is_empty() {
                assert!(s[1].is_empty(), "kararsız satırda hedef DOLU: {l}");
                bos_hedef += 1;
            }
        }
        println!("\nkararsız satırların hepsinde hedef boş: {bos_hedef} satır");
        assert_eq!(bos_hedef, undecided);
    }

    #[test]
    fn csv_alani_virgul_ve_tirnagi_kaciriyor() {
        assert_eq!(csv("basit"), "basit");
        assert_eq!(csv("a,b"), "\"a,b\"");
        assert_eq!(csv("14\" ekran"), "\"14\"\" ekran\"");
        assert_eq!(csv("iki\nsatir"), "\"iki\nsatir\"");
    }

    /// ⚠️ Hedefsiz bir 301 kaydedilememeli — boş hedefli satır CSV'ye çıkarsa panele
    /// yanlış girilir. `keep` ise tam tersi: hedef taşımamalı.
    #[test]
    fn hedefsiz_301_reddediliyor_keep_hedefi_temizliyor() {
        assert!(validate("redirect_301", None, None).is_err());
        assert!(validate("redirect_301", Some("   ".into()), None).is_err());
        assert!(validate("bilinmeyen", Some("x".into()), None).is_err());

        assert_eq!(
            validate("redirect_301", Some("yeni-urun".into()), Some("SKU1".into())).unwrap(),
            (Some("yeni-urun".to_string()), Some("SKU1".to_string()))
        );
        // "Bilinçli tutuyorum" bir yere yönlendirmek değil: hedef temizleniyor.
        assert_eq!(
            validate("keep", Some("yanlislikla".into()), Some("SKU1".into())).unwrap(),
            (None, None)
        );
    }
}
