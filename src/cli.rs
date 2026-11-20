//! Command line interface.

use std::{
    ffi::OsString,
    io::{self, IsTerminal as _},
    path::PathBuf,
    process::{Command as Process, ExitCode},
};

use anyhow::Context as _;
use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::{Verbosity, WarnLevel};
use dialoguer::Confirm;

use crate::{
    install, output,
    package::{self, Report},
    registry::{self, SparseIndex},
    update::{self, Installer},
};

#[derive(Debug, Parser)]
#[command(name = "cargo-binlist", bin_name = "cargo binlist", version, about)]
pub struct Cli {
    #[command(flatten)]
    pub verbosity: Verbosity<WarnLevel>,

    #[arg(
        long,
        global = true,
        value_name = "DIR",
        help = "Install root to inspect [default: $CARGO_INSTALL_ROOT, $CARGO_HOME or ~/.cargo]"
    )]
    pub root: Option<PathBuf>,

    /// Also consider prerelease versions as updates
    #[arg(long, global = true)]
    pub pre: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List installed binaries and whether updates are available (default)
    List(ListArgs),
    /// Exit with status 1 if any installed binary is outdated
    Check(CheckArgs),
    /// Update outdated binaries
    Update(UpdateArgs),
    /// Install cargo-binstall to update using prebuilt binaries
    Init(InitArgs),
}

#[derive(Debug, Default, Args)]
pub struct ListArgs {
    /// Only show packages with available updates
    #[arg(short, long)]
    pub outdated: bool,
    /// Print JSON instead of a table
    #[arg(long, conflicts_with = "uncondensed")]
    pub json: bool,
    /// Separate table rows with lines
    #[arg(short, long)]
    pub uncondensed: bool,
}

#[derive(Debug, Args)]
pub struct CheckArgs {
    /// Print JSON instead of a table
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Only update these packages [default: all outdated packages]
    #[arg(value_name = "PACKAGE")]
    pub packages: Vec<String>,
    /// Do not update these packages
    #[arg(short = 'x', long, value_name = "PACKAGE")]
    pub exclude: Vec<String>,
    /// Show what would be updated without doing it
    #[arg(short = 'n', long)]
    pub dry_run: bool,
    /// Do not ask for confirmation
    #[arg(short, long)]
    pub yes: bool,
}

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Do not ask for confirmation
    #[arg(short, long)]
    pub yes: bool,
}

/// Drop the `binlist` argument cargo inserts when invoked as `cargo binlist`.
///
/// Only the first argument is removed, so a package called `binlist` can still
/// be passed to `update`.
pub fn normalize_args(args: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    let mut args: Vec<_> = args.into_iter().collect();
    if args.get(1).is_some_and(|arg| arg == "binlist") {
        args.remove(1);
    }
    args
}

impl Cli {
    pub fn run(self) -> anyhow::Result<ExitCode> {
        match self.command {
            None => self.list(&ListArgs::default()),
            Some(Command::List(ref args)) => self.list(args),
            Some(Command::Check(ref args)) => self.check(args),
            Some(Command::Update(ref args)) => self.update(args),
            Some(Command::Init(ref args)) => init(args),
        }
    }

    fn reports(&self) -> anyhow::Result<Vec<Report>> {
        let root = install::install_root(self.root.as_deref())?;
        log::info!("Reading installed packages from {}", root.display());
        let packages = install::load(&root)?;

        let mut reports = registry::check_all(packages, &SparseIndex::crates_io(), self.pre);
        for report in &reports {
            if let package::Status::Error { message } = &report.status {
                log::warn!("Could not check {}: {message}", report.package.name);
            }
        }
        package::sort_reports(&mut reports);
        Ok(reports)
    }

    fn list(&self, args: &ListArgs) -> anyhow::Result<ExitCode> {
        let reports = self.reports()?;
        let shown: Vec<_> = reports
            .iter()
            .filter(|r| !args.outdated || r.is_outdated())
            .collect();

        if args.json {
            println!("{}", output::json(&shown)?);
        } else if reports.is_empty() {
            println!("No packages installed");
        } else if shown.is_empty() {
            println!("All packages are up to date");
        } else {
            println!("{}", output::table(&shown, args.uncondensed));
            println!("{}", output::summary(&reports));
        }
        Ok(ExitCode::SUCCESS)
    }

