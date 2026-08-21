use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub const MANIFEST_VERSION: u32 = 2;

/// One artifact in a run directory: file name and content hash.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactEntry {
    pub file: String,
    pub blake3: String,
}

/// One stage instance's record: its config echoed verbatim, the crates
/// that produced it, and its artifacts with content hashes (the
/// determinism witness).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageRecord {
    /// The stage's full config, echoed verbatim.
    pub config: serde_json::Value,
    pub crate_versions: BTreeMap<String, String>,
    pub artifacts: BTreeMap<String, ArtifactEntry>,
}

/// The run manifest: everything needed to identify, verify, and resume a
/// run. One record per stage instance, keyed by the stage's output
/// namespace — "terrain", "ecology", "suitability.<label>" (label-namespaced
/// stages get one record per config; ADR 0011 Consequences). Maps are
/// `BTreeMap` so serialization order never depends on hash state.
///
/// Version 1 carried a single stage; `load` migrates v1 files
/// transparently and `save` always writes the current version.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunManifest {
    pub manifest_version: u32,
    /// The world seed (the terrain stage's; downstream stages are
    /// deterministic functions of the artifacts).
    pub seed: u64,
    pub stages: BTreeMap<String, StageRecord>,
}

/// The version-1 on-disk shape, kept for migration only.
#[derive(Deserialize)]
struct RunManifestV1 {
    stage: String,
    seed: u64,
    config: serde_json::Value,
    crate_versions: BTreeMap<String, String>,
    artifacts: BTreeMap<String, ArtifactEntry>,
}

impl RunManifest {
    pub fn new(seed: u64) -> Self {
        Self {
            manifest_version: MANIFEST_VERSION,
            seed,
            stages: BTreeMap::new(),
        }
    }

    pub fn stage(&self, name: &str) -> Option<&StageRecord> {
        self.stages.get(name)
    }

    /// A stage instance's echoed config, if that stage ran.
    pub fn stage_config(&self, name: &str) -> Option<&serde_json::Value> {
        self.stages.get(name).map(|s| &s.config)
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json + "\n")
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let bytes = std::fs::read(path)?;
        let value: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        if value.get("stages").is_some() {
            serde_json::from_value(value).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        } else {
            let v1: RunManifestV1 = serde_json::from_value(value)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            let mut stages = BTreeMap::new();
            stages.insert(
                v1.stage,
                StageRecord {
                    config: v1.config,
                    crate_versions: v1.crate_versions,
                    artifacts: v1.artifacts,
                },
            );
            Ok(Self {
                manifest_version: MANIFEST_VERSION,
                seed: v1.seed,
                stages,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_manifests_migrate_transparently() {
        let v1 = serde_json::json!({
            "stage": "terrain",
            "seed": 42,
            "config": { "sea_level_m": 0.5 },
            "crate_versions": { "via-terrain": "0.1.0" },
            "artifacts": {
                "heights_cm": { "file": "heights_cm.vrast", "blake3": "aa" }
            }
        });
        let dir = std::env::temp_dir().join(format!("via-manifest-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("manifest.json");
        std::fs::write(&path, serde_json::to_string_pretty(&v1).unwrap()).unwrap();

        let m = RunManifest::load(&path).unwrap();
        assert_eq!(m.manifest_version, MANIFEST_VERSION);
        assert_eq!(m.seed, 42);
        assert_eq!(
            m.stage_config("terrain").unwrap()["sea_level_m"]
                .as_f64()
                .unwrap(),
            0.5
        );
        assert_eq!(
            m.stage("terrain").unwrap().artifacts["heights_cm"].file,
            "heights_cm.vrast"
        );

        // Saving writes v2; the reload round-trips.
        m.save(&path).unwrap();
        let back = RunManifest::load(&path).unwrap();
        assert_eq!(back.manifest_version, MANIFEST_VERSION);
        assert_eq!(back.stage("terrain").unwrap().artifacts.len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }
}
