//! Ayarlar, bağlantı testleri ve yedekleme (dışa/içe aktarma).

use super::*;

#[tauri::command]
pub async fn test_capsolver_key(key: String) -> Result<String, String> {
    seo_data::ahrefs::test_key(&key).await
}

/// Kurulum sihirbazı gösterilmeli mi? (Karar mantığı `seo_core::db::needs_setup`'ta.)
#[tauri::command]
pub fn needs_setup(state: State<'_, AppState>) -> Result<bool, String> {
    let conn = state.conn.lock().unwrap();
    db::needs_setup(&conn)
}

/// Sihirbaz tamamlandı (ya da bilinçli olarak atlandı) — bir daha kendiliğinden açılmasın.
///
/// ⚠️ Atlandığında da yazılıyor: her açılışta sihirbazla karşılaşmak, atlama seçeneğini
/// anlamsız kılardı. Kullanıcı Ayarlar'dan istediğinde tekrar çalıştırabiliyor.
#[tauri::command]
pub fn mark_setup_done(state: State<'_, AppState>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    db::set_setting(&conn, "setup_done", &now_str())
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let conn = state.conn.lock().unwrap();
    Ok(Settings {
        feed_url: db::feed_url(&conn)?,
        gemini_api_key: db::get_setting(&conn, "gemini_api_key")?.unwrap_or_default(),
        capsolver_api_key: db::get_setting(&conn, "capsolver_api_key")?.unwrap_or_default(),
        seo_country: db::get_setting(&conn, "seo_country")?.unwrap_or_else(|| "tr".to_string()),
        gsc_site_url: db::get_setting(&conn, "gsc_site_url")?.unwrap_or_default(),
        gsc_client_email: db::get_setting(&conn, "gsc_service_account_json")?
            .as_deref()
            .and_then(seo_data::gsc::client_email_of)
            .unwrap_or_default(),
        ideasoft_domain: db::get_setting(&conn, "ideasoft_domain")?.unwrap_or_default(),
        ideasoft_token: db::get_setting(&conn, "ideasoft_token")?.unwrap_or_default(),
        ideasoft_active: !db::get_setting(&conn, "ideasoft_domain")?.unwrap_or_default().trim().is_empty()
            && !db::get_setting(&conn, "ideasoft_token")?.unwrap_or_default().trim().is_empty(),
        theme: db::get_setting(&conn, "theme")?,
        last_backup_at: db::get_setting(&conn, "last_backup_at")?,
    })
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, AppState>,
    feed_url: String,
    gemini_api_key: String,
    capsolver_api_key: String,
    seo_country: String,
    gsc_site_url: String,
    ideasoft_domain: String,
    ideasoft_token: String,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let url = feed_url.trim();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Geçerli bir URL girin (http/https).".to_string());
    }
    db::set_setting(&conn, "feed_url", url)?;
    db::set_setting(&conn, "gemini_api_key", gemini_api_key.trim())?;
    db::set_setting(&conn, "capsolver_api_key", capsolver_api_key.trim())?;
    let country = seo_country.trim().to_lowercase();
    let country = if country.is_empty() { "tr".to_string() } else { country };
    db::set_setting(&conn, "seo_country", &country)?;
    db::set_setting(&conn, "gsc_site_url", gsc_site_url.trim())?;
    db::set_setting(&conn, "ideasoft_domain", ideasoft_domain.trim())?;
    db::set_setting(&conn, "ideasoft_token", ideasoft_token.trim())?;
    Ok(())
}

/// Faz 5: seçilen service-account JSON dosyasını okur, doğrular + saklar. UI'ya client_email döner.
#[tauri::command]
pub fn set_gsc_service_account(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Dosya okunamadı: {e}"))?;
    let email = seo_data::gsc::validate_json(&json)?;
    let conn = state.conn.lock().unwrap();
    db::set_setting(&conn, "gsc_service_account_json", json.trim())?;
    Ok(email)
}

/// Faz 5: yüklü SA'yı kaldırır (GSC'yi devre dışı bırakır).
#[tauri::command]
pub fn clear_gsc_service_account(state: State<'_, AppState>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    db::set_setting(&conn, "gsc_service_account_json", "")?;
    Ok(())
}

/// Faz 5: Ayarlarda "Bağlantıyı test et" — token al + mülk erişimini doğrula.
#[tauri::command]
pub async fn test_gsc_credentials(state: State<'_, AppState>) -> Result<String, String> {
    let (json, site) = {
        let conn = state.conn.lock().unwrap();
        (
            db::get_setting(&conn, "gsc_service_account_json")?.unwrap_or_default(),
            db::get_setting(&conn, "gsc_site_url")?.unwrap_or_default(),
        )
    };
    if json.trim().is_empty() {
        return Err("Önce bir service-account JSON dosyası yükleyin.".to_string());
    }
    seo_data::gsc::test(&json, &site).await
}

#[tauri::command]
pub fn set_theme(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    theme: String,
) -> Result<(), String> {
    {
        let conn = state.conn.lock().unwrap();
        db::set_setting(&conn, "theme", &theme)?;
    }
    // Pencere çerçevesi de temayla birlikte değişsin: koyu temada açık bir başlık çubuğu
    // uygulamanın "içine yapıştırılmış" gibi durmasına yol açıyordu (kullanıcı isteği).
    apply_window_theme(&app, Some(theme.as_str()));
    Ok(())
}

