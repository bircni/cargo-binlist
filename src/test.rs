use crate::utils;

#[test]
fn test_parse_cargo_output() {
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

    let parsed = utils::parse_cargo_output(output);
    assert_eq!(parsed.len(), 4);
    assert_eq!(parsed[0].0, "cargo-binstall");
    assert_eq!(parsed[0].1.to_string(), "1.10.18");
    assert_eq!(parsed[1].0, "cargo-bloat");
    assert_eq!(parsed[1].1.to_string(), "0.12.1");
    assert_eq!(parsed[2].0, "cargo-deny");
    assert_eq!(parsed[2].1.to_string(), "0.16.1");
    assert_eq!(parsed[3].0, "cargo-edit");
    assert_eq!(parsed[3].1.to_string(), "0.13.0");
}

#[test]
fn test_parse_package_line() {
    let line = "tester v1.10.18:";
    let parsed = utils::parse_package_line(line).unwrap();
    assert_eq!(parsed.0, "tester");
    assert_eq!(parsed.1.to_string(), "1.10.18");
}

#[test]
fn test_get_latest_version() {
    utils::get_latest_version("serde").unwrap();
}

#[test]
fn test_get_installed_bins() {
    utils::get_installed_bins().unwrap();
}
