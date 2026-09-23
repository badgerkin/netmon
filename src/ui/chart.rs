//! Live in/out throughput chart, styled after a Grafana time-series panel:
//! faint gridlines, a crisp braille line over a translucent area fill,
//! nice y-ticks, wall-clock x-ticks, and a legend with last/mean/max.
//!
//! Rendered straight into the buffer (instead of ratatui's `Chart`) so the
//! fill can be drawn as a cell background underneath the line.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType},
    Frame,
};

use crate::fmt;
use crate::model::{NetState, HISTORY};

// Grafana's classic palette, plus ~25% "opacity" fills against a dark bg.
const IN: Color = Color::Rgb(115, 191, 105);
const OUT: Color = Color::Rgb(87, 148, 242);
const IN_FILL: Color = Color::Rgb(29, 48, 26);
const OUT_FILL: Color = Color::Rgb(22, 37, 61);
const BOTH_FILL: Color = Color::Rgb(34, 56, 58);
const GRID: Color = Color::Rgb(50, 50, 50);
const AXIS: Color = Color::Rgb(140, 140, 140);
const BORDER: Color = Color::Rgb(70, 70, 70);

pub fn draw(frame: &mut Frame, area: Rect, state: &NetState) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
        .title(Line::from(" Throughput ").bold().fg(Color::White));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    // Need room for at least a 2-row plot + x labels + legend.
    if inner.height < 4 || inner.width < 30 {
        return;
    }

    let rx: Vec<f64> = state.history.iter().map(|s| s.rx).collect();
    let tx: Vec<f64> = state.history.iter().map(|s| s.tx).collect();
    let dt = state.dt.max(1e-3);
    let span = (HISTORY - 1) as f64 * dt;

    // Y scale: autoscale to what's visible (like Grafana), on nice binary steps.
    let plot_h = inner.height - 2;
    let vis_max = rx.iter().chain(&tx).fold(0.0f64, |a, &b| a.max(b));
    let divisions = (plot_h / 3).clamp(2, 5) as f64;
    let step = nice_step(vis_max.max(1024.0) * 1.05 / divisions);
    let top = ((vis_max * 1.05) / step).ceil().max(1.0) * step;
    let n_ticks = (top / step).round() as usize;
    let y_labels: Vec<String> = (0..=n_ticks).map(|k| tick_label(step * k as f64)).collect();
    let gutter = y_labels.iter().map(|s| s.len()).max().unwrap_or(0) as u16 + 1;

    let plot = Rect {
        x: inner.x + gutter,
        y: inner.y,
        width: inner.width - gutter,
        height: plot_h,
    };
    let (w, h) = (plot.width as usize, plot.height as usize);
    let sub_h = (h * 4) as f64 - 1.0;
    let to_sub = |v: f64| ((1.0 - (v / top).clamp(0.0, 1.0)) * sub_h).round() as usize;

    // Sample each series at every braille sub-column (2 per cell); newest
    // sample sits on the right edge, missing history leaves the left empty.
    let px_n = w * 2;
    let sample = |series: &[f64]| -> Vec<Option<usize>> {
        (0..px_n)
            .map(|px| {
                let age = span * (1.0 - px as f64 / (px_n - 1) as f64);
                value_at(series, age / dt).map(to_sub)
            })
            .collect()
    };
    let in_sub = sample(&rx);
    let out_sub = sample(&tx);
    let in_dots = braille(&in_sub, w, h);
    let out_dots = braille(&out_sub, w, h);

    // Gridlines: one row per y tick, one column per x tick.
    let tick_rows: Vec<usize> = (0..=n_ticks).map(|k| to_sub(step * k as f64) / 4).collect();
    let x_ticks = time_ticks(span, w);

    let buf = frame.buffer_mut();
    for r in 0..h {
        for c in 0..w {
            let hgrid = tick_rows.contains(&r);
            let vgrid = x_ticks.iter().any(|(col, _)| *col == c);
            let grid = match (hgrid, vgrid) {
                (true, true) => "┼",
                (true, false) => "─",
                (false, true) => "│",
                (false, false) => " ",
            };
            let under = |s: &[Option<usize>]| match (s[2 * c], s[2 * c + 1]) {
                (Some(a), Some(b)) => 4 * r > a.max(b),
                _ => false,
            };
            let bg = match (under(&in_sub), under(&out_sub)) {
                (true, true) => BOTH_FILL,
                (true, false) => IN_FILL,
                (false, true) => OUT_FILL,
                (false, false) => Color::Reset,
            };
            let (ib, ob) = (in_dots[r * w + c], out_dots[r * w + c]);
            let Some(cell) = buf.cell_mut((plot.x + c as u16, plot.y + r as u16)) else {
                continue;
            };
            cell.reset();
            cell.set_bg(bg);
            if ib | ob != 0 {
                let fg = if ib.count_ones() >= ob.count_ones() { IN } else { OUT };
                cell.set_char(char::from_u32(0x2800 + (ib | ob) as u32).unwrap_or(' '));
                cell.set_fg(fg);
            } else {
                cell.set_symbol(grid);
                cell.set_fg(GRID);
            }
        }
    }

    // Y labels, right-aligned in the gutter on their gridline.
    let axis = Style::default().fg(AXIS);
    for (label, row) in y_labels.iter().zip(&tick_rows) {
        let x = inner.x + gutter - 1 - label.len() as u16;
        buf.set_string(x, plot.y + *row as u16, label, axis);
    }

    // X labels (wall clock), centered under their gridline.
    let label_y = plot.y + plot.height;
    for (col, label) in &x_ticks {
        let half = label.len() as u16 / 2;
        let x = (plot.x + *col as u16).saturating_sub(half);
        if x + label.len() as u16 <= plot.x + plot.width {
            buf.set_string(x, label_y, label, axis);
        }
    }

    draw_legend(buf, Rect { y: label_y + 1, height: 1, ..plot }, &rx, &tx, state);
}

