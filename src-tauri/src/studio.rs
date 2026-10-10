//! Managed local Chatterbox runtime, verified downloads, and a cancellable worker.
use crate::{
    error::{Error, Result},
    state::AppState,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

const MANIFEST: &str = include_str!("../resources/studio-manifest.json");
const REQUIREMENTS: &str = include_str!("../resources/studio-requirements.txt");
const WORKER: &str = include_str!("../resources/studio-worker.py");
const LANGUAGES: &[&str] = &[
    "ar", "da", "de", "el", "en", "es", "fi", "fr", "he", "hi", "it", "ja", "ko", "ms", "nl", "no",
    "pl", "pt", "ru", "sv", "sw", "tr", "zh",
];

fn error(message: impl std::fmt::Display) -> Error {
    Error::Message(format!("HALITE_STUDIO_FAILED|{message}"))
}
fn cancelled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::SeqCst) {
        Err(Error::Message("HALITE_CANCELLED".into()))
    } else {
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct Asset {
    url: String,
    sha256: String,
    size: u64,
}
#[derive(Debug, Deserialize)]
struct ModelFile {
    name: String,
    url: String,
    sha256: String,
    size: u64,
}
#[derive(Debug, Deserialize)]
struct Manifest {
    id: String,
    code_revision: String,
    model_revision: String,
    python: String,
    required_free_bytes: u64,
    source: Asset,
    runtimes: BTreeMap<String, Asset>,
    files: Vec<ModelFile>,
}
fn manifest() -> Result<Manifest> {
    serde_json::from_str(MANIFEST).map_err(Error::from)
}
fn root(data_dir: &Path) -> PathBuf {
    data_dir.join("studio-v1")
}
fn python(root: &Path) -> PathBuf {
    root.join(if cfg!(windows) {
        "venv/Scripts/python.exe"
    } else {
        "venv/bin/python"
    })
}
fn uv_path(root: &Path) -> PathBuf {
    root.join(if cfg!(windows) { "uv.exe" } else { "uv" })
}
fn release_id() -> String {
    let mut hash = Sha256::new();
    hash.update(MANIFEST);
    hash.update(REQUIREMENTS);
    hash.update(WORKER);
    format!("{:x}", hash.finalize())
}
fn supported() -> bool {
    !(cfg!(target_os = "macos") && cfg!(target_arch = "x86_64"))
        && manifest()
            .map(|m| {
                m.runtimes.contains_key(&format!(
                    "{}-{}",
                    std::env::consts::OS,
                    std::env::consts::ARCH
                ))
            })
            .unwrap_or(false)
}
fn ready(root: &Path) -> bool {
    python(root).is_file()
        && root.join("source/src/chatterbox/mtl_tts.py").is_file()
        && std::fs::read_to_string(root.join("ready"))
            .map(|value| value == release_id())
            .unwrap_or(false)
        && manifest()
            .map(|m| {
                m.files.iter().all(|file| {
                    std::fs::metadata(root.join("models").join(&file.name))
                        .map(|meta| meta.len() == file.size)
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
}

#[derive(Debug, Serialize)]
pub struct StudioStatus {
    installed: bool,
    supported: bool,
    busy: bool,
    model_bytes: u64,
    required_free_bytes: u64,
    installed_bytes: u64,
    location: String,
    version: String,
    license: &'static str,
}
#[tauri::command]
pub fn studio_status(state: State<Arc<AppState>>) -> Result<StudioStatus> {
    let manifest = manifest()?;
    let root = root(&state.data_dir);
    Ok(StudioStatus {
        installed: ready(&root),
        supported: supported(),
        busy: state.studio_busy.load(Ordering::SeqCst),
        model_bytes: manifest.files.iter().map(|file| file.size).sum(),
        required_free_bytes: manifest.required_free_bytes,
        installed_bytes: directory_size(&root),
        location: root.to_string_lossy().into(),
        version: manifest.id,
        license: "MIT",
    })
}
fn directory_size(path: &Path) -> u64 {
    std::fs::read_dir(path)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| {
                    entry
                        .file_type()
                        .map(|kind| {
                            if kind.is_dir() {
                                directory_size(&entry.path())
                            } else if kind.is_file() {
                                entry.metadata().map(|m| m.len()).unwrap_or(0)
                            } else {
                                0
                            }
                        })
                        .unwrap_or(0)
                })
                .sum()
        })
        .unwrap_or(0)
}

#[derive(Clone, Serialize)]
struct Progress {
    task_id: String,
    pct: f64,
    stage: String,
}
fn progress(app: &AppHandle, task: &str, pct: f64, stage: &str) {
    let _ = app.emit(
        "studio://progress",
        Progress {
            task_id: task.into(),
            pct: pct.clamp(0.0, 1.0),
            stage: stage.into(),
        },
    );
}
#[derive(Clone, Serialize)]
struct Outcome {
    task_id: String,
    path: Option<String>,
    message: Option<String>,
}
fn finish(app: &AppHandle, task: String, result: Result<Option<PathBuf>>) {
    match result {
        Ok(path) => {
            let _ = app.emit(
                "studio://done",
                Outcome {
                    task_id: task,
                    path: path.map(|p| p.to_string_lossy().into()),
                    message: None,
                },
            );
        }
        Err(e) => {
            let message = e.to_string();
            let event = if message.contains("HALITE_CANCELLED") {
                "studio://cancelled"
            } else {
                "studio://error"
            };
            let _ = app.emit(
                event,
                Outcome {
                    task_id: task,
                    path: None,
                    message: Some(message),
                },
            );
        }
    }
}
fn validate_task_id(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(error("invalid task id"));
    }
    Ok(())
}
fn acquire(state: &AppState) -> Result<()> {
    state
        .studio_busy
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .map(|_| ())
        .map_err(|_| Error::Message("HALITE_STUDIO_BUSY".into()))
}

pub fn hash_file(path: &Path, cancel: &AtomicBool) -> Result<String> {
    let mut input = std::fs::File::open(path)?;
    let mut buffer = [0u8; 128 * 1024];
    let mut hash = Sha256::new();
    loop {
        cancelled(cancel)?;
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

async fn download(
    client: &reqwest::Client,
    asset: &Asset,
    path: &Path,
    cancel: &AtomicBool,
    mut report: impl FnMut(f64),
) -> Result<()> {
    cancelled(cancel)?;
    if path.is_file()
        && std::fs::metadata(path)?.len() == asset.size
        && hash_file(path, cancel)? == asset.sha256
    {
        report(1.0);
        return Ok(());
    }
    let partial = path.with_extension("part");
    let mut offset = std::fs::metadata(&partial)
        .map(|meta| meta.len())
        .unwrap_or(0);
    if offset > asset.size {
        std::fs::remove_file(&partial)?;
        offset = 0;
    }
    if offset < asset.size {
        let mut request = client.get(&asset.url);
        if offset > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={offset}-"));
        }
        let mut response = request.send().await?.error_for_status()?;
        if offset > 0 && response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            offset = 0;
        }
        if offset > 0 {
            let expected = format!("bytes {offset}-");
            if !response
                .headers()
                .get(reqwest::header::CONTENT_RANGE)
                .and_then(|value| value.to_str().ok())
                .map(|value| value.starts_with(&expected))
                .unwrap_or(false)
            {
                return Err(error("invalid resumed response"));
            }
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(offset > 0)
            .truncate(offset == 0)
            .open(&partial)?;
        while let Some(bytes) = response.chunk().await? {
            cancelled(cancel)?;
            offset += bytes.len() as u64;
            if offset > asset.size {
                return Err(error("download exceeds manifest size"));
            }
            file.write_all(&bytes)?;
            report(offset as f64 / asset.size.max(1) as f64);
        }
        file.sync_all()?;
    }
    cancelled(cancel)?;
    if offset != asset.size || hash_file(&partial, cancel)? != asset.sha256 {
        let _ = std::fs::remove_file(&partial);
        return Err(Error::Message("HALITE_STUDIO_CHECKSUM".into()));
    }
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    std::fs::rename(partial, path)?;
    report(1.0);
    Ok(())
}

fn unpack_uv(archive: &Path, root: &Path) -> Result<()> {
    let executable = if cfg!(windows) { "uv.exe" } else { "uv" };
    let mut found = false;
    if cfg!(windows) {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(archive)?).map_err(error)?;
        for i in 0..zip.len() {
            let mut file = zip.by_index(i).map_err(error)?;
            if file
                .enclosed_name()
                .and_then(|path| path.file_name().map(|name| name == executable))
                .unwrap_or(false)
            {
                std::io::copy(&mut file, &mut std::fs::File::create(uv_path(root))?)?;
                found = true;
            }
        }
    } else {
        let mut tar =
            tar::Archive::new(flate2::read::GzDecoder::new(std::fs::File::open(archive)?));
        for entry in tar.entries()? {
            let mut file = entry?;
            if file.header().entry_type().is_file()
                && file
                    .path()?
                    .file_name()
                    .map(|name| name == executable)
                    .unwrap_or(false)
            {
                std::io::copy(&mut file, &mut std::fs::File::create(uv_path(root))?)?;
                found = true;
            }
        }
    }
    if !found {
        return Err(error("runtime executable missing"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(uv_path(root), std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn unpack_source(archive: &Path, root: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(archive)?).map_err(error)?;
    let source = root.join("source");
    std::fs::create_dir_all(&source)?;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(error)?;
        let Some(path) = file.enclosed_name() else {
            return Err(error("invalid archive path"));
        };
        let relative: PathBuf = path.components().skip(1).collect();
        if !relative.starts_with("src") && relative != Path::new("LICENSE") {
            continue;
        }
        if file
            .unix_mode()
            .map(|mode| mode & 0o170000 == 0o120000)
            .unwrap_or(false)
        {
            return Err(error("archive symlink"));
        }
        let output = source.join(relative);
        if file.is_dir() {
            std::fs::create_dir_all(output)?;
        } else {
            std::fs::create_dir_all(output.parent().unwrap())?;
            std::io::copy(&mut file, &mut std::fs::File::create(output)?)?;
        }
    }
    // Source is loaded through PYTHONPATH. Supply the distribution metadata that
    // upstream __init__ reads, without installing its demo/web dependencies.
    let dist = source.join("src/chatterbox_tts-0.1.7.dist-info");
    std::fs::create_dir_all(&dist)?;
    std::fs::write(
        dist.join("METADATA"),
        "Metadata-Version: 2.1\nName: chatterbox-tts\nVersion: 0.1.7\nLicense: MIT\n",
    )?;
    Ok(())
}

fn configure(command: &mut Command, root: &Path) {
    command
        .env("UV_CACHE_DIR", root.join("cache"))
        .env("UV_PYTHON_INSTALL_DIR", root.join("python"))
        .env("UV_NO_CONFIG", "1")
        .env("PYTHONNOUSERSITE", "1")
        .env("PYTHONUNBUFFERED", "1")
        .env("PYTHONPATH", root.join("source/src"))
        .env("HF_HOME", root.join("cache/huggingface"))
        .env("TORCH_HOME", root.join("cache/torch"))
        .env("HF_HUB_OFFLINE", "1")
        .env("TRANSFORMERS_OFFLINE", "1")
        .env("HF_HUB_DISABLE_TELEMETRY", "1")
        .env("DO_NOT_TRACK", "1")
        .env_remove("HF_TOKEN")
        .env_remove("HUGGING_FACE_HUB_TOKEN")
        .env_remove("PYTHONSTARTUP")
        .env_remove("PYTHONHOME");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
}

fn stop(child: &mut std::process::Child) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &child.id().to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

struct ManagedChild(std::process::Child);
impl std::ops::Deref for ManagedChild {
    type Target = std::process::Child;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for ManagedChild {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Drop for ManagedChild {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            stop(&mut self.0);
        }
    }
}

fn run_process(
    command: &mut Command,
    request: Option<&serde_json::Value>,
    cancel: &AtomicBool,
    mut on_event: impl FnMut(&serde_json::Value),
) -> Result<()> {
    command
        .stdin(if request.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = ManagedChild(command.spawn().map_err(error)?);
    if let Some(request) = request {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| error("worker input missing"))?;
        writeln!(stdin, "{}", request)?;
    }
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| error("worker output missing"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| error("worker diagnostics missing"))?;
    let (send, receive) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else {
                break;
            };
            if line.len() <= 65536 && send.send(line).is_err() {
                break;
            }
        }
    });
    let diagnostic = std::thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        let mut stream = BufReader::new(stderr);
        let mut tail = Vec::new();
        loop {
            match stream.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    tail.extend_from_slice(&buffer[..count]);
                    if tail.len() > 8192 {
                        tail.drain(..tail.len() - 8192);
                    }
                }
            }
        }
        String::from_utf8_lossy(&tail).into_owned()
    });
    let start = Instant::now();
    let mut worker_done = request.is_none();
    let mut worker_error = None;
    loop {
        if cancel.load(Ordering::SeqCst) || start.elapsed() > Duration::from_secs(3600) {
            stop(&mut child);
            let _ = reader.join();
            let _ = diagnostic.join();
            return Err(Error::Message(
                if cancel.load(Ordering::SeqCst) {
                    "HALITE_CANCELLED"
                } else {
                    "HALITE_STUDIO_TIMEOUT"
                }
                .into(),
            ));
        }
        for line in receive.try_iter() {
            if request.is_some() {
                let value: serde_json::Value = serde_json::from_str(&line).map_err(error)?;
                if value["protocol"] != 1 {
                    stop(&mut child);
                    return Err(Error::Message("HALITE_STUDIO_PROTOCOL".into()));
                }
                if value["kind"] == "done" {
                    worker_done = true;
                }
                if value["kind"] == "error" {
                    worker_error = Some(
                        value["code"]
                            .as_str()
                            .unwrap_or("HALITE_STUDIO_FAILED")
                            .to_owned(),
                    );
                }
                on_event(&value);
            }
        }
        if let Some(status) = child.try_wait()? {
            let _ = reader.join();
            let diagnostics = diagnostic.join().unwrap_or_default();
            for line in receive.try_iter() {
                if request.is_some() {
                    let value: serde_json::Value = serde_json::from_str(&line).map_err(error)?;
                    if value["protocol"] != 1 {
                        return Err(Error::Message("HALITE_STUDIO_PROTOCOL".into()));
                    }
                    if value["kind"] == "done" {
                        worker_done = true;
                    }
                    if value["kind"] == "error" {
                        worker_error = Some(
                            value["code"]
                                .as_str()
                                .unwrap_or("HALITE_STUDIO_FAILED")
                                .into(),
                        );
                    }
                    on_event(&value);
                }
            }
            if let Some(message) = worker_error {
                return Err(Error::Message(format!("{message}|{diagnostics}")));
            }
            return if status.success() && worker_done {
                Ok(())
            } else {
                Err(error(format!("process exited {status}: {diagnostics}")))
            };
        }
        std::thread::sleep(Duration::from_millis(80));
    }
}

