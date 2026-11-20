//! Selecting and installing updates.

use std::{
    ffi::{OsStr, OsString},
    fmt,
    path::PathBuf,
    process::Command,
};

use anyhow::Context as _;
use semver::Version;

use crate::package::{InstalledPackage, Report};

/// How a package gets installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// `cargo binstall`, downloading prebuilt binaries where available
    Binstall,
    /// `cargo install`, building from source
    CargoInstall,
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Binstall => "cargo binstall",
            Self::CargoInstall => "cargo install",
        })
    }
}

/// A package that is going to be updated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update<'a> {
    pub package: &'a InstalledPackage,
    pub target: &'a Version,
    pub method: Method,
}

/// Runs `cargo install` / `cargo binstall`.
#[derive(Debug, Clone)]
pub struct Installer {
    /// The cargo executable
    pub cargo: OsString,
    /// Whether `cargo-binstall` is available
    pub binstall: bool,
    /// Custom install root, passed on as `--root`
    pub root: Option<PathBuf>,
}

impl Installer {
    /// Detect the cargo executable and whether `cargo-binstall` is installed.
    pub fn detect(root: Option<PathBuf>) -> Self {
        Self {
            // Set by cargo when running as `cargo binlist`
            cargo: std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()),
            binstall: which::which("cargo-binstall").is_ok(),
            root,
        }
    }

    /// Pick the install method. Packages built with custom features are
    /// rebuilt from source, since prebuilt binaries would lose those features.
    pub const fn method_for(&self, package: &InstalledPackage) -> Method {
        if self.binstall && !package.has_custom_features() {
            Method::Binstall
        } else {
            Method::CargoInstall
        }
    }

    /// Plan updates for the given packages.
    pub fn plan<'a>(&self, outdated: &[(&'a InstalledPackage, &'a Version)]) -> Vec<Update<'a>> {
        outdated
            .iter()
            .map(|&(package, target)| Update {
                package,
                target,
                method: self.method_for(package),
            })
            .collect()
    }

    /// The command that installs `update`.
    pub fn command(&self, update: &Update<'_>) -> Command {
        let package = update.package;
        let mut cmd = Command::new(&self.cargo);
        match update.method {
            Method::Binstall => {
                cmd.args(["binstall", "--no-confirm"]);
            }
            Method::CargoInstall => {
                cmd.args(["install", "--locked"]);
                if !package.features.is_empty() {
                    cmd.arg("--features").arg(package.features.join(","));
                }
                if package.all_features {
                    cmd.arg("--all-features");
                }
                if package.no_default_features {
                    cmd.arg("--no-default-features");
                }
                if let Some(profile) = &package.profile {
                    cmd.arg("--profile").arg(profile);
                }
            }
        }
        if let Some(root) = &self.root {
            cmd.arg("--root").arg(root);
        }
        cmd.arg(format!("{}@{}", package.name, update.target));
        cmd
    }

    /// Install all updates one after another, streaming cargo's output.
    /// Returns the names of the packages that failed.
    pub fn execute(&self, updates: &[Update<'_>]) -> Vec<String> {
        let mut failed = Vec::new();
        for update in updates {
            eprintln!(
                "Updating {} {} -> {} ({})",
                update.package.name, update.package.version, update.target, update.method
            );
            let result = self
                .command(update)
                .status()
                .with_context(|| format!("Failed to run {}", OsStr::new(&self.cargo).display()));
            match result {
                Ok(status) if status.success() => {}
                Ok(status) => {
                    log::error!("Updating {} failed ({status})", update.package.name);
                    failed.push(update.package.name.clone());
                }
                Err(e) => {
                    log::error!("{e:#}");
                    failed.push(update.package.name.clone());
                }
            }
        }
        failed
    }
}

