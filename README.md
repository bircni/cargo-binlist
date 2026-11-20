# cargo-binlist

[![Crates.io](https://img.shields.io/crates/v/cargo-binlist.svg)](https://crates.io/crates/cargo-binlist)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/bircni/cargo-binlist/blob/main/LICENSE)
[![CI](https://github.com/bircni/cargo-binlist/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/bircni/cargo-binlist/actions/workflows/ci.yml)

`cargo-binlist` lists every binary you installed with `cargo install`, shows which ones
have newer versions on crates.io, and updates them — using prebuilt binaries via
[`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall) when available.

```text
╭─────────────────────┬───────────┬─────────┬──────────────────╮
│ Name                ┆ Installed ┆ Latest  ┆ Status           │
╞═════════════════════╪═══════════╪═════════╪══════════════════╡
│ cargo-deny          ┆ 0.20.2    ┆ 0.22.0  ┆ update available │
│ cargo-nextest       ┆ 0.9.151   ┆ 0.9.152 ┆ update available │
│ git-cliff           ┆ 2.14.2    ┆ 2.14.2  ┆ up to date       │
│ my-tool             ┆ 0.1.0     ┆ -       ┆ skipped (path)   │
╰─────────────────────┴───────────┴─────────┴──────────────────╯
4 packages, 2 outdated
```

## Installation

```sh
cargo binstall cargo-binlist
# or build from source
cargo install --locked cargo-binlist
```

## Usage

```sh
cargo binlist                      # list installed binaries (same as `cargo binlist list`)
cargo binlist list --outdated      # only show packages with updates
cargo binlist list --json          # machine readable output
cargo binlist check                # exit with status 1 if anything is outdated (CI friendly)
cargo binlist update               # update everything that is outdated
cargo binlist update ripgrep bat   # update only these packages
cargo binlist update -x cargo-deny # update everything except cargo-deny
cargo binlist update --dry-run     # show what would be updated
cargo binlist init                 # install cargo-binstall
```

Global options:

| Option          | Description                                                                 |
| --------------- | --------------------------------------------------------------------------- |
| `--root <DIR>`  | Install root to inspect (default: `$CARGO_INSTALL_ROOT`, `$CARGO_HOME`, `~/.cargo`) |
| `--pre`         | Also consider prerelease versions as updates                                |
| `-v` / `-q`     | Increase / decrease log verbosity                                           |

## How it works

- Installed packages are read from cargo's install metadata (`.crates2.json`), so the
  source of every package is known. Packages installed from git, a local path or an
  alternative registry are shown but not checked or updated.
- Latest versions come from the crates.io [sparse index](https://doc.rust-lang.org/cargo/reference/registry-index.html#sparse-protocol).
  Yanked versions are ignored, as are prereleases unless `--pre` is passed or the
  installed version is itself a prerelease. A `--version` requirement used at install
  time is respected.
- Updates use `cargo binstall` when it is installed. Packages that were built with
  custom features (`--features`, `--all-features`, `--no-default-features`) are rebuilt
  with `cargo install --locked` using the same flags, as are all packages when
  `cargo-binstall` is not available.
