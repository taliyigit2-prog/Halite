use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::error::{Error, Result};

pub const MODEL_SAMPLE_RATE: u32 = 44_100;

/// Decode any supported audio file into interleaved stereo f32 samples.
pub fn decode_to_stereo(path: &Path) -> Result<(Vec<f32>, u32)> {
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let hint = Hint::new();
    let mut probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let track = probed
        .format
        .default_track()
        .ok_or_else(|| Error::Message("no audio track".to_string()))?;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(MODEL_SAMPLE_RATE);

    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    let mut out: Vec<f32> = Vec::new();
    let track_id = track.id;

    loop {
        let packet = match probed.format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break
            }
            Err(SymphoniaError::ResetRequired) => continue,
            Err(e) => return Err(Error::Symphonia(e)),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(Error::Symphonia(e)),
        };

        let spec = *decoded.spec();
        let mut buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        buf.copy_interleaved_ref(decoded);

        let n_channels = spec.channels.count();
        for frame in buf.samples().chunks_exact(n_channels) {
            let l = frame[0];
            let r = if n_channels >= 2 { frame[1] } else { l };
            out.push(l);
            out.push(r);
        }
    }

    if out.is_empty() {
        return Err(Error::Message("decoded zero audio frames".to_string()));
    }

    Ok((out, sample_rate))
}

/// Linear-interpolation resampler for interleaved stereo samples.
pub fn resample_stereo(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }
    let n_frames = samples.len() / 2;
    let out_frames = ((n_frames as f64 * to_rate as f64 / from_rate as f64).round()) as usize;
    let ratio = from_rate as f64 / to_rate as f64;
    let mut out = Vec::with_capacity(out_frames * 2);
    for i in 0..out_frames {
        let pos = i as f64 * ratio;
        let idx = pos.floor() as usize;
        let frac = (pos - idx as f64) as f32;
        let i0 = idx.min(n_frames.saturating_sub(1));
        let i1 = (idx + 1).min(n_frames.saturating_sub(1));
        let l = samples[i0 * 2] * (1.0 - frac) + samples[i1 * 2] * frac;
        let r = samples[i0 * 2 + 1] * (1.0 - frac) + samples[i1 * 2 + 1] * frac;
        out.push(l);
        out.push(r);
    }
    out
}

/// Trim stereo samples to the given [start_sec, end_sec) range.
pub fn trim_stereo(
    samples: &[f32],
    sample_rate: u32,
    start_sec: f64,
    end_sec: Option<f64>,
) -> Vec<f32> {
    let total = (samples.len() / 2) as f64 / sample_rate as f64;
    let start = (start_sec.max(0.0) * sample_rate as f64) as usize * 2;
    let end = match end_sec {
        Some(e) => ((e.min(total) * sample_rate as f64) as usize * 2).max(start),
        None => samples.len(),
    };
    samples
        .get(start.min(samples.len())..end.min(samples.len()))
        .unwrap_or(&[])
        .to_vec()
}

/// Write interleaved stereo f32 samples to a 16-bit PCM WAV file.
pub fn write_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<i64> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        writer.write_sample(v)?;
    }
    writer.finalize()?;
    let size = std::fs::metadata(path)?.len() as i64;
    Ok(size)
}

