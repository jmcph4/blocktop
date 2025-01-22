use std::time::{Duration, Instant};

use alloy::{
    eips::{BlockHashOrNumber, HashOrNumber},
    primitives::TxHash,
};
use app::{App, View};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::DefaultTerminal;

use crate::db::Database;

pub mod app;
mod components;

const TICK_MILLIS: u64 = 250; /* 250ms */

/// Drives the TUI app
pub fn run(
    mut terminal: DefaultTerminal,
    db: &Database,
    block: Option<BlockHashOrNumber>,
    transaction: Option<TxHash>,
) -> eyre::Result<()> {
    /* we're able to wet the UI with selected chain objects due to wetting the
     * database on startup */
    let latest_block = db.latest_block()?.expect(
        "invariant violated: database must always have at least one block",
    );
    let latest_tx = latest_block
        .clone()
        .transactions
        .into_transactions()
        .next()
        .expect("invariant violated: latest block must be non-empty");
    let mut app = App::new("blocktop".to_string(), latest_block, latest_tx);

    if let Some(specified_block) = block {
        app.view = View::Block;
        let specified_block_hash = match specified_block {
            HashOrNumber::Hash(h) => h,
            HashOrNumber::Number(x) => {
                db.block_by_number(x)?.unwrap().header.hash
            }
        };
        app.selected_block = db.block_by_hash(specified_block_hash)?.unwrap();
    } else if let Some(specified_tx) = transaction {
        app.view = View::Transaction;
        app.selected_transaction = db.transaction(specified_tx)?.unwrap();
    }

    let tick_rate: Duration = Duration::from_millis(TICK_MILLIS);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|frame| app.draw(frame))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => app.on_up(),
                    KeyCode::Down | KeyCode::Char('j') => app.on_down(),
                    KeyCode::Enter => app.on_enter(),
                    KeyCode::Esc => app.on_esc(),
                    KeyCode::Char('c')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        app.on_quit()
                    }
                    KeyCode::Char(c) => app.on_key(c),
                    _ => {}
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
