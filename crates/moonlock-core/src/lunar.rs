//! Vietnamese lunar calendar conversion.
//!
//! Port of Hồ Ngọc Đức's astronomical algorithm
//! (https://www.informatik.uni-leipzig.de/~duc/amlich/), which is the de-facto
//! reference used by Vietnamese calendars. All "day numbers" are Julian Day
//! Numbers (integer days).

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Vietnam standard time (UTC+7).
pub const VN_TZ: f64 = 7.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LunarDate {
    pub day: i64,
    pub month: i64,
    pub year: i64,
    pub leap: bool,
}

/// Gregorian (or Julian before 1582-10-15) date → Julian Day Number.
pub fn jd_from_date(d: i64, m: i64, y: i64) -> i64 {
    let a = (14 - m) / 12;
    let y = y + 4800 - a;
    let m = m + 12 * a - 3;
    let jd = d + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045;
    if jd < 2299161 {
        d + (153 * m + 2) / 5 + 365 * y + y / 4 - 32083
    } else {
        jd
    }
}

/// Julian Day Number → (day, month, year).
pub fn jd_to_date(jd: i64) -> (i64, i64, i64) {
    let (b, c) = if jd > 2299160 {
        let a = jd + 32044;
        let b = (4 * a + 3) / 146097;
        (b, a - (b * 146097) / 4)
    } else {
        (0, jd + 32082)
    };
    let d = (4 * c + 3) / 1461;
    let e = c - (1461 * d) / 4;
    let m = (5 * e + 2) / 153;
    let day = e - (153 * m + 2) / 5 + 1;
    let month = m + 3 - 12 * (m / 10);
    let year = b * 100 + d - 4800 + m / 10;
    (day, month, year)
}

/// 0 = Sunday … 6 = Saturday.
pub fn weekday(jd: i64) -> i64 {
    (jd + 1).rem_euclid(7)
}

/// Julian day (fractional) of the k-th new moon after 1900-01-01.
fn new_moon(k: f64) -> f64 {
    let t = k / 1236.85;
    let t2 = t * t;
    let t3 = t2 * t;
    let dr = PI / 180.0;
    let mut jd1 = 2415020.75933 + 29.53058868 * k + 0.0001178 * t2 - 0.000000155 * t3;
    jd1 += 0.00033 * ((166.56 + 132.87 * t - 0.009173 * t2) * dr).sin();
    let m = 359.2242 + 29.10535608 * k - 0.0000333 * t2 - 0.00000347 * t3;
    let mpr = 306.0253 + 385.81691806 * k + 0.0107306 * t2 + 0.00001236 * t3;
    let f = 21.2964 + 390.67050646 * k - 0.0016528 * t2 - 0.00000239 * t3;
    let mut c1 = (0.1734 - 0.000393 * t) * (m * dr).sin() + 0.0021 * (2.0 * dr * m).sin();
    c1 = c1 - 0.4068 * (mpr * dr).sin() + 0.0161 * (dr * 2.0 * mpr).sin();
    c1 -= 0.0004 * (dr * 3.0 * mpr).sin();
    c1 = c1 + 0.0104 * (dr * 2.0 * f).sin() - 0.0051 * (dr * (m + mpr)).sin();
    c1 = c1 - 0.0074 * (dr * (m - mpr)).sin() + 0.0004 * (dr * (2.0 * f + m)).sin();
    c1 = c1 - 0.0004 * (dr * (2.0 * f - m)).sin() - 0.0006 * (dr * (2.0 * f + mpr)).sin();
    c1 = c1 + 0.0010 * (dr * (2.0 * f - mpr)).sin() + 0.0005 * (dr * (2.0 * mpr + m)).sin();
    let deltat = if t < -11.0 {
        0.001 + 0.000839 * t + 0.0002261 * t2 - 0.00000845 * t3 - 0.000000081 * t * t3
    } else {
        -0.000278 + 0.000265 * t + 0.000262 * t2
    };
    jd1 + c1 - deltat
}

/// Sun longitude in radians [0, 2π) at the given (fractional) Julian day.
fn sun_longitude(jdn: f64) -> f64 {
    let t = (jdn - 2451545.0) / 36525.0;
    let t2 = t * t;
    let dr = PI / 180.0;
    let m = 357.52910 + 35999.05030 * t - 0.0001559 * t2 - 0.00000048 * t * t2;
    let l0 = 280.46645 + 36000.76983 * t + 0.0003032 * t2;
    let mut dl = (1.914600 - 0.004817 * t - 0.000014 * t2) * (dr * m).sin();
    dl += (0.019993 - 0.000101 * t) * (dr * 2.0 * m).sin() + 0.000290 * (dr * 3.0 * m).sin();
    let l = (l0 + dl) * dr;
    l - 2.0 * PI * (l / (2.0 * PI)).floor()
}

fn new_moon_day(k: i64, tz: f64) -> i64 {
    (new_moon(k as f64) + 0.5 + tz / 24.0).floor() as i64
}

/// Which of the 12 solar-term sectors (0..11) the sun is in at local midnight.
fn sun_sector(day: i64, tz: f64) -> i64 {
    (sun_longitude(day as f64 - 0.5 - tz / 24.0) / PI * 6.0).floor() as i64
}

/// Day number of the start of lunar month 11 (the month containing the winter solstice).
fn lunar_month11(y: i64, tz: f64) -> i64 {
    let off = (jd_from_date(31, 12, y) - 2415021) as f64;
    let k = (off / 29.530588853).floor() as i64;
    let nm = new_moon_day(k, tz);
    if sun_sector(nm, tz) >= 9 {
        new_moon_day(k - 1, tz)
    } else {
        nm
    }
}

