use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DailyEntry {
    pub date: String,
    pub text: String,
    pub source: String,
    pub generated_at: String,
    #[serde(default = "default_style")]
    pub style: String,
    #[serde(default = "default_slot")]
    pub slot: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSettings {
    #[serde(default = "default_style")]
    pub style: String,
    #[serde(default = "default_rhythm")]
    pub rhythm: String,
    #[serde(default = "default_sharpness")]
    pub sharpness: String,
    #[serde(default)]
    pub personal_note: String,
}

fn storage_path(app: &AppHandle) -> Result<PathBuf> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("daily.json"))
}

fn history_path(app: &AppHandle) -> Result<PathBuf> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("history.json"))
}

fn settings_path(app: &AppHandle) -> Result<PathBuf> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("settings.json"))
}

pub fn read_current(
    app: &AppHandle,
    date: &str,
    slot: &str,
    style: &str,
) -> Result<Option<DailyEntry>> {
    let path = storage_path(app)?;
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path)?;
    let entry: DailyEntry = match serde_json::from_str(&raw) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };
    if entry.date == date && entry.slot == slot && entry.style == style {
        Ok(Some(entry))
    } else {
        Ok(None)
    }
}

pub fn read_recent_texts(app: &AppHandle, limit: usize) -> Result<Vec<String>> {
    let mut texts = Vec::new();
    let mut seen = HashSet::new();

    if let Some(entry) = read_stored_entry(app)? {
        push_unique_text(&mut texts, &mut seen, entry.text);
    }

    for entry in read_history(app)?.into_iter().rev() {
        push_unique_text(&mut texts, &mut seen, entry.text);
        if texts.len() >= limit {
            break;
        }
    }

    Ok(texts)
}

pub fn read_history_entries(app: &AppHandle, limit: usize) -> Result<Vec<DailyEntry>> {
    let mut entries = read_history(app)?;
    entries.reverse();
    entries.truncate(limit);
    Ok(entries)
}

pub fn read_settings(app: &AppHandle) -> Result<AppSettings> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(AppSettings {
            style: default_style(),
            rhythm: default_rhythm(),
            sharpness: default_sharpness(),
            personal_note: String::new(),
        });
    }
    let raw = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw).unwrap_or_else(|_| AppSettings {
        style: default_style(),
        rhythm: default_rhythm(),
        sharpness: default_sharpness(),
        personal_note: String::new(),
    }))
}

pub fn write_settings(app: &AppHandle, settings: &AppSettings) -> Result<()> {
    let path = settings_path(app)?;
    let raw = serde_json::to_string_pretty(settings)?;
    std::fs::write(path, raw)?;
    Ok(())
}

pub fn write(app: &AppHandle, entry: &DailyEntry) -> Result<()> {
    let path = storage_path(app)?;
    let raw = serde_json::to_string_pretty(entry)?;
    std::fs::write(path, raw)?;
    Ok(())
}

pub fn append_history(app: &AppHandle, entry: &DailyEntry, limit: usize) -> Result<()> {
    let path = history_path(app)?;
    let mut entries = read_history(app)?;
    entries.retain(|old| old.text.trim() != entry.text.trim());
    entries.push(entry.clone());

    if entries.len() > limit {
        entries = entries.split_off(entries.len() - limit);
    }

    let raw = serde_json::to_string_pretty(&entries)?;
    std::fs::write(path, raw)?;
    Ok(())
}

fn read_stored_entry(app: &AppHandle) -> Result<Option<DailyEntry>> {
    let path = storage_path(app)?;
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&raw).ok())
}

fn read_history(app: &AppHandle) -> Result<Vec<DailyEntry>> {
    let path = history_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw).unwrap_or_default())
}

fn push_unique_text(texts: &mut Vec<String>, seen: &mut HashSet<String>, text: String) {
    let normalized = text.trim().to_string();
    if normalized.is_empty() || !seen.insert(normalized.clone()) {
        return;
    }
    texts.push(normalized);
}

fn default_style() -> String {
    "commute".into()
}

fn default_rhythm() -> String {
    "moments".into()
}

fn default_sharpness() -> String {
    "quiet".into()
}

fn default_slot() -> String {
    "daily".into()
}
