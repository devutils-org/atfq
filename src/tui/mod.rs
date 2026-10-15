//! `atfq tui`: the interactive mode. This module owns the terminal; all the
//! behaviour lives in [`App`].

mod app;

pub use app::{App, Pane, Region};

use std::io::{self, stdout};

use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::Terminal;

use crate::content::Library;
use crate::probes::ProbeCtx;
use crate::prompt::SystemInfo;

/// Runs the TUI until the user quits, opening `start` first if given.
pub fn run(lib: &Library, ctx: ProbeCtx, sys: SystemInfo, start: Option<&str>) -> io::Result<()> {
    let mut app = App::new(lib, ctx, sys);
    if let Some(q) = start {
        app.open(q);
    }

    // Restore the terminal before the panic message is printed, or it
    // would land on the alternate screen and the shell would stay raw.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        previous(info);
    }));

    enable_raw_mode()?;
    let result = execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)
        .and_then(|_| Terminal::new(CrosstermBackend::new(stdout())))
        .and_then(|mut terminal| event_loop(&mut terminal, &mut app));
    restore();
    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App<'_>,
) -> io::Result<()> {
    let mut captured = true;
    loop {
        terminal.draw(|f| app.draw(f))?;
        if app.should_quit() {
            return Ok(());
        }
        app.handle(event::read()?);
        if app.mouse_captured() != captured {
            captured = app.mouse_captured();
            if captured {
                execute!(stdout(), EnableMouseCapture)?;
            } else {
                execute!(stdout(), DisableMouseCapture)?;
            }
        }
    }
}

/// Puts the terminal back the way the shell expects it. Safe to call twice.
fn restore() {
    let _ = disable_raw_mode();
    let _ = execute!(
        stdout(),
        DisableMouseCapture,
        LeaveAlternateScreen,
        ratatui::crossterm::cursor::Show
    );
}
