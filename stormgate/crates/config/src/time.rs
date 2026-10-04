//! Minimal UTC date helpers so the core does not need a date/time dependency.

use std::time::{SystemTime, UNIX_EPOCH};

/// Seconds since the Unix epoch.
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Converts days since 1970-01-01 to a (year, month, day) civil date.
/// Algorithm from Howard Hinnant's `civil_from_days`.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `YYYY-MM-DD` (UTC) for the given Unix timestamp.
pub fn date_string(unix: u64) -> String {
    let (y, m, d) = civil_from_days((unix / 86_400) as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// `HHMMSS` (UTC) for the given Unix timestamp.
pub fn time_string(unix: u64) -> String {
    let s = unix % 86_400;
    format!("{:02}{:02}{:02}", s / 3600, (s % 3600) / 60, s % 60)
}

/// RFC 3339 timestamp (UTC) for the given Unix timestamp.
pub fn rfc3339(unix: u64) -> String {
    let s = unix % 86_400;
    format!(
        "{}T{:02}:{:02}:{:02}Z",
        date_string(unix),
        s / 3600,
        (s % 3600) / 60,
        s % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_dates() {
        assert_eq!(date_string(0), "1970-01-01");
        assert_eq!(date_string(951_782_400), "2000-02-29");
        // 2026-10-04T12:34:56Z
        assert_eq!(rfc3339(1_791_117_296), "2026-10-04T12:34:56Z");
        assert_eq!(time_string(1_791_117_296), "123456");
    }
}
