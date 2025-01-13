use std::process::{self, Command};

use anyhow::Context as _;
use log::LevelFilter;
use rayon::iter::{IntoParallelRefMutIterator, ParallelBridge, ParallelIterator as _};
use semver::Version;
use simplelog::{ColorChoice, ConfigBuilder, TerminalMode};

use crate::data::{PackageInfo, VersionCheck};

/// Function to parse the output of `cargo install --list`
pub fn get_package_infos(output: &str) -> Vec<PackageInfo> {
    let lines = output.lines();

    let mut packages: Vec<PackageInfo> = lines
        .par_bridge()
        .filter_map(|line| parse_package_line(line).ok())
        .collect();

    packages.par_iter_mut().for_each(|pkg| {
        if let Ok(latest) = pkg.latest_version() {
            pkg.set_info(&latest);
        }
    });

    packages
}

/// Function to print the version occurrences
pub fn version_occurrences(packages: &[PackageInfo]) {
    let mut local_newer_count = 0;
    let mut newer_available_count = 0;
    let mut unavailable_count = 0;
    let mut up_to_date_count = 0;

    for package in packages {
        match &package.info {
            VersionCheck::LocalNewer => local_newer_count += 1,
            VersionCheck::NewerAvailable => {
                newer_available_count += 1;
                log::debug!("{} has a newer version available", package.name);
            }
            VersionCheck::UnAvailable => unavailable_count += 1,
            VersionCheck::UpToDate => up_to_date_count += 1,
        }
    }

    log::info!("Results:");
    log::info!("{} packages total", packages.len());
    if local_newer_count > 0 {
        log::info!("{local_newer_count} packages are newer than the latest version");
    }
    if unavailable_count > 0 {
        log::info!("{unavailable_count} packages could not be found");
    }
    if up_to_date_count > 0 {
        log::info!("{up_to_date_count} packages are up-to-date");
    }
    if newer_available_count > 0 {
        log::info!("{newer_available_count} packages have newer versions available");
    }
}

/// Helper function to parse a package line (e.g., "cargo-binstall v1.10.18:")
pub fn parse_package_line(line: &str) -> anyhow::Result<PackageInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() == 2
        && parts
            .get(1)
            .context("Could not split parts correctly")?
            .ends_with(':')
    {
        let name = (*parts.first().context("Could not split parts correctly")?).to_owned();
        let version = Version::parse(
            &parts
                .get(1)
                .context("Could not split parts correctly")?
                .trim_end_matches(':')
                .replace('v', ""),
        )?;

        return Ok(PackageInfo::new(name, version));
    }
    anyhow::bail!("Invalid package line: {}", line)
}

/// Get the installed binaries
pub fn get_installed_bins() -> anyhow::Result<String> {
    let output = Command::new("cargo")
        .arg("install")
        .arg("--list")
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to get installed binaries: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Update the packages
pub fn update(pkgs: &[PackageInfo]) -> anyhow::Result<()> {
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
        let output = process::Command::new("cargo")
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

/// Initialize the logger
pub fn initialize_logger() -> anyhow::Result<()> {
    simplelog::TermLogger::init(
        #[cfg(debug_assertions)]
        LevelFilter::max(),
        #[cfg(not(debug_assertions))]
        LevelFilter::Info,
        ConfigBuilder::new()
            // suppress all logs from dependencies
            .add_filter_allow_str("cargo-verset")
            .build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .context("Failed to initialize logger")
}