fn check_worker(root: &Path, cancel: &AtomicBool) -> Result<()> {
    let mut command = Command::new(python(root));
    configure(&mut command, root);
    command.arg(root.join("worker.py"));
    run_process(
        &mut command,
        Some(&serde_json::json!({"protocol":1,"command":"check","model_dir":root.join("models")})),
        cancel,
        |_| {},
    )
}

pub fn install(
    data_dir: &Path,
    cancel: &AtomicBool,
    mut report: impl FnMut(f64, &str),
) -> Result<()> {
    if !supported() {
        return Err(Error::Message("HALITE_STUDIO_UNSUPPORTED".into()));
    }
    let manifest = manifest()?;
    let root = root(data_dir);
    std::fs::create_dir_all(root.join("models"))?;
    if fs2::available_space(&root)? < manifest.required_free_bytes {
        return Err(Error::Message("HALITE_STUDIO_DISK".into()));
    }
    let key = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    let runtime_asset = manifest
        .runtimes
        .get(&key)
        .ok_or_else(|| Error::Message("HALITE_STUDIO_UNSUPPORTED".into()))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3600))
            .connect_timeout(Duration::from_secs(30))
            .user_agent("Halite/0.2")
            .build()?;
        download(
            &client,
            runtime_asset,
            &root.join("uv.archive"),
            cancel,
            |pct| report(pct * 0.02, "runtime"),
        )
        .await?;
        unpack_uv(&root.join("uv.archive"), &root)?;
        download(
            &client,
            &manifest.source,
            &root.join("source.zip"),
            cancel,
            |pct| report(0.02 + pct * 0.02, "runtime"),
        )
        .await?;
        unpack_source(&root.join("source.zip"), &root)?;
        Ok::<_, Error>(())
    })?;
    std::fs::write(root.join("requirements.txt"), REQUIREMENTS)?;
    std::fs::write(root.join("worker.py"), WORKER)?;
    report(0.04, "runtime");
    let mut command = Command::new(uv_path(&root));
    configure(&mut command, &root);
    command
        .args([
            "venv",
            "--allow-existing",
            "--managed-python",
            "--python",
            &manifest.python,
        ])
        .arg(root.join("venv"));
    run_process(&mut command, None, cancel, |_| {})?;
    report(0.08, "dependencies");
    let mut command = Command::new(uv_path(&root));
    configure(&mut command, &root);
    command
        .args([
            "pip",
            "sync",
            "--require-hashes",
            "--index-strategy",
            "unsafe-best-match",
            "--python",
        ])
        .arg(python(&root))
        .arg(root.join("requirements.txt"));
    run_process(&mut command, None, cancel, |_| {})?;
    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3600))
            .user_agent("Halite/0.2")
            .build()?;
        let total: u64 = manifest.files.iter().map(|file| file.size).sum();
        let mut previous = 0u64;
        for file in &manifest.files {
            let asset = Asset {
                url: file.url.clone(),
                sha256: file.sha256.clone(),
                size: file.size,
            };
            download(
                &client,
                &asset,
                &root.join("models").join(&file.name),
                cancel,
                |pct| {
                    report(
                        0.18 + 0.78 * (previous as f64 + pct * file.size as f64) / total as f64,
                        "models",
                    )
                },
            )
            .await?;
            previous += file.size;
        }
        Ok::<_, Error>(())
    })?;
    report(0.97, "verifying");
    check_worker(&root, cancel)?;
    cancelled(cancel)?;
    std::fs::write(root.join("ready"), release_id())?;
    report(1.0, "ready");
    Ok(())
}

