//! `moonlock` — âm lịch trong terminal + chốt chặn deploy cho CI.
//!
//!   moonlock [today] [--date YYYY-MM-DD]
//!   moonlock month [YYYY-MM]
//!   moonlock upcoming [--days N]
//!   moonlock deploy-check [--date YYYY-MM-DD] [--strict]    # exit 1 nếu ngày kiêng
//!   moonlock ics [--days N] [-o file.ics]
//!   moonlock lunar <ngày>/<tháng>[/<năm>] [--leap]           # âm → dương
//!   moonlock init                                            # tạo .moonlock.toml
//!
//! Cấu hình: --config <file>, hoặc .moonlock.toml ở thư mục hiện tại / thư mục cha.
//! Bỏ qua chặn khẩn cấp: MOONLOCK_ALLOW_DEPLOY="lý do" (vd: "đã thắp hương xin phép").

use moonlock_core::{lunar, Config, DayInfo, Verdict};
use std::{env, fs, io::Write, path::PathBuf, process::ExitCode};

const CONFIG_FILE: &str = ".moonlock.toml";

struct Args {
    cmd: String,
    pos: Vec<String>,
    date: Option<String>,
    days: Option<i64>,
    out: Option<String>,
    config: Option<String>,
    strict: bool,
    leap: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args { cmd: "today".into(), pos: vec![], date: None, days: None, out: None, config: None, strict: false, leap: false };
    let mut it = env::args().skip(1);
    let mut first = true;
    while let Some(arg) = it.next() {
        let mut val = |name: &str| it.next().ok_or(format!("thiếu giá trị cho {name}"));
        match arg.as_str() {
            "--date" | "-d" => a.date = Some(val(&arg)?),
            "--days" | "-n" => a.days = Some(val(&arg)?.parse().map_err(|_| "--days phải là số")?),
            "--out" | "-o" => a.out = Some(val(&arg)?),
            "--config" | "-c" => a.config = Some(val(&arg)?),
            "--strict" => a.strict = true,
            "--leap" => a.leap = true,
            "-h" | "--help" | "help" => a.cmd = "help".into(),
            s if s.starts_with('-') => return Err(format!("không hiểu tuỳ chọn {s}")),
            s if first => a.cmd = s.into(),
            s => a.pos.push(s.into()),
        }
        first = false;
    }
    Ok(a)
}

fn find_config(explicit: Option<&str>) -> Result<(Config, Option<PathBuf>), String> {
    let path = match explicit {
        Some(p) => Some(PathBuf::from(p)),
        None => env::current_dir()
            .ok()
            .and_then(|d| d.ancestors().map(|a| a.join(CONFIG_FILE)).find(|p| p.is_file())),
    };
    match path {
        Some(p) => {
            let s = fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            Ok((Config::from_toml(&s).map_err(|e| format!("{}: {e}", p.display()))?, Some(p)))
        }
        None => Ok((Config::default(), None)),
    }
}

fn badge(v: Verdict) -> &'static str {
    match v {
        Verdict::Ok => "✅ DEPLOY ĐƯỢC",
        Verdict::Warn => "⚠️  CẨN THẬN",
        Verdict::Block => "⛔ CẤM DEPLOY",
    }
}

fn print_day(d: &DayInfo) {
    println!(
        "📅 {}, {:02}/{:02}/{}  ·  🌙 {}/{} năm {}{}",
        d.weekday_name,
        d.day,
        d.month,
        d.year,
        d.lunar.day,
        d.lunar_month_name,
        d.can_chi_year,
        if d.lunar.leap { " (nhuận)" } else { "" }
    );
    println!(
        "   Ngày {} · tháng {} · {} ({})",
        d.can_chi_day,
        d.can_chi_month,
        if d.hoang_dao { "hoàng đạo" } else { "hắc đạo" },
        d.star
    );
    for e in &d.events {
        println!("   🌕 {e}");
    }
    println!("🚦 {}", badge(d.verdict));
    for h in &d.block_reasons {
        println!("   ⛔ {}", h.label);
    }
    for h in &d.warn_reasons {
        println!("   ⚠️  {}", h.label);
    }
}

