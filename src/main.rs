use clap::Parser;
use color_eyre::eyre;

use crate::{app::App, cli::Cli};

mod app;
mod cli;
mod parse;
mod walk;

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();
    color_eyre::install()?;

    let mut app = App::new(cli);
    ratatui::run(|terminal| app.run(terminal))?;

    Ok(())
}
