use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::error::{Error, Result};
use crate::separation::demucs::{Separator, SAMPLE_RATE};
use crate::separation::models;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeparateOptions {
    pub source_path: String,
    pub model_id: String,
    pub output_dir: Option<String>,
    pub format: String,
    pub mode: Option<String>,
    pub start_sec: Option<f64>,
    pub end_sec: Option<f64>,
    pub use_coreml: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskStarted {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize)]
struct ProgressPayload {
    task_id: String,
    pct: f64,
}

#[derive(Debug, Clone, Serialize)]
struct ErrorPayload {
    task_id: String,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
struct ModelProgress {
    model_id: String,
    pct: f64,
}

fn make_task_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}-{}", nanos, COUNTER.fetch_add(1, Ordering::Relaxed))
}

fn downloads_dir() -> String {
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|d| d.join("Downloads")))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string())
}

// ---------- Models ----------

#[tauri::command]
pub fn get_models(state: State<Arc<AppState>>) -> Vec<models::ModelInfo> {
    models::list_models(&state.data_dir)
}

#[tauri::command]
pub fn install_model(
    app: AppHandle,
    state: State<Arc<AppState>>,
    model_id: String,
) -> Result<()> {
    let data_dir = state.data_dir.clone();
    let id = model_id.clone();
    std::thread::spawn(move || {
        let result = models::download_model(&data_dir, &id, |pct| {
            let _ = app.emit("model://progress", ModelProgress { model_id: id.clone(), pct });
        });
        match result {
            Ok(_) => {
                let _ = app.emit("model://done", ModelProgress { model_id: id.clone(), pct: 1.0 });
            }
            Err(e) => {
                let _ = app.emit("model://error", ErrorPayload { task_id: id, message: e.to_string() });
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub fn delete_model(state: State<Arc<AppState>>, model_id: String) -> Result<()> {
    models::delete_model(&state.data_dir, &model_id)
}

// ---------- Separation ----------

#[tauri::command]
pub async fn separate(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    options: SeparateOptions,
) -> Result<TaskStarted> {
    let task_id = make_task_id();
    let state = state.inner().clone();
    let cancel = state.cancel_flag(&task_id);
    let return_task_id = task_id.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let result = run_separation(&state, &options, &cancel, &task_id, &app);
        state.clear_cancel(&task_id);
        match result {
            Ok(job_id) => {
                let _ = app.emit("separation://done", ProgressPayload { task_id, pct: 1.0 });
                let _ = job_id;
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("cancelled") {
                    let _ = app.emit("separation://cancelled", ErrorPayload { task_id, message: msg });
                } else {
                    let _ = app.emit("separation://error", ErrorPayload { task_id, message: msg });
                }
            }
        }
    });

    Ok(TaskStarted { task_id: return_task_id })
}

fn run_separation(
    state: &AppState,
    options: &SeparateOptions,
    cancel: &Arc<AtomicBool>,
    task_id: &str,
    app: &AppHandle,
) -> Result<i64> {
    let source = std::path::Path::new(&options.source_path);
    let source_name = source
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    // Ensure model is present.
    let model_path = models::model_path(&state.data_dir, &options.model_id);
    if !model_path.exists() {
        return Err(Error::Message(
            "model not installed; install it first".to_string(),
        ));
    }

    // Decode.
    let (samples, rate) = crate::audio::decode_to_stereo(source)?;
    let samples = crate::audio::resample_stereo(&samples, rate, SAMPLE_RATE);
    let samples = crate::audio::trim_stereo(
        &samples,
        SAMPLE_RATE,
        options.start_sec.unwrap_or(0.0),
        options.end_sec,
    );
    if samples.len() < 2 {
        return Err(Error::Message("empty audio after trim".to_string()));
    }

    let duration_secs = (samples.len() / 2) as f64 / SAMPLE_RATE as f64;

    let output_dir = options
        .output_dir
        .clone()
        .unwrap_or_else(downloads_dir);

    // Insert job.
    let job_id = {
        let db = state.db.lock().unwrap();
        db.insert_job(
            &options.source_path,
            &source_name,
            &options.model_id,
            &output_dir,
            duration_secs,
        )?
    };

    // De-interleave to planar.
    let n = samples.len() / 2;
    let mut mix_l = Vec::with_capacity(n);
    let mut mix_r = Vec::with_capacity(n);
    for i in 0..n {
        mix_l.push(samples[i * 2]);
        mix_r.push(samples[i * 2 + 1]);
    }

    let use_coreml = options.use_coreml.unwrap_or(true);
    let mut separator = Separator::new(&model_path, use_coreml)?;

    let progress_task_id = task_id.to_string();
    let progress_app = app.clone();
    let stems = separator.separate(&[mix_l, mix_r], move |pct| {
        let _ = progress_app.emit(
            "separation://progress",
            ProgressPayload {
                task_id: progress_task_id.clone(),
                pct,
            },
        );
    })?;

    // Write stems.
    std::fs::create_dir_all(&output_dir)?;
    let base = source
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "stem".to_string());

    let fmt = options.format.to_lowercase();
    let mode = options.mode.as_deref().unwrap_or("all");
    let mut written = Vec::new();
    for (stem_name, interleaved) in build_outputs(&stems, mode) {
        if cancel.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(Error::Message("cancelled".to_string()));
        }
        let wav_path = std::path::Path::new(&output_dir).join(format!(
            "{base}.{stem_name}.wav"
        ));
        crate::audio::write_wav(&wav_path, &interleaved, SAMPLE_RATE)?;

        let final_path = if fmt == "wav" {
            wav_path
        } else {
            let out_path = std::path::Path::new(&output_dir).join(format!(
                "{base}.{stem_name}.{fmt}"
            ));
            crate::audio::transcode(&wav_path, &out_path, &fmt)?;
            let _ = std::fs::remove_file(&wav_path);
            out_path
        };

        let bytes = std::fs::metadata(&final_path).map(|m| m.len() as i64).unwrap_or(0);
        written.push((stem_name, final_path.to_string_lossy().to_string(), bytes));
    }

    {
        let db = state.db.lock().unwrap();
        for (name, path, bytes) in &written {
            db.insert_stem(job_id, name, path, *bytes)?;
        }
    }

    Ok(job_id)
}

fn interleave(l: &[f32], r: &[f32]) -> Vec<f32> {
    let n = l.len().min(r.len());
    let mut out = Vec::with_capacity(n * 2);
    for i in 0..n {
        out.push(l[i]);
        out.push(r[i]);
    }
    out
}

fn build_outputs(stems: &[[Vec<f32>; 2]], mode: &str) -> Vec<(String, Vec<f32>)> {
    let sources = crate::separation::demucs::SOURCES;
    match mode {
        "vocals" => vec![(
            "vocals".to_string(),
            interleave(&stems[3][0], &stems[3][1]),
        )],
        "instrumental" => {
            let n = stems[0][0].len();
            let mut l = vec![0f32; n];
            let mut r = vec![0f32; n];
            for i in 0..3 {
                for j in 0..n {
                    l[j] += stems[i][0][j];
                    r[j] += stems[i][1][j];
                }
            }
            vec![("instrumental".to_string(), interleave(&l, &r))]
        }
        _ => (0..4)
            .map(|i| (sources[i].to_string(), interleave(&stems[i][0], &stems[i][1])))
            .collect(),
    }
}

#[tauri::command]
pub fn cancel_separation(state: State<Arc<AppState>>, task_id: String) {
    state.request_cancel(&task_id);
}

// ---------- Jobs & presets ----------

#[tauri::command]
pub fn list_jobs(state: State<Arc<AppState>>) -> Result<Vec<crate::db::Job>> {
    let db = state.db.lock().unwrap();
    db.list_jobs(200)
}

#[tauri::command]
pub fn delete_job(state: State<Arc<AppState>>, id: i64) -> Result<()> {
    let db = state.db.lock().unwrap();
    db.delete_job(id)
}

#[tauri::command]
pub fn save_preset(
    state: State<Arc<AppState>>,
    name: String,
    model_id: String,
    stems_json: String,
    format: String,
) -> Result<i64> {
    let db = state.db.lock().unwrap();
    db.save_preset(&name, &model_id, &stems_json, &format)
}

#[tauri::command]
pub fn list_presets(state: State<Arc<AppState>>) -> Result<Vec<crate::db::Preset>> {
    let db = state.db.lock().unwrap();
    db.list_presets()
}

#[tauri::command]
pub fn delete_preset(state: State<Arc<AppState>>, id: i64) -> Result<()> {
    let db = state.db.lock().unwrap();
    db.delete_preset(id)
}

// ---------- File dialogs & path helpers ----------

#[tauri::command]
pub fn pick_audio_files(app: AppHandle) -> Option<Vec<String>> {
    use tauri_plugin_dialog::DialogExt;
    let files = app
        .dialog()
        .file()
        .add_filter("Audio", &["mp3", "wav", "flac", "m4a", "ogg", "aac", "aiff", "mp4"])
        .blocking_pick_files();
    files.map(|fs| fs.into_iter().filter_map(|f| f.into_path().ok()).map(|p| p.to_string_lossy().to_string()).collect())
}

#[tauri::command]
pub fn pick_folder(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog()
        .file()
        .blocking_pick_folder()
        .and_then(|f| f.into_path().ok())
        .map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_path(path: String) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(&path).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd").args(["/C", "start", "", &path]).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
}

#[tauri::command]
pub fn reveal_path(path: String) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg("-R").arg(&path).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer").arg(format!("/select,{path}")).spawn();
    #[cfg(target_os = "linux")]
    {
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
        }
    }
}

