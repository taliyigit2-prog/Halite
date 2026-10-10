use std::ffi::OsString;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadResult {
    pub title: String,
    pub path: String,
    pub ext: String,
}

static YTDLP_INSTALL_LOCK: Mutex<()> = Mutex::new(());

fn ytdlp_asset() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "yt-dlp_macos"
    }
    #[cfg(target_os = "windows")]
    {
        "yt-dlp.exe"
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "yt-dlp_linux_aarch64"
    }
    #[cfg(all(target_os = "linux", not(target_arch = "aarch64")))]
    {
        "yt-dlp_linux"
    }
}

fn ytdlp_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "yt-dlp.exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "yt-dlp"
    }
}

pub fn ytdlp_path(data_dir: &Path) -> PathBuf {
    data_dir.join("bin").join(ytdlp_name())
}

fn ytdlp_is_usable(path: &Path) -> bool {
    path.is_file()
        && std::process::Command::new(path)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
}

fn checksum_from_manifest(manifest: &str, asset: &str) -> Option<String> {
    manifest.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let digest = parts.next()?;
        let name = parts.next()?.trim_start_matches('*');
        (name == asset && digest.len() == 64).then(|| digest.to_ascii_lowercase())
    })
}

/// Ensure the official yt-dlp binary is present in Halite's writable data
/// directory. The release checksum is verified before the atomic rename.
pub fn ensure_ytdlp(
    data_dir: &Path,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(f64),
) -> Result<PathBuf> {
    if cancel.load(Ordering::SeqCst) {
        return Err(Error::Message("HALITE_CANCELLED".to_string()));
    }
    let path = ytdlp_path(data_dir);
    if ytdlp_is_usable(&path) {
        return Ok(path);
    }

    let _guard = YTDLP_INSTALL_LOCK
        .lock()
        .map_err(|_| Error::Message("HALITE_HELPER_SETUP|installer lock failed".to_string()))?;
    if cancel.load(Ordering::SeqCst) {
        return Err(Error::Message("HALITE_CANCELLED".to_string()));
    }
    if ytdlp_is_usable(&path) {
        return Ok(path);
    }
    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| Error::Message(format!("HALITE_HELPER_SETUP|{e}")))?;
    }

    let dir = path
        .parent()
        .ok_or_else(|| Error::Message("HALITE_HELPER_SETUP|invalid helper path".to_string()))?;
    std::fs::create_dir_all(dir)?;
    let asset = ytdlp_asset();
    let url = format!("https://github.com/yt-dlp/yt-dlp/releases/latest/download/{asset}");
    let sums_url = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/SHA2-256SUMS";
    let tmp = path.with_extension("part");
    let _ = std::fs::remove_file(&tmp);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Error::Message(format!("HALITE_HELPER_SETUP|{e}")))?;

    let result = runtime.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30 * 60))
            .user_agent("Halite/0.1")
            .build()?;
        let manifest = client
            .get(sums_url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let expected = checksum_from_manifest(&manifest, asset)
            .ok_or_else(|| Error::Message("HALITE_HELPER_SETUP|checksum not found".to_string()))?;

        let mut response = client.get(&url).send().await?.error_for_status()?;
        let total = response.content_length().unwrap_or(20_000_000);
        let mut file = tokio::fs::File::create(&tmp).await?;
        let mut downloaded = 0u64;
        let mut hash = Sha256::new();
        while let Some(chunk) = response.chunk().await? {
            if cancel.load(Ordering::SeqCst) {
                return Err(Error::Message("HALITE_CANCELLED".to_string()));
            }
            use tokio::io::AsyncWriteExt;
            file.write_all(&chunk).await?;
            hash.update(&chunk);
            downloaded += chunk.len() as u64;
            on_progress((downloaded as f64 / total.max(1) as f64).clamp(0.0, 1.0));
        }
        file.sync_all().await?;
        let actual = format!("{:x}", hash.finalize());
        if actual != expected {
            return Err(Error::Message(
                "HALITE_HELPER_SETUP|yt-dlp checksum verification failed".to_string(),
            ));
        }
        tokio::fs::rename(&tmp, &path).await?;
        Ok::<(), Error>(())
    });

    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result?;

    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(path)
}

