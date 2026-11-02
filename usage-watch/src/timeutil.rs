//! UTC calendar math and duration formatting without external date crates.

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Utc {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn epoch_to_utc(secs: u64) -> Utc {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    Utc {
        year,
        month,
        day,
        hour: (rem / 3_600) as u32,
        minute: ((rem % 3_600) / 60) as u32,
        second: (rem % 60) as u32,
    }
}

pub fn utc_now() -> Utc {
    epoch_to_utc(unix_now())
}

pub fn format_ymd(u: &Utc) -> String {
    format!("{:04}-{:02}-{:02}", u.year, u.month, u.day)
}

pub fn format_ymd_hms(u: &Utc) -> String {
    format!(
        "{} {:02}:{:02}:{:02} UTC",
        format_ymd(u),
        u.hour,
        u.minute,
        u.second
    )
}

/// Parse "YYYY-MM-DDTHH:MM:SS(.frac)?(Z|+HH:MM|-HH:MM)?" to epoch seconds.
pub fn parse_iso8601(s: &str) -> Option<u64> {
    let b = s.as_bytes();
    if b.len() < 19 {
        return None;
    }
    let year: i64 = s.get(0..4)?.parse().ok()?;
    if b[4] != b'-' {
        return None;
    }
    let month: u32 = s.get(5..7)?.parse().ok()?;
    if b[7] != b'-' {
        return None;
    }
    let day: u32 = s.get(8..10)?.parse().ok()?;
    let sep = *b.get(10)?;
    if sep != b'T' && sep != b' ' {
        return None;
    }
    let hour: u32 = s.get(11..13)?.parse().ok()?;
    if b[13] != b':' {
        return None;
    }
    let minute: u32 = s.get(14..16)?.parse().ok()?;
    if b[16] != b':' {
        return None;
    }
    let second: u32 = s.get(17..19)?.parse().ok()?;
    let mut rest = s.get(19..).unwrap_or("");
    if let Some(stripped) = rest.strip_prefix('.') {
        let digits: String = stripped
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        rest = &stripped[digits.len()..];
    }
    let mut offset = 0i64;
    match rest {
        "" | "Z" | "z" => {}
        _ => {
            let sign = match rest.as_bytes().first()? {
                b'+' => 1,
                b'-' => -1,
                _ => return None,
            };
            let oh: i64 = rest.get(1..3)?.parse().ok()?;
            let om: i64 = if rest.len() >= 6 && rest.as_bytes()[3] == b':' {
                rest.get(4..6)?.parse().ok()?
            } else if rest.len() >= 5 {
                rest.get(3..5)?.parse().ok()?
            } else {
                0
            };
            offset = sign * (oh * 3_600 + om * 60);
        }
    }
    let days = days_from_civil(year, month, day);
    let secs = days * 86_400 + hour as i64 * 3_600 + minute as i64 * 60 + second as i64 - offset;
    u64::try_from(secs).ok()
}

/// Human duration: "16d07h", "2h00m", "1m30s", "30s".
pub fn fmt_duration(secs: u64) -> String {
    if secs >= 86_400 {
        format!("{}d{:02}h", secs / 86_400, (secs % 86_400) / 3_600)
    } else if secs >= 3_600 {
        format!("{}h{:02}m", secs / 3_600, (secs % 3_600) / 60)
    } else if secs >= 60 {
        format!("{}m{:02}s", secs / 60, secs % 60)
    } else {
        format!("{secs}s")
    }
}

/// Howard Hinnant's civil-from-days algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Howard Hinnant's days-from-civil algorithm.
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m as i64 - 3 } else { m as i64 + 9 };
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}
