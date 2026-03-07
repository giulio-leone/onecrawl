//! Shared utilities used across multiple CDP modules.

use onecrawl_core::{Error, Result};

// ── Name / path-traversal validation ────────────────────────────────

/// Reject names that contain path-traversal characters (`/`, `\`, `..`).
///
/// Call this wherever a user-supplied name is later used as part of a
/// filesystem path (session names, plugin names, project IDs, …).
pub fn validate_safe_name(name: &str) -> Result<()> {
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(Error::Cdp("Invalid name: contains path traversal characters".into()));
    }
    Ok(())
}

// ── ISO-8601 timestamps (no `chrono` dependency) ────────────────────

/// ISO-8601 UTC timestamp without fractional seconds: `2025-01-15T12:34:56Z`.
pub fn iso_now() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs();
    let (y, mo, day, h, m, s) = secs_to_ymdhms(secs);
    format!("{y:04}-{mo:02}-{day:02}T{h:02}:{m:02}:{s:02}Z")
}

/// ISO-8601 UTC timestamp *with* milliseconds: `2025-01-15T12:34:56.789Z`.
pub fn iso_now_millis() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs();
    let millis = d.subsec_millis();
    let (y, mo, day, h, m, s) = secs_to_ymdhms(secs);
    format!("{y:04}-{mo:02}-{day:02}T{h:02}:{m:02}:{s:02}.{millis:03}Z")
}

// ── Internal helpers ────────────────────────────────────────────────

/// Convert a Unix-epoch seconds value to `(year, month, day, hour, minute, second)`.
fn secs_to_ymdhms(secs: u64) -> (i64, u64, u64, u64, u64, u64) {
    const SECS_PER_DAY: u64 = 86_400;
    let days = secs / SECS_PER_DAY;
    let time_of_day = secs % SECS_PER_DAY;
    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    let s = time_of_day % 60;

    let (y, mo, d) = days_to_ymd(days);
    (y, mo, d, h, m, s)
}

/// Convert a day count since the Unix epoch to `(year, month, day)`.
///
/// Uses Howard Hinnant's algorithm:
/// <http://howardhinnant.github.io/date_algorithms.html>
fn days_to_ymd(days: u64) -> (i64, u64, u64) {
    const DAYS_PER_400Y: u64 = 146_097;
    const DAYS_PER_100Y: u64 = 36_524;
    const DAYS_PER_4Y: u64 = 1_461;

    let days_from_epoch = days as i64 + 719_468; // shift to 0000-03-01
    let era = if days_from_epoch >= 0 {
        days_from_epoch / DAYS_PER_400Y as i64
    } else {
        (days_from_epoch - (DAYS_PER_400Y as i64 - 1)) / DAYS_PER_400Y as i64
    };
    let doe = (days_from_epoch - era * DAYS_PER_400Y as i64) as u64;
    let yoe = (doe - doe / (DAYS_PER_4Y - 1) + doe / DAYS_PER_100Y
        - doe / (DAYS_PER_400Y - 1))
        / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100 + yoe / 400);
    let mp = (5 * doy + 2) / 153;
    let d_val = doy - (153 * mp + 2) / 5 + 1;
    let m_val = if mp < 10 { mp + 3 } else { mp - 9 };
    let y_val = if m_val <= 2 { y + 1 } else { y };
    (y_val, m_val, d_val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_name_rejects_traversal() {
        assert!(validate_safe_name("good-name").is_ok());
        assert!(validate_safe_name("my_session").is_ok());
        assert!(validate_safe_name("../etc/passwd").is_err());
        assert!(validate_safe_name("foo/bar").is_err());
        assert!(validate_safe_name("foo\\bar").is_err());
    }

    #[test]
    fn iso_now_format() {
        let ts = iso_now();
        assert!(ts.ends_with('Z'));
        assert_eq!(ts.len(), 20); // YYYY-MM-DDTHH:MM:SSZ
    }

    #[test]
    fn iso_now_millis_format() {
        let ts = iso_now_millis();
        assert!(ts.ends_with('Z'));
        assert_eq!(ts.len(), 24); // YYYY-MM-DDTHH:MM:SS.mmmZ
    }

    #[test]
    fn days_to_ymd_epoch() {
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
    }

    #[test]
    fn days_to_ymd_known_date() {
        // 2024-01-01 = day 19723
        assert_eq!(days_to_ymd(19723), (2024, 1, 1));
    }
}
