//! Layout: header / meters / chart / totals / connections table / footer.

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

use crate::model::NetState;

pub mod chart;
pub mod conns;
pub mod meters;
pub mod totals;

pub struct UiState {
    pub sort: conns::SortKey,
    pub filter: conns::Filter,
    pub show_procs: bool,
    pub help: bool,
    pub delay_ms: u64,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            sort: conns::SortKey::Bytes,
            filter: conns::Filter::All,
            show_procs: true,
            help: false,
            delay_ms: 500,
        }
    }
}

pub const FOOTER: &str = " q quit  s sort  f filter  p proc col  d delay  h help ";

pub fn draw(frame: &mut Frame, state: &NetState, ui: &UiState) {
    let size = frame.area();
    if size.width < 60 || size.height < 12 {
        frame.render_widget(
            Paragraph::new("terminal too small (need >= 60x12)"),
            frame.area(),
        );
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // header
            Constraint::Length(2),  // meters
            Constraint::Length(3),  // chart
            Constraint::Length(1),  // totals
            Constraint::Min(0),     // connections
            Constraint::Length(1),  // footer
        ])
        .split(size);

    let header = Paragraph::new(format!(
        " netmon  |  {}ms  |  {} conns  |  up {}  |  sort: {}  filter: {} ",
        ui.delay_ms,
        state.conns.len(),
        crate::fmt::fmt_duration(state.started.elapsed()),
        ui.sort.label(),
        ui.filter.label(),
    ))
    .style(Style::default().fg(Color::Blue));
    frame.render_widget(header, chunks[0]);

    meters::draw(frame, chunks[1], state);
    chart::draw(frame, chunks[2], state);
    totals::draw(frame, chunks[3], state);
    conns::draw(frame, chunks[4], state, ui);

    let footer = Paragraph::new(FOOTER).style(Style::default().fg(Color::Gray));
    frame.render_widget(footer, chunks[5]);

    if ui.help {
        draw_help(frame);
    }
}

fn draw_help(frame: &mut Frame) {
    let size = frame.area();
    // Centered 40x14 overlay.
    let w = size.width.saturating_sub(80).min(60);
    let h = size.height.saturating_sub(28).min(20).max(10);
    let x = (size.width.saturating_sub(w)) / 2;
    let y = (size.height.saturating_sub(h)) / 2;

    let lines = vec![
        "q - quit",
        "s - cycle sort: bytes / state / peer / proto",
        "f - cycle filter: all / tcp / udp",
        "p - toggle process (pid/prog) column",
        "d - cycle refresh delay: 100 / 500 / 1000 ms",
        "h / Esc - close this help",
    ];
    let help = Paragraph::new(lines.join("\n"))
        .wrap(ratatui::widgets::Wrap { trim: false })
        .style(Style::default().fg(Color::White))
        .block(
            ratatui::widgets::Block::default()
                .title(" help (h) ")
                .borders(ratatui::widgets::Borders::ALL)
                .style(Style::default().fg(Color::Blue)),
        );
    frame.render_widget(help, ratatui::layout::Rect::new(x, y, w, h));
}
