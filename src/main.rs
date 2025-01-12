use std::{
    env,
    process::{self},
};

use clap::Parser as _;
use cli::Cli;

mod cli;
#[cfg(test)]
mod test;
mod utils;

fn main() {
    match real_main() {
        Ok(()) => {}
        Err(e) => {
            log::error!("{:#}", e);
            process::exit(1);
        }
    }
}

fn real_main() -> anyhow::Result<()> {
    Cli::parse_from(env::args().filter(|a| a != "binlist")).run()?;

    Ok(())
}
