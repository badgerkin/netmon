//! Shared state: rates, history, totals, connections.

use std::time::Instant;

use crate::collect::{ifaces::Counters, Connection};

/// Number of rate samples to keep for the chart.
pub const HISTORY: usize = 60;

pub struct Sample {
    pub rx: f64, // bytes/s
    pub tx: f64, // bytes/s
}

pub struct NetState {
    /// (name, rx, tx) counters from the previous tick, for delta computation.
    pub(crate) prev: std::collections::HashMap<String, (u64, u64)>,
    pub history: std::collections::VecDeque<Sample>,
    pub rate_rx: f64,
    pub rate_tx: f64,
    pub peak_rx: f64,
    pub peak_tx: f64,
    pub min_rx: f64,
    pub min_tx: f64,
    pub totals_rx: u64,
    pub totals_tx: u64,
    /// Number of full samples since start (drives averages).
    pub samples: u64,
    /// Accumulated sampled time in seconds; averages = totals / elapsed.
    pub elapsed_secs: f64,
    /// Last sampling interval in seconds (for averages).
    pub(crate) dt: f64,
    pub conns: Vec<Connection>,
    pub started: Instant,
    pub(crate) last_tick: Option<Instant>,
}

impl NetState {
    pub fn new() -> Self {
        Self {
            prev: std::collections::HashMap::new(),
            history: std::collections::VecDeque::with_capacity(HISTORY),
            rate_rx: 0.0,
            rate_tx: 0.0,
            peak_rx: 0.0,
            peak_tx: 0.0,
            min_rx: f64::INFINITY,
            min_tx: f64::INFINITY,
            totals_rx: 0,
            totals_tx: 0,
            samples: 0,
            elapsed_secs: 0.0,
            dt: 1.0,
            conns: Vec::new(),
            started: Instant::now(),
            last_tick: None,
        }
    }

    /// Feed one counter sample; computes rates vs the previous sample,
    /// updating history / totals / peaks / mins. `dt` is the sampling
    /// interval in seconds.
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
            self.totals_rx += rx_d;
            self.totals_tx += tx_d;
            self.peak_rx = self.peak_rx.max(self.rate_rx);
            self.peak_tx = self.peak_tx.max(self.rate_tx);
            self.min_rx = self.min_rx.min(self.rate_rx);
            self.min_tx = self.min_tx.min(self.rate_tx);
            self.samples += 1;
            self.elapsed_secs += dt;
        }

        self.history.push_back(Sample {
            rx: self.rate_rx,
            tx: self.rate_tx,
        });
        while self.history.len() > HISTORY {
            self.history.pop_front();
        }
    }

    pub fn avg_rx(&self) -> f64 {
        if self.samples == 0 {
            0.0
        } else {
            self.totals_rx as f64 / self.elapsed_secs.max(0.01)
        }
    }

    pub fn avg_tx(&self) -> f64 {
        if self.samples == 0 {
            0.0
        } else {
            self.totals_tx as f64 / self.elapsed_secs.max(0.01)
        }
    }
}
