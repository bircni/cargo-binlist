//! End-to-end tests of the binary. They only use git and path installs, so no
//! network access is needed.
#![expect(clippy::unwrap_used, reason = "unwrap is used for test setup")]

use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use tempfile::TempDir;

const OFFLINE_INSTALLS: &str = r#"{
  "installs": {
    "my-tool 0.1.0 (path+file:///home/me/my-tool)": { "bins": ["my-tool"] },
    "other 2.0.0 (git+https://example.com/other#abc123)": { "bins": ["other"] }
  }
}"#;

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-binlist"))
        .args(args)
        .env_remove("CARGO_INSTALL_ROOT")
        .output()
        .unwrap()
}

fn root_with(metadata: Option<&str>) -> TempDir {
    let dir = TempDir::new().unwrap();
    if let Some(metadata) = metadata {
        fs::write(dir.path().join(".crates2.json"), metadata).unwrap();
    }
    dir
}

fn run_in(root: &Path, args: &[&str]) -> Output {
    let mut all = args.to_vec();
    all.extend(["--root", root.to_str().unwrap()]);
    run(&all)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn cargo_subcommand_help_is_available() {
    let output = run(&["binlist", "--help"]);
    assert!(output.status.success(), "help failed: {output:?}");
    assert!(
        stdout(&output).contains("List all installed binaries"),
        "unexpected help output: {}",
        stdout(&output)
    );
}

#[test]
fn version_is_available() {
    let output = run(&["--version"]);
    assert!(output.status.success(), "version failed: {output:?}");
    assert!(
        stdout(&output).contains(env!("CARGO_PKG_VERSION")),
        "unexpected version output: {}",
        stdout(&output)
    );
}

#[test]
fn invalid_subcommand_is_rejected() {
    let output = run(&["unknown"]);
    assert!(
        !output.status.success(),
        "invalid command unexpectedly passed"
    );
    assert!(
        stderr(&output).contains("unrecognized subcommand"),
        "unexpected error output: {}",
        stderr(&output)
    );
}

#[test]
fn list_is_the_default_command() {
    let root = root_with(None);
    let output = run_in(root.path(), &[]);
    assert!(output.status.success(), "list failed: {output:?}");
    assert_eq!(stdout(&output).trim(), "No packages installed");
}

#[test]
fn list_shows_skipped_git_and_path_installs() {
    let root = root_with(Some(OFFLINE_INSTALLS));
    let output = run_in(root.path(), &["list"]);
    let out = stdout(&output);

    assert!(output.status.success(), "list failed: {output:?}");
    assert!(out.contains("skipped (path)"), "unexpected output: {out}");
    assert!(out.contains("skipped (git)"), "unexpected output: {out}");
    assert!(
        out.contains("2 packages, 0 outdated"),
        "unexpected output: {out}"
    );
}

#[test]
fn list_outputs_json() {
    let root = root_with(Some(OFFLINE_INSTALLS));
    let output = run_in(root.path(), &["list", "--json"]);
    assert!(output.status.success(), "list failed: {output:?}");

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let rows = json.as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["name"], "my-tool");
    assert_eq!(rows[0]["source"]["kind"], "path");
    assert_eq!(rows[0]["status"], "skipped");
}

#[test]
fn check_succeeds_when_nothing_is_outdated() {
    let root = root_with(Some(OFFLINE_INSTALLS));
    let output = run_in(root.path(), &["check"]);
    assert!(output.status.success(), "check failed: {output:?}");
    assert_eq!(stdout(&output).trim(), "All packages are up to date");
}

#[test]
fn update_does_nothing_when_nothing_is_outdated() {
    let root = root_with(Some(OFFLINE_INSTALLS));
    let output = run_in(root.path(), &["update", "--yes"]);
    assert!(output.status.success(), "update failed: {output:?}");
    assert_eq!(stdout(&output).trim(), "All packages are up to date");
}

#[test]
fn update_rejects_unknown_packages() {
    let root = root_with(Some(OFFLINE_INSTALLS));
    let output = run_in(root.path(), &["update", "nope"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("Package `nope` is not installed"),
        "unexpected error output: {}",
        stderr(&output)
    );
}

#[test]
fn reports_invalid_metadata() {
    let root = root_with(Some("{ not json"));
    let output = run_in(root.path(), &["list"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("Failed to parse"),
        "unexpected error output: {}",
        stderr(&output)
    );
}

#[test]
#[cfg(unix)]
fn init_succeeds_when_cargo_binstall_is_available() {
    use std::{env, os::unix::fs::PermissionsExt as _};

    let dir = TempDir::new().unwrap();
    let binstall = dir.path().join("cargo-binstall");
    fs::write(&binstall, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&binstall, fs::Permissions::from_mode(0o755)).unwrap();
    let path = env::join_paths(
        std::iter::once(dir.path().to_path_buf())
            .chain(env::split_paths(&env::var_os("PATH").unwrap())),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_cargo-binlist"))
        .arg("init")
        .env("PATH", path)
        .output()
        .unwrap();

    assert!(output.status.success(), "init failed: {output:?}");
    assert!(
        stdout(&output).contains("already installed"),
        "unexpected output: {}",
        stdout(&output)
    );
}
