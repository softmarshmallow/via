//! via-bench — the benchmark's measurement half (ADR 0009 Decision 4;
//! ADR 0010).
//!
//! One code path measures both sides of every comparison it mediates:
//! reference towns imported from OSM extracts, and generated fabric. That
//! is the same-code rule the townfabric spike's validation record
//! established — when real and synthetic fabric are measured by different
//! implementations, the comparison measures the implementations, not the
//! towns.
//!
//! The measurement kernel is ported from `spikes/townfabric` (commit
//! 07a95b8) per ADR 0009's Rejected section: ported with tests, against
//! the protocol and defect list VALIDATION.md records and research 0012 as
//! the specification it must satisfy — never against VALIDATION.md's
//! numbers, which ADR 0010 rules historical. The independent check on this
//! crate is the analysis sidecar's cross-validation against
//! `osmnx`/`momepy` on identical extracts.
//!
//! Not a chain stage: like `via-viz`, this crate is never a dependency of
//! a stage crate.

pub mod corpus;
pub mod ensemble;
pub mod font;
pub mod geom;
pub mod graph;
pub mod measure;
pub mod null;
pub mod osm;
pub mod render;

use serde::{Deserialize, Serialize};

use geom::P2;

/// The measurement protocol, restated with every number (ADR 0008 D4).
///
/// `id` is the protocol identifier ADR 0008 D11 requires. Until the freeze
/// (ADR 0010 Decision 3) the pilot runs under `pilot-0`; the freeze binds
/// an identified, immutable `reference/protocol-<id>.md` and this struct
/// echoes its values.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Protocol {
    pub id: String,
    /// Statistics describe the disc of this radius about the study centre
    /// (0012 P1).
    pub radius_m: f64,
    /// Fabric participates in the graph out to `radius_m * buffer`, so
    /// clipped streets are not false dead ends (0012 P2).
    pub buffer: f64,
    /// Node-snap radius for the planar insertion, identical on both sides
    /// of any comparison. The spike hardcoded 6.0 for reference towns and
    /// varied it per synthetic town — a protocol asymmetry this parameter
    /// removes.
    pub snap_m: f64,
    /// Faces of the street graph count as blocks inside this area range,
    /// identical on both sides. The spike used 300..=90_000 for reference
    /// and per-town caps up to 200_000 for synthetic fabric — the same
    /// asymmetry, removed the same way.
    pub block_area_min_m2: f64,
    pub block_area_max_m2: f64,
    /// Building footprints below this area are not fabric.
    pub building_min_area_m2: f64,
    /// Building characters are computed only when the disc holds at least
    /// this many footprints — a declared convention (ADR 0008 D8): below
    /// it, medians are noise (the spike's 7-footprint abilene-res disc is
    /// the cautionary case). The count itself is always reported.
    pub min_footprints: usize,
}

impl Default for Protocol {
    fn default() -> Self {
        Self {
            id: "pilot-0".to_string(),
            radius_m: 300.0,
            buffer: 1.45,
            snap_m: 6.0,
            block_area_min_m2: 300.0,
            block_area_max_m2: 90_000.0,
            building_min_area_m2: 8.0,
            min_footprints: 30,
        }
    }
}

/// A building footprint. Plain data; `storeys` is NaN when the source
/// does not tag it (OSM rarely tags `building:levels`), and the battery
/// reports storey statistics over tagged buildings only.
pub struct Building {
    pub poly: Vec<P2>,
    pub storeys: f64,
    pub floor_area_m2: f64,
}

/// A cadastral parcel. Only synthetic fabric has these (OSM has no
/// cadastre), so parcel statistics are optional throughout.
pub struct Parcel {
    pub poly: Vec<P2>,
    pub area_m2: f64,
    pub frontage_m: f64,
    pub centroid: P2,
}

/// The git revision this binary was built from (ADR 0008 D11).
pub fn code_revision() -> &'static str {
    env!("VIA_BENCH_GIT_REV")
}
