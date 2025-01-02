use std::time::{Duration, Instant};

use alloy::primitives::{BlockNumber, B256};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Color,
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
    block_numbers: StatefulList<BlockNumber>,
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

        if let Ok(t) = db.latest_block_number() {
            if let Some(num) = t {
                if !self.block_numbers.items.contains(&num) {
                    self.block_numbers.items.push(num);
                }
            }
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let chunks = Layout::horizontal([
            Constraint::Percentage(50),
            Constraint::Min(0),
        ])
        .split(frame.area());
        let app_box = Block::bordered()
            .title(Line::from(self.title.clone()).centered())
            .border_style(Color::Green);
        frame.render_widget(app_box.clone(), frame.area());
        app_box.inner(chunks[0]);
        self.draw_block_numbers_list(frame, chunks[0]);
    }

    fn draw_block_numbers_list(&mut self, frame: &mut Frame, area: Rect) {
        let block_numbers: Vec<ListItem> = self
            .block_numbers
            .items
            .iter()
            .map(|hash| {
                ListItem::new(vec![Line::from(vec![Span::raw(
                    hash.to_string(),
                )])])
            })
            .collect();
        let block_numbers_list = List::new(block_numbers).block(
            Block::bordered()
                .title(Line::from("Latest blocks").centered())
                .border_style(Color::Green),
        );
        frame.render_stateful_widget(
            block_numbers_list,
            area,
            &mut self.block_numbers.state,
        );
    }

    fn draw_block_hashes_list(&mut self, frame: &mut Frame, area: Rect) {
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
        let block_hashes_list = List::new(block_hashes).block(
            Block::bordered()
                .title(Line::from("Latest blocks").centered())
                .border_style(Color::Green),
        );
        frame.render_stateful_widget(
            block_hashes_list,
            area,
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
