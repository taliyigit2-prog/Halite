use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analysis {
    pub bpm: f64,
    pub key: String,
    pub key_tonic: Option<String>,
    pub key_mode: Option<String>,
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
    let detected = estimate_key(&mono, sample_rate);
    let (key, key_tonic, key_mode, key_camelot) = match detected {
        Some((tonic, mode)) => {
            let notation = format!("{tonic} {mode}");
            let camelot = camelot(&tonic, &mode);
            (notation, Some(tonic), Some(mode), camelot)
        }
        None => ("—".to_string(), None, None, None),
    };
    Ok(Analysis {
        bpm,
        key,
        key_tonic,
        key_mode,
        key_camelot,
    })
}

fn downmix(samples: &[f32]) -> Vec<f32> {
    samples
        .chunks_exact(2)
        .map(|frame| (frame[0] + frame[1]) * 0.5)
        .collect()
}

fn estimate_bpm(mono: &[f32], sample_rate: u32) -> f64 {
    let hop = 512usize;
    let frame = 1024usize;
    let frame_rate = sample_rate as f64 / hop as f64;
    let n_frames = mono.len().saturating_sub(frame) / hop + 1;
    if n_frames < 4 {
        return 0.0;
    }

    // Positive energy flux is a compact onset envelope suitable for a quick,
    // deterministic desktop analysis.
    let mut energy = Vec::with_capacity(n_frames);
    for index in 0..n_frames {
        let start = index * hop;
        let sum = mono[start..start + frame]
            .iter()
            .map(|sample| sample * sample)
            .sum::<f32>();
        energy.push(sum / frame as f32);
    }
    let mut onset = Vec::with_capacity(n_frames - 1);
    for index in 1..n_frames {
        onset.push((energy[index] - energy[index - 1]).max(0.0));
    }
    let mean = onset.iter().copied().sum::<f32>() / onset.len().max(1) as f32;
    for value in &mut onset {
        *value = (*value - mean).max(0.0);
    }

    let min_bpm = 60.0;
    let max_bpm = 200.0;
    // BPM = 60 * envelope_frame_rate / lag. The previous implementation
    // omitted 60, allowed lag zero and could loop forever on infinity.
    let min_lag = ((60.0 * frame_rate / max_bpm).floor() as usize).max(1);
    let max_lag = (60.0 * frame_rate / min_bpm).ceil() as usize;
    if onset.len() <= max_lag {
        return 0.0;
    }

    let mut best_lag = min_lag;
    let mut best_score = f64::MIN;
    for lag in min_lag..=max_lag {
        let (dot, left_energy, right_energy) = onset[..onset.len() - lag]
            .iter()
            .zip(&onset[lag..])
            .fold((0.0, 0.0, 0.0), |(dot, left, right), (a, b)| {
                let a = *a as f64;
                let b = *b as f64;
                (dot + a * b, left + a * a, right + b * b)
            });
        let score = dot / (left_energy * right_energy).sqrt().max(f64::EPSILON);
        if score > best_score {
            best_score = score;
            best_lag = lag;
        }
    }
    if best_score <= 0.0 {
        return 0.0;
    }
    let bpm = 60.0 * frame_rate / best_lag as f64;
    (bpm * 10.0).round() / 10.0
}