fn leap_month_offset(a11: i64, tz: f64) -> i64 {
    let k = ((a11 as f64 - 2415021.076998695) / 29.530588853 + 0.5).floor() as i64;
    let mut i = 1;
    let mut arc = sun_sector(new_moon_day(k + i, tz), tz);
    loop {
        let last = arc;
        i += 1;
        arc = sun_sector(new_moon_day(k + i, tz), tz);
        if arc == last || i >= 14 {
            break;
        }
    }
    i - 1
}

/// Julian day number → lunar date.
pub fn lunar_from_jd(jd: i64, tz: f64) -> LunarDate {
    let k = ((jd as f64 - 2415021.076998695) / 29.530588853).floor() as i64;
    let mut month_start = new_moon_day(k + 1, tz);
    if month_start > jd {
        month_start = new_moon_day(k, tz);
    }
    let (_, _, yy) = jd_to_date(jd);
    let mut a11 = lunar_month11(yy, tz);
    let mut b11 = a11;
    let mut year;
    if a11 >= month_start {
        year = yy;
        a11 = lunar_month11(yy - 1, tz);
    } else {
        year = yy + 1;
        b11 = lunar_month11(yy + 1, tz);
    }
    let day = jd - month_start + 1;
    let diff = ((month_start - a11) as f64 / 29.0).floor() as i64;
    let mut leap = false;
    let mut month = diff + 11;
    if b11 - a11 > 365 {
        let leap_diff = leap_month_offset(a11, tz);
        if diff >= leap_diff {
            month = diff + 10;
            leap = diff == leap_diff;
        }
    }
    if month > 12 {
        month -= 12;
    }
    if month >= 11 && diff < 4 {
        year -= 1;
    }
    LunarDate { day, month, year, leap }
}

pub fn solar_to_lunar(d: i64, m: i64, y: i64, tz: f64) -> LunarDate {
    lunar_from_jd(jd_from_date(d, m, y), tz)
}

/// Lunar date → Julian day number. `None` if the leap month does not exist that year.
pub fn lunar_to_jd(day: i64, month: i64, year: i64, leap: bool, tz: f64) -> Option<i64> {
    let (a11, b11) = if month < 11 {
        (lunar_month11(year - 1, tz), lunar_month11(year, tz))
    } else {
        (lunar_month11(year, tz), lunar_month11(year + 1, tz))
    };
    let k = (0.5 + (a11 as f64 - 2415021.076998695) / 29.530588853).floor() as i64;
    let mut off = month - 11;
    if off < 0 {
        off += 12;
    }
    if b11 - a11 > 365 {
        let leap_off = leap_month_offset(a11, tz);
        let mut leap_month = leap_off - 2;
        if leap_month < 0 {
            leap_month += 12;
        }
        if leap && month != leap_month {
            return None;
        } else if leap || off >= leap_off {
            off += 1;
        }
    } else if leap {
        return None;
    }
    Some(new_moon_day(k + off, tz) + day - 1)
}

/// Is `jd` the last day of its lunar month (29th or 30th)?
pub fn is_last_day_of_month(jd: i64, tz: f64) -> bool {
    lunar_from_jd(jd + 1, tz).day == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lunar(d: i64, m: i64, y: i64) -> (i64, i64, i64, bool) {
        let l = solar_to_lunar(d, m, y, VN_TZ);
        (l.day, l.month, l.year, l.leap)
    }

    #[test]
    fn tet_dates() {
        assert_eq!(lunar(22, 1, 2023), (1, 1, 2023, false));
        assert_eq!(lunar(10, 2, 2024), (1, 1, 2024, false));
        assert_eq!(lunar(29, 1, 2025), (1, 1, 2025, false));
        assert_eq!(lunar(17, 2, 2026), (1, 1, 2026, false));
        assert_eq!(lunar(6, 2, 2027), (1, 1, 2027, false));
    }

    #[test]
    fn mid_autumn() {
        assert_eq!(lunar(17, 9, 2024), (15, 8, 2024, false));
        assert_eq!(lunar(25, 9, 2026), (15, 8, 2026, false));
    }

    #[test]
    fn leap_months() {
        // 2023 có tháng 2 nhuận, 2025 có tháng 6 nhuận.
        assert_eq!(lunar(22, 3, 2023), (1, 2, 2023, true));
        assert_eq!(lunar(25, 7, 2025), (1, 6, 2025, true));
        assert_eq!(lunar_to_jd(1, 6, 2025, true, VN_TZ), Some(jd_from_date(25, 7, 2025)));
        assert_eq!(lunar_to_jd(1, 6, 2026, true, VN_TZ), None);
    }

    #[test]
    fn roundtrip() {
        let start = jd_from_date(1, 1, 1990);
        for jd in (start..start + 365 * 60).step_by(7) {
            let l = lunar_from_jd(jd, VN_TZ);
            assert_eq!(lunar_to_jd(l.day, l.month, l.year, l.leap, VN_TZ), Some(jd), "{l:?}");
        }
    }

    #[test]
    fn jd_and_weekday() {
        assert_eq!(jd_from_date(1, 1, 2000), 2451545);
        assert_eq!(jd_to_date(2451545), (1, 1, 2000));
        assert_eq!(weekday(jd_from_date(1, 11, 2026)), 0); // Chủ nhật
    }
}
