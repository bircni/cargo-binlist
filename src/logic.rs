use std::process::Command;

use anyhow::Context as _;
use comfy_table::{modifiers, presets, Attribute, Cell, ContentArrangement, Table};
use dialoguer::Confirm;
use itertools::Itertools as _;
use rayon::iter::{IntoParallelRefMutIterator, ParallelBridge, ParallelIterator as _};
use semver::Version;
use which::which;

use crate::data::{PackageInfo, VersionCheck};

pub fn init() -> anyhow::Result<()> {
    if which("cargo-binstall").is_ok() {
        log::info!("cargo-binstall is already installed - aborting");
        return Ok(());
    }
    log::info!("cargo-binstall is not installed");
    if !Confirm::new()
        .with_prompt("Do you want to install cargo-binstall now?")
        .default(false)
        .interact()?
    {
        log::info!("Aborting installation");
        return Ok(());
    }
    log::info!("Installing cargo-binstall now - this may take a while");

    let command = Command::new("cargo")
        .arg("install")
        .arg("cargo-binstall")
        .spawn()?;

    let output = command.wait_with_output()?;

    if output.status.success() {
        log::info!("cargo-binstall installed successfully");
    } else {
        anyhow::bail!(
            "Failed to install cargo-binstall: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

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
        "{up_to_date_count}/{} packages are up-to-date",
        packages.len()
    );
}

/// Helper function to parse a package line (e.g., "cargo-binstall v1.10.18:")
pub fn parse_package_line(line: &str) -> anyhow::Result<PackageInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() == 2 || parts.len() == 3 {
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
    anyhow::bail!("Failed to parse package line: {line}")
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
pub fn update(pkgs: &[PackageInfo], no_confirm: bool) -> anyhow::Result<()> {
    let packages = pkgs
        .iter()
        .filter(|pkg| matches!(pkg.info, VersionCheck::NewerAvailable(_)))
        .map(|pkg| pkg.name.clone())
        .collect::<Vec<String>>();

    if packages.is_empty() {
        log::info!("No packages to update");
        return Ok(());
    }

    log::info!("Updating packages: {}", packages.join(", "));

    if !no_confirm {
        // ask for confirmation
        if !Confirm::new()
            .with_prompt("Do you want to continue?")
            .default(false)
            .interact()?
        {
            log::info!("Aborting update");
            return Ok(());
        }
    }

    if pkgs.iter().any(|pkg| pkg.name == "cargo-binstall") {
        log::info!("Using cargo-binstall to update packages");
        let output = Command::new("cargo")
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

pub fn list_pkgs(pkg_vec: Vec<PackageInfo>, only_updates: bool, uncondensed: bool) {
    if pkg_vec.is_empty() {
        log::info!("No packages found");
        return;
    }
    let len = pkg_vec.len();
    let table = if only_updates {
        let res = pkg_vec
            .into_iter()
            .filter(|pkg| matches!(pkg.info, VersionCheck::NewerAvailable(_)))
            .collect::<Vec<PackageInfo>>();
        if res.is_empty() {
            log::info!("No packages with newer versions found");
            return;
        }
        log::info!("Packages with newer versions:");

        create_table(&res, uncondensed)
    } else {
        log::info!("All installed packages:");
        create_table(&pkg_vec, uncondensed)
    };

    println!("{table}");
    println!("Count: {len}");
}

fn create_table(pkgs: &[PackageInfo], uncondensed: bool) -> Table {
    let header = vec![
        Cell::new("Name").add_attribute(Attribute::Bold),
        Cell::new("Version").add_attribute(Attribute::Bold),
        Cell::new("Status").add_attribute(Attribute::Bold),
    ];

    let rows = pkgs
        .iter()
        .sorted_by(|a, b| {
            // Show packages with updates first, then sort by name
            let a_update = matches!(a.info, VersionCheck::NewerAvailable(_));
            let b_update = matches!(b.info, VersionCheck::NewerAvailable(_));
            b_update.cmp(&a_update).then_with(|| a.name.cmp(&b.name))
        })
        .map(|pkg| {
            vec![
                Cell::new(&pkg.name),
                Cell::new(&pkg.version),
                pkg.info.colored_cell(),
            ]
        })
        .collect::<Vec<_>>();

    let preset = if uncondensed {
        presets::UTF8_FULL
    } else {
        presets::UTF8_FULL_CONDENSED
    };
    let mut table = Table::new();
    table
        .load_preset(preset)
        .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(header)
        .add_rows(rows);

    table
}