fn draw_legend(buf: &mut Buffer, area: Rect, rx: &[f64], tx: &[f64], state: &NetState) {
    let gray = Style::default().fg(AXIS);
    let white = Style::default().fg(Color::White);
    let entry = |name: &'static str, color: Color, series: &[f64], last: f64| {
        let max = series.iter().fold(0.0f64, |a, &b| a.max(b));
        let mean = series.iter().sum::<f64>() / series.len().max(1) as f64;
        vec![
            Span::styled("━━ ", Style::default().fg(color)),
            Span::styled(format!("{name:<5}"), white),
            Span::styled("last ", gray),
            Span::styled(format!("{:<11}", fmt::rate(last)), white),
            Span::styled("mean ", gray),
            Span::styled(format!("{:<11}", fmt::rate(mean)), white),
            Span::styled("max ", gray),
            Span::styled(format!("{:<11}", fmt::rate(max)), white),
        ]
    };
    let mut spans = entry("in", IN, rx, state.rate_rx);
    spans.push(Span::raw("  "));
    spans.extend(entry("out", OUT, tx, state.rate_tx));
    buf.set_line(area.x, area.y, &Line::from(spans), area.width);
}

/// Series value at fractional sample index `back` samples before the newest,
/// linearly interpolated; `None` where history doesn't reach.
fn value_at(series: &[f64], back: f64) -> Option<f64> {
    let last = series.len().checked_sub(1)? as f64;
    let f = last - back;
    if f < -1e-6 {
        return None;
    }
    let f = f.max(0.0);
    let k = f.floor() as usize;
    let frac = f - k as f64;
    let a = series[k];
    let b = series.get(k + 1).copied().unwrap_or(a);
    Some(a + (b - a) * frac)
}

/// Rasterize a sub-pixel polyline (one y per braille column) into per-cell
/// braille dot masks, joining neighbours vertically so steep edges stay solid.
fn braille(ys: &[Option<usize>], w: usize, h: usize) -> Vec<u8> {
    const BITS: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
    let mut cells = vec![0u8; w * h];
    let mut prev: Option<usize> = None;
    for (px, y) in ys.iter().enumerate() {
        let Some(y) = *y else {
            prev = None;
            continue;
        };
        let (lo, hi) = match prev {
            Some(p) => (p.min(y), p.max(y)),
            None => (y, y),
        };
        for sy in lo..=hi.min(h * 4 - 1) {
            cells[(sy / 4) * w + px / 2] |= BITS[px % 2][sy % 4];
        }
        prev = Some(y);
    }
    cells
}

/// Smallest 1/2/5 step in binary units (B, KB, MB, ...) that is >= `raw`.
fn nice_step(raw: f64) -> f64 {
    let mut unit = 1.0;
    while raw / unit >= 1024.0 {
        unit *= 1024.0;
    }
    [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0]
        .into_iter()
        .map(|m| m * unit)
        .find(|&s| s >= raw)
        .unwrap_or(unit * 1024.0)
}

/// Rate label without a pointless ".0" ("5 MB/s", "1.5 KB/s").
fn tick_label(v: f64) -> String {
    fmt::rate(v).replace(".0 ", " ")
}

/// Wall-clock x ticks aligned to round intervals, as (plot column, "HH:MM:SS").
fn time_ticks(span: f64, w: usize) -> Vec<(usize, String)> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let cols_per_sec = (w - 1) as f64 / span;
    // Keep ~14+ columns between labels ("HH:MM:SS" plus breathing room).
    let interval = [1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0]
        .into_iter()
        .find(|i| i * cols_per_sec >= 14.0)
        .unwrap_or(600.0);
    let mut ticks = Vec::new();
    let mut t = (now / interval).floor() * interval;
    while now - t <= span {
        let col = ((w - 1) as f64 - (now - t) * cols_per_sec).round() as usize;
        ticks.push((col, clock(t as i64)));
        t -= interval;
    }
    ticks
}

fn clock(epoch: i64) -> String {
    let t = epoch as libc::time_t;
    // SAFETY: `tm` is plain old data; localtime_r only writes into it.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&t, &mut tm) };
    format!("{:02}:{:02}:{:02}", tm.tm_hour, tm.tm_min, tm.tm_sec)
}
