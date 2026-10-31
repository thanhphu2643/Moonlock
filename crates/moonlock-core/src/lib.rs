//! moonlock-core — âm lịch Việt Nam + luật "ngày kiêng deploy".

pub mod lunar;

use lunar::{is_last_day_of_month, jd_from_date, jd_to_date, lunar_from_jd, weekday, LunarDate};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

pub use lunar::VN_TZ;

pub const CAN: [&str; 10] = ["Giáp", "Ất", "Bính", "Đinh", "Mậu", "Kỷ", "Canh", "Tân", "Nhâm", "Quý"];
pub const CHI: [&str; 12] = ["Tý", "Sửu", "Dần", "Mão", "Thìn", "Tỵ", "Ngọ", "Mùi", "Thân", "Dậu", "Tuất", "Hợi"];
pub const WEEKDAYS: [&str; 7] = ["Chủ nhật", "Thứ 2", "Thứ 3", "Thứ 4", "Thứ 5", "Thứ 6", "Thứ 7"];

/// 12 vị thần trực ngày; `true` = hoàng đạo.
const STARS: [(&str, bool); 12] = [
    ("Thanh Long", true),
    ("Minh Đường", true),
    ("Thiên Hình", false),
    ("Chu Tước", false),
    ("Kim Quỹ", true),
    ("Kim Đường", true),
    ("Bạch Hổ", false),
    ("Ngọc Đường", true),
    ("Thiên Lao", false),
    ("Huyền Vũ", false),
    ("Tư Mệnh", true),
    ("Câu Trận", false),
];

/// Lễ âm lịch (tháng, ngày, tên). Chỉ tính tháng không nhuận.
const HOLIDAYS: [(i64, i64, &str); 12] = [
    (1, 1, "Tết Nguyên Đán"),
    (1, 2, "Mùng 2 Tết"),
    (1, 3, "Mùng 3 Tết"),
    (1, 15, "Tết Nguyên Tiêu"),
    (3, 3, "Tết Hàn Thực"),
    (3, 10, "Giỗ Tổ Hùng Vương"),
    (4, 15, "Lễ Phật Đản"),
    (5, 5, "Tết Đoan Ngọ"),
    (7, 15, "Lễ Vu Lan"),
    (8, 15, "Tết Trung Thu"),
    (10, 15, "Tết Hạ Nguyên"),
    (12, 23, "Ông Công Ông Táo"),
];

pub fn can_chi_day(jd: i64) -> String {
    format!("{} {}", CAN[(jd + 9).rem_euclid(10) as usize], CHI[(jd + 1).rem_euclid(12) as usize])
}
pub fn can_chi_month(month: i64, year: i64) -> String {
    format!("{} {}", CAN[(year * 12 + month + 3).rem_euclid(10) as usize], CHI[(month + 1).rem_euclid(12) as usize])
}
pub fn can_chi_year(year: i64) -> String {
    format!("{} {}", CAN[(year + 6).rem_euclid(10) as usize], CHI[(year + 8).rem_euclid(12) as usize])
}

pub fn month_name(month: i64, leap: bool) -> String {
    let base = match month {
        1 => "Giêng".to_string(),
        12 => "Chạp".to_string(),
        m => m.to_string(),
    };
    if leap { format!("{base} nhuận") } else { base }
}

/// Trực tinh của ngày: (tên sao, hoàng đạo?).
pub fn star_of_day(jd: i64, lunar_month: i64) -> (&'static str, bool) {
    let day_chi = (jd + 1).rem_euclid(12);
    let start = ((lunar_month - 1).rem_euclid(6)) * 2; // tháng 1,7: Tý; 2,8: Dần; ...
    STARS[(day_chi - start).rem_euclid(12) as usize]
}

// ───────────────────────────── Rules & config ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    MungMot,
    Ram,
    TamNuong,
    NguyetKy,
    HacDao,
    Gio,
    Tet,
    LeLon,
    Friday,
    Weekend,
}