fn estimate_key(mono: &[f32], sample_rate: u32) -> Option<(String, String)> {
    let frame_len = 4096usize;
    if mono.len() < frame_len || sample_rate == 0 {
        return None;
    }

    // Analyze at most 96 evenly distributed frames. Goertzel evaluates only
    // the musically relevant note bins and avoids the former O(song * N^2)
    // naive DFT that made multi-minute tracks appear frozen.
    let available = (mono.len() - frame_len) / frame_len + 1;
    let frame_count = available.min(96);
    let step = ((mono.len() - frame_len) / frame_count.max(1)).max(1);
    let mut chroma = [0f64; 12];

    for frame_index in 0..frame_count {
        let start = (frame_index * step).min(mono.len() - frame_len);
        let frame = &mono[start..start + frame_len];
        for midi in 36..=95 {
            let frequency = 440.0f64 * 2.0f64.powf((midi as f64 - 69.0) / 12.0);
            let magnitude = goertzel_power(frame, sample_rate, frequency);
            chroma[(midi % 12) as usize] += magnitude.sqrt();
        }
    }

    let total: f64 = chroma.iter().sum();
    if !total.is_finite() || total <= f64::EPSILON {
        return None;
    }
    for value in &mut chroma {
        *value /= total;
    }

    let mut best = (f64::MIN, 0usize, "major");
    for tonic in 0..12 {
        let mut major = 0.0;
        let mut minor = 0.0;
        for pitch_class in 0..12 {
            let relative = (pitch_class + 12 - tonic) % 12;
            major += chroma[pitch_class] * MAJOR_PROFILE[relative] as f64;
            minor += chroma[pitch_class] * MINOR_PROFILE[relative] as f64;
        }
        if major > best.0 {
            best = (major, tonic, "major");
        }
        if minor > best.0 {
            best = (minor, tonic, "minor");
        }
    }
    Some((KEY_NAMES[best.1].to_string(), best.2.to_string()))
}

fn goertzel_power(frame: &[f32], sample_rate: u32, frequency: f64) -> f64 {
    let omega = 2.0 * std::f64::consts::PI * frequency / sample_rate as f64;
    let coefficient = 2.0 * omega.cos();
    let mut previous = 0.0f64;
    let mut previous_two = 0.0f64;
    let denominator = (frame.len().saturating_sub(1)).max(1) as f32;
    for (index, sample) in frame.iter().enumerate() {
        let window = 0.5 * (1.0 - (2.0 * PI * index as f32 / denominator).cos());
        let current = *sample as f64 * window as f64 + coefficient * previous - previous_two;
        previous_two = previous;
        previous = current;
    }
    (previous_two * previous_two + previous * previous - coefficient * previous * previous_two)
        .max(0.0)
}

fn camelot(tonic: &str, mode: &str) -> Option<String> {
    const MAJOR: [(&str, &str); 12] = [
        ("B", "1B"),
        ("F#", "2B"),
        ("C#", "3B"),
        ("G#", "4B"),
        ("D#", "5B"),
        ("A#", "6B"),
        ("F", "7B"),
        ("C", "8B"),
        ("G", "9B"),
        ("D", "10B"),
        ("A", "11B"),
        ("E", "12B"),
    ];
    const MINOR: [(&str, &str); 12] = [
        ("G#", "1A"),
        ("D#", "2A"),
        ("A#", "3A"),
        ("F", "4A"),
        ("C", "5A"),
        ("G", "6A"),
        ("D", "7A"),
        ("A", "8A"),
        ("E", "9A"),
        ("B", "10A"),
        ("F#", "11A"),
        ("C#", "12A"),
    ];
    let table = if mode == "major" { &MAJOR } else { &MINOR };
    table
        .iter()
        .find(|(note, _)| *note == tonic)
        .map(|(_, value)| (*value).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camelot_reference_values_are_correct() {
        assert_eq!(camelot("C", "major").as_deref(), Some("8B"));
        assert_eq!(camelot("A", "minor").as_deref(), Some("8A"));
        assert_eq!(camelot("F#", "major").as_deref(), Some("2B"));
    }

    #[test]
    fn detects_120_bpm_impulses() {
        let sample_rate = 44_100u32;
        let seconds = 12usize;
        let mut mono = vec![0.0f32; sample_rate as usize * seconds];
        for beat in (0..mono.len()).step_by((sample_rate / 2) as usize) {
            for offset in 0..256usize {
                if beat + offset < mono.len() {
                    mono[beat + offset] = 1.0 - offset as f32 / 256.0;
                }
            }
        }
        let bpm = estimate_bpm(&mono, sample_rate);
        assert!((bpm - 120.0).abs() < 2.0, "detected {bpm}");
    }
}
