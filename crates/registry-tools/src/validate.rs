// SPDX-License-Identifier: MPL-2.0

use jsonschema::Validator;
use serde_json::Value;

use crate::asset::{Asset, AssetProbe, MAX_ASSET_BYTES};
use crate::entry::{Entry, RawEntry};
use crate::problem::Problem;

const RELEASE_ASSET_MARKER: &str = "/releases/download/";
const BRIDGE_CAPABILITY_PREFIX: &str = "bridge:";

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

/// Runs before schema validation on purpose: the entry schema does not list
/// `bridge:` among its capability patterns, so a bridge entry would otherwise
/// fail with an opaque pattern mismatch instead of the reason it was refused.
pub fn bridge_problems(raw: &RawEntry) -> Vec<Problem> {
    let Some(versions) = raw.value.get("versions").and_then(Value::as_array) else {
        return Vec::new();
    };

    versions
        .iter()
        .filter_map(|version| version.get("capabilities")?.as_array())
        .flatten()
        .filter_map(Value::as_str)
        .filter(|capability| capability.starts_with(BRIDGE_CAPABILITY_PREFIX))
        .map(|capability| {
            Problem::new(
                &raw.path,
                format!(
                    "`{capability}` needs a mod inside the game process; the registry does not                      distribute bridge plugins. See POLICY.md and ADR-0014."
                ),
            )
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

pub fn url_shape_problems(raw: &RawEntry, entry: &Entry) -> Vec<Problem> {
    entry
        .versions
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

pub fn asset_problems(raw: &RawEntry, entry: &Entry, probe: &dyn AssetProbe) -> Vec<Problem> {
    entry
        .versions
        .iter()
        .filter_map(|version| {
            let url = &version.url;
            let message = match probe.fetch(url) {
                Err(error) => format!("{url} is unreachable: {error}"),
                Ok(Asset::Status(code)) => format!("{url} returned HTTP {code}"),
                Ok(Asset::TooLarge) => {
                    format!("{url} exceeds the {MAX_ASSET_BYTES} byte limit for a plugin package")
                }
                Ok(Asset::Fetched { sha256 }) if sha256 != version.sha256 => format!(
                    "{url} hashes to {sha256}, but the entry declares {}",
                    version.sha256
                ),
                Ok(Asset::Fetched { .. }) => return None,
            };
            Some(Problem::new(&raw.path, message))
        })
        .collect()
}

pub fn check(
    entries: &[RawEntry],
    validator: &Validator,
    probe: Option<&dyn AssetProbe>,
) -> Vec<Problem> {
    let mut problems = Vec::new();

    for raw in entries {
        let bridges = bridge_problems(raw);
        if !bridges.is_empty() {
            problems.extend(bridges);
            continue;
        }

        let schema = schema_problems(validator, raw);
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

        problems.extend(policy_problem(raw, &entry));

        let shape = url_shape_problems(raw, &entry);
        let reachable = shape.is_empty();
        problems.extend(shape);

        if let (true, Some(probe)) = (reachable, probe) {
            problems.extend(asset_problems(raw, &entry, probe));
        }
    }

    problems
}

pub fn build_validator(schema: &Value) -> anyhow::Result<Validator> {
    jsonschema::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| anyhow::anyhow!("the entry schema is itself invalid: {error}"))
}
