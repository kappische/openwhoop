use chrono::{NaiveDate, NaiveDateTime, TimeDelta};
use whoop::ParsedHistoryReading;

use super::ActivityPeriod;

// Physiological bounds for RR intervals (in milliseconds)
// These filter out impossible/artifact values before HRV calculation
const MIN_RR_MS: u64 = 300; // >200 bpm - physiologically impossible at rest
const MAX_RR_MS: u64 = 2000; // <30 bpm - too slow for normal rhythm

// Threshold for detecting ectopic beats / artifacts
// RR intervals deviating more than this percentage from local median are removed
// WHOOP uses a "Strong" filter equivalent to ~20-25% threshold
const ECTOPIC_THRESHOLD_PERCENT: f64 = 0.20;

// Window size for local median calculation (number of surrounding beats)
const MEDIAN_WINDOW_SIZE: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SleepCycle {
    pub id: NaiveDate,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub min_bpm: u8,
    pub max_bpm: u8,
    pub avg_bpm: u8,
    pub min_hrv: u16,
    pub max_hrv: u16,
    pub avg_hrv: u16,
    pub score: f64,
}

impl SleepCycle {
    pub fn from_event(event: ActivityPeriod, history: &[ParsedHistoryReading]) -> SleepCycle {
        let (heart_rate, rr): (Vec<u64>, Vec<Vec<_>>) = history
            .iter()
            .filter(|h| h.time >= event.start && h.time <= event.end)
            .map(|h| (h.bpm as u64, h.rr.clone()))
            .unzip();

        let rr = Self::clean_rr(rr);
        let rolling_hrv = Self::rolling_hrv(rr);

        let min_hrv = rolling_hrv.iter().min().copied().unwrap_or_default() as u16;
        let max_hrv = rolling_hrv.iter().max().copied().unwrap_or_default() as u16;

        let hrv_count = rolling_hrv.len() as u64;
        let avg_hrv = if hrv_count > 0 {
            (rolling_hrv.into_iter().sum::<u64>() / hrv_count) as u16
        } else {
            0
        };

        let min_bpm = heart_rate.iter().min().copied().unwrap_or_default() as u8;
        let max_bpm = heart_rate.iter().max().copied().unwrap_or_default() as u8;

        let heart_rate_count = heart_rate.len() as u64;
        let bpm = heart_rate.into_iter().sum::<u64>() / heart_rate_count;
        let avg_bpm = bpm as u8;

        let id = event.end.date();

        Self {
            id,
            start: event.start,
            end: event.end,
            min_bpm,
            max_bpm,
            avg_bpm,
            min_hrv,
            max_hrv,
            avg_hrv,
            score: Self::sleep_score(event.start, event.end),
        }
    }

    pub fn duration(&self) -> TimeDelta {
        self.end - self.start
    }

    /// Cleans and filters RR intervals to match WHOOP's methodology.
    ///
    /// This applies three levels of filtering:
    /// 1. Flattens and averages multiple RR values per reading
    /// 2. Removes physiologically impossible values (outside 300-2000ms)
    /// 3. Removes ectopic beats that deviate >20% from local median
    fn clean_rr(rr: Vec<Vec<u16>>) -> Vec<u64> {
        // Step 1: Flatten and average RR intervals per reading
        let flattened: Vec<u64> = rr
            .into_iter()
            .filter_map(|rr| {
                if rr.is_empty() {
                    return None;
                }
                let count = rr.len() as u64;
                let rr_sum = rr.into_iter().map(u64::from).sum::<u64>();
                Some(rr_sum / count)
            })
            .collect();

        // Step 2: Apply physiological bounds filter
        let bounded: Vec<u64> = flattened
            .into_iter()
            .filter(|&rr| rr >= MIN_RR_MS && rr <= MAX_RR_MS)
            .collect();

        // Step 3: Apply ectopic beat / artifact filter using local median
        Self::filter_ectopic_beats(bounded)
    }

    /// Filters out ectopic beats and artifacts using threshold-based detection.
    ///
    /// An RR interval is considered an artifact if it deviates more than
    /// ECTOPIC_THRESHOLD_PERCENT (20%) from the local median of surrounding beats.
    /// This matches WHOOP's "Strong" filter methodology.
    fn filter_ectopic_beats(rr: Vec<u64>) -> Vec<u64> {
        if rr.len() < MEDIAN_WINDOW_SIZE {
            return rr;
        }

        let mut filtered = Vec::with_capacity(rr.len());
        let half_window = MEDIAN_WINDOW_SIZE / 2;

        for i in 0..rr.len() {
            // Calculate local median from surrounding beats
            let start = i.saturating_sub(half_window);
            let end = (i + half_window + 1).min(rr.len());

            let mut window: Vec<u64> = rr[start..end].to_vec();
            window.sort_unstable();
            let local_median = window[window.len() / 2];

            // Check if current beat deviates too much from local median
            let current = rr[i];
            let deviation = (current as f64 - local_median as f64).abs() / local_median as f64;

            if deviation <= ECTOPIC_THRESHOLD_PERCENT {
                filtered.push(current);
            }
            // If deviation > threshold, the beat is filtered out (ectopic/artifact)
        }

        filtered
    }

    fn rolling_hrv(rr: Vec<u64>) -> Vec<u64> {
        rr.windows(300).filter_map(Self::calculate_rmssd).collect()
    }

    fn calculate_rmssd(window: &[u64]) -> Option<u64> {
        Self::calculate_rmssd_f64(window).map(|v| v as u64)
    }

    /// Calculates RMSSD (Root Mean Square of Successive Differences) as f64.
    ///
    /// RMSSD is the standard HRV metric used by WHOOP and other devices.
    /// It measures the variation between consecutive heartbeats.
    fn calculate_rmssd_f64(window: &[u64]) -> Option<f64> {
        if window.len() < 2 {
            return None;
        }

        let rr_diff: Vec<f64> = window
            .windows(2)
            .map(|w| (w[1] as f64 - w[0] as f64).powi(2))
            .collect();

        let rr_count = rr_diff.len() as f64;
        Some((rr_diff.into_iter().sum::<f64>() / rr_count).sqrt())
    }

    /// Calculates the natural logarithm of RMSSD (Ln RMSSD).
    ///
    /// WHOOP and many research studies use Ln RMSSD because:
    /// - It normalizes the typically skewed distribution of RMSSD values
    /// - It's more stable and less affected by outliers
    /// - It allows for better comparison across individuals
    ///
    /// Typical Ln RMSSD values range from ~2.5 (low HRV) to ~4.5 (high HRV)
    #[allow(dead_code)]
    fn calculate_ln_rmssd(window: &[u64]) -> Option<f64> {
        Self::calculate_rmssd_f64(window).map(|rmssd| rmssd.ln())
    }

    pub fn sleep_score(start: NaiveDateTime, end: NaiveDateTime) -> f64 {
        let duration = (end - start).num_seconds();
        const IDEAL_DURATION: i64 = 60 * 60 * 8;

        let score = (duration / IDEAL_DURATION) as f64;

        (score * 100.0).clamp(0.0, 100.0)
    }
}
