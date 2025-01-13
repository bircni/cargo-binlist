use clap::Parser;

use crate::{data::VersionCheck, utils};

#[derive(Parser)]
#[command(author, version, about)]
/// Check for outdated crates and update them
pub struct Cli {
    /// dry run
    #[clap(short, long = "dry-run")]
    pub(crate) dry_run: bool,
}

impl Cli {
    /// Run the CLI
    pub fn run(&self) -> anyhow::Result<()> {
        utils::initialize_logger()?;

        let cargo_bins = utils::get_installed_bins()?;
        let packages = utils::get_package_infos(&cargo_bins);
        utils::version_occurrences(&packages);

        if self.dry_run {
            log::info!("Dry run enabled, not updating packages");
            return Ok(());
        }

        if packages
            .iter()
            .any(|pkg| matches!(pkg.info, VersionCheck::NewerAvailable))
        {
            log::info!("Updating packages");
            utils::update(&packages)?;
        } else {
            log::info!("No packages to update");
            return Ok(());
        }

        Ok(())
    }
}
