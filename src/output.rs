//! Rendering reports as tables and JSON.

use std::fmt::Write as _;

use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table, presets};
use semver::Version;
use serde::Serialize;

use crate::package::{Report, Source, Status};

/// Table with one row per package.
pub fn table(reports: &[&Report], uncondensed: bool) -> Table {
    let preset = if uncondensed {
        presets::UTF8_FULL
    } else {
        presets::UTF8_FULL_CONDENSED
    };
    let header = ["Name", "Installed", "Latest", "Status"]
        .map(|title| Cell::new(title).add_attribute(Attribute::Bold));

    let mut table = Table::new();
    table
        .load_style(preset.with_rounded_corners())
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(header);

    for report in reports {
        table.add_row([
            Cell::new(&report.package.name),
            Cell::new(&report.package.version),
            Cell::new(latest_column(report)),
            status_cell(report),
        ]);
    }
    table
}

fn latest_column(report: &Report) -> String {
    if report.status == Status::UpToDate {
        return report.package.version.to_string();
    }
    report
        .status
        .latest()
        .map_or_else(|| "-".to_owned(), Version::to_string)
}

fn status_cell(report: &Report) -> Cell {
    match &report.status {
        Status::Outdated { .. } => Cell::new("update available")
            .fg(Color::Green)
            .add_attribute(Attribute::Bold),
        Status::UpToDate => Cell::new("up to date").fg(Color::Blue),
        Status::Ahead { .. } => Cell::new("newer than crates.io").add_attribute(Attribute::Dim),
        Status::NotFound => Cell::new("not on crates.io").fg(Color::Yellow),
        Status::Skipped => Cell::new(format!("skipped ({})", report.package.source.kind()))
            .add_attribute(Attribute::Dim),
        Status::Error { .. } => Cell::new("lookup failed").fg(Color::Red),
    }
}

#[derive(Serialize)]
struct JsonRow<'a> {
    name: &'a str,
    version: &'a Version,
    source: &'a Source,
    #[serde(flatten)]
    status: &'a Status,
}

/// Machine readable report.
pub fn json(reports: &[&Report]) -> anyhow::Result<String> {
    let rows: Vec<_> = reports
        .iter()
        .map(|r| JsonRow {
            name: &r.package.name,
            version: &r.package.version,
            source: &r.package.source,
            status: &r.status,
        })
        .collect();
    Ok(serde_json::to_string_pretty(&rows)?)
}

/// One line summary, e.g. `12 packages, 2 outdated, 1 lookup failed`.
pub fn summary(reports: &[Report]) -> String {
    let outdated = reports.iter().filter(|r| r.is_outdated()).count();
    let failed = reports
        .iter()
        .filter(|r| matches!(r.status, Status::Error { .. }))
        .count();

    let mut summary = format!(
        "{} {}, {outdated} outdated",
        reports.len(),
        plural(reports.len(), "package", "packages")
    );
    if failed > 0 {
        let _ = write!(
            summary,
            ", {failed} {} failed",
            plural(failed, "lookup", "lookups")
        );
    }
    summary
}

const fn plural<'a>(count: usize, one: &'a str, many: &'a str) -> &'a str {
    if count == 1 { one } else { many }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests")]

    use super::*;
    use crate::package::InstalledPackage;

    fn reports() -> Vec<Report> {
        let report = |name: &str, source: Source, status: Status| {
            let mut package = InstalledPackage::new(name, Version::new(1, 0, 0));
            package.source = source;
            Report { package, status }
        };
        vec![
            report(
                "cargo-deny",
                Source::CratesIo,
                Status::Outdated {
                    latest: Version::new(1, 2, 0),
                },
            ),
            report("bat", Source::CratesIo, Status::UpToDate),
            report(
                "local",
                Source::Path("/src/local".to_owned()),
                Status::Skipped,
            ),
            report(
                "flaky",
                Source::CratesIo,
                Status::Error {
                    message: "timeout".to_owned(),
                },
            ),
        ]
    }

    #[test]
    fn renders_table() {
        let reports = reports();
        let refs: Vec<_> = reports.iter().collect();
        insta::assert_snapshot!(table(&refs, false).to_string());
    }

    #[test]
    fn uncondensed_table_separates_rows() {
        let reports = reports();
        let refs: Vec<_> = reports.iter().collect();
        let condensed = table(&refs, false).to_string();
        let uncondensed = table(&refs, true).to_string();
        assert!(
            uncondensed.lines().count() > condensed.lines().count(),
            "uncondensed layout should use more lines"
        );
    }

    #[test]
    fn renders_json() {
        let reports = reports();
        let refs: Vec<_> = reports.iter().collect();
        insta::assert_snapshot!(json(&refs).unwrap());
    }

    #[test]
    fn summarizes() {
        assert_eq!(
            summary(&reports()),
            "4 packages, 1 outdated, 1 lookup failed"
        );
        assert_eq!(summary(&reports()[1..2]), "1 package, 0 outdated");
    }
}
