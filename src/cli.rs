use clap::Parser;
use log::LevelFilter;

use crate::utils;

#[expect(
    clippy::struct_excessive_bools,
    reason = "This struct is used to parse CLI arguments"
)]
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
    /// Log Level Filter [Debug, Info, Error, Warn]
    #[clap(short = 'f', long = "loglevel", default_value = "Info")]
    pub(crate) filter: LevelFilter,
    /// Initialize everything to use this crate
    #[clap(short, long)]
    pub(crate) init: bool,
}

impl Cli {
    /// Run the CLI
    pub fn run(&self) -> anyhow::Result<()> {
        utils::initialize_logger(self.filter)?;

        if self.init {
            utils::init()?;
        } else if self.update {
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