#[tauri::command]
pub async fn install_studio(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    task_id: String,
) -> Result<()> {
    validate_task_id(&task_id)?;
    acquire(&state)?;
    let state = state.inner().clone();
    let cancel = state.cancel_flag(&task_id);
    tauri::async_runtime::spawn_blocking(move || {
        let result = install(&state.data_dir, &cancel, |pct, stage| {
            progress(&app, &task_id, pct, stage)
        })
        .map(|_| None);
        state.clear_cancel(&task_id);
        state.studio_busy.store(false, Ordering::SeqCst);
        finish(&app, task_id, result);
    });
    Ok(())
}

#[derive(Clone, Deserialize)]
pub struct GenerateOptions {
    text: String,
    language: String,
    seed: u32,
    exaggeration: f64,
    cfg_weight: f64,
    reference: Option<String>,
    consent: bool,
    device: String,
}
fn validate_options(options: &GenerateOptions) -> Result<()> {
    if options.text.trim().is_empty()
        || options.text.chars().count() > 3000
        || options.text.contains('\0')
    {
        return Err(Error::Message("HALITE_STUDIO_TEXT".into()));
    }
    if !LANGUAGES.contains(&options.language.as_str()) {
        return Err(Error::Message("HALITE_STUDIO_LANGUAGE".into()));
    }
    if !options.exaggeration.is_finite()
        || !(0.0..=2.0).contains(&options.exaggeration)
        || !options.cfg_weight.is_finite()
        || !(0.0..=1.0).contains(&options.cfg_weight)
        || !["auto", "cpu"].contains(&options.device.as_str())
    {
        return Err(error("invalid options"));
    }
    if options.reference.is_some() && !options.consent {
        return Err(Error::Message("HALITE_STUDIO_CONSENT".into()));
    }
    Ok(())
}

