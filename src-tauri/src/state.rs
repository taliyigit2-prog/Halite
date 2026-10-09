use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use crate::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: String,
    pub theme: String,
    pub output_dir: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "auto".to_string(),
            theme: "system".to_string(),
            output_dir: None,
        }
    }
}

pub struct AppState {
    pub db: Mutex<Db>,
    pub settings: Mutex<Settings>,
    pub cancel_flags: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub data_dir: PathBuf,
    pub resource_dir: PathBuf,
    pub selected_files: Mutex<HashSet<PathBuf>>,
    pub studio_busy: AtomicBool,
}

impl AppState {
    pub fn new(data_dir: PathBuf, resource_dir: PathBuf) -> Result<Self, crate::error::Error> {
        let db = Db::open(data_dir.join("halite.db"))?;
        let settings = Settings::load(&data_dir).unwrap_or_default().normalized();
        Ok(Self {
            db: Mutex::new(db),
            settings: Mutex::new(settings),
            cancel_flags: Mutex::new(HashMap::new()),
            data_dir,
            resource_dir,
            selected_files: Mutex::new(HashSet::new()),
            studio_busy: AtomicBool::new(false),
        })
    }

    pub fn cancel_flag(&self, job_id: &str) -> Arc<AtomicBool> {
        let mut flags = self
            .cancel_flags
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        flags
            .entry(job_id.to_string())
            .or_insert_with(|| Arc::new(AtomicBool::new(false)))
            .clone()
    }

    pub fn request_cancel(&self, job_id: &str) {
        let flags = self
            .cancel_flags
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(flag) = flags.get(job_id) {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    pub fn clear_cancel(&self, job_id: &str) {
        let mut flags = self
            .cancel_flags
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        flags.remove(job_id);
    }
}

impl Settings {
    pub fn normalized(mut self) -> Self {
        if !matches!(
            self.language.as_str(),
            "auto" | "en" | "tr" | "de" | "es" | "fr" | "ru" | "ja"
        ) {
            self.language = "auto".to_string();
        }
        if !matches!(self.theme.as_str(), "system" | "light" | "dark") {
            self.theme = "system".to_string();
        }
        self.output_dir = self
            .output_dir
            .take()
            .filter(|path| !path.trim().is_empty());
        self
    }

    pub fn path(data_dir: &std::path::Path) -> PathBuf {
        data_dir.join("settings.json")
    }

    pub fn load(data_dir: &std::path::Path) -> Option<Self> {
        let text = std::fs::read_to_string(Self::path(data_dir)).ok()?;
        serde_json::from_str(&text).ok()
    }

    pub fn save(&self, data_dir: &std::path::Path) -> Result<(), crate::error::Error> {
        std::fs::create_dir_all(data_dir)?;
        let text = serde_json::to_string_pretty(self)?;
        let path = Self::path(data_dir);
        let temporary = data_dir.join("settings.json.tmp");
        {
            use std::io::Write;
            let mut file = std::fs::File::create(&temporary)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
        }
        #[cfg(target_os = "windows")]
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        std::fs::rename(temporary, path)?;
        Ok(())
    }
}
