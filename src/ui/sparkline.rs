//! Sparkline rendering utility
//!
//! Renders time-series data as a compact Unicode sparkline using block characters.

use crate::terminal::escape::{color, fg, RESET};

/// Unicode block characters for sparkline levels (8 levels)
const BLOCKS: &[char] = &[' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Fixed maximum for absolute scaling (10 tools = full height)
const SPARKLINE_MAX: u32 = 10;

/// Render a sparkline from binned counts
///
/// # Arguments
/// * `bins` - Vector of counts for each time bin
/// * `width` - Maximum width in characters
///
/// # Returns
/// A colored string representing the sparkline
pub fn render_sparkline(bins: &[u32], width: usize) -> String {
    if bins.is_empty() || width == 0 {
        return String::new();
    }

    // Check if there's any activity
    let has_activity = bins.iter().any(|&c| c > 0);
    if !has_activity {
        // No activity - show empty sparkline
        return format!("{}{}{}", fg(color::GRAY), " ".repeat(width.min(bins.len())), RESET);
    }

    // Build the sparkline string
    let mut result = String::new();
    result.push_str(&fg(color::ORANGE));

    for &count in bins.iter().take(width) {
        let level = if count == 0 {
            0
        } else {
            // Scale to 1-8 range using fixed max (absolute scale)
            let scaled = (count as f64 / SPARKLINE_MAX as f64 * 8.0).ceil() as usize;
            scaled.clamp(1, 8)
        };
        result.push(BLOCKS[level]);
    }

    result.push_str(RESET);
    result
}

/// Bin timestamps into fixed-width buckets
///
/// # Arguments
/// * `timestamps` - Unix timestamps of events
/// * `session_start` - Unix timestamp when session started
/// * `now` - Current unix timestamp
/// * `num_bins` - Number of bins to create
///
/// # Returns
/// Vector of counts for each time bin
pub fn bin_timestamps(
    timestamps: &[f64],
    session_start: f64,
    now: f64,
    num_bins: usize,
) -> Vec<u32> {
    if num_bins == 0 || now <= session_start {
        return vec![];
    }

    let duration = now - session_start;
    let bin_size = duration / num_bins as f64;

    let mut bins = vec![0u32; num_bins];

    for &ts in timestamps {
        if ts >= session_start && ts <= now {
            let bin_idx = ((ts - session_start) / bin_size) as usize;
            let bin_idx = bin_idx.min(num_bins - 1); // Clamp to last bin
            bins[bin_idx] += 1;
        }
    }

    bins
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_empty() {
        assert_eq!(render_sparkline(&[], 10), "");
        assert_eq!(render_sparkline(&[0, 0, 0], 10).contains(' '), true);
    }

    #[test]
    fn test_render_single_peak() {
        let bins = vec![0, 0, 5, 0, 0];
        let result = render_sparkline(&bins, 5);
        // 5 tools = half of max (10), so should be mid-height block
        assert!(result.contains('▄') || result.contains('▅'));
    }

    #[test]
    fn test_render_full_height() {
        let bins = vec![0, 0, 10, 0, 0];
        let result = render_sparkline(&bins, 5);
        // 10 tools = full height
        assert!(result.contains('█'));
    }

    #[test]
    fn test_bin_timestamps() {
        let start = 1000.0;
        let now = 2000.0;
        let timestamps = vec![1100.0, 1200.0, 1500.0, 1900.0];

        let bins = bin_timestamps(&timestamps, start, now, 10);
        assert_eq!(bins.len(), 10);
        assert_eq!(bins[1], 1); // 1100 -> bin 1
        assert_eq!(bins[2], 1); // 1200 -> bin 2
        assert_eq!(bins[5], 1); // 1500 -> bin 5
        assert_eq!(bins[9], 1); // 1900 -> bin 9
    }

    #[test]
    fn test_bin_timestamps_clustering() {
        let start = 0.0;
        let now = 100.0;
        let timestamps = vec![10.0, 11.0, 12.0, 90.0];

        let bins = bin_timestamps(&timestamps, start, now, 10);
        assert_eq!(bins[1], 3); // 10, 11, 12 all in bin 1
        assert_eq!(bins[9], 1); // 90 in last bin
    }
}
