use std::time::{SystemTime, UNIX_EPOCH};

/// Format duration as compact text (e.g., "1m", "2h3m").
pub(crate) fn format_duration_compact(secs: u64) -> String {
    if secs >= 3600 {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        if m > 0 {
            format!("{}h{}m", h, m)
        } else {
            format!("{}h", h)
        }
    } else {
        let m = secs / 60;
        format!("{}m", m)
    }
}

/// Format elapsed time since a Unix timestamp, matching the Stats widget.
pub(crate) fn format_elapsed_age(timestamp: Option<f64>) -> Option<String> {
    let since = timestamp?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    let secs = (now - since).max(0.0) as u64;
    if secs >= 60 {
        Some(format!("{} ago", format_duration_compact(secs)))
    } else {
        Some("just now".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elapsed_age_uses_just_now_under_a_minute() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        assert_eq!(
            format_elapsed_age(Some(now - 30.0)).as_deref(),
            Some("just now")
        );
    }

    #[test]
    fn elapsed_age_matches_stats_compact_units() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        assert_eq!(
            format_elapsed_age(Some(now - 60.0)).as_deref(),
            Some("1m ago")
        );
        assert_eq!(
            format_elapsed_age(Some(now - 3660.0)).as_deref(),
            Some("1h1m ago")
        );
    }
}
