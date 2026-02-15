use chrono::{NaiveTime, TimeDelta, Timelike as _};

pub fn map_time(time: &NaiveTime) -> i64 {
    let mut h = time.hour() as i64;
    if h > 12 {
        h -= 24;
    }
    let m = time.minute() as i64;
    let s = time.second() as i64;
    h * 3600 + m * 60 + s
}

pub fn std_time(times: &[NaiveTime], mean: &NaiveTime) -> NaiveTime {
    if times.is_empty() {
        NaiveTime::default()
    } else {
        let mean = map_time(mean);
        let variance = times
            .iter()
            .map(map_time)
            .map(|x| (x - mean).pow(2))
            .sum::<i64>()
            / times.len() as i64;

        let variance = variance.isqrt();
        let h = variance / 3600;
        let m = (variance % 3600) / 60;
        let s = variance % 60;

        NaiveTime::from_hms_opt(h as u32, m as u32, s as u32).expect("Invalid time")
    }
}

pub fn mean_time(times: &[NaiveTime]) -> NaiveTime {
    if times.is_empty() {
        NaiveTime::default()
    } else {
        let mut mean = times.iter().map(map_time).sum::<i64>() / times.len() as i64;
        if mean < 0 {
            mean += 86400;
        }
        let h = mean / 3600;
        let m = (mean % 3600) / 60;
        let s = mean % 60;
        NaiveTime::from_hms_opt(h as u32, m as u32, s as u32).expect("Invalid time")
    }
}

pub fn mean_deltas(durations: &[TimeDelta]) -> TimeDelta {
    if durations.is_empty() {
        TimeDelta::default()
    } else {
        durations.iter().sum::<TimeDelta>() / durations.len() as i32
    }
}

pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0_f64
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

pub fn std_dev_delta(durations: &[TimeDelta], mean: TimeDelta) -> TimeDelta {
    if durations.is_empty() {
        TimeDelta::default()
    } else {
        let variance = durations
            .iter()
            .map(|x| (*x - mean).num_seconds().pow(2))
            .sum::<i64>()
            / durations.len() as i64;

        TimeDelta::seconds(variance.isqrt())
    }
}

