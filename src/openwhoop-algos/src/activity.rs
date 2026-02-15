use chrono::{Duration, NaiveDateTime, TimeDelta};
use whoop::{Activity, ParsedHistoryReading};

const ACTIVITY_CHANGE_THRESHOLD: Duration = Duration::minutes(15);
const MIN_SLEEP_DURATION: Duration = Duration::minutes(60);
pub const MAX_SLEEP_PAUSE: Duration = Duration::minutes(60);
const MAX_PAUSE: Duration = Duration::minutes(10);

#[derive(Clone, Copy, Debug)]
pub struct ActivityPeriod {
    pub activity: Activity,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub duration: TimeDelta,
}

#[derive(Clone, Copy, Debug)]
struct TempActivity {
    activity: Activity,
    start: NaiveDateTime,
    end: NaiveDateTime,
}

impl ActivityPeriod {
    pub fn detect(history: &mut [ParsedHistoryReading]) -> Vec<ActivityPeriod> {
        Self::smooth_spikes(history);
        let changes = Self::detect_changes(history);

        Self::filter_merge(changes)
            .into_iter()
            .map(|a| ActivityPeriod {
                activity: a.activity,
                start: a.start,
                end: a.end,
                duration: a.end - a.start,
            })
            .collect()
    }

    pub fn is_active(&self) -> bool {
        matches!(self.activity, Activity::Active)
    }

    pub fn find_sleep(events: &mut Vec<ActivityPeriod>) -> Option<ActivityPeriod> {
        let mut next = || {
            if events.is_empty() {
                None
            } else {
                Some(events.remove(0))
            }
        };

        while let Some(event) = next() {
            if matches!(event.activity, Activity::Sleep) && event.duration > MIN_SLEEP_DURATION {
                return Some(event);
            }
        }

        None
    }

    fn smooth_spikes(data: &mut [ParsedHistoryReading]) {
        if data.len() < 3 {
            return;
        }

        let mut new_values = data.iter().map(|m| m.activity).collect::<Vec<_>>();

        for i in 1..data.len() - 1 {
            if data[i - 1].activity == data[i + 1].activity
                && data[i].activity != data[i - 1].activity
            {
                new_values[i] = data[i - 1].activity;
            }
        }

        for (i, model) in data.iter_mut().enumerate() {
            model.activity = new_values[i];
        }
    }

