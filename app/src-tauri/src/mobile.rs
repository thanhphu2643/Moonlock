//! Mobile-only (iOS/Android): the OS suspends apps in the background, so instead of polling
//! we hand the OS a batch of scheduled local notifications and rebuild it on every launch
//! and every config change.

use crate::AppState;
use moonlock_core::Verdict;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::{NotificationExt, PermissionState, Schedule};
use time::{OffsetDateTime, UtcOffset};

/// How far ahead to schedule. Rebuilt on each launch, so a short window is enough.
const HORIZON_DAYS: i64 = 45;
/// iOS keeps at most 64 pending local notifications per app.
const MAX_PENDING: usize = 60;

/// Local wall-clock `hour:00` on Julian day `jd` in timezone `tz` (hours east of UTC),
/// returned in UTC: the Android side parses dates with the pattern `…ss.SSS'Z'`, so any
/// non-`Z` offset (e.g. `+07:00`) is silently rejected and the reminder never fires.
fn at(jd: i64, hour: i64, tz: f64) -> Option<OffsetDateTime> {
    let offset = (tz * 3600.0).round() as i64;
    let unix = (jd - 2440588) * 86_400 + hour * 3_600 - offset;
    OffsetDateTime::from_unix_timestamp(unix).ok().map(|t| t.to_offset(UtcOffset::UTC))
}

struct Planned {
    priority: u8,
    day: i64,
    hour: i64,
    kind: i64,
    title: String,
    body: String,
}

/// Cancel and re-create every scheduled reminder. Runs off the main thread because the
/// first call shows the OS permission prompt and blocks until the user answers.
pub fn reschedule(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let n = app.notification();
        let granted = match n.permission_state() {
            Ok(PermissionState::Granted) => true,
            _ => matches!(n.request_permission(), Ok(PermissionState::Granted)),
        };
        if !granted {
            return;
        }
        let _ = n.cancel_all();

        let cfg = app.state::<AppState>().cfg();
        let tz = cfg.timezone;
        let today = moonlock_core::today_jd(tz);
        let mut plan = Vec::new();
        for jd in today..today + HORIZON_DAYS {
            let d = moonlock_core::day_info(jd, &cfg);
            if !d.events.is_empty() {
                for ahead in 1..=cfg.remind_days_before.clamp(0, 7) {
                    plan.push(Planned {
                        priority: 0,
                        day: jd - ahead,
                        hour: 20,
                        kind: ahead,
                        title: crate::ahead_title(ahead, &d),
                        body: crate::ahead_body(&d),
                    });
                }
                plan.push(Planned {
                    priority: 0,
                    day: jd,
                    hour: 7,
                    kind: 0,
                    title: format!("🌕 Hôm nay: {}", d.events.join(" · ")),
                    body: "Nhớ thắp hương trước khi merge 🙏".into(),
                });
            }
            // Only on workdays — nobody needs a "no deploy" ping on Sunday.
            if d.verdict == Verdict::Block && (1..=5).contains(&d.weekday) {
                let why: Vec<&str> = d.block_reasons.iter().map(|h| h.label.as_str()).collect();
                plan.push(Planned {
                    priority: 1,
                    day: jd,
                    hour: 8,
                    kind: 9,
                    title: "⛔ Hôm nay cấm deploy".into(),
                    body: why.join("\n"),
                });
            }
        }
        plan.sort_by_key(|p| (p.priority, p.day, p.hour));

        let now = OffsetDateTime::now_utc();
        let mut scheduled = 0;
        for p in plan {
            if scheduled >= MAX_PENDING {
                break;
            }
            let Some(when) = at(p.day, p.hour, tz).filter(|w| *w > now) else { continue };
            // Stable id per (day, kind) so reschedules replace rather than duplicate.
            let id = ((p.day % 100_000) * 10 + p.kind) as i32;
            let ok = n
                .builder()
                .id(id)
                .title(&p.title)
                .body(&p.body)
                .schedule(Schedule::At { date: when, repeating: false, allow_while_idle: true })
                .show()
                .is_ok();
            if ok {
                scheduled += 1;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn local_time() {
        // 2026-11-01 07:00 at UTC+7 == 2026-11-01 00:00 UTC.
        let jd = moonlock_core::lunar::jd_from_date(1, 11, 2026);
        let t = super::at(jd, 7, 7.0).unwrap();
        assert_eq!(t.unix_timestamp(), 1_793_491_200);
        assert!(t.offset().is_utc());
    }
}

/// iOS: the system UITabBar (Liquid Glass on iOS 26+) replaces the CSS tab bar.
/// Lives in swift/MoonlockKit/Sources/MoonlockKit/NativeTabBar.swift.
#[cfg(target_os = "ios")]
pub mod tabbar {
    use std::{ffi::{c_char, c_void, CString}, sync::OnceLock};
    use tauri::{AppHandle, Emitter, Manager};

    extern "C" {
        fn moonlock_tabbar_install(vc: *mut c_void, json: *const c_char, cb: extern "C" fn(i32)) -> bool;
        fn moonlock_tabbar_select(index: i32);
    }

    static APP: OnceLock<AppHandle> = OnceLock::new();

    extern "C" fn on_select(index: i32) {
        if let Some(app) = APP.get() {
            let _ = app.emit("native-tab", index);
        }
    }

    pub fn install(app: &AppHandle, tabs_json: String) {
        let _ = APP.set(app.clone());
        let Some(win) = app.get_webview_window("main") else { return };
        let _ = win.with_webview(move |wv| {
            let json = CString::new(tabs_json).unwrap_or_default();
            unsafe { moonlock_tabbar_install(wv.view_controller() as *mut c_void, json.as_ptr(), on_select) };
        });
    }

    pub fn select(app: &AppHandle, index: i32) {
        let Some(win) = app.get_webview_window("main") else { return };
        let _ = win.with_webview(move |_| unsafe { moonlock_tabbar_select(index) });
    }
}
