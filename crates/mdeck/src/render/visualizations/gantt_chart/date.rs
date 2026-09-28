//! Calendar dates and task durations.

// ─── Date Arithmetic ────────────────────────────────────────────────────────

/// Simple date representation (year, month 1-based, day 1-based).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Date {
    pub(super) year: i32,
    pub(super) month: u32,
    pub(super) day: u32,
}

impl Date {
    pub(super) fn new(year: i32, month: u32, day: u32) -> Self {
        Self { year, month, day }
    }

    pub(super) fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return None;
        }
        let year = parts[0].parse().ok()?;
        let month = parts[1].parse().ok()?;
        let day = parts[2].parse().ok()?;
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return None;
        }
        Some(Self { year, month, day })
    }

    #[cfg(test)]
    pub(super) fn format(&self) -> String {
        format!("{}-{:02}-{:02}", self.year, self.month, self.day)
    }

    pub(super) fn format_short(self) -> String {
        static MONTHS: [&str; 12] = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        let m = self.month.saturating_sub(1).min(11) as usize;
        format!("{} {}", MONTHS[m], self.day)
    }

    pub(super) fn format_month_year(self) -> String {
        static MONTHS: [&str; 12] = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        let m = self.month.saturating_sub(1).min(11) as usize;
        format!("{} {}", MONTHS[m], self.year)
    }

    /// Convert to a day number (days since an epoch). Used for arithmetic.
    pub(super) fn to_days(self) -> i64 {
        // Algorithm from https://en.wikipedia.org/wiki/Julian_day
        let y = self.year as i64;
        let m = self.month as i64;
        let d = self.day as i64;
        let a = (14 - m) / 12;
        let yy = y + 4800 - a;
        let mm = m + 12 * a - 3;
        d + (153 * mm + 2) / 5 + 365 * yy + yy / 4 - yy / 100 + yy / 400 - 32045
    }

    pub(super) fn from_days(jdn: i64) -> Self {
        // Inverse of to_days
        let a = jdn + 32044;
        let b = (4 * a + 3) / 146097;
        let c = a - (146097 * b) / 4;
        let d = (4 * c + 3) / 1461;
        let e = c - (1461 * d) / 4;
        let m = (5 * e + 2) / 153;
        let day = (e - (153 * m + 2) / 5 + 1) as u32;
        let month = (m + 3 - 12 * (m / 10)) as u32;
        let year = (100 * b + d - 4800 + m / 10) as i32;
        Self { year, month, day }
    }

    pub(super) fn add_days(self, n: i64) -> Self {
        Self::from_days(self.to_days() + n)
    }

    pub(super) fn add_workdays(self, n: i64) -> Self {
        let mut current = self.to_days();
        let dir: i64 = if n >= 0 { 1 } else { -1 };
        let mut remaining = n.unsigned_abs();
        while remaining > 0 {
            current += dir;
            if Self::from_days(current).weekday() < 5 {
                // Mon-Fri
                remaining -= 1;
            }
        }
        Self::from_days(current)
    }

    /// 0=Mon, 1=Tue, ..., 6=Sun
    pub(super) fn weekday(self) -> u32 {
        let jdn = self.to_days();
        ((jdn % 7) as u32 + 7) % 7 // Adjusted so Monday = 0
    }

    pub(super) fn days_between(self, other: &Date) -> i64 {
        other.to_days() - self.to_days()
    }
}

// ─── Duration Parsing ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub(super) enum Duration {
    Days(i64),
    WorkDays(i64),
}

pub(super) fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    if let Some(n) = s.strip_suffix("wd") {
        return n.trim().parse().ok().map(Duration::WorkDays);
    }
    if let Some(n) = s.strip_suffix('d') {
        return n.trim().parse().ok().map(Duration::Days);
    }
    if let Some(n) = s.strip_suffix('w') {
        return n.trim().parse::<i64>().ok().map(|w| Duration::Days(w * 7));
    }
    if let Some(n) = s.strip_suffix('m') {
        return n.trim().parse::<i64>().ok().map(|m| Duration::Days(m * 30));
    }
    None
}

pub(super) fn apply_duration(start: &Date, dur: Duration) -> Date {
    match dur {
        Duration::Days(n) => start.add_days(n),
        Duration::WorkDays(n) => start.add_workdays(n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_parse() {
        let d = Date::parse("2024-01-15").unwrap();
        assert_eq!(d.year, 2024);
        assert_eq!(d.month, 1);
        assert_eq!(d.day, 15);
    }

    #[test]
    fn test_date_invalid() {
        assert!(Date::parse("not-a-date").is_none());
        assert!(Date::parse("2024-13-01").is_none());
        assert!(Date::parse("2024-01-32").is_none());
    }

    #[test]
    fn test_date_arithmetic() {
        let d = Date::new(2024, 1, 15);
        let d2 = d.add_days(10);
        assert_eq!(d2.format(), "2024-01-25");

        let d3 = d.add_days(20);
        assert_eq!(d3.format(), "2024-02-04");
    }

    #[test]
    fn test_date_days_between() {
        let d1 = Date::new(2024, 1, 1);
        let d2 = Date::new(2024, 1, 31);
        assert_eq!(d1.days_between(&d2), 30);
    }

    #[test]
    fn test_date_roundtrip() {
        let d = Date::new(2024, 6, 15);
        let days = d.to_days();
        let d2 = Date::from_days(days);
        assert_eq!(d, d2);
    }

    #[test]
    fn test_date_weekday() {
        // 2024-01-15 is a Monday
        let d = Date::new(2024, 1, 15);
        assert_eq!(d.weekday(), 0); // Monday
    }

    #[test]
    fn test_date_workdays() {
        // From Monday, add 5 working days = next Monday
        let d = Date::new(2024, 1, 15); // Monday
        let d2 = d.add_workdays(5);
        assert_eq!(d2.format(), "2024-01-22"); // Next Monday
    }

    #[test]
    fn test_parse_duration() {
        assert!(matches!(parse_duration("10d"), Some(Duration::Days(10))));
        assert!(matches!(parse_duration("5wd"), Some(Duration::WorkDays(5))));
        assert!(matches!(parse_duration("2w"), Some(Duration::Days(14))));
        assert!(matches!(parse_duration("3m"), Some(Duration::Days(90))));
        assert!(parse_duration("foo").is_none());
    }

    #[test]
    fn test_format_short() {
        let d = Date::new(2024, 3, 15);
        assert_eq!(d.format_short(), "Mar 15");
    }

    #[test]
    fn test_format_month_year() {
        let d = Date::new(2024, 3, 1);
        assert_eq!(d.format_month_year(), "Mar 2024");
    }

    #[test]
    fn test_workdays_backwards() {
        // Monday 2024-01-08 minus 1 workday is Friday 2024-01-05
        let d = Date::new(2024, 1, 8);
        assert_eq!(d.add_workdays(-1), Date::new(2024, 1, 5));
        assert_eq!(d.add_workdays(0), d);
    }
}
