//! Tauri 2 shell for "Moonlock".
//!
//! - All calendar logic lives in `moonlock-core` (shared with the CLI).
//! - Config is the same TOML format as the CLI's `.moonlock.toml`, stored in the app config dir,
//!   so you can copy it into a repo and CI enforces exactly what the app shows.
//! - macOS + iOS: Apple Calendar sync goes through Swift/EventKit (swift/MoonlockKit).
//! - Desktop (`desktop.rs`): tray with today's lunar date + polling reminder loop.
//! - Mobile (`mobile.rs`): reminders are pre-scheduled as OS local notifications.

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

use moonlock_core::{Config, DayInfo, Verdict};
use serde::Serialize;
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Manager, State};

#[cfg(any(target_os = "macos", target_os = "ios"))]
extern "C" {
    fn moonlock_calendar_sync(json: *const std::os::raw::c_char) -> i32;
}

struct AppState {
    cfg: Mutex<Config>,
    path: PathBuf,
}

impl AppState {
    fn cfg(&self) -> Config {
        self.cfg.lock().unwrap().clone()
    }
}

type CmdResult<T> = Result<T, String>;

#[tauri::command]
fn get_config(s: State<AppState>) -> Config {
    s.cfg()
}

#[tauri::command]
fn save_config(app: AppHandle, s: State<AppState>, config: Config) -> CmdResult<()> {
    if let Some(dir) = s.path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(&s.path, config.to_toml()).map_err(|e| e.to_string())?;
    *s.cfg.lock().unwrap() = config;
    on_config_changed(&app);
    Ok(())
}

#[tauri::command]
fn config_info(s: State<AppState>) -> (String, String) {
    (s.path.display().to_string(), s.cfg().to_toml())
}

#[tauri::command]
fn month_view(s: State<AppState>, year: i64, month: i64) -> CmdResult<Vec<DayInfo>> {
    if !(1..=12).contains(&month) {
        return Err("tháng phải 1..12".into());
    }
    Ok(moonlock_core::month_view(year, month, &s.cfg()))
}

#[tauri::command]
fn today(s: State<AppState>) -> DayInfo {
    let cfg = s.cfg();
    moonlock_core::day_info(moonlock_core::today_jd(cfg.timezone), &cfg)
}

#[tauri::command]
fn upcoming(s: State<AppState>, days: i64) -> Vec<DayInfo> {
    let cfg = s.cfg();
    moonlock_core::upcoming(moonlock_core::today_jd(cfg.timezone), days.clamp(1, 400), &cfg)
}

#[tauri::command]
fn next_window(s: State<AppState>) -> Option<DayInfo> {
    let cfg = s.cfg();
    moonlock_core::next_deploy_window(moonlock_core::today_jd(cfg.timezone), &cfg)
}

/// Lunar date → solar. With `year` = that lunar year; without = next occurrence from today.
/// Day 30 in a 29-day month falls back to day 29 (same rule as giỗ).
#[tauri::command]
fn find_lunar(s: State<AppState>, day: i64, month: i64, year: Option<i64>) -> Option<DayInfo> {
    if !(1..=30).contains(&day) || !(1..=12).contains(&month) {
        return None;
    }
    let cfg = s.cfg();
    let tz = cfg.timezone;
    let today = moonlock_core::today_jd(tz);
    let resolve = |y: i64| {
        moonlock_core::lunar::lunar_to_jd(day, month, y, false, tz)
            .map(|jd| if moonlock_core::lunar::lunar_from_jd(jd, tz).month != month { jd - 1 } else { jd })
    };
    let jd = match year {
        Some(y) => resolve(y)?,
        None => {
            let ly = moonlock_core::lunar::lunar_from_jd(today, tz).year;
            (ly..=ly + 1).filter_map(resolve).find(|&jd| jd >= today)?
        }
    };
    Some(moonlock_core::day_info(jd, &cfg))
}

#[tauri::command]
fn day_at(s: State<AppState>, date: String) -> Option<DayInfo> {
    let cfg = s.cfg();
    moonlock_core::parse_date(&date).map(|jd| moonlock_core::day_info(jd, &cfg))
}

#[derive(Serialize)]
struct RuleMeta {
    id: moonlock_core::Rule,
    title: &'static str,
}

#[tauri::command]
fn rules() -> Vec<RuleMeta> {
    moonlock_core::Rule::ALL.iter().map(|&id| RuleMeta { id, title: id.title() }).collect()
}

#[tauri::command]
fn export_ics(app: AppHandle, s: State<AppState>, days: i64) -> CmdResult<String> {
    let cfg = s.cfg();
    let ics = moonlock_core::build_ics(moonlock_core::today_jd(cfg.timezone), days.clamp(1, 1500), &cfg);
    // Desktop: ~/Downloads. Mobile: the app's Documents folder (visible in the iOS Files app).
    #[cfg(desktop)]
    let dir = app.path().download_dir().map_err(|e| e.to_string())?;
    #[cfg(mobile)]
    let dir = app.path().document_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("moonlock.ics");
    fs::write(&path, ics).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

#[tauri::command]
fn platform() -> &'static str {
    std::env::consts::OS
}

