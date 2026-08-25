// SPDX-License-Identifier: MPL-2.0

use sha2::{Digest, Sha256};

pub const MAX_ASSET_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asset {
    Fetched { sha256: String },
    Status(u16),
    TooLarge,
}

pub trait AssetProbe {
    fn fetch(&self, url: &str) -> Result<Asset, String>;
}

pub struct HttpProbe;

impl AssetProbe for HttpProbe {
    fn fetch(&self, url: &str) -> Result<Asset, String> {
        let mut response = match ureq::get(url).call() {
            Ok(response) => response,
            Err(ureq::Error::StatusCode(code)) => return Ok(Asset::Status(code)),
            Err(error) => return Err(error.to_string()),
        };

        let bytes = response
            .body_mut()
            .with_config()
            .limit(MAX_ASSET_BYTES + 1)
            .read_to_vec()
            .map_err(|error| error.to_string())?;

        if bytes.len() as u64 > MAX_ASSET_BYTES {
            return Ok(Asset::TooLarge);
        }

        Ok(Asset::Fetched {
            sha256: hex(&Sha256::digest(&bytes)),
        })
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