/// Pencere çerçevesini (başlık çubuğu, zemin, yazı) uygulama temasına uydurur.
///
/// ⚠️ Ön yüzden DEĞİL buradan: aynı fonksiyon açılışta da çağrılıyor (bkz. `lib.rs`), böylece
/// pencere ilk karede doğru renkte açılıyor. JS'ten yapılsaydı açık çerçeveyle açılıp bir kare
/// sonra koyuya dönerdi.
///
/// # 🔬 Üç çağrı da gerekli — çalışan uygulamada ölçüldü (2026-08-08)
///
/// Bu iki adımda ve iki ayrı yanılgıyla bulundu:
///
/// 1. **Yalnızca `set_theme` hiçbir şey yapmadı.** Başlık çubuğu koyu temada bembeyaz kaldı.
///    Tanılama şaşırtıcıydı: pencere bulunuyor, `set_theme` `Ok` dönüyor, `w.theme()` geri
///    `Dark` okuyor. Yani gördüğümüz beyaz **NSAppearance değil, pencerenin ZEMİN rengiydi** —
///    macOS başlık çubuğunu pencere zemininin üstüne çiziyor ve zemin varsayılanı beyaz.
///    → `set_background_color` eklendi, çubuk anında koyuya döndü.
/// 2. **Ama başlık YAZISI koyu kaldı** ve koyu zeminde kayboldu (kullanıcı yakaladı).
///    Sebep: Tauri'nin `set_theme`i macOS'te `NSApp.appearance`ı değiştiriyor, **pencerenin
///    kendi görünümünü değil**. Pencere Aqua'da kaldığı için macOS başlığı koyu mürekkeple
///    çiziyordu. → Görünüm doğrudan `NSWindow`a uygulanıyor ([`ns_appearance`]).
/// 3. **Görünüm doğru olduğu hâlde başlık hâlâ koyu çizildi.** Ölçüm: `win.appearance()` ve
///    `effectiveAppearance()` ikisi de `NSAppearanceNameDarkAqua` okuyor — yani sorun bizde
///    değil, macOS'in bu pencere için başlığı çizme biçiminde. → Yerel başlık YAZISI
///    gizlendi (`hiddenTitle: true`, `tauri.conf.json`). Zaten gereksizdi: uygulama adı
///    kenar çubuğunun tepesinde duruyor. Kalan çubuk temayla uyumlu ve boş.
///
/// Zemin renkleri `styles.css`teki `--c-bg` ile aynı: açık `#ffffff`, koyu `#1c1c1e`.
/// ⚠️ Orası değişirse burası da değişmeli.
///
/// `theme` `None` ise **sisteme bırakılır**: ilk kurulumda kayıtlı tema yok ve boş değeri
/// "light" saymak, macOS'i koyu modda kullanan yeni bir kullanıcının penceresini zorla açık
/// yapardı — sistem ayarını ezmek bizim işimiz değil.
///
/// Sessiz başarısız oluyor: tema kozmetik, pencere bulunamadı diye ayar kaydı düşmemeli.
pub fn apply_window_theme(app: &tauri::AppHandle, theme: Option<&str>) {
    use tauri::{window::Color, Manager, Theme};
    let Some(w) = app.get_webview_window("main") else { return };
    let koyu = match theme {
        Some("dark") => true,
        Some("light") => false,
        _ => return, // sisteme bırak
    };

    let _ = w.set_theme(Some(if koyu { Theme::Dark } else { Theme::Light }));
    let _ = w.set_background_color(Some(if koyu {
        Color(0x1c, 0x1c, 0x1e, 0xff)
    } else {
        Color(0xff, 0xff, 0xff, 0xff)
    }));
    ns_appearance(&w, koyu);
}

/// Pencerenin **kendi** `NSAppearance`ını ayarlar — başlık yazısının rengi buna bağlı.
///
/// Tauri'nin `set_theme`i uygulama düzeyinde çalıştığı için pencere Aqua'da kalıyor ve macOS
/// başlığı koyu mürekkeple çiziyor; koyu zeminde okunmuyor (bkz. [`apply_window_theme`]).
#[cfg(target_os = "macos")]
fn ns_appearance(w: &tauri::WebviewWindow, koyu: bool) {
    use objc2::rc::Retained;
    use objc2_app_kit::{
        NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
        NSWindow,
    };

    let Ok(ptr) = w.ns_window() else { return };
    if ptr.is_null() {
        return;
    }
    let ad = if koyu { unsafe { NSAppearanceNameDarkAqua } } else { unsafe { NSAppearanceNameAqua } };
    let Some(gorunum): Option<Retained<NSAppearance>> = NSAppearance::appearanceNamed(ad) else {
        return;
    };
    // SAFETY: `ns_window()` geçerli bir NSWindow işaretçisi döndürüyor ve bu çağrı ana
    // iş parçacığından yapılıyor (Tauri komutları ve `setup` ana iş parçacığında koşuyor).
    unsafe {
        let win: &NSWindow = &*(ptr as *mut NSWindow);
        win.setAppearance(Some(&gorunum));
    }
}

#[cfg(not(target_os = "macos"))]
fn ns_appearance(_w: &tauri::WebviewWindow, _koyu: bool) {}


#[tauri::command]
pub async fn test_feed_url(url: String) -> Result<i64, String> {
    let u = url.trim();
    if !u.starts_with("http://") && !u.starts_with("https://") {
        return Err("Geçerli bir URL girin (http/https).".to_string());
    }
    let items = feed::fetch_and_parse(u).await?;
    Ok(items.len() as i64)
}

#[tauri::command]
pub async fn test_gemini_key(key: String) -> Result<String, String> {
    gemini::test_key(&key).await
}

