
use clap::Parser;
use log::LevelFilter;

use crate::utils;

#[derive(Parser)]
#[command(author, version, about)]
/// Check for outdated crates and update them
pub struct Cli {
    /// List all installed crates
    #[clap(short, long)]
    pub(crate) list: bool,
    /// List crates with newer versions available
    #[clap(short = 'n', long)]
    pub(crate) list_updates: bool,
    /// Update all crates
    #[clap(short, long)]
    pub(crate) update: bool,
    /// Verbose mode [Debug, Info, Error, Warn]
    #[clap(short = 'v', long, default_value = "Info")]
    pub(crate) verbose: LevelFilter,
}

impl Cli {
    /// Run the CLI
    pub fn run(&self) -> anyhow::Result<()> {
        utils::initialize_logger(self.verbose)?;

        if self.update {
            let cargo_bins = utils::get_installed_bins()?;
            let packages = utils::get_package_infos(&cargo_bins);

            utils::version_occurrences(&packages);
            log::debug!("Updating packages");
            utils::update(&packages)?;
        } else if self.list_updates || self.list {
            let cargo_bins = utils::get_installed_bins()?;
            let packages = utils::get_package_infos(&cargo_bins);

            utils::list_pkgs(packages, self.list_updates);
        } else {
            println!("No action specified");
        }

        Ok(())
    }
}
