use std::time::{Duration, Instant};

use app::App;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;

use crate::db::Database;

mod app;
mod components;

const TICK_MILLIS: u64 = 500; /* 500ms */

/// Drives the TUI app
pub fn run(mut terminal: DefaultTerminal, db: &Database) -> eyre::Result<()> {
    let mut app = App::new("blocktop".to_string());
    let tick_rate: Duration = Duration::from_millis(TICK_MILLIS);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|frame| app.draw(frame))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if let KeyCode::Char(c) = key.code {
                        app.on_key(c)
                    }
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick(db);
            last_tick = Instant::now();
        }
    }
}
