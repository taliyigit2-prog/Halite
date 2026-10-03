use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::Tensor;

use crate::error::{Error, Result};

pub const SAMPLE_RATE: u32 = 44_100;
pub const N_SAMPLES: usize = 343_980; // 7.8s @ 44.1 kHz
pub const OVERLAP: usize = N_SAMPLES / 4;
pub const STRIDE: usize = N_SAMPLES - OVERLAP;

pub const SOURCES: [&str; 4] = ["drums", "bass", "other", "vocals"];

static INIT: Once = Once::new();

fn ensure_init() {
    INIT.call_once(|| {
        // Keep the global runtime provider-neutral. Execution providers belong
        // to individual sessions; registering CoreML globally made even a CPU
        // session fail while CoreML tried to create its compilation workspace.
        let _ = ort::init().commit();
    });
}

pub struct Separator {
    session: Session,
}

impl Separator {
    pub fn new(model_path: &Path, use_coreml: bool) -> Result<Self> {
        ensure_init();

        #[cfg(target_os = "macos")]
        if use_coreml {
            // CoreML does not support every ONNX graph and can also be
            // unavailable on otherwise supported macOS versions. Prefer it,
            // but always fall back to the CPU provider so separation remains
            // functional instead of failing immediately.
            let coreml_session: std::result::Result<Session, ort::Error> = (|| {
                Session::builder()?
                    .with_optimization_level(GraphOptimizationLevel::Level3)?
                    .with_execution_providers([ort::ep::CoreML::default().build()])?
                    .commit_from_file(model_path)
            })();
            if let Ok(session) = coreml_session {
                return Ok(Self { session });
            }
        }

        let session = Session::builder()
            .map_err(|e| Error::Ort(e.to_string()))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| Error::Ort(e.to_string()))?
            .commit_from_file(model_path)
            .map_err(|e| Error::Ort(e.to_string()))?;
        Ok(Self { session })
    }

    /// Run separation on planar stereo input (`mix[0]` = left, `mix[1]` = right).
    /// Returns 4 stems, each as planar stereo (`stems[s][0]` = left, `stems[s][1]` = right).
    pub fn separate(
        &mut self,
        mix: &[Vec<f32>],
        cancel: &AtomicBool,
        mut on_progress: impl FnMut(f64),
    ) -> Result<Vec<[Vec<f32>; 2]>> {
        if mix.len() < 2 {
            return Err(Error::Message("expected stereo input".to_string()));
        }
        let total = mix[0].len();
        if mix[1].len() != total {
            return Err(Error::Message("expected stereo input".to_string()));
        }

        let n_chunks = ((total + STRIDE - 1) / STRIDE).max(1);
        let window = make_window(N_SAMPLES, OVERLAP);

        let mut out: Vec<[Vec<f32>; 2]> = (0..4)
            .map(|_| [vec![0f32; total], vec![0f32; total]])
            .collect();
        let mut weight = vec![0f32; total];

        for i in 0..n_chunks {
            if cancel.load(Ordering::SeqCst) {
                return Err(Error::Message("HALITE_CANCELLED".to_string()));
            }
            let start = i * STRIDE;
            let end = (start + N_SAMPLES).min(total);
            let clen = end - start;

            // Build planar chunk of size N_SAMPLES, zero-padded.
            let mut chunk_l = vec![0f32; N_SAMPLES];
            let mut chunk_r = vec![0f32; N_SAMPLES];
            chunk_l[..clen].copy_from_slice(&mix[0][start..end]);
            chunk_r[..clen].copy_from_slice(&mix[1][start..end]);

            // Flatten to batch-major [1, 2, N]: ch0 then ch1.
            let mut flat = Vec::with_capacity(2 * N_SAMPLES);
            flat.extend_from_slice(&chunk_l);
            flat.extend_from_slice(&chunk_r);

            let input = Tensor::from_array((vec![1i64, 2, N_SAMPLES as i64], flat))
                .map_err(|e| Error::Ort(e.to_string()))?;

            let outputs = self
                .session
                .run(ort::inputs!["mix" => input])
                .map_err(|e| Error::Ort(e.to_string()))?;

            if cancel.load(Ordering::SeqCst) {
                return Err(Error::Message("HALITE_CANCELLED".to_string()));
            }

            let (shape, data) = outputs["stems"]
                .try_extract_tensor::<f32>()
                .map_err(|e| Error::Ort(e.to_string()))?;
            let expected = 4 * 2 * N_SAMPLES;
            if data.len() < expected || shape.len() != 4 {
                return Err(Error::Ort(format!(
                    "unexpected model output shape {shape:?}"
                )));
            }

            // data layout: [4][2][N]
            for s in 0..4usize {
                for c in 0..2usize {
                    let base = s * (2 * N_SAMPLES) + c * N_SAMPLES;
                    for j in 0..clen {
                        let w = window[j];
                        out[s][c][start + j] += data[base + j] * w;
                    }
                }
            }
            for j in 0..clen {
                weight[start + j] += window[j];
            }

            on_progress((i + 1) as f64 / n_chunks as f64);
        }

        for s in 0..4usize {
            for c in 0..2usize {
                for j in 0..total {
                    let w = weight[j];
                    if w > 1e-8 {
                        out[s][c][j] /= w;
                    }
                }
            }
        }

        Ok(out)
    }
}

fn make_window(n: usize, overlap: usize) -> Vec<f32> {
    let mut w = vec![1f32; n];
    let denominator = overlap.saturating_sub(1).max(1) as f32;
    let fade: Vec<f32> = (0..overlap)
        .map(|i| i as f32 / denominator)
        .collect();
    for i in 0..overlap {
        w[i] = fade[i];
        w[n - 1 - i] = fade[i];
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    // This is intentionally ignored during the fast unit-test suite because it
    // loads the 300 MB production model. Run it explicitly for release QA with:
    // cargo test --release bundled_model_infers_on_cpu -- --ignored --nocapture
    #[test]
    #[ignore]
    fn bundled_model_infers_on_cpu() {
        let model = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/models/htdemucs.onnx");
        assert!(
            model.is_file(),
            "bundled model is missing: {}",
            model.display()
        );

        let mut separator = Separator::new(&model, false).expect("model should load on CPU");
        let samples = SAMPLE_RATE as usize * 2;
        let mix = [vec![0.0; samples], vec![0.0; samples]];
        let stems = separator
            .separate(&mix, &AtomicBool::new(false), |_| {})
            .expect("model should run on CPU");

        assert_eq!(stems.len(), SOURCES.len());
        assert!(stems
            .iter()
            .all(|stem| stem[0].len() == samples && stem[1].len() == samples));
    }

    #[test]
    #[ignore]
    fn bundled_model_loads_with_acceleration_fallback() {
        let model = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/models/htdemucs.onnx");
        assert!(
            model.is_file(),
            "bundled model is missing: {}",
            model.display()
        );
        Separator::new(&model, true).expect("CoreML or its CPU fallback should load the model");
    }
}
