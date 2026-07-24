#![expect(clippy::unwrap_used, reason = "unwrap is used for testing purposes")]
#[cfg(unix)]
use std::{fs, path::Path};

use comfy_table::{Attribute, Cell, Color};
use log::LevelFilter;
use semver::Version;
#[cfg(unix)]
use tempfile::TempDir;

use crate::{
    cli::{Cli, ListOpts, Opts, UpdateOpts},
    data::{PackageInfo, VersionCheck},
    logic,
};

#[cfg(unix)]
fn write_fake_cargo(dir: &Path, script: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt as _;

    let path = dir.join("cargo");
    fs::write(&path, script).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
    path
}

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

    let mut parsed = logic::get_package_infos_with_latest(output, |pkg| {
        let version = match pkg.name.as_str() {
            "cargo-binstall" => Version::new(1, 10, 18),
            "cargo-bloat" => Version::new(0, 12, 2),
            "cargo-deny" => Version::new(0, 16, 1),
            "cargo-edit" => Version::new(0, 12, 9),
            _ => return Err(anyhow::anyhow!("unexpected package")),
        };
        Ok(version)
    });
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

    assert!(matches!(parsed[0].info, VersionCheck::UpToDate));
    assert!(matches!(parsed[1].info, VersionCheck::NewerAvailable(_)));
    assert!(matches!(parsed[2].info, VersionCheck::UpToDate));
    assert!(matches!(parsed[3].info, VersionCheck::LocalNewer));
}

#[test]
fn test_get_package_infos_ignores_invalid_lines_and_failed_lookups() {
    let output = r"
not a package
cargo-deny v0.16.1:
    cargo-deny
";

    let parsed = logic::get_package_infos_with_latest(output, |_| {
        Err(anyhow::anyhow!("crate is unavailable"))
    });

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].name, "cargo-deny");
    assert_eq!(parsed[0].info, VersionCheck::UnAvailable);
}

#[test]
fn test_parse_package_line() {
    let line = "tester v1.10.18:";
    let parsed = logic::parse_package_line(line).unwrap();
    assert_eq!(parsed.name, "tester");
    assert_eq!(parsed.version.to_string(), "1.10.18");
}

#[test]
fn test_parse_package_line_invalid() {
    let line = "not a package line";
    let err = logic::parse_package_line(line).err().unwrap();
    assert!(err.to_string().contains("Failed to parse package line"));
}

#[test]
fn test_parse_package_line_invalid_version() {
    let err = logic::parse_package_line("tester not-a-version:")
        .err()
        .unwrap();
    assert!(
        err.to_string().contains("unexpected character"),
        "unexpected parse error: {err}"
    );
}

#[test]
fn test_packageinfo() {
    let mut pkg = PackageInfo::new("serde".to_owned(), Version::new(1, 0, 0));
    pkg.set_info(&Version::new(1, 0, 1));
    assert!(matches!(pkg.info, VersionCheck::NewerAvailable(_)));

    pkg.set_info(&Version::new(1, 0, 0));
    assert_eq!(pkg.info, VersionCheck::UpToDate);

    pkg.set_info(&Version::new(0, 9, 9));
    assert_eq!(pkg.info, VersionCheck::LocalNewer);
}

#[test]
fn test_cli_update() {
    logic::update(&[], true).unwrap();
}

#[test]
#[cfg(unix)]
fn test_update_ignores_packages_without_updates() {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("cargo_log.txt");
    let script = r#"#!/bin/sh
script_dir=$(dirname "$0")
printf "%s\n" "$@" > "$script_dir/cargo_log.txt"
exit 0
"#;
    let cargo_path = write_fake_cargo(temp_dir.path(), script);
    let packages = vec![
        PackageInfo {
            name: "cargo-binstall".to_owned(),
            version: Version::new(1, 0, 0),
            info: VersionCheck::UpToDate,
        },
        PackageInfo {
            name: "cargo-deny".to_owned(),
            version: Version::new(0, 16, 1),
            info: VersionCheck::LocalNewer,
        },
    ];

    logic::update_with_cargo(&packages, true, &cargo_path).unwrap();

    assert!(!log_path.exists());
}

#[test]
#[cfg(unix)]
fn test_get_installed_bins_uses_fake_cargo() {
    let temp_dir = TempDir::new().unwrap();
    let script = r#"#!/bin/sh
if [ "$1" = "install" ] && [ "$2" = "--list" ]; then
  echo "cargo-binstall v1.10.18:"
  exit 0
fi
echo "unexpected args" >&2
exit 1
"#;
    let cargo_path = write_fake_cargo(temp_dir.path(), script);

    let output = logic::get_installed_bins_with_cargo(&cargo_path).unwrap();
    assert!(output.contains("cargo-binstall v1.10.18:"));
}

#[test]
#[cfg(unix)]
fn test_update_uses_fake_cargo_binstall() {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("cargo_log.txt");
    let script = r#"#!/bin/sh
script_dir=$(dirname "$0")
printf "%s\n" "$@" > "$script_dir/cargo_log.txt"
exit 0
"#;
    let cargo_path = write_fake_cargo(temp_dir.path(), script);

    let packages = vec![
        PackageInfo {
            name: "cargo-binstall".to_owned(),
            version: Version::new(1, 0, 0),
            info: VersionCheck::NewerAvailable(Version::new(1, 1, 0)),
        },
        PackageInfo {
            name: "cargo-deny".to_owned(),
            version: Version::new(0, 16, 0),
            info: VersionCheck::NewerAvailable(Version::new(0, 16, 1)),
        },
    ];

    logic::update_with_cargo(&packages, true, &cargo_path).unwrap();

    let logged = fs::read_to_string(&log_path).unwrap();
    let args = logged.lines().collect::<Vec<_>>();
    assert_eq!(args, vec!["binstall", "cargo-binstall", "cargo-deny", "-y"]);
}

