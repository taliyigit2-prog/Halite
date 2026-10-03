use std::path::Path;
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
        #[cfg(target_os = "macos")]
        {
            let _ = ort::init()
                .with_execution_providers([ort::ep::CoreML::default().build()])
                .commit();
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = ort::init().commit();
        }
    });
}

pub struct Separator {
    session: Session,
}

impl Separator {
    pub fn new(model_path: &Path, use_coreml: bool) -> Result<Self> {
        ensure_init();

        let mut builder = Session::builder()
            .map_err(|e| Error::Ort(e.to_string()))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| Error::Ort(e.to_string()))?;

        #[cfg(target_os = "macos")]
        if use_coreml {
            builder = builder
                .with_execution_providers([ort::ep::CoreML::default().build()])
                .map_err(|e| Error::Ort(e.to_string()))?;
        }

        let session = builder
            .commit_from_file(model_path)
            .map_err(|e| Error::Ort(e.to_string()))?;
        Ok(Self { session })
    }

    /// Run separation on planar stereo input (`mix[0]` = left, `mix[1]` = right).
    /// Returns 4 stems, each as planar stereo (`stems[s][0]` = left, `stems[s][1]` = right).
    pub fn separate(
        &mut self,
        mix: &[Vec<f32>],
        mut on_progress: impl FnMut(f64),
    ) -> Result<Vec<[Vec<f32>; 2]>> {
        let total = mix[0].len();
        if mix.len() < 2 || mix[1].len() != total {
            return Err(Error::Message("expected stereo input".to_string()));
        }

        let n_chunks = ((total + STRIDE - 1) / STRIDE).max(1);
        let window = make_window(N_SAMPLES, OVERLAP);

        let mut out: Vec<[Vec<f32>; 2]> = (0..4)
            .map(|_| [vec![0f32; total], vec![0f32; total]])
            .collect();
        let mut weight = vec![0f32; total];

        for i in 0..n_chunks {
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

            let (_shape, data) = outputs["stems"]
                .try_extract_tensor::<f32>()
                .map_err(|e| Error::Ort(e.to_string()))?;

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
    let fade: Vec<f32> = (0..overlap)
        .map(|i| (i as f32) / (overlap as f32))
        .collect();
    for i in 0..overlap {
        w[i] = fade[i];
        w[n - 1 - i] = fade[i];
    }
    w
}
