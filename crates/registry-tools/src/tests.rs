// SPDX-License-Identifier: MPL-2.0

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::asset::{Asset, AssetProbe, MAX_ASSET_BYTES};
use crate::entry::{Entry, EntryVersion, ModEntry, RawEntry};
use crate::index;
use crate::problem::Problem;
use crate::validate::{self, Schemas};

const ASSET_URL: &str = "https://github.com/you/plugin/releases/download/v1.0.0/plugin.zip";
const MOD_URL: &str = "https://github.com/you/mod/releases/download/v1.0.0/mod.zip";
const DECLARED_SHA: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn schema(name: &str) -> Value {
    let path = repo_root().join("schema").join(name);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn schemas() -> Schemas {
    Schemas {
        plugin: validate::build_validator(&schema("plugin-entry.schema.json")).unwrap(),
        mod_entry: validate::build_validator(&schema("mod-entry.schema.json")).unwrap(),
    }
}

fn raw(name: &str, value: Value) -> RawEntry {
    RawEntry {
        path: PathBuf::from("entries").join(name),
        value,
    }
}

fn typed<T: serde::de::DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}

fn assets(entry: &Entry) -> Vec<&EntryVersion> {
    entry.versions.iter().collect()
}

fn messages(problems: Vec<Problem>) -> Vec<String> {
    problems.into_iter().map(|p| p.message).collect()
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

fn valid_mod(id: &str, plugin: &str, bridge: &str) -> Value {
    json!({
        "id": id,
        "name": "Cookie Clicker bridge",
        "plugin": plugin,
        "bridge": { "name": bridge },
        "loader": "bepinex",
        "source": { "repository": "https://github.com/you/mod" },
        "versions": [{
            "version": "1.0.0",
            "api_version": "^0.1",
            "url": MOD_URL,
            "sha256": DECLARED_SHA,
            "attestation": { "repository": "you/mod" }
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
    let parsed: Entry = typed(&entry.value);
    messages(validate::asset_problems(&entry, &assets(&parsed), probe))
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
    let parsed: Entry = typed(&entry.value);

    let problems = validate::url_shape_problems(&entry, &assets(&parsed));
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

    let probe = FakeProbe(HashMap::new());
    let problems = validate::check(std::slice::from_ref(&entry), &[], &schemas(), Some(&probe));

    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].message.contains("not a release asset URL"),
        "{}",
        problems[0]
    );
}

#[test]
fn the_schema_rejects_unknown_properties_and_missing_requirements() {
    let validator = validate::build_validator(&schema("plugin-entry.schema.json")).unwrap();

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
fn a_mod_entry_without_provenance_is_rejected_by_its_schema() {
    let validator = validate::build_validator(&schema("mod-entry.schema.json")).unwrap();

    let mut missing_attestation = valid_mod("dev.example.mod", "dev.example.plugin", "cookie");
    missing_attestation["versions"][0]
        .as_object_mut()
        .unwrap()
        .remove("attestation");
    assert!(!validate::schema_problems(&validator, &raw("m.json", missing_attestation)).is_empty());

    let mut missing_source = valid_mod("dev.example.mod", "dev.example.plugin", "cookie");
    missing_source.as_object_mut().unwrap().remove("source");
    assert!(!validate::schema_problems(&validator, &raw("m.json", missing_source)).is_empty());
}

#[test]
fn both_published_examples_validate_against_their_schemas() {
    for (template, schema_name) in [
        ("plugins/EXAMPLE.json.template", "plugin-entry.schema.json"),
        ("mods/EXAMPLE.json.template", "mod-entry.schema.json"),
    ] {
        let validator = validate::build_validator(&schema(schema_name)).unwrap();
        let path = repo_root().join(template);
        let example: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();

        let problems = validate::schema_problems(&validator, &raw("EXAMPLE.json", example));
        assert!(problems.is_empty(), "{template}: {problems:?}");
    }
}

#[test]
fn a_bridge_capability_without_a_mod_that_provides_it_is_refused() {
    let mut value = valid_entry("dev.example.plugin");
    value["versions"][0]["capabilities"] = json!(["capture", "bridge:cookie"]);
    let entry = raw("dev.example.plugin.json", value);

    let problems = messages(validate::bridge_link_problems(
        &entry,
        &typed(&entry.value),
        &[],
    ));
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("bridge:cookie"), "{}", problems[0]);
    assert!(problems[0].contains("ADR-0014"), "{}", problems[0]);
}

#[test]
fn a_bridge_capability_backed_by_a_mod_for_that_plugin_is_accepted() {
    let mut value = valid_entry("dev.example.plugin");
    value["versions"][0]["capabilities"] = json!(["bridge:cookie"]);
    let entry = raw("dev.example.plugin.json", value);

    let mod_raw = raw(
        "m.json",
        valid_mod("dev.example.mod", "dev.example.plugin", "cookie"),
    );
    let mods = [(&mod_raw, typed::<ModEntry>(&mod_raw.value))];

    assert!(validate::bridge_link_problems(&entry, &typed(&entry.value), &mods).is_empty());
}

#[test]
fn a_mod_for_a_different_plugin_does_not_satisfy_the_capability() {
    let mut value = valid_entry("dev.example.plugin");
    value["versions"][0]["capabilities"] = json!(["bridge:cookie"]);
    let entry = raw("dev.example.plugin.json", value);

    let mod_raw = raw(
        "m.json",
        valid_mod("dev.example.mod", "dev.example.other", "cookie"),
    );
    let mods = [(&mod_raw, typed::<ModEntry>(&mod_raw.value))];

    assert!(!validate::bridge_link_problems(&entry, &typed(&entry.value), &mods).is_empty());
}

#[test]
fn a_mod_serving_a_plugin_that_does_not_exist_is_refused() {
    let mod_raw = raw(
        "m.json",
        valid_mod("dev.example.mod", "dev.example.ghost", "cookie"),
    );
    let known = ["dev.example.other".to_owned()];

    let problem = validate::orphan_mod_problem(&mod_raw, &typed(&mod_raw.value), &known)
        .expect("orphan must be reported");
    assert!(problem.message.contains("dev.example.ghost"), "{problem}");
}

#[test]
fn an_attestation_pointing_elsewhere_cannot_borrow_another_repository_credibility() {
    let mut value = valid_mod("dev.example.mod", "dev.example.plugin", "cookie");
    value["versions"][0]["attestation"]["repository"] = json!("someone-else/trusted");
    let mod_raw = raw("m.json", value);

    let problems = messages(validate::attestation_problems(
        &mod_raw,
        &typed(&mod_raw.value),
    ));
    assert_eq!(problems.len(), 1);
    assert!(
        problems[0].contains("someone-else/trusted"),
        "{}",
        problems[0]
    );
}

#[test]
fn attestation_matching_tolerates_the_shapes_a_repository_url_actually_takes() {
    for repository in [
        "https://github.com/you/mod",
        "https://github.com/you/mod/",
        "https://github.com/you/mod.git",
        "https://github.com/You/Mod",
    ] {
        let mut value = valid_mod("dev.example.mod", "dev.example.plugin", "cookie");
        value["source"]["repository"] = json!(repository);
        let mod_raw = raw("m.json", value);

        let problems = validate::attestation_problems(&mod_raw, &typed(&mod_raw.value));
        assert!(
            problems.is_empty(),
            "{repository} was rejected: {problems:?}"
        );
    }
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

    let built = index::build(&forwards, &[]);
    let ids: Vec<&str> = built["plugins"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["dev.example.alpha", "dev.example.zulu"]);

    assert_eq!(
        index::render(&built),
        index::render(&index::build(&backwards, &[]))
    );
}

#[test]
fn the_index_carries_mods_alongside_plugins() {
    let plugins = [raw("a.json", valid_entry("dev.example.alpha"))];
    let mods = [raw(
        "m.json",
        valid_mod("dev.example.mod", "dev.example.alpha", "cookie"),
    )];

    let built = index::build(&plugins, &mods);
    assert_eq!(built["mods"].as_array().unwrap().len(), 1);
    assert_eq!(built["mods"][0]["bridge"]["name"], "cookie");
}

#[test]
fn the_index_carries_no_timestamp_so_regeneration_is_a_no_op() {
    let entries = [raw("a.json", valid_entry("dev.example.alpha"))];
    assert_eq!(index::build(&entries, &[])["generated_at"], Value::Null);
    assert_eq!(
        index::render(&index::build(&entries, &[])),
        index::render(&index::build(&entries, &[]))
    );
}
