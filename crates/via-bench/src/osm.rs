//! Reference fabric from OpenStreetMap, imported into the *same* structures
//! the synthetic fabric uses, so one measurement battery reads both
//! (VALIDATION.md's same-code rule; ADR 0009 Decision 4).
//!
//! Data: an Overpass JSON extract fetched by `analysis/fetch_extract.py`
//! with full ADR 0010 Decision 4 provenance. Attribution: © OpenStreetMap
//! contributors, ODbL. Extracts live outside version control and are never
//! redistributed from this repository.
//!
//! Street sets follow the adopted specification, research 0012 P4,
//! verbatim — two sets, measured and reported separately:
//!
//! - **Carriageway**: highway ∈ {motorway, trunk, primary, secondary,
//!   tertiary, unclassified, residential, living_street, service,
//!   pedestrian} plus `_link` variants, excluding
//!   `service=driveway|parking_aisle`.
//! - **All-ways**: the above plus {footway, path, steps, cycleway}.
//!
//! Two spike drifts are resolved by following the spec: the spike also
//! excluded `service=drive-through` (not in P4's exclusion list), and its
//! `--include-paths` matched a `highway=alley` value that does not exist
//! in OSM tagging, while real alleys (`highway=service` +
//! `service=alley`) were silently in the default set — under P4 they are
//! in the carriageway set, deliberately. The protocol freeze (ADR 0010
//! Decision 3) may revisit both, on instrument grounds only.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::geom::*;
use crate::graph::{Class, Graph};
use crate::{Building, Protocol};

const CARRIAGEWAY_KINDS: &[&str] = &[
    "motorway",
    "motorway_link",
    "trunk",
    "trunk_link",
    "primary",
    "primary_link",
    "secondary",
    "secondary_link",
    "tertiary",
    "tertiary_link",
    "unclassified",
    "residential",
    "living_street",
    "service",
    "pedestrian",
];

const PATH_KINDS: &[&str] = &["footway", "path", "steps", "cycleway"];

/// Which 0012 P4 street set an import carries.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize)]
pub enum StreetSet {
    Carriageway,
    AllWays,
}

impl StreetSet {
    pub fn label(self) -> &'static str {
        match self {
            StreetSet::Carriageway => "carriageway",
            StreetSet::AllWays => "all_ways",
        }
    }

    fn keeps(self, kind: &str) -> bool {
        CARRIAGEWAY_KINDS.contains(&kind)
            || (self == StreetSet::AllWays && PATH_KINDS.contains(&kind))
    }
}

pub struct Reference {
    pub graph: Graph,
    pub buildings: Vec<Building>,
    pub blocks: Vec<Vec<P2>>,
}

fn class_of(kind: &str) -> Class {
    match kind {
        "motorway" | "trunk" | "primary" | "motorway_link" | "trunk_link" | "primary_link" => {
            Class::Corridor
        }
        "secondary" | "tertiary" | "unclassified" | "residential" | "secondary_link"
        | "tertiary_link" => Class::Street,
        _ => Class::Lane,
    }
}

