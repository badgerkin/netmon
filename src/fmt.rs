use std::time::Duration;

/// Format a byte count human-readably, e.g. "1.4 GB".
pub fn bytes(n: u64) -> String {
    fmt_bytes(n as f64)
}

/// Format a byte/s rate human-readably, e.g. "12.3 MB/s".
pub fn rate(r: f64) -> String {
    format!("{}/s", fmt_bytes(r))
}

fn fmt_bytes(mut n: f64) -> String {
    for unit in ["B", "KB", "MB", "GB", "TB"] {
        if n < 1024.0 || unit == "TB" {
            return if unit == "B" {
                format!("{:.0} B", n)
            } else {
                format!("{:.1} {}", n, unit)
            };
        }
        n /= 1024.0;
    }
    unreachable!()
}

pub fn fmt_duration(d: Duration) -> String {
    let h = d.as_secs() / 3600;
    let m = d.as_secs() % 3600 / 60;
    let s = d.as_secs() % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}