fn normalized_reference(source: &Path, output: &Path) -> Result<()> {
    if std::fs::metadata(source)?.len() > 20_000_000 {
        return Err(Error::Message("HALITE_STUDIO_REFERENCE".into()));
    }
    let info = crate::metadata::read(source)?;
    if !(3.0..=30.0).contains(&info.duration) {
        return Err(Error::Message("HALITE_STUDIO_REFERENCE".into()));
    }
    let (samples, rate) = crate::audio::decode_to_stereo(source)?;
    let samples = crate::audio::resample_stereo(&samples, rate, 24000);
    let rms = (samples
        .iter()
        .map(|sample| (*sample as f64).powi(2))
        .sum::<f64>()
        / samples.len().max(1) as f64)
        .sqrt();
    if !rms.is_finite() || rms < 0.001 {
        return Err(Error::Message("HALITE_STUDIO_REFERENCE".into()));
    }
    crate::audio::write_wav(output, &samples, 24000).map(|_| ())
}

#[tauri::command]
pub fn save_voice_preferences(
    state: State<Arc<AppState>>,
    preferences: crate::state::VoicePreferences,
) -> Result<()> {
    let mut settings = state.settings.lock().map_err(error)?;
    settings.voice = preferences.normalized();
    settings.save(&state.data_dir)
}

