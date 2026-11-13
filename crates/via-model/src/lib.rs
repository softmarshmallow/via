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
//! | `decl` | the merged world read: fields, kinds, features, instances, scatters, hand edits; formulas checked |
//! | `place` | `geometry.md` 2, `rules.md` 2 and 3: shapes, plans, place names, footprint features |
//! | `resolve` | `fields.md` 5 and 6, `authority.md` 2 to 5: entries in R21 order, views, conflicts |
//! | `rules` | `rules.md` 6 to 8, `identity.md` 4: scatter, hand edits, grounding, checks |
//! | [`report`] | `identity.md` 5, `assets.md` 7: the change report |
//!
//! # Not yet
//!
//! [`build_graph`] runs the same build through `via-graph`: each field is one `field` node over the
//! whole grid, keyed by its statements and the fields it reads; the other node families
//! (`shape`, `instance`, `rule`, `held`, `footprint`, `height`) and chunked windows are not nodes
//! yet. `where` regions, `follows`, `smooth`, a field read from a `file`,
//! selections by place (`{ of, near }`) and clears `on` a path are refused with `doc.not_yet`.

mod build;
mod decl;
mod graph;
pub mod pack;
mod place;
pub mod report;
mod resolve;
mod rules;
mod text;

pub use build::{build, Build, FeatureOut, FieldOut, FootprintOut, InstanceOut, Meeting, Options};
pub use decl::Tier;
pub use graph::{build_graph, ModelPlan};
pub use rules::scatter_name;

#[cfg(test)]
mod tests;