/// Select the outdated packages to update.
///
/// `only` restricts the update to the named packages (all of which must be
/// installed); `exclude` removes packages from the selection.
pub fn select<'a>(
    reports: &'a [Report],
    only: &[String],
    exclude: &[String],
) -> anyhow::Result<Vec<(&'a InstalledPackage, &'a Version)>> {
    for name in only.iter().chain(exclude) {
        if !reports.iter().any(|r| &r.package.name == name) {
            anyhow::bail!("Package `{name}` is not installed");
        }
    }

    Ok(reports
        .iter()
        .filter(|r| only.is_empty() || only.contains(&r.package.name))
        .filter(|r| !exclude.contains(&r.package.name))
        .filter_map(|r| r.update_target().map(|target| (&r.package, target)))
        .collect())
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests")]

    use super::*;
    use crate::package::Status;

    fn report(name: &str, status: Status) -> Report {
        Report {
            package: InstalledPackage::new(name, Version::new(1, 0, 0)),
            status,
        }
    }

    fn outdated(name: &str) -> Report {
        report(
            name,
            Status::Outdated {
                latest: Version::new(1, 1, 0),
            },
        )
    }

    fn installer(binstall: bool) -> Installer {
        Installer {
            cargo: "cargo".into(),
            binstall,
            root: None,
        }
    }

    fn args(cmd: &Command) -> Vec<String> {
        cmd.get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    fn names(selected: &[(&InstalledPackage, &Version)]) -> Vec<String> {
        selected.iter().map(|(p, _)| p.name.clone()).collect()
    }

    #[test]
    fn selects_only_outdated_packages() {
        let reports = [
            outdated("a"),
            report("b", Status::UpToDate),
            report("c", Status::Skipped),
            outdated("d"),
        ];
        assert_eq!(names(&select(&reports, &[], &[]).unwrap()), ["a", "d"]);
    }

    #[test]
    fn selection_respects_only_and_exclude() {
        let reports = [outdated("a"), outdated("b"), outdated("c")];
        let only = ["a".to_owned(), "b".to_owned()];
        let exclude = ["b".to_owned()];
        assert_eq!(names(&select(&reports, &only, &exclude).unwrap()), ["a"]);
    }

    #[test]
    fn selecting_unknown_package_fails() {
        let reports = [outdated("a")];
        let err = select(&reports, &["nope".to_owned()], &[]).unwrap_err();
        assert!(err.to_string().contains("`nope` is not installed"), "{err}");
    }

    #[test]
    fn binstall_command_pins_version() {
        let package = InstalledPackage::new("cargo-deny", Version::new(0, 16, 0));
        let target = Version::new(0, 16, 1);
        let installer = installer(true);
        let update = &installer.plan(&[(&package, &target)])[0];

        assert_eq!(update.method, Method::Binstall);
        assert_eq!(
            args(&installer.command(update)),
            ["binstall", "--no-confirm", "cargo-deny@0.16.1"]
        );
    }

    #[test]
    fn cargo_install_used_without_binstall() {
        // Regression: without cargo-binstall nothing was updated at all.
        let package = InstalledPackage::new("cargo-deny", Version::new(0, 16, 0));
        let target = Version::new(0, 16, 1);
        let mut installer = installer(false);
        installer.root = Some(PathBuf::from("/opt/cargo"));
        let update = &installer.plan(&[(&package, &target)])[0];

        assert_eq!(update.method, Method::CargoInstall);
        assert_eq!(
            args(&installer.command(update)),
            [
                "install",
                "--locked",
                "--root",
                "/opt/cargo",
                "cargo-deny@0.16.1"
            ]
        );
    }

    #[test]
    fn custom_features_are_preserved() {
        let mut package = InstalledPackage::new("ripgrep", Version::new(14, 0, 0));
        package.features = vec!["pcre2".to_owned(), "simd".to_owned()];
        package.no_default_features = true;
        package.profile = Some("dev".to_owned());
        let target = Version::new(14, 1, 0);
        let installer = installer(true);
        let update = &installer.plan(&[(&package, &target)])[0];

        assert_eq!(
            update.method,
            Method::CargoInstall,
            "binstall would drop features"
        );
        assert_eq!(
            args(&installer.command(update)),
            [
                "install",
                "--locked",
                "--features",
                "pcre2,simd",
                "--no-default-features",
                "--profile",
                "dev",
                "ripgrep@14.1.0"
            ]
        );
    }

    #[test]
    #[cfg(unix)]
    fn execute_reports_failed_packages() {
        use std::{fs, os::unix::fs::PermissionsExt as _};

        let dir = tempfile::tempdir().unwrap();
        let cargo = dir.path().join("cargo");
        // Fails for anything mentioning "bad", succeeds otherwise.
        fs::write(
            &cargo,
            "#!/bin/sh\ncase \"$*\" in *bad*) exit 1;; esac\nexit 0\n",
        )
        .unwrap();
        fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755)).unwrap();

        let good = InstalledPackage::new("good", Version::new(1, 0, 0));
        let bad = InstalledPackage::new("bad", Version::new(1, 0, 0));
        let target = Version::new(1, 1, 0);
        let installer = Installer {
            cargo: cargo.into(),
            binstall: false,
            root: None,
        };
        let updates = installer.plan(&[(&good, &target), (&bad, &target)]);

        assert_eq!(installer.execute(&updates), ["bad"]);
    }
}