#[tauri::command]
pub fn export_db(state: State<'_, AppState>, path: String, format: String) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    match format.as_str() {
        "db" => {
            let mut dest = Connection::open(&path)
                .map_err(|e| format!("Hedef dosya açılamadı: {e}"))?;
            let backup = rusqlite::backup::Backup::new(&conn, &mut dest)
                .map_err(|e| format!("Yedekleme başlatılamadı: {e}"))?;
            backup
                .run_to_completion(100, std::time::Duration::from_millis(5), None)
                .map_err(|e| format!("Yedekleme tamamlanamadı: {e}"))?;
        }
        "json" => {
            let json = export_json(&conn)?;
            std::fs::write(&path, json).map_err(|e| format!("Dosya yazılamadı: {e}"))?;
        }
        other => return Err(format!("Bilinmeyen format: {other}")),
    }
    db::set_setting(&conn, "last_backup_at", &now_str())?;
    Ok(())
}

fn export_json(conn: &Connection) -> Result<String, String> {
    use serde_json::{json, Value};
    fn dump(conn: &Connection, table: &str, cols: &[&str]) -> Result<Vec<Value>, String> {
        let sql = format!("SELECT {} FROM {}", cols.join(", "), table);
        let mut stmt = conn.prepare(&sql).map_err(|e| format!("{table} okunamadı: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                let mut obj = serde_json::Map::new();
                for (i, col) in cols.iter().enumerate() {
                    let v: Option<rusqlite::types::Value> = row.get(i)?;
                    let jv = match v {
                        Some(rusqlite::types::Value::Text(s)) => Value::String(s),
                        Some(rusqlite::types::Value::Integer(n)) => Value::from(n),
                        Some(rusqlite::types::Value::Real(f)) => Value::from(f),
                        _ => Value::Null,
                    };
                    obj.insert((*col).to_string(), jv);
                }
                Ok(Value::Object(obj))
            })
            .map_err(|e| format!("{table} okunamadı: {e}"))?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }
    let products = dump(
        conn,
        "products",
        &[
            "sku", "id", "name", "brand", "main_category", "category", "quantity", "url",
            "img_url", "title", "descriptions", "keywords", "search_keywords", "details",
            "last_synced_at", "picture2", "picture3", "picture4",
            // ⚠️ feed_fp yedeğe DAHİL olmak zorunda: geri yüklemede boş kalırsa ilk senkronda
            // her onaylı ürün "feed değişti" diye yanlış bayraklanır (iz ↔ damga ayrışması).
            "feed_fp", "feed_changed",
        ],
    )?;
    // Not: draft_details/research_json/tech_* alanları da yedeklenir. Teknik tablo feed'de YOK,
    // yani yedekte yoksa geri getirilemez (gerçek emek kaybı).
    let seo = dump(
        conn,
        "seo_status",
        &[
            "sku", "meta_status", "details_status", "target_keyword", "draft_title",
            "draft_descriptions", "draft_keywords", "draft_search_keywords", "updated_at",
            "draft_details", "research_json", "image_check_json", "image_check_fp",
            "tech_source_text", "tech_specs_json", "tech_status", "tech_history_json",
            "ideasoft_product_id", "ideasoft_pushed_at", "ideasoft_seo_rule",
            "meta_model", "details_model", "tech_model",
            "meta_history_json", "details_history_json",
            // Onay damgası + o anki alan değerleri; feed_fp ile birlikte anlam taşır.
            "reviewed_fp", "reviewed_facts_json",
        ],
    )?;
    let log = dump(
        conn,
        "sync_log",
        &["run_at", "active", "added", "updated", "deleted", "duplicate_skipped"],
    )?;
    let settings = dump(conn, "settings", &["key", "value"])?;
    // ⚠️ Ölçüm omurgası (Faz Ö). `work_events` YENİDEN ÜRETİLEMEZ — hangi işi ne zaman
    // yaptığımızın tek kaydı. Anlık görüntüler 16 aya kadar GSC'den tazelenebilir ama yine
    // de taşınıyor: kısmi yedek "geri yükledim ama eksik" sınıfı bir sürpriz üretir (K2 dersi).
    let events = dump(
        conn,
        "work_events",
        &["id", "at", "sku", "url", "kind", "reaches_store", "payload_json"],
    )?;
    let snaps = dump(
        conn,
        "metric_snapshots",
        &["id", "captured_at", "window_start", "window_end", "source", "rows", "clicks", "impressions"],
    )?;
    let snap_rows = dump(
        conn,
        "metric_page_rows",
        &["snapshot_id", "url", "sku", "clicks", "impressions", "position"],
    )?;
    // ⚠️ Sohbet geçmişi yedeğe DAHİL. Teknik tablo gibi yeniden üretilemeyen kullanıcı emeği;
    // yedekte olmazsa geri yüklemede sessizce kaybolur. (`ideasoft_catalog` bilinçli olarak
    // yedeklenmiyor — o tek komutla yeniden çekiliyor.)
    let chats = dump(
        conn,
        "chat_sessions",
        &["id", "title", "tool_page", "messages_json", "model", "created_at", "updated_at"],
    )?;
    // 🔴 CRM (Faz C) — yedeğe DAHİL. Kişi kayıtları yeniden üretilemez: feed'den gelmiyor,
    // GSC'den tazelenmiyor, tamamen kullanıcının emeği. Dışarıda bırakmak K2'de öğrenilen
    // "geri yükledim ama eksik" sürprizinin en ağır hâli olurdu.
    // ⚠️ Bu tablolarla birlikte yedek dosyası KİŞİSEL VERİ taşıyor; Ayarlar ekranı bunu
    // kullanıcıya açıkça söylüyor.
    let contacts = dump(
        conn,
        "contacts",
        &[
            "id", "name", "company", "email", "phone", "channel", "note", "last_contact_at",
            "next_step_at", "next_step_note", "created_at", "updated_at", "archived",
        ],
    )?;
    let contact_events = dump(conn, "contact_events", &["id", "contact_id", "at", "kind", "note"])?;
    let contact_tags = dump(conn, "contact_tags", &["contact_id", "tag"])?;
    let contact_products = dump(conn, "contact_products", &["contact_id", "sku", "at"])?;

    // Teklif (Faz T) — yeniden üretilemez: teklif bir KAYIT, katalogdan türetilemiyor
    // (fiyat, maliyet ve kur o günkü hâliyle donmuş durumda).
    let quotes = dump(
        conn,
        "quotes",
        &[
            "id", "no", "contact_id", "status", "currency", "fx_rate", "fx_date", "valid_until",
            "note", "created_at", "updated_at", "sent_at", "closed_at", "close_reason",
        ],
    )?;
    let quote_items = dump(
        conn,
        "quote_items",
        &["id", "quote_id", "sku", "name", "qty", "unit_price", "tax_rate", "cost", "sort"],
    )?;
    let quote_versions =
        dump(conn, "quote_versions", &["id", "quote_id", "version", "snapshot_json", "at"])?;

    let root = json!({
        "app": "seo-yoneticisi",
        "exported_at": now_str(),
        "products": products,
        "seo_status": seo,
        "sync_log": log,
        "settings": settings,
        "chat_sessions": chats,
        "work_events": events,
        "metric_snapshots": snaps,
        "metric_page_rows": snap_rows,
        "contacts": contacts,
        "contact_events": contact_events,
        "contact_tags": contact_tags,
        "contact_products": contact_products,
        "quotes": quotes,
        "quote_items": quote_items,
        "quote_versions": quote_versions,
    });
    serde_json::to_string_pretty(&root).map_err(|e| format!("JSON oluşturulamadı: {e}"))
}

