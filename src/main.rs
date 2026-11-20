//! `cargo binlist`: list and update binaries installed with `cargo install`.

use std::{env, process::ExitCode};

use clap::Parser as _;

use crate::cli::Cli;

mod cli;
mod install;
mod output;
mod package;
mod registry;
mod update;

fn main() -> ExitCode {
    let cli = Cli::parse_from(cli::normalize_args(env::args_os()));

    env_logger::Builder::new()
        .filter_module(env!("CARGO_CRATE_NAME"), cli.verbosity.log_level_filter())
        .format_timestamp(None)
        .format_target(false)
        .init();

    match cli.run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
