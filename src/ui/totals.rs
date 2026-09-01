//! Cumulative totals + peak/min/average rates, one line.

use ratatui::{text::Line, widgets::Paragraph, Frame};

use crate::fmt;
use crate::model::NetState;

pub fn draw(frame: &mut Frame, area: ratatui::layout::Rect, state: &NetState) {
    let min_rx = if state.min_rx.is_infinite() { 0.0 } else { state.min_rx };
    let min_tx = if state.min_tx.is_infinite() { 0.0 } else { state.min_tx };
    let text = format!(
        " Σ in  {}  |  peak {}  avg {}  min {}    Σ out  {}  |  peak {}  avg {}  min {}",
        fmt::bytes(state.totals_rx),
        fmt::rate(state.peak_rx),
        fmt::rate(state.avg_rx()),
        fmt::rate(min_rx),
        fmt::bytes(state.totals_tx),
        fmt::rate(state.peak_tx),
        fmt::rate(state.avg_tx()),
        fmt::rate(min_tx),
    );
    frame.render_widget(Paragraph::new(Line::from(text)), area);
}
