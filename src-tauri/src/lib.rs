mod commands;
mod db;
mod feed;
mod gemini;
mod ideasoft;
mod images;
mod seo_data;
mod sync;
mod validation;

use commands::AppState;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let dir = app
                .path()
                .app_data_dir()
                .expect("uygulama veri klasörü bulunamadı");
            std::fs::create_dir_all(&dir).expect("veri klasörü oluşturulamadı");
            let db_path = dir.join("seo-yoneticisi.db");
            let conn = db::open(&db_path).expect("veritabanı hazırlanamadı");
            app.manage(AppState {
                conn: Mutex::new(conn),
                db_path,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::sync_feed,
            commands::get_last_sync,
            commands::list_products,
            commands::get_product,
            commands::set_target_keyword,
            commands::save_meta_draft,
            commands::mark_meta_done,
            commands::mark_details_done,
            commands::generate_meta,
            commands::generate_details,
            commands::check_images,
            commands::save_tech_source,
            commands::structure_tech_specs,
            commands::save_tech_specs,
            commands::tech_table_html,
            commands::mark_tech_done,
            commands::restore_tech_version,
            commands::test_ideasoft,
            commands::ideasoft_preview,
            commands::ideasoft_push,
            commands::ideasoft_pull_keyword,
            commands::research_seo,
            commands::get_settings,
            commands::save_settings,
            commands::set_gsc_service_account,
            commands::clear_gsc_service_account,
            commands::set_theme,
            commands::test_feed_url,
            commands::test_gemini_key,
            commands::test_capsolver_key,
            commands::test_gsc_credentials,
            commands::export_db,
            commands::import_db,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
