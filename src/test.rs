#![expect(clippy::unwrap_used, reason = "unwrap is used for testing purposes")]
use comfy_table::{Attribute, Cell, Color};
use log::LevelFilter;
use semver::Version;

use crate::{
    cli::{Cli, Opts},
    data::{PackageInfo, VersionCheck},
    logic,
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

    let mut parsed = logic::get_package_infos(output);
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
    let parsed = logic::parse_package_line(line).unwrap();
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
    logic::get_installed_bins().unwrap();
}

#[test]
fn test_cli_list() {
    Cli::List(Opts {
        filter: LevelFilter::Trace,
    })
    .run()
    .unwrap();
}

#[test]
fn test_cli_update() {
    logic::update(&[]).unwrap();
}

#[test]
fn test_version_occurrences() {
    let pkgs = vec![
        PackageInfo::new("cargo-binstall".to_owned(), Version::new(1, 10, 18)),
        PackageInfo::new("cargo-bloat".to_owned(), Version::new(0, 12, 1)),
        PackageInfo::new("cargo-deny".to_owned(), Version::new(0, 16, 1)),
        PackageInfo::new("cargo-edit".to_owned(), Version::new(0, 13, 0)),
    ];

    logic::version_occurrences(&pkgs);
}

#[test]
fn test_colored_cell() {
    let local_newer_cell = Cell::new("No Update").add_attribute(Attribute::Dim);
    let cell = VersionCheck::LocalNewer.colored_cell();
    assert_eq!(cell.content(), "No Update");
    assert_eq!(cell, local_newer_cell);

    let newer_available_cell = Cell::new("Update Available (2.0.0)")
        .fg(Color::Green)
        .add_attribute(Attribute::Bold);
    let cell = VersionCheck::NewerAvailable(Version::new(2, 0, 0)).colored_cell();
    assert_eq!(cell.content(), "Update Available (2.0.0)");
    assert_eq!(cell, newer_available_cell);

    let unavailable_cell = Cell::new("Not Available").fg(Color::Red);
    let cell = VersionCheck::UnAvailable.colored_cell();
    assert_eq!(cell.content(), "Not Available");
    assert_eq!(cell, unavailable_cell);

    let up_to_date_cell = Cell::new("Up to date").fg(Color::Blue);
    let cell = VersionCheck::UpToDate.colored_cell();
    assert_eq!(cell.content(), "Up to date");
    assert_eq!(cell, up_to_date_cell);
}
