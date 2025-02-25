# cargo-binlist

[![Crates.io](https://img.shields.io/crates/v/cargo-binlist.svg)](https://crates.io/crates/cargo-binlist)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/bircni/cargo-binlist/blob/main/LICENSE)
[![CI](https://github.com/bircni/cargo-binlist/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/bircni/cargo-binlist/actions/workflows/ci.yml)

`cargo-binlist` is a tool to list all the installed binaries with cargo and their versions.
You can get a list of installed binaries and update them using cargo-binstall.

## Installation

Using `cargo install`

```sh
cargo install cargo-binlist
```

or using `cargo binstall`

```sh
cargo binstall cargo-binlist
```

## Usage

```sh
Usage: cargo-binlist [OPTIONS]

Options:
  -l, --list               List all installed crates
  -n, --list-updates       List crates with newer versions available
  -u, --update             Update all crates
  -f, --loglevel <FILTER>  Log Level Filter [Debug, Info, Error, Warn] [default: Info]
  -h, --help               Print help
  -V, --version            Print version
```
