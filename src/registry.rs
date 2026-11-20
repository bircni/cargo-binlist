//! Looking up published versions in the crates.io sparse index.

use std::{
    cmp::Ordering,
    sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering as AtomicOrdering},
    },
    thread,
    time::Duration,
};

use anyhow::Context as _;
use semver::{Version, VersionReq};
use serde::Deserialize;

use crate::package::{InstalledPackage, Report, Source, Status};

const CRATES_IO_INDEX: &str = "https://index.crates.io";
const USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (+",
    env!("CARGO_PKG_REPOSITORY"),
    ")"
);
const TIMEOUT: Duration = Duration::from_secs(15);
const MAX_WORKERS: usize = 8;

/// A single published version of a crate.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Published {
    #[serde(rename = "vers")]
    pub version: Version,
    #[serde(default)]
    pub yanked: bool,
}

/// Something that knows which versions of a crate are published.
pub trait VersionSource: Sync {
    /// All published versions of `name`, or `None` if the crate does not exist.
    fn versions(&self, name: &str) -> anyhow::Result<Option<Vec<Published>>>;
}

/// Client for a sparse registry index such as `https://index.crates.io`.
pub struct SparseIndex {
    agent: ureq::Agent,
    base: String,
}

impl SparseIndex {
    pub fn crates_io() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .user_agent(USER_AGENT)
            .build()
            .into();
        Self {
            agent,
            base: CRATES_IO_INDEX.to_owned(),
        }
    }
}

impl VersionSource for SparseIndex {
    fn versions(&self, name: &str) -> anyhow::Result<Option<Vec<Published>>> {
        let Some(path) = index_path(name) else {
            return Ok(None);
        };
        let url = format!("{}/{path}", self.base);
        log::debug!("Fetching {url}");

        match self.agent.get(&url).call() {
            Ok(mut response) => {
                let body = response
                    .body_mut()
                    .read_to_string()
                    .with_context(|| format!("Failed to read {url}"))?;
                Ok(Some(parse_index_file(&body)))
            }
            Err(ureq::Error::StatusCode(404 | 410 | 451)) => Ok(None),
            Err(e) => Err(e).with_context(|| format!("Failed to fetch {url}")),
        }
    }
}

/// Path of a crate's file within a sparse index, see
/// <https://doc.rust-lang.org/cargo/reference/registry-index.html#index-files>.
fn index_path(name: &str) -> Option<String> {
    if name.is_empty() || !name.is_ascii() {
        return None;
    }
    let name = name.to_ascii_lowercase();
    Some(match name.len() {
        1 => format!("1/{name}"),
        2 => format!("2/{name}"),
        3 => format!("3/{}/{name}", name.get(..1)?),
        _ => format!("{}/{}/{name}", name.get(..2)?, name.get(2..4)?),
    })
}

