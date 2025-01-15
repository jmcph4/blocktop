use alloy::{
    consensus::Transaction as AbstractTransaction,
    primitives::{Address, Bytes},
    rpc::types::{Header, Transaction},
};
use chrono::{TimeZone, Utc};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols,
    text::{Line, Span, Text},
    widgets::{
        Bar, BarChart, BarGroup, Block, List, ListItem, Paragraph, Wrap,
    },
    Frame,
};

use crate::{
    db::Database,
    utils::{
        self, etherscan_block_url, etherscan_transaction_url, to_ether,
        to_gwei, useful_gas_price, BuilderIdentity,
    },
};

use super::components::stateful_list::StatefulList;

#[derive(Copy, Clone, Debug)]
pub enum View {
    Default,
    Block,
    Transaction,
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
    pub selected_transaction: alloy::rpc::types::Transaction,
}

impl App {
    pub fn new(
        title: String,
        selected_block: alloy::rpc::types::Block,
        selected_transaction: alloy::rpc::types::Transaction,
    ) -> Self {
        Self {
            title,
            selected_block,
            selected_transaction,
            block_headers: StatefulList::with_items(vec![]),
            transactions: StatefulList::with_items(vec![]),
            should_quit: false,
            view: View::default(),
        }
    }

    pub fn on_esc(&mut self) {
        match self.view {
            View::Default => self.should_quit = true,
            View::Block => self.view = View::Default,
            View::Transaction => self.view = View::Block,
        }
    }