impl Rule {
    pub const ALL: [Rule; 10] = [
        Rule::MungMot,
        Rule::Ram,
        Rule::TamNuong,
        Rule::NguyetKy,
        Rule::HacDao,
        Rule::Gio,
        Rule::Tet,
        Rule::LeLon,
        Rule::Friday,
        Rule::Weekend,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Rule::MungMot => "Mùng 1",
            Rule::Ram => "Rằm (15 âm)",
            Rule::TamNuong => "Tam Nương (3, 7, 13, 18, 22, 27)",
            Rule::NguyetKy => "Nguyệt Kỵ (5, 14, 23)",
            Rule::HacDao => "Ngày hắc đạo",
            Rule::Gio => "Ngày giỗ trong nhà",
            Rule::Tet => "Tết (23 tháng Chạp → mùng 7 Giêng)",
            Rule::LeLon => "Lễ âm lịch lớn",
            Rule::Friday => "Thứ 6",
            Rule::Weekend => "Cuối tuần",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Gio {
    pub name: String,
    /// Ngày âm (1..30). Ngày 30 mà tháng thiếu thì cúng vào ngày 29.
    pub day: i64,
    /// Tháng âm (1..12). Giỗ tháng nhuận được cúng vào tháng thường.
    pub month: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DeployPolicy {
    /// Trúng luật này → cấm deploy (CLI exit 1).
    pub block: Vec<Rule>,
    /// Trúng luật này → cảnh báo (exit 0, trừ khi --strict).
    pub warn: Vec<Rule>,
}

impl Default for DeployPolicy {
    fn default() -> Self {
        Self {
            block: vec![Rule::MungMot, Rule::Ram, Rule::TamNuong, Rule::NguyetKy, Rule::Gio, Rule::Tet],
            warn: vec![Rule::HacDao, Rule::LeLon, Rule::Friday],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    /// Múi giờ tính âm lịch (Việt Nam = 7).
    pub timezone: f64,
    /// Nhắc trước bao nhiêu ngày (rằm, mùng 1, giỗ, lễ).
    pub remind_days_before: i64,
    pub deploy: DeployPolicy,
    pub gio: Vec<Gio>,
}

impl Default for Config {
    fn default() -> Self {
        Self { timezone: VN_TZ, remind_days_before: 1, deploy: DeployPolicy::default(), gio: vec![] }
    }
}

impl Config {
    pub fn from_toml(s: &str) -> Result<Self, String> {
        toml::from_str(s).map_err(|e| e.to_string())
    }
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).expect("config is always serializable")
    }
}

// ───────────────────────────── Day evaluation ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hit {
    pub rule: Rule,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Ok,
    Warn,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayInfo {
    pub jd: i64,
    /// YYYY-MM-DD
    pub date: String,
    pub year: i64,
    pub month: i64,
    pub day: i64,
    /// 0 = Chủ nhật
    pub weekday: i64,
    pub weekday_name: String,
    pub lunar: LunarDate,
    pub lunar_month_name: String,
    pub can_chi_day: String,
    pub can_chi_month: String,
    pub can_chi_year: String,
    pub star: String,
    pub hoang_dao: bool,
    /// Mọi luật mà ngày này dính.
    pub hits: Vec<Hit>,
    /// Kết luận deploy theo policy.
    pub verdict: Verdict,
    pub block_reasons: Vec<Hit>,
    pub warn_reasons: Vec<Hit>,
    /// Sự kiện cần nhắc: rằm, mùng 1, giỗ, lễ.
    pub events: Vec<String>,
}

fn holiday(l: &LunarDate, last_day: bool) -> Option<&'static str> {
    if l.leap {
        return None;
    }
    if l.month == 12 && last_day {
        return Some("Giao Thừa (Trừ Tịch)");
    }
    HOLIDAYS.iter().find(|(m, d, _)| *m == l.month && *d == l.day).map(|(_, _, n)| *n)
}

fn gio_matches(g: &Gio, l: &LunarDate, last_day: bool) -> bool {
    if l.leap || g.month != l.month {
        return false;
    }
    g.day == l.day || (g.day == 30 && l.day == 29 && last_day)
}

pub fn day_info(jd: i64, cfg: &Config) -> DayInfo {
    let tz = cfg.timezone;
    let (d, m, y) = jd_to_date(jd);
    let l = lunar_from_jd(jd, tz);
    let last_day = is_last_day_of_month(jd, tz);
    let wd = weekday(jd);
    let (star, hoang_dao) = star_of_day(jd, l.month);
    let mname = month_name(l.month, l.leap);

    let mut hits = Vec::new();
    let mut events = Vec::new();
    let mut hit = |rule: Rule, label: String| hits.push(Hit { rule, label });

    let hol = holiday(&l, last_day);

    if l.day == 1 {
        hit(Rule::MungMot, format!("Mùng 1 tháng {mname} — deploy đầu tháng là bug cả tháng"));
        if hol.is_none() {
            events.push(format!("Mùng 1 tháng {mname}"));
        }
    }
    if l.day == 15 {
        hit(Rule::Ram, format!("Rằm tháng {mname} — để team còn đi chùa"));
        if hol.is_none() {
            events.push(format!("Rằm tháng {mname}"));
        }
    }
    if [3, 7, 13, 18, 22, 27].contains(&l.day) {
        hit(Rule::TamNuong, "Tam Nương — \"mùng 3, mùng 7, 13, 18, 22, 27\" làm gì cũng hỏng".into());
    }
    if [5, 14, 23].contains(&l.day) {
        hit(Rule::NguyetKy, "Nguyệt Kỵ — \"mùng 5, 14, 23, đi chơi còn lỗ nữa là deploy\"".into());
    }
    if !hoang_dao {
        hit(Rule::HacDao, format!("Hắc đạo ({star})"));
    }
    for g in cfg.gio.iter().filter(|g| gio_matches(g, &l, last_day)) {
        hit(Rule::Gio, format!("Giỗ {} — về nhà phụ cỗ, không phải trực on-call", g.name));
        events.push(format!("Giỗ {}", g.name));
    }
    if !l.leap && ((l.month == 12 && l.day >= 23) || (l.month == 1 && l.day <= 7)) {
        hit(Rule::Tet, "Tết — code freeze toàn quốc".into());
    }
    if let Some(name) = hol {
        hit(Rule::LeLon, name.to_string());
        events.push(name.to_string());
    }
    if wd == 5 {
        hit(Rule::Friday, "Thứ 6 — deploy chiều thứ 6 = on-call cả cuối tuần".into());
    }
    if wd == 0 || wd == 6 {
        hit(Rule::Weekend, "Cuối tuần".into());
    }

    let block_reasons: Vec<Hit> = hits.iter().filter(|h| cfg.deploy.block.contains(&h.rule)).cloned().collect();
    let warn_reasons: Vec<Hit> = hits
        .iter()
        .filter(|h| !cfg.deploy.block.contains(&h.rule) && cfg.deploy.warn.contains(&h.rule))
        .cloned()
        .collect();
    let verdict = if !block_reasons.is_empty() {
        Verdict::Block
    } else if !warn_reasons.is_empty() {
        Verdict::Warn
    } else {
        Verdict::Ok
    };

    DayInfo {
        jd,
        date: format!("{y:04}-{m:02}-{d:02}"),
        year: y,
        month: m,
        day: d,
        weekday: wd,
        weekday_name: WEEKDAYS[wd as usize].into(),
        lunar_month_name: mname,
        can_chi_day: can_chi_day(jd),
        can_chi_month: can_chi_month(l.month, l.year),
        can_chi_year: can_chi_year(l.year),
        star: star.into(),
        hoang_dao,
        lunar: l,
        hits,
        verdict,
        block_reasons,
        warn_reasons,
        events,
    }
}

// ───────────────────────────── Helpers ─────────────────────────────

/// Julian day number of "today" in the given timezone.
pub fn today_jd(tz: f64) -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    ((secs + tz * 3600.0) / 86400.0).floor() as i64 + 2440588
}

/// Parse "YYYY-MM-DD" → Julian day number.
pub fn parse_date(s: &str) -> Option<i64> {
    let mut it = s.trim().split('-').map(|p| p.parse::<i64>().ok());
    let (y, m, d) = (it.next()??, it.next()??, it.next()??);
    if it.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let jd = jd_from_date(d, m, y);
    (jd_to_date(jd) == (d, m, y)).then_some(jd)
}

pub fn month_view(year: i64, month: i64, cfg: &Config) -> Vec<DayInfo> {
    let start = jd_from_date(1, month, year);
    let end = if month == 12 { jd_from_date(1, 1, year + 1) } else { jd_from_date(1, month + 1, year) };
    (start..end).map(|jd| day_info(jd, cfg)).collect()
}

/// Days in [from, from+days) that have something to remind about.
pub fn upcoming(from: i64, days: i64, cfg: &Config) -> Vec<DayInfo> {
    (from..from + days).map(|jd| day_info(jd, cfg)).filter(|d| !d.events.is_empty()).collect()
}

/// Next day (from `from`, inclusive) that is not blocked.
pub fn next_deploy_window(from: i64, cfg: &Config) -> Option<DayInfo> {
    (from..from + 120).map(|jd| day_info(jd, cfg)).find(|d| d.verdict != Verdict::Block)
}

// ───────────────────────────── ICS export ─────────────────────────────

fn ics_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace(';', "\\;").replace(',', "\\,").replace('\n', "\\n")
}

