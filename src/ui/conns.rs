//! Sortable table of active connections.

use ratatui::{
    layout::Constraint,
    style::{Color, Modifier, Style},
    widgets::{Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table},
    Frame,
};

use crate::fmt;
use crate::collect::Connection;
use crate::model::NetState;
use crate::ui::{UiState, RX, TX};

/// Byte count cell; "-" when the kernel doesn't report it (e.g. UDP).
fn bytes_cell(n: Option<u64>, color: Color) -> Cell<'static> {
    Cell::from(n.map(fmt::bytes).unwrap_or_else(|| "-".into())).style(Style::default().fg(color))
}

/// One sort key per table column, in column order.
#[derive(Clone, Copy, PartialEq)]
pub enum SortKey {
    Proto,
    State,
    Local,
    Peer,
    Rx,
    Tx,
    Pid,
    Prog,
}

const ALL_KEYS: [SortKey; 8] = [
    SortKey::Proto,
    SortKey::State,
    SortKey::Local,
    SortKey::Peer,
    SortKey::Rx,
    SortKey::Tx,
    SortKey::Pid,
    SortKey::Prog,
];

impl SortKey {
    /// Next (`step` = 1) or previous (`step` = -1) column in table order,
    /// skipping PID/PROG while the process columns are hidden.
    pub fn cycle(&self, step: isize, show_procs: bool) -> Self {
        let keys: &[SortKey] = if show_procs { &ALL_KEYS } else { &ALL_KEYS[..6] };
        let n = keys.len() as isize;
        let i = keys.iter().position(|k| k == self).unwrap_or(0) as isize;
        keys[(i + step).rem_euclid(n) as usize]
    }

    pub fn is_proc(&self) -> bool {
        matches!(self, SortKey::Pid | SortKey::Prog)
    }

    pub fn label(&self) -> &'static str {
        match self {
            SortKey::Proto => "proto",
            SortKey::State => "state",
            SortKey::Local => "local",
            SortKey::Peer => "peer",
            SortKey::Rx => "rx",
            SortKey::Tx => "tx",
            SortKey::Pid => "pid",
            SortKey::Prog => "prog",
        }
    }

    /// Natural direction: biggest byte counts first, everything else ascending.
    pub fn default_desc(&self) -> bool {
        matches!(self, SortKey::Rx | SortKey::Tx)
    }

    fn column(&self) -> &'static str {
        match self {
            SortKey::Proto => "PROTO",
            SortKey::State => "STATE",
            SortKey::Local => "LOCAL",
            SortKey::Peer => "PEER",
            SortKey::Rx => "RX",
            SortKey::Tx => "TX",
            SortKey::Pid => "PID",
            SortKey::Prog => "PROG",
        }
    }
}

/// Order "host:port" numerically: IPs by address (v4 before v6), then port,
/// so 10.0.0.9 < 10.0.0.10 and :80 < :443. Unparseable parts ("*") sort first.
fn addr_key(s: &str) -> (Option<std::net::IpAddr>, u32, &str) {
    let (host, port) = s.rsplit_once(':').unwrap_or((s, ""));
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let host = host.split('%').next().unwrap_or(host); // drop "%iface"
    (host.parse().ok(), port.parse().unwrap_or(0), s)
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

/// Connections as shown in the table: filtered, then sorted.
pub fn visible<'a>(state: &'a NetState, ui: &UiState) -> Vec<&'a Connection> {
    let conns: Vec<&Connection> = state
        .conns
        .iter()
        .filter(|c| ui.filter.matches(&c.proto))
        .collect();

    // Byte columns compare the raw u64 counters, never the formatted text,
    // so "900 KB" < "1.2 MB" regardless of unit.
    let mut conns = conns;
    conns.sort_by(|a, b| {
        let ord = match ui.sort {
            SortKey::Rx => a.bytes_in
                .cmp(&b.bytes_in)
                .then(a.bytes_out.cmp(&b.bytes_out))
                .then(a.queued_bytes().cmp(&b.queued_bytes())),
            SortKey::Tx => a.bytes_out
                .cmp(&b.bytes_out)
                .then(a.bytes_in.cmp(&b.bytes_in))
                .then(a.queued_bytes().cmp(&b.queued_bytes())),
            SortKey::State => a.state.cmp(&b.state).then(addr_key(&a.peer).cmp(&addr_key(&b.peer))),
            SortKey::Peer => addr_key(&a.peer)
                .cmp(&addr_key(&b.peer))
                .then(addr_key(&a.local).cmp(&addr_key(&b.local))),
            SortKey::Proto => a.proto.cmp(&b.proto).then(b.total_bytes().cmp(&a.total_bytes())),
            SortKey::Local => addr_key(&a.local)
                .cmp(&addr_key(&b.local))
                .then(addr_key(&a.peer).cmp(&addr_key(&b.peer))),
            // Rows without process info go after the ones that have it.
            SortKey::Pid => (a.pid.is_none(), a.pid)
                .cmp(&(b.pid.is_none(), b.pid))
                .then(addr_key(&a.local).cmp(&addr_key(&b.local))),
            SortKey::Prog => (a.name.is_empty(), &a.name)
                .cmp(&(b.name.is_empty(), &b.name))
                .then(a.pid.cmp(&b.pid))
                .then(addr_key(&a.local).cmp(&addr_key(&b.local))),
        };
        if ui.sort_desc { ord.reverse() } else { ord }
    });
    conns
}

