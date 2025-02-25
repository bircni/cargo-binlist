use std::process::{self, Command};

use anyhow::Context as _;
use comfy_table::{Attribute, Cell, ContentArrangement, Table, modifiers, presets};
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

    packages.sort_by(|a, b| a.info.cmp(&b.info));

    packages
}

/// Function to print the version occurrences
pub fn version_occurrences(packages: &[PackageInfo]) {
    let mut up_to_date_count = 0;

    for package in packages {
        match &package.info {
            VersionCheck::NewerAvailable(_) => {
                log::debug!("{} has a newer version available", package.name);
            }
            VersionCheck::UpToDate => up_to_date_count += 1,
            VersionCheck::LocalNewer | VersionCheck::UnAvailable => {}
        }
    }

    log::info!(
        "{}/{} packages are up-to-date",
        up_to_date_count,
        packages.len()
    );
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
    let mut packages = pkgs
        .iter()
        .filter(|pkg| matches!(pkg.info, VersionCheck::NewerAvailable(_)))
        .map(|pkg| pkg.name.clone())
        .collect::<Vec<String>>();

    // filter out the cargo-binstall package as we cannot update cargo-binstall using itself
    if packages.contains(&"cargo-binstall".to_owned()) {
        println!("cargo-binstall cannot update itself, please update it manually");
        packages.retain(|pkg| pkg != "cargo-binstall");
    }

    if packages.is_empty() {
        log::info!("No packages to update");
        return Ok(());
    }

    if pkgs.iter().any(|pkg| pkg.name == "cargo-binstall") {
        log::info!("Using cargo-binstall to update packages");
        let output = process::Command::new("cargo")
            .arg("binstall")
            .args(&packages)
            .arg("-y")
            .output()?;

        if output.status.success() {
            log::info!("Packages updated successfully: {}", packages.join(", "));
        } else {
            log::error!("{}", String::from_utf8_lossy(&output.stderr));
            log::error!("{}", String::from_utf8_lossy(&output.stdout));
            anyhow::bail!("Failed to update packages");
        }
    } else {
        log::info!("Not updating packages");
        log::info!("cargo-binstall is not installed");
        log::info!("Run `cargo install cargo-binstall` to install it");
    }

    Ok(())
}

pub fn list_pkgs(pkg_vec: Vec<PackageInfo>, only_updates: bool) {
    if pkg_vec.is_empty() {
        log::info!("No packages found");
        return;
    }
    let table = if only_updates {
        let res = pkg_vec
            .into_iter()
            .filter(|pkg| matches!(pkg.info, VersionCheck::NewerAvailable(_)))
            .collect::<Vec<PackageInfo>>();
        if res.is_empty() {
            println!("No packages with newer versions found");
            return;
        }
        println!("Packages with newer versions:");

        create_table(&res)
    } else {
        println!("All installed packages:");
        create_table(&pkg_vec)
    };

    println!("{table}");
}

fn create_table(pkgs: &[PackageInfo]) -> Table {
    let header = vec![
        Cell::new("Name").add_attribute(Attribute::Bold),
        Cell::new("Version").add_attribute(Attribute::Bold),
        Cell::new("Status").add_attribute(Attribute::Bold),
    ];

    let rows = pkgs
        .iter()
        .map(|pkg| {
            vec![
                Cell::new(&pkg.name),
                Cell::new(&pkg.version),
                pkg.info.colored_cell(),
            ]
        })
        .collect::<Vec<_>>();

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(header)
        .add_rows(rows);

    table
}

/// Initialize the logger
pub fn initialize_logger(verbose: LevelFilter) -> anyhow::Result<()> {
    let filter = if cfg!(debug_assertions) {
        LevelFilter::max()
    } else {
        verbose
    };
    simplelog::TermLogger::init(
        filter,
        ConfigBuilder::new()
            // suppress all logs from dependencies
            .add_filter_allow_str("cargo_binlist")
            .build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .context("Failed to initialize logger")
}
