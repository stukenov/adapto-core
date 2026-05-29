use chrono::{DateTime, Datelike, Duration as ChDuration, TimeZone, Utc, Weekday};
use chrono_tz::Tz;
use std::time::Duration;

/// When a job should run. Times are interpreted in the scheduler timezone.
///
/// The [`Schedule::Cron`] variant uses the `cron` crate's **6-field** syntax
/// (`sec min hour day-of-month month day-of-week`).
#[derive(Clone, Debug)]
pub enum Schedule {
    /// Fire every fixed duration after the previous run.
    Interval(Duration),
    /// Fire daily at the given local time.
    DailyAt { hour: u8, min: u8 },
    /// Fire weekly on the given weekday at the given local time.
    WeeklyOn { weekday: Weekday, hour: u8, min: u8 },
    /// Fire monthly on the given day (1..=31, clamped to month end) at the given local time.
    MonthlyOn { day: u8, hour: u8, min: u8 },
    /// 6-field cron expression (`sec min hour dom month dow`).
    Cron(String),
}

impl Schedule {
    /// Next fire time strictly after `now`, computed in `now`'s timezone, returned as UTC.
    /// Returns `Err` for an unparseable cron expression or an impossible local time.
    pub fn next_after(&self, now: DateTime<Tz>) -> Result<DateTime<Utc>, String> {
        let tz = now.timezone();
        match self {
            Schedule::Interval(d) => {
                let dd = ChDuration::from_std(*d).map_err(|e| e.to_string())?;
                Ok(now.with_timezone(&Utc) + dd)
            }
            Schedule::DailyAt { hour, min } => {
                let mut cand = local_at(tz, now.year(), now.month(), now.day(), *hour, *min)?;
                if cand <= now {
                    let d = (cand + ChDuration::days(1)).date_naive();
                    cand = local_at(tz, d.year(), d.month(), d.day(), *hour, *min)?;
                }
                Ok(cand.with_timezone(&Utc))
            }
            Schedule::WeeklyOn { weekday, hour, min } => {
                let mut cand = local_at(tz, now.year(), now.month(), now.day(), *hour, *min)?;
                while cand <= now || cand.weekday() != *weekday {
                    let d = (cand + ChDuration::days(1)).date_naive();
                    cand = local_at(tz, d.year(), d.month(), d.day(), *hour, *min)?;
                }
                Ok(cand.with_timezone(&Utc))
            }
            Schedule::MonthlyOn { day, hour, min } => {
                let mut y = now.year();
                let mut m = now.month();
                loop {
                    let d = clamp_day(y, m, *day);
                    let cand = local_at(tz, y, m, d, *hour, *min)?;
                    if cand > now {
                        return Ok(cand.with_timezone(&Utc));
                    }
                    if m == 12 {
                        y += 1;
                        m = 1;
                    } else {
                        m += 1;
                    }
                }
            }
            Schedule::Cron(expr) => {
                use std::str::FromStr;
                let sched = cron::Schedule::from_str(expr).map_err(|e| e.to_string())?;
                sched
                    .after(&now)
                    .next()
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok_or_else(|| "cron yielded no next time".to_string())
            }
        }
    }
}

fn clamp_day(year: i32, month: u32, day: u8) -> u32 {
    let last = last_day_of_month(year, month);
    (day as u32).clamp(1, last)
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    let (ny, nm) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    let first_next = chrono::NaiveDate::from_ymd_opt(ny, nm, 1).unwrap();
    (first_next - ChDuration::days(1)).day()
}

/// Build a tz-aware datetime, picking the earliest instant on DST ambiguity,
/// and bumping forward across a DST gap.
fn local_at(tz: Tz, y: i32, mo: u32, d: u32, h: u8, mi: u8) -> Result<DateTime<Tz>, String> {
    let naive = chrono::NaiveDate::from_ymd_opt(y, mo, d)
        .and_then(|nd| nd.and_hms_opt(h as u32, mi as u32, 0))
        .ok_or_else(|| format!("invalid local time {y}-{mo}-{d} {h}:{mi}"))?;
    match tz.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => Ok(dt),
        chrono::LocalResult::Ambiguous(a, _) => Ok(a),
        chrono::LocalResult::None => {
            let bumped = naive + ChDuration::hours(1);
            tz.from_local_datetime(&bumped)
                .single()
                .ok_or_else(|| "unresolvable local time across DST".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn almaty(y: i32, m: u32, d: u32, h: u32, mi: u32) -> DateTime<Tz> {
        chrono_tz::Asia::Almaty
            .with_ymd_and_hms(y, m, d, h, mi, 0)
            .single()
            .unwrap()
    }

    #[test]
    fn daily_at_same_day_future() {
        let now = almaty(2026, 5, 29, 5, 0);
        let s = Schedule::DailyAt { hour: 6, min: 30 };
        let next = s.next_after(now).unwrap();
        // 06:30 Almaty (UTC+5) == 01:30 UTC same day
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 5, 29, 1, 30, 0).unwrap());
    }

    #[test]
    fn daily_at_already_passed_rolls_tomorrow() {
        let now = almaty(2026, 5, 29, 7, 0);
        let s = Schedule::DailyAt { hour: 6, min: 30 };
        let next = s.next_after(now).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 5, 30, 1, 30, 0).unwrap());
    }

    #[test]
    fn monthly_clamps_day_31_to_month_end() {
        // April has 30 days; asking for day 31 -> April 30.
        let now = almaty(2026, 4, 1, 0, 0);
        let s = Schedule::MonthlyOn { day: 31, hour: 0, min: 0 };
        let next = s.next_after(now).unwrap();
        // 00:00 Almaty Apr 30 == 19:00 UTC Apr 29
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 4, 29, 19, 0, 0).unwrap());
    }

    #[test]
    fn monthly_rolls_to_next_month_when_passed() {
        let now = almaty(2026, 5, 10, 0, 0);
        let s = Schedule::MonthlyOn { day: 2, hour: 3, min: 0 };
        let next = s.next_after(now).unwrap();
        // June 2 03:00 Almaty == June 1 22:00 UTC
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 6, 1, 22, 0, 0).unwrap());
    }

    #[test]
    fn interval_adds_duration() {
        let now = almaty(2026, 5, 29, 5, 0);
        let s = Schedule::Interval(Duration::from_secs(900));
        let next = s.next_after(now).unwrap();
        assert_eq!(next, now.with_timezone(&Utc) + ChDuration::seconds(900));
    }

    #[test]
    fn cron_daily_six_am() {
        let now = almaty(2026, 5, 29, 0, 0);
        let s = Schedule::Cron("0 0 6 * * *".to_string()); // sec min hour dom mon dow
        let next = s.next_after(now).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 5, 29, 1, 0, 0).unwrap());
    }

    #[test]
    fn cron_invalid_errs() {
        let now = almaty(2026, 5, 29, 0, 0);
        let s = Schedule::Cron("not a cron".to_string());
        assert!(s.next_after(now).is_err());
    }
}
