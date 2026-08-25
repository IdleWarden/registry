// SPDX-License-Identifier: MPL-2.0

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug)]
pub struct RawEntry {
    pub path: PathBuf,
    pub value: Value,
}

#[derive(Debug, Deserialize)]
pub struct Entry {
    pub multiplayer: bool,
    pub versions: Vec<EntryVersion>,
}

#[derive(Debug, Deserialize)]
pub struct EntryVersion {
    pub url: String,
    pub sha256: String,
}

pub fn load_all(dir: &Path) -> Result<Vec<RawEntry>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("cannot read {}", dir.display()))?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();

    paths.into_iter().map(read).collect()
}

fn read(path: PathBuf) -> Result<RawEntry> {
    let text =
        fs::read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?;
    let value = serde_json::from_str(&text)
        .with_context(|| format!("{} is not valid JSON", path.display()))?;
    Ok(RawEntry { path, value })
}

impl RawEntry {
    pub fn id(&self) -> Option<&str> {
        self.value.get("id").and_then(Value::as_str)
    }
}
