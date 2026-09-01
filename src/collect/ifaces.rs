//! Per-interface byte counters from /proc/net/dev.

pub type Counters = Vec<(String, u64, u64)>; // (name, rx_bytes, tx_bytes)

/// Read /proc/net/dev and return cumulative (name, rx, tx) for each interface.
pub fn sample() -> Counters {
    let s = match std::fs::read_to_string("/proc/net/dev") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    // First two lines are the "Inter|:" header; data lines look like:
    //     eth0:  12345  0 0 0 0 0  0 0  2345 0 0 0 0 0 0 0
    // rx bytes is the 1st number, tx bytes the 9th (8 rx fields, then tx).
    let mut out = Vec::new();
    for line in s.lines().skip(2) {
        let Some((iface, rest)) = line.split_once(':') else {
            continue;
        };
        let iface = iface.trim();
        if iface.is_empty() {
            continue;
        }
        let nums: Vec<&str> = rest.split_whitespace().collect();
        if nums.len() >= 9 {
            let rx = nums[0].parse::<u64>().unwrap_or(0);
            let tx = nums[8].parse::<u64>().unwrap_or(0);
            out.push((iface.to_string(), rx, tx));
        }
    }
    out
}
