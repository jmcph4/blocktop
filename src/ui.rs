use std::time::{Duration, Instant};

use alloy::rpc::types::Header;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState},
    DefaultTerminal, Frame,
};

use crate::{db::Database, utils::BuilderIdentity};

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

#[derive(Clone, Debug, Default)]
pub struct App {
    title: String,
    should_quit: bool,
    block_headers: StatefulList<Header>,
}

impl App {
    pub fn new(title: String) -> Self {
        Self {
            title,
            ..Self::default()
        }
    }

    pub fn on_key(&mut self, c: char) {
        if c == 'q' {
            self.should_quit = true;
        }
    }

    pub fn on_tick(&mut self, db: &Database) {
        if let Ok(Some(header)) = db.latest_block_header() {
            if !self.block_headers.items.contains(&header) {
                self.block_headers.items.push(header);
            }
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let chunks = Layout::horizontal([
            Constraint::Percentage(25),
            Constraint::Min(0),
        ])
        .split(frame.area());
        let app_box = Block::bordered()
            .title(Line::from(self.title.clone()).centered())
            .border_style(Color::Green);
        frame.render_widget(app_box.clone(), frame.area());
        app_box.inner(chunks[0]);
        self.draw_latest_blocks_list(frame, chunks[0]);
    }

    fn draw_latest_blocks_list(&mut self, frame: &mut Frame, area: Rect) {
        let block_headers: Vec<ListItem> = self
            .block_headers
            .items
            .iter()
            .map(|header| {
                ListItem::new(vec![Line::from(vec![
                    Span::raw(format!("{:<16}", header.number.to_string())),
                    Span::styled(
                        BuilderIdentity::from(header.extra_data.clone())
                            .to_string(),
                        Style::new().bold(),
                    ),
                ])])
            })
            .collect();
        let latest_blocks_list = List::new(block_headers).block(
            Block::bordered()
                .title(Line::from("Latest blocks").centered())
                .border_style(Color::Green),
        );
        frame.render_stateful_widget(
            latest_blocks_list,
            area,
            &mut self.block_headers.state,
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
