use core::cmp::Ordering;

use clap::Parser;
use log::LevelFilter;

use crate::utils;

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    /// The log level to use
    #[clap(short, long, default_value = "info")]
    loglevel: LevelFilter,
}

impl Cli {
    pub fn run(&self) -> anyhow::Result<()> {
        utils::initialize_logger(self.loglevel)?;

        let cargo_bins = utils::get_installed_bins()?;
        let mut newer_available = Vec::new();
        let mut local_newer = 0;
        let mut uptodate = 0;

        log::info!("Installed Cargo packages and their versions:");
        for (package, version, latest) in utils::parse_cargo_list_output(&cargo_bins) {
            match version.cmp(&latest) {
                Ordering::Less => {
                    newer_available.push((package, version, latest));
                }
                Ordering::Greater => {
                    local_newer += 1;
                }
                Ordering::Equal => {
                    uptodate += 1;
                }
            }
        }
        log::info!("Results:");
        if uptodate > 0 {
            log::info!("{} packages are up-to-date", uptodate);
        }
        if local_newer > 0 {
            log::info!("{} packages are newer than the latest version", local_newer);
        }
        if !newer_available.is_empty() {
            log::info!(
                "{} packages have newer versions available:",
                newer_available.len()
            );
            for (package, version, latest_version) in newer_available {
                log::info!("{}: {} (latest: {})", package, version, latest_version);
            }
        }

        Ok(())
    }
}
