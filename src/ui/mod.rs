//! Layout: header / meters / chart / totals / connections table / footer.

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Span,
    widgets::{Clear, Paragraph, TableState},
    Frame,
};

use crate::fmt;
use crate::model::{NetState, Rates};

pub mod chart;
pub mod conns;
pub mod meters;
pub mod totals;

/// Traffic colours shared by meters, chart, totals and the connection table.
pub const RX: Color = Color::Rgb(115, 191, 105);
pub const TX: Color = Color::Rgb(87, 148, 242);
pub const STAT_LABEL: Color = Color::Rgb(140, 140, 140);

/// The standard rate readout used everywhere: "max X  min X  avg X  cur X".
/// `None` (no samples / no counters) prints "-" for every value.
pub fn rate_spans(r: Option<&Rates>, color: Color) -> Vec<Span<'static>> {
    let vals = r.map(|r| [r.max, r.min, r.avg, r.cur]);
    let mut spans = Vec::with_capacity(8);
    for (i, label) in ["max", "min", "avg", "cur"].into_iter().enumerate() {
        let v = vals.map_or_else(|| "-".to_string(), |v| fmt::rate(v[i]));
        let sep = if i == 0 { "" } else { "  " };
        spans.push(Span::styled(format!("{sep}{label} "), Style::default().fg(STAT_LABEL)));
        spans.push(Span::styled(format!("{v:>10}"), Style::default().fg(color)));
    }
    spans
}

pub struct UiState {
    pub sort: conns::SortKey,
    /// Descending when true; reset to the key's natural direction on `s`.
    pub sort_desc: bool,
    pub filter: conns::Filter,
    pub show_procs: bool,
    pub help: bool,
    pub delay_ms: u64,
    /// Selection + scroll offset of the connections table.
    pub table: TableState,
    /// Visible connection rows at last draw (page size for PgUp/PgDn).
    pub page: u16,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            sort: conns::SortKey::Rx,
            sort_desc: true,
            filter: conns::Filter::All,
            show_procs: true,
            help: false,
            delay_ms: 500,
            // No selection: the stats lines cover all connections.
            table: TableState::default(),
            page: 10,
        }
    }
}

pub const FOOTER: &str = " q quit  ↑↓/PgUp/PgDn scroll  s sort  r reverse  f filter  p proc col  d delay  h help ";

pub fn draw(frame: &mut Frame, state: &NetState, ui: &mut UiState) {
    let size = frame.area();
    if size.width < 60 || size.height < 12 {
        frame.render_widget(
            Paragraph::new("terminal too small (need >= 60x12)"),
            frame.area(),
        );
        return;
    }

    // Give the graph roughly half the screen (clamped) so it's actually readable.
    let chart_h = (size.height / 2).clamp(8, 18);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),    // header
            Constraint::Length(2),    // meters
            Constraint::Length(chart_h), // chart
            Constraint::Length(2),    // connection stats (RX / TX)
            Constraint::Min(0),       // connections
            Constraint::Length(1),    // footer
        ])
        .split(size);

    let header = Paragraph::new(format!(
        " netmon  |  {}ms  |  {} conns  |  up {}  |  sort: {} {}  filter: {} ",
        ui.delay_ms,
        state.conns.len(),
        crate::fmt::fmt_duration(state.started.elapsed()),
        ui.sort.label(),
        if ui.sort_desc { "▼" } else { "▲" },
        ui.filter.label(),
    ))
    .style(Style::default().fg(Color::Blue));
    frame.render_widget(header, chunks[0]);

    meters::draw(frame, chunks[1], state);
    chart::draw(frame, chunks[2], state);
    // One filtered+sorted list feeds both the table and the stats lines, so
    // the selected index maps to the highlighted row. The table clamps an
    // out-of-range selection to the last row when rendering; mirror that.
    let conns = conns::visible(state, ui);
    let selected = ui
        .table
        .selected()
        .and_then(|i| conns.get(i.min(conns.len().saturating_sub(1))).copied());
    totals::draw(frame, chunks[3], state, selected);
    conns::draw(frame, chunks[4], &conns, ui);

    let footer = Paragraph::new(FOOTER).style(Style::default().fg(Color::Gray));
    frame.render_widget(footer, chunks[5]);

    if ui.help {
        draw_help(frame);
    }
}

fn draw_help(frame: &mut Frame) {
    let size = frame.area();
    let lines = [
        "q - quit",
        "j/k Up/Down PgUp/PgDn Home/End - scroll",
        "s / S - sort by next / previous column",
        "r - reverse sort direction (asc / desc)",
        "f - cycle filter: all / tcp / udp",
        "p - toggle process (pid/prog) column",
        "d - cycle refresh delay: 100 / 500 / 1000 ms",
        "Esc - clear selection (stats cover all conns)",
        "h / Esc - close this help",
    ];
    // Size to the content (plus border + padding), centered, clamped to screen.
    let text_w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as u16;
    let w = (text_w + 4).min(size.width);
    let h = (lines.len() as u16 + 2).min(size.height);
    let area = ratatui::layout::Rect::new(
        (size.width - w) / 2,
        (size.height - h) / 2,
        w,
        h,
    );

    let help = Paragraph::new(lines.join("\n"))
        .wrap(ratatui::widgets::Wrap { trim: false })
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .block(
            ratatui::widgets::Block::default()
                .title(" help (h) ")
                .borders(ratatui::widgets::Borders::ALL)
                .padding(ratatui::widgets::Padding::horizontal(1))
                .border_style(Style::default().fg(Color::Blue))
                .style(Style::default().bg(Color::Black)),
        );
    // Wipe what's underneath so the dialog is opaque.
    frame.render_widget(Clear, area);
    frame.render_widget(help, area);
}
