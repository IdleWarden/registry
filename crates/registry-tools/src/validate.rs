// SPDX-License-Identifier: MPL-2.0

use jsonschema::Validator;
use serde_json::Value;

use crate::asset::{Asset, AssetProbe, MAX_ASSET_BYTES};
use crate::entry::{Entry, EntryVersion, ModEntry, RawEntry};
use crate::problem::Problem;

const RELEASE_ASSET_MARKER: &str = "/releases/download/";
const BRIDGE_CAPABILITY_PREFIX: &str = "bridge:";

pub struct Schemas {
    pub plugin: Validator,
    pub mod_entry: Validator,
}

pub fn schema_problems(validator: &Validator, raw: &RawEntry) -> Vec<Problem> {
    validator
        .iter_errors(&raw.value)
        .map(|error| {
            let at = error.instance_path().to_string();
            let at = if at.is_empty() { "/".to_owned() } else { at };
            Problem::new(&raw.path, format!("{at}: {error}"))
        })
        .collect()
}

pub fn filename_problem(raw: &RawEntry) -> Option<Problem> {
    let id = raw.id()?;
    let actual = raw.path.file_name()?.to_str()?;
    let expected = format!("{id}.json");
    (actual != expected).then(|| {
        Problem::new(
            &raw.path,
            format!("filename does not match id; expected {expected}"),
        )
    })
}

pub fn policy_problem(raw: &RawEntry, entry: &Entry) -> Option<Problem> {
    entry
        .multiplayer
        .then(|| Problem::new(&raw.path, "multiplayer titles are refused; see POLICY.md"))
}

pub fn url_shape_problems(raw: &RawEntry, versions: &[&EntryVersion]) -> Vec<Problem> {
    versions
        .iter()
        .filter(|version| !version.url.contains(RELEASE_ASSET_MARKER))
        .map(|version| {
            Problem::new(
                &raw.path,
                format!("{} is not a release asset URL", version.url),
            )
        })
        .collect()
}

pub fn asset_problems(
    raw: &RawEntry,
    versions: &[&EntryVersion],
    probe: &dyn AssetProbe,
) -> Vec<Problem> {
    versions
        .iter()
        .filter_map(|version| {
            let url = &version.url;
            let message = match probe.fetch(url) {
                Err(error) => format!("{url} is unreachable: {error}"),
                Ok(Asset::Status(code)) => format!("{url} returned HTTP {code}"),
                Ok(Asset::TooLarge) => {
                    format!("{url} exceeds the {MAX_ASSET_BYTES} byte limit for a package")
                }
                Ok(Asset::Fetched { sha256 }) if sha256 != version.sha256 => {
                    format!(
                        "{url} hashes to {sha256}, but the entry declares {}",
                        version.sha256
                    )
                }
                Ok(Asset::Fetched { .. }) => return None,
            };
            Some(Problem::new(&raw.path, message))
        })
        .collect()
}

/// A mod is verified by provenance, not by review (ADR-0014). An attestation
/// pointing at a repository other than the declared source would let an entry
/// borrow the credibility of a project it was not built from.
pub fn attestation_problems(raw: &RawEntry, entry: &ModEntry) -> Vec<Problem> {
    let source = normalise_repository(&entry.source.repository);

    entry
        .versions
        .iter()
        .filter(|version| {
            source.as_deref() != Some(version.attestation.repository.to_lowercase().as_str())
        })
        .map(|version| {
            Problem::new(
                &raw.path,
                format!(
                    "attestation repository `{}` does not match source `{}`",
                    version.attestation.repository, entry.source.repository
                ),
            )
        })
        .collect()
}

fn normalise_repository(url: &str) -> Option<String> {
    let path = url.to_lowercase();
    let path = path.strip_suffix('/').unwrap_or(&path);
    let path = path.strip_suffix(".git").unwrap_or(path);
    let mut segments = path.rsplit('/');
    let repo = segments.next()?;
    let owner = segments.next()?;
    (!repo.is_empty() && !owner.is_empty()).then(|| format!("{owner}/{repo}"))
}