fn resolve_date(a: &Args, cfg: &Config) -> Result<i64, String> {
    match &a.date {
        Some(s) => moonlock_core::parse_date(s).ok_or(format!("ngày không hợp lệ: {s} (cần YYYY-MM-DD)")),
        None => Ok(moonlock_core::today_jd(cfg.timezone)),
    }
}

fn cmd_month(a: &Args, cfg: &Config) -> Result<(), String> {
    let today = moonlock_core::today_jd(cfg.timezone);
    let (_, tm, ty) = lunar::jd_to_date(today);
    let (y, m) = match a.pos.first() {
        Some(s) => {
            let (y, m) = s.split_once('-').ok_or("cần YYYY-MM")?;
            (y.parse().map_err(|_| "năm sai")?, m.parse().map_err(|_| "tháng sai")?)
        }
        None => (ty, tm),
    };
    if !(1..=12).contains(&m) {
        return Err("tháng phải 1..12".into());
    }
    let days = moonlock_core::month_view(y, m, cfg);
    println!("  Tháng {m}/{y}   \x1b[31m■ cấm\x1b[0m  \x1b[33m■ cẩn thận\x1b[0m  \x1b[32m■ ok\x1b[0m");
    println!("  T2     T3     T4     T5     T6     T7     CN");
    let offset = ((days[0].weekday + 6) % 7) as usize;
    let mut line = "       ".repeat(offset);
    for (i, d) in days.iter().enumerate() {
        let color = match d.verdict {
            Verdict::Block => 31,
            Verdict::Warn => 33,
            Verdict::Ok => 32,
        };
        let lunar = if d.lunar.day == 1 { format!("{}/{}", d.lunar.day, d.lunar.month) } else { d.lunar.day.to_string() };
        let style = if d.jd == today { format!("7;{color}") } else { color.to_string() };
        line.push_str(&format!("\x1b[{style}m{:>2}\x1b[0m\x1b[2m{lunar:<5}\x1b[0m", d.day));
        if (offset + i + 1).is_multiple_of(7) {
            println!("{line}");
            line.clear();
        }
    }
    if !line.is_empty() {
        println!("{line}");
    }
    println!("   (số nhỏ = ngày âm)");
    Ok(())
}

fn cmd_deploy_check(a: &Args, cfg: &Config) -> Result<ExitCode, String> {
    let jd = resolve_date(a, cfg)?;
    let d = moonlock_core::day_info(jd, cfg);
    print_day(&d);
    let blocked = d.verdict == Verdict::Block || (a.strict && d.verdict == Verdict::Warn);

    let next = if blocked { moonlock_core::next_deploy_window(jd + 1, cfg) } else { None };
    if let Some(n) = &next {
        println!("👉 Ngày deploy gần nhất: {}, {} ({}/{} âm)", n.weekday_name, n.date, n.lunar.day, n.lunar_month_name);
    }

    if let Ok(path) = env::var("GITHUB_STEP_SUMMARY") {
        let reasons: Vec<String> = d.block_reasons.iter().chain(&d.warn_reasons).map(|h| format!("- {}", h.label)).collect();
        let summary = format!(
            "### 🌙 moonlock deploy-check: {}\n**{}** — {}/{} âm lịch, ngày {}\n\n{}\n",
            badge(d.verdict),
            d.date,
            d.lunar.day,
            d.lunar_month_name,
            d.can_chi_day,
            reasons.join("\n")
        );
        if let Ok(mut f) = fs::OpenOptions::new().append(true).create(true).open(path) {
            let _ = f.write_all(summary.as_bytes());
        }
    }

    if !blocked {
        return Ok(ExitCode::SUCCESS);
    }
    match env::var("MOONLOCK_ALLOW_DEPLOY") {
        Ok(reason) if !reason.trim().is_empty() => {
            println!("🙏 Bỏ qua chặn: \"{}\". Chúc may mắn.", reason.trim());
            Ok(ExitCode::SUCCESS)
        }
        _ => {
            eprintln!("\nDeploy bị chặn. Khẩn cấp thì đặt MOONLOCK_ALLOW_DEPLOY=\"lý do\".");
            Ok(ExitCode::from(1))
        }
    }
}

