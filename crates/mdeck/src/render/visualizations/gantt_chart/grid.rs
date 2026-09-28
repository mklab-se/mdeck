//! The time axis: its unit and where its date labels go.

use super::date::Date;

// ─── Timeline Scale ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub(super) enum TimeScale {
    Days,
    Weeks,
    Months,
}

pub(super) struct TimeGrid {
    pub(super) scale: TimeScale,
    pub(super) labels: Vec<(f32, String)>, // (fraction 0..1, label)
}

pub(super) fn compute_time_grid(min_date: &Date, max_date: &Date, total_days: i64) -> TimeGrid {
    let scale = if total_days <= 21 {
        TimeScale::Days
    } else if total_days <= 120 {
        TimeScale::Weeks
    } else {
        TimeScale::Months
    };

    let mut labels = Vec::new();

    match scale {
        TimeScale::Days => {
            let mut d = *min_date;
            while d <= *max_date {
                let frac = min_date.days_between(&d) as f32 / total_days as f32;
                labels.push((frac, d.format_short()));
                d = d.add_days(1);
            }
        }
        TimeScale::Weeks => {
            // Start from first Monday on or after min_date
            let mut d = *min_date;
            let wd = d.weekday();
            if wd > 0 {
                d = d.add_days((7 - wd as i64) % 7);
            }
            while d <= *max_date {
                let frac = min_date.days_between(&d) as f32 / total_days as f32;
                labels.push((frac, d.format_short()));
                d = d.add_days(7);
            }
        }
        TimeScale::Months => {
            // First day of each month
            let mut y = min_date.year;
            let mut m = min_date.month;
            loop {
                let d = Date::new(y, m, 1);
                if d > *max_date {
                    break;
                }
                if d >= *min_date {
                    let frac = min_date.days_between(&d) as f32 / total_days as f32;
                    labels.push((frac, d.format_month_year()));
                }
                m += 1;
                if m > 12 {
                    m = 1;
                    y += 1;
                }
            }
        }
    }

    TimeGrid { scale, labels }
}

pub(super) fn format_duration_label(days: i64, scale: &TimeScale) -> String {
    match scale {
        TimeScale::Days => {
            if days == 1 {
                "1 day".to_string()
            } else {
                format!("{days} days")
            }
        }
        TimeScale::Weeks => {
            if days < 7 {
                format!("{days}d")
            } else if days % 7 == 0 {
                let w = days / 7;
                if w == 1 {
                    "1 wk".to_string()
                } else {
                    format!("{w} wks")
                }
            } else {
                format!("{days}d")
            }
        }
        TimeScale::Months => {
            if days < 30 {
                format!("{days}d")
            } else {
                let months = days / 30;
                if months == 1 {
                    "~1 mo".to_string()
                } else {
                    format!("~{months} mo")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_grid_days() {
        let min = Date::new(2024, 1, 1);
        let max = Date::new(2024, 1, 14);
        let grid = compute_time_grid(&min, &max, 13);
        assert!(matches!(grid.scale, TimeScale::Days));
        assert!(!grid.labels.is_empty());
    }

    #[test]
    fn test_time_grid_weeks() {
        let min = Date::new(2024, 1, 1);
        let max = Date::new(2024, 3, 1);
        let grid = compute_time_grid(&min, &max, 60);
        assert!(matches!(grid.scale, TimeScale::Weeks));
    }

    #[test]
    fn test_time_grid_months() {
        let min = Date::new(2024, 1, 1);
        let max = Date::new(2024, 12, 31);
        let grid = compute_time_grid(&min, &max, 365);
        assert!(matches!(grid.scale, TimeScale::Months));
    }
}
