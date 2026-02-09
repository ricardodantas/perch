//! Time parsing utilities for scheduled posts

use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, Local, NaiveDateTime, NaiveTime, TimeZone, Utc};

/// Parse a schedule time string into a `DateTime`<Utc>
///
/// Supports formats:
/// - Relative: "in 5m", "in 2h", "in 1d", "in 30 minutes", "in 2 hours"
/// - Absolute time today: "15:00", "3pm", "15:30"
/// - Absolute datetime: "YYYY-MM-DD 15:00", "YYYY-MM-DDT15:00:00"
/// - ISO 8601: "YYYY-MM-DDT15:00:00Z", "YYYY-MM-DDT15:00:00+01:00"
pub fn parse_schedule_time(input: &str) -> Result<DateTime<Utc>> {
    let input = input.trim().to_lowercase();

    // Try relative time first
    if let Some(rest) = input.strip_prefix("in ") {
        return parse_relative_time(rest);
    }

    // Try ISO 8601 with timezone
    if let Ok(dt) = DateTime::parse_from_rfc3339(&input) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Try ISO 8601 variants
    if let Ok(dt) = DateTime::parse_from_str(&input, "%Y-%m-%dT%H:%M:%S%z") {
        return Ok(dt.with_timezone(&Utc));
    }

    // Try datetime without timezone (assume local)
    if let Ok(naive) = NaiveDateTime::parse_from_str(&input, "%Y-%m-%d %H:%M:%S") {
        return local_to_utc(naive);
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(&input, "%Y-%m-%d %H:%M") {
        return local_to_utc(naive);
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(&input, "%Y-%m-%dT%H:%M:%S") {
        return local_to_utc(naive);
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(&input, "%Y-%m-%dT%H:%M") {
        return local_to_utc(naive);
    }

    // Try time only (assume today, or tomorrow if time has passed)
    if let Some(time) = parse_time_only(&input) {
        let today = Local::now().date_naive();
        let naive_dt = today.and_time(time);
        let local_dt = Local.from_local_datetime(&naive_dt).single();

        if let Some(dt) = local_dt {
            // If time has passed, schedule for tomorrow
            if dt <= Local::now() {
                let tomorrow = today + Duration::days(1);
                let naive_dt = tomorrow.and_time(time);
                if let Some(dt) = Local.from_local_datetime(&naive_dt).single() {
                    return Ok(dt.with_timezone(&Utc));
                }
            }
            return Ok(dt.with_timezone(&Utc));
        }
    }

    Err(anyhow!(
        "Could not parse schedule time: '{}'\n\
         Supported formats:\n  \
         - Relative: 'in 5m', 'in 2h', 'in 1d', 'in 30 minutes'\n  \
         - Time today: '15:00', '3pm', '15:30'\n  \
         - Date+time: 'YYYY-MM-DD 15:00'",
        input
    ))
}

/// Parse relative time like "5m", "2h", "1d", "30 minutes", "2 hours"
fn parse_relative_time(input: &str) -> Result<DateTime<Utc>> {
    let input = input.trim();

    // Try short format: 5m, 2h, 1d
    if let Some(duration) = parse_short_duration(input) {
        return Ok(Utc::now() + duration);
    }

    // Try long format: "30 minutes", "2 hours", "1 day"
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() >= 2
        && let Ok(amount) = parts[0].parse::<i64>()
    {
        let unit = parts[1].trim_end_matches('s'); // Remove trailing 's'
        let duration = match unit {
            "second" | "sec" => Duration::seconds(amount),
            "minute" | "min" => Duration::minutes(amount),
            "hour" | "hr" => Duration::hours(amount),
            "day" => Duration::days(amount),
            "week" => Duration::weeks(amount),
            _ => return Err(anyhow!("Unknown time unit: {}", parts[1])),
        };
        return Ok(Utc::now() + duration);
    }

    Err(anyhow!(
        "Could not parse relative time: '{}'\n\
         Examples: '5m', '2h', '1d', '30 minutes', '2 hours'",
        input
    ))
}

/// Parse short duration format: 5m, 2h, 1d, 30s
fn parse_short_duration(input: &str) -> Option<Duration> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    let (num_str, unit) = input.split_at(input.len() - 1);
    let amount: i64 = num_str.parse().ok()?;

    match unit {
        "s" => Some(Duration::seconds(amount)),
        "m" => Some(Duration::minutes(amount)),
        "h" => Some(Duration::hours(amount)),
        "d" => Some(Duration::days(amount)),
        "w" => Some(Duration::weeks(amount)),
        _ => None,
    }
}

/// Parse time-only string like "15:00", "3pm", "15:30"
fn parse_time_only(input: &str) -> Option<NaiveTime> {
    // Try 24-hour format
    if let Ok(time) = NaiveTime::parse_from_str(input, "%H:%M:%S") {
        return Some(time);
    }
    if let Ok(time) = NaiveTime::parse_from_str(input, "%H:%M") {
        return Some(time);
    }

    // Try 12-hour format with am/pm
    let input = input.replace(' ', "");
    if input.ends_with("am") || input.ends_with("pm") {
        let is_pm = input.ends_with("pm");
        let time_part = input.trim_end_matches("am").trim_end_matches("pm");

        // Parse hour (and optional minutes)
        let parts: Vec<&str> = time_part.split(':').collect();
        if let Ok(mut hour) = parts[0].parse::<u32>() {
            let minute = parts.get(1).and_then(|m| m.parse().ok()).unwrap_or(0);

            // Convert to 24-hour
            if is_pm && hour != 12 {
                hour += 12;
            } else if !is_pm && hour == 12 {
                hour = 0;
            }

            return NaiveTime::from_hms_opt(hour, minute, 0);
        }
    }

    None
}

/// Convert naive local datetime to UTC
fn local_to_utc(naive: NaiveDateTime) -> Result<DateTime<Utc>> {
    Local
        .from_local_datetime(&naive)
        .single()
        .map(|dt| dt.with_timezone(&Utc))
        .ok_or_else(|| anyhow!("Ambiguous or invalid local time"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_relative_short() {
        let now = Utc::now();
        let result = parse_schedule_time("in 5m").unwrap();
        let diff = result - now;
        assert!(diff.num_minutes() >= 4 && diff.num_minutes() <= 6);
    }

    #[test]
    fn test_relative_long() {
        let now = Utc::now();
        let result = parse_schedule_time("in 2 hours").unwrap();
        let diff = result - now;
        assert!(diff.num_hours() >= 1 && diff.num_hours() <= 3);
    }

    #[test]
    fn test_datetime() {
        let result = parse_schedule_time("2030-01-15 14:30").unwrap();
        assert_eq!(result.year(), 2030);
        assert_eq!(result.month(), 1);
        assert_eq!(result.day(), 15);
    }
}
