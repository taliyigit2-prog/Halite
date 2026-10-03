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
    let sample_rate = track
        .codec_params
        .sample_rate
        .unwrap_or(MODEL_SAMPLE_RATE);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())?;

    let mut out: Vec<f32> = Vec::new();
    let track_id = track.id;

    loop {
        let packet = match probed.format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
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
pub fn trim_stereo(samples: &[f32], sample_rate: u32, start_sec: f64, end_sec: Option<f64>) -> Vec<f32> {
    let total = (samples.len() / 2) as f64 / sample_rate as f64;
    let start = (start_sec.max(0.0) * sample_rate as f64) as usize * 2;
    let end = match end_sec {
        Some(e) => ((e.min(total) * sample_rate as f64) as usize * 2).max(start),
        None => samples.len(),
    };
    samples.get(start.min(samples.len())..end.min(samples.len()))
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

pub fn ffmpeg_path() -> Result<std::path::PathBuf> {
    let _ = ffmpeg_sidecar::download::auto_download();
    Ok(ffmpeg_sidecar::paths::ffmpeg_path())
}

/// Transcode a WAV file to the requested format using ffmpeg.
pub fn transcode(input: &Path, output: &Path, format: &str) -> Result<()> {
    let ffmpeg = ffmpeg_path()?;
    let codec = match format {
        "mp3" => "libmp3lame",
        "flac" => "flac",
        "m4a" | "aac" | "mp4" => "aac",
        _ => "pcm_s16le",
    };
    let status = std::process::Command::new(ffmpeg)
        .args([
            "-y",
            "-i",
            input.to_str().unwrap_or(""),
            "-codec:a",
            codec,
            output.to_str().unwrap_or(""),
        ])
        .status()
        .map_err(|e| Error::Ffmpeg(e.to_string()))?;
    if !status.success() {
        return Err(Error::Ffmpeg(format!(
            "ffmpeg exited with status {:?}",
            status.code()
        )));
    }
    Ok(())
}
