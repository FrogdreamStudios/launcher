//! File size formatting utilities.
//!
//! Convert byte sizes into human-readable format with appropriate units.

/// Formats a byte size into a human-readable string.
///
/// Converts raw byte values into formatted strings with appropriate
/// units (B, KB, MB, GB, TB). Shows one decimal place for small values
/// and whole numbers for larger values.
/// #### Examples
/// ```
/// assert_eq!(format_size(1024), "1 KB");
/// assert_eq!(format_size(1536), "1.5 KB");
/// assert_eq!(format_size(1048576), "1 MB");
/// ```
pub fn format_size<T: Into<f64>>(bytes: T) -> String {
    // Array of size units from smallest to largest
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes.into();
    let mut unit = 0;

    // Keep dividing by 1024 until we find the right unit
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    // Show decimal for small values, whole numbers for large values
    if size < 10.0 && unit > 0 {
        format!("{:.1} {}", size, UNITS[unit]) // One decimal place
    } else {
        format!("{:.0} {}", size, UNITS[unit]) // No decimal places
    }
}