pub fn draw(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    conns: &[&Connection],
    ui: &mut UiState,
) {
    let procs = ui.show_procs;
    // The header already shows "sort: X"; keep the column header short.
    let mut names = vec!["PROTO", "STATE", "LOCAL", "PEER", "RX", "TX"];
    if procs {
        names.extend(["PID", "PROG"]);
    }
    let arrow = if ui.sort_desc { "▼" } else { "▲" };
    // htop-style: the sort column's header cell stands out from the bar,
    // with an arrow for the direction.
    let header_cells: Vec<Cell> = names
        .into_iter()
        .map(|n| {
            if n == ui.sort.column() {
                Cell::from(format!("{n} {arrow}"))
                    .style(Style::default().bg(Color::Cyan).add_modifier(Modifier::BOLD))
            } else {
                Cell::from(n)
            }
        })
        .collect();
    let header = Row::new(header_cells).style(Style::default().fg(Color::Black).bg(Color::Blue));

    let rows: Vec<Row> = conns.iter().map(|c| {
        let mut cells: Vec<Cell> = vec![
            Cell::from(c.proto.as_str()),
            Cell::from(c.state.clone()),
            Cell::from(c.local.clone()),
            Cell::from(c.peer.clone()),
            bytes_cell(c.bytes_in, RX),
            bytes_cell(c.bytes_out, TX),
        ];
        if procs {
            cells.push(Cell::from(c.pid.map(|p| p.to_string()).unwrap_or_default()));
            cells.push(Cell::from(c.name.clone()));
        }
        Row::new(cells)
    }).collect();

    let widths = if procs {
        vec![
            Constraint::Length(7), // fits "PROTO ▲"
            Constraint::Length(9),
            Constraint::Percentage(22),
            Constraint::Percentage(22),
            Constraint::Length(9),
            Constraint::Length(9),
            Constraint::Length(7),
            Constraint::Fill(1), // PROG takes whatever width is left
        ]
    } else {
        vec![
            Constraint::Length(7), // fits "PROTO ▲"
            Constraint::Length(9),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
            Constraint::Length(9),
            Constraint::Length(9),
        ]
    };

    let len = rows.len();
    // Visible data rows (minus the header); used for PgUp/PgDn.
    ui.page = area.height.saturating_sub(1).max(1);
    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(Style::default().fg(Color::Black).bg(Color::White));
    frame.render_stateful_widget(table, area, &mut ui.table);

    // Scrollbar only when the list doesn't fit.
    if len > ui.page as usize {
        let mut sb = ScrollbarState::new(len.saturating_sub(ui.page as usize))
            .position(ui.table.offset());
        let sb_area = ratatui::layout::Rect {
            y: area.y + 1,
            height: area.height.saturating_sub(1),
            ..area
        };
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            sb_area,
            &mut sb,
        );
    }
}
