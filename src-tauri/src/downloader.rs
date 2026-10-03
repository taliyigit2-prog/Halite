use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadResult {
    pub title: String,
    pub path: String,
    pub ext: String,
}

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

/// Ensure the yt-dlp binary is present, downloading it from the official releases if needed.
pub fn ensure_ytdlp(data_dir: &Path, on_progress: impl FnMut(f64)) -> Result<PathBuf> {
    let path = ytdlp_path(data_dir);
    if path.exists() {
        return Ok(path);
    }

    let dir = path.parent().unwrap();
    std::fs::create_dir_all(dir)?;
    let url = format!(
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/{}",
        ytdlp_asset()
    );

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Error::Message(e.to_string()))?;

    let tmp = path.with_extension("part");
    let mut on_progress = on_progress;
    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30 * 60))
            .build()?;
        let mut resp = client.get(&url).send().await?.error_for_status()?;
        let total = resp.content_length().unwrap_or(20_000_000);
        let mut file = tokio::fs::File::create(&tmp).await?;
        let mut downloaded: u64 = 0;
        while let Some(chunk) = resp.chunk().await? {
            use tokio::io::AsyncWriteExt;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            on_progress(downloaded as f64 / total.max(1) as f64);
        }
        file.sync_all().await?;
        tokio::fs::rename(&tmp, &path).await?;
        #[cfg(not(target_os = "windows"))]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755));
        }
        Ok::<_, Error>(())
    })?;

    Ok(path)
}

pub fn is_ytdlp_installed(data_dir: &Path) -> bool {
    ytdlp_path(data_dir).exists()
}

/// Download audio from a URL using yt-dlp, with progress and cancellation.
pub fn download(
    data_dir: &Path,
    url: &str,
    output_dir: &Path,
    format: &str,
    cancel: Arc<AtomicBool>,
    mut on_progress: impl FnMut(f64) + Send,
) -> Result<DownloadResult> {
    let ytdlp = ensure_ytdlp(data_dir, |_| {})?;
    let ffmpeg = crate::audio::ffmpeg_path()?;

    std::fs::create_dir_all(output_dir)?;

    let mut args: Vec<String> = vec![
        "-f".to_string(),
        "bestaudio/best".to_string(),
        "-x".to_string(),
        "--audio-format".to_string(),
        format.to_string(),
        "--audio-quality".to_string(),
        "0".to_string(),
        "--ffmpeg-location".to_string(),
        ffmpeg.to_string_lossy().to_string(),
        "--newline".to_string(),
        "--no-playlist".to_string(),
        "-o".to_string(),
        output_dir
            .join("%(title)s [%(id)s].%(ext)s")
            .to_string_lossy()
            .to_string(),
        url.to_string(),
    ];

    // Skip any cookie/warning prompts.
    args.insert(0, "--no-warnings".to_string());

    let child = std::process::Command::new(&ytdlp)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Error::Message(format!("failed to start yt-dlp: {e}")))?;

    let stdout = child.stdout.ok_or_else(|| Error::Message("no stdout".to_string()))?;
    let stderr = child.stderr.ok_or_else(|| Error::Message("no stderr".to_string()))?;

    use std::io::{BufRead, BufReader};
    let mut title = String::new();
    let mut final_path = String::new();

    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        if cancel.load(Ordering::SeqCst) {
            break;
        }
        let line = line.unwrap_or_default();
        if let Some(p) = parse_progress(&line) {
            on_progress(p);
        }
        if let Some(t) = parse_destination(&line) {
            title = t;
        }
        if is_path(&line) {
            final_path = line.clone();
        }
    }

    // Read stderr for error reporting (non-blocking best effort).
    let stderr_text = {
        let mut s = String::new();
        let _ = std::io::Read::read_to_string(&mut BufReader::new(stderr).into_inner(), &mut s);
        s
    };

    if cancel.load(Ordering::SeqCst) {
        return Err(Error::Message("cancelled".to_string()));
    }

    // Best effort: if we couldn't capture the path, find the newest file.
    if final_path.is_empty() || !Path::new(&final_path).exists() {
        if let Some(p) = newest_file(output_dir, format) {
            final_path = p;
        }
    }

    if final_path.is_empty() {
        let msg = stderr_text
            .lines()
            .filter(|l| l.contains("ERROR") || l.contains("error"))
            .map(|l| l.to_string())
            .next()
            .unwrap_or_else(|| "download failed".to_string());
        return Err(Error::Message(msg));
    }

    let path = PathBuf::from(&final_path);
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_else(|| format.to_string());

    on_progress(1.0);

    Ok(DownloadResult {
        title: if title.is_empty() {
            path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        } else {
            title
        },
        path: final_path,
        ext,
    })
}

fn parse_progress(line: &str) -> Option<f64> {
    let idx = line.find("% of")?;
    let mut start = idx;
    let bytes = line.as_bytes();
    while start > 0 && (bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b'.') {
        start -= 1;
    }
    let num = &line[start..idx];
    num.parse::<f64>().ok().map(|v| v / 100.0)
}

fn parse_destination(line: &str) -> Option<String> {
    let marker = "[download] Destination: ";
    let idx = line.find(marker)?;
    let start = idx + marker.len();
    let raw = &line[start..];
    Some(
        Path::new(raw.trim())
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default(),
    )
}

fn is_path(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with('[')
        && (Path::new(trimmed).is_absolute() || trimmed.contains('/') || trimmed.contains('\\'))
        && !trimmed.contains(' ')
}

fn newest_file(dir: &Path, format: &str) -> Option<String> {
    let ext = match format {
        "m4a" | "mp4" => "m4a",
        other => other,
    };
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == ext).unwrap_or(false) {
                if let Ok(m) = p.metadata().and_then(|m| m.modified()) {
                    if best.as_ref().map(|(t, _)| m > *t).unwrap_or(true) {
                        best = Some((m, p));
                    }
                }
            }
        }
    }
    best.map(|(_, p)| p.to_string_lossy().to_string())
}
