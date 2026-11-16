//! Roll-call test keeping `docs/migration.md` in sync with `CHANGELOG.md`.
//!
//! Every release whose `CHANGELOG.md` entry has a `BREAKING CHANGES` heading
//! needs a `## X.Y.Z` section in the migration guide. release-please writes
//! the changelog entry in the release PR, so a missing section fails that PR
//! and the release waits for the guide.
//!
//! To avoid that stop, add the section in the PR that makes the breaking
//! change, under the version release-please will cut next (the next minor
//! while the crate is `0.x`). A section for a version that is not released
//! yet is allowed for this reason.
//!
//! Rationale: three crates on crates.io sat on 0.30, 0.33 and 0.41 while the
//! current release was 0.45, and the only upgrade path was to read twelve
//! changelog entries (#992). The code examples of the guide are compiled by
//! the `MigrationDoctests` include in `src/lib.rs`.

use std::collections::BTreeSet;
use std::path::Path;

/// The guide starts here: it serves dependents on 0.30 and later, so the
/// first release it has to describe is 0.31.0. Older breaking releases stay
/// in `CHANGELOG.md` only.
const FIRST_COVERED: Version = Version(0, 31, 0);

/// A release number, ordered as Cargo orders it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Version(u32, u32, u32);

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

fn read(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Parse `X.Y.Z` at the start of `s`; whatever follows the number is ignored.
fn leading_version(s: &str) -> Option<Version> {
    let end = s
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(s.len());
    let mut parts = s[..end].split('.');
    let version = Version(
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    );
    parts.next().is_none().then_some(version)
}

/// The version of a second-level heading: `## [0.38.0](…) (date)` in the
/// changelog, `## 0.38.0` in the guide.
fn heading_version(line: &str) -> Option<Version> {
    let rest = line.strip_prefix("## ")?;
    leading_version(rest.strip_prefix('[').unwrap_or(rest))
}

/// Every released version, and those whose entry has a breaking-changes
/// heading.
fn changelog_versions() -> (BTreeSet<Version>, BTreeSet<Version>) {
    let mut released = BTreeSet::new();
    let mut breaking = BTreeSet::new();
    let mut current = None;
    for line in read("CHANGELOG.md").lines() {
        if let Some(version) = heading_version(line) {
            released.insert(version);
            current = Some(version);
        } else if line.starts_with("### ") && line.contains("BREAKING CHANGES") {
            breaking.insert(current.expect("a breaking-changes heading before any release"));
        }
    }
    (released, breaking)
}

/// The versions that have a section in the guide.
fn guide_sections() -> BTreeSet<Version> {
    read("docs/migration.md")
        .lines()
        .filter_map(heading_version)
        .collect()
}

#[test]
fn changelog_parser_still_reads_the_release_please_format() {
    // Without this check a change of heading format would empty the sets
    // below and let the roll call pass with nothing compared.
    let (released, breaking) = changelog_versions();
    assert!(
        released.contains(&Version(0, 45, 0)),
        "0.45.0 not found among the CHANGELOG.md release headings"
    );
    for known in [Version(0, 38, 0), Version(0, 45, 0)] {
        assert!(
            breaking.contains(&known),
            "{known} has a BREAKING CHANGES heading in CHANGELOG.md, but the parser missed it"
        );
    }
    assert!(
        !breaking.contains(&Version(0, 44, 0)),
        "0.44.0 has no BREAKING CHANGES heading, but the parser reported one"
    );
}

#[test]
fn migration_guide_covers_every_breaking_release() {
    let (_, breaking) = changelog_versions();
    let sections = guide_sections();
    let missing: Vec<String> = breaking
        .iter()
        .filter(|v| **v >= FIRST_COVERED && !sections.contains(v))
        .map(Version::to_string)
        .collect();
    assert!(
        missing.is_empty(),
        "CHANGELOG.md lists breaking changes for {missing:?}, but docs/migration.md has no \
         `## <version>` section for them. Add one section per release: what breaks, with the \
         code before and after."
    );
}

#[test]
fn migration_guide_has_no_section_for_a_non_breaking_release() {
    // A heading with a wrong number would satisfy nothing and mislead the
    // reader. A version above the newest release is a section written ahead
    // of that release, which is the intended workflow.
    let (released, breaking) = changelog_versions();
    let newest = *released
        .iter()
        .next_back()
        .expect("CHANGELOG.md has releases");
    let stray: Vec<String> = guide_sections()
        .iter()
        .filter(|v| **v <= newest && !breaking.contains(v))
        .map(Version::to_string)
        .collect();
    assert!(
        stray.is_empty(),
        "docs/migration.md has sections for {stray:?}, but CHANGELOG.md lists no breaking \
         change for these releases"
    );
}

#[test]
fn readme_links_the_migration_guide() {
    assert!(
        read("README.md").contains("(docs/migration.md)"),
        "README.md must link docs/migration.md from its documentation table"
    );
}
