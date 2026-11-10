mod commands;
mod state;

use archiveplayer_core::Service;
use state::AppState;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .setup(|app| {
            let data_dir = std::env::var_os("ARCHIVEPLAYER_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or(app.path().app_data_dir()?);

            log::info!("ArchivePlayer data directory: {}", data_dir.display());
            let service = Service::new(data_dir)?;
            app.manage(AppState::new(service));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth::auth_status,
            commands::auth::register,
            commands::auth::login,
            commands::auth::logout,
            commands::library::get_library,
            commands::library::get_continue_watching,
            commands::library::add_to_library,
            commands::library::remove_from_library,
            commands::library::clear_watch_history,
            commands::library::set_autoplay,
            commands::search::search_archive,
            commands::search::get_category_suggestions,
            commands::playback::get_playback_details,
            commands::playback::save_progress,
            commands::playback::mark_completed,
            commands::settings::get_settings,
            commands::settings::save_tmdb_key,
            commands::settings::clear_tmdb_key,
            commands::settings::test_tmdb_key,
            commands::settings::get_app_info,
            commands::settings::open_external,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ArchivePlayer");
}
