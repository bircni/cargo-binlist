#![expect(clippy::unwrap_used, reason = "unwrap is used for testing purposes")]
use log::LevelFilter;
use semver::Version;

use crate::{
    cli::Cli,
    data::{PackageInfo, VersionCheck},
    utils,
};

#[test]
fn test_get_package_infos() {
    let output = r"
cargo-binstall v1.10.18:
    cargo-binstall
cargo-bloat v0.12.1:
    cargo-bloat
cargo-deny v0.16.1:
    cargo-deny
cargo-edit v0.13.0:
    cargo-add
    cargo-rm
    cargo-set-version
    cargo-upgrade
";

    let mut parsed = utils::get_package_infos(output);
    parsed.sort_by(|a, b| a.name.cmp(&b.name));
    assert_eq!(parsed.len(), 4);
    assert_eq!(parsed[0].name, "cargo-binstall");
    assert_eq!(parsed[0].version.to_string(), "1.10.18");
    assert_eq!(parsed[1].name, "cargo-bloat");
    assert_eq!(parsed[1].version.to_string(), "0.12.1");
    assert_eq!(parsed[2].name, "cargo-deny");
    assert_eq!(parsed[2].version.to_string(), "0.16.1");
    assert_eq!(parsed[3].name, "cargo-edit");
    assert_eq!(parsed[3].version.to_string(), "0.13.0");
}

#[test]
fn test_parse_package_line() {
    let line = "tester v1.10.18:";
    let parsed = utils::parse_package_line(line).unwrap();
    assert_eq!(parsed.name, "tester");
    assert_eq!(parsed.version.to_string(), "1.10.18");
}

#[test]
fn test_packageinfo() {
    let mut pkg = PackageInfo::new("serde".to_owned(), Version::new(0, 0, 1));
    let latest = pkg.latest_version().unwrap();
    assert!(latest > pkg.version);
    pkg.set_info(&latest);
    assert_eq!(pkg.info, VersionCheck::NewerAvailable(latest));
}

#[test]
fn test_get_installed_bins() {
    utils::get_installed_bins().unwrap();
}

#[test]
fn test_cli_list() {
    Cli {
        list: true,
        update: false,
        list_updates: false,
        filter: LevelFilter::Trace,
        init: false,
    }
    .run()
    .unwrap();
}

#[test]
fn test_cli_update() {
    utils::update(&[]).unwrap();
}

#[test]
fn test_version_occurrences() {
    let pkgs = vec![
        PackageInfo::new("cargo-binstall".to_owned(), Version::new(1, 10, 18)),
        PackageInfo::new("cargo-bloat".to_owned(), Version::new(0, 12, 1)),
        PackageInfo::new("cargo-deny".to_owned(), Version::new(0, 16, 1)),
        PackageInfo::new("cargo-edit".to_owned(), Version::new(0, 13, 0)),
    ];

    utils::version_occurrences(&pkgs);
}
