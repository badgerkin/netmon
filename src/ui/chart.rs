//! Live in/out throughput chart.

use ratatui::{
    style::{Color, Style},
    text::Line,
    widgets::{Axis, Chart, Dataset},
    Frame,
};

use crate::fmt;
use crate::model::NetState;

pub fn draw(frame: &mut Frame, area: ratatui::layout::Rect, state: &NetState) {
    let hi = state.peak().max(1.0) * 1.1;

    // Chart data points are (x, y) pairs; x is the sample index.
    let in_data: Vec<(f64, f64)> = state
        .history
        .iter()
        .enumerate()
        .map(|(i, s)| (i as f64, s.rx))
        .collect();
    let out_data: Vec<(f64, f64)> = state
        .history
        .iter()
        .enumerate()
        .map(|(i, s)| (i as f64, s.tx))
        .collect();

    let chart = Chart::new(vec![
        Dataset::default()
            .name(format!("▼ in ({})", fmt::rate(state.rate_rx)))
            .data(&in_data)
            .style(Style::default().fg(Color::Green)),
        Dataset::default()
            .name(format!("▲ out ({})", fmt::rate(state.rate_tx)))
            .data(&out_data)
            .style(Style::default().fg(Color::Cyan)),
    ])
    .x_axis(Axis::default().bounds([0.0, (crate::model::HISTORY - 1) as f64]))
    .y_axis(
        Axis::default()
            .bounds([0.0, hi])
            .labels(vec![Line::from("0"), Line::from(fmt::rate(hi))]),
    );
    frame.render_widget(chart, area);
}