pub fn round_float(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    // ==================== map_time tests ====================

    #[test]
    fn test_map_time_morning_returns_positive_seconds() {
        // 08:00:00 should return 8 * 3600 = 28800 seconds
        let time = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
        assert_eq!(map_time(&time), 28800);
    }

    #[test]
    fn test_map_time_noon_returns_positive_seconds() {
        // 12:00:00 should return 12 * 3600 = 43200 seconds
        let time = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
        assert_eq!(map_time(&time), 43200);
    }

    #[test]
    fn test_map_time_afternoon_returns_negative_seconds() {
        // 22:00:00 should return (22-24) * 3600 = -7200 seconds
        // This is for proper averaging of sleep times across midnight
        let time = NaiveTime::from_hms_opt(22, 0, 0).unwrap();
        assert_eq!(map_time(&time), -7200);
    }

    #[test]
    fn test_map_time_just_after_noon_returns_negative() {
        // 13:00:00 should return (13-24) * 3600 = -39600 seconds
        let time = NaiveTime::from_hms_opt(13, 0, 0).unwrap();
        assert_eq!(map_time(&time), -39600);
    }

    #[test]
    fn test_map_time_with_minutes_and_seconds() {
        // 10:30:45 should return 10*3600 + 30*60 + 45 = 37845
        let time = NaiveTime::from_hms_opt(10, 30, 45).unwrap();
        assert_eq!(map_time(&time), 37845);
    }

    #[test]
    fn test_map_time_midnight() {
        // 00:00:00 should return 0
        let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        assert_eq!(map_time(&time), 0);
    }

    // ==================== mean_time tests ====================

    #[test]
    fn test_mean_time_empty_returns_default() {
        let times: Vec<NaiveTime> = vec![];
        assert_eq!(mean_time(&times), NaiveTime::default());
    }

    #[test]
    fn test_mean_time_single_time_returns_same() {
        let times = vec![NaiveTime::from_hms_opt(8, 0, 0).unwrap()];
        assert_eq!(mean_time(&times), NaiveTime::from_hms_opt(8, 0, 0).unwrap());
    }

    #[test]
    fn test_mean_time_morning_times() {
        // Mean of 08:00 and 10:00 should be 09:00
        let times = vec![
            NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
        ];
        assert_eq!(mean_time(&times), NaiveTime::from_hms_opt(9, 0, 0).unwrap());
    }

    #[test]
    fn test_mean_time_across_midnight_for_sleep_times() {
        // Mean of 23:00 and 01:00 should be around midnight (00:00)
        // This tests the PM adjustment logic for sleep time averaging
        let times = vec![
            NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(1, 0, 0).unwrap(),
        ];
        // 23:00 maps to -3600, 01:00 maps to 3600
        // Mean = 0 = midnight
        assert_eq!(mean_time(&times), NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    }

    // ==================== std_time tests ====================

    #[test]
    fn test_std_time_empty_returns_default() {
        let times: Vec<NaiveTime> = vec![];
        let mean = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        assert_eq!(std_time(&times, &mean), NaiveTime::default());
    }

    #[test]
    fn test_std_time_identical_times_returns_zero() {
        let times = vec![
            NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
        ];
        let mean = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
        assert_eq!(std_time(&times, &mean), NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    }

    #[test]
    fn test_std_time_with_variation() {
        // Times with 1 hour difference from mean
        let times = vec![
            NaiveTime::from_hms_opt(7, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
        ];
        let mean = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
        let std = std_time(&times, &mean);
        // Variance = ((7-8)^2 + (9-8)^2) * 3600^2 / 2 = 3600^2
        // Std dev = 3600 seconds = 1 hour
        assert_eq!(std, NaiveTime::from_hms_opt(1, 0, 0).unwrap());
    }

    // ==================== mean_deltas tests ====================

    #[test]
    fn test_mean_deltas_empty_returns_default() {
        let durations: Vec<TimeDelta> = vec![];
        assert_eq!(mean_deltas(&durations), TimeDelta::default());
    }

    #[test]
    fn test_mean_deltas_single_duration() {
        let durations = vec![TimeDelta::hours(8)];
        assert_eq!(mean_deltas(&durations), TimeDelta::hours(8));
    }

    #[test]
    fn test_mean_deltas_multiple_durations() {
        let durations = vec![
            TimeDelta::hours(6),
            TimeDelta::hours(8),
            TimeDelta::hours(10),
        ];
        assert_eq!(mean_deltas(&durations), TimeDelta::hours(8));
    }

    #[test]
    fn test_mean_deltas_with_minutes() {
        let durations = vec![
            TimeDelta::minutes(30),
            TimeDelta::minutes(90),
        ];
        assert_eq!(mean_deltas(&durations), TimeDelta::minutes(60));
    }

    // ==================== std_dev_delta tests ====================

    #[test]
    fn test_std_dev_delta_empty_returns_default() {
        let durations: Vec<TimeDelta> = vec![];
        let mean = TimeDelta::hours(0);
        assert_eq!(std_dev_delta(&durations, mean), TimeDelta::default());
    }

    #[test]
    fn test_std_dev_delta_identical_returns_zero() {
        let durations = vec![
            TimeDelta::hours(8),
            TimeDelta::hours(8),
            TimeDelta::hours(8),
        ];
        let mean = TimeDelta::hours(8);
        assert_eq!(std_dev_delta(&durations, mean), TimeDelta::seconds(0));
    }

    #[test]
    fn test_std_dev_delta_with_variation() {
        // 6h and 10h around mean of 8h
        let durations = vec![
            TimeDelta::hours(6),
            TimeDelta::hours(10),
        ];
        let mean = TimeDelta::hours(8);
        let std = std_dev_delta(&durations, mean);
        // Differences: -2h = -7200s, +2h = +7200s
        // Variance = (7200^2 + 7200^2) / 2 = 51840000
        // Std dev = sqrt(51840000) = 7200 seconds = 2 hours
        assert_eq!(std, TimeDelta::seconds(7200));
    }

    // ==================== mean tests ====================

    #[test]
    fn test_mean_empty_returns_zero() {
        let values: Vec<f64> = vec![];
        assert_eq!(mean(&values), 0.0);
    }

    #[test]
    fn test_mean_single_value() {
        let values = vec![42.0];
        assert_eq!(mean(&values), 42.0);
    }

    #[test]
    fn test_mean_multiple_values() {
        let values = vec![10.0, 20.0, 30.0];
        assert_eq!(mean(&values), 20.0);
    }

    #[test]
    fn test_mean_with_decimals() {
        let values = vec![1.5, 2.5, 3.0];
        let result = mean(&values);
        assert!((result - 2.333333).abs() < 0.0001);
    }

    // ==================== round_float tests ====================

    #[test]
    fn test_round_float_two_decimal_places() {
        assert_eq!(round_float(3.14159), 3.14);
    }

    #[test]
    fn test_round_float_rounds_up() {
        assert_eq!(round_float(3.145), 3.15);
    }

    #[test]
    fn test_round_float_rounds_down() {
        assert_eq!(round_float(3.144), 3.14);
    }

    #[test]
    fn test_round_float_whole_number() {
        assert_eq!(round_float(5.0), 5.0);
    }

    #[test]
    fn test_round_float_negative() {
        assert_eq!(round_float(-2.567), -2.57);
    }

    #[test]
    fn test_round_float_zero() {
        assert_eq!(round_float(0.0), 0.0);
    }
}
