use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

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
struct DownloadDonePayload {
    task_id: String,
    title: String,
    path: String,
    ext: String,
}

fn downloads_dir() -> String {
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|d| d.join("Downloads")))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string())
}

fn validate_task_id(task_id: &str) -> Result<()> {
    if task_id.is_empty()
        || task_id.len() > 64
        || !task_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(Error::Message("invalid task id".to_string()));
    }
    Ok(())
}

// ---------- Models ----------

#[tauri::command]
pub fn get_models(state: State<Arc<AppState>>) -> Vec<models::ModelInfo> {
    models::list_models(&state.resource_dir)
}

// ---------- Separation ----------

#[tauri::command]
pub async fn separate(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    task_id: String,
    options: SeparateOptions,
) -> Result<TaskStarted> {
    validate_task_id(&task_id)?;
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
                if msg.contains("HALITE_CANCELLED") {
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
    if !source.is_file() {
        return Err(Error::Message("HALITE_INVALID_AUDIO|file not found".to_string()));
    }
    let source_name = source
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let model_path = models::bundled_model_path(&state.resource_dir, &options.model_id)
        .ok_or_else(|| Error::Message("HALITE_MODEL_MISSING|unknown model".to_string()))?;
    if !model_path.is_file() {
        return Err(Error::Message("HALITE_MODEL_MISSING".to_string()));
    }

    let fmt = options.format.to_ascii_lowercase();
    if !matches!(fmt.as_str(), "wav" | "mp3" | "flac" | "m4a") {
        return Err(Error::Message("HALITE_INVALID_FORMAT".to_string()));
    }
    let mode = options.mode.as_deref().unwrap_or("all");
    if !matches!(mode, "all" | "vocals" | "instrumental") {
        return Err(Error::Message("HALITE_INVALID_MODE".to_string()));
    }
    let start_sec = options.start_sec.unwrap_or(0.0);
    if !start_sec.is_finite() || start_sec < 0.0 {
        return Err(Error::Message("HALITE_INVALID_TRIM".to_string()));
    }
    if let Some(end_sec) = options.end_sec {
        if !end_sec.is_finite() || end_sec <= start_sec {
            return Err(Error::Message("HALITE_INVALID_TRIM".to_string()));
        }
    }

    let output_root = std::path::PathBuf::from(
        options.output_dir.clone().unwrap_or_else(downloads_dir),
    );
    std::fs::create_dir_all(&output_root)
        .map_err(|e| Error::Message(format!("HALITE_OUTPUT|{e}")))?;
    if !output_root.is_dir() {
        return Err(Error::Message("HALITE_OUTPUT|not a directory".to_string()));
    }
    let base = safe_file_component(
        &source
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "stem".to_string()),
    );
    let job_output_dir = unique_output_dir(&output_root, &format!("{base} - Halite Stems"));
    std::fs::create_dir(&job_output_dir)
        .map_err(|e| Error::Message(format!("HALITE_OUTPUT|{e}")))?;
    let mut output_guard = OutputGuard::new(job_output_dir.clone());

    // Decode.
    let (samples, rate) = crate::audio::decode_to_stereo(source)
        .map_err(|error| Error::Message(format!("HALITE_INVALID_AUDIO|{error}")))?;
    let samples = crate::audio::resample_stereo(&samples, rate, SAMPLE_RATE);
    let samples = crate::audio::trim_stereo(
        &samples,
        SAMPLE_RATE,
        start_sec,
        options.end_sec,
    );
    if samples.len() < 2 {
        return Err(Error::Message("HALITE_INVALID_TRIM|empty audio".to_string()));
    }

    let duration_secs = (samples.len() / 2) as f64 / SAMPLE_RATE as f64;

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
    let stems = separator.separate(&[mix_l, mix_r], cancel, move |pct| {
        let _ = progress_app.emit(
            "separation://progress",
            ProgressPayload {
                task_id: progress_task_id.clone(),
                pct: pct * 0.9,
            },
        );
    })?;

    // Write stems into a collision-free job directory.
    let mut written = Vec::new();
    let outputs = build_outputs(&stems, mode);
    let output_count = outputs.len().max(1);
    for (index, (stem_name, interleaved)) in outputs.into_iter().enumerate() {
        if cancel.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(Error::Message("HALITE_CANCELLED".to_string()));
        }
        let wav_path = job_output_dir.join(format!("{base}.{stem_name}.wav"));
        crate::audio::write_wav(&wav_path, &interleaved, SAMPLE_RATE)?;

        let final_path = if fmt == "wav" {
            wav_path
        } else {
            let out_path = job_output_dir.join(format!("{base}.{stem_name}.{fmt}"));
            crate::audio::transcode(&state.data_dir, &wav_path, &out_path, &fmt, cancel)?;
            let _ = std::fs::remove_file(&wav_path);
            out_path
        };

        if cancel.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(Error::Message("HALITE_CANCELLED".to_string()));
        }

        let bytes = std::fs::metadata(&final_path).map(|m| m.len() as i64).unwrap_or(0);
        written.push((stem_name, final_path.to_string_lossy().to_string(), bytes));
        let _ = app.emit(
            "separation://progress",
            ProgressPayload {
                task_id: task_id.to_string(),
                pct: 0.9 + ((index + 1) as f64 / output_count as f64) * 0.1,
            },
        );
    }

    let output_dir_string = job_output_dir.to_string_lossy().to_string();
    let job_id = state
        .db
        .lock()
        .map_err(|_| Error::Message("database lock failed".to_string()))?
        .insert_completed_job(
            &options.source_path,
            &source_name,
            &options.model_id,
            &output_dir_string,
            duration_secs,
            &written,
        )?;
    output_guard.keep();

    Ok(job_id)
}