#[tauri::command]
pub fn import_db(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let mut conn = state.conn.lock().unwrap();
    let lower = path.to_lowercase();
    if lower.ends_with(".json") {
        let text = std::fs::read_to_string(&path).map_err(|e| format!("Dosya okunamadı: {e}"))?;
        import_json(&mut conn, &text)
    } else {
        // .db: kaynak dosyadan mevcut bağlantının üzerine geri yükle
        let src = Connection::open(&path).map_err(|e| format!("Yedek dosyası açılamadı: {e}"))?;
        src.query_row("SELECT COUNT(*) FROM sqlite_master WHERE name='products'", [], |r| {
            r.get::<_, i64>(0)
        })
        .ok()
        .filter(|n| *n > 0)
        .ok_or("Bu dosya geçerli bir SEO Yöneticisi yedeği değil.")?;
        {
            let backup = rusqlite::backup::Backup::new(&src, &mut conn)
                .map_err(|e| format!("Geri yükleme başlatılamadı: {e}"))?;
            backup
                .run_to_completion(100, std::time::Duration::from_millis(5), None)
                .map_err(|e| format!("Geri yükleme tamamlanamadı: {e}"))?;
        }
        db::init(&conn)?; // eksik tablo/pragma varsa tamamla
        Ok(())
    }
}

