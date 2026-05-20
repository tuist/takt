mod cli;
mod domain;
mod output;

use clap::Parser;
use color_eyre::eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;
    cli::Cli::parse().run()
}
