//! Installed packages and their update status.

use std::fmt;

use semver::{Version, VersionReq};
use serde::Serialize;

/// Index URLs that cargo records for crates.io installs.
const CRATES_IO_INDEXES: [&str; 2] = [
    "https://github.com/rust-lang/crates.io-index",
    "https://index.crates.io",
];

/// Where an installed package was installed from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "location", rename_all = "snake_case")]
pub enum Source {
    /// The crates.io registry
    CratesIo,
    /// An alternative registry, identified by its index URL
    Registry(String),
    /// A git repository (`cargo install --git`)
    Git(String),
    /// A local directory (`cargo install --path`)
    Path(String),
}

impl Source {
    /// Parse a cargo source id such as `registry+https://github.com/rust-lang/crates.io-index`.
    pub fn parse(id: &str) -> Self {
        if let Some(path) = id.strip_prefix("path+") {
            return Self::Path(path.strip_prefix("file://").unwrap_or(path).to_owned());
        }
        if let Some(url) = id.strip_prefix("git+") {
            return Self::Git(url.to_owned());
        }
        let url = id
            .strip_prefix("registry+")
            .or_else(|| id.strip_prefix("sparse+"))
            .unwrap_or(id);
        if CRATES_IO_INDEXES.contains(&url.trim_end_matches('/')) {
            Self::CratesIo
        } else {
            Self::Registry(url.to_owned())
        }
    }

    /// Short, human readable name of the source kind.
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::CratesIo => "crates.io",
            Self::Registry(_) => "registry",
            Self::Git(_) => "git",
            Self::Path(_) => "path",
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CratesIo => f.write_str("crates.io"),
            Self::Registry(location) | Self::Git(location) | Self::Path(location) => {
                write!(f, "{} {location}", self.kind())
            }
        }
    }
}

/// A package installed with `cargo install`, as recorded by cargo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledPackage {
    pub name: String,
    pub version: Version,
    pub source: Source,
    /// Version requirement passed via `--version`, if any
    pub version_req: Option<VersionReq>,
    pub features: Vec<String>,
    pub all_features: bool,
    pub no_default_features: bool,
    /// Build profile, if it was not the default `release`
    pub profile: Option<String>,
}

impl InstalledPackage {
    /// A crates.io package installed with default settings.
    #[cfg(test)]
    pub fn new(name: &str, version: Version) -> Self {
        Self {
            name: name.to_owned(),
            version,
            source: Source::CratesIo,
            version_req: None,
            features: Vec::new(),
            all_features: false,
            no_default_features: false,
            profile: None,
        }
    }

    /// Whether the package was built with a non-default feature selection.
    pub const fn has_custom_features(&self) -> bool {
        !self.features.is_empty() || self.all_features || self.no_default_features
    }
}

/// Result of comparing an installed package with its registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Status {
    /// A newer version is available
    Outdated { latest: Version },
    /// The installed version is the latest one
    UpToDate,
    /// The installed version is newer than the latest published one
    Ahead { latest: Version },
    /// The registry has no installable version of this crate
    NotFound,
    /// The package was not installed from crates.io and is not checked
    Skipped,
    /// Looking up the package failed
    Error { message: String },
}

impl Status {
    /// Latest published version, if known.
    pub const fn latest(&self) -> Option<&Version> {
        match self {
            Self::Outdated { latest } | Self::Ahead { latest } => Some(latest),
            Self::UpToDate | Self::NotFound | Self::Skipped | Self::Error { .. } => None,
        }
    }
}

/// An installed package together with its update status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub package: InstalledPackage,
    pub status: Status,
}

impl Report {
    /// The version this package can be updated to, if it is outdated.
    pub const fn update_target(&self) -> Option<&Version> {
        match &self.status {
            Status::Outdated { latest } => Some(latest),
            Status::UpToDate
            | Status::Ahead { .. }
            | Status::NotFound
            | Status::Skipped
            | Status::Error { .. } => None,
        }
    }

    pub const fn is_outdated(&self) -> bool {
        self.update_target().is_some()
    }
}

/// Sort reports so that outdated packages come first, then by name.
pub fn sort_reports(reports: &mut [Report]) {
    reports.sort_by(|a, b| {
        b.is_outdated()
            .cmp(&a.is_outdated())
            .then_with(|| a.package.name.cmp(&b.package.name))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_crates_io_sources() {
        assert_eq!(
            Source::parse("registry+https://github.com/rust-lang/crates.io-index"),
            Source::CratesIo
        );
        assert_eq!(
            Source::parse("sparse+https://index.crates.io/"),
            Source::CratesIo
        );
    }

    #[test]
    fn parses_other_sources() {
        assert_eq!(
            Source::parse("sparse+https://my.registry/index/"),
            Source::Registry("https://my.registry/index/".to_owned())
        );
        assert_eq!(
            Source::parse("git+https://github.com/foo/bar#abc123"),
            Source::Git("https://github.com/foo/bar#abc123".to_owned())
        );
        assert_eq!(
            Source::parse("path+file:///home/me/tool"),
            Source::Path("/home/me/tool".to_owned())
        );
    }

    #[test]
    fn sorts_outdated_first_then_by_name() {
        let report = |name: &str, status: Status| Report {
            package: InstalledPackage::new(name, Version::new(1, 0, 0)),
            status,
        };
        let mut reports = vec![
            report("a-current", Status::UpToDate),
            report(
                "z-update",
                Status::Outdated {
                    latest: Version::new(2, 0, 0),
                },
            ),
            report("b-skipped", Status::Skipped),
        ];

        sort_reports(&mut reports);

        let names: Vec<_> = reports.iter().map(|r| r.package.name.as_str()).collect();
        assert_eq!(names, ["z-update", "a-current", "b-skipped"]);
    }
}
