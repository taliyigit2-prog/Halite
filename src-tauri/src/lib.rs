mod audio;
mod bpm;
mod commands;
mod db;
mod downloader;
mod error;
mod lyrics;
mod nowplaying;
mod separation;
mod state;

use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let app_state = Arc::new(state::AppState::new(data_dir)?);
            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_models,
            commands::install_model,
            commands::delete_model,
            commands::separate,
            commands::cancel_separation,
            commands::list_jobs,
            commands::delete_job,
            commands::save_preset,
            commands::list_presets,
            commands::delete_preset,
            commands::pick_audio_files,
            commands::pick_folder,
            commands::open_path,
            commands::reveal_path,
            commands::download,
            commands::cancel_download,
            commands::is_ytdlp_installed,
            commands::get_settings,
            commands::set_settings,
            commands::get_lyrics,
            commands::search_lyrics,
            commands::get_now_playing,
            commands::analyze,
            commands::get_system_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