fn ics_date(jd: i64) -> String {
    let (d, m, y) = jd_to_date(jd);
    format!("{y:04}{m:02}{d:02}")
}

/// iCalendar: ngày cấm deploy (BUSY) + nhắc rằm/mùng 1/giỗ (có VALARM).
pub fn build_ics(from: i64, days: i64, cfg: &Config) -> String {
    let stamp = format!("{}T000000Z", ics_date(today_jd(0.0)));
    let mut out = String::from(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//moonlock//VI\r\nCALSCALE:GREGORIAN\r\n\
         X-WR-CALNAME:Moonlock\r\n",
    );
    let mut ev = |uid: String, jd: i64, summary: String, desc: String, busy: bool, alarm: Option<i64>| {
        let _ = write!(
            out,
            "BEGIN:VEVENT\r\nUID:{uid}@moonlock.dev\r\nDTSTAMP:{stamp}\r\nDTSTART;VALUE=DATE:{}\r\n\
             DTEND;VALUE=DATE:{}\r\nSUMMARY:{}\r\nDESCRIPTION:{}\r\nTRANSP:{}\r\n",
            ics_date(jd),
            ics_date(jd + 1),
            ics_escape(&summary),
            ics_escape(&desc),
            if busy { "OPAQUE" } else { "TRANSPARENT" }
        );
        if let Some(n) = alarm {
            let _ = write!(
                out,
                "BEGIN:VALARM\r\nACTION:DISPLAY\r\nDESCRIPTION:{}\r\nTRIGGER:-P{n}DT-9H\r\nEND:VALARM\r\n",
                ics_escape(&summary)
            );
        }
        out.push_str("END:VEVENT\r\n");
    };
    for jd in from..from + days {
        let info = day_info(jd, cfg);
        let lunar = format!("{}/{} âm lịch", info.lunar.day, info.lunar_month_name);
        if info.verdict == Verdict::Block {
            let reasons: Vec<&str> = info.block_reasons.iter().map(|h| h.label.as_str()).collect();
            ev(
                format!("{}-freeze", info.date),
                jd,
                "🚫 Deploy freeze".into(),
                format!("{lunar}\n{}", reasons.join("\n")),
                true,
                None,
            );
        }
        if !info.events.is_empty() {
            ev(
                format!("{}-event", info.date),
                jd,
                format!("🌕 {}", info.events.join(" · ")),
                lunar,
                false,
                Some(cfg.remind_days_before.max(0)),
            );
        }
    }
    out.push_str("END:VCALENDAR\r\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_chi() {
        assert_eq!(can_chi_year(2026), "Bính Ngọ");
        assert_eq!(can_chi_year(2024), "Giáp Thìn");
        // 2000-01-01 là ngày Mậu Ngọ.
        assert_eq!(can_chi_day(2451545), "Mậu Ngọ");
        // Tháng Giêng năm Giáp Thìn là tháng Bính Dần.
        assert_eq!(can_chi_month(1, 2024), "Bính Dần");
    }

    #[test]
    fn tet_is_blocked() {
        let cfg = Config::default();
        let d = day_info(jd_from_date(17, 2, 2026), &cfg);
        assert_eq!(d.verdict, Verdict::Block);
        assert!(d.events.contains(&"Tết Nguyên Đán".to_string()));
        assert!(d.hits.iter().any(|h| h.rule == Rule::Tet));
    }

    #[test]
    fn gio_on_short_month() {
        let mut cfg = Config::default();
        cfg.gio.push(Gio { name: "Cụ".into(), day: 30, month: 8, note: None });
        // Tháng 8 âm 2026 bắt đầu 11/9/2026; tìm ngày cuối tháng.
        let start = lunar::lunar_to_jd(1, 8, 2026, false, VN_TZ).unwrap();
        let last = (start..start + 31).find(|&jd| lunar::is_last_day_of_month(jd, VN_TZ)).unwrap();
        let d = day_info(last, &cfg);
        assert!(d.events.iter().any(|e| e == "Giỗ Cụ"), "{:?}", d.lunar);
    }

    #[test]
    fn config_roundtrip() {
        let mut cfg = Config::default();
        cfg.gio.push(Gio { name: "Ông nội".into(), day: 12, month: 3, note: Some("mua gà".into()) });
        assert_eq!(Config::from_toml(&cfg.to_toml()).unwrap(), cfg);
        let partial = Config::from_toml("[deploy]\nblock = [\"ram\"]\n").unwrap();
        assert_eq!(partial.deploy.block, vec![Rule::Ram]);
        assert_eq!(partial.timezone, 7.0);
    }

    #[test]
    fn parse() {
        assert_eq!(parse_date("2000-01-01"), Some(2451545));
        assert_eq!(parse_date("2026-02-30"), None);
        assert_eq!(parse_date("nope"), None);
    }

    #[test]
    fn ics_has_events() {
        let ics = build_ics(jd_from_date(1, 2, 2026), 30, &Config::default());
        assert!(ics.contains("Tết Nguyên Đán"));
        assert!(ics.contains("Deploy freeze"));
        assert!(ics.ends_with("END:VCALENDAR\r\n"));
    }
}
