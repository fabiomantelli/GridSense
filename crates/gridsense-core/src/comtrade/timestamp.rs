/// Days since 1970-01-01 for a proleptic-Gregorian calendar date (Howard Hinnant's
/// `days_from_civil` formulation). Returns `None` if `m`/`d` are out of range for the
/// given year (catches e.g. 29 Feb on a non-leap year, or month 13).
fn days_from_civil(y: i64, m: u32, d: u32) -> Option<i64> {
    if !(1..=12).contains(&m) || d < 1 || d > days_in_month(y, m) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let m_adj = if m > 2 { m as i64 - 3 } else { m as i64 + 9 };
    let doy = (153 * m_adj + 2) / 5 + d as i64 - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    Some(era * 146097 + doe - 719468)
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(y) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Parses a COMTRADE start/trigger timestamp line (`dd/mm/yyyy,hh:mm:ss.ffffff`, per
/// IEEE C37.111) into microseconds since the Unix epoch (1970-01-01T00:00:00, no UTC
/// offset applied — the format carries none). Returns `None` on any
/// malformed/unparseable input rather than erroring, matching this crate's existing
/// lenient-defaults convention (see `cfg.rs`) — a record with a missing or
/// vendor-mangled timestamp should still load, just without an absolute time anchor.
pub fn parse_comtrade_timestamp(raw: &str) -> Option<f64> {
    let raw = raw.trim();
    let (date_part, time_part) = raw.split_once(',')?;

    let mut date_fields = date_part.splitn(3, '/');
    let dd: u32 = date_fields.next()?.trim().parse().ok()?;
    let mm: u32 = date_fields.next()?.trim().parse().ok()?;
    let yyyy: i64 = date_fields.next()?.trim().parse().ok()?;
    // Reject non-standard 2-digit years rather than silently misreading them — the
    // spec mandates 4-digit years, so this is a safe, documented lenient-reject.
    if yyyy < 1000 {
        return None;
    }

    let mut time_fields = time_part.splitn(3, ':');
    let hh: u32 = time_fields.next()?.trim().parse().ok()?;
    let mm_time: u32 = time_fields.next()?.trim().parse().ok()?;
    let sec_part = time_fields.next()?.trim();
    if hh >= 24 || mm_time >= 60 {
        return None;
    }

    let (ss_str, frac_str) = sec_part.split_once('.').unwrap_or((sec_part, ""));
    let ss: u32 = ss_str.parse().ok()?;
    if ss >= 60 {
        return None;
    }
    // Right-pad/truncate to exactly 6 digits — some vendors emit milliseconds (3
    // digits) instead of the spec's microseconds (6 digits).
    let mut frac_digits = frac_str.to_string();
    if frac_digits.len() > 6 {
        frac_digits.truncate(6);
    } else {
        while frac_digits.len() < 6 {
            frac_digits.push('0');
        }
    }
    let frac_us: u32 = frac_digits.parse().ok()?;

    let days = days_from_civil(yyyy, mm, dd)?;

    Some(
        days as f64 * 86_400_000_000.0
            + hh as f64 * 3_600_000_000.0
            + mm_time as f64 * 60_000_000.0
            + ss as f64 * 1_000_000.0
            + frac_us as f64,
    )
}