fn cmd_lunar(a: &Args, cfg: &Config) -> Result<(), String> {
    let s = a.pos.first().ok_or("cần ngày âm, vd: moonlock lunar 15/8/2026")?;
    let parts: Vec<i64> = s.split('/').map(|p| p.parse().map_err(|_| format!("sai định dạng: {s}"))).collect::<Result<_, _>>()?;
    let today = lunar::lunar_from_jd(moonlock_core::today_jd(cfg.timezone), cfg.timezone);
    let (d, m, y) = match parts[..] {
        [d, m] => (d, m, today.year),
        [d, m, y] => (d, m, y),
        _ => return Err("cần ngày/tháng[/năm]".into()),
    };
    let jd = lunar::lunar_to_jd(d, m, y, a.leap, cfg.timezone).ok_or("năm này không có tháng nhuận đó")?;
    print_day(&moonlock_core::day_info(jd, cfg));
    Ok(())
}

fn run() -> Result<ExitCode, String> {
    let a = parse_args()?;
    let (cfg, cfg_path) = find_config(a.config.as_deref())?;
    match a.cmd.as_str() {
        "today" | "day" => print_day(&moonlock_core::day_info(resolve_date(&a, &cfg)?, &cfg)),
        "month" => cmd_month(&a, &cfg)?,
        "upcoming" => {
            let from = resolve_date(&a, &cfg)?;
            for d in moonlock_core::upcoming(from, a.days.unwrap_or(60), &cfg) {
                println!("{} {:<8} {:>5} âm  {}", d.date, d.weekday_name, format!("{}/{}", d.lunar.day, d.lunar.month), d.events.join(" · "));
            }
        }
        "deploy-check" | "check" => return cmd_deploy_check(&a, &cfg),
        "ics" => {
            let from = resolve_date(&a, &cfg)?;
            let ics = moonlock_core::build_ics(from, a.days.unwrap_or(365), &cfg);
            match &a.out {
                Some(p) => {
                    fs::write(p, ics).map_err(|e| e.to_string())?;
                    eprintln!("Đã ghi {p} — import vào Google/Apple/Outlook Calendar.");
                }
                None => print!("{ics}"),
            }
        }
        "lunar" => cmd_lunar(&a, &cfg)?,
        "init" => {
            if PathBuf::from(CONFIG_FILE).exists() {
                return Err(format!("{CONFIG_FILE} đã tồn tại"));
            }
            let mut sample = Config::default();
            sample.gio.push(moonlock_core::Gio { name: "Ông nội".into(), day: 12, month: 3, note: None });
            fs::write(CONFIG_FILE, format!("# moonlock — rules: {}\n{}", rule_list(), sample.to_toml()))
                .map_err(|e| e.to_string())?;
            println!("Đã tạo {CONFIG_FILE}");
        }
        "config" => {
            match cfg_path {
                Some(p) => println!("# {}", p.display()),
                None => println!("# (mặc định — chưa có {CONFIG_FILE})"),
            }
            print!("{}", cfg.to_toml());
        }
        "help" => print!("{}", include_str!("help.txt")),
        other => return Err(format!("lệnh không tồn tại: {other} (xem moonlock help)")),
    }
    Ok(ExitCode::SUCCESS)
}

fn rule_list() -> &'static str {
    "mung_mot, ram, tam_nuong, nguyet_ky, hac_dao, gio, tet, le_lon, friday, weekend"
}

fn main() -> ExitCode {
    match run() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("moonlock: {e}");
            ExitCode::from(2)
        }
    }
}
