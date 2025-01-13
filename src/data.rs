use std::{cmp::Ordering, time::Duration};

use crates_io_api::SyncClient;
use semver::Version;

#[derive(Default, Debug, PartialEq, Eq)]
pub enum VersionCheck {
    LocalNewer,
    NewerAvailable,
    #[default]
    UnAvailable,
    UpToDate,
}
pub struct PackageInfo {
    pub name: String,
    pub version: Version,
    pub info: VersionCheck,
}

impl PackageInfo {
    pub fn new(name: String, version: Version) -> Self {
        Self {
            name,
            version,
            info: VersionCheck::default(),
        }
    }

    pub fn set_info(&mut self, latest: &Version) {
        self.info = match self.version.cmp(latest) {
            Ordering::Less => VersionCheck::NewerAvailable,
            Ordering::Greater => VersionCheck::LocalNewer,
            Ordering::Equal => VersionCheck::UpToDate,
        }
    }

    pub fn latest_version(&self) -> anyhow::Result<Version> {
        let client = SyncClient::new("cargo-binlist", Duration::from_millis(1000))?;
        log::debug!("Fetching latest version for {}", self.name);
        let cr = client.get_crate(&self.name)?;
        let version = Version::parse(&cr.crate_data.max_version)?;
        Ok(version)
    }
}
