mod cli;
mod storage;
#[cfg(target_os = "macos")]
mod macos;

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tokio::sync::Mutex;

use storage::DailyEntry;

type GenLock = Arc<Mutex<()>>;

struct AppState {
    lock: GenLock,
}

#[tauri::command]
async fn get_today(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<DailyEntry, String> {
    ensure_today(&app, state.lock.clone())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn refresh(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<DailyEntry, String> {
    let _guard = state.lock.lock().await;
    let entry = generate_and_save(&app).await.map_err(|e| e.to_string())?;
    let _ = app.emit("aphorism", entry.clone());
    Ok(entry)
}

async fn ensure_today(app: &AppHandle, lock: GenLock) -> anyhow::Result<DailyEntry> {
    if let Some(entry) = storage::read_today(app)? {
        return Ok(entry);
    }
    let _guard = lock.lock().await;
    if let Some(entry) = storage::read_today(app)? {
        return Ok(entry);
    }
    generate_and_save(app).await
}

async fn generate_and_save(app: &AppHandle) -> anyhow::Result<DailyEntry> {
    let (text, source) = tauri::async_runtime::spawn_blocking(cli::generate).await??;
    let entry = DailyEntry {
        date: chrono::Local::now().format("%Y-%m-%d").to_string(),
        text,
        source,
        generated_at: chrono::Local::now().to_rfc3339(),
    };
    storage::write(app, &entry)?;
    Ok(entry)
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_today, refresh])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            macos::set_accessory_policy();

            let window = app
                .get_webview_window("main")
                .expect("main window must exist");
            position_bottom_right(&window);

            #[cfg(target_os = "macos")]
            macos::pin_to_desktop_level(&window);

            window.show().ok();

            let lock: GenLock = Arc::new(Mutex::new(()));
            app.manage(AppState { lock: lock.clone() });

            let handle = app.handle().clone();
            let lock_init = lock.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                match ensure_today(&handle, lock_init).await {
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
    use chrono::{Duration, Local, TimeZone};
    loop {
        let now = Local::now();
        let tomorrow_naive = (now + Duration::days(1)).date_naive();
        let target_naive = match tomorrow_naive.and_hms_opt(0, 5, 0) {
            Some(t) => t,
            None => {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                continue;
            }
        };
        let target = match Local.from_local_datetime(&target_naive).single() {
            Some(t) => t,
            None => now + Duration::hours(24),
        };
        let wait = target
            .signed_duration_since(now)
            .to_std()
            .unwrap_or(std::time::Duration::from_secs(3600));
        tokio::time::sleep(wait).await;
        match ensure_today(&handle, lock.clone()).await {
            Ok(entry) => {
                let _ = handle.emit("aphorism", entry);
            }
            Err(e) => {
                eprintln!("[sotto] daily refresh failed: {e}");
            }
        }
    }
}