// ---------- Download (yt-dlp) ----------

#[tauri::command]
pub async fn download(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    url: String,
    output_dir: Option<String>,
    format: String,
) -> Result<TaskStarted> {
    let task_id = make_task_id();
    let state = state.inner().clone();
    let cancel = state.cancel_flag(&task_id);
    let return_task_id = task_id.clone();
    let output_dir = output_dir.unwrap_or_else(downloads_dir);

    tauri::async_runtime::spawn_blocking(move || {
        let result = crate::downloader::download(
            &state.data_dir,
            &url,
            std::path::Path::new(&output_dir),
            &format,
            cancel.clone(),
            {
                let app = app.clone();
                let task_id = task_id.clone();
                move |pct| {
                    let _ = app.emit(
                        "download://progress",
                        ProgressPayload { task_id: task_id.clone(), pct },
                    );
                }
            },
        );
        state.clear_cancel(&task_id);
        match result {
            Ok(r) => {
                let _ = app.emit(
                    "download://done",
                    crate::downloader::DownloadResult { title: r.title, path: r.path, ext: r.ext },
                );
            }
            Err(e) => {
                let msg = e.to_string();
                let event = if msg.contains("cancelled") { "download://cancelled" } else { "download://error" };
                let _ = app.emit(event, ErrorPayload { task_id, message: msg });
            }
        }
    });

    Ok(TaskStarted { task_id: return_task_id })
}

