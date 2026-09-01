//! netmon — htop-like live network usage monitor.

mod collect;
mod fmt;
mod model;
mod ui;

use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{self};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Paragraph, Wrap},
    DefaultTerminal,
};

fn main() -> std::io::Result<()> {
    terminal::enable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::EnableMouseCapture,
        crossterm::terminal::EnterAlternateScreen,
    )?;
    let mut term = DefaultTerminal::new(CrosstermBackend::new(std::io::stdout()))?;
    term.hide_cursor()?;

    let mut state = model::NetState::new();
    let mut uistate = ui::UiState::default();
    let mut running = true;
    let mut next_sample = Instant::now() + Duration::from_millis(100);

    while running {
        let now = Instant::now();
        if now >= next_sample {
            // Tick is due: drain any pending input (non-blocking), then sample.
            loop {
                match event::poll(Duration::from_millis(0)) {
                    Ok(true) => match event::read() {
                        Ok(Event::Key(k)) => {
                            handle_key(k, &mut uistate, &mut running);
                        }
                        Ok(_) => {}
                        Err(_) => break,
                    },
                    Ok(false) | Err(_) => break, // no pending event
                }
            }
            sample(&mut state);
            next_sample = now + Duration::from_millis(uistate.delay_ms);
        } else {
            match event::poll(next_sample - now) {
                Ok(true) => match event::read() {
                    Ok(Event::Key(k)) => {
                        handle_key(k, &mut uistate, &mut running);
                    }
                    Ok(_) => {}
                    Err(_) => {}
                },
                Ok(false) => {}
                Err(_) => break, // event error; stop
            }
        }

        let size = crossterm::terminal::size()?;
        if size.0 < 60 || size.1 < 12 {
            if term
                .draw(|f| {
                    f.render_widget(
                        Paragraph::new("terminal too small (need >= 60x12)")
                            .wrap(Wrap { trim: false }),
                        f.area(),
                    );
                })
                .is_err()
            {
                break;
            }
        } else {
            let state_ref = &state;
            let uistate_ref = &uistate;
            if term.draw(|f| ui::draw(f, state_ref, uistate_ref)).is_err() {
                break;
            }
        }
    }

    term.show_cursor()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::DisableMouseCapture,
        crossterm::terminal::LeaveAlternateScreen,
    )?;
    terminal::disable_raw_mode()?;
    Ok(())
}

fn handle_key(
    k: crossterm::event::KeyEvent,
    uistate: &mut ui::UiState,
    running: &mut bool,
) {
    if k.kind != KeyEventKind::Press {
        return;
    }
    match k.code {
        KeyCode::Char('q') => *running = false,
        KeyCode::Char('s') => uistate.sort = uistate.sort.cycle(),
        KeyCode::Char('f') => uistate.filter = uistate.filter.cycle(),
        KeyCode::Char('p') => uistate.show_procs = !uistate.show_procs,
        KeyCode::Char('d') => {
            uistate.delay_ms = match uistate.delay_ms {
                100 => 500,
                500 => 1000,
                _ => 100,
            };
        }
        KeyCode::Char('h') => uistate.help = !uistate.help,
        KeyCode::Esc => uistate.help = false,
        _ => {}
    }
}

fn sample(state: &mut model::NetState) {
    let now = Instant::now();
    let dt = state
        .last_tick
        .map_or(1.0, |t| now.duration_since(t).as_secs_f64())
        .max(0.01);
    state.last_tick = Some(now);
    state.tick(collect::ifaces::sample(), dt);
    state.conns = collect::ss::sample();
}
