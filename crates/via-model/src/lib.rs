//! The world model.
//!
//! Responsibility: what the graph's nodes compute about a world: kinds and authority and the
//! order of entries, the core rules (`set`, `run`, `scatter`) and instances, surfaces, assets
//! and packs, names by place, hand edits and the change report. It holds mechanisms only; every
//! domain noun lives in a library as data (principle 5).
//!
//! Governed by `spec/authority.md`, `spec/fields.md`, `spec/rules.md`, `spec/surfaces.md`,
//! `spec/assets.md` and `spec/identity.md`.
//!
//! # Use
//!
//! [`build`] takes a merged world (`via_doc::load`) and [`Options`] (where packs are found) and an
//! optional baseline [`Build`], and gives a [`Build`]: every field's samples, every feature,
//! footprint and instance, the meetings, the diagnostics raised (emit them against the merged
//! world, which locates them) and the change report. The same text gives the same build.
//!
//! # Map
//!
//! | Module | Chapter |
//! |---|---|
//! | [`pack`] | `assets.md` 1, 2, 5, 6: finding and reading a pack, primitive facts |
//! | [`asset`], `asset_gltf`, `asset_json` | `assets.md` 2.2, 2.3, 3: model assets, their bound files, measuring glTF |
//! | [`asset_facts`] | `assets.md` 4, 8.1: the facts file, `via pack accept`, revisions |
//! | `decl` | the merged world read: fields, kinds, features, instances, scatters, hand edits; formulas checked |
//! | `place` | `geometry.md` 2, `rules.md` 2 and 3: shapes, plans, place names, footprint features |
//! | `resolve` | `fields.md` 5 and 6, `authority.md` 2 to 5: entries in R21 order, views, conflicts |
//! | `rules` | `rules.md` 6 to 8, `identity.md` 4: scatter, hand edits, grounding, checks |
//! | [`report`] | `identity.md` 5, `assets.md` 7: the change report |
//! | [`query`] | `surfaces.md` 2: the four surface questions over a build, on the terrain |
//!
//! # Not yet
//!
//! [`ModelPlan`] is the build as `via-graph` nodes (`graph.md` 1): every statement's node, fields
//! cut by the build's chunks; node values are taken from the model's whole-extent passes, made
//! once per process and only when a node must run. [`build_graph`] runs it with the model's
//! nodes alone. A selection by place of a scatter's instance (`on = { of = "<scatter>", near }`) is
//! taken by pins and clears only, which read the scatter's run stage (`identity.md` 4.1); a
//! feature or a hand instance placed on one waits for the instance nodes and is `doc.not_yet`.
//! `starts_at` and `ends_at` with `follows` are `doc.not_yet` until the M3 waterfront case
//! decides them (`geometry.md` 4).

pub mod asset;
pub mod asset_facts;
mod asset_gltf;
mod asset_json;
mod build;
mod decl;
pub mod derived;
pub mod derived_checks;
mod graph;
pub mod pack;
mod place;
pub mod query;
pub mod report;
mod resolve;
mod rules;
mod text;

pub use build::{build, Build, FeatureOut, FieldOut, FootprintOut, InstanceOut, Meeting, Options};
pub use decl::Tier;
pub use graph::{build_graph, dedupe, leaves, memo_ms, FileReads, ModelPlan};
pub use rules::scatter_name;

/// `documents.md` 5.1 at merge: the names formulas read that lie above the reader
/// (`doc.upward_ref`). `via merge` reports them beside the document checks, so a merge fails
/// where the build would.
pub fn formula_direction(world: &via_doc::World) -> Vec<via_diag::Diag> {
    decl::Decls::read(world)
        .diags
        .into_iter()
        .filter(|x| *x.code() == via_diag::DiagCode::Core(via_diag::codes::doc::UPWARD_REF))
        .collect()
}

#[cfg(test)]
mod tests;
