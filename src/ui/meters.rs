//! htop-style colored rate meters.

use ratatui::{
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::fmt;
use crate::model::NetState;

/// Fixed width for the rate labels so the bars start at the same column
/// on the in/out rows. "999.9 GB/s" / "999.9 TB/s" is the widest rate we
/// can print.
const RATE_W: usize = 10;

pub fn draw(frame: &mut Frame, area: ratatui::layout::Rect, state: &NetState) {
    // Prefix: " ▼ in "(6) + rate (10) + "  "(2) = 18; suffix: "  "(2) + peak (10).
    let bar_w = (area.width.saturating_sub(18 + 2 + RATE_W as u16)).max(10);
    let in_line = meter_line(state, "in", Color::Green, bar_w, state.rate_rx, state.peak_rx);
    let out_line = meter_line(
        state,
        "out",
        Color::Cyan,
        bar_w,
        state.rate_tx,
        state.peak_tx,
    );
    frame.render_widget(Paragraph::new(vec![in_line, out_line]), area);
}

fn meter_line(
    _state: &NetState,
    label: &str,
    color: Color,
    bar_w: u16,
    rate: f64,
    peak: f64,
) -> Line<'static> {
    let frac = if peak > 0.0 {
        (rate / peak).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let filled = (frac * bar_w as f64) as u16;
    let bar = "█".repeat(filled as usize) + &"░".repeat((bar_w - filled) as usize);
    Line::from(vec![
        Span::styled(
            format!(" {} ", if label == "in" { "▼" } else { "▲" }),
            Style::default().fg(color),
        ),
        Span::raw(format!(" {} ", label)),
        Span::styled(
            format!("{:RATE_W$}", fmt::rate(rate)),
            Style::default().fg(color).bold(),
        ),
        Span::raw("  "),
        Span::styled(bar, Style::default().fg(color)),
        Span::raw("  "),
        Span::styled(
            format!("{:RATE_W$}", fmt::rate(peak)),
            Style::default().fg(Color::DarkGray),
        ),
    ])
}
