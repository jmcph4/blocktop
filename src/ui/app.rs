use alloy::{
    consensus::Transaction as AbstractTransaction,
    rpc::types::{Header, Transaction},
};
use chrono::{TimeZone, Utc};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols,
    text::{Line, Span, Text},
    widgets::{Bar, BarChart, BarGroup, Block, List, ListItem, Paragraph},
    Frame,
};

use crate::{
    db::Database,
    utils::{self, etherscan_block_url, BuilderIdentity},
};

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

#[derive(Clone, Debug)]
pub struct App {
    pub title: String,
    pub should_quit: bool,
    pub block_headers: StatefulList<Header>,
    pub transactions: StatefulList<alloy::rpc::types::eth::Transaction>,
    pub view: View,
    pub selected_block: alloy::rpc::types::Block,
}

impl App {
    pub fn new(
        title: String,
        selected_block: alloy::rpc::types::Block,
    ) -> Self {
        Self {
            title,
            selected_block,
            block_headers: StatefulList::with_items(vec![]),
            transactions: StatefulList::with_items(vec![]),
            should_quit: false,
            view: View::default(),
        }
    }

    pub fn on_esc(&mut self) {
        match self.view {
            View::Default => self.should_quit = true,
            _ => self.view = View::Default,
        }
    }

    pub fn on_key(&mut self, c: char) {
        if c == 'q' {
            self.should_quit = true;
        }

        if let View::Block = self.view {
            if c == 'e' {
                webbrowser::open(
                    etherscan_block_url(
                        self.selected_block.clone().header.number,
                    )
                    .as_str(),
                )
                .unwrap()
            }
        }
    }

    pub fn on_enter(&mut self) {
        if self.get_selected_header().is_some() {
            self.view = View::Block;
        }

        match self.view {
            View::Default => {
                if self.get_selected_header().is_some() {
                    self.view = View::Block
                }
            }
            View::Block => {
                if self.get_selected_transaction().is_some() {
                    todo!()
                }
            }
        }
    }

    pub fn on_up(&mut self) {
        match self.view {
            View::Default => self.block_headers.previous(),
            View::Block => self.transactions.previous(),
        }
    }

    pub fn on_down(&mut self) {
        match self.view {
            View::Default => self.block_headers.next(),
            View::Block => self.transactions.next(),
        }
    }

    pub fn on_tick(&mut self, db: &Database) {
        let latest_header = db
            .latest_block_header()
            .unwrap()
            .expect("invariant violated: must always have at least one header");

        if !self.block_headers.items.contains(&latest_header) {
            self.block_headers.items.push(latest_header.clone());
        }

        if let Some(selected_header) = self.get_selected_header() {
            if !matches!(self.view, View::Block) {
                if let Some(selected_block) =
                    db.block(selected_header.hash).unwrap()
                {
                    self.selected_block = selected_block;
                    self.transactions = StatefulList::with_items(
                        self.selected_block
                            .transactions
                            .clone()
                            .into_transactions()
                            .collect(),
                    );
                }
            }
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
                let chunks = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .margin(1)
                .split(frame.area());
                self.draw_block_view(frame, chunks[1]);
            }
        }
    }

    fn draw_block_view(&mut self, frame: &mut Frame, area: Rect) {
        let chunks =
            Layout::vertical([Constraint::Percentage(20), Constraint::Min(0)])
                .split(area);
        let block = &self.selected_block;
        let lines = vec![
            Line::from(vec![Span::styled(
                format!("Block #{} {}", block.header.number, block.header.hash),
                Style::default().bold(),
            )]),
            Line::from(vec![Span::raw(format!("Timestamp: {}", Utc.timestamp_opt(block.header.timestamp as i64, 0).unwrap()))]),
            Line::from(vec![Span::raw(format!(
                "Gas Usage (wei): {}  / {} ({:.2}%)        Base Fee (gwei): {:.3}",
                block.header.gas_used,
                block.header.gas_limit,
                (block.header.gas_used as f64) / (block.header.gas_limit as f64) * 100.0,
                block.header.base_fee_per_gas.unwrap_or_default() as f64
                    / f64::powi(10.0, 9)
            ))]),
            Line::from(vec![Span::raw(
                match BuilderIdentity::from(block.header.extra_data.clone()) {
                    BuilderIdentity::Local => format!("Beneficiary: {} (locally built)", block.header.beneficiary),
                    iden => format!("Beneficiary: {} ({})", block.header.beneficiary, iden),
                })]),
                Line::from(vec![Span::raw(format!("State Root: {}", block.header.state_root))]),
                Line::from(vec![Span::raw(format!("Contains {} transactions", block.transactions.len()))])
        ];
        let block_header_text = Paragraph::new(Text::from(lines));
        frame.render_widget(block_header_text, chunks[0]);
        self.draw_transactions_list(frame, chunks[1]);
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

    fn draw_transactions_list(&mut self, frame: &mut Frame, area: Rect) {
        let transactions: Vec<ListItem> = self
            .selected_block
            .transactions
            .clone()
            .into_transactions()
            .map(|tx| {
                let tx_info = tx.info();
                ListItem::new(vec![Line::from(vec![
                    Span::styled(
                        format!("{:<4}", tx_info.index.unwrap().to_string()),
                        Style::new().bold(),
                    ),
                    Span::raw(format!(
                        "{:<20}",
                        format!(
                            "{}",
                            utils::shorten_hash(&tx_info.hash.unwrap())
                        )
                    )),
                    Span::raw(format!(
                        "{:<20}",
                        utils::shorten_address(&tx.from)
                    )),
                    Span::raw(format!(
                        "{:<20}",
                        utils::shorten_address(&tx.to().unwrap_or_default())
                    )),
                    Span::raw(format!("{:<20}", tx.nonce())),
                    Span::raw(if tx.to().is_none() {
                        "📄".to_string()
                    } else {
                        "".to_string()
                    }),
                ])])
            })
            .collect();
        let transactions_list = List::new(transactions)
            .block(
                Block::bordered()
                    .title(Line::from("Transactions").centered())
                    .border_style(Color::Green),
            )
            .highlight_style(Style::default().bg(Color::Magenta))
            .highlight_symbol("> ");
        frame.render_stateful_widget(
            transactions_list,
            area,
            &mut self.transactions.state,
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

    fn get_selected_header(&self) -> Option<&Header> {
        self.block_headers
            .state
            .selected()
            .and_then(|offset| self.block_headers.items.get(offset))
    }

    fn get_selected_transaction(&self) -> Option<&Transaction> {
        self.transactions
            .state
            .selected()
            .and_then(|offset| self.transactions.items.get(offset))
    }
}
