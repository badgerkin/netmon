//! netmon — htop-like live network usage monitor.

mod collect;
mod fmt;
mod model;
mod ui;

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{self};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Paragraph, Wrap},
    DefaultTerminal,
};

/// Set on SIGINT; the main loop checks it and exits cleanly. In raw mode
/// Ctrl-C arrives as a literal byte (handled in `handle_key`); this covers
/// `kill -INT` and other sources.
static SIGINT: AtomicBool = AtomicBool::new(false);

/// Put the terminal back where we found it. Runs on the normal exit path
/// and in the panic/SIGINT handlers, so it must be idempotent and never
/// allocate in a way that panics.
fn restore_terminal() {
    let _ = terminal::disable_raw_mode();
    let _ = crossterm::execute!(
        std::io::stdout(),
        crossterm::event::DisableMouseCapture,
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show,
    );
}

extern "C" fn on_sigint(_sig: libc::c_int) {
    SIGINT.store(true, Ordering::Relaxed);
}

fn setup() -> std::io::Result<()> {
    // Raw mode swallows Ctrl-C into the input stream, so SIGINT mostly comes
    // from `kill`; keep the terminal clean either way.
    let _ = unsafe { libc::signal(libc::SIGINT, on_sigint as *const () as usize) };
    std::panic::set_hook(Box::new(|_info: &std::panic::PanicHookInfo| {
        restore_terminal();
        eprintln!("netmon: panicked (see `RUST_BACKTRACE=1` for details)");
        std::process::exit(101);
    }));

    terminal::enable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::EnableMouseCapture,
        crossterm::terminal::EnterAlternateScreen,
    )?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    setup()?;
    let mut term = DefaultTerminal::new(CrosstermBackend::new(std::io::stdout()))?;
    term.hide_cursor()?;

    let mut state = model::NetState::new();
    let mut uistate = ui::UiState::default();
    let mut running = true;
    let mut next_sample = Instant::now() + Duration::from_millis(100);
    // Re-spawning `ss` every tick is wasteful at 100ms delay; keep the
    // connection list at most ~1s old (or resample until we have one).
    let mut last_ss = Instant::now();

    while running {
        if SIGINT.load(Ordering::Relaxed) {
            break;
        }
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
            if state.conns.is_empty() || now.duration_since(last_ss) >= Duration::from_millis(500) {
                state.conns = collect::ss::sample();
                last_ss = now;
            }
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

    restore_terminal();
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
        // Raw mode: the terminal doesn't raise SIGINT for Ctrl-C, it sends
        // the byte instead. crossterm reports it as Ctrl+c (not the raw
        // byte), so match on the character + modifier.
        KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => *running = false,
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
}
