use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analysis {
    pub bpm: f64,
    pub key: String,
    pub key_camelot: Option<String>,
}

const KEY_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

const MAJOR_PROFILE: [f32; 12] = [
    6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
];
const MINOR_PROFILE: [f32; 12] = [
    6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
];

/// Analyze BPM and musical key of interleaved stereo samples.
pub fn analyze(samples: &[f32], sample_rate: u32) -> Result<Analysis> {
    let mono = downmix(samples);
    let bpm = estimate_bpm(&mono, sample_rate);
    let key = estimate_key(&mono, sample_rate);
    let camelot = key.as_deref().and_then(camelot);
    Ok(Analysis {
        bpm,
        key: key.unwrap_or_else(|| "—".to_string()),
        key_camelot: camelot,
    })
}

fn downmix(samples: &[f32]) -> Vec<f32> {
    let n = samples.len() / 2;
    (0..n).map(|i| (samples[i * 2] + samples[i * 2 + 1]) * 0.5).collect()
}

fn estimate_bpm(mono: &[f32], sample_rate: u32) -> f64 {
    let hop = 512usize;
    let frame = 1024usize;
    let frame_rate = sample_rate as f64 / hop as f64;

    let n_frames = mono.len().saturating_sub(frame) / hop + 1;
    if n_frames < 2 {
        return 0.0;
    }

    // Onset strength via energy flux.
    let mut energy = Vec::with_capacity(n_frames);
    for i in 0..n_frames {
        let start = i * hop;
        let mut e = 0.0f32;
        for j in 0..frame {
            let v = mono[start + j];
            e += v * v;
        }
        energy.push(e / frame as f32);
    }

    let mut onset = Vec::with_capacity(n_frames - 1);
    for i in 1..n_frames {
        onset.push((energy[i] - energy[i - 1]).max(0.0));
    }
    let onset = onset;

    let min_bpm = 60.0;
    let max_bpm = 200.0;
    let min_lag = (frame_rate / max_bpm).round() as usize;
    let max_lag = (frame_rate / min_bpm).round() as usize;

    if max_lag >= onset.len() {
        return 0.0;
    }

    let mut best_lag = min_lag;
    let mut best_score = f64::MIN;
    for lag in min_lag..=max_lag {
        let mut score = 0.0f64;
        for i in 0..(onset.len() - lag) {
            score += onset[i] as f64 * onset[i + lag] as f64;
        }
        if score > best_score {
            best_score = score;
            best_lag = lag;
        }
    }

    if best_score <= 0.0 {
        return 0.0;
    }

    let mut bpm = 60.0 * frame_rate / best_lag as f64;
    // Simple octave fold to keep the result in a musical range.
    while bpm < min_bpm {
        bpm *= 2.0;
    }
    while bpm > max_bpm {
        bpm /= 2.0;
    }
    (bpm * 10.0).round() / 10.0
}

fn estimate_key(mono: &[f32], sample_rate: u32) -> Option<String> {
    let n_fft = 4096usize;
    let hop = n_fft / 2;
    if mono.len() < n_fft {
        return None;
    }

    let n_frames = (mono.len() - n_fft) / hop + 1;
    let mut chroma = [0f32; 12];

    for f in 0..n_frames {
        let start = f * hop;
        let (re, im) = fft_frame(&mono[start..start + n_fft]);
        for k in 1..(n_fft / 2) {
            let freq = k as f32 * sample_rate as f32 / n_fft as f32;
            let midi = 69.0 + 12.0 * (freq / 440.0).log2();
            let pc = (midi.round() as i32).rem_euclid(12) as usize;
            let mag = (re[k] * re[k] + im[k] * im[k]).sqrt();
            chroma[pc] += mag;
        }
    }

    let sum: f32 = chroma.iter().sum();
    if sum <= 0.0 {
        return None;
    }
    for c in chroma.iter_mut() {
        *c /= sum;
    }

    let (best_major, best_minor) = match_profiles(&chroma);
    if best_major.0 > best_minor.0 {
        Some(format!("{} major", KEY_NAMES[best_major.1]))
    } else {
        Some(format!("{} minor", KEY_NAMES[best_minor.1]))
    }
}

fn fft_frame(frame: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let n = frame.len();
    // Apply Hann window and compute naive DFT (n=4096 is fine for one-shot analysis).
    let mut re = vec![0f32; n / 2];
    let mut im = vec![0f32; n / 2];
    for k in 0..(n / 2) {
        let mut sr = 0f32;
        let mut si = 0f32;
        for (t, &x) in frame.iter().enumerate() {
            let w = 0.5 * (1.0 - (2.0 * PI * t as f32 / n as f32).cos());
            let ang = 2.0 * PI * k as f32 * t as f32 / n as f32;
            sr += x * w * ang.cos();
            si -= x * w * ang.sin();
        }
        re[k] = sr;
        im[k] = si;
    }
    (re, im)
}

fn match_profiles(chroma: &[f32; 12]) -> ((f32, usize), (f32, usize)) {
    let mut best_major = (f32::MIN, 0usize);
    let mut best_minor = (f32::MIN, 0usize);
    for tonic in 0..12 {
        let mut sm = 0f32;
        let mut sn = 0f32;
        for i in 0..12 {
            let idx = (i + tonic) % 12;
            sm += chroma[i] * MAJOR_PROFILE[idx];
            sn += chroma[i] * MINOR_PROFILE[idx];
        }
        if sm > best_major.0 {
            best_major = (sm, tonic);
        }
        if sn > best_minor.0 {
            best_minor = (sn, tonic);
        }
    }
    (best_major, best_minor)
}

fn camelot(key: &str) -> Option<String> {
    let key = key.trim();
    let mut parts = key.split_whitespace();
    let note = parts.next()?;
    let mode = parts.next()?;
    let idx = KEY_NAMES.iter().position(|n| *n == note)?;
    let number = if mode == "major" {
        (idx * 7) % 12 + 1
    } else {
        (idx * 7 + 3) % 12 + 1
    };
    let letter = if mode == "major" { "B" } else { "A" };
    Some(format!("{number}{letter}"))
}
