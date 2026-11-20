//! Discovery of packages installed with `cargo install`.
//!
//! Cargo records every install in `<root>/.crates2.json`, including the source
//! (crates.io, git, path, ...) and the feature flags used. Reading that file is
//! more reliable than parsing the human readable `cargo install --list` output.

use std::{
    collections::BTreeMap,
    env,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
};

use anyhow::Context as _;
use semver::{Version, VersionReq};
use serde::Deserialize;

use crate::package::{InstalledPackage, Source};

const METADATA_FILE: &str = ".crates2.json";

/// Determine the cargo install root the same way cargo does (minus config files):
/// `--root`, then `$CARGO_INSTALL_ROOT`, then `$CARGO_HOME`, then `~/.cargo`.
pub fn install_root(explicit: Option<&Path>) -> anyhow::Result<PathBuf> {
    resolve_root(
        explicit,
        env::var_os("CARGO_INSTALL_ROOT"),
        env::var_os("CARGO_HOME"),
        env::home_dir(),
    )
    .context("Could not determine the cargo install root; set CARGO_HOME or pass --root")
}

fn resolve_root(
    explicit: Option<&Path>,
    install_root: Option<OsString>,
    cargo_home: Option<OsString>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    explicit
        .map(Path::to_path_buf)
        .or_else(|| install_root.filter(|p| !p.is_empty()).map(PathBuf::from))
        .or_else(|| cargo_home.filter(|p| !p.is_empty()).map(PathBuf::from))
        .or_else(|| home.map(|h| h.join(".cargo")))
}

/// Load all packages installed into `root`.
pub fn load(root: &Path) -> anyhow::Result<Vec<InstalledPackage>> {
    let path = root.join(METADATA_FILE);
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            log::debug!("{} does not exist", path.display());
            return Ok(Vec::new());
        }
        Err(e) => return Err(e).with_context(|| format!("Failed to read {}", path.display())),
    };
    parse(&content).with_context(|| format!("Failed to parse {}", path.display()))
}

#[derive(Deserialize)]
struct Metadata {
    #[serde(default)]
    installs: BTreeMap<String, InstallInfo>,
}

#[derive(Deserialize)]
struct InstallInfo {
    version_req: Option<String>,
    #[serde(default)]
    features: Vec<String>,
    #[serde(default)]
    all_features: bool,
    #[serde(default)]
    no_default_features: bool,
    profile: Option<String>,
}

/// Parse the contents of a `.crates2.json` file.
pub fn parse(content: &str) -> anyhow::Result<Vec<InstalledPackage>> {
    let metadata: Metadata = serde_json::from_str(content)?;
    Ok(metadata
        .installs
        .into_iter()
        .filter_map(|(key, info)| {
            package_from_entry(&key, info)
                .inspect_err(|e| log::warn!("Ignoring install entry `{key}`: {e:#}"))
                .ok()
        })
        .collect())
}

/// Build a package from an entry keyed like `name 1.2.3 (registry+https://...)`.
fn package_from_entry(key: &str, info: InstallInfo) -> anyhow::Result<InstalledPackage> {
    let (name, rest) = key.split_once(' ').context("missing version")?;
    let (version, source) = rest.split_once(' ').context("missing source")?;
    let source = source
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .context("malformed source")?;

    let version_req = info.version_req.as_deref().and_then(|req| {
        VersionReq::parse(req)
            .inspect_err(|e| log::warn!("Ignoring version requirement `{req}` of {name}: {e}"))
            .ok()
    });

    Ok(InstalledPackage {
        name: name.to_owned(),
        version: Version::parse(version).context("invalid version")?,
        source: Source::parse(source),
        version_req,
        features: info.features,
        all_features: info.all_features,
        no_default_features: info.no_default_features,
        profile: info.profile.filter(|p| p != "release"),
    })
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests")]

    use super::*;

    const SAMPLE: &str = r#"{
  "installs": {
    "cargo-deny 0.16.1 (registry+https://github.com/rust-lang/crates.io-index)": {
      "version_req": null, "bins": ["cargo-deny"], "features": [], "all_features": false,
      "no_default_features": false, "profile": "release", "target": "x86_64-unknown-linux-gnu", "rustc": "rustc 1.90.0"
    },
    "ripgrep 14.1.0 (sparse+https://index.crates.io/)": {
      "version_req": "^14", "bins": ["rg"], "features": ["pcre2"], "all_features": false,
      "no_default_features": false, "profile": "dev", "target": null, "rustc": null
    },
    "my-tool 0.1.0 (path+file:///home/me/my-tool)": {
      "version_req": null, "bins": ["my-tool"], "features": [], "all_features": false,
      "no_default_features": false, "profile": "release", "target": null, "rustc": null
    },
    "garbage": {}
  }
}"#;

    #[test]
    fn parses_install_metadata() {
        let packages = parse(SAMPLE).unwrap();

        assert_eq!(packages.len(), 3, "invalid entries should be skipped");

        let deny = &packages[0];
        assert_eq!(deny.name, "cargo-deny");
        assert_eq!(deny.version, Version::new(0, 16, 1));
        assert_eq!(deny.source, Source::CratesIo);
        assert_eq!(deny.profile, None);
        assert!(!deny.has_custom_features(), "no custom features expected");

        let tool = &packages[1];
        assert_eq!(tool.name, "my-tool");
        assert_eq!(tool.source, Source::Path("/home/me/my-tool".to_owned()));

        let rg = &packages[2];
        assert_eq!(rg.source, Source::CratesIo);
        assert_eq!(rg.version_req, Some(VersionReq::parse("^14").unwrap()));
        assert_eq!(rg.features, ["pcre2"]);
        assert_eq!(rg.profile.as_deref(), Some("dev"));
        assert!(rg.has_custom_features(), "features were recorded");
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(parse("not json").is_err(), "invalid JSON must fail");
    }

    #[test]
    fn missing_metadata_means_nothing_installed() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load(dir.path()).unwrap().is_empty(), "expected no packages");
    }

    #[test]
    fn resolves_root_in_cargo_order() {
        let explicit = Path::new("/explicit");
        let install = || Some(OsString::from("/install"));
        let cargo_home = || Some(OsString::from("/cargo-home"));
        let home = || Some(PathBuf::from("/home/me"));

        assert_eq!(
            resolve_root(Some(explicit), install(), cargo_home(), home()),
            Some(PathBuf::from("/explicit"))
        );
        assert_eq!(
            resolve_root(None, install(), cargo_home(), home()),
            Some(PathBuf::from("/install"))
        );
        assert_eq!(
            resolve_root(None, Some(OsString::new()), cargo_home(), home()),
            Some(PathBuf::from("/cargo-home"))
        );
        assert_eq!(
            resolve_root(None, None, None, home()),
            Some(PathBuf::from("/home/me/.cargo"))
        );
        assert_eq!(resolve_root(None, None, None, None), None);
    }
}
