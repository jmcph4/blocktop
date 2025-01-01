use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use alloy::primitives::B256;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState},
    DefaultTerminal, Frame,
};

use crate::db::Database;

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
                    match key.code {
                        KeyCode::Char(c) => app.on_key(c),
                        _ => {}
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

#[derive(Clone, Debug, Default)]
pub struct App {
    title: String,
    should_quit: bool,
    block_hashes: StatefulList<B256>,
}

impl App {
    pub fn new(title: String) -> Self {
        Self {
            title,
            ..Self::default()
        }
    }

    pub fn on_key(&mut self, c: char) {
        match c {
            'q' => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    pub fn on_tick(&mut self, db: &Database) {
        if let Ok(t) = db.latest_block_hash() {
            if let Some(hash) = t {
                if !self.block_hashes.items.contains(&hash) {
                    self.block_hashes.items.push(hash);
                }
            }
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let chunks =
            Layout::vertical([Constraint::Length(3), Constraint::Min(0)])
                .split(frame.area());
        let block_hashes_box = Block::bordered().title("Latest blocks");
        let app_box =
            Block::bordered().title(Line::from(self.title.clone()).centered());
        frame.render_widget(app_box.clone(), frame.area());
        frame.render_widget(block_hashes_box, app_box.inner(frame.area()));
        self.draw_block_hashes_list(frame, chunks);
    }

    fn draw_block_hashes_list(
        &mut self,
        frame: &mut Frame,
        chunks: Rc<[Rect]>,
    ) {
        let block_hashes: Vec<ListItem> = self
            .block_hashes
            .items
            .iter()
            .map(|hash| {
                ListItem::new(vec![Line::from(vec![Span::raw(
                    hash.to_string(),
                )])])
            })
            .collect();
        let block_hashes_list = List::new(block_hashes);
        frame.render_stateful_widget(
            block_hashes_list,
            chunks[1],
            &mut self.block_hashes.state,
        );
    }
}

#[derive(Clone, Debug, Default)]
pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn with_items(items: Vec<T>) -> Self {
        Self {
            state: ListState::default(),
            items,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}