fn executable_on_path(names: &[&str]) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        for name in names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn js_runtime_arg() -> Option<String> {
    let candidates: &[(&str, &[&str])] = if cfg!(target_os = "windows") {
        &[
            ("deno", &["deno.exe"]),
            ("node", &["node.exe"]),
            ("bun", &["bun.exe"]),
        ]
    } else {
        &[("deno", &["deno"]), ("node", &["node"]), ("bun", &["bun"])]
    };
    candidates.iter().find_map(|(kind, names)| {
        executable_on_path(names).map(|path| format!("{kind}:{}", path.to_string_lossy()))
    })
}

fn validate_request(url: &str, format: &str) -> Result<()> {
    let parsed =
        reqwest::Url::parse(url).map_err(|_| Error::Message("HALITE_INVALID_URL".to_string()))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(Error::Message("HALITE_INVALID_URL".to_string()));
    }
    if !matches!(format, "mp3" | "m4a" | "wav" | "flac") {
        return Err(Error::Message("HALITE_INVALID_FORMAT".to_string()));
    }
    Ok(())
}

fn classify_ytdlp_error(stderr: &str, code: Option<i32>) -> Error {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("sign in to confirm your age") || lower.contains("age-restricted") {
        return Error::Message("HALITE_AGE_RESTRICTED".to_string());
    }
    if lower.contains("login required")
        || lower.contains("sign in")
        || lower.contains("cookies-from-browser")
    {
        return Error::Message("HALITE_AUTH_REQUIRED".to_string());
    }
    if lower.contains("javascript runtime") || lower.contains("js runtime") {
        return Error::Message("HALITE_JS_RUNTIME_REQUIRED".to_string());
    }
    let detail = stderr
        .lines()
        .rev()
        .find(|line| line.contains("ERROR:"))
        .or_else(|| stderr.lines().rev().find(|line| !line.trim().is_empty()))
        .unwrap_or("yt-dlp failed")
        .trim();
    Error::Message(format!("HALITE_DOWNLOAD_FAILED|exit={code:?}|{detail}"))
}

fn handle_stdout_line(
    line: &str,
    title: &mut String,
    final_path: &mut String,
    on_progress: &mut impl FnMut(f64),
) {
    if let Some(raw) = line.strip_prefix("HALITE_PROGRESS:") {
        if let Ok(pct) = raw.trim().trim_end_matches('%').trim().parse::<f64>() {
            on_progress(0.1 + (pct / 100.0).clamp(0.0, 1.0) * 0.9);
        }
    } else if let Some(raw) = line.strip_prefix("HALITE_TITLE:") {
        *title = raw.trim().to_string();
    } else if let Some(raw) = line.strip_prefix("HALITE_FILE:") {
        *final_path = raw.trim().to_string();
    }
}

struct DownloadTempGuard(PathBuf);

