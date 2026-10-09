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
mod metadata;
mod studio;

use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let resource_dir = app.path().resource_dir()?;
            let app_state = Arc::new(state::AppState::new(data_dir, resource_dir)?);
            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_models,
            commands::separate,
            commands::cancel_separation,
            commands::list_jobs,
            commands::delete_job,
            commands::pick_audio_files,
            commands::pick_folder,
            commands::open_path,
            commands::reveal_path,
            commands::allow_audio_preview,
            commands::download,
            commands::cancel_download,
            metadata::pick_tag_files,
            metadata::pick_cover,
            metadata::save_tags,
            metadata::restore_tags,
            metadata::export_cover,
            metadata::search_metadata,
            studio::studio_status,
            studio::install_studio,
            studio::generate_speech,
            studio::cancel_studio,
            studio::pick_studio_reference,
            studio::verify_studio,
            studio::remove_studio,
            studio::export_speech,
            studio::save_voice_profile,
            studio::list_voice_profiles,
            studio::delete_voice_profile,
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
