//! Connection stats, one line each for RX and TX: the highlighted
//! connection if there is one, else all connections summed.

use ratatui::{
    style::{Color, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::collect::Connection;
use crate::fmt;
use crate::model::{conn_key, NetState, RateStats};
use crate::ui::{rate_spans, RX, TX};

fn stat_line(
    arrow: &'static str,
    label: &'static str,
    color: Color,
    total: Option<u64>,
    stats: Option<&RateStats>,
) -> Line<'static> {
    let total = total.map_or_else(|| "-".to_string(), fmt::bytes);
    let mut spans = vec![
        Span::from(format!(" {arrow} {label} total {total:>10}")).fg(color).bold(),
        Span::from("  |  ").dark_gray(),
    ];
    spans.extend(rate_spans(stats.and_then(|s| s.rates()).as_ref(), color));
    Line::from(spans)
}

pub fn draw(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    state: &NetState,
    selected: Option<&Connection>,
) {
    let (rx, tx, scope) = match selected {
        Some(c) => {
            let st = state.conn_stats.get(&conn_key(c));
            // UDP sockets have no byte counters: nothing to show.
            let rx = (c.bytes_in, st.filter(|_| c.bytes_in.is_some()).map(|s| &s.rx));
            let tx = (c.bytes_out, st.filter(|_| c.bytes_out.is_some()).map(|s| &s.tx));
            (rx, tx, "selected connection")
        }
        None => {
            let sum = |f: fn(&Connection) -> Option<u64>| state.conns.iter().filter_map(f).sum();
            (
                (Some(sum(|c| c.bytes_in)), Some(&state.all_conns.rx)),
                (Some(sum(|c| c.bytes_out)), Some(&state.all_conns.tx)),
                "all connections",
            )
        }
    };
    let mut rx_line = stat_line("▼", "RX", RX, rx.0, rx.1);
    rx_line.push_span(Span::from(format!("   ({scope})")).dark_gray());
    let lines = vec![rx_line, stat_line("▲", "TX", TX, tx.0, tx.1)];
    frame.render_widget(Paragraph::new(lines), area);
}