    pub fn on_key(&mut self, c: char) {
        if c == 'q' {
            self.should_quit = true;
        }

        match self.view {
            View::Block => {
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
            View::Transaction => {
                if c == 'e' {
                    webbrowser::open(
                        etherscan_transaction_url(
                            self.selected_transaction
                                .clone()
                                .info()
                                .hash
                                .unwrap(),
                        )
                        .as_str(),
                    )
                    .unwrap()
                }
            }
            _ => {}
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
                    self.view = View::Transaction
                }
            }
            _ => {}
        }
    }

    pub fn on_up(&mut self) {
        match self.view {
            View::Default => self.block_headers.previous(),
            View::Block => self.transactions.previous(),
            View::Transaction => {}
        }
    }

    pub fn on_down(&mut self) {
        match self.view {
            View::Default => self.block_headers.next(),
            View::Block => self.transactions.next(),
            View::Transaction => {}
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

        if let Some(selected_tx) = self.get_selected_transaction() {
            if !matches!(self.view, View::Transaction) {
                self.selected_transaction = selected_tx.clone();
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
            View::Transaction => {
                let chunks = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .margin(1)
                .split(frame.area());
                self.draw_transaction_view(frame, chunks[1]);
            }
        }
    }

    fn draw_transaction_view(&mut self, frame: &mut Frame, area: Rect) {
        self.draw_transaction_header_text(frame, area);
    }

    fn draw_transaction_header_text(&mut self, frame: &mut Frame, area: Rect) {
        let tx = self.selected_transaction.clone();
        let timestamp = self.selected_block.header.timestamp;

        let chunks =
            Layout::vertical([Constraint::Percentage(20), Constraint::Min(0)])
                .split(area);

        let lines = vec![
            Line::from(Span::styled(
                format!("Transaction {}", tx.info().hash.unwrap()),
                Style::new().bold(),
            )),
            Line::from(vec![
                Span::styled("Timestamp: ", Style::new().bold()),
                Span::raw(format!(
                    "{} ({})",
                    Utc.timestamp_opt(timestamp as i64, 0).unwrap(),
                    timeago::Formatter::new()
                        .convert(utils::duration_since_timestamp(timestamp))
                )),
            ]),
            Line::from(vec![
                Span::styled("From: ", Style::new().bold()),
                Span::raw(format!("{}", tx.from)),
            ]),
            Line::from(vec![
                Span::styled("To:   ", Style::new().bold()),
                match tx.to() {
                    Some(addr) => Span::raw(format!("{}", addr)),
                    None => Span::raw(format!("{} (CREATE)", Address::ZERO)),
                },
            ]),
            Line::from(vec![
                Span::styled("Value: ", Style::new().bold()),
                Span::raw(format!("{} Ether", to_ether(tx.value()))),
            ]),
        ];
        let transaction_header_text = Paragraph::new(Text::from(lines));
        frame.render_widget(transaction_header_text, chunks[0]);
        self.draw_hex_display(tx.input(), frame, chunks[1]);
    }

    fn draw_block_view(&mut self, frame: &mut Frame, area: Rect) {
        let chunks =
            Layout::vertical([Constraint::Percentage(20), Constraint::Min(0)])
                .split(area);
        self.draw_block_header_text(frame, chunks[0]);
        self.draw_transactions_list(frame, chunks[1]);
    }

    fn draw_block_header_text(&mut self, frame: &mut Frame, area: Rect) {
        let block = &self.selected_block;
        let lines = vec![
            Line::from(vec![Span::styled(
                format!("Block #{} {}", block.header.number, block.header.hash),
                Style::default().bold(),
            )]),
            Line::from(vec![
                Span::styled("Timestamp: ", Style::new().bold()),
                Span::raw(format!(
                    "{} ({})",
                    Utc.timestamp_opt(block.header.timestamp as i64, 0)
                        .unwrap(),
                    timeago::Formatter::new().convert(
                        utils::duration_since_timestamp(block.header.timestamp)
                    )
                )),
            ]),
            Line::from(vec![
                Span::styled("Gas Usage (wei): ", Style::new().bold()),
                Span::raw(format!(
                    "{}  / {} ({:.2}%)",
                    block.header.gas_used,
                    block.header.gas_limit,
                    (block.header.gas_used as f64)
                        / (block.header.gas_limit as f64)
                        * 100.0
                )),
                Span::styled("        Base Fee (gwei): ", Style::new().bold()),
                Span::raw(format!(
                    " {:.3}",
                    to_gwei(block.header.base_fee_per_gas.unwrap_or_default()
                        as f64)
                )),
            ]),
            Line::from(vec![
                Span::styled("Beneficiary: ", Style::new().bold()),
                Span::raw(
                    match BuilderIdentity::from(block.header.extra_data.clone())
                    {
                        BuilderIdentity::Local => format!(
                            "{} (locally built)",
                            block.header.beneficiary
                        ),
                        iden => {
                            format!("{} ({})", block.header.beneficiary, iden)
                        }
                    },
                ),
            ]),
            Line::from(vec![
                Span::styled("State Root: ", Style::new().bold()),
                Span::raw(format!("{}", block.header.state_root)),
            ]),
            Line::from(vec![Span::raw(format!(
                "Contains {} transactions",
                block.transactions.len()
            ))]),
        ];
        let block_header_text = Paragraph::new(Text::from(lines));
        frame.render_widget(block_header_text, area);
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
                            to_gwei(
                                header.base_fee_per_gas.unwrap_or_default()
                                    as f64
                            )
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
                        "{:<16}",
                        format!(
                            "{}",
                            utils::shorten_hash(&tx_info.hash.unwrap())
                        )
                    )),
                    Span::raw(format!(
                        "{:<16}",
                        utils::shorten_address(&tx.from)
                    )),
                    Span::raw(format!(
                        "{:<16}",
                        utils::shorten_address(&tx.to().unwrap_or_default())
                    )),
                    Span::raw(format!("{:<8}", tx.nonce())),
                    Span::raw(format!(
                        "{:<4}",
                        if tx.to().is_none() {
                            "📄".to_string()
                        } else {
                            "".to_string()
                        }
                    )),
                    Span::raw(format!(
                        "{:<20}",
                        utils::human_readable_tx_data(tx.input().clone(),)
                    )),
                    Span::raw(format!(
                        "{:<20}",
                        format!(
                            "{:.3} gwei",
                            to_gwei(useful_gas_price(&tx) as f64),
                        )
                    )),
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

    fn draw_hex_display(
        &mut self,
        bytes: &Bytes,
        frame: &mut Frame,
        area: Rect,
    ) {
        frame.render_widget(
            Paragraph::new(format!("{}", bytes)).wrap(Wrap { trim: true }),
            area,
        );
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