struct OutputGuard {
    path: std::path::PathBuf,
    keep: bool,
}

impl OutputGuard {
    fn new(path: std::path::PathBuf) -> Self {
        Self { path, keep: false }
    }

    fn keep(&mut self) {
        self.keep = true;
    }
}

impl Drop for OutputGuard {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

fn safe_file_component(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                ch
            }
        })
        .take(140)
        .collect();
    let cleaned = cleaned.trim().trim_end_matches('.');
    if cleaned.is_empty() {
        "audio".to_string()
    } else {
        cleaned.to_string()
    }
}

fn unique_output_dir(root: &std::path::Path, name: &str) -> std::path::PathBuf {
    let first = root.join(name);
    if !first.exists() {
        return first;
    }
    for index in 2..10_000 {
        let candidate = root.join(format!("{name} ({index})"));
        if !candidate.exists() {
            return candidate;
        }
    }
    root.join(format!("{name} - {}", std::process::id()))
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
    let db = state
        .db
        .lock()
        .map_err(|_| Error::Message("database lock failed".to_string()))?;
    db.list_jobs(200)
}

#[tauri::command]
pub fn delete_job(state: State<Arc<AppState>>, id: i64) -> Result<()> {
    let db = state
        .db
        .lock()
        .map_err(|_| Error::Message("database lock failed".to_string()))?;
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
    let db = state
        .db
        .lock()
        .map_err(|_| Error::Message("database lock failed".to_string()))?;
    db.save_preset(&name, &model_id, &stems_json, &format)
}

#[tauri::command]
pub fn list_presets(state: State<Arc<AppState>>) -> Result<Vec<crate::db::Preset>> {
    let db = state
        .db
        .lock()
        .map_err(|_| Error::Message("database lock failed".to_string()))?;
    db.list_presets()
}

#[tauri::command]
pub fn delete_preset(state: State<Arc<AppState>>, id: i64) -> Result<()> {
    let db = state
        .db
        .lock()
        .map_err(|_| Error::Message("database lock failed".to_string()))?;
    db.delete_preset(id)
}

// ---------- File dialogs & path helpers ----------

#[tauri::command]
pub async fn pick_audio_files(app: AppHandle) -> Option<Vec<String>> {
    use tauri_plugin_dialog::DialogExt;
    let files = app
        .dialog()
        .file()
        .add_filter("Audio", &["mp3", "wav", "flac", "m4a", "ogg", "aac", "aiff", "mp4"])
        .blocking_pick_files();
    files.map(|fs| {
        fs.into_iter()
            .filter_map(|f| f.into_path().ok())
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    })
}