#[tauri::command]
pub async fn pick_studio_reference(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<crate::metadata::Metadata>> {
    let Some(path) = app
        .dialog()
        .file()
        .add_filter(
            "Reference audio",
            &["wav", "flac", "mp3", "m4a", "ogg", "aiff"],
        )
        .blocking_pick_file()
        .and_then(|file| file.into_path().ok())
    else {
        return Ok(None);
    };
    let path = std::fs::canonicalize(path)?;
    let metadata = crate::metadata::read(&path)?;
    if !(3.0..=30.0).contains(&metadata.duration) || metadata.stamp.bytes > 20_000_000 {
        return Err(Error::Message("HALITE_STUDIO_REFERENCE".into()));
    }
    state.selected_files.lock().map_err(error)?.insert(path);
    Ok(Some(metadata))
}

#[tauri::command]
pub async fn generate_speech(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    task_id: String,
    options: GenerateOptions,
) -> Result<()> {
    validate_task_id(&task_id)?;
    validate_options(&options)?;
    if !ready(&root(&state.data_dir)) {
        return Err(Error::Message("HALITE_STUDIO_NOT_READY".into()));
    }
    let reference = options
        .reference
        .as_ref()
        .map(|path| crate::metadata::require_selected(&state, path))
        .transpose()?;
    acquire(&state)?;
    if state
        .ai_busy
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        state.studio_busy.store(false, Ordering::SeqCst);
        return Err(Error::Message("HALITE_AI_BUSY".into()));
    }
    let state = state.inner().clone();
    let cancel = state.cancel_flag(&task_id);
    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| {
            let root = root(&state.data_dir);
            progress(&app, &task_id, 0.0, "verifying");
            for file in manifest()?.files {
                if hash_file(&root.join("models").join(file.name), &cancel)? != file.sha256 {
                    return Err(Error::Message("HALITE_STUDIO_CHECKSUM".into()));
                }
            }
            let work = tempfile::tempdir_in(&root)?;
            let reference_path = if let Some(reference) = &reference {
                let path = work.path().join("reference.wav");
                normalized_reference(reference, &path)?;
                Some(path)
            } else {
                None
            };
            let output = work.path().join("speech.wav");
            let request = serde_json::json!({ "protocol":1, "command":"generate", "model_dir":root.join("models"), "output":output, "reference":reference_path, "consent":options.consent, "text":options.text, "language":options.language, "seed":options.seed, "exaggeration":options.exaggeration, "cfg_weight":options.cfg_weight, "device":options.device });
            let mut command = Command::new(python(&root));
            configure(&mut command, &root);
            command.arg(root.join("worker.py"));
            let mut details = serde_json::Value::Null;
            run_process(&mut command, Some(&request), &cancel, |event| {
                if event["kind"] == "progress" {
                    progress(
                        &app,
                        &task_id,
                        event["pct"].as_f64().unwrap_or(0.0),
                        event["stage"].as_str().unwrap_or("generating"),
                    );
                }
                if event["kind"] == "done" {
                    details = event.clone();
                }
            })?;
            cancelled(&cancel)?;
            let info = hound::WavReader::open(&output)?;
            if info.duration() < 2400
                || info.duration() > 24_000 * 610
                || info.spec().sample_rate != 24000
            {
                return Err(Error::Message("HALITE_STUDIO_OUTPUT".into()));
            }
            std::fs::create_dir_all(root.join("outputs"))?;
            let destination = root.join("outputs").join(format!("{task_id}.wav"));
            std::fs::rename(&output, &destination)?;
            let provenance = serde_json::json!({ "generator":"Halite", "engine":"Chatterbox Multilingual V3", "code_revision":manifest()?.code_revision, "model_revision":manifest()?.model_revision, "language":options.language, "seed":options.seed, "cloned_voice":reference.is_some(), "watermark":"PerTh", "created_at":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(), "details":details });
            std::fs::write(
                destination.with_extension("json"),
                serde_json::to_vec_pretty(&provenance)?,
            )?;
            app.asset_protocol_scope()
                .allow_file(&destination)
                .map_err(error)?;
            state
                .selected_files
                .lock()
                .map_err(error)?
                .insert(std::fs::canonicalize(&destination)?);
            Ok(Some(destination))
        })();
        state.clear_cancel(&task_id);
        state.studio_busy.store(false, Ordering::SeqCst);
        state.ai_busy.store(false, Ordering::SeqCst);
        finish(&app, task_id, result);
    });
    Ok(())
}

