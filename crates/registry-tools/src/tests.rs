// SPDX-License-Identifier: MPL-2.0

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::asset::{Asset, AssetProbe, MAX_ASSET_BYTES};
use crate::entry::{Entry, RawEntry};
use crate::index;
use crate::validate;

const ASSET_URL: &str = "https://github.com/you/plugin/releases/download/v1.0.0/plugin.zip";
const DECLARED_SHA: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn entry_schema() -> Value {
    let path = repo_root().join("schema/plugin-entry.schema.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn raw(name: &str, value: Value) -> RawEntry {
    RawEntry {
        path: PathBuf::from("plugins").join(name),
        value,
    }
}

fn typed(value: &Value) -> Entry {
    serde_json::from_value(value.clone()).unwrap()
}

fn valid_entry(id: &str) -> Value {
    json!({
        "id": id,
        "name": "Cookie Clicker",
        "game": { "title": "Cookie Clicker" },
        "multiplayer": false,
        "source": { "repository": "https://github.com/you/plugin" },
        "versions": [{
            "version": "1.0.0",
            "api_version": "^0.1",
            "url": ASSET_URL,
            "sha256": DECLARED_SHA
        }]
    })
}

struct FakeProbe(HashMap<String, Result<Asset, String>>);

impl FakeProbe {
    fn stub(outcome: Result<Asset, String>) -> Self {
        Self(HashMap::from([(ASSET_URL.to_owned(), outcome)]))
    }
}

impl AssetProbe for FakeProbe {
    fn fetch(&self, url: &str) -> Result<Asset, String> {
        self.0
            .get(url)
            .cloned()
            .unwrap_or(Err("not stubbed".to_owned()))
    }
}

fn asset_problems(probe: &FakeProbe) -> Vec<String> {
    let entry = raw("dev.example.plugin.json", valid_entry("dev.example.plugin"));
    validate::asset_problems(&entry, &typed(&entry.value), probe)
        .into_iter()
        .map(|p| p.message)
        .collect()
}

#[test]
fn filename_must_match_the_entry_id() {
    let good = raw(
        "dev.example.cookie-clicker.json",
        valid_entry("dev.example.cookie-clicker"),
    );
    assert!(validate::filename_problem(&good).is_none());

    let bad = raw(
        "cookie-clicker.json",
        valid_entry("dev.example.cookie-clicker"),
    );
    let problem = validate::filename_problem(&bad).expect("mismatch must be reported");
    assert!(
        problem.message.contains("dev.example.cookie-clicker.json"),
        "{problem}"
    );
}

#[test]
fn only_immutable_release_assets_are_accepted() {
    let mut value = valid_entry("dev.example.plugin");
    value["versions"][0]["url"] =
        json!("https://github.com/you/plugin/archive/refs/heads/main.zip");
    let entry = raw("dev.example.plugin.json", value);

    let problems = validate::url_shape_problems(&entry, &typed(&entry.value));
    assert_eq!(problems.len(), 1);
    assert!(
        problems[0].message.contains("not a release asset URL"),
        "{}",
        problems[0]
    );
}

#[test]
fn multiplayer_entries_are_refused() {
    let mut value = valid_entry("dev.example.plugin");
    value["multiplayer"] = json!(true);
    let entry = raw("dev.example.plugin.json", value);

    let problem = validate::policy_problem(&entry, &typed(&entry.value)).expect("policy breach");
    assert!(problem.message.contains("POLICY.md"), "{problem}");
}

#[test]
fn a_matching_asset_raises_nothing() {
    let probe = FakeProbe::stub(Ok(Asset::Fetched {
        sha256: DECLARED_SHA.to_owned(),
    }));
    assert!(asset_problems(&probe).is_empty());
}

#[test]
fn a_missing_or_unreachable_asset_is_reported() {
    let probe = FakeProbe::stub(Ok(Asset::Status(404)));
    assert!(asset_problems(&probe)[0].contains("HTTP 404"));

    let probe = FakeProbe::stub(Err("dns failure".to_owned()));
    assert!(asset_problems(&probe)[0].contains("dns failure"));
}

#[test]
fn an_asset_whose_checksum_does_not_match_the_entry_is_reported() {
    let actual = "a".repeat(64);
    let probe = FakeProbe::stub(Ok(Asset::Fetched {
        sha256: actual.clone(),
    }));

    let message = asset_problems(&probe).remove(0);
    assert!(message.contains(&actual), "{message}");
    assert!(message.contains(DECLARED_SHA), "{message}");
}

#[test]
fn an_oversized_asset_is_refused_rather_than_hashed() {
    let probe = FakeProbe::stub(Ok(Asset::TooLarge));
    let message = asset_problems(&probe).remove(0);
    assert!(message.contains(&MAX_ASSET_BYTES.to_string()), "{message}");
}

#[test]
fn a_non_release_url_is_never_downloaded() {
    let mut value = valid_entry("dev.example.plugin");
    value["versions"][0]["url"] = json!("https://example.com/plugin.zip");
    let entry = raw("dev.example.plugin.json", value);

    let validator = validate::build_validator(&entry_schema()).unwrap();
    let probe = FakeProbe(HashMap::new());
    let problems = validate::check(std::slice::from_ref(&entry), &validator, Some(&probe));

    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].message.contains("not a release asset URL"),
        "{}",
        problems[0]
    );
}

#[test]
fn the_schema_rejects_unknown_properties_and_missing_requirements() {
    let validator = validate::build_validator(&entry_schema()).unwrap();

    let mut extra = valid_entry("dev.example.plugin");
    extra["surprise"] = json!("hello");
    assert!(!validate::schema_problems(&validator, &raw("x.json", extra)).is_empty());

    let mut incomplete = valid_entry("dev.example.plugin");
    incomplete.as_object_mut().unwrap().remove("versions");
    assert!(!validate::schema_problems(&validator, &raw("x.json", incomplete)).is_empty());

    let mut bad_id = valid_entry("dev.example.plugin");
    bad_id["id"] = json!("NotReverseDns");
    assert!(!validate::schema_problems(&validator, &raw("x.json", bad_id)).is_empty());
}

#[test]
fn the_published_example_validates_against_the_published_schema() {
    let validator = validate::build_validator(&entry_schema()).unwrap();
    let path = repo_root().join("plugins/EXAMPLE.json.template");
    let example: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();

    let problems = validate::schema_problems(&validator, &raw("EXAMPLE.json", example));
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn the_index_is_sorted_by_id_and_independent_of_input_order() {
    let forwards = [
        raw("a.json", valid_entry("dev.example.alpha")),
        raw("z.json", valid_entry("dev.example.zulu")),
    ];
    let backwards = [
        raw("z.json", valid_entry("dev.example.zulu")),
        raw("a.json", valid_entry("dev.example.alpha")),
    ];

    let built = index::build(&forwards);
    let ids: Vec<&str> = built["plugins"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["dev.example.alpha", "dev.example.zulu"]);

    assert_eq!(
        index::render(&built),
        index::render(&index::build(&backwards))
    );
}

#[test]
fn the_index_carries_no_timestamp_so_regeneration_is_a_no_op() {
    let entries = [raw("a.json", valid_entry("dev.example.alpha"))];
    assert_eq!(index::build(&entries)["generated_at"], Value::Null);
    assert_eq!(
        index::render(&index::build(&entries)),
        index::render(&index::build(&entries))
    );
}
