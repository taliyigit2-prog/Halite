use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub stems: Vec<String>,
    pub file_name: String,
    pub url: String,
    pub size_bytes: u64,
    pub sha256: Option<String>,
    pub installed: bool,
    pub path: Option<String>,
}

pub const SOURCES: [&str; 4] = ["drums", "bass", "other", "vocals"];

fn stems_list() -> Vec<String> {
    SOURCES.iter().map(|s| s.to_string()).collect()
}

pub static MODELS: LazyLock<Vec<ModelInfo>> = LazyLock::new(|| {
    vec![
        ModelInfo {
            id: "htdemucs".to_string(),
            name: "htdemucs (4-stem, high quality)".to_string(),
            description: "Drums / bass / other / vocals — best quality".to_string(),
            stems: stems_list(),
            file_name: "htdemucs.onnx".to_string(),
            url: "https://huggingface.co/StemSplitio/htdemucs-onnx/resolve/main/htdemucs.onnx"
                .to_string(),
            size_bytes: 331_350_016,
            sha256: Some(
                "68d0bf16428ef66e692cdff8a9ccf28f1ef3f69440d57e58605a4cc55fcc5e74".to_string(),
            ),
            installed: false,
            path: None,
        },
        ModelInfo {
            id: "htdemucs_fp16".to_string(),
            name: "htdemucs (4-stem, light)".to_string(),
            description: "Same stems, ~half the download size".to_string(),
            stems: stems_list(),
            file_name: "htdemucs_fp16weights.onnx".to_string(),
            url: "https://huggingface.co/StemSplitio/htdemucs-onnx/resolve/main/htdemucs_fp16weights.onnx"
                .to_string(),
            size_bytes: 174_063_616,
            sha256: None,
            installed: false,
            path: None,
        },
    ]
});

pub fn model_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("models")
}

pub fn model_path(data_dir: &Path, model_id: &str) -> PathBuf {
    let file_name = MODELS
        .iter()
        .find(|m| m.id == model_id)
        .map(|m| m.file_name.as_str())
        .unwrap_or("model.onnx");
    model_dir(data_dir).join(file_name)
}

pub fn list_models(data_dir: &Path) -> Vec<ModelInfo> {
    MODELS
        .iter()
        .map(|m| {
            let path = model_path(data_dir, &m.id);
            let installed = path.exists();
            ModelInfo {
                installed,
                path: installed.then(|| path.to_string_lossy().to_string()),
                ..m.clone()
            }
        })
        .collect()
}

pub fn get_model(id: &str) -> Option<&'static ModelInfo> {
    MODELS.iter().find(|m| m.id == id)
}

/// Download a model with progress reporting via the provided callback (0.0..=1.0).
pub fn download_model(
    data_dir: &Path,
    model_id: &str,
    mut on_progress: impl FnMut(f64),
) -> Result<PathBuf> {
    let model = get_model(model_id)
        .ok_or_else(|| Error::Message(format!("unknown model: {model_id}")))?;
    let dir = model_dir(data_dir);
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join(&model.file_name);
    if dest.exists() {
        return Ok(dest);
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Error::Message(e.to_string()))?;

    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60 * 60))
            .build()?;
        let mut resp = client.get(&model.url).send().await?.error_for_status()?;
        let total = resp.content_length().unwrap_or(model.size_bytes);
        let tmp = dest.with_extension("part");

        let mut file = tokio::fs::File::create(&tmp).await?;
        let mut downloaded: u64 = 0;
        while let Some(chunk) = resp.chunk().await? {
            use tokio::io::AsyncWriteExt;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            on_progress(downloaded as f64 / total.max(1) as f64);
        }
        file.sync_all().await?;
        tokio::fs::rename(&tmp, &dest).await?;
        Ok::<PathBuf, Error>(dest)
    })
}

pub fn delete_model(data_dir: &Path, model_id: &str) -> Result<()> {
    let path = model_path(data_dir, model_id);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