    fn filter_merge(mut activities: Vec<TempActivity>) -> Vec<TempActivity> {
        if activities.is_empty() {
            return Vec::new();
        }

        let mut merged = Vec::new();
        let mut i = 0;

        while i < activities.len() {
            let current = &activities[i];
            let duration = current.end - current.start;

            if duration < ACTIVITY_CHANGE_THRESHOLD {
                if i > 0
                    && i + 1 < activities.len()
                    && activities[i - 1].activity == activities[i + 1].activity
                    && !merged.is_empty()
                {
                    // Merge with both previous and next activity
                    let prev: TempActivity = merged.pop().unwrap();
                    merged.push(TempActivity {
                        activity: prev.activity,
                        start: prev.start,
                        end: activities[i + 1].end,
                    });
                    i += 1; // Skip next since it’s merged
                } else if i + 1 < activities.len() {
                    // Merge with next
                    activities[i + 1] = TempActivity {
                        activity: activities[i + 1].activity,
                        start: current.start,
                        end: activities[i + 1].end,
                    };
                } else if !merged.is_empty() {
                    // Merge with previous if at the end
                    let prev = merged.pop().unwrap();
                    merged.push(TempActivity {
                        activity: prev.activity,
                        start: prev.start,
                        end: current.end,
                    });
                }
            } else {
                merged.push(*current);
            }

            i += 1;
        }

        merged
    }
    fn detect_changes(history: &[ParsedHistoryReading]) -> Vec<TempActivity> {
        let mut periods = Vec::new();
        let mut iter = history.iter();

        if let Some(first) = iter.next() {
            let mut current_activity = first.activity;
            let mut start_time = first.time;
            let mut last_time = first.time;

            for model in iter {
                if model.activity != current_activity || (model.time - last_time > MAX_PAUSE) {
                    periods.push(TempActivity {
                        activity: current_activity,
                        start: start_time,
                        end: last_time,
                    });

                    current_activity = model.activity;
                    start_time = model.time;
                }
                last_time = model.time;
            }

            periods.push({
                TempActivity {
                    activity: current_activity,
                    start: start_time,
                    end: last_time,
                }
            });
        }

        periods
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_reading(hour: u32, min: u32, activity: Activity) -> ParsedHistoryReading {
        ParsedHistoryReading {
            time: NaiveDate::from_ymd_opt(2025, 1, 1)
                .unwrap()
                .and_hms_opt(hour, min, 0)
                .unwrap(),
            bpm: 70,
            rr: vec![],
            activity,
        }
    }

    fn make_reading_at(day: u32, hour: u32, min: u32, activity: Activity) -> ParsedHistoryReading {
        ParsedHistoryReading {
            time: NaiveDate::from_ymd_opt(2025, 1, day)
                .unwrap()
                .and_hms_opt(hour, min, 0)
                .unwrap(),
            bpm: 70,
            rr: vec![],
            activity,
        }
    }

    // ==================== smooth_spikes tests ====================

    #[test]
    fn test_smooth_spikes_empty_data() {
        let mut data: Vec<ParsedHistoryReading> = vec![];
        ActivityPeriod::smooth_spikes(&mut data);
        assert!(data.is_empty());
    }

    #[test]
    fn test_smooth_spikes_single_element() {
        let mut data = vec![make_reading(8, 0, Activity::Sleep)];
        ActivityPeriod::smooth_spikes(&mut data);
        assert_eq!(data[0].activity, Activity::Sleep);
    }

    #[test]
    fn test_smooth_spikes_two_elements() {
        let mut data = vec![
            make_reading(8, 0, Activity::Sleep),
            make_reading(8, 1, Activity::Active),
        ];
        ActivityPeriod::smooth_spikes(&mut data);
        // Should not change with only 2 elements
        assert_eq!(data[0].activity, Activity::Sleep);
        assert_eq!(data[1].activity, Activity::Active);
    }

    #[test]
    fn test_smooth_spikes_removes_single_spike() {
        // [Sleep, Active, Sleep] -> [Sleep, Sleep, Sleep]
        let mut data = vec![
            make_reading(8, 0, Activity::Sleep),
            make_reading(8, 1, Activity::Active),
            make_reading(8, 2, Activity::Sleep),
        ];
        ActivityPeriod::smooth_spikes(&mut data);
        assert_eq!(data[0].activity, Activity::Sleep);
        assert_eq!(data[1].activity, Activity::Sleep); // Smoothed
        assert_eq!(data[2].activity, Activity::Sleep);
    }

    #[test]
    fn test_smooth_spikes_preserves_sustained_change() {
        // [Sleep, Active, Active, Sleep] -> no change
        let mut data = vec![
            make_reading(8, 0, Activity::Sleep),
            make_reading(8, 1, Activity::Active),
            make_reading(8, 2, Activity::Active),
            make_reading(8, 3, Activity::Sleep),
        ];
        ActivityPeriod::smooth_spikes(&mut data);
        assert_eq!(data[0].activity, Activity::Sleep);
        assert_eq!(data[1].activity, Activity::Active);
        assert_eq!(data[2].activity, Activity::Active);
        assert_eq!(data[3].activity, Activity::Sleep);
    }

    #[test]
    fn test_smooth_spikes_multiple_spikes() {
        // [Sleep, Active, Sleep, Active, Sleep] -> [Sleep, Sleep, Sleep, Sleep, Sleep]
        let mut data = vec![
            make_reading(8, 0, Activity::Sleep),
            make_reading(8, 1, Activity::Active),
            make_reading(8, 2, Activity::Sleep),
            make_reading(8, 3, Activity::Active),
            make_reading(8, 4, Activity::Sleep),
        ];
        ActivityPeriod::smooth_spikes(&mut data);
        assert_eq!(data[1].activity, Activity::Sleep);
        assert_eq!(data[3].activity, Activity::Sleep);
    }

    // ==================== detect_changes tests ====================

    #[test]
    fn test_detect_changes_empty() {
        let data: Vec<ParsedHistoryReading> = vec![];
        let changes = ActivityPeriod::detect_changes(&data);
        assert!(changes.is_empty());
    }

    #[test]
    fn test_detect_changes_single_activity() {
        let data = vec![
            make_reading(8, 0, Activity::Sleep),
            make_reading(8, 1, Activity::Sleep),
            make_reading(8, 2, Activity::Sleep),
        ];
        let changes = ActivityPeriod::detect_changes(&data);
        assert_eq!(changes.len(), 1);
        assert!(matches!(changes[0].activity, Activity::Sleep));
    }

    #[test]
    fn test_detect_changes_identifies_transitions() {
        let data = vec![
            make_reading(8, 0, Activity::Sleep),
            make_reading(8, 1, Activity::Sleep),
            make_reading(8, 2, Activity::Active),
            make_reading(8, 3, Activity::Active),
        ];
        let changes = ActivityPeriod::detect_changes(&data);
        assert_eq!(changes.len(), 2);
        assert!(matches!(changes[0].activity, Activity::Sleep));
        assert!(matches!(changes[1].activity, Activity::Active));
    }

    #[test]
    fn test_detect_changes_gap_creates_new_period() {
        // Gap > MAX_PAUSE (10 min) creates new period even with same activity
        let data = vec![
            make_reading(8, 0, Activity::Sleep),
            make_reading(8, 1, Activity::Sleep),
            make_reading(8, 15, Activity::Sleep), // 14 min gap > MAX_PAUSE
        ];
        let changes = ActivityPeriod::detect_changes(&data);
        assert_eq!(changes.len(), 2);
    }

    // ==================== find_sleep tests ====================

    #[test]
    fn test_find_sleep_empty_events() {
        let mut events: Vec<ActivityPeriod> = vec![];
        assert!(ActivityPeriod::find_sleep(&mut events).is_none());
    }

    #[test]
    fn test_find_sleep_no_sleep_events() {
        let mut events = vec![ActivityPeriod {
            activity: Activity::Active,
            start: make_reading(8, 0, Activity::Active).time,
            end: make_reading(10, 0, Activity::Active).time,
            duration: TimeDelta::hours(2),
        }];
        assert!(ActivityPeriod::find_sleep(&mut events).is_none());
        assert!(events.is_empty()); // Events are consumed
    }

    #[test]
    fn test_find_sleep_ignores_short_sleep() {
        // Sleep < 60 min should be ignored
        let mut events = vec![ActivityPeriod {
            activity: Activity::Sleep,
            start: make_reading(8, 0, Activity::Sleep).time,
            end: make_reading(8, 30, Activity::Sleep).time,
            duration: TimeDelta::minutes(30),
        }];
        assert!(ActivityPeriod::find_sleep(&mut events).is_none());
    }

    #[test]
    fn test_find_sleep_returns_first_valid_sleep() {
        let mut events = vec![
            ActivityPeriod {
                activity: Activity::Active,
                start: make_reading(7, 0, Activity::Active).time,
                end: make_reading(8, 0, Activity::Active).time,
                duration: TimeDelta::hours(1),
            },
            ActivityPeriod {
                activity: Activity::Sleep,
                start: make_reading_at(1, 23, 0, Activity::Sleep).time,
                end: make_reading_at(2, 7, 0, Activity::Sleep).time,
                duration: TimeDelta::hours(8),
            },
        ];
        let sleep = ActivityPeriod::find_sleep(&mut events);
        assert!(sleep.is_some());
        assert!(matches!(sleep.unwrap().activity, Activity::Sleep));
        assert_eq!(sleep.unwrap().duration, TimeDelta::hours(8));
    }

    // ==================== is_active tests ====================

    #[test]
    fn test_is_active_returns_true_for_active() {
        let period = ActivityPeriod {
            activity: Activity::Active,
            start: make_reading(8, 0, Activity::Active).time,
            end: make_reading(9, 0, Activity::Active).time,
            duration: TimeDelta::hours(1),
        };
        assert!(period.is_active());
    }

    #[test]
    fn test_is_active_returns_false_for_sleep() {
        let period = ActivityPeriod {
            activity: Activity::Sleep,
            start: make_reading(23, 0, Activity::Sleep).time,
            end: make_reading_at(2, 7, 0, Activity::Sleep).time,
            duration: TimeDelta::hours(8),
        };
        assert!(!period.is_active());
    }

    // ==================== full detect pipeline tests ====================

    #[test]
    fn test_detect_empty_history() {
        let mut history: Vec<ParsedHistoryReading> = vec![];
        let periods = ActivityPeriod::detect(&mut history);
        assert!(periods.is_empty());
    }

    #[test]
    fn test_detect_single_activity_type() {
        let mut history: Vec<ParsedHistoryReading> = (0..30)
            .map(|i| make_reading(8, i, Activity::Sleep))
            .collect();
        let periods = ActivityPeriod::detect(&mut history);
        // Should produce a single period
        assert_eq!(periods.len(), 1);
        assert!(matches!(periods[0].activity, Activity::Sleep));
    }

    #[test]
    fn test_detect_smooths_and_merges() {
        // Create readings spanning 20 minutes (> 15 min threshold)
        // with a spike in the middle that should be smoothed
        let mut history = vec![
            make_reading(8, 0, Activity::Sleep),
            make_reading(8, 5, Activity::Sleep),
            make_reading(8, 10, Activity::Active), // Spike - should be smoothed
            make_reading(8, 15, Activity::Sleep),
            make_reading(8, 20, Activity::Sleep),
        ];
        let periods = ActivityPeriod::detect(&mut history);
        // After smoothing, should be all sleep -> 1 period
        assert_eq!(periods.len(), 1);
        assert!(matches!(periods[0].activity, Activity::Sleep));
    }
}
