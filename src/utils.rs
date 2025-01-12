use std::{process::Command, time::Duration};

use anyhow::Context as _;
use crates_io_api::SyncClient;
use log::LevelFilter;
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use semver::Version;
use simplelog::{ColorChoice, ConfigBuilder, TerminalMode};

/// Function to parse the output of `cargo install --list`
pub fn parse_cargo_list_output(output: &str) -> Vec<(String, Version, Version)> {
    let lines = output.lines().collect::<Vec<_>>();

    lines
        .par_iter()
        .filter_map(|line| {
            if let Ok((name, version)) = parse_package_line(line) {
                if let Ok(latest) = get_latest_version(&name) {
                    return Some((name, version, latest));
                }
            }
            None
        })
        .collect()
}

/// Helper function to parse a package line (e.g., "cargo-binstall v1.10.18:")
fn parse_package_line(line: &str) -> anyhow::Result<(String, Version)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() == 2
        && parts
            .get(1)
            .context("Could not split parts correctly")?
            .ends_with(':')
    {
        let package = (*parts.first().context("Could not split parts correctly")?).to_owned();
        let version = Version::parse(
            &parts
                .get(1)
                .context("Could not split parts correctly")?
                .trim_end_matches(':')
                .replace('v', ""),
        )?;

        return Ok((package, version));
    }
    anyhow::bail!("Invalid package line: {}", line)
}

pub fn get_latest_version(crate_name: &str) -> anyhow::Result<Version> {
    let client = SyncClient::new(
        "cargo-binlist (help@my_bot.com)",
        Duration::from_millis(1000),
    )?;
    log::debug!("Fetching latest version for {}", crate_name);
    let cr = client.get_crate(crate_name)?;
    let version = Version::parse(&cr.crate_data.max_version)?;
    Ok(version)
}

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

pub fn initialize_logger(log_level: LevelFilter) -> anyhow::Result<()> {
    simplelog::TermLogger::init(
        log_level,
        ConfigBuilder::new()
            // suppress all logs from dependencies
            .add_filter_allow_str("cargo_binlist")
            .build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .context("Failed to initialize logger")
}