/// Parse an Overpass extract centred on (lat0, lon0) under `protocol`,
/// keeping the `set` street set.
pub fn load(
    path: &std::path::Path,
    lat0: f64,
    lon0: f64,
    protocol: &Protocol,
    set: StreetSet,
) -> Result<Reference> {
    let import_m = protocol.radius_m * protocol.buffer;
    let v: Value = serde_json::from_slice(&std::fs::read(path)?)
        .with_context(|| format!("parsing {}", path.display()))?;
    let elements = v["elements"].as_array().context("no elements")?;

    // Local equirectangular about the study centre (0012 P5).
    let mx = 111_320.0 * lat0.to_radians().cos();
    let my = 110_540.0;
    let project = |lat: f64, lon: f64| -> P2 { [(lon - lon0) * mx, (lat - lat0) * my] };

    let mut nodes: BTreeMap<i64, P2> = BTreeMap::new();
    for el in elements {
        if el["type"] == "node" {
            if let (Some(id), Some(lat), Some(lon)) =
                (el["id"].as_i64(), el["lat"].as_f64(), el["lon"].as_f64())
            {
                nodes.insert(id, project(lat, lon));
            }
        }
    }

    let mut graph = Graph::new(40.0);
    let mut buildings = Vec::new();
    // Ways in id order: the import must not depend on Overpass's ordering.
    let mut ways: Vec<&Value> = elements.iter().filter(|e| e["type"] == "way").collect();
    ways.sort_by_key(|w| w["id"].as_i64().unwrap_or(0));

    for w in ways {
        let tags = &w["tags"];
        let refs: Vec<i64> = w["nodes"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x.as_i64()).collect())
            .unwrap_or_default();
        if refs.len() < 2 {
            continue;
        }
        let pts: Vec<P2> = refs.iter().filter_map(|r| nodes.get(r).copied()).collect();
        if pts.len() < 2 {
            continue;
        }
        // Keep anything with a vertex inside the participation buffer.
        if !pts.iter().any(|p| len(*p) <= import_m) {
            continue;
        }

        if let Some(kind) = tags["highway"].as_str() {
            if !set.keeps(kind) {
                continue;
            }
            // 0012 P4's exclusion list, verbatim.
            if matches!(
                tags["service"].as_str(),
                Some("driveway") | Some("parking_aisle")
            ) {
                continue;
            }
            let class = class_of(kind);
            for pair in pts.windows(2) {
                // Clip where both endpoints are outside the buffer, so the
                // boundary is the same for real and synthetic fabric.
                if len(pair[0]) > import_m && len(pair[1]) > import_m {
                    continue;
                }
                if dist(pair[0], pair[1]) > 0.5 {
                    graph.insert_segment(pair[0], pair[1], class, protocol.snap_m);
                }
            }
        } else if tags["building"].is_string() && tags["building"] != "no" {
            // A building is a *closed* way (protocol: Elements); an
            // unclosed ring or an explicit building=no is not a footprint.
            // The sidecar applies the same two rules.
            if refs.first() != refs.last() {
                continue;
            }
            let mut poly = pts.clone();
            if dist(poly[0], *poly.last().unwrap()) < 0.5 {
                poly.pop();
            }
            if poly.len() < 3 {
                continue;
            }
            let c = polygon_centroid(&poly);
            if len(c) > import_m {
                continue;
            }
            let a = polygon_area(&poly).abs();
            if a < protocol.building_min_area_m2 {
                continue;
            }
            buildings.push(Building {
                poly,
                // OSM levels where tagged; otherwise NaN, and the battery
                // reports storeys over tagged buildings only.
                storeys: tags["building:levels"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(f64::NAN),
                floor_area_m2: a,
            });
        }
    }

    // Blocks are the faces of the street graph, filtered by the protocol's
    // area caps — identical on both sides of any comparison.
    let blocks: Vec<Vec<P2>> = graph
        .faces()
        .into_iter()
        .filter(|f| {
            let a = polygon_area(f).abs();
            let c = polygon_centroid(f);
            (protocol.block_area_min_m2..=protocol.block_area_max_m2).contains(&a)
                && len(c) <= import_m
        })
        .collect();

    Ok(Reference {
        graph,
        buildings,
        blocks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hand-written miniature Overpass extract: two crossing residential
    /// streets, one footway, one driveway, one service alley, one tagged
    /// and one untagged building, and one building outside the buffer.
    fn fixture() -> serde_json::Value {
        // Centre at (0,0); 1e-3 deg lat ≈ 110.5 m, 1e-3 deg lon ≈ 111.3 m
        // at the equator.
        serde_json::json!({
            "version": 0.6,
            "osm3s": {"timestamp_osm_base": "2026-08-20T00:00:00Z", "generator": "test"},
            "elements": [
                {"type": "node", "id": 1, "lat": -0.003, "lon": 0.0},
                {"type": "node", "id": 2, "lat": 0.003, "lon": 0.0},
                {"type": "node", "id": 3, "lat": 0.0, "lon": -0.003},
                {"type": "node", "id": 4, "lat": 0.0, "lon": 0.003},
                {"type": "node", "id": 5, "lat": 0.001, "lon": 0.001},
                {"type": "node", "id": 6, "lat": 0.002, "lon": 0.001},
                {"type": "node", "id": 7, "lat": 0.001, "lon": -0.001},
                {"type": "node", "id": 8, "lat": 0.002, "lon": -0.001},
                {"type": "node", "id": 9, "lat": -0.001, "lon": 0.001},
                {"type": "node", "id": 10, "lat": -0.002, "lon": 0.001},
                // Building A (tagged levels), ~11x11 m at (0.0005, 0.0005).
                {"type": "node", "id": 20, "lat": 0.00045, "lon": 0.00045},
                {"type": "node", "id": 21, "lat": 0.00055, "lon": 0.00045},
                {"type": "node", "id": 22, "lat": 0.00055, "lon": 0.00055},
                {"type": "node", "id": 23, "lat": 0.00045, "lon": 0.00055},
                // Building B (no levels tag).
                {"type": "node", "id": 24, "lat": -0.00055, "lon": 0.00045},
                {"type": "node", "id": 25, "lat": -0.00045, "lon": 0.00045},
                {"type": "node", "id": 26, "lat": -0.00045, "lon": 0.00055},
                {"type": "node", "id": 27, "lat": -0.00055, "lon": 0.00055},
                // Building C: outside the 435 m buffer (lat 0.005 ≈ 553 m).
                {"type": "node", "id": 28, "lat": 0.005, "lon": 0.0},
                {"type": "node", "id": 29, "lat": 0.0051, "lon": 0.0},
                {"type": "node", "id": 30, "lat": 0.0051, "lon": 0.0001},
                {"type": "node", "id": 31, "lat": 0.005, "lon": 0.0001},
                {"type": "way", "id": 100, "nodes": [1, 2],
                 "tags": {"highway": "residential"}},
                {"type": "way", "id": 101, "nodes": [3, 4],
                 "tags": {"highway": "residential"}},
                {"type": "way", "id": 102, "nodes": [5, 6],
                 "tags": {"highway": "footway"}},
                {"type": "way", "id": 103, "nodes": [7, 8],
                 "tags": {"highway": "service", "service": "driveway"}},
                {"type": "way", "id": 104, "nodes": [9, 10],
                 "tags": {"highway": "service", "service": "alley"}},
                {"type": "way", "id": 200, "nodes": [20, 21, 22, 23, 20],
                 "tags": {"building": "yes", "building:levels": "2"}},
                {"type": "way", "id": 201, "nodes": [24, 25, 26, 27, 24],
                 "tags": {"building": "house"}},
                {"type": "way", "id": 202, "nodes": [28, 29, 30, 31, 28],
                 "tags": {"building": "yes"}},
            ]
        })
    }

    fn load_fixture(set: StreetSet) -> Reference {
        let dir = std::env::temp_dir().join("via-bench-osm-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("fixture-{}.json", set.label()));
        std::fs::write(&path, serde_json::to_vec(&fixture()).unwrap()).unwrap();
        load(&path, 0.0, 0.0, &Protocol::default(), set).unwrap()
    }

    #[test]
    fn street_sets_follow_0012_p4() {
        let carriageway = load_fixture(StreetSet::Carriageway);
        // Two crossing residentials plus the service alley; the driveway
        // and the footway are out. The crossing splits each street in two:
        // 4 street edges + 1 alley edge.
        assert_eq!(carriageway.graph.edges.len(), 5);
        let all = load_fixture(StreetSet::AllWays);
        // The footway joins.
        assert_eq!(all.graph.edges.len(), 6);
    }

    #[test]
    fn buildings_filtered_and_tagged() {
        let r = load_fixture(StreetSet::Carriageway);
        // Building C is outside the buffer; A and B survive.
        assert_eq!(r.buildings.len(), 2);
        let tagged: Vec<bool> = r.buildings.iter().map(|b| b.storeys.is_finite()).collect();
        assert_eq!(tagged.iter().filter(|&&t| t).count(), 1);
    }
}
