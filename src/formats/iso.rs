//! # iso
//!
//! ISO 8601 date-time parsing shared by readers whose metadata stores
//! times as text (CV7000, Panta Rhei).
//!
//! Provenance: ISO 8601 (date and time format); proleptic Gregorian day
//! count. Own code (moved from the CV7000 reader). No GPL code consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-11-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

/// Seconds since 1970 of an ISO 8601 time `YYYY-MM-DDThh:mm:ss[.f][Z|±hh:mm]`.
pub(super) fn iso_seconds(s: &str) -> Option<f64> {
    let (date, time) = s.split_once('T')?;
    let mut d = date.splitn(3, '-').map(|v| v.parse::<i64>().ok());
    let (y, m, day) = (d.next()??, d.next()??, d.next()??);
    let split = time.find(['Z', '+']).or_else(|| time.rfind('-')).unwrap_or(time.len());
    let (clock, zone) = time.split_at(split);
    let mut c = clock.splitn(3, ':');
    let (h, min, sec): (f64, f64, f64) = (c.next()?.parse().ok()?, c.next()?.parse().ok()?, c.next()?.parse().ok()?);
    let offset = match zone.as_bytes().first() {
        Some(b'+' | b'-') => {
            let (oh, om) = zone[1..].split_once(':').unwrap_or((&zone[1..], "0"));
            let v = oh.parse::<f64>().ok()? * 3600.0 + om.parse::<f64>().ok()? * 60.0;
            if zone.starts_with('-') { -v } else { v }
        }
        _ => 0.0,
    };
    // Days from civil (proleptic Gregorian).
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + day - 1;
    let days = era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468;
    Some(days as f64 * 86400.0 + h * 3600.0 + min * 60.0 + sec - offset)
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_zones_and_fractions() {
        let a = iso_seconds("2017-09-16T15:09:27.897+02:00").unwrap();
        let b = iso_seconds("2017-09-16T13:09:28.897Z").unwrap();
        assert!((b - a - 1.0).abs() < 1e-6);
        assert_eq!(iso_seconds("1970-01-02T00:00:00").unwrap(), 86400.0);
        assert!((iso_seconds("2026-10-02T11:43:51.742615").unwrap() - iso_seconds("2026-10-02T11:43:50.5").unwrap() - 1.242615).abs() < 1e-6);
        assert_eq!(iso_seconds("not a time"), None);
    }
}