impl Drop for DownloadTempGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn terminate_process_tree(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        // The child is started as the leader of its own process group, so this
        // also stops ffmpeg or a JavaScript runtime launched by yt-dlp.
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
    }
    #[cfg(windows)]
    {
        let pid = child.id().to_string();
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", pid.as_str(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// Download audio from a URL using yt-dlp, with deterministic output capture,
/// concurrent pipe draining and cancellation that does not depend on new output.
pub fn download(
    data_dir: &Path,
    task_id: &str,
    url: &str,
    output_dir: &Path,
    format: &str,
    cancel: Arc<AtomicBool>,
    mut on_progress: impl FnMut(f64) + Send,
) -> Result<DownloadResult> {
    validate_request(url, format)?;
    let ytdlp = ensure_ytdlp(data_dir, &cancel, |pct| on_progress(pct * 0.08))?;
    on_progress(0.08);
    let ffmpeg = crate::audio::ffmpeg_path(data_dir, &cancel)?;
    if cancel.load(Ordering::SeqCst) {
        return Err(Error::Message("HALITE_CANCELLED".to_string()));
    }
    on_progress(0.1);

    std::fs::create_dir_all(output_dir)
        .map_err(|e| Error::Message(format!("HALITE_OUTPUT|{e}")))?;
    if !output_dir.is_dir() {
        return Err(Error::Message("HALITE_OUTPUT|not a directory".to_string()));
    }

    let temp_dir = data_dir.join("tmp").join(format!("download-{task_id}"));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).map_err(|e| Error::Message(format!("HALITE_OUTPUT|{e}")))?;
    let _temp_guard = DownloadTempGuard(temp_dir.clone());

    let output_template = output_dir.join("%(title).180B [%(id)s].%(ext)s");
    let mut args: Vec<OsString> = vec![
        "--no-config".into(),
        "--no-playlist".into(),
        "--no-simulate".into(),
        "-f".into(),
        "bestaudio/best".into(),
        "-x".into(),
        "--audio-format".into(),
        format.into(),
        "--audio-quality".into(),
        "0".into(),
        "--ffmpeg-location".into(),
        ffmpeg.into_os_string(),
        "--newline".into(),
        "--progress-template".into(),
        "download:HALITE_PROGRESS:%(progress._percent_str)s".into(),
        "--print".into(),
        "before_dl:HALITE_TITLE:%(title)s".into(),
        "--print".into(),
        "after_move:HALITE_FILE:%(filepath)s".into(),
        "-o".into(),
        output_template.into_os_string(),
        "-P".into(),
        format!("temp:{}", temp_dir.to_string_lossy()).into(),
    ];
    if let Some(runtime) = js_runtime_arg() {
        args.push("--js-runtimes".into());
        args.push(runtime.into());
    }
    args.push("--".into());
    args.push(url.into());

    let mut command = std::process::Command::new(&ytdlp);
    command
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|e| Error::Message(format!("HALITE_HELPER_SETUP|{e}")))?;

    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            terminate_process_tree(&mut child);
            return Err(Error::Message(
                "HALITE_DOWNLOAD_FAILED|stdout unavailable".to_string(),
            ));
        }
    };
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            terminate_process_tree(&mut child);
            return Err(Error::Message(
                "HALITE_DOWNLOAD_FAILED|stderr unavailable".to_string(),
            ));
        }
    };

    let (line_tx, line_rx) = mpsc::channel();
    let stdout_thread = std::thread::spawn(move || {
        for line in BufReader::new(stdout)
            .lines()
            .map_while(std::result::Result::ok)
        {
            if line_tx.send(line).is_err() {
                break;
            }
        }
    });
    let stderr_thread = std::thread::spawn(move || {
        use std::io::Read;
        let mut text = String::new();
        let _ = BufReader::new(stderr).read_to_string(&mut text);
        text
    });

    let mut title = String::new();
    let mut final_path = String::new();
    let status = loop {
        for line in line_rx.try_iter() {
            handle_stdout_line(&line, &mut title, &mut final_path, &mut on_progress);
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => {
                terminate_process_tree(&mut child);
                return Err(Error::Message(format!("HALITE_DOWNLOAD_FAILED|{error}")));
            }
        }
        if cancel.load(Ordering::SeqCst) {
            terminate_process_tree(&mut child);
            let _ = stdout_thread.join();
            let _ = stderr_thread.join();
            return Err(Error::Message("HALITE_CANCELLED".to_string()));
        }
        std::thread::sleep(Duration::from_millis(75));
    };

    let _ = stdout_thread.join();
    for line in line_rx.try_iter() {
        handle_stdout_line(&line, &mut title, &mut final_path, &mut on_progress);
    }
    let stderr_text = stderr_thread.join().unwrap_or_default();

    if !status.success() {
        return Err(classify_ytdlp_error(&stderr_text, status.code()));
    }
    let path = PathBuf::from(&final_path);
    if final_path.is_empty()
        || !path.is_file()
        || std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0) == 0
    {
        return Err(Error::Message("HALITE_OUTPUT_MISSING".to_string()));
    }
    let canonical_output = output_dir
        .canonicalize()
        .map_err(|e| Error::Message(format!("HALITE_OUTPUT|{e}")))?;
    let canonical_file = path
        .canonicalize()
        .map_err(|e| Error::Message(format!("HALITE_OUTPUT|{e}")))?;
    if !canonical_file.starts_with(&canonical_output) {
        return Err(Error::Message(
            "HALITE_OUTPUT|unexpected output path".to_string(),
        ));
    }

    let ext = path
        .extension()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| format.to_string());
    if title.is_empty() {
        title = path
            .file_stem()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default();
    }
    on_progress(1.0);

    Ok(DownloadResult {
        title,
        path: final_path,
        ext,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_release_checksum() {
        let manifest = "abc  other\n0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  yt-dlp_macos\n";
        assert_eq!(
            checksum_from_manifest(manifest, "yt-dlp_macos").as_deref(),
            Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
        );
    }

    #[test]
    fn validates_supported_requests() {
        assert!(validate_request("https://example.com/watch?v=1", "mp3").is_ok());
        assert!(validate_request("file:///tmp/song", "mp3").is_err());
        assert!(validate_request("https://example.com", "exe").is_err());
    }
}
