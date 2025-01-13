use clap::Parser;

use crate::{
    data::{PackageInfo, VersionCheck},
    utils,
};

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    /// dry run
    #[clap(short, long = "dry-run")]
    pub(crate) dry_run: bool,
}

impl Cli {
    pub fn run(&self) -> anyhow::Result<()> {
        utils::initialize_logger()?;

        let cargo_bins = utils::get_installed_bins()?;
        let packages = utils::get_package_infos(&cargo_bins);
        utils::version_occurences(&packages);

        if self.dry_run {
            log::info!("Dry run enabled, not updating packages");
            return Ok(());
        }

        if packages
            .iter()
            .any(|pkg| matches!(pkg.info, VersionCheck::NewerAvailable))
        {
            log::info!("Updating packages");
            Self::update(&packages)?;
        } else {
            log::info!("No packages to update");
            return Ok(());
        }

        Ok(())
    }

    pub(crate) fn update(pkgs: &[PackageInfo]) -> anyhow::Result<()> {
        let mut string = pkgs
            .iter()
            .filter(|pkg| matches!(pkg.info, VersionCheck::NewerAvailable))
            .map(|pkg| pkg.name.clone())
            .collect::<Vec<String>>()
            .join(" ");

        // filter out the cargo-binstall package as we cannot update cargo-binstall using itself
        if string.contains("cargo-binstall") {
            log::warn!("cargo-binstall cannot update itself, please update it manually");
            string
                .replace("cargo-binstall", "")
                .trim()
                .clone_into(&mut string);
        }

        if string.is_empty() {
            log::info!("No packages to update");
            return Ok(());
        }

        if pkgs.iter().any(|pkg| pkg.name == "cargo-binstall") {
            log::info!("Using cargo-binstall to update packages");
            let output = std::process::Command::new("cargo")
                .arg("binstall")
                .arg(string)
                .arg("-y")
                .output()?;

            if output.status.success() {
                log::info!("{}", String::from_utf8_lossy(&output.stdout));
            } else {
                log::error!("{}", String::from_utf8_lossy(&output.stderr));
                anyhow::bail!("Failed to update packages");
            }
        } else {
            log::info!("Not updating packages");
            log::info!("cargo-binstall is not installed");
            log::info!("Run `cargo install cargo-binstall` to install it");
        }

        Ok(())
    }
}
