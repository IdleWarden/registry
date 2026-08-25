// SPDX-License-Identifier: MPL-2.0

use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub struct Problem {
    pub file: PathBuf,
    pub message: String,
}

impl Problem {
    pub fn new(file: &Path, message: impl Into<String>) -> Self {
        Self {
            file: file.to_path_buf(),
            message: message.into(),
        }
    }

    pub fn annotate(&self) -> String {
        if std::env::var_os("GITHUB_ACTIONS").is_some() {
            format!("::error file={}::{}", self.file.display(), self.message)
        } else {
            self.to_string()
        }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.file.display(), self.message)
    }
}
