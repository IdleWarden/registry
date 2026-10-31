// SPDX-License-Identifier: MPL-2.0
#![no_main]

use std::path::PathBuf;
use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use registry_tools::entry::{Entry, ModEntry, RawEntry};
use registry_tools::validate::{build_validator, check, Schemas};
use serde_json::Value;

const PLUGIN_SCHEMA: &str = include_str!("../../schema/plugin-entry.schema.json");
const MOD_SCHEMA: &str = include_str!("../../schema/mod-entry.schema.json");

fn schemas() -> &'static Schemas {
    static SCHEMAS: OnceLock<Schemas> = OnceLock::new();
    SCHEMAS.get_or_init(|| {
        let build = |text: &str| {
            build_validator(&serde_json::from_str(text).expect("schema is JSON"))
                .expect("schema compiles")
        };
        Schemas {
            plugin: build(PLUGIN_SCHEMA),
            mod_entry: build(MOD_SCHEMA),
        }
    })
}

fn raw(dir: &str, value: Value) -> RawEntry {
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("entry")
        .to_owned();
    RawEntry {
        path: PathBuf::from(dir).join(format!("{id}.json")),
        value,
    }
}

fn mods_path_is_inside_the_game(path: &str) -> bool {
    !path.is_empty()
        && !path.contains("..")
        && !path.starts_with('/')
        && !path.contains(':')
        && !path.contains('\\')
}

fuzz_target!(|data: &[u8]| {
    let mut halves = data.splitn(2, |byte| *byte == b'\n');
    let plugin: Option<Value> = serde_json::from_slice(halves.next().unwrap_or_default()).ok();
    let mod_entry: Option<Value> = serde_json::from_slice(halves.next().unwrap_or_default()).ok();

    let plugins: Vec<RawEntry> = plugin.iter().cloned().map(|v| raw("plugins", v)).collect();
    let mods: Vec<RawEntry> = mod_entry.iter().cloned().map(|v| raw("mods", v)).collect();

    let problems = check(&plugins, &mods, schemas(), None);
    if !problems.is_empty() {
        return;
    }

    for raw in &plugins {
        let entry: Entry = serde_json::from_value(raw.value.clone())
            .expect("an entry that passed validation must deserialize");
        assert!(!entry.multiplayer, "a multiplayer plugin passed validation");
        assert!(entry
            .versions
            .iter()
            .all(|version| version.url.contains("/releases/download/")));
    }
    for raw in &mods {
        let entry: ModEntry = serde_json::from_value(raw.value.clone())
            .expect("an entry that passed validation must deserialize");
        if let Some(path) = entry.mods_path.as_deref() {
            assert!(
                mods_path_is_inside_the_game(path),
                "unsafe mods_path `{path}`"
            );
        }
    }
});
