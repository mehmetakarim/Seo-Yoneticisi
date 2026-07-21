mod commands;
mod db;
mod feed;
mod gemini;
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
            commands::get_settings,
            commands::save_settings,
            commands::set_theme,
            commands::test_feed_url,
            commands::test_gemini_key,
            commands::export_db,
            commands::import_db,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