#[test]
#[cfg(unix)]
fn test_get_installed_bins_error_when_cargo_fails() {
    let temp_dir = TempDir::new().unwrap();
    let script = r#"#!/bin/sh
echo "boom" >&2
exit 1
"#;
    let cargo_path = write_fake_cargo(temp_dir.path(), script);

    let err = logic::get_installed_bins_with_cargo(&cargo_path)
        .err()
        .unwrap();
    assert!(err.to_string().contains("Failed to get installed binaries"));
    assert!(err.to_string().contains("boom"));
}

#[test]
#[cfg(unix)]
fn test_update_skips_when_cargo_binstall_missing() {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("cargo_log.txt");
    let script = r#"#!/bin/sh
script_dir=$(dirname "$0")
printf "%s\n" "$@" > "$script_dir/cargo_log.txt"
exit 0
"#;
    let cargo_path = write_fake_cargo(temp_dir.path(), script);

    let packages = vec![PackageInfo {
        name: "cargo-deny".to_owned(),
        version: Version::new(0, 16, 0),
        info: VersionCheck::NewerAvailable(Version::new(0, 16, 1)),
    }];

    logic::update_with_cargo(&packages, true, &cargo_path).unwrap();

    assert!(!log_path.exists());
}

#[test]
#[cfg(unix)]
fn test_update_returns_error_on_cargo_failure() {
    let temp_dir = TempDir::new().unwrap();
    let script = r#"#!/bin/sh
echo "nope" >&2
exit 1
"#;
    let cargo_path = write_fake_cargo(temp_dir.path(), script);

    let packages = vec![PackageInfo {
        name: "cargo-binstall".to_owned(),
        version: Version::new(1, 0, 0),
        info: VersionCheck::NewerAvailable(Version::new(1, 1, 0)),
    }];

    let err = logic::update_with_cargo(&packages, true, &cargo_path)
        .err()
        .unwrap();
    assert!(err.to_string().contains("Failed to update packages"));
}

#[test]
fn test_version_occurrences() {
    let pkgs = vec![
        PackageInfo {
            name: "cargo-binstall".to_owned(),
            version: Version::new(1, 10, 18),
            info: VersionCheck::NewerAvailable(Version::new(1, 10, 19)),
        },
        PackageInfo {
            name: "cargo-bloat".to_owned(),
            version: Version::new(0, 12, 1),
            info: VersionCheck::UpToDate,
        },
        PackageInfo {
            name: "cargo-deny".to_owned(),
            version: Version::new(0, 16, 1),
            info: VersionCheck::LocalNewer,
        },
        PackageInfo::new("cargo-edit".to_owned(), Version::new(0, 13, 0)),
    ];

    logic::version_occurrences(&pkgs);
}

#[test]
fn test_create_table_sorts_updates_first_and_supports_both_layouts() {
    let packages = vec![
        PackageInfo {
            name: "z-current".to_owned(),
            version: Version::new(1, 0, 0),
            info: VersionCheck::UpToDate,
        },
        PackageInfo {
            name: "b-update".to_owned(),
            version: Version::new(1, 0, 0),
            info: VersionCheck::NewerAvailable(Version::new(1, 1, 0)),
        },
        PackageInfo {
            name: "a-update".to_owned(),
            version: Version::new(1, 0, 0),
            info: VersionCheck::NewerAvailable(Version::new(1, 2, 0)),
        },
    ];

    let condensed = logic::create_table(&packages, false).to_string();
    let uncondensed = logic::create_table(&packages, true).to_string();

    let first_update = condensed.find("a-update").unwrap();
    let second_update = condensed.find("b-update").unwrap();
    let current = condensed.find("z-current").unwrap();
    assert!(
        first_update < second_update && second_update < current,
        "packages were not ordered correctly:\n{condensed}"
    );
    assert!(
        uncondensed.lines().count() > condensed.lines().count(),
        "uncondensed layout should use more table rows"
    );
}

#[test]
fn test_list_pkgs_handles_filters_and_empty_results() {
    let current = PackageInfo {
        name: "cargo-current".to_owned(),
        version: Version::new(1, 0, 0),
        info: VersionCheck::UpToDate,
    };
    let update = PackageInfo {
        name: "cargo-update".to_owned(),
        version: Version::new(1, 0, 0),
        info: VersionCheck::NewerAvailable(Version::new(1, 1, 0)),
    };

    logic::list_pkgs(Vec::new(), false, false);
    logic::list_pkgs(vec![current], true, false);
    logic::list_pkgs(vec![update], true, true);
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

#[test]
fn test_needs_binlist() {
    assert!(
        Cli::List(ListOpts {
            filter: LevelFilter::Info,
            outdated: false,
            uncondensed: false,
        })
        .needs_binlist()
    );
    assert!(
        Cli::Update(UpdateOpts {
            filter: LevelFilter::Warn,
            no_confirm: true,
        })
        .needs_binlist()
    );
    assert!(
        !Cli::Init(Opts {
            filter: LevelFilter::Error,
        })
        .needs_binlist()
    );
}

#[test]
fn test_get_filter() {
    let list = Cli::List(ListOpts {
        filter: LevelFilter::Trace,
        outdated: false,
        uncondensed: false,
    });
    assert_eq!(list.get_filter(), LevelFilter::Trace);

    let update = Cli::Update(UpdateOpts {
        filter: LevelFilter::Debug,
        no_confirm: true,
    });
    assert_eq!(update.get_filter(), LevelFilter::Debug);

    let init = Cli::Init(Opts {
        filter: LevelFilter::Warn,
    });
    assert_eq!(init.get_filter(), LevelFilter::Warn);
}