/// A `bridge:<name>` capability is a promise that a mod provides that endpoint.
/// Nothing else in the repository can check it: the schema sees one file at a
/// time, and the two sides live in different directories.
pub fn bridge_link_problems(
    raw: &RawEntry,
    entry: &Entry,
    mods: &[(&RawEntry, ModEntry)],
) -> Vec<Problem> {
    entry
        .versions
        .iter()
        .flat_map(|version| version.capabilities.iter())
        .filter_map(|capability| capability.strip_prefix(BRIDGE_CAPABILITY_PREFIX))
        .filter(|name| {
            !mods
                .iter()
                .any(|(_, m)| m.bridge.name == *name && m.plugin == entry.id)
        })
        .map(|name| {
            Problem::new(
                &raw.path,
                format!(
                    "declares `bridge:{name}` but no entry under mods/ provides that endpoint \
                     for `{}`; see POLICY.md and ADR-0014",
                    entry.id
                ),
            )
        })
        .collect()
}

pub fn orphan_mod_problem(raw: &RawEntry, entry: &ModEntry, plugins: &[String]) -> Option<Problem> {
    (!plugins.contains(&entry.plugin)).then(|| {
        Problem::new(
            &raw.path,
            format!(
                "serves `{}`, which has no entry under plugins/",
                entry.plugin
            ),
        )
    })
}

pub fn check(
    plugins: &[RawEntry],
    mods: &[RawEntry],
    schemas: &Schemas,
    probe: Option<&dyn AssetProbe>,
) -> Vec<Problem> {
    let mut problems = Vec::new();

    let mut typed_mods: Vec<(&RawEntry, ModEntry)> = Vec::new();
    for raw in mods {
        let schema = schema_problems(&schemas.mod_entry, raw);
        if !schema.is_empty() {
            problems.extend(schema);
            continue;
        }
        match serde_json::from_value(raw.value.clone()) {
            Ok(entry) => typed_mods.push((raw, entry)),
            Err(error) => problems.push(Problem::new(&raw.path, error.to_string())),
        }
    }

    let mut plugin_ids = Vec::new();
    for raw in plugins {
        let schema = schema_problems(&schemas.plugin, raw);
        if !schema.is_empty() {
            problems.extend(schema);
            continue;
        }

        problems.extend(filename_problem(raw));

        let entry: Entry = match serde_json::from_value(raw.value.clone()) {
            Ok(entry) => entry,
            Err(error) => {
                problems.push(Problem::new(&raw.path, error.to_string()));
                continue;
            }
        };

        plugin_ids.push(entry.id.clone());
        problems.extend(policy_problem(raw, &entry));
        problems.extend(bridge_link_problems(raw, &entry, &typed_mods));

        let versions: Vec<&EntryVersion> = entry.versions.iter().collect();
        problems.extend(check_assets(raw, &versions, probe));
    }

    for (raw, entry) in &typed_mods {
        problems.extend(filename_problem(raw));
        problems.extend(orphan_mod_problem(raw, entry, &plugin_ids));
        problems.extend(attestation_problems(raw, entry));
        problems.extend(check_assets(raw, &entry.assets(), probe));
    }

    problems
}

fn check_assets(
    raw: &RawEntry,
    versions: &[&EntryVersion],
    probe: Option<&dyn AssetProbe>,
) -> Vec<Problem> {
    let shape = url_shape_problems(raw, versions);
    if !shape.is_empty() {
        return shape;
    }
    match probe {
        Some(probe) => asset_problems(raw, versions, probe),
        None => Vec::new(),
    }
}

pub fn build_validator(schema: &Value) -> anyhow::Result<Validator> {
    jsonschema::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| anyhow::anyhow!("a registry schema is itself invalid: {error}"))
}
