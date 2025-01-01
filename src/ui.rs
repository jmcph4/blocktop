use crossterm::event::{self, Event};
use ratatui::DefaultTerminal;

use crate::db::Database;

pub fn run(mut terminal: DefaultTerminal, db: &Database) -> eyre::Result<()> {
    loop {
        terminal.draw(|frame| {
            frame.render_widget(
                match db.latest_block_hash() {
                    Ok(t) => match t {
                        Some(hash) => format!("{hash}"),
                        None => "No blocks".to_string(),
                    },
                    Err(e) => format!("Error: {e}"),
                },
                frame.area(),
            );
        })?;
        if matches!(event::read()?, Event::Key(_)) {
            break Ok(());
        }
    }
}
