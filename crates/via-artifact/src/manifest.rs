use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// One artifact in a run directory: file name and content hash.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactEntry {
    pub file: String,
    pub blake3: String,
}

/// The run manifest: everything needed to identify, verify, and resume a run.
/// Maps are `BTreeMap` so serialization order never depends on hash state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunManifest {
    pub stage: String,
    pub seed: u64,
    /// The stage's full config, echoed verbatim.
    pub config: serde_json::Value,
    pub crate_versions: BTreeMap<String, String>,
    pub artifacts: BTreeMap<String, ArtifactEntry>,
}

impl RunManifest {
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json + "\n")
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let bytes = std::fs::read(path)?;
        serde_json::from_slice(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}