#[tauri::command]
pub fn cancel_download(state: State<Arc<AppState>>, task_id: String) {
    state.request_cancel(&task_id);
}

#[tauri::command]
pub fn is_ytdlp_installed(state: State<Arc<AppState>>) -> bool {
    crate::downloader::is_ytdlp_installed(&state.data_dir)
}

// ---------- Settings ----------

#[tauri::command]
pub fn get_settings(state: State<Arc<AppState>>) -> crate::state::Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_settings(state: State<Arc<AppState>>, settings: crate::state::Settings) -> Result<()> {
    {
        let mut s = state.settings.lock().unwrap();
        *s = settings.clone();
    }
    settings.save(&state.data_dir)
}

// ---------- Lyrics & now playing ----------

#[tauri::command]
pub async fn get_lyrics(
    track: String,
    artist: String,
    album: String,
    duration: f64,
) -> Result<Option<crate::lyrics::LyricsResult>> {
    crate::lyrics::get_lyrics(&track, &artist, &album, duration).await
}

#[tauri::command]
pub async fn search_lyrics(query: String) -> Result<Vec<crate::lyrics::LyricsResult>> {
    crate::lyrics::search_lyrics(&query).await
}

#[tauri::command]
pub fn get_now_playing() -> Option<crate::nowplaying::NowPlaying> {
    crate::nowplaying::now_playing()
}

// ---------- Analysis ----------

#[tauri::command]
pub async fn analyze(path: String) -> Result<crate::bpm::Analysis> {
    tauri::async_runtime::spawn_blocking(move || {
        let (samples, rate) = crate::audio::decode_to_stereo(std::path::Path::new(&path))?;
        crate::bpm::analyze(&samples, rate)
    })
    .await
    .map_err(|e| Error::Message(e.to_string()))?
}

// ---------- System info ----------

#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub version: String,
}

#[tauri::command]
pub fn get_system_info() -> SystemInfo {
    SystemInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}