fn ffmpeg_on_path() -> Option<std::path::PathBuf> {
    let candidates: &[&str] = {
        #[cfg(target_os = "macos")]
        {
            &[
                "/opt/homebrew/bin/ffmpeg",
                "/usr/local/bin/ffmpeg",
                "/usr/bin/ffmpeg",
            ]
        }
        #[cfg(target_os = "windows")]
        {
            &[]
        }
        #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
        {
            &["/usr/bin/ffmpeg", "/usr/local/bin/ffmpeg"]
        }
    };
    for c in candidates {
        let p = std::path::Path::new(c);
        if ffmpeg_is_usable(p) {
            return Some(p.to_path_buf());
        }
    }
    let executable = if cfg!(target_os = "windows") {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(executable);
            if ffmpeg_is_usable(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

fn ffmpeg_is_usable(path: &Path) -> bool {
    path.is_file()
        && std::process::Command::new(path)
            .arg("-version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
}

static FFMPEG_INSTALL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn ffmpeg_path(
    data_dir: &Path,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<std::path::PathBuf> {
    if cancel.load(std::sync::atomic::Ordering::SeqCst) {
        return Err(Error::Message("HALITE_CANCELLED".to_string()));
    }
    // Prefer an already-installed ffmpeg (no download needed).
    if let Some(p) = ffmpeg_on_path() {
        return Ok(p);
    }

    let bin_dir = data_dir.join("bin");
    let executable = if cfg!(target_os = "windows") {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    let target = bin_dir.join(executable);
    if ffmpeg_is_usable(&target) {
        return Ok(target);
    }

    // ffmpeg-sidecar's convenience installer writes next to the application
    // binary, which is read-only after a normal macOS/Windows installation.
    // Install explicitly into the per-user application data directory instead.
    let _guard = FFMPEG_INSTALL_LOCK
        .lock()
        .map_err(|_| Error::Ffmpeg("HALITE_FFMPEG_SETUP|installer lock failed".to_string()))?;
    if cancel.load(std::sync::atomic::Ordering::SeqCst) {
        return Err(Error::Message("HALITE_CANCELLED".to_string()));
    }
    if ffmpeg_is_usable(&target) {
        return Ok(target);
    }

    std::fs::create_dir_all(&bin_dir)
        .map_err(|e| Error::Ffmpeg(format!("HALITE_FFMPEG_SETUP|{e}")))?;
    let url = ffmpeg_sidecar::download::ffmpeg_download_url()
        .map_err(|e| Error::Ffmpeg(format!("HALITE_FFMPEG_SETUP|{e}")))?;
    let staging = bin_dir.join(".ffmpeg-install");
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging)
        .map_err(|e| Error::Ffmpeg(format!("HALITE_FFMPEG_SETUP|{e}")))?;
    let install_result = (|| -> Result<()> {
        let archive = ffmpeg_sidecar::download::download_ffmpeg_package(url, &staging)
            .map_err(|e| Error::Ffmpeg(format!("HALITE_FFMPEG_SETUP|{e}")))?;
        ffmpeg_sidecar::download::unpack_ffmpeg_without_extras(&archive, &staging)
            .map_err(|e| Error::Ffmpeg(format!("HALITE_FFMPEG_SETUP|{e}")))?;
        let staged_binary = staging.join(executable);
        if !ffmpeg_is_usable(&staged_binary) {
            return Err(Error::Ffmpeg(
                "HALITE_FFMPEG_SETUP|download completed but ffmpeg was not usable".to_string(),
            ));
        }
        if target.exists() {
            std::fs::remove_file(&target)
                .map_err(|e| Error::Ffmpeg(format!("HALITE_FFMPEG_SETUP|{e}")))?;
        }
        std::fs::rename(staged_binary, &target)
            .map_err(|e| Error::Ffmpeg(format!("HALITE_FFMPEG_SETUP|{e}")))?;
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&staging);
    install_result?;

    if !ffmpeg_is_usable(&target) {
        return Err(Error::Ffmpeg(
            "HALITE_FFMPEG_SETUP|download completed but ffmpeg was not found".to_string(),
        ));
    }
    Ok(target)
}

/// Transcode a WAV file to the requested format using ffmpeg.
pub fn transcode(
    data_dir: &Path,
    input: &Path,
    output: &Path,
    format: &str,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<()> {
    let ffmpeg = ffmpeg_path(data_dir, cancel)?;
    let codec = match format {
        "mp3" => "libmp3lame",
        "flac" => "flac",
        "m4a" | "aac" | "mp4" => "aac",
        _ => "pcm_s16le",
    };
    let mut child = std::process::Command::new(ffmpeg)
        .arg("-y")
        .arg("-nostdin")
        .arg("-nostats")
        .arg("-loglevel")
        .arg("error")
        .arg("-i")
        .arg(input)
        .arg("-codec:a")
        .arg(codec)
        .arg(output)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| Error::Ffmpeg(format!("HALITE_FFMPEG_FAILED|{e}")))?;
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::Ffmpeg(
                "HALITE_FFMPEG_FAILED|stderr unavailable".to_string(),
            ));
        }
    };
    let stderr_thread = std::thread::spawn(move || {
        use std::io::Read;
        let mut text = String::new();
        let _ = std::io::BufReader::new(stderr).read_to_string(&mut text);
        text
    });
    let status = loop {
        if cancel.load(std::sync::atomic::Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stderr_thread.join();
            let _ = std::fs::remove_file(output);
            return Err(Error::Message("HALITE_CANCELLED".to_string()));
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(75)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stderr_thread.join();
                return Err(Error::Ffmpeg(format!("HALITE_FFMPEG_FAILED|{error}")));
            }
        }
    };
    let stderr = stderr_thread.join().unwrap_or_default();
    if !status.success() {
        let detail = stderr
            .lines()
            .last()
            .unwrap_or("unknown ffmpeg error")
            .trim();
        return Err(Error::Ffmpeg(format!(
            "HALITE_FFMPEG_FAILED|ffmpeg exited with status {:?}: {detail}",
            status.code()
        )));
    }
    if std::fs::metadata(output)
        .map(|meta| meta.len())
        .unwrap_or(0)
        == 0
    {
        return Err(Error::Ffmpeg(
            "HALITE_FFMPEG_FAILED|ffmpeg reported success but produced no output".to_string(),
        ));
    }
    Ok(())
}
