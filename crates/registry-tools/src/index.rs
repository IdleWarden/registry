// SPDX-License-Identifier: MPL-2.0

use serde_json::{json, Value};

use crate::entry::RawEntry;

pub const SCHEMA_VERSION: &str = "1.1.0";
pub const API_VERSION: &str = "0.1.0";

const GENERATED_NOTICE: &str = "Generated from plugins/*.json and mods/*.json by CI. \
                                Do not edit by hand.";

pub fn build(plugins: &[RawEntry], mods: &[RawEntry]) -> Value {
    json!({
        "$comment": GENERATED_NOTICE,
        "schema_version": SCHEMA_VERSION,
        "api_version": API_VERSION,
        "generated_at": Value::Null,
        "plugins": sorted_by_id(plugins),
        "mods": sorted_by_id(mods),
    })
}

fn sorted_by_id(entries: &[RawEntry]) -> Vec<Value> {
    let mut values: Vec<Value> = entries.iter().map(|e| e.value.clone()).collect();
    values.sort_by(|a, b| {
        a.get("id")
            .and_then(Value::as_str)
            .cmp(&b.get("id").and_then(Value::as_str))
    });
    values
}

pub fn render(index: &Value) -> String {
    let mut text = serde_json::to_string_pretty(index).expect("an index is always serialisable");
    text.push('\n');
    text
}
