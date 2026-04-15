//! Optional UTC deploy windows on environments (`deploy_schedule_json`).

use chrono::{Datelike, Timelike, Utc, Weekday};

/// Returns true if deploy is allowed now. `None`/empty JSON means no restriction.
pub fn deploy_schedule_allows_now(schedule_json: Option<&str>) -> Result<bool, &'static str> {
    let Some(raw) = schedule_json.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(true);
    };
    let v: serde_json::Value =
        serde_json::from_str(raw).map_err(|_| "invalid deploy_schedule_json")?;
    let utc_start = v
        .get("utc_start_hour")
        .and_then(|x| x.as_u64())
        .unwrap_or(0) as u32;
    let utc_end = v
        .get("utc_end_hour")
        .and_then(|x| x.as_u64())
        .unwrap_or(24) as u32;
    let weekdays_only = v
        .get("weekdays_only")
        .and_then(|x| x.as_bool())
        .unwrap_or(false);

    let now = Utc::now();
    if weekdays_only {
        let wd = now.weekday();
        if matches!(wd, Weekday::Sat | Weekday::Sun) {
            return Ok(false);
        }
    }
    let hour = now.hour();
    let ok = if utc_start < utc_end {
        hour >= utc_start && hour < utc_end
    } else if utc_start > utc_end {
        // overnight window, e.g. 22–06
        hour >= utc_start || hour < utc_end
    } else {
        hour == utc_start
    };
    Ok(ok)
}