#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog()
        .file()
        .blocking_pick_folder()
        .and_then(|f| f.into_path().ok())
        .map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_path(path: String) -> Result<()> {
    if !std::path::Path::new(&path).exists() {
        return Err(Error::Message("HALITE_OUTPUT_MISSING".to_string()));
    }
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(&path).spawn();
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("explorer").arg(&path).spawn();
    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open").arg(&path).spawn();
    result
        .map(|_| ())
        .map_err(|error| Error::Message(format!("HALITE_OPEN_FAILED|{error}")))
}

#[tauri::command]
pub fn reveal_path(path: String) -> Result<()> {
    if !std::path::Path::new(&path).exists() {
        return Err(Error::Message("HALITE_OUTPUT_MISSING".to_string()));
    }
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg("-R").arg(&path).spawn();
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("explorer").arg(format!("/select,{path}")).spawn();
    #[cfg(target_os = "linux")]
    let result = {
        if let Some(parent) = std::path::Path::new(&path).parent() {
            std::process::Command::new("xdg-open").arg(parent).spawn()
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "path has no parent",
            ))
        }
    };
    result
        .map(|_| ())
        .map_err(|error| Error::Message(format!("HALITE_OPEN_FAILED|{error}")))
}

#[tauri::command]
pub fn allow_audio_preview(
    app: AppHandle,
    state: State<Arc<AppState>>,
    path: String,
) -> Result<()> {
    let known = state
        .db
        .lock()
        .map_err(|_| Error::Message("database lock failed".to_string()))?
        .is_known_stem_path(&path)?;
    if !known || !std::path::Path::new(&path).is_file() {
        return Err(Error::Message("HALITE_PREVIEW_DENIED".to_string()));
    }
    app.asset_protocol_scope()
        .allow_file(&path)
        .map_err(|e| Error::Message(format!("HALITE_PREVIEW_DENIED|{e}")))
}

// ---------- Download (yt-dlp) ----------

#[tauri::command]
pub async fn download(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    task_id: String,
    url: String,
    output_dir: Option<String>,
    format: String,
) -> Result<TaskStarted> {
    validate_task_id(&task_id)?;
    let state = state.inner().clone();
    let cancel = state.cancel_flag(&task_id);
    let return_task_id = task_id.clone();
    let output_dir = output_dir.unwrap_or_else(downloads_dir);

    tauri::async_runtime::spawn_blocking(move || {
        let result = crate::downloader::download(
            &state.data_dir,
            &task_id,
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
                    DownloadDonePayload {
                        task_id,
                        title: r.title,
                        path: r.path,
                        ext: r.ext,
                    },
                );
            }
            Err(e) => {
                let msg = e.to_string();
                let event = if msg.contains("HALITE_CANCELLED") { "download://cancelled" } else { "download://error" };
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
pub fn get_settings(state: State<Arc<AppState>>) -> Result<crate::state::Settings> {
    state
        .settings
        .lock()
        .map(|settings| settings.clone())
        .map_err(|_| Error::Message("settings lock failed".to_string()))
}

#[tauri::command]
pub fn set_settings(state: State<Arc<AppState>>, settings: crate::state::Settings) -> Result<()> {
    let settings = settings.normalized();
    {
        let mut s = state
            .settings
            .lock()
            .map_err(|_| Error::Message("settings lock failed".to_string()))?;
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
        let (samples, rate) = crate::audio::decode_to_stereo(std::path::Path::new(&path))
            .map_err(|error| Error::Message(format!("HALITE_INVALID_AUDIO|{error}")))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_ids_are_restricted_to_safe_components() {
        assert!(validate_task_id("a0b1-uuid_value").is_ok());
        assert!(validate_task_id("").is_err());
        assert!(validate_task_id("../other-task").is_err());
    }

    #[test]
    fn output_names_remove_platform_separators() {
        assert_eq!(safe_file_component("a/b:c\\d"), "a_b_c_d");
        assert_eq!(safe_file_component("..."), "audio");
    }
}
