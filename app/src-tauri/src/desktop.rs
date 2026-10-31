//! Desktop-only: tray with today's lunar date + a polling reminder loop.
//! (Desktop apps stay running, so polling is simplest and survives sleep/wake.)

use crate::AppState;
use moonlock_core::{DayInfo, Verdict};
use std::{collections::HashSet, time::Duration};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};
use tauri_plugin_notification::NotificationExt;

fn tray_text(d: &DayInfo) -> String {
    let mark = match d.verdict {
        Verdict::Ok => "✅",
        Verdict::Warn => "⚠️",
        Verdict::Block => "⛔",
    };
    format!("🌙 {}/{} {mark}", d.lunar.day, d.lunar.month)
}

pub fn refresh_tray(app: &AppHandle) {
    let cfg = app.state::<AppState>().cfg();
    let d = moonlock_core::day_info(moonlock_core::today_jd(cfg.timezone), &cfg);
    if let Some(tray) = app.tray_by_id("main") {
        let text = tray_text(&d);
        #[cfg(target_os = "macos")]
        let _ = tray.set_title(Some(&text));
        let _ = tray.set_tooltip(Some(&format!("{text} — {} {}", d.can_chi_day, d.can_chi_year)));
    }
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

/// Checks every 20 minutes; each (date, kind) is notified once per app run.
pub fn spawn_reminders(app: AppHandle) {
    std::thread::spawn(move || {
        let mut sent: HashSet<String> = HashSet::new();
        loop {
            let cfg = app.state::<AppState>().cfg();
            let t = moonlock_core::today_jd(cfg.timezone);
            let today = moonlock_core::day_info(t, &cfg);

            if today.verdict == Verdict::Block && sent.insert(format!("{}-block", today.date)) {
                let why: Vec<&str> = today.block_reasons.iter().map(|h| h.label.as_str()).collect();
                notify(&app, "⛔ Hôm nay cấm deploy", &why.join("\n"));
            }
            if !today.events.is_empty() && sent.insert(format!("{}-today", today.date)) {
                notify(&app, &format!("🌕 Hôm nay: {}", today.events.join(" · ")), "Nhớ thắp hương trước khi merge 🙏");
            }
            for ahead in 1..=cfg.remind_days_before.max(0) {
                let d = moonlock_core::day_info(t + ahead, &cfg);
                if !d.events.is_empty() && sent.insert(format!("{}-ahead", d.date)) {
                    notify(&app, &crate::ahead_title(ahead, &d), &crate::ahead_body(&d));
                }
            }
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || refresh_tray(&handle));
            std::thread::sleep(Duration::from_secs(20 * 60));
        }
    });
}

pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Mở Moonlock", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Thoát", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let mut builder = TrayIconBuilder::with_id("main").menu(&menu).on_menu_event(|app, ev| match ev.id().as_ref() {
        "open" => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
        "quit" => app.exit(0),
        _ => {}
    });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}
