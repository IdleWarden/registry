// SPDX-License-Identifier: MPL-2.0
#![no_main]

use std::path::PathBuf;

use libfuzzer_sys::fuzz_target;
use registry_tools::entry::RawEntry;
use registry_tools::index::{build, render};
use serde_json::Value;

fn entries(values: Vec<Value>) -> Vec<RawEntry> {
    values
        .into_iter()
        .map(|value| RawEntry {
            path: PathBuf::from("entry.json"),
            value,
        })
        .collect()
}

fn ids(index: &Value, section: &str) -> Vec<Option<String>> {
    index[section]
        .as_array()
        .expect("section is an array")
        .iter()
        .map(|entry| entry.get("id").and_then(Value::as_str).map(str::to_owned))
        .collect()
}

fuzz_target!(|data: &[u8]| {
    let Ok(Value::Array(values)) = serde_json::from_slice::<Value>(data) else {
        return;
    };
    let (plugins, mods) = values.split_at(values.len() / 2);
    let plugins = entries(plugins.to_vec());
    let mods = entries(mods.to_vec());

    let index = build(&plugins, &mods);
    assert_eq!(
        index["plugins"].as_array().map(Vec::len),
        Some(plugins.len())
    );
    assert_eq!(index["mods"].as_array().map(Vec::len), Some(mods.len()));
    for section in ["plugins", "mods"] {
        let found = ids(&index, section);
        assert!(
            found.windows(2).all(|pair| pair[0] <= pair[1]),
            "{section} not sorted"
        );
    }

    let rendered = render(&index);
    assert!(rendered.ends_with('\n'));
    let reparsed: Value = serde_json::from_str(&rendered).expect("rendered index is JSON");
    assert_eq!(reparsed, index);
    assert_eq!(render(&build(&plugins, &mods)), rendered);
});
