use anyhow::Context;
use clap::Parser;
use log::LevelFilter;
use simplelog::{ColorChoice, ConfigBuilder, TerminalMode};

use crate::logic;

#[derive(Parser)]
#[command(author, version, about)]
pub enum Cli {
    /// List all installed crates
    List(Opts),
    /// List crates with newer versions available
    ListUpdates(Opts),
    /// Update all crates which are outdated
    Update(Opts),
    /// Initialize everything to use this crate
    Init(Opts),
}

#[derive(Parser)]
pub struct Opts {
    /// Verbosity level
    #[arg(short, long, default_value_t = LevelFilter::Info)]
    pub filter: LevelFilter,
}

impl Cli {
    const fn needs_binlist(&self) -> bool {
        !matches!(self, Self::Init(_))
    }

    const fn get_filter(&self) -> LevelFilter {
        match self {
            Self::List(opts) | Self::ListUpdates(opts) | Self::Update(opts) | Self::Init(opts) => {
                opts.filter
            }
        }
    }

    /// Run the CLI
    pub fn run(&self) -> anyhow::Result<()> {
        initialize_logger(self.get_filter())?;
        let packages = if self.needs_binlist() {
            let bin_list = logic::get_installed_bins()?;
            Some(logic::get_package_infos(&bin_list))
        } else {
            None
        };

        match self {
            Self::List(_) => {
                logic::list_pkgs(packages.context("Failed to get installed binaries")?, false);
            }
            Self::ListUpdates(_) => {
                logic::list_pkgs(packages.context("Failed to get installed binaries")?, true);
            }
            Self::Update(_) => {
                let packages = packages.context("Failed to get installed binaries")?;
                logic::version_occurrences(&packages);
                logic::update(&packages)?;
            }
            Self::Init(_) => logic::init()?,
        }

        Ok(())
    }
}
/// Initialize the logger
pub fn initialize_logger(verbose: LevelFilter) -> anyhow::Result<()> {
    let filter = if cfg!(debug_assertions) {
        LevelFilter::max()
    } else {
        verbose
    };
    if !log::log_enabled!(filter.to_level().context("Failed to get log level")?) {
        return simplelog::TermLogger::init(
            filter,
            ConfigBuilder::new()
                // suppress all logs from dependencies
                .add_filter_allow_str("cargo_binlist")
                .build(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        )
        .context("Failed to initialize logger");
    }
    Ok(())
}
