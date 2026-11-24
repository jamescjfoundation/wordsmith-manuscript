// SPDX-License-Identifier: MIT OR Apache-2.0
//! Snapshots (INS-5): named, dated copies of one item's text.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::Text;

/// A snapshot of an item's text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// When it was taken, as a UTC stamp (`20261124T151230Z`); also its file name.
    pub taken: String,
    /// Its name.
    pub name: String,
    /// The text as it was.
    pub text: Text,
}

/// The most snapshots kept for one item.
pub const MAX_PER_ITEM: usize = 500;

fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// A UTC stamp for a moment: `YYYYMMDDTHHMMSSZ`.
pub fn stamp(at: SystemTime) -> String {
    let secs = at.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let secs = i64::try_from(secs).unwrap_or(0);
    let (y, m, d) = civil(secs.div_euclid(86_400));
    let s = secs.rem_euclid(86_400);
    format!("{y:04}{m:02}{d:02}T{:02}{:02}{:02}Z", s / 3600, (s / 60) % 60, s % 60)
}

/// Whether text is a snapshot stamp (with an optional `-n` when two fall in one second).
pub fn is_stamp(s: &str) -> bool {
    let (base, n) = s.split_once('-').map_or((s, None), |(a, b)| (a, Some(b)));
    let ok_base = base.len() == 16
        && base.char_indices().all(|(i, c)| match i {
            8 => c == 'T',
            15 => c == 'Z',
            _ => c.is_ascii_digit(),
        });
    ok_base && n.is_none_or(|n| !n.is_empty() && n.len() <= 4 && n.chars().all(|c| c.is_ascii_digit()))
}

/// The order of stamps: by time, then by the `-n` counter (none first).
pub fn order_key(stamp: &str) -> (String, u32) {
    match stamp.split_once('-') {
        Some((base, n)) => (base.to_owned(), n.parse().unwrap_or(u32::MAX)),
        None => (stamp.to_owned(), 1),
    }
}

/// A readable form of a stamp: `2026-11-24 15:12 UTC`.
pub fn readable(stamp: &str) -> String {
    let g = |a: usize, b: usize| stamp.get(a..b).unwrap_or("");
    format!("{}-{}-{} {}:{} UTC", g(0, 4), g(4, 6), g(6, 8), g(9, 11), g(11, 13))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn stamps() {
        let t = UNIX_EPOCH + Duration::from_secs(1_795_000_000);
        let s = stamp(t);
        assert_eq!(s.len(), 16);
        assert!(is_stamp(&s));
        assert!(is_stamp(&format!("{s}-2")));
        assert!(!is_stamp("2026-11-24"));
        assert!(!is_stamp("../../etc"));
        assert_eq!(stamp(UNIX_EPOCH), "19700101T000000Z");
        assert_eq!(readable("20261124T151230Z"), "2026-11-24 15:12 UTC");
    }
}
