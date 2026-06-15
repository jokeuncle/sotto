use anyhow::Result;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DailyEntry {
    pub date: String,
    pub text: String,
    pub source: String,
    pub generated_at: String,
}

fn storage_path(app: &AppHandle) -> Result<PathBuf> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("daily.json"))
}

pub fn read_today(app: &AppHandle) -> Result<Option<DailyEntry>> {
    let path = storage_path(app)?;
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path)?;
    let entry: DailyEntry = match serde_json::from_str(&raw) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };
    let today = Local::now().format("%Y-%m-%d").to_string();
    if entry.date == today {
        Ok(Some(entry))
    } else {
        Ok(None)
    }
}

pub fn write(app: &AppHandle, entry: &DailyEntry) -> Result<()> {
    let path = storage_path(app)?;
    let raw = serde_json::to_string_pretty(entry)?;
    std::fs::write(path, raw)?;
    Ok(())
}
