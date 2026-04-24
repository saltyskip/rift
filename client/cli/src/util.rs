use std::time::{Duration, UNIX_EPOCH};

/// Normalize a user-entered URL by prepending `https://` if no scheme is present.
pub fn normalize_web_url(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    }
}

/// Format a Unix epoch timestamp (milliseconds) as `YYYY-MM-DD`. Returns the
/// raw millisecond value as a string if decomposition fails — billing output
/// shouldn't error just because a date is unexpectedly far in the past.
pub fn format_date_ymd(ms: i64) -> String {
    let secs = (ms / 1000).max(0) as u64;
    let system = UNIX_EPOCH + Duration::from_secs(secs);
    match system.duration_since(UNIX_EPOCH) {
        Ok(d) => {
            let days = d.as_secs() / 86_400;
            let (y, m, day) = civil_from_days(days as i64);
            format!("{y:04}-{m:02}-{day:02}")
        }
        Err(_) => format!("{ms}"),
    }
}

/// Howard Hinnant's civil_from_days — decompose days-since-epoch into
/// (year, month, day). Used by `format_date_ymd`. Kept here (rather than
/// in chrono) to avoid pulling a full date-time crate just to print
/// billing period ends.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

#[cfg(test)]
#[path = "util_tests.rs"]
mod tests;
