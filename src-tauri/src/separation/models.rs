use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

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
            url: "https://huggingface.co/StemSplitio/htdemucs-onnx/resolve/d54ed9eb60e258ea82131c6ee14578628816456a/htdemucs.onnx".to_string(),
            size_bytes: 316_446_953,
            sha256: Some(
                "68d0bf16428ef66e692cdff8a9ccf28f1ef3f69440d57e58605a4cc55fcc5e74".to_string(),
            ),
            installed: false,
            path: None,
        },
        ModelInfo {
            id: "htdemucs_fp16".to_string(),
            name: "htdemucs (4-stem, light)".to_string(),
            description: "Same stems in a smaller bundled model".to_string(),
            stems: stems_list(),
            file_name: "htdemucs_fp16weights.onnx".to_string(),
            url: "https://huggingface.co/StemSplitio/htdemucs-onnx/resolve/d54ed9eb60e258ea82131c6ee14578628816456a/htdemucs_fp16weights.onnx".to_string(),
            size_bytes: 165_612_636,
            sha256: Some(
                "d05c269d0178d2a72ad484b10b11dd370193fc923201c3b27a99f848745db70a".to_string(),
            ),
            installed: false,
            path: None,
        },
    ]
});

fn file_name_for(model_id: &str) -> Option<String> {
    MODELS
        .iter()
        .find(|m| m.id == model_id)
        .map(|m| m.file_name.clone())
}

/// Path where the model is bundled inside the app (Tauri resources).
pub fn bundled_model_path(resource_dir: &Path, model_id: &str) -> Option<PathBuf> {
    file_name_for(model_id).map(|name| resource_dir.join("models").join(name))
}

pub fn list_models(resource_dir: &Path) -> Vec<ModelInfo> {
    MODELS
        .iter()
        .map(|m| {
            let path = resource_dir.join("models").join(&m.file_name);
            let installed = path.exists();
            ModelInfo {
                installed,
                path: installed.then(|| path.to_string_lossy().to_string()),
                ..m.clone()
            }
        })
        .collect()
}
