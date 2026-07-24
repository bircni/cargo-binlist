#![expect(clippy::unwrap_used, reason = "unwrap is used for test setup")]

#[cfg(unix)]
use std::fs;
use std::{
    env,
    path::Path,
    process::{Command, Output},
};
#[cfg(unix)]
use tempfile::TempDir;

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-binlist"))
        .args(args)
        .output()
        .unwrap()
}

#[cfg(unix)]
fn write_executable(dir: &Path, name: &str, script: &str) {
    use std::os::unix::fs::PermissionsExt as _;

    let path = dir.join(name);
    fs::write(&path, script).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

#[cfg(unix)]
fn run_with_path(args: &[&str], path_prefix: &Path) -> Output {
    let current_path = env::var_os("PATH").unwrap();
    let path = env::join_paths(
        std::iter::once(path_prefix.to_path_buf()).chain(env::split_paths(&current_path)),
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_cargo-binlist"))
        .args(args)
        .env("PATH", path)
        .output()
        .unwrap()
}

#[test]
fn cargo_subcommand_help_is_available() {
    let output = run(&["binlist", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "help failed: {output:?}");
    assert!(
        stdout.contains("List all installed binaries"),
        "unexpected help output: {stdout}"
    );
}

#[test]
fn version_is_available() {
    let output = run(&["--version"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "version failed: {output:?}");
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "unexpected version output: {stdout}"
    );
}

#[test]
fn invalid_subcommand_is_rejected() {
    let output = run(&["unknown"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "invalid command unexpectedly passed"
    );
    assert!(
        stderr.contains("unrecognized subcommand"),
        "unexpected error output: {stderr}"
    );
}

#[test]
#[cfg(unix)]
fn init_succeeds_when_cargo_binstall_is_available() {
    let temp_dir = TempDir::new().unwrap();
    write_executable(temp_dir.path(), "cargo-binstall", "#!/bin/sh\nexit 0\n");

    let output = run_with_path(&["init"], temp_dir.path());

    assert!(output.status.success(), "init failed: {output:?}");
}

#[test]
#[cfg(unix)]
fn list_handles_an_empty_installation() {
    let temp_dir = TempDir::new().unwrap();
    write_executable(
        temp_dir.path(),
        "cargo",
        "#!/bin/sh\n[ \"$1\" = install ] && [ \"$2\" = --list ]\n",
    );

    let output = run_with_path(&["list"], temp_dir.path());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "list failed: {output:?}");
    assert!(
        stdout.contains("No packages found"),
        "unexpected list output: {stdout}"
    );
}

#[test]
#[cfg(unix)]
fn update_handles_an_empty_installation() {
    let temp_dir = TempDir::new().unwrap();
    write_executable(
        temp_dir.path(),
        "cargo",
        "#!/bin/sh\n[ \"$1\" = install ] && [ \"$2\" = --list ]\n",
    );

    let output = run_with_path(&["update", "--no-confirm"], temp_dir.path());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "update failed: {output:?}");
    assert!(
        stdout.contains("No packages to update"),
        "unexpected update output: {stdout}"
    );
}

#[test]
#[cfg(unix)]
fn list_reports_cargo_failures() {
    let temp_dir = TempDir::new().unwrap();
    write_executable(
        temp_dir.path(),
        "cargo",
        "#!/bin/sh\necho boom >&2\nexit 1\n",
    );

    let output = run_with_path(&["list"], temp_dir.path());
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr.contains("Failed to get installed binaries: boom"),
        "unexpected failure output: {stderr}"
    );
}
