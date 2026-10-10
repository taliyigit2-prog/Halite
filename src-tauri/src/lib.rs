mod audio;
mod bpm;
mod commands;
mod db;
mod downloader;
mod error;
mod lyrics;
mod metadata;
mod nowplaying;
mod separation;
mod state;
mod studio;

use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            #[cfg(debug_assertions)]
            let data_dir = std::env::var_os("HALITE_TEST_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or(data_dir);
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
            studio::save_voice_preferences,
            commands::get_settings,
            commands::set_settings,
            commands::get_lyrics,
            commands::search_lyrics,
            commands::get_now_playing,
            commands::analyze,
            commands::get_system_info,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                let state = app.state::<Arc<state::AppState>>().inner().clone();
                state.cancel_all();
                if state.studio_busy.load(std::sync::atomic::Ordering::SeqCst) {
                    api.prevent_exit();
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        while state.studio_busy.load(std::sync::atomic::Ordering::SeqCst) {
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        }
                        app.exit(0);
                    });
                }
            }
        });
}
