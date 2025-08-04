pub fn format_size<T: Into<f64>>(bytes: T) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes.into();
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if size < 10.0 && unit > 0 {
        format!("{:.1} {}", size, UNITS[unit])
    } else {
        format!("{:.0} {}", size, UNITS[unit])
    }
}
