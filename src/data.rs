use std::{cmp::Ordering, time::Duration};

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
        let client = SyncClient::new("cargo-binlist", Duration::from_millis(1000))?;
        log::debug!("Fetching latest version for {}", self.name);
        let cr = client.get_crate(&self.name)?;
        let version = Version::parse(&cr.crate_data.max_version)?;
        Ok(version)
    }
}