/// Parse an index file: one JSON object per line. Unparsable lines are skipped.
fn parse_index_file(body: &str) -> Vec<Published> {
    body.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// Pick the newest non-yanked version an update may move to.
///
/// Prereleases are only considered if requested or if the installed version is
/// itself a prerelease. A `--version` requirement recorded at install time is
/// honored.
pub fn select_latest(
    published: &[Published],
    installed: &InstalledPackage,
    include_pre: bool,
) -> Option<Version> {
    let include_pre = include_pre || !installed.version.pre.is_empty();
    published
        .iter()
        .filter(|p| !p.yanked)
        .map(|p| &p.version)
        .filter(|v| include_pre || v.pre.is_empty())
        .filter(|v| {
            installed
                .version_req
                .as_ref()
                .is_none_or(|req| matches_req(req, v))
        })
        .max()
        .cloned()
}

/// `VersionReq::matches` rejects prereleases unless the requirement names one;
/// with `--pre` we still want e.g. `^1` to accept `1.2.0-beta.1`.
fn matches_req(req: &VersionReq, version: &Version) -> bool {
    req.matches(version) || {
        let mut release = version.clone();
        release.pre = semver::Prerelease::EMPTY;
        req.matches(&release)
    }
}

/// Determine the status of a single package.
pub fn check(package: &InstalledPackage, source: &dyn VersionSource, include_pre: bool) -> Status {
    match &package.source {
        Source::CratesIo => {}
        Source::Registry(_) | Source::Git(_) | Source::Path(_) => return Status::Skipped,
    }

    match source.versions(&package.name) {
        Ok(Some(published)) => select_latest(&published, package, include_pre)
            .map_or(Status::NotFound, |latest| compare(&package.version, latest)),
        Ok(None) => Status::NotFound,
        Err(e) => Status::Error {
            message: format!("{e:#}"),
        },
    }
}

fn compare(installed: &Version, latest: Version) -> Status {
    match installed.cmp_precedence(&latest) {
        Ordering::Less => Status::Outdated { latest },
        Ordering::Equal => Status::UpToDate,
        Ordering::Greater => Status::Ahead { latest },
    }
}

/// Check all packages, querying the registry from a small pool of threads.
pub fn check_all(
    packages: Vec<InstalledPackage>,
    source: &dyn VersionSource,
    include_pre: bool,
) -> Vec<Report> {
    let results: Vec<OnceLock<Status>> = packages.iter().map(|_| OnceLock::new()).collect();
    let next = AtomicUsize::new(0);
    let workers = packages.len().clamp(1, MAX_WORKERS);

    thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, AtomicOrdering::Relaxed);
                    let (Some(package), Some(slot)) = (packages.get(index), results.get(index))
                    else {
                        break;
                    };
                    let _ = slot.set(check(package, source, include_pre));
                }
            });
        }
    });

    packages
        .into_iter()
        .zip(results)
        .map(|(package, status)| Report {
            package,
            status: status.into_inner().unwrap_or_else(|| Status::Error {
                message: "lookup did not complete".to_owned(),
            }),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests")]

    use std::collections::HashMap;

    use super::*;

    /// In-memory version source for tests.
    #[derive(Default)]
    struct FakeIndex {
        crates: HashMap<String, Vec<Published>>,
        failing: Vec<String>,
    }

    impl FakeIndex {
        fn with(mut self, name: &str, versions: &[&str]) -> Self {
            let published = versions
                .iter()
                .map(|v| {
                    let (version, yanked) = v
                        .strip_suffix(" (yanked)")
                        .map_or((*v, false), |v| (v, true));
                    Published {
                        version: Version::parse(version).unwrap(),
                        yanked,
                    }
                })
                .collect();
            self.crates.insert(name.to_owned(), published);
            self
        }

        fn failing(mut self, name: &str) -> Self {
            self.failing.push(name.to_owned());
            self
        }
    }

    impl VersionSource for FakeIndex {
        fn versions(&self, name: &str) -> anyhow::Result<Option<Vec<Published>>> {
            if self.failing.iter().any(|n| n == name) {
                anyhow::bail!("connection reset");
            }
            Ok(self.crates.get(name).cloned())
        }
    }

    fn pkg(name: &str, version: &str) -> InstalledPackage {
        InstalledPackage::new(name, Version::parse(version).unwrap())
    }

    fn latest(index: &FakeIndex, package: &InstalledPackage, pre: bool) -> Option<Version> {
        let published = index.versions(&package.name).unwrap().unwrap();
        select_latest(&published, package, pre)
    }

    #[test]
    fn index_paths_follow_cargo_layout() {
        assert_eq!(index_path("a").as_deref(), Some("1/a"));
        assert_eq!(index_path("ab").as_deref(), Some("2/ab"));
        assert_eq!(index_path("abc").as_deref(), Some("3/a/abc"));
        assert_eq!(
            index_path("Cargo-Deny").as_deref(),
            Some("ca/rg/cargo-deny")
        );
        assert_eq!(index_path(""), None);
        assert_eq!(index_path("crä"), None);
    }

    #[test]
    fn parses_index_lines_and_skips_garbage() {
        let body = concat!(
            r#"{"name":"x","vers":"1.0.0","deps":[],"cksum":"","features":{},"yanked":false}"#,
            "\n\nnot json\n",
            r#"{"name":"x","vers":"1.1.0","yanked":true}"#,
            "\n"
        );
        let published = parse_index_file(body);
        assert_eq!(
            published,
            [
                Published {
                    version: Version::new(1, 0, 0),
                    yanked: false
                },
                Published {
                    version: Version::new(1, 1, 0),
                    yanked: true
                },
            ]
        );
    }

    #[test]
    fn ignores_yanked_and_prerelease_versions() {
        let index =
            FakeIndex::default().with("x", &["1.0.0", "1.1.0", "1.2.0 (yanked)", "2.0.0-alpha.1"]);
        assert_eq!(
            latest(&index, &pkg("x", "1.0.0"), false),
            Some(Version::new(1, 1, 0))
        );
        assert_eq!(
            latest(&index, &pkg("x", "1.0.0"), true),
            Some(Version::parse("2.0.0-alpha.1").unwrap())
        );
    }

    #[test]
    fn installed_prerelease_considers_prereleases() {
        let index = FakeIndex::default().with("x", &["0.7.9", "0.8.0-alpha.1", "0.8.0-alpha.2"]);
        assert_eq!(
            latest(&index, &pkg("x", "0.8.0-alpha.1"), false),
            Some(Version::parse("0.8.0-alpha.2").unwrap())
        );
    }

    #[test]
    fn honors_recorded_version_requirement() {
        let index = FakeIndex::default().with("x", &["1.0.0", "1.5.0", "2.0.0", "2.1.0-rc.1"]);
        let mut package = pkg("x", "1.0.0");
        package.version_req = Some(VersionReq::parse("^1").unwrap());
        assert_eq!(latest(&index, &package, false), Some(Version::new(1, 5, 0)));

        package.version_req = Some(VersionReq::parse("^2").unwrap());
        assert_eq!(
            latest(&index, &package, true),
            Some(Version::parse("2.1.0-rc.1").unwrap())
        );
    }

    #[test]
    fn non_crates_io_sources_are_skipped() {
        // Regression: path and git installs used to be compared against an
        // unrelated crates.io crate with the same name.
        let index = FakeIndex::default().with("my-tool", &["9.9.9"]);
        let mut package = pkg("my-tool", "0.1.0");

        for source in [
            Source::Path("/home/me/my-tool".to_owned()),
            Source::Git("https://example.com/my-tool#abc".to_owned()),
            Source::Registry("https://my.registry/".to_owned()),
        ] {
            package.source = source;
            assert_eq!(check(&package, &index, false), Status::Skipped);
        }
    }

    #[test]
    fn lookup_failures_are_reported_as_errors() {
        // Regression: failed lookups used to be indistinguishable from crates
        // that do not exist.
        let index = FakeIndex::default().failing("flaky");
        assert_eq!(
            check(&pkg("flaky", "1.0.0"), &index, false),
            Status::Error {
                message: "connection reset".to_owned()
            }
        );
        assert_eq!(
            check(&pkg("missing", "1.0.0"), &index, false),
            Status::NotFound
        );
    }

    #[test]
    fn only_yanked_or_prerelease_versions_mean_not_found() {
        let index = FakeIndex::default().with("x", &["1.0.0 (yanked)", "2.0.0-beta.1"]);
        assert_eq!(check(&pkg("x", "1.0.0"), &index, false), Status::NotFound);
    }

    #[test]
    fn checks_all_packages_and_keeps_order() {
        let index = FakeIndex::default()
            .with("current", &["1.0.0"])
            .with("old", &["1.0.0", "1.1.0"])
            .with("ahead", &["0.9.0"])
            .failing("flaky");
        let packages = vec![
            pkg("current", "1.0.0"),
            pkg("old", "1.0.0"),
            pkg("ahead", "1.0.0"),
            pkg("flaky", "1.0.0"),
            pkg("missing", "1.0.0"),
        ];

        let statuses: Vec<_> = check_all(packages, &index, false)
            .into_iter()
            .map(|r| (r.package.name, r.status))
            .collect();

        assert_eq!(
            statuses,
            [
                ("current".to_owned(), Status::UpToDate),
                (
                    "old".to_owned(),
                    Status::Outdated {
                        latest: Version::new(1, 1, 0)
                    }
                ),
                (
                    "ahead".to_owned(),
                    Status::Ahead {
                        latest: Version::new(0, 9, 0)
                    }
                ),
                (
                    "flaky".to_owned(),
                    Status::Error {
                        message: "connection reset".to_owned()
                    }
                ),
                ("missing".to_owned(), Status::NotFound),
            ]
        );
    }

    #[test]
    fn check_all_handles_no_packages() {
        assert!(
            check_all(Vec::new(), &FakeIndex::default(), false).is_empty(),
            "expected no reports"
        );
    }
}
