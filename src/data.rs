use std::{cmp::Ordering, time::Duration};

use anyhow::Context as _;
use comfy_table::{Attribute, Cell, Color};
use crates_io_api::SyncClient;
use semver::Version;

/// Enum to represent the version check status
#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum VersionCheck {
    /// Latest version is newer than the local version
    NewerAvailable(Version),
    /// Local version is up to date
    UpToDate,
    /// Local version is newer than the latest version
    LocalNewer,
    /// Crate is not available on crates.io
    #[default]
    UnAvailable,
}

impl VersionCheck {
    pub fn colored_cell(&self) -> Cell {
        match self {
            Self::LocalNewer => Cell::new("No Update").add_attribute(Attribute::Dim),
            Self::NewerAvailable(version) => Cell::new(format!("Update Available ({version})"))
                .fg(Color::Green)
                .add_attribute(Attribute::Bold),
            Self::UnAvailable => Cell::new("Not Available").fg(Color::Red),
            Self::UpToDate => Cell::new("Up to date").fg(Color::Blue),
        }
    }
}

pub struct PackageInfo {
    /// Name of the package
    pub name: String,
    /// Version of the package
    pub version: Version,
    /// Version check status
    pub info: VersionCheck,
}

impl PackageInfo {
    /// Create a new `PackageInfo` instance
    pub fn new(name: String, version: Version) -> Self {
        Self {
            name,
            version,
            info: VersionCheck::default(),
        }
    }

    /// Set the version check status based on the latest version
    pub fn set_info(&mut self, latest: &Version) {
        self.info = match self.version.cmp(latest) {
            Ordering::Less => VersionCheck::NewerAvailable(latest.clone()),
            Ordering::Greater => VersionCheck::LocalNewer,
            Ordering::Equal => VersionCheck::UpToDate,
        }
    }

    /// Fetch the latest version of the package from crates.io
    pub fn latest_version(&self) -> anyhow::Result<Version> {
        let client = SyncClient::new("cargo-binlist", Duration::from_secs(1))?;
        log::debug!("Fetching latest version for {}", self.name);
        let cr = client.get_crate(&self.name)?;
        latest_installable_version(
            &cr.crate_data.max_version,
            cr.crate_data.max_stable_version.as_deref(),
        )
    }
}

pub fn latest_installable_version(
    max_version: &str,
    max_stable_version: Option<&str>,
) -> anyhow::Result<Version> {
    let version = max_stable_version.with_context(|| {
        format!("No stable version is available; latest release is {max_version}")
    })?;
    Version::parse(version).context("Failed to parse latest stable version")
}
