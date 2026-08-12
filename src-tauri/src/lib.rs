mod commands;

use commands::AppState;
use seo_core::db;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            // Faz 10: güncelleme eklentileri yalnızca masaüstünde
            #[cfg(desktop)]
            {
                app.handle()
                    .plugin(tauri_plugin_updater::Builder::new().build())
                    .expect("updater eklentisi yüklenemedi");
                app.handle()
                    .plugin(tauri_plugin_process::init())
                    .expect("process eklentisi yüklenemedi");
            }

            let dir = app
                .path()
                .app_data_dir()
                .expect("uygulama veri klasörü bulunamadı");
            std::fs::create_dir_all(&dir).expect("veri klasörü oluşturulamadı");
            let db_path = dir.join("seo-yoneticisi.db");
            let conn = db::open(&db_path).expect("veritabanı hazırlanamadı");
            // Uygulama seans açıkken kapatılmışsa o seans ölçüm değil (bkz. focus.rs).
            commands::close_stale_session(&conn);

            // Pencere çerçevesini kayıtlı temayla açılışta hizala — ilk karede doğru renk.
            let kayitli = db::get_setting(&conn, "theme").ok().flatten();
            commands::apply_window_theme(app.handle(), kayitli.as_deref());

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
            commands::mark_feed_reviewed,
            commands::get_feed_diff,
            commands::seed_metric_history,
            commands::get_outcome_summary,
            commands::get_outcome_badges,
            commands::get_product_timeline,
            commands::get_today_queue,
            commands::start_focus_session,
            commands::resolve_focus_item,
            commands::end_focus_session,
            commands::get_focus_state,
            commands::get_focus_calibration,
            commands::set_focus_durations,
            commands::has_lockable_item,
            commands::get_eol_decisions,
            commands::save_eol_decision,
            commands::delete_eol_decision,
            commands::export_redirect_csv,
            // --- CRM ince dilim (Faz C) ---
            commands::list_contacts,
            commands::get_contact,
            commands::save_contact,
            commands::archive_contact,
            commands::get_contact_events,
            commands::add_contact_event,
            commands::list_contact_tags,
            commands::set_contact_tags,
            commands::get_contact_products,
            commands::contacts_of_product,
            commands::link_contact_product,
            commands::unlink_contact_product,
            commands::preview_contact_csv,
            commands::import_contacts_csv,
            commands::get_silence_state,
            commands::set_silence_days,
            // --- Teklif (Faz T) ---
            commands::list_quotes,
            commands::get_quote,
            commands::create_quote,
            commands::save_quote,
            commands::delete_quote,
            commands::add_quote_item_from_catalog,
            commands::add_quote_item_manual,
            commands::update_quote_item,
            commands::delete_quote_item,
            commands::set_quote_status,
            commands::snapshot_quote,
            commands::get_quote_defaults,
            commands::set_quote_defaults,
            commands::render_quote,
            commands::export_quote_html,
            commands::quote_summary,
            commands::quotes_of_contact,
            commands::dismiss_queue_item,
            commands::complete_queue_item,
            commands::restore_queue_item,
            commands::restore_queue_items,
            commands::get_jsonld,
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
            commands::restore_meta_version,
            commands::restore_details_version,
            commands::test_ideasoft,
            commands::ideasoft_preview,
            commands::ideasoft_push,
            commands::ideasoft_pull_keyword,
            commands::research_seo,
            commands::analyze_opportunities,
            commands::get_opportunity_cache,
            commands::suggest_eol_successor,
            commands::sync_ideasoft_catalog,
            commands::lookup_catalog,
            commands::search_live_products,
            commands::assistant_ask,
            commands::list_chat_sessions,
            commands::get_chat_session,
            commands::save_chat_session,
            commands::delete_chat_session,
            commands::delete_all_chat_sessions,
            commands::preview_canonical,
            commands::apply_canonical,
            commands::needs_setup,
            commands::mark_setup_done,
            commands::get_settings,
            commands::save_settings,
            commands::set_gsc_service_account,
            commands::clear_gsc_service_account,
            commands::set_theme,
            commands::test_feed_url,
            commands::test_gemini_key,
            commands::get_model_chains,
            commands::set_model_chains,
            commands::list_gemini_models,
            commands::probe_gemini_model,
            commands::gemini_usage,
            commands::test_capsolver_key,
            commands::test_gsc_credentials,
            commands::export_db,
            commands::import_db,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
