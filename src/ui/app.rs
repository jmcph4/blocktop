use alloy::rpc::types::Header;
use chrono::{TimeZone, Utc};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols,
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Block, List, ListItem, Row, Table},
    Frame,
};

use crate::{db::Database, utils::BuilderIdentity};

use super::components::stateful_list::StatefulList;

#[derive(Copy, Clone, Debug)]
pub enum View {
    Default,
    Block,
}

impl Default for View {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Clone, Debug, Default)]
pub struct App {
    pub title: String,
    pub should_quit: bool,
    pub block_headers: StatefulList<Header>,
    pub view: View,
    pub selected_block: Option<alloy::rpc::types::Block>,
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

    pub fn on_enter(&mut self) {
        if self.get_selected().is_some() {
            self.view = View::Block;
        }
    }

    pub fn on_up(&mut self) {
        self.block_headers.previous();
    }

    pub fn on_down(&mut self) {
        self.block_headers.next();
    }

    pub fn on_tick(&mut self, db: &Database) {
        if let Ok(Some(header)) = db.latest_block_header() {
            if !self.block_headers.items.contains(&header) {
                self.block_headers.items.push(header);
            }
        }

        if let Some(header) = self.get_selected() {
            self.selected_block = db.block(header.number.into());
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let app_box = Block::bordered()
            .title(Line::from(self.title.clone()).centered())
            .border_style(Color::Green);
        frame.render_widget(app_box.clone(), frame.area());

        match self.view {
            View::Default => {
                let chunks =
                    Layout::vertical([Constraint::Min(20), Constraint::Min(0)])
                        .split(frame.area());
                self.draw_latest_blocks_list(frame, chunks[1]);
                self.draw_gas_barchart(frame, chunks[0], app_box);
            }
            View::Block => {
                let chunks =
                    Layout::vertical([Constraint::Min(20), Constraint::Min(0)])
                        .split(frame.area());
                self.draw_block_view(frame, chunks[0]);
            }
        }
    }

    fn draw_block_view(&mut self, frame: &mut Frame, area: Rect) {
        let block = self.selected_block.as_ref().expect(
            "invariant violated: entered block view without selected block",
        );
        let block_header_table = Table::new(
            vec![
                Row::new(vec!["Number", "Hash"]),
                Row::new(vec![
                    block.header.number.to_string(),
                    block.header.hash.to_string(),
                ]),
            ],
            [Constraint::Length(16), Constraint::Length(64)],
        );
        frame.render_widget(block_header_table, area);
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
        let latest_blocks_list = List::new(block_headers)
            .block(
                Block::bordered()
                    .title(Line::from("Latest blocks").centered())
                    .border_style(Color::Green),
            )
            .highlight_style(Style::default().bg(Color::Magenta))
            .highlight_symbol("> ");
        frame.render_stateful_widget(
            latest_blocks_list,
            area,
            &mut self.block_headers.state,
        );
    }

    fn draw_gas_barchart(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        block: Block,
    ) {
        let barchart = BarChart::default()
            .block(block)
            .data(self.gas_bar_group())
            .bar_width(8)
            .bar_gap(8)
            .bar_set(symbols::bar::NINE_LEVELS)
            .value_style(
                Style::default().fg(Color::Black).bg(Color::Green).italic(),
            )
            .label_style(Style::default().fg(Color::Yellow))
            .bar_style(Style::default().fg(Color::Green));
        frame.render_widget(barchart, area);
    }

    fn chart_data(&self) -> Vec<(String, u64)> {
        self.block_headers
            .items
            .iter()
            .map(|header| (header.number.to_string(), header.gas_used))
            .collect()
    }

    fn gas_bar_group(&self) -> BarGroup<'_> {
        let mut xs = BarGroup::default();
        let bars: Vec<Bar<'_>> = self
            .chart_data()
            .iter()
            .map(|(k, v)| {
                Bar::default()
                    .label(Line::from(k.clone()))
                    .value(*v)
                    .text_value(String::new())
            })
            .collect();
        xs = xs.clone().bars(&bars[..]);
        xs.clone()
    }

    fn get_selected(&self) -> Option<&Header> {
        self.block_headers
            .state
            .selected()
            .map(|offset| self.block_headers.items.get(offset))
            .flatten()
    }
}
