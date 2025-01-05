use alloy::rpc::types::Header;
use chrono::{TimeZone, Utc};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, List, ListItem},
    Frame,
};

use crate::{db::Database, utils::BuilderIdentity};

use super::components::stateful_list::StatefulList;

#[derive(Clone, Debug, Default)]
pub struct App {
    pub title: String,
    pub should_quit: bool,
    pub block_headers: StatefulList<Header>,
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
        let chunks =
            Layout::vertical([Constraint::Min(20), Constraint::Min(0)])
                .split(frame.area());
        let app_box = Block::bordered()
            .title(Line::from(self.title.clone()).centered())
            .border_style(Color::Green);
        frame.render_widget(app_box.clone(), frame.area());
        app_box.inner(chunks[1]);
        self.draw_latest_blocks_list(frame, chunks[1]);
    }

    fn draw_latest_blocks_list(&mut self, frame: &mut Frame, area: Rect) {
        let block_headers: Vec<ListItem> = self
            .block_headers
            .items
            .iter()
            .map(|header| {
                ListItem::new(vec![Line::from(vec![
                    Span::styled(
                        format!("{:<20}", header.number.to_string()),
                        Style::new().bold(),
                    ),
                    Span::raw(format!(
                        "{:<20}",
                        format!(
                            "{:.3} gwei",
                            header.base_fee_per_gas.unwrap_or_default() as f64
                                / f64::powi(10.0, 9)
                        )
                    )),
                    Span::raw(format!("{:<20}", header.gas_used)),
                    Span::raw(format!("{:<20}", header.gas_limit)),
                    Span::styled(
                        format!(
                            "{:<20}",
                            Utc.timestamp_opt(header.timestamp as i64, 0)
                                .unwrap()
                        ),
                        Style::new().underlined(),
                    ),
                    Span::styled(
                        format!(
                            "    {:<20}",
                            BuilderIdentity::from(header.extra_data.clone())
                        ),
                        Style::new().italic(),
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
