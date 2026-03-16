/// Convert Unix epoch seconds (UTC) to civil date components.
///
/// Returns `(year, month, day, hour, minute)` using the Howard Hinnant
/// civil_from_days algorithm (Gregorian proleptic calendar).
pub fn epoch_to_civil(epoch: i64) -> (i64, i64, i64, i64, i64) {
    let days = epoch / 86400;
    let secs_in_day = epoch.rem_euclid(86400);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    let hh = secs_in_day / 3600;
    let mm = (secs_in_day % 3600) / 60;
    (y, m, d, hh, mm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_epoch_zero() {
        let (y, m, d, h, min) = epoch_to_civil(0);
        assert_eq!((y, m, d, h, min), (1970, 1, 1, 0, 0));
    }

    #[test]
    fn known_date() {
        // 2026-03-15 14:30:00 UTC = 1773764400 + some offset
        // Let's use a simpler known value: 2000-01-01 00:00:00 = 946684800
        let (y, m, d, h, min) = epoch_to_civil(946684800);
        assert_eq!((y, m, d, h, min), (2000, 1, 1, 0, 0));
    }
}
