mod cli;
#[cfg(target_os = "macos")]
mod macos;
mod storage;

use std::sync::Arc;

use chrono::{DateTime, Duration, Local, LocalResult, TimeZone, Timelike};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tokio::sync::Mutex;

use storage::{AppSettings, DailyEntry};

type GenLock = Arc<Mutex<()>>;
const HISTORY_PROMPT_LIMIT: usize = 30;
const HISTORY_STORE_LIMIT: usize = 120;
const HISTORY_DISPLAY_LIMIT: usize = 30;
const SCHEDULE_SLOTS: [(&str, u32, u32); 5] = [
    ("after_midnight", 0, 5),
    ("morning", 9, 5),
    ("noon", 12, 35),
    ("evening", 18, 5),
    ("night", 23, 20),
];

struct AppState {
    lock: GenLock,
}

#[tauri::command]
async fn get_today(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<DailyEntry, String> {
    ensure_current(&app, state.lock.clone())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn refresh(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<DailyEntry, String> {
    let _guard = state.lock.lock().await;
    let settings = normalized_settings(&app).map_err(|e| e.to_string())?;
    let slot = slot_for_settings(&settings, Local::now()).to_string();
    let entry = generate_and_save(&app, &settings, &slot)
        .await
        .map_err(|e| e.to_string())?;
    let _ = app.emit("aphorism", entry.clone());
    Ok(entry)
}

#[tauri::command]
fn get_history(app: AppHandle) -> Result<Vec<DailyEntry>, String> {
    storage::read_history_entries(&app, HISTORY_DISPLAY_LIMIT).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Result<AppSettings, String> {
    normalized_settings(&app).map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_style(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    style: String,
) -> Result<DailyEntry, String> {
    let mut settings = normalized_settings(&app).map_err(|e| e.to_string())?;
    settings.style = cli::normalize_style(&style).to_string();
    storage::write_settings(&app, &settings).map_err(|e| e.to_string())?;

    let _guard = state.lock.lock().await;
    let slot = slot_for_settings(&settings, Local::now()).to_string();
    let entry = generate_and_save(&app, &settings, &slot)
        .await
        .map_err(|e| e.to_string())?;
    let _ = app.emit("aphorism", entry.clone());
    Ok(entry)
}

#[tauri::command]
async fn save_settings(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    settings: AppSettings,
) -> Result<DailyEntry, String> {
    let settings = normalize_settings(settings);
    storage::write_settings(&app, &settings).map_err(|e| e.to_string())?;

    let _guard = state.lock.lock().await;
    let slot = slot_for_settings(&settings, Local::now()).to_string();
    let entry = generate_and_save(&app, &settings, &slot)
        .await
        .map_err(|e| e.to_string())?;
    let _ = app.emit("aphorism", entry.clone());
    Ok(entry)
}

async fn ensure_current(app: &AppHandle, lock: GenLock) -> anyhow::Result<DailyEntry> {
    let settings = normalized_settings(app)?;
    let now = Local::now();
    let date = now.format("%Y-%m-%d").to_string();
    let slot = slot_for_settings(&settings, now).to_string();

    if let Some(entry) = storage::read_current(app, &date, &slot, &settings.style)? {
        return Ok(entry);
    }
    let _guard = lock.lock().await;
    if let Some(entry) = storage::read_current(app, &date, &slot, &settings.style)? {
        return Ok(entry);
    }
    generate_and_save(app, &settings, &slot).await
}

async fn generate_and_save(
    app: &AppHandle,
    settings: &AppSettings,
    slot: &str,
) -> anyhow::Result<DailyEntry> {
    let recent = storage::read_recent_texts(app, HISTORY_PROMPT_LIMIT)?;
    let settings = normalize_settings(settings.clone());
    let prompt_settings = settings.clone();
    let (text, source) = tauri::async_runtime::spawn_blocking(move || {
        cli::generate(
            &recent,
            &prompt_settings.style,
            &prompt_settings.sharpness,
            &prompt_settings.personal_note,
        )
    })
    .await??;
    let entry = DailyEntry {
        date: Local::now().format("%Y-%m-%d").to_string(),
        text,
        source,
        generated_at: Local::now().to_rfc3339(),
        style: settings.style,
        slot: slot.to_string(),
    };
    storage::write(app, &entry)?;
    storage::append_history(app, &entry, HISTORY_STORE_LIMIT)?;
    Ok(entry)
}

fn normalized_settings(app: &AppHandle) -> anyhow::Result<AppSettings> {
    let settings = storage::read_settings(app)?;
    let normalized = normalize_settings(settings.clone());
    if settings.style != normalized.style
        || settings.rhythm != normalized.rhythm
        || settings.sharpness != normalized.sharpness
        || settings.personal_note != normalized.personal_note
    {
        let settings = normalized.clone();
        storage::write_settings(app, &settings)?;
    }
    Ok(normalized)
}

fn normalize_settings(mut settings: AppSettings) -> AppSettings {
    settings.style = cli::normalize_style(&settings.style).to_string();
    settings.rhythm = normalize_rhythm(&settings.rhythm).to_string();
    settings.sharpness = cli::normalize_sharpness(&settings.sharpness).to_string();
    settings.personal_note = settings
        .personal_note
        .trim()
        .chars()
        .take(80)
        .collect::<String>();
    settings
}

fn normalize_rhythm(rhythm: &str) -> &'static str {
    match rhythm {
        "daily" => "daily",
        "manual" => "manual",
        _ => "moments",
    }
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_today,
            refresh,
            get_history,
            get_settings,
            set_style,
            save_settings
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            macos::set_accessory_policy();

            let window = app
                .get_webview_window("main")
                .expect("main window must exist");
            position_bottom_right(&window);

            #[cfg(target_os = "macos")]
            macos::pin_below_normal(&window);

            window.show().ok();

            let lock: GenLock = Arc::new(Mutex::new(()));
            app.manage(AppState { lock: lock.clone() });

            let handle = app.handle().clone();
            let lock_init = lock.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                match ensure_current(&handle, lock_init).await {
                    Ok(entry) => {
                        let _ = handle.emit("aphorism", entry);
                    }
                    Err(e) => {
                        let _ = handle.emit("aphorism-error", e.to_string());
                    }
                }
            });

            let handle2 = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                schedule_daily_refresh(handle2, lock).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running sotto");
}

fn position_bottom_right(window: &WebviewWindow) {
    let monitor = match window.primary_monitor() {
        Ok(Some(m)) => m,
        _ => return,
    };
    let scale = monitor.scale_factor();
    let screen = monitor.size().to_logical::<f64>(scale);
    let window_size = match window.outer_size() {
        Ok(s) => s.to_logical::<f64>(scale),
        Err(_) => return,
    };
    let monitor_origin = monitor.position().to_logical::<f64>(scale);
    let margin_x = 40.0;
    let margin_y = 60.0;
    let x = monitor_origin.x + screen.width - window_size.width - margin_x;
    let y = monitor_origin.y + screen.height - window_size.height - margin_y;
    let _ = window.set_position(tauri::LogicalPosition { x, y });
}

async fn schedule_daily_refresh(handle: AppHandle, lock: GenLock) {
    loop {
        let now = Local::now();
        let settings = match normalized_settings(&handle) {
            Ok(settings) => settings,
            Err(e) => {
                eprintln!("[sotto] settings read failed: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                continue;
            }
        };
        if settings.rhythm == "manual" {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            continue;
        }

        let target = if settings.rhythm == "daily" {
            next_daily_time(now)
        } else {
            next_scheduled_time(now)
        };
        let wait = target
            .signed_duration_since(now)
            .to_std()
            .unwrap_or(std::time::Duration::from_secs(3600));
        tokio::time::sleep(wait).await;
        match ensure_current(&handle, lock.clone()).await {
            Ok(entry) => {
                let _ = handle.emit("aphorism", entry);
            }
            Err(e) => {
                eprintln!("[sotto] daily refresh failed: {e}");
            }
        }
    }
}

fn slot_for_settings(settings: &AppSettings, now: DateTime<Local>) -> &'static str {
    match settings.rhythm.as_str() {
        "daily" => "daily",
        "manual" => "manual",
        _ => current_slot(now),
    }
}

fn current_slot(now: DateTime<Local>) -> &'static str {
    let current_minutes = now.hour() * 60 + now.minute();
    let mut slot = SCHEDULE_SLOTS.last().map(|s| s.0).unwrap_or("night");
    for (id, hour, minute) in SCHEDULE_SLOTS {
        let slot_minutes = hour * 60 + minute;
        if current_minutes >= slot_minutes {
            slot = id;
        }
    }
    slot
}

fn next_daily_time(now: DateTime<Local>) -> DateTime<Local> {
    let current_minutes = now.hour() * 60 + now.minute();
    if current_minutes < 5 {
        local_time_on_date(now, 0, 5)
    } else {
        local_time_on_date(now + Duration::days(1), 0, 5)
    }
}

fn next_scheduled_time(now: DateTime<Local>) -> DateTime<Local> {
    let current_minutes = now.hour() * 60 + now.minute();
    for (_, hour, minute) in SCHEDULE_SLOTS {
        let slot_minutes = hour * 60 + minute;
        if current_minutes < slot_minutes {
            return local_time_on_date(now, hour, minute);
        }
    }

    let tomorrow = now + Duration::days(1);
    let (_, hour, minute) = SCHEDULE_SLOTS[0];
    local_time_on_date(tomorrow, hour, minute)
}

fn local_time_on_date(base: DateTime<Local>, hour: u32, minute: u32) -> DateTime<Local> {
    let Some(naive) = base.date_naive().and_hms_opt(hour, minute, 0) else {
        return base + Duration::hours(1);
    };
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(time) => time,
        LocalResult::Ambiguous(earlier, _) => earlier,
        LocalResult::None => base + Duration::hours(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(hour: u32, minute: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 6, 24, hour, minute, 0)
            .single()
            .expect("test time should be valid")
    }

    #[test]
    fn normalizes_unknown_settings() {
        let settings = normalize_settings(AppSettings {
            style: "other".into(),
            rhythm: "often".into(),
            sharpness: "loud".into(),
            personal_note: "  城市、晚睡、写代码  ".into(),
        });

        assert_eq!(settings.style, "commute");
        assert_eq!(settings.rhythm, "moments");
        assert_eq!(settings.sharpness, "quiet");
        assert_eq!(settings.personal_note, "城市、晚睡、写代码");
    }

    #[test]
    fn moments_rhythm_uses_day_part_slots() {
        let settings = AppSettings {
            style: "commute".into(),
            rhythm: "moments".into(),
            sharpness: "quiet".into(),
            personal_note: String::new(),
        };

        assert_eq!(slot_for_settings(&settings, at(8, 40)), "after_midnight");
        assert_eq!(slot_for_settings(&settings, at(9, 5)), "morning");
        assert_eq!(slot_for_settings(&settings, at(18, 30)), "evening");
    }

    #[test]
    fn daily_and_manual_rhythm_use_stable_slots() {
        let mut settings = AppSettings {
            style: "commute".into(),
            rhythm: "daily".into(),
            sharpness: "quiet".into(),
            personal_note: String::new(),
        };
        assert_eq!(slot_for_settings(&settings, at(18, 30)), "daily");

        settings.rhythm = "manual".into();
        assert_eq!(slot_for_settings(&settings, at(18, 30)), "manual");
    }
}