fn import_json(conn: &mut Connection, text: &str) -> Result<(), String> {
    use serde_json::Value;
    let root: Value = serde_json::from_str(text).map_err(|e| format!("JSON çözümlenemedi: {e}"))?;
    let obj = root.as_object().ok_or("Beklenmeyen JSON biçimi.")?;
    if !obj.contains_key("products") {
        return Err("Bu dosya geçerli bir SEO Yöneticisi yedeği değil.".to_string());
    }
    let tx = conn.transaction().map_err(|e| format!("İşlem başlatılamadı: {e}"))?;
    tx.execute_batch(
        "DELETE FROM seo_status; DELETE FROM products; DELETE FROM sync_log; DELETE FROM settings;
         DELETE FROM work_events; DELETE FROM metric_page_rows; DELETE FROM metric_snapshots;
         DELETE FROM contact_products; DELETE FROM contact_tags; DELETE FROM contact_events;
         DELETE FROM quote_versions; DELETE FROM quote_items; DELETE FROM quotes;
         DELETE FROM contacts;",
    )
    .map_err(|e| format!("Mevcut veriler temizlenemedi: {e}"))?;

    fn s(v: &Value, key: &str) -> Option<String> {
        v.get(key).and_then(|x| x.as_str()).map(String::from)
    }
    fn i(v: &Value, key: &str) -> Option<i64> {
        v.get(key).and_then(|x| x.as_i64())
    }
    fn f(v: &Value, key: &str) -> f64 {
        v.get(key).and_then(|x| x.as_f64()).unwrap_or(0.0)
    }
    /// Boş bırakılabilen sayısal alan.
    ///
    /// 🔴 `f()` KULLANILAMAZ: eksik değeri 0.0 yapıyor. Teklif satırında maliyet `NULL` ise
    /// bu "elle satır, maliyeti bilinmiyor" demek; 0.0'a düşerse geri yüklemede o satır
    /// **%100 marjlı** görünür ve teklifin kârı olduğundan yüksek okunur.
    fn fo(v: &Value, key: &str) -> Option<f64> {
        v.get(key).and_then(|x| x.as_f64())
    }
    let arr = |key: &str| -> Vec<Value> {
        obj.get(key).and_then(|x| x.as_array()).cloned().unwrap_or_default()
    };

    for p in arr("products") {
        tx.execute(
            "INSERT OR REPLACE INTO products (sku, id, name, brand, main_category, category,
               quantity, url, img_url, title, descriptions, keywords, search_keywords, details, last_synced_at,
               picture2, picture3, picture4, feed_fp, feed_changed)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
            params![
                s(&p, "sku"), s(&p, "id"), s(&p, "name").unwrap_or_default(), s(&p, "brand"),
                s(&p, "main_category"), s(&p, "category"), i(&p, "quantity"), s(&p, "url"),
                s(&p, "img_url"), s(&p, "title"), s(&p, "descriptions"), s(&p, "keywords"),
                s(&p, "search_keywords"), s(&p, "details"), s(&p, "last_synced_at"),
                s(&p, "picture2"), s(&p, "picture3"), s(&p, "picture4"),
                s(&p, "feed_fp"), s(&p, "feed_changed"),
            ],
        )
        .map_err(|e| format!("Ürün geri yüklenemedi: {e}"))?;
    }
    for r in arr("seo_status") {
        tx.execute(
            "INSERT OR REPLACE INTO seo_status (sku, meta_status, details_status, target_keyword,
               draft_title, draft_descriptions, draft_keywords, draft_search_keywords, updated_at,
               draft_details, research_json, image_check_json, image_check_fp,
               tech_source_text, tech_specs_json, tech_status, tech_history_json,
               ideasoft_product_id, ideasoft_pushed_at, ideasoft_seo_rule,
               meta_model, details_model, tech_model,
               meta_history_json, details_history_json, reviewed_fp, reviewed_facts_json)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,
                     ?21,?22,?23,?24,?25,?26,?27)",
            params![
                s(&r, "sku"),
                s(&r, "meta_status").unwrap_or_else(|| "pending".into()),
                s(&r, "details_status").unwrap_or_else(|| "pending".into()),
                s(&r, "target_keyword"), s(&r, "draft_title"), s(&r, "draft_descriptions"),
                s(&r, "draft_keywords"), s(&r, "draft_search_keywords"), s(&r, "updated_at"),
                s(&r, "draft_details"), s(&r, "research_json"), s(&r, "image_check_json"),
                s(&r, "image_check_fp"), s(&r, "tech_source_text"), s(&r, "tech_specs_json"),
                s(&r, "tech_status").unwrap_or_else(|| "pending".into()),
                s(&r, "tech_history_json"), i(&r, "ideasoft_product_id"),
                s(&r, "ideasoft_pushed_at"), i(&r, "ideasoft_seo_rule"),
                s(&r, "meta_model"), s(&r, "details_model"), s(&r, "tech_model"),
                s(&r, "meta_history_json"), s(&r, "details_history_json"),
                s(&r, "reviewed_fp"), s(&r, "reviewed_facts_json"),
            ],
        )
        .map_err(|e| format!("SEO durumu geri yüklenemedi: {e}"))?;
    }
    for l in arr("sync_log") {
        tx.execute(
            "INSERT INTO sync_log (run_at, active, added, updated, deleted, duplicate_skipped)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                s(&l, "run_at"), i(&l, "active"), i(&l, "added"), i(&l, "updated"),
                i(&l, "deleted"), i(&l, "duplicate_skipped"),
            ],
        )
        .map_err(|e| format!("Senkron geçmişi geri yüklenemedi: {e}"))?;
    }
    for kv in arr("settings") {
        if let (Some(k), Some(v)) = (s(&kv, "key"), s(&kv, "value")) {
            tx.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                params![k, v],
            )
            .map_err(|e| format!("Ayar geri yüklenemedi: {e}"))?;
        }
    }
    // Sohbet geçmişi. Eski yedeklerde bu bölüm YOK — `arr()` boş liste döndürdüğü için
    // döngü hiç dönmez ve geri yükleme kırılmaz.
    for c in arr("chat_sessions") {
        tx.execute(
            "INSERT OR REPLACE INTO chat_sessions
               (id, title, tool_page, messages_json, model, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                c.get("id").and_then(|v| v.as_i64()),
                s(&c, "title").unwrap_or_default(),
                s(&c, "tool_page"),
                s(&c, "messages_json").unwrap_or_else(|| "[]".into()),
                s(&c, "model"),
                s(&c, "created_at").unwrap_or_default(),
                s(&c, "updated_at").unwrap_or_default(),
            ],
        )
        .map_err(|e| format!("Sohbet geri yüklenemedi: {e}"))?;
    }
    // Ölçüm omurgası. Sıra önemli: satırlar anlık görüntüye yabancı anahtarla bağlı.
    for e in arr("work_events") {
        let _ = tx.execute(
            "INSERT OR REPLACE INTO work_events (id, at, sku, url, kind, reaches_store, payload_json)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                i(&e, "id"), s(&e, "at"), s(&e, "sku"), s(&e, "url"), s(&e, "kind"),
                i(&e, "reaches_store").unwrap_or(0), s(&e, "payload_json")
            ],
        );
    }
    // CRM. ⚠️ Sıra önemli: alt tablolar `contacts.id`e yabancı anahtarla bağlı.
    for c in arr("contacts") {
        let _ = tx.execute(
            "INSERT OR REPLACE INTO contacts (id, name, company, email, phone, channel, note,
                last_contact_at, next_step_at, next_step_note, created_at, updated_at, archived)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                i(&c, "id"),
                s(&c, "name").unwrap_or_default(),
                s(&c, "company").unwrap_or_default(),
                s(&c, "email").unwrap_or_default(),
                s(&c, "phone").unwrap_or_default(),
                s(&c, "channel").unwrap_or_default(),
                s(&c, "note").unwrap_or_default(),
                s(&c, "last_contact_at"),
                s(&c, "next_step_at"),
                s(&c, "next_step_note").unwrap_or_default(),
                s(&c, "created_at").unwrap_or_default(),
                s(&c, "updated_at").unwrap_or_default(),
                i(&c, "archived").unwrap_or(0)
            ],
        );
    }
    for e in arr("contact_events") {
        let _ = tx.execute(
            "INSERT OR REPLACE INTO contact_events (id, contact_id, at, kind, note)
             VALUES (?1,?2,?3,?4,?5)",
            params![
                i(&e, "id"), i(&e, "contact_id"), s(&e, "at"), s(&e, "kind"),
                s(&e, "note").unwrap_or_default()
            ],
        );
    }
    for t in arr("contact_tags") {
        let _ = tx.execute(
            "INSERT OR IGNORE INTO contact_tags (contact_id, tag) VALUES (?1,?2)",
            params![i(&t, "contact_id"), s(&t, "tag")],
        );
    }
    for cp in arr("contact_products") {
        let _ = tx.execute(
            "INSERT OR IGNORE INTO contact_products (contact_id, sku, at) VALUES (?1,?2,?3)",
            params![i(&cp, "contact_id"), s(&cp, "sku"), s(&cp, "at")],
        );
    }
    // Teklif. ⚠️ Sıra: satırlar ve sürümler `quotes.id`e bağlı.
    for q in arr("quotes") {
        let _ = tx.execute(
            "INSERT OR REPLACE INTO quotes (id, no, contact_id, status, currency, fx_rate,
                fx_date, valid_until, note, created_at, updated_at, sent_at, closed_at,
                close_reason)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                i(&q, "id"), s(&q, "no"), i(&q, "contact_id"),
                s(&q, "status").unwrap_or_else(|| "draft".into()),
                s(&q, "currency").unwrap_or_else(|| "USD".into()),
                fo(&q, "fx_rate"), s(&q, "fx_date"), s(&q, "valid_until"),
                s(&q, "note").unwrap_or_default(), s(&q, "created_at").unwrap_or_default(),
                s(&q, "updated_at").unwrap_or_default(), s(&q, "sent_at"), s(&q, "closed_at"),
                s(&q, "close_reason").unwrap_or_default()
            ],
        );
    }
    for it in arr("quote_items") {
        let _ = tx.execute(
            "INSERT OR REPLACE INTO quote_items (id, quote_id, sku, name, qty, unit_price,
                tax_rate, cost, sort)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                i(&it, "id"), i(&it, "quote_id"), s(&it, "sku"),
                s(&it, "name").unwrap_or_default(), fo(&it, "qty").unwrap_or(1.0),
                f(&it, "unit_price"), f(&it, "tax_rate"),
                fo(&it, "cost"), i(&it, "sort").unwrap_or(0)
            ],
        );
    }
    for v in arr("quote_versions") {
        let _ = tx.execute(
            "INSERT OR REPLACE INTO quote_versions (id, quote_id, version, snapshot_json, at)
             VALUES (?1,?2,?3,?4,?5)",
            params![
                i(&v, "id"), i(&v, "quote_id"), i(&v, "version"),
                s(&v, "snapshot_json").unwrap_or_default(), s(&v, "at").unwrap_or_default()
            ],
        );
    }
    for sn in arr("metric_snapshots") {
        let _ = tx.execute(
            "INSERT OR REPLACE INTO metric_snapshots
               (id, captured_at, window_start, window_end, source, rows, clicks, impressions)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                i(&sn, "id"), s(&sn, "captured_at"), s(&sn, "window_start"), s(&sn, "window_end"),
                s(&sn, "source").unwrap_or_else(|| "gsc".into()),
                i(&sn, "rows").unwrap_or(0), f(&sn, "clicks"), f(&sn, "impressions")
            ],
        );
    }
    for r in arr("metric_page_rows") {
        let _ = tx.execute(
            "INSERT OR REPLACE INTO metric_page_rows
               (snapshot_id, url, sku, clicks, impressions, position)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                i(&r, "snapshot_id"), s(&r, "url"), s(&r, "sku"),
                f(&r, "clicks"), f(&r, "impressions"), f(&r, "position")
            ],
        );
    }

    tx.commit().map_err(|e| format!("İşlem tamamlanamadı: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 🔴 **Teklif de yeniden üretilemez** (Faz T): katalogdan türetilemiyor çünkü fiyat,
    /// maliyet ve kur o günkü hâliyle DONMUŞ. Ayrıca maliyeti olmayan (elle) satırın
    /// `NULL`u korunmalı — 0.0'a düşerse teklif %100 marjlı görünür.
    #[test]
    fn yedek_teklifi_ve_bos_maliyeti_koruyor() {
        let src = Connection::open_in_memory().unwrap();
        seo_core::db::init(&src).unwrap();
        src.execute(
            "INSERT INTO quotes (id, no, status, currency, fx_rate, created_at, updated_at)
             VALUES (3,'T-2026-009','sent','TRY',47.5911,'2026-08-10T09:00:00','2026-08-10T09:00:00')",
            [],
        )
        .unwrap();
        src.execute(
            "INSERT INTO quote_items (quote_id, sku, name, qty, unit_price, tax_rate, cost, sort)
             VALUES (3,'A-1','Katalog ürünü',2,949.0,20.0,860.0,1),
                    (3,NULL,'Kurulum',1,150.0,20.0,NULL,2)",
            [],
        )
        .unwrap();

        let json = export_json(&src).expect("dışa aktarılamadı");
        let mut dst = Connection::open_in_memory().unwrap();
        seo_core::db::init(&dst).unwrap();
        import_json(&mut dst, &json).expect("içe aktarılamadı");

        let (no, kur): (String, Option<f64>) = dst
            .query_row("SELECT no, fx_rate FROM quotes WHERE id=3", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .expect("teklif geri yüklenmedi");
        assert_eq!(no, "T-2026-009");
        assert_eq!(kur, Some(47.5911), "kur teklifin kaydının parçası");

        let maliyetli: Option<f64> = dst
            .query_row("SELECT cost FROM quote_items WHERE sku='A-1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(maliyetli, Some(860.0));

        let elle: Option<f64> = dst
            .query_row("SELECT cost FROM quote_items WHERE sku IS NULL", [], |r| r.get(0))
            .unwrap();
        assert_eq!(elle, None, "elle satırın maliyeti 0'a DÜŞMEMELİ — marj bozulur");
    }

    /// 🔴 **Kişi kayıtları da yeniden üretilemez** (Faz C): feed'den gelmiyor, GSC'den
    /// tazelenmiyor — tamamen kullanıcının emeği. Dört tablo birlikte taşınmalı, yoksa
    /// "kişi geri geldi ama temas geçmişi ve ürün bağları gitti" olur.
    #[test]
    fn yedek_crm_verisini_dort_tabloyla_tasiyor() {
        let src = Connection::open_in_memory().unwrap();
        seo_core::db::init(&src).unwrap();
        src.execute(
            "INSERT INTO contacts (id, name, company, email, next_step_at, next_step_note,
                created_at, updated_at) VALUES (7,'Ahmet Yılmaz','Kurumsal BT','a@b.com',
                '2026-08-20','fiyat ver','2026-08-01T09:00:00','2026-08-01T09:00:00')",
            [],
        )
        .unwrap();
        src.execute(
            "INSERT INTO contact_events (contact_id, at, kind, note)
             VALUES (7,'2026-08-01T09:00:00','call','fiyat sordu')",
            [],
        )
        .unwrap();
        src.execute("INSERT INTO contact_tags (contact_id, tag) VALUES (7,'sunucu')", []).unwrap();
        src.execute(
            "INSERT INTO contact_products (contact_id, sku, at) VALUES (7,'A-1','2026-08-01T09:00:00')",
            [],
        )
        .unwrap();

        let json = export_json(&src).expect("dışa aktarılamadı");
        let mut dst = Connection::open_in_memory().unwrap();
        seo_core::db::init(&dst).unwrap();
        import_json(&mut dst, &json).expect("içe aktarılamadı");

        let (ad, adim): (String, Option<String>) = dst
            .query_row("SELECT name, next_step_at FROM contacts WHERE id=7", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .expect("kişi geri yüklenmedi");
        assert_eq!(ad, "Ahmet Yılmaz");
        assert_eq!(adim.as_deref(), Some("2026-08-20"), "sonraki adım kuyruğu besliyor, kaybolamaz");

        let temas: String = dst
            .query_row("SELECT note FROM contact_events WHERE contact_id=7", [], |r| r.get(0))
            .expect("temas geçmişi geri yüklenmedi");
        assert_eq!(temas, "fiyat sordu");

        let etiket: String = dst
            .query_row("SELECT tag FROM contact_tags WHERE contact_id=7", [], |r| r.get(0))
            .expect("etiket geri yüklenmedi");
        assert_eq!(etiket, "sunucu");

        let sku: String = dst
            .query_row("SELECT sku FROM contact_products WHERE contact_id=7", [], |r| r.get(0))
            .expect("ürün bağı geri yüklenmedi");
        assert_eq!(sku, "A-1");
    }

    /// 🔴 **`work_events` yeniden ÜRETİLEMEZ.** Hangi işi ne zaman yaptığımızın tek kaydı;
    /// yedekte taşınmazsa geri yüklemede "işe yaradı mı?" sorusu kalıcı olarak cevapsız kalır.
    /// Anlık görüntüler GSC'den tazelenebilir ama kısmi yedek "geri yükledim ama eksik"
    /// sınıfı bir sürpriz üretir — üçü birden taşınıyor.
    #[test]
    fn yedek_olcum_omurgasini_tasiyor() {
        let src = Connection::open_in_memory().unwrap();
        seo_core::db::init(&src).unwrap();
        src.execute("INSERT INTO products (sku, name, url) VALUES ('A-1','Ürün','https://x/a')", [])
            .unwrap();
        src.execute(
            "INSERT INTO work_events (at, sku, url, kind, reaches_store)
             VALUES ('2026-06-01T10:00:00','A-1','https://x/a','ideasoft_push',1)",
            [],
        )
        .unwrap();
        src.execute(
            "INSERT INTO metric_snapshots (captured_at, window_start, window_end, rows, clicks, impressions)
             VALUES ('2026-07-01T00:00:00','2026-06-02','2026-06-30',1,42.0,900.0)",
            [],
        )
        .unwrap();
        src.execute(
            "INSERT INTO metric_page_rows (snapshot_id, url, sku, clicks, impressions, position)
             VALUES (1,'https://x/a','A-1',42.0,900.0,6.5)",
            [],
        )
        .unwrap();

        let json = export_json(&src).expect("dışa aktarılamadı");
        let mut dst = Connection::open_in_memory().unwrap();
        seo_core::db::init(&dst).unwrap();
        import_json(&mut dst, &json).expect("içe aktarılamadı");

        let (kind, reaches): (String, i64) = dst
            .query_row("SELECT kind, reaches_store FROM work_events WHERE sku='A-1'", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .expect("olay geri yüklenmedi");
        assert_eq!(kind, "ideasoft_push");
        assert_eq!(reaches, 1, "ölçülebilirlik bayrağı kayboldu");

        let pencere: String = dst
            .query_row("SELECT window_start FROM metric_snapshots", [], |r| r.get(0))
            .expect("anlık görüntü geri yüklenmedi");
        assert_eq!(pencere, "2026-06-02");

        let tik: f64 = dst
            .query_row("SELECT clicks FROM metric_page_rows WHERE url='https://x/a'", [], |r| r.get(0))
            .expect("sayfa satırı geri yüklenmedi");
        assert_eq!(tik, 42.0);
    }

    /// 🔴 Yedekte `feed_fp`/`reviewed_fp` taşınmazsa oluşan hasar sessiz DEĞİL, gürültülü:
    /// geri yüklemeden sonraki ilk senkronda iz yeniden hesaplanır, damga ile ayrışır ve
    /// **onaylanmış her ürün "feed verisi değişti" diye yanlış bayraklanır.** Kullanıcı
    /// kataloğun tamamını gözden geçirmeye çağrılır; bir kez olduğunda bayrağa güven biter.
    #[test]
    fn yedek_parmak_izi_ve_onay_damgasini_tasiyor() {
        let src = Connection::open_in_memory().unwrap();
        seo_core::db::init(&src).unwrap();
        src.execute(
            "INSERT INTO products (sku, name, feed_fp) VALUES ('ABC-1', 'Ürün', 'a1b2c3d4')",
            [],
        )
        .unwrap();
        src.execute(
            "INSERT INTO seo_status (sku, meta_status, details_status, reviewed_fp,
                                     reviewed_facts_json)
             VALUES ('ABC-1', 'done', 'done', 'a1b2c3d4',
                     '{\"name\":\"Ürün\",\"brand\":\"\",\"main_category\":\"\",\"category\":\"\",\"details\":\"eski\",\"images\":[]}')",
            [],
        )
        .unwrap();

        let json = export_json(&src).expect("dışa aktarılamadı");
        let mut dst = Connection::open_in_memory().unwrap();
        seo_core::db::init(&dst).unwrap();
        import_json(&mut dst, &json).expect("içe aktarılamadı");

        let (fp, reviewed): (Option<String>, Option<String>) = dst
            .query_row(
                "SELECT p.feed_fp, s.reviewed_fp FROM products p
                 JOIN seo_status s ON s.sku = p.sku WHERE p.sku = 'ABC-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("ürün geri yüklenmedi");
        assert_eq!(fp.as_deref(), Some("a1b2c3d4"), "parmak izi yedekte taşınmadı");
        assert_eq!(reviewed.as_deref(), Some("a1b2c3d4"), "onay damgası yedekte taşınmadı");
        // ⚠️ Karşılaştırma kaydı da taşınmalı: yedekten dönen kullanıcı "ne değişti?"
        // sorusunun cevabını kaybetmemeli — bu veri feed'den yeniden üretilemez.
        let snap: Option<String> = dst
            .query_row("SELECT reviewed_facts_json FROM seo_status WHERE sku='ABC-1'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(
            snap.as_deref().unwrap_or("").contains("eski"),
            "onay anındaki değerler yedekte taşınmadı: {snap:?}"
        );
        // Asıl korunan davranış: geri yüklemenin hemen ardından bayrak YOK.
        assert_eq!(feed_change_note(fp, reviewed, None), None, "geri yükleme yanlış bayrak üretti");
    }

    /// ⚠️ Bu testin koruduğu risk somut: yedekleme `products`/`seo_status`/`sync_log`/
    /// `settings` tablolarını elle sayıyor. Yeni bir tablo eklenip buraya YAZILMAZSA
    /// içeriği geri yüklemede SESSİZCE kaybolur — hata da vermez. Sohbet geçmişi teknik
    /// tablo gibi yeniden üretilemeyen kullanıcı emeği olduğu için yolculuk teste bağlandı.
    #[test]
    fn yedek_sohbet_gecmisini_tasiyor() {
        let src = Connection::open_in_memory().unwrap();
        seo_core::db::init(&src).unwrap();
        src.execute(
            "INSERT INTO chat_sessions (title, tool_page, messages_json, model, created_at, updated_at)
             VALUES ('en acil üç iş', 'opportunities',
                     '[{\"role\":\"user\",\"text\":\"soru\"},{\"role\":\"model\",\"text\":\"cevap\"}]',
                     'gemma-4-31b-it', '2026-07-30T10:00', '2026-07-30T10:05')",
            [],
        )
        .unwrap();

        let json = export_json(&src).expect("dışa aktarılamadı");
        assert!(json.contains("chat_sessions"), "yedekte sohbet bölümü yok");

        let mut dst = Connection::open_in_memory().unwrap();
        seo_core::db::init(&dst).unwrap();
        import_json(&mut dst, &json).expect("içe aktarılamadı");

        let (title, page, model, msgs): (String, String, String, String) = dst
            .query_row(
                "SELECT title, tool_page, model, messages_json FROM chat_sessions",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .expect("sohbet geri yüklenmedi");
        assert_eq!(title, "en acil üç iş");
        assert_eq!(page, "opportunities");
        assert_eq!(model, "gemma-4-31b-it");
        assert!(msgs.contains("cevap"), "mesaj gövdeleri kayboldu: {msgs}");
    }

    /// Sohbet bölümü olmayan ESKİ yedekler kırılmadan geri yüklenebilmeli.
    #[test]
    fn eski_yedek_sohbet_bolumu_olmadan_calisir() {
        let mut dst = Connection::open_in_memory().unwrap();
        seo_core::db::init(&dst).unwrap();
        let eski = r#"{"app":"seo-yoneticisi","products":[],"seo_status":[],"sync_log":[],"settings":[]}"#;
        import_json(&mut dst, eski).expect("eski yedek geri yüklenemedi");
        let n: i64 = dst
            .query_row("SELECT count(*) FROM chat_sessions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }
}