#[tauri::command]
pub fn cancel_studio(state: State<Arc<AppState>>, task_id: String) {
    state.request_cancel(&task_id);
}

#[tauri::command]
pub async fn verify_studio(state: State<'_, Arc<AppState>>) -> Result<()> {
    acquire(&state)?;
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| {
            let root = root(&state.data_dir);
            let cancel = AtomicBool::new(false);
            if !ready(&root) {
                return Err(Error::Message("HALITE_STUDIO_NOT_READY".into()));
            }
            for file in manifest()?.files {
                if hash_file(&root.join("models").join(file.name), &cancel)? != file.sha256 {
                    return Err(Error::Message("HALITE_STUDIO_CHECKSUM".into()));
                }
            }
            check_worker(&root, &cancel)
        })();
        state.studio_busy.store(false, Ordering::SeqCst);
        result
    })
    .await
    .map_err(error)?
}

#[tauri::command]
pub async fn remove_studio(state: State<'_, Arc<AppState>>) -> Result<()> {
    acquire(&state)?;
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| {
            let path = root(&state.data_dir);
            // Exact application-owned target. Voice profiles are stored separately.
            if path.is_dir() {
                std::fs::remove_dir_all(path)?;
            }
            Ok(())
        })();
        state.studio_busy.store(false, Ordering::SeqCst);
        result
    })
    .await
    .map_err(error)?
}

