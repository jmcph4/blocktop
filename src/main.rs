use clap::Parser;
use crossterm::event::{self, Event};
use ratatui::{DefaultTerminal, Frame};

use crate::cli::Opts;

pub mod cli;

fn run(mut terminal: DefaultTerminal) -> eyre::Result<()> {
    loop {
        terminal.draw(render)?;
        if matches!(event::read()?, Event::Key(_)) {
            break Ok(());
        }
    }
}

fn render(frame: &mut Frame) {
    frame.render_widget("hello world", frame.area());
}

fn main() -> eyre::Result<()> {
    pretty_env_logger::init_timed();
    let _opts: Opts = Opts::parse();

    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();

    result
}