/// Push freeze days + reminders into Apple Calendar (macOS/iOS, via Swift/EventKit).
/// Async so it runs off the main thread: EventKit's permission prompt blocks.
#[tauri::command]
async fn sync_apple_calendar(s: State<'_, AppState>, days: i64) -> CmdResult<i32> {
    let cfg = s.cfg();
    let from = moonlock_core::today_jd(cfg.timezone);
    let mut items = Vec::new();
    for jd in from..from + days.clamp(1, 1500) {
        let d = moonlock_core::day_info(jd, &cfg);
        let lunar = format!("{}/{} âm lịch · ngày {}", d.lunar.day, d.lunar_month_name, d.can_chi_day);
        if d.verdict == Verdict::Block {
            let why: Vec<&str> = d.block_reasons.iter().map(|h| h.label.as_str()).collect();
            items.push(serde_json::json!({
                "date": d.date, "title": "🚫 Deploy freeze",
                "notes": format!("{lunar}\n{}", why.join("\n")), "alarm_days": null
            }));
        }
        if !d.events.is_empty() {
            items.push(serde_json::json!({
                "date": d.date, "title": format!("🌕 {}", d.events.join(" · ")),
                "notes": lunar, "alarm_days": cfg.remind_days_before.max(0)
            }));
        }
    }
    sync_native(serde_json::Value::Array(items).to_string())
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn sync_native(json: String) -> CmdResult<i32> {
    #[cfg(target_os = "macos")]
    const DENIED: &str = "Chưa cấp quyền Lịch. Mở System Settings → Privacy & Security → Calendars.";
    #[cfg(target_os = "ios")]
    const DENIED: &str = "Chưa cấp quyền Lịch. Mở Cài đặt → Moonlock → Lịch.";
    let c = std::ffi::CString::new(json).map_err(|e| e.to_string())?;
    // SAFETY: `c` is a valid NUL-terminated string that outlives the call; Swift copies it.
    match unsafe { moonlock_calendar_sync(c.as_ptr()) } {
        n if n >= 0 => Ok(n),
        -1 => Err(DENIED.into()),
        -3 => Err("Không tạo được lịch \"Moonlock\".".into()),
        n => Err(format!("EventKit lỗi ({n}).")),
    }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
fn sync_native(_json: String) -> CmdResult<i32> {
    Err("Đồng bộ Apple Calendar chỉ có trên macOS và iOS — hãy dùng Export .ics.".into())
}

// ───────────────────────────── Reminders (shared text) ─────────────────────────────

pub(crate) fn ahead_title(ahead: i64, d: &DayInfo) -> String {
    let when = if ahead == 1 { "Ngày mai".to_string() } else { format!("{ahead} ngày nữa") };
    format!("📿 {when}: {}", d.events.join(" · "))
}

pub(crate) fn ahead_body(d: &DayInfo) -> String {
    format!("{} {} ({}/{} âm) — chuẩn bị mâm cỗ, dời lịch release.", d.weekday_name, d.date, d.lunar.day, d.lunar_month_name)
}

/// iOS: lay the system tab bar over the webview. `tabs` is `[{title, icon}]` (SF Symbol names).
/// Returns whether a native bar is in use, so the UI can hide its CSS one.
#[tauri::command]
fn native_tabbar(app: AppHandle, tabs: serde_json::Value) -> bool {
    #[cfg(target_os = "ios")]
    {
        mobile::tabbar::install(&app, tabs.to_string());
        true
    }
    #[cfg(not(target_os = "ios"))]
    {
        let _ = (app, tabs);
        false
    }
}

#[tauri::command]
fn native_tab_select(app: AppHandle, index: i32) {
    #[cfg(target_os = "ios")]
    mobile::tabbar::select(&app, index);
    #[cfg(not(target_os = "ios"))]
    let _ = (app, index);
}

/// Re-derive everything that depends on config: tray title (desktop), scheduled notifications (mobile).
fn on_config_changed(app: &AppHandle) {
    #[cfg(desktop)]
    desktop::refresh_tray(app);
    #[cfg(mobile)]
    mobile::reschedule(app);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let path = app.path().app_config_dir()?.join("moonlock.toml");
            let cfg = fs::read_to_string(&path).ok().and_then(|s| Config::from_toml(&s).ok()).unwrap_or_default();
            app.manage(AppState { cfg: Mutex::new(cfg), path });
            #[cfg(desktop)]
            {
                desktop::setup_tray(app)?;
                desktop::spawn_reminders(app.handle().clone());
            }
            on_config_changed(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            native_tabbar,
            native_tab_select,
            get_config,
            save_config,
            config_info,
            month_view,
            today,
            upcoming,
            next_window,
            find_lunar,
            day_at,
            rules,
            export_ics,
            platform,
            sync_apple_calendar
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
