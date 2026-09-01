//! Active TCP/UDP connections from `ss -Htunap`.

use std::process::Command;

pub struct Connection {
    pub proto: String, // "TCP" | "UDP" | other
    pub state: String,
    pub local: String,
    pub peer: String,
    /// Bytes currently in the kernel send/receive queues for this socket.
    pub send_q: u64,
    pub recv_q: u64,
    /// Owening program name (from ss -p) and pid, if available.
    pub name: String,
    pub pid: Option<u32>,
}

impl Connection {
    pub fn queued_bytes(&self) -> u64 {
        self.send_q.saturating_add(self.recv_q)
    }
}

pub fn sample() -> Vec<Connection> {
    // `-H` (no header) is newer; without it, `parse` skips the header line.
    let out = match Command::new("ss").args(["-tunap"]).output() {
        Ok(o) if o.status.success() => o.stdout,
        _ => return Vec::new(),
    };
    parse(&out)
}

fn parse(data: &[u8]) -> Vec<Connection> {
    let s = String::from_utf8_lossy(data);
    let mut conns: Vec<Connection> = Vec::new();
    for line in s.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Process info that ss wraps onto a continuation line: attach to the
        // previous connection.
        if trimmed.starts_with('(') || trimmed.starts_with("users:") {
            if let Some(last) = conns.last_mut() {
                apply_users(last, trimmed);
            }
            continue;
        }
        // Normal line: tab-separated; fall back to whitespace for old ss.
        let fields: Vec<&str> = if line.contains('\t') {
            line.split('\t').collect()
        } else {
            line.split_whitespace().collect()
        };
        if fields[0].trim() == "Netid" {
            continue; // header row (when `-H` is unavailable)
        }
        // [netid, state, recv-q, send-q, local, peer, users?]
        if fields.len() < 6 {
            continue;
        }
        let netid = fields[0].trim();
        let proto = match netid {
            "t" | "t-*" | "tw" => "TCP".into(),
            "u" | "u-*" | "ud" => "UDP".into(),
            other => other.to_uppercase(),
        };
        let mut c = Connection {
            proto,
            state: fields[1].trim().to_string(),
            local: fields[4].trim().to_string(),
            peer: fields[5].trim().to_string(),
            send_q: fields[3].trim().parse::<u64>().unwrap_or(0),
            recv_q: fields[2].trim().parse::<u64>().unwrap_or(0),
            name: String::new(),
            pid: None,
        };
        if let Some(users) = fields.get(6) {
            apply_users(&mut c, users);
        }
        conns.push(c);
    }
    conns
}

/// Extract program name ("users:("name",pid=...,fd=...)") into the connection.
fn apply_users(c: &mut Connection, s: &str) {
    if s.contains('"') {
        if let Some(open) = s.find('"') {
            if let Some(close) = s[open + 1..].find('"') {
                c.name = s[open + 1..open + 1 + close].to_string();
            }
        }
    }
    if let Some(i) = s.find("pid=") {
        let digits = s[i + 4..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>();
        c.pid = digits.parse::<u32>().ok();
    }
}