    fn check(&self, args: &CheckArgs) -> anyhow::Result<ExitCode> {
        let reports = self.reports()?;
        let outdated: Vec<_> = reports.iter().filter(|r| r.is_outdated()).collect();

        if args.json {
            println!("{}", output::json(&outdated)?);
        } else if outdated.is_empty() {
            println!("All packages are up to date");
        } else {
            println!("{}", output::table(&outdated, false));
            println!("{}", output::summary(&reports));
        }
        Ok(if outdated.is_empty() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        })
    }

    fn update(&self, args: &UpdateArgs) -> anyhow::Result<ExitCode> {
        let reports = self.reports()?;
        let selected = update::select(&reports, &args.packages, &args.exclude)?;
        if selected.is_empty() {
            println!("All packages are up to date");
            return Ok(ExitCode::SUCCESS);
        }

        let installer = Installer::detect(self.root.clone());
        if !installer.binstall {
            log::warn!(
                "cargo-binstall not found, building from source (run `cargo binlist init` for faster updates)"
            );
        }
        let updates = installer.plan(&selected);

        println!("Packages to update:");
        for update in &updates {
            println!(
                "  {} {} -> {} ({})",
                update.package.name, update.package.version, update.target, update.method
            );
        }

        if args.dry_run || !(args.yes || confirm("Update these packages?")?) {
            return Ok(ExitCode::SUCCESS);
        }

        let failed = installer.execute(&updates);
        if !failed.is_empty() {
            anyhow::bail!("Failed to update: {}", failed.join(", "));
        }
        println!(
            "Updated {} {}",
            updates.len(),
            if updates.len() == 1 {
                "package"
            } else {
                "packages"
            }
        );
        Ok(ExitCode::SUCCESS)
    }
}

fn init(args: &InitArgs) -> anyhow::Result<ExitCode> {
    let installer = Installer::detect(None);
    if installer.binstall {
        println!("cargo-binstall is already installed");
        return Ok(ExitCode::SUCCESS);
    }
    if !(args.yes || confirm("cargo-binstall is not installed. Install it now?")?) {
        return Ok(ExitCode::SUCCESS);
    }

    let status = Process::new(&installer.cargo)
        .args(["install", "--locked", "cargo-binstall"])
        .status()
        .context("Failed to run cargo")?;
    if !status.success() {
        anyhow::bail!("Installing cargo-binstall failed ({status})");
    }
    println!("cargo-binstall installed successfully");
    Ok(ExitCode::SUCCESS)
}

fn confirm(prompt: &str) -> anyhow::Result<bool> {
    if !io::stdin().is_terminal() {
        anyhow::bail!("Cannot ask for confirmation in a non-interactive session; pass --yes");
    }
    Ok(Confirm::new()
        .with_prompt(prompt)
        .default(false)
        .interact()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory as _;

    fn os(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn strips_cargo_subcommand_name() {
        assert_eq!(
            normalize_args(os(&["cargo-binlist", "binlist", "list"])),
            os(&["cargo-binlist", "list"])
        );
        assert_eq!(
            normalize_args(os(&["cargo-binlist", "list"])),
            os(&["cargo-binlist", "list"])
        );
    }

    #[test]
    fn keeps_package_named_binlist() {
        // Regression: every argument equal to `binlist` used to be removed.
        assert_eq!(
            normalize_args(os(&["cargo-binlist", "binlist", "update", "binlist"])),
            os(&["cargo-binlist", "update", "binlist"])
        );
    }

    #[test]
    fn defaults_to_list() {
        let cli = Cli::parse_from(["cargo-binlist"]);
        assert!(cli.command.is_none(), "no subcommand expected");
    }

    #[test]
    fn parses_update_selection() {
        let cli = Cli::parse_from([
            "cargo-binlist",
            "update",
            "a",
            "b",
            "-x",
            "c",
            "--yes",
            "--pre",
        ]);
        let Some(Command::Update(args)) = cli.command else {
            panic!("expected update command");
        };
        assert_eq!(args.packages, ["a", "b"]);
        assert_eq!(args.exclude, ["c"]);
        assert!(args.yes && !args.dry_run, "flags parsed incorrectly");
        assert!(cli.pre, "global flag after subcommand");
    }
}