#[tauri::command]
pub async fn export_speech(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    path: String,
) -> Result<()> {
    let source = crate::metadata::require_selected(&state, &path)?;
    if !source.starts_with(root(&state.data_dir).join("outputs"))
        || source.extension().and_then(|s| s.to_str()) != Some("wav")
    {
        return Err(error("invalid output"));
    }
    if let Some(destination) = app
        .dialog()
        .file()
        .set_file_name("Halite Speech.wav")
        .add_filter("WAV", &["wav"])
        .blocking_save_file()
        .and_then(|file| file.into_path().ok())
    {
        let destination = destination.with_extension("wav");
        let mut temporary = tempfile::NamedTempFile::new_in(
            destination
                .parent()
                .ok_or_else(|| error("invalid destination"))?,
        )?;
        std::io::copy(&mut std::fs::File::open(&source)?, temporary.as_file_mut())?;
        temporary.as_file().sync_all()?;
        temporary.persist(&destination).map_err(error)?;
        std::fs::copy(
            source.with_extension("json"),
            destination.with_extension("halite.json"),
        )?;
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct VoiceProfile {
    id: String,
    name: String,
    path: String,
}
fn profile_id(id: &str) -> Result<()> {
    if id.is_empty() || id.len() > 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(error("invalid profile id"));
    }
    Ok(())
}
#[tauri::command]
pub async fn save_voice_profile(
    state: State<'_, Arc<AppState>>,
    path: String,
    name: String,
    consent: bool,
) -> Result<()> {
    if !consent {
        return Err(Error::Message("HALITE_STUDIO_CONSENT".into()));
    }
    if name.trim().is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
        return Err(error("invalid profile name"));
    }
    let source = crate::metadata::require_selected(&state, &path)?;
    let directory = state.data_dir.join("voice-profiles");
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::create_dir_all(&directory)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
        }
        let id = format!(
            "{:x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let path = directory.join(format!("{id}.wav"));
        normalized_reference(&source, &path)?;
        let profile = VoiceProfile {
            id: id.clone(),
            name: name.trim().into(),
            path: path.to_string_lossy().into(),
        };
        std::fs::write(
            directory.join(format!("{id}.json")),
            serde_json::to_vec(&profile)?,
        )?;
        Ok(())
    })
    .await
    .map_err(error)?
}
#[tauri::command]
pub fn list_voice_profiles(state: State<Arc<AppState>>) -> Result<Vec<VoiceProfile>> {
    let directory = state.data_dir.join("voice-profiles");
    let mut profiles = Vec::new();
    if directory.is_dir() {
        for entry in std::fs::read_dir(&directory)? {
            let path = entry?.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let profile: VoiceProfile = serde_json::from_slice(&std::fs::read(&path)?)?;
            profile_id(&profile.id)?;
            let expected = std::fs::canonicalize(directory.join(format!("{}.wav", profile.id)))?;
            if !expected.starts_with(std::fs::canonicalize(&directory)?) {
                return Err(error("profile escapes its directory"));
            }
            if std::fs::canonicalize(&profile.path)? != expected {
                return Err(error("invalid profile path"));
            }
            state.selected_files.lock().map_err(error)?.insert(expected);
            profiles.push(profile);
        }
    }
    Ok(profiles)
}
#[tauri::command]
pub fn delete_voice_profile(state: State<Arc<AppState>>, id: String) -> Result<()> {
    profile_id(&id)?;
    let directory = state.data_dir.join("voice-profiles");
    let path = directory.join(format!("{id}.wav"));
    state.selected_files.lock().map_err(error)?.remove(&path);
    if path.is_file() {
        std::fs::remove_file(path)?;
    }
    let path = directory.join(format!("{id}.json"));
    if path.is_file() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manifest_is_pinned_and_all_downloads_have_hashes() {
        let manifest = manifest().unwrap();
        assert_eq!(manifest.files.len(), 6);
        assert_eq!(manifest.model_revision.len(), 40);
        assert_eq!(manifest.code_revision.len(), 40);
        for asset in manifest
            .runtimes
            .values()
            .chain(std::iter::once(&manifest.source))
        {
            assert_eq!(asset.sha256.len(), 64);
            assert!(asset.url.starts_with("https://"));
        }
        for file in manifest.files {
            assert_eq!(file.sha256.len(), 64);
            assert!(!file.name.contains('/'));
            assert!(file.size > 0);
        }
    }
    #[test]
    fn consent_and_parameters_are_validated_before_starting() {
        let mut options = GenerateOptions {
            text: "Merhaba".into(),
            language: "tr".into(),
            seed: 42,
            exaggeration: 0.5,
            cfg_weight: 0.5,
            reference: None,
            consent: false,
            device: "auto".into(),
        };
        assert!(validate_options(&options).is_ok());
        options.reference = Some("reference.wav".into());
        assert!(validate_options(&options).is_err());
        options.consent = true;
        options.exaggeration = f64::NAN;
        assert!(validate_options(&options).is_err());
        assert!(profile_id("../private").is_err());
        assert!(validate_task_id("../output").is_err());
    }

    #[test]
    fn worker_cancellation_reaps_the_child_process() {
        let directory = tempfile::tempdir().unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let thread = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            signal.store(true, Ordering::SeqCst);
        });
        let mut command = Command::new(if cfg!(windows) { "python" } else { "python3" });
        configure(&mut command, directory.path());
        command.args(["-c", "import time; time.sleep(20)"]);
        let started = Instant::now();
        let result = run_process(&mut command, None, &cancel, |_| {});
        thread.join().unwrap();
        assert!(result.unwrap_err().to_string().contains("HALITE_CANCELLED"));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn worker_protocol_rejects_invalid_output() {
        let directory = tempfile::tempdir().unwrap();
        let mut command = Command::new(if cfg!(windows) { "python" } else { "python3" });
        configure(&mut command, directory.path());
        command.args(["-c", "print('{\"protocol\":99,\"kind\":\"done\"}')"]);
        let result = run_process(
            &mut command,
            Some(&serde_json::json!({"protocol":1,"command":"check"})),
            &AtomicBool::new(false),
            |_| {},
        );
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("HALITE_STUDIO_PROTOCOL"));
    }

    #[test]
    #[ignore = "requires a loopback HTTP server; exercised separately in CI"]
    fn partial_download_resumes_and_checksums_reject_corruption() {
        use std::net::TcpListener;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("model.bin");
        let data = b"verified model bytes";
        std::fs::write(path.with_extension("part"), &data[..5]).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut headers = String::new();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                headers.push_str(&line);
                if line == "\r\n" {
                    break;
                }
            }
            assert!(headers.to_ascii_lowercase().contains("range: bytes=5-"));
            write!(stream, "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes 5-{}/{}\r\nConnection: close\r\n\r\n", data.len() - 5, data.len() - 1, data.len()).unwrap();
            stream.write_all(&data[5..]).unwrap();
        });
        let asset = Asset {
            url: format!("http://{address}/model"),
            sha256: format!("{:x}", Sha256::digest(data)),
            size: data.len() as u64,
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime
            .block_on(download(
                &reqwest::Client::new(),
                &asset,
                &path,
                &AtomicBool::new(false),
                |_| {},
            ))
            .unwrap();
        server.join().unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), data);
        std::fs::remove_file(&path).unwrap();
        std::fs::write(path.with_extension("part"), vec![0; data.len()]).unwrap();
        let error = runtime
            .block_on(download(
                &reqwest::Client::new(),
                &asset,
                &path,
                &AtomicBool::new(false),
                |_| {},
            ))
            .unwrap_err();
        assert!(error.to_string().contains("HALITE_STUDIO_CHECKSUM"));
        assert!(!path.exists());
    }
    #[test]
    #[ignore = "downloads the managed runtime and 3.2 GB model; manual release smoke test"]
    fn install_real_studio() {
        let path = std::env::var("HALITE_STUDIO_TEST_DIR").expect("set isolated test data dir");
        let mut last = String::new();
        install(Path::new(&path), &AtomicBool::new(false), |pct, stage| {
            let label = format!("{stage} {:.0}%", pct * 100.0);
            if label != last {
                eprintln!("{label}");
                last = label;
            }
        })
        .unwrap();
        assert!(ready(&root(Path::new(&path))));
    }

    #[test]
    #[ignore = "uses the installed real model; manual release generation smoke test"]
    fn real_speech_smoke() {
        let path = std::env::var("HALITE_STUDIO_TEST_DIR").expect("set isolated test data dir");
        let root = root(Path::new(&path));
        let cancel = AtomicBool::new(false);
        assert!(ready(&root));
        for (language, text, reference) in [
            (
                "en",
                "Hello. This is a local voice created by Halite. Welcome to the voice studio.",
                None,
            ),
            (
                "tr",
                "Merhaba. Halite ile sesler cihazınızda güvenle oluşturulur.",
                None,
            ),
            (
                "en",
                "This voice is cloned from an authorized synthetic reference.",
                Some(root.join("smoke-en.wav")),
            ),
        ] {
            let output = root.join(format!(
                "smoke-{language}-{}.wav",
                if reference.is_some() { "clone" } else { "tts" }
            ));
            let request = serde_json::json!({ "protocol":1,"command":"generate","model_dir":root.join("models"),"output":output,"reference":reference,"consent":true,"text":text,"language":language,"seed":42,"exaggeration":0.5,"cfg_weight":0.5,"device":"cpu" });
            let mut command = Command::new(python(&root));
            configure(&mut command, &root);
            command.arg(root.join("worker.py"));
            run_process(&mut command, Some(&request), &cancel, |event| {
                eprintln!("{event}")
            })
            .unwrap();
            let mut wav = hound::WavReader::open(&output).unwrap();
            assert_eq!(wav.spec().sample_rate, 24000);
            assert!(wav.duration() > 2400);
            assert!(wav
                .samples::<i16>()
                .flatten()
                .any(|sample| sample.unsigned_abs() > 100));
            if language == "en" && reference.is_none() {
                std::fs::copy(&output, root.join("smoke-en.wav")).unwrap();
            }
        }
    }
}
