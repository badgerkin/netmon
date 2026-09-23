//! Shared state: interface rates + history, connections and their rate stats.

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use crate::collect::{ifaces::Counters, Connection};

/// Number of rate samples to keep for the chart.
pub const HISTORY: usize = 60;

pub struct Sample {
    pub rx: f64, // bytes/s
    pub tx: f64, // bytes/s
}

/// max / min / avg / cur of a byte rate (bytes/s).
pub struct Rates {
    pub max: f64,
    pub min: f64,
    pub avg: f64,
    pub cur: f64,
}

impl Rates {
    /// Rates over a series of samples (e.g. the chart window).
    pub fn of(series: &[f64]) -> Option<Rates> {
        let cur = *series.last()?;
        Some(Rates {
            max: series.iter().copied().fold(0.0, f64::max),
            min: series.iter().copied().fold(f64::INFINITY, f64::min),
            avg: series.iter().sum::<f64>() / series.len() as f64,
            cur,
        })
    }
}

/// Running rate stats for one direction, fed with byte deltas.
#[derive(Default)]
pub struct RateStats {
    max: f64,
    min: f64,
    cur: f64,
    bytes: u64,
    secs: f64,
    samples: u64,
}

impl RateStats {
    fn push(&mut self, delta: u64, dt: f64) {
        let r = delta as f64 / dt;
        self.min = if self.samples == 0 { r } else { self.min.min(r) };
        self.max = self.max.max(r);
        self.cur = r;
        self.bytes += delta;
        self.secs += dt;
        self.samples += 1;
    }

    /// `None` until two samples have been seen (no delta yet).
    pub fn rates(&self) -> Option<Rates> {
        (self.samples > 0).then(|| Rates {
            max: self.max,
            min: self.min,
            avg: self.bytes as f64 / self.secs,
            cur: self.cur,
        })
    }
}

#[derive(Default)]
pub struct ConnStats {
    pub rx: RateStats,
    pub tx: RateStats,
    /// Lifetime counters at the previous `ss` sample, for deltas.
    prev: (Option<u64>, Option<u64>),
}

/// Identity of a connection across `ss` samples.
pub type ConnKey = (String, String, String); // (proto, local, peer)

pub fn conn_key(c: &Connection) -> ConnKey {
    (c.proto.clone(), c.local.clone(), c.peer.clone())
}

pub struct NetState {
    /// (name, rx, tx) counters from the previous tick, for delta computation.
    pub(crate) prev: HashMap<String, (u64, u64)>,
    pub history: VecDeque<Sample>,
    pub rate_rx: f64,
    pub rate_tx: f64,
    pub peak_rx: f64,
    pub peak_tx: f64,
    /// Last sampling interval in seconds (chart time axis).
    pub(crate) dt: f64,
    pub conns: Vec<Connection>,
    /// Per-connection rate stats, keyed by `conn_key`.
    pub conn_stats: HashMap<ConnKey, ConnStats>,
    /// Rate stats of the summed traffic of all connections.
    pub all_conns: ConnStats,
    last_conns: Option<Instant>,
    pub started: Instant,
    pub(crate) last_tick: Option<Instant>,
}

impl NetState {
    pub fn new() -> Self {
        Self {
            prev: HashMap::new(),
            history: VecDeque::with_capacity(HISTORY),
            rate_rx: 0.0,
            rate_tx: 0.0,
            peak_rx: 0.0,
            peak_tx: 0.0,
            dt: 1.0,
            conns: Vec::new(),
            conn_stats: HashMap::new(),
            all_conns: ConnStats::default(),
            last_conns: None,
            started: Instant::now(),
            last_tick: None,
        }
    }

    /// Feed one counter sample; computes rates vs the previous sample,
    /// updating history / peaks. `dt` is the sampling interval in seconds.
    pub fn tick(&mut self, cur: Counters, dt: f64) {
        self.dt = dt;
        let mut had_prev = false;
        let mut rx_d: u64 = 0;
        let mut tx_d: u64 = 0;
        for (name, rx, tx) in &cur {
            if let Some((pr, pt)) = self.prev.get(name) {
                had_prev = true;
                // Counter resets (or interface reappearances) make the delta
                // uncomputable; saturate to zero rather than go negative.
                rx_d += rx.saturating_sub(*pr);
                tx_d += tx.saturating_sub(*pt);
            }
            self.prev.insert(name.clone(), (*rx, *tx));
        }

        if had_prev {
            self.rate_rx = rx_d as f64 / dt;
            self.rate_tx = tx_d as f64 / dt;
            self.peak_rx = self.peak_rx.max(self.rate_rx);
            self.peak_tx = self.peak_tx.max(self.rate_tx);
        }

        self.history.push_back(Sample {
            rx: self.rate_rx,
            tx: self.rate_tx,
        });
        while self.history.len() > HISTORY {
            self.history.pop_front();
        }
    }

    /// Replace the connection list, updating per-connection and aggregate
    /// rate stats from the change in each socket's lifetime byte counters.
    /// Connections seen for the first time only set a baseline; stats of
    /// connections that disappeared are dropped.
    pub fn update_conns(&mut self, conns: Vec<Connection>, now: Instant) {
        let dt = self
            .last_conns
            .map(|t| now.duration_since(t).as_secs_f64())
            .filter(|dt| *dt > 0.0);
        self.last_conns = Some(now);

        let mut stats = HashMap::with_capacity(conns.len());
        let (mut all_rx, mut all_tx) = (0u64, 0u64);
        for c in &conns {
            let key = conn_key(c);
            let mut st = self.conn_stats.remove(&key).unwrap_or_default();
            if let Some(dt) = dt {
                if let (Some(cur), Some(prev)) = (c.bytes_in, st.prev.0) {
                    let d = cur.saturating_sub(prev);
                    st.rx.push(d, dt);
                    all_rx += d;
                }
                if let (Some(cur), Some(prev)) = (c.bytes_out, st.prev.1) {
                    let d = cur.saturating_sub(prev);
                    st.tx.push(d, dt);
                    all_tx += d;
                }
            }
            st.prev = (c.bytes_in, c.bytes_out);
            stats.insert(key, st);
        }
        if let Some(dt) = dt {
            self.all_conns.rx.push(all_rx, dt);
            self.all_conns.tx.push(all_tx, dt);
        }
        self.conn_stats = stats;
        self.conns = conns;
    }
}
