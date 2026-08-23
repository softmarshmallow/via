//! Reference fabric from OpenStreetMap, imported into the *same* structures
//! the synthetic town uses, so one measurement battery reads both.
//!
//! Data: an Overpass JSON extract (see the spike README for the query).
//! Attribution: © OpenStreetMap contributors, ODbL. Extracts live in the
//! gitignored `runs/` tree and are never redistributed from this repo.
//!
//! Declared choices, because they move the numbers:
//!
//! - Street set: the carriageway and pedestrian-street network —
//!   motorway…residential, unclassified, service, living_street,
//!   pedestrian — minus driveways and parking aisles. Footpaths, tracks,
//!   steps and cycleways are excluded; in a medieval core the alleys are
//!   genuinely fabric, so this understates meshedness in exactly the case
//!   where it matters most.
//! - Projection: local equirectangular about the query centre. Over a
//!   1 km town the distortion is far below the DEM's own resolution.
//! - The graph is built with the same snap-and-split insertion the
//!   synthetic fabric uses, so junction counts are comparable rather than
//!   an artifact of OSM's node placement.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::geom::*;
use crate::graph::{Class, Graph};
use crate::parcels::Building;

const STREET_KINDS: &[&str] = &[
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

/// Parse an Overpass extract centred on (lat0, lon0), keeping everything
/// within `radius_m`.
pub fn load(
    path: &std::path::Path,
    lat0: f64,
    lon0: f64,
    radius_m: f64,
    buffer: f64,
    snap_m: f64,
    include_paths: bool,
) -> Result<Reference> {
    let import_m = radius_m * buffer;
    let v: Value = serde_json::from_slice(&std::fs::read(path)?)
        .with_context(|| format!("parsing {}", path.display()))?;
    let elements = v["elements"].as_array().context("no elements")?;

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
        // Keep anything with a vertex inside the study radius.
        if !pts.iter().any(|p| len(*p) <= import_m) {
            continue;
        }

        if let Some(kind) = tags["highway"].as_str() {
            let is_path = matches!(kind, "footway" | "path" | "steps" | "alley" | "cycleway");
            if !(STREET_KINDS.contains(&kind) || (include_paths && is_path)) {
                continue;
            }
            // Driveways and parking aisles are not town fabric; keeping
            // them would inflate the node count and the dead-end share.
            if matches!(
                tags["service"].as_str(),
                Some("driveway") | Some("parking_aisle") | Some("drive-through")
            ) {
                continue;
            }
            let class = class_of(kind);
            for pair in pts.windows(2) {
                // Clip to the study area so the boundary is the same for
                // real and synthetic fabric.
                if len(pair[0]) > import_m && len(pair[1]) > import_m {
                    continue;
                }
                if dist(pair[0], pair[1]) > 0.5 {
                    graph.insert_segment(pair[0], pair[1], class, snap_m);
                }
            }
        } else if tags["building"].is_string() {
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
            if a < 8.0 {
                continue;
            }
            buildings.push(Building {
                poly,
                // OSM levels where tagged; otherwise unknown, and the
                // battery's storey figure is not comparable.
                storeys: tags["building:levels"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(f64::NAN),
                floor_area_m2: a,
                parcel: usize::MAX,
            });
        }
    }

    // Blocks are the faces of the real street graph, filtered the same way
    // the synthetic ones are.
    let blocks: Vec<Vec<P2>> = graph
        .faces()
        .into_iter()
        .filter(|f| {
            let a = polygon_area(f).abs();
            let c = polygon_centroid(f);
            (300.0..=90_000.0).contains(&a) && len(c) <= import_m
        })
        .collect();

    Ok(Reference {
        graph,
        buildings,
        blocks,
    })
}
