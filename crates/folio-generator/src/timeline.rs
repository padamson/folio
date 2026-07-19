//! Spreading capture timestamps across a time window.
//!
//! A collection is usually several photos taken over a stretch of time. This
//! module turns a window and a count into evenly-spaced timestamps, which the
//! caller pairs with prompts and output paths.

use chrono::{DateTime, Utc};

/// Evenly spread `count` timestamps across `[start, end)`.
///
/// The first timestamp is `start`; each subsequent one advances by
/// `(end - start) / count`, so the last lands just short of `end` (leaving room
/// to abut a following batch without overlap). Returns an empty vector for
/// `count == 0`, and `[start]` for `count == 1`.
///
/// # Example
///
/// ```
/// use chrono::{TimeZone, Utc, Duration};
/// use folio_generator::timeline::spread;
///
/// let start = Utc.with_ymd_and_hms(2024, 11, 28, 14, 0, 0).unwrap();
/// let end = start + Duration::hours(2);
/// let stamps = spread(start, end, 4);
/// assert_eq!(stamps.len(), 4);
/// assert_eq!(stamps[0], start);
/// // Evenly spaced by 30 minutes; the last is 90 minutes in, not at `end`.
/// assert_eq!(stamps[3], start + Duration::minutes(90));
/// ```
pub fn spread(start: DateTime<Utc>, end: DateTime<Utc>, count: usize) -> Vec<DateTime<Utc>> {
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![start];
    }
    let window = end - start;
    (0..count)
        .map(|i| start + window * i as i32 / count as i32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn base() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 11, 28, 14, 0, 0).unwrap()
    }

    #[test]
    fn zero_count_is_empty() {
        assert!(spread(base(), base() + Duration::hours(1), 0).is_empty());
    }

    #[test]
    fn single_count_is_just_start() {
        assert_eq!(spread(base(), base() + Duration::hours(1), 1), vec![base()]);
    }

    #[test]
    fn spreads_evenly_and_stays_in_bounds() {
        let start = base();
        let end = start + Duration::hours(2);
        let stamps = spread(start, end, 4);
        assert_eq!(stamps.len(), 4);
        assert_eq!(stamps[0], start);
        assert_eq!(stamps[1], start + Duration::minutes(30));
        assert_eq!(stamps[2], start + Duration::hours(1));
        assert_eq!(stamps[3], start + Duration::minutes(90));
        // Strictly increasing and below `end`.
        for pair in stamps.windows(2) {
            assert!(pair[0] < pair[1]);
        }
        assert!(*stamps.last().unwrap() < end);
    }
}
