# Changelog - [cargo-binlist](https://github.com/bircni/cargo-binlist)

All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

## [1.3.0](https://github.com/bircni/cargo-binlist/compare/1.2.0..1.3.0) - 2025-10-20

### Features

- add uncondensed layout & correctly count the crates - ([3f43db7](https://github.com/bircni/cargo-binlist/commit/3f43db7199018ae3340b292add7186917b074341)) - Nicolas

### Build

- update Cargo.toml to exclude unnecessary files - ([58aad55](https://github.com/bircni/cargo-binlist/commit/58aad554a93224ea240b0504becc113df8cea959)) - Nicolas

## [1.2.0](https://github.com/bircni/cargo-binlist/compare/1.1.0..1.2.0) - 2025-09-15

### Continuous Integration

- switch to ubuntu-latest for all jobs - ([b49bb38](https://github.com/bircni/cargo-binlist/commit/b49bb382f83ed291c1dfd6d62a4cdc139f5cd19b)) - Nicolas

### Documentation

- update usage section in README - ([c6ce7de](https://github.com/bircni/cargo-binlist/commit/c6ce7dec07eabfd8d780df757c0f7b25624d1dce)) - Nicolas

### Features

- enhance update command with confirmation request - ([cd73621](https://github.com/bircni/cargo-binlist/commit/cd73621dc14345e622fa82c0595c6879c0cca7fc)) - Nicolas

### Miscellaneous Chores

- remove build script as it is no longer needed - ([46bc184](https://github.com/bircni/cargo-binlist/commit/46bc1849b663097b7d6496af32044356a2fcd35c)) - Nicolas

### Refactoring

- remove `list-updates` and migrate it to `list` - ([4ff1087](https://github.com/bircni/cargo-binlist/commit/4ff1087394a8c092bea36c6c73fe3d5a6dbb6e40)) - Nicolas

### Build

- update Rust toolchain to stable and adjust dependencies - ([e846892](https://github.com/bircni/cargo-binlist/commit/e8468920226fbc1bba4225d151dd27ccd2d324cf)) - Nicolas

## [1.1.0](https://github.com/bircni/cargo-binlist/compare/1.0.1..1.1.0) - 2025-05-26

### Bug Fixes

- fix license check - ([fbc58fa](https://github.com/bircni/cargo-binlist/commit/fbc58fa186c6a1a85d623256d03dd526f1e09a31)) - Nicolas

### Features

- stop skipping `cargo-binstall` as installing itself works - ([5a0f98b](https://github.com/bircni/cargo-binlist/commit/5a0f98b9be701cb04c5409f5df1b6986b81e20c3)) - Nicolas

### Build

- update release scripts - ([a20b6a7](https://github.com/bircni/cargo-binlist/commit/a20b6a79ae591d0e221b4156fa5e6f920ddf7bb2)) - Nicolas

## [1.0.1](https://github.com/bircni/cargo-binlist/compare/1.0.0..1.0.1) - 2025-04-26

### Bug Fixes

- print out with logger instead of println - ([931952d](https://github.com/bircni/cargo-binlist/commit/931952d97c756d6d4135d1d881909b89ec7b441d)) - Nicolas

## [1.0.0](https://github.com/bircni/cargo-binlist/compare/0.4.1..1.0.0) - 2025-04-26

### Refactoring

-  [**breaking**]use enum instead of optional bools - ([1fcc1b5](https://github.com/bircni/cargo-binlist/commit/1fcc1b50c5e76b7f0432b6952959cf3cba415784)) - Nicolas

### Tests

- add more tests to check table contents - ([74d73a4](https://github.com/bircni/cargo-binlist/commit/74d73a4cbe875832fe50ceb1e02318bb36a5cbd0)) - Nicolas

## [0.4.1](https://github.com/bircni/cargo-binlist/compare/0.3.0..0.4.1) - 2025-04-18

### Bug Fixes

- fix lint errors occurring in new rust version - ([bdb4247](https://github.com/bircni/cargo-binlist/commit/bdb424727a102c98ffbf346f250fb135bdbdc672)) - Nicolas
- ignore typos in Changelog as some old commits have typos - ([686d9d5](https://github.com/bircni/cargo-binlist/commit/686d9d55324cf262afab2bdc94c768cc8b12908a)) - Nicolas

### Continuous Integration

- only run on ubuntu-latest - ([5a60cc8](https://github.com/bircni/cargo-binlist/commit/5a60cc81b03593f75310ded44fca25fb2574f55f)) - Nicolas

### Features

- add build script for correct versioning - ([52d1d7a](https://github.com/bircni/cargo-binlist/commit/52d1d7a9bef5a7d40c04410f38cfd458b7313c6e)) - Nicolas
- add release scripts - ([fa09664](https://github.com/bircni/cargo-binlist/commit/fa0966417892d82b6e63b900fe3e8d9d19e90c76)) - Nicolas

## [0.3.0](https://github.com/bircni/cargo-binlist/compare/0.2.1..0.3.0) - 2025-04-18

### Bug Fixes

- Preparation for Version `0.3` (#5) - ([b649d20](https://github.com/bircni/cargo-binlist/commit/b649d204e59869a37f01abd0fb9644772364901f)) - Nicolas

## [0.2.1](https://github.com/bircni/cargo-binlist/compare/0.2.0..0.2.1) - 2025-04-18

### Bug Fixes

- errors from `0.2.0` (#3) - ([05d4615](https://github.com/bircni/cargo-binlist/commit/05d4615ca2286357878dcbfba48c23d88c4eafd0)) - Nicolas

## [0.2.0](https://github.com/bircni/cargo-binlist/compare/0.1.1..0.2.0) - 2025-04-18

### Refacor

- Reformatting & Linting (#2) - ([0c94d8e](https://github.com/bircni/cargo-binlist/commit/0c94d8e3b6fd3cdc670cc17ab5c17b8dcd83c3c6)) - Nicolas

## [0.1.1](https://github.com/bircni/cargo-binlist/compare/0.1.0..0.1.1) - 2025-04-18

### Bug Fixes

- fix deploy script (#1) - ([6e1a90c](https://github.com/bircni/cargo-binlist/commit/6e1a90cb95da9e88a20d62749e9cff44cfad0eef)) - Nicolas

### Miscellaneous Chores

- remove deploy jobs - ([146ea12](https://github.com/bircni/cargo-binlist/commit/146ea129a7855348916d58e2f8cdc682a08187e1)) - Nicolas

### Tests

- add tests - ([9ec4d59](https://github.com/bircni/cargo-binlist/commit/9ec4d59bdf25212d12d86fdaa1976defbc408d03)) - Nicolas
