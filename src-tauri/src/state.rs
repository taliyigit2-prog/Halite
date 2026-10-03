use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Result<Self, crate::error::Error> {
        let db = Db::open(data_dir.join("halite.db"))?;
        let settings = Settings::load(&data_dir).unwrap_or_default();
        Ok(Self {
            db: Mutex::new(db),
            settings: Mutex::new(settings),
            cancel_flags: Mutex::new(HashMap::new()),
            data_dir,
        })
    }

    pub fn cancel_flag(&self, job_id: &str) -> Arc<AtomicBool> {
        let mut flags = self.cancel_flags.lock().unwrap();
        flags
            .entry(job_id.to_string())
            .or_insert_with(|| Arc::new(AtomicBool::new(false)))
            .clone()
    }

    pub fn request_cancel(&self, job_id: &str) {
        let flags = self.cancel_flags.lock().unwrap();
        if let Some(flag) = flags.get(job_id) {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    pub fn clear_cancel(&self, job_id: &str) {
        let mut flags = self.cancel_flags.lock().unwrap();
        flags.remove(job_id);
    }
}

impl Settings {
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
        std::fs::write(Self::path(data_dir), text)?;
        Ok(())
    }
}
