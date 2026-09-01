//! Sortable table of active connections.

use ratatui::{
    layout::Constraint,
    style::{Color, Style},
    widgets::{Cell, Row, Table},
    Frame,
};

use crate::fmt;
use crate::model::NetState;
use crate::ui::UiState;

#[derive(Clone, Copy, PartialEq)]
pub enum SortKey {
    Bytes,
    State,
    Peer,
    Proto,
}

impl SortKey {
    pub fn cycle(&self) -> Self {
        match self {
            SortKey::Bytes => SortKey::State,
            SortKey::State => SortKey::Peer,
            SortKey::Peer => SortKey::Proto,
            SortKey::Proto => SortKey::Bytes,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SortKey::Bytes => "bytes",
            SortKey::State => "state",
            SortKey::Peer => "peer",
            SortKey::Proto => "proto",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Filter {
    All,
    Tcp,
    Udp,
}

impl Filter {
    pub fn cycle(&self) -> Self {
        match self {
            Filter::All => Filter::Tcp,
            Filter::Tcp => Filter::Udp,
            Filter::Udp => Filter::All,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Filter::All => "all",
            Filter::Tcp => "tcp",
            Filter::Udp => "udp",
        }
    }

    pub fn matches(&self, proto: &str) -> bool {
        match self {
            Filter::All => true,
            Filter::Tcp => proto == "TCP",
            Filter::Udp => proto == "UDP",
        }
    }
}

pub fn draw(frame: &mut Frame, area: ratatui::layout::Rect, state: &NetState, ui: &UiState) {
    use crate::collect::Connection;

    let conns: Vec<&Connection> = state
        .conns
        .iter()
        .filter(|c| ui.filter.matches(&c.proto))
        .collect();

    let mut conns = conns;
    match ui.sort {
        SortKey::Bytes => conns.sort_by(|a, b| b.queued_bytes().cmp(&a.queued_bytes())),
        SortKey::State => conns.sort_by(|a, b| a.state.cmp(&b.state).then(a.peer.cmp(&b.peer))),
        SortKey::Peer => conns.sort_by(|a, b| a.peer.cmp(&b.peer)),
        SortKey::Proto => conns.sort_by(|a, b| a.proto.cmp(&b.proto).then(b.queued_bytes().cmp(&a.queued_bytes()))),
    }

    let procs = ui.show_procs;
    let header_cells: Vec<Cell> = if procs {
        vec![
            Cell::from("PROTO"),
            Cell::from("STATE"),
            Cell::from("LOCAL"),
            Cell::from("PEER"),
            Cell::from(format!("BYTES ({} sort)", ui.sort.label())),
            Cell::from("PID"),
            Cell::from("PROG"),
        ]
    } else {
        vec![
            Cell::from("PROTO"),
            Cell::from("STATE"),
            Cell::from("LOCAL"),
            Cell::from("PEER"),
            Cell::from(format!("BYTES ({} sort)", ui.sort.label())),
        ]
    };
    let header = Row::new(header_cells).style(Style::default().fg(Color::Black).bg(Color::Blue));

    let rows: Vec<Row> = conns.iter().map(|c| {
        let mut cells: Vec<Cell> = vec![
            Cell::from(c.proto.as_str()),
            Cell::from(c.state.clone()),
            Cell::from(c.local.clone()),
            Cell::from(c.peer.clone()),
            Cell::from(fmt::bytes(c.queued_bytes())),
        ];
        if procs {
            cells.push(Cell::from(c.pid.map(|p| p.to_string()).unwrap_or_default()));
            cells.push(Cell::from(c.name.clone()));
        }
        Row::new(cells)
    }).collect();

    let widths = if procs {
        vec![
            Constraint::Length(5),
            Constraint::Length(10),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Length(9),
            Constraint::Length(7),
        ]
    } else {
        vec![
            Constraint::Length(5),
            Constraint::Length(10),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
            Constraint::Length(9),
        ]
    };

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(Style::default().bg(Color::White));
    frame.render_widget(table, area);
}
