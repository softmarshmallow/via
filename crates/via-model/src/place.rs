//! Shapes and placements (`geometry.md` 2, `rules.md` 2 and 3, `identity.md` 1): every feature's
//! resolved geometry, every hand instance's and pin's plan, and place names (`beck.end` style:
//! `<path>.start`, `<path>.end`, `<instance>`, `<instance>.<mount>`, `<instance>.ground`).
//!
//! Resolution is lazy and memoised: a feature placed `on` an instance waits for that instance's
//! plan, which may wait for a feature. A name met again while it is being resolved is a cycle,
//! `graph.cycle`, naming every member.

use std::collections::{BTreeMap, BTreeSet};

use via_diag::{codes, Diag, KeyPath, Site, Subject, Value as DValue};
use via_geom::follow::{follow, Reference};
use via_geom::lineref::{self, Line};
use via_geom::mask::{mask, MaskGrid};
use via_geom::network::{representative, PostGrid};
use via_geom::offset::{offset, Join};
use via_geom::pred::{dist2_rings, Dist2, Side, Wide};
use via_geom::shape::{read_path, read_ring};
use via_geom::{Region, P};
use via_num::Frame;

use crate::decl::{Decls, Face, FeatureDecl, Follows, On, Place, Select, ShapeDecl};
use crate::pack::{Asset, Pack};
use crate::resolve::Fields;
use crate::text::{self, at_key, at_path};

/// The side of the coarse lattice square that names a `where` piece (`identity.md` 2.3), mm.
const PIECE_SQUARE_MM: i64 = 32_000;

/// One piece of a `where` region, named by place (`identity.md` 2.3, class `piece`).
#[derive(Clone, Debug)]
pub struct PieceOut {
    /// `<feature>#p<i>_<j>_<ordinal>`.
    pub name: String,
    pub region: Region,
    /// The representative point (`geometry.md` 3.2) that names and orders it.
    pub point: P,
}

/// Where a clear acts (`identity.md` 4.3), resolved.
#[derive(Clone, Debug)]
pub enum ClearSpot {
    /// Origins within the radius of a point.
    Disc(P, i64),
    /// Origins inside a region, outline included.
    Inside(Region),
    /// Origins within the radius of the nearest point of a whole path: a strip.
    Strip(Line, i64),
}

/// A resolved shape.
#[derive(Clone, Debug)]
pub enum Geo {
    /// A point, with its facing when it has one.
    Point {
        p: P,
        facing: Option<i64>,
    },
    Path {
        line: Line,
    },
    Region {
        region: Region,
        line: Option<Line>,
    },
}

impl Geo {
    /// The shape's nearest point to `near` on the lattice, and its squared distance in square
    /// millimetres: 0 inside a region or on the shape. Selections compare these exactly
    /// (`identity.md` 3, "compared exactly in integer millimetres"); a point off the lattice is
    /// never nearest.
    pub fn nearest_mm(&self, near: P) -> (i128, P) {
        if self.contains(near) {
            return (0, near);
        }
        let m = self.project(near);
        match (
            via_num::round_to_lattice(m.nearest.0),
            via_num::round_to_lattice(m.nearest.1),
        ) {
            (Ok(x), Ok(y)) => (
                via_num::distance_sq_mm((x, y), (near.x, near.y)),
                P::new(x, y),
            ),
            _ => (i128::MAX, near),
        }
    }

    /// The plan bounding box.
    pub fn bbox(&self) -> (P, P) {
        match self {
            Geo::Point { p, .. } => (*p, *p),
            Geo::Path { line } => via_geom::shape::bbox(line.pts.iter().copied())
                .unwrap_or((P::new(0, 0), P::new(0, 0))),
            Geo::Region { region, .. } => region.bbox().unwrap_or((P::new(0, 0), P::new(0, 0))),
        }
    }

    /// `project` (`geometry.md` 3.8): `t`, `d`, `sd` in metres.
    pub fn project(&self, p: P) -> lineref::Measure {
        match self {
            Geo::Point { p: q, .. } => lineref::project_point(*q, p),
            Geo::Path { line } => lineref::project_path(line, p),
            Geo::Region { region, .. } => {
                lineref::project_region(region, p).unwrap_or(lineref::Measure {
                    t: None,
                    d: 0.0,
                    sd: 0.0,
                    nearest: (0.0, 0.0),
                })
            }
        }
    }

    /// Exact containment of a point: inside or on (regions), on (paths), equal (points).
    pub fn contains(&self, p: P) -> bool {
        match self {
            Geo::Point { p: q, .. } => *q == p,
            Geo::Path { line } => (0..line.segments()).any(|k| {
                let (a, b) = line.seg(k);
                via_geom::pred::on_segment(p, a, b)
            }),
            Geo::Region { region, .. } => region.side(p) != via_geom::pred::Side::Out,
        }
    }

    /// The shape kind word for messages.
    pub fn word(&self) -> &'static str {
        match self {
            Geo::Point { .. } => "a point",
            Geo::Path { .. } => "a path",
            Geo::Region { .. } => "a region",
        }
    }

    /// The point a name gives in `features.json` (`authority.md` 1.2): a point's point, a
    /// path's point at `t = 0.5`, a region's first vertex standing for its representative point.
    pub fn name_point(&self) -> P {
        match self {
            Geo::Point { p, .. } => *p,
            Geo::Path { line } => lineref::interpolate(line, "name", 0.5, 0)
                .map(|a| a.point)
                .unwrap_or(line.pts[0]),
            Geo::Region { region, .. } => region
                .pieces
                .first()
                .map(|p| p.shell[0])
                .unwrap_or(P::new(0, 0)),
        }
    }
}

/// A hand instance's or a pin's plan (`rules.md` 3.4): plan only, height comes later.
#[derive(Clone, Debug)]
pub struct Plan {
    pub name: String,
    pub asset: String,
    pub origin: P,
    pub facing: i64,
    pub scale: f64,
    pub tint: f64,
    /// The footprint in the world frame; empty for a point footprint.
    pub footprint: Region,
    /// What it stands `on`, by place name, if anything.
    pub on: Option<String>,
    #[allow(dead_code)] // layer rank: giving way reads down only (`rules.md` 3.4)
    pub rank: i64,
}

/// What a place name resolves to.
#[derive(Clone, Debug)]
pub enum Spot {
    /// A thing with a length: a path or a region.
    Long(Geo),
    /// A thing with no length: its point and facing (`None` where it has none).
    At {
        p: P,
        facing: Option<i64>,
        end: bool,
    },
}

/// Every resolved shape and plan.
#[derive(Clone, Debug, Default)]
pub struct Placed {
    /// Authored features by id; footprint features by `<instance>.ground`.
    pub shapes: BTreeMap<String, Geo>,
    pub plans: BTreeMap<String, Plan>,
    pub pins: BTreeMap<String, Plan>,
    /// Footprint features: place name to (instance, kind).
    pub footprints: BTreeMap<String, (String, String)>,
    /// Name attributes resolved: (feature, attribute) to (name, point).
    pub names: BTreeMap<(String, String), (String, P)>,
    /// Instances not placed, so dependants report `surface.host_missing`.
    pub missing: BTreeSet<String>,
    /// The pieces of each `where` region, by feature, in name order.
    pub pieces: BTreeMap<String, Vec<PieceOut>>,
    /// A scatter's `within` written as a selection, resolved to a piece's region.
    pub within: BTreeMap<String, Region>,
    /// Each clear's place, resolved; a clear that selects a scatter's instance is in `deferred`.
    pub clears: BTreeMap<String, ClearSpot>,
    /// What each selection by place resolved to: `<table>.<id>` to (produced name, its point).
    pub selections: BTreeMap<String, (String, P)>,
    /// Hand edits that select a scatter's instance (`identity.md` 4.1): `<table>.<id>` to the
    /// selection. They read the scatter's run stage, so they resolve after it runs.
    pub deferred: BTreeMap<String, Select>,
    /// What each `[network]` and `[cells]` table made (`geometry.md` 8), by its id.
    pub made: BTreeMap<String, crate::makers::Made>,
    /// Each network, kept for the cells tables that divide its areas.
    pub nets: BTreeMap<String, crate::makers::KeptNet>,
    /// The made things that write fields and that shape selectors count (`geometry.md` 8.1,
    /// Effects of a made region): every area or cell a table gives a declared kind, by its
    /// produced name, to (its maker table's id, the kind, its region).
    pub made_writers: BTreeMap<String, (String, String, Geo)>,
}

impl Placed {
    /// A placed thing's shape by place name: a feature's or footprint's, else a made writer's.
    pub fn shape(&self, name: &str) -> Option<&Geo> {
        self.shapes
            .get(name)
            .or_else(|| self.made_writers.get(name).map(|(_, _, g)| g))
    }
}

pub struct Placer<'a> {
    pub(crate) d: &'a Decls,
    pack: &'a Pack,
    pub out: Placed,
    visiting: Vec<String>,
    pub(crate) failed: BTreeSet<String>,
    pub diags: Vec<Diag>,
    cycles: BTreeSet<Vec<String>>,
    /// What stands, directly or not, on a produced instance that gave way (`authority.md` 2.2):
    /// not placed, with no record.
    gone: BTreeSet<String>,
    /// Each `identity.near_none` said so far, by maker and class: its index in `diags` and the
    /// selections it lists (`identity.md` 3: one record per maker and class).
    near: BTreeMap<(String, String), (usize, Vec<String>)>,
}

fn pt(p: (i64, i64)) -> P {
    P::new(p.0, p.1)
}

/// The declared id a place name reads (`identity.md` 1) when that declaration failed its own
/// check: the name itself, a path's `.start`/`.end`, or the host of `<instance>.<mount>`.
pub(crate) fn failed_owner(d: &Decls, name: &str) -> Option<(String, &'static str)> {
    let mut heads = vec![name];
    if let Some(s) = name
        .strip_suffix(".start")
        .or_else(|| name.strip_suffix(".end"))
    {
        heads.push(s);
    }
    if let Some((host, _)) = name.split_once('.') {
        heads.push(host);
    }
    heads.into_iter().find_map(|h| {
        let top = *d.declared.get(h)?;
        let live = match top {
            "feature" => d.features.contains_key(h),
            "instance" => d.instances.contains_key(h),
            _ => d.pins.contains_key(h),
        };
        (!live).then(|| (h.to_string(), top))
    })
}

/// Whether a place name reads a declaration that failed ([`failed_owner`]).
pub(crate) fn failed_declaration(d: &Decls, name: &str) -> bool {
    failed_owner(d, name).is_some()
}

/// The world-frame footprint of an asset at a frame.
pub fn footprint(asset: &Asset, origin: P, facing: i64, scale: f64) -> Option<Region> {
    if asset.footprint.is_empty() {
        return Some(Region::default());
    }
    let f = Frame::new((origin.x, origin.y), facing, scale).ok()?;
    let mut ring = Vec::with_capacity(asset.footprint.len());
    for p in &asset.footprint {
        let w = f.to_world(*p).ok()?;
        ring.push(P::new(w.0, w.1));
    }
    Some(Region::from_ring(ring))
}

impl<'a> Placer<'a> {
    pub(crate) fn new(d: &'a Decls, pack: &'a Pack) -> Placer<'a> {
        Placer {
            d,
            pack,
            out: Placed::default(),
            visiting: Vec::new(),
            failed: BTreeSet::new(),
            gone: BTreeSet::new(),
            diags: Vec::new(),
            cycles: BTreeSet::new(),
            near: BTreeMap::new(),
        }
    }

    /// Resolves every feature, instance, pin and footprint, in id order.
    pub fn run(mut self) -> (Placed, Vec<Diag>) {
        for id in self.d.features.keys() {
            self.feature(id);
        }
        self.make_all();
        self.out.made_writers = crate::makers::made_writers(self.d, &self.out.made);
        for id in self.d.instances.keys() {
            self.instance(id);
        }
        for id in self.d.pins.keys() {
            self.pin(id);
        }
        for (id, s) in &self.d.scatters {
            if let Some(sel) = &s.within_of {
                if let Some(Spot::Long(Geo::Region { region, .. })) =
                    self.select("scatter", id, sel)
                {
                    self.out.within.insert(id.clone(), region);
                }
            }
        }
        for id in self.d.clears.keys() {
            self.clear(id);
        }
        self.name_attributes();
        (self.out, self.diags)
    }

    pub(crate) fn enter(&mut self, name: &str, top: &str) -> bool {
        if let Some(at) = self.visiting.iter().position(|v| v == name) {
            let mut members: Vec<String> = self.visiting[at..].to_vec();
            members.sort();
            if self.cycles.insert(members.clone()) {
                self.diags.push(text::diag(
                    codes::graph::CYCLE,
                    Subject::entity(top, name),
                    Site::Header,
                    &format!("a cycle of placements: {}", members.join(" -> ")),
                    "break the loop: place one of them by `at`, or on something outside the loop",
                ));
            }
            for m in &members {
                self.failed.insert(m.clone());
            }
            return false;
        }
        self.visiting.push(name.to_string());
        true
    }

    pub(crate) fn leave(&mut self) {
        self.visiting.pop();
    }

    /// A feature's shape.
    pub fn feature(&mut self, id: &str) -> Option<Geo> {
        if let Some(g) = self.out.shapes.get(id) {
            return Some(g.clone());
        }
        if self.failed.contains(id) {
            return None;
        }
        let f = self.d.features.get(id)?;
        if !self.enter(id, "feature") {
            return None;
        }
        let g = self.make_feature(f);
        self.leave();
        match &g {
            Some(g) => {
                self.out.shapes.insert(id.to_string(), g.clone());
            }
            None => {
                self.failed.insert(id.to_string());
            }
        }
        g
    }

    fn geom_fault(&mut self, top: &str, id: &str, key: &str, e: via_geom::GeomDiag) {
        self.diags
            .push(e.to_diag(Subject::entity(top, id), Site::Key(KeyPath::key(key))));
    }

    fn make_feature(&mut self, f: &FeatureDecl) -> Option<Geo> {
        match &f.shape {
            ShapeDecl::Path(pts) => {
                let mut v: Vec<P> = pts.iter().map(|p| pt(*p)).collect();
                if let Some(s) = &f.starts_at {
                    let p = self.end_point(&f.id, "starts_at", s)?;
                    v.insert(0, p);
                }
                if let Some(e) = &f.ends_at {
                    let p = self.end_point(&f.id, "ends_at", e)?;
                    v.push(p);
                }
                if let Some(cost) = f.route {
                    // `route` (`derived.md` 3): the least-cost course through the written points,
                    // over the cost in the head view; smoothed when `smooth` is written.
                    let grid = self.d.grid?;
                    // The features its cost reads, through fields too, are placed first.
                    if !self.place_reads(&f.id, cost) {
                        return None;
                    }
                    let snapshot = self.out.clone();
                    let fields = Fields::new(self.d, &snapshot, grid);
                    let values = fields.eval_where(cost, &f.id);
                    self.diags.extend(fields.diags.borrow().iter().cloned());
                    let values = values?;
                    return match crate::derived::route_path(&f.id, &grid, &values, &v, f.smooth) {
                        Ok(points) => {
                            let mut gd = Vec::new();
                            let path = read_path(&f.id, &points, &mut gd);
                            for g in gd {
                                self.geom_fault("feature", &f.id, "route", g);
                            }
                            path.map(|p| Geo::Path {
                                line: Line::new(p, false),
                            })
                        }
                        Err(crate::derived::RouteFault::Raster(e)) => {
                            self.diags.push(e.diag(
                                Subject::entity("feature", f.id.clone()),
                                Site::Key(KeyPath::key("route")),
                            ));
                            None
                        }
                        Err(crate::derived::RouteFault::Geom(e)) => {
                            self.geom_fault("feature", &f.id, "smooth", e);
                            None
                        }
                    };
                }
                let mut gd = Vec::new();
                let path = read_path(&f.id, &v, &mut gd);
                for g in gd {
                    self.geom_fault("feature", &f.id, "path", g);
                }
                let mut path = path?;
                if f.smooth {
                    match via_geom::curve::smooth(&f.id, &path) {
                        Ok(p) => path = p,
                        Err(e) => {
                            self.geom_fault("feature", &f.id, "smooth", e);
                            return None;
                        }
                    }
                }
                Some(Geo::Path {
                    line: Line::new(path, false),
                })
            }
            ShapeDecl::Follows(fl) => self.follows_path(f, fl),
            ShapeDecl::Where(w) => self.where_region(f, *w),
            ShapeDecl::Region(pts) => {
                let v: Vec<P> = pts.iter().map(|p| pt(*p)).collect();
                let mut gd = Vec::new();
                let ring = read_ring(&f.id, &v, &mut gd);
                for g in gd {
                    self.geom_fault("feature", &f.id, "region", g);
                }
                let ring = ring?;
                let line = Line::new(ring.clone(), true);
                let region = Region::from_ring(ring);
                Some(Geo::Region {
                    region,
                    line: Some(line),
                })
            }
            ShapeDecl::Point(place) => {
                let (p, facing) = self.locate("feature", &f.id, place, f.face, true)?;
                let own = match f.face {
                    Some(Face::Bearing(b)) => Some(b),
                    _ => match place {
                        Place::On(_) | Place::Of(_) => Some(facing),
                        Place::At(_) => None,
                    },
                };
                Some(Geo::Point { p, facing: own })
            }
        }
    }

    /// The point a `starts_at` or `ends_at` gives (`geometry.md` 2.7).
    fn end_point(&mut self, id: &str, key: &str, name: &str) -> Option<P> {
        match self.spot("feature", id, key, name)? {
            Spot::At { p, .. } => Some(p),
            Spot::Long(g) => {
                self.diags.push(at_key(
                    codes::geom::MEET_SHAPE,
                    "feature",
                    id,
                    key,
                    &format!("`{name}` is {}, which has no one point to meet", g.word()),
                    "name an end, or place a point feature on it with `on`",
                ));
                None
            }
        }
    }

    /// Resolves a place name (`identity.md` 1).
    pub fn spot(&mut self, top: &str, id: &str, key: &str, name: &str) -> Option<Spot> {
        if let Some(stripped) = name
            .strip_suffix(".start")
            .or_else(|| name.strip_suffix(".end"))
        {
            if self.d.features.contains_key(stripped) {
                let end = name.ends_with(".end");
                return match self.feature(stripped)? {
                    Geo::Path { line } => {
                        let n = line.pts.len();
                        let (p, q) = if end {
                            (line.pts[n - 1], line.pts[n - 2])
                        } else {
                            (line.pts[0], line.pts[1])
                        };
                        let (dx, dy) = if end {
                            (p.x - q.x, p.y - q.y)
                        } else {
                            (q.x - p.x, q.y - p.y)
                        };
                        Some(Spot::At {
                            p,
                            facing: via_num::bearing(dx, dy),
                            end: true,
                        })
                    }
                    g => {
                        self.diags.push(at_key(
                            codes::identity::UNKNOWN_PLACE,
                            top,
                            id,
                            key,
                            &format!("`{stripped}` is {}, which has no ends", g.word()),
                            "name the end of a path",
                        ));
                        None
                    }
                };
            }
        }
        if self.d.features.contains_key(name) {
            return Some(match self.feature(name)? {
                Geo::Point { p, facing } => Spot::At {
                    p,
                    facing: Some(facing.unwrap_or(0)),
                    end: false,
                },
                g => Spot::Long(g),
            });
        }
        if self.d.gave_way.contains(name) {
            self.host_missing(top, id, key, name);
            return None;
        }
        if self.d.instances.contains_key(name) || self.d.pins.contains_key(name) {
            let plan = if self.d.instances.contains_key(name) {
                self.instance(name)
            } else {
                self.pin(name)
            };
            return match plan {
                Some(pl) => Some(Spot::At {
                    p: pl.origin,
                    facing: Some(pl.facing),
                    end: false,
                }),
                None => {
                    self.host_missing(top, id, key, name);
                    None
                }
            };
        }
        if let Some((host, part)) = name.split_once('.') {
            if self.d.gave_way.contains(host) {
                self.host_missing(top, id, key, host);
                return None;
            }
            if self.d.instances.contains_key(host) {
                let Some(pl) = self.instance(host) else {
                    self.host_missing(top, id, key, host);
                    return None;
                };
                if part == "ground" {
                    let fp = format!("{host}.ground");
                    return match self.out.shapes.get(&fp) {
                        Some(g) => Some(Spot::Long(g.clone())),
                        None => {
                            self.diags.push(at_key(
                                codes::identity::UNKNOWN_PLACE,
                                top,
                                id,
                                key,
                                &format!("`{host}` has no `ground`"),
                                "give the instance `ground = \"<kind>\"`",
                            ));
                            None
                        }
                    };
                }
                let asset = self.pack.assets.get(&pl.asset)?;
                return match asset.mounts.get(part) {
                    Some((at, face)) => {
                        let f = Frame::new((pl.origin.x, pl.origin.y), pl.facing, pl.scale).ok()?;
                        let w = f.to_world(*at).ok()?;
                        Some(Spot::At {
                            p: P::new(w.0, w.1),
                            facing: Some(f.facing_to_world(*face)),
                            end: false,
                        })
                    }
                    None => {
                        self.diags.push(text::with_nearest(
                            at_key(
                                codes::identity::UNKNOWN_PLACE,
                                top,
                                id,
                                key,
                                &format!("`{part}` is no mount of `{host}`'s asset `{}`", pl.asset),
                                &text::near_hint(
                                    part,
                                    asset.mounts.keys().map(String::as_str),
                                    "name a mount the asset declares",
                                ),
                            ),
                            part,
                            asset.mounts.keys().map(String::as_str),
                            |_: &str| self.pack.declaration("asset", &pl.asset),
                        ));
                        None
                    }
                };
            }
        }
        // A name whose declaration failed its own check still names it (`diagnostics.md` 4): its
        // record is the one cause, and the reader reads the failed entity, which the graph makes
        // an edge to the failed node, so the reader is `skipped`, not `names nothing`.
        if self.failed_declaration(name) {
            return None;
        }
        let mut names: Vec<String> = self.d.declared.keys().cloned().collect();
        for (fid, f) in &self.d.features {
            if matches!(f.shape, ShapeDecl::Path(_)) {
                names.push(format!("{fid}.start"));
                names.push(format!("{fid}.end"));
            }
        }
        self.diags.push(text::with_nearest(
            at_key(
                codes::identity::UNKNOWN_PLACE,
                top,
                id,
                key,
                &format!("`{name}` names nothing"),
                &text::near_hint(
                    name,
                    names.iter().map(String::as_str),
                    "name a feature, an end, an instance or a mount",
                ),
            ),
            name,
            names.iter().map(String::as_str),
            |n: &str| declared_at(self.d, n),
        ));
        None
    }

    /// Whether a place name names a declared feature, instance or pin whose declaration failed:
    /// the name itself, a path's end, or an instance's mount or `ground`.
    pub(crate) fn failed_declaration(&self, name: &str) -> bool {
        failed_declaration(self.d, name)
    }

    fn host_missing(&mut self, top: &str, id: &str, key: &str, host: &str) {
        if self.d.gave_way.contains(host) || self.gone.contains(host) {
            self.gone.insert(id.to_string());
            return;
        }
        if self.failed.contains(host) && !self.out.missing.contains(host) {
            return;
        }
        let mut d = at_key(
            codes::surface::HOST_MISSING,
            top,
            id,
            key,
            &format!("`{host}` was not placed, so nothing stands on it"),
            "fix the host's own fault",
        );
        d = d.related(via_diag::Related {
            subject: Subject::entity("instance", host),
            site: Site::Header,
            position: None,
            note: via_diag::text!("the host that was not placed"),
        });
        self.diags.push(d);
        self.out.missing.insert(id.to_string());
    }

    /// The point and facing of a placement (`rules.md` 3.3).
    fn locate(
        &mut self,
        top: &str,
        id: &str,
        place: &Place,
        face: Option<Face>,
        feature: bool,
    ) -> Option<(P, i64)> {
        let _ = feature;
        match place {
            Place::At(p) => {
                let facing = match face {
                    None => 0,
                    Some(Face::Bearing(b)) => b,
                    Some(_) => {
                        self.diags.push(at_key(
                            codes::geom::FACE_NEEDS_ON,
                            top,
                            id,
                            "face",
                            "a facing word needs `on`",
                            "write a bearing in degrees",
                        ));
                        return None;
                    }
                };
                Some((pt(*p), facing))
            }
            Place::On(On {
                reference,
                t,
                offset,
            }) => {
                let spot = self.spot(top, id, "on", reference)?;
                self.on_spot(top, id, reference, spot, *t, *offset, face)
            }
            Place::Of(sel) => {
                let spot = self.select(top, id, sel)?;
                self.on_spot(top, id, &sel.of, spot, sel.t, sel.offset, face)
            }
        }
    }

    /// The point and facing at a resolved place, by `t`, `offset` and `face` (`geometry.md` 2.6).
    #[allow(clippy::too_many_arguments)] // one placement's slots, as `geometry.md` 2.6 lists them
    pub(crate) fn on_spot(
        &mut self,
        top: &str,
        id: &str,
        reference: &str,
        spot: Spot,
        t: Option<f64>,
        offset: Option<i64>,
        face: Option<Face>,
    ) -> Option<(P, i64)> {
        match spot {
            Spot::Long(g) => {
                let line = match &g {
                    Geo::Path { line } => line.clone(),
                    Geo::Region { line: Some(l), .. } => l.clone(),
                    Geo::Region { region, .. } if region.is_empty() => {
                        self.diags.push(at_key(
                            codes::geom::NO_LENGTH,
                            top,
                            id,
                            "on",
                            &format!("`{reference}` is an empty region, which has no length"),
                            "use `at`, or place on a region that has pieces",
                        ));
                        return None;
                    }
                    _ => {
                        self.diags.push(at_key(
                            codes::geom::MANY_RINGS,
                            top,
                            id,
                            "on",
                            "`t` is undefined on a region of several rings",
                            "place it on a point feature instead",
                        ));
                        return None;
                    }
                };
                let Some(t) = t else {
                    self.diags.push(at_path(
                        codes::doc::MISSING_KEY,
                        top,
                        id,
                        &["on"],
                        "`on` a path or region needs `t`",
                        "write the whole table: `on = { ref, t, offset }`",
                    ));
                    return None;
                };
                let off = offset.unwrap_or(0);
                let at = match lineref::interpolate(&line, reference, t, off) {
                    Ok(a) => a,
                    Err(e) => {
                        self.diags.push(e.to_diag(
                            Subject::entity(top, id),
                            Site::Key(text::path(&["on", "t"])),
                        ));
                        return None;
                    }
                };
                let facing = match face {
                    None | Some(Face::Along) => at.bearing,
                    Some(Face::Bearing(b)) => b,
                    Some(Face::Toward) | Some(Face::Away) => {
                        let on_line = lineref::interpolate(&line, reference, t, 0).ok()?.point;
                        let (dx, dy) = (on_line.x - at.point.x, on_line.y - at.point.y);
                        let Some(b) = via_num::bearing(dx, dy) else {
                            self.diags.push(at_key(
                                codes::geom::FACE_SIDE,
                                top,
                                id,
                                "face",
                                "`toward` and `away` need an offset to face across",
                                "write a bearing, or give `offset`",
                            ));
                            return None;
                        };
                        if face == Some(Face::Away) {
                            via_num::normalise_facing(b + 180_000)
                        } else {
                            b
                        }
                    }
                };
                Some((at.point, facing))
            }
            Spot::At { p, facing, end } => {
                if t.is_some() || offset.is_some() {
                    self.diags.push(at_key(
                        codes::geom::NO_LENGTH,
                        top,
                        id,
                        "on",
                        &format!("`{reference}` has no length, so no `t` or `offset`"),
                        "place a set on it, or drop `t` and `offset`",
                    ));
                    return None;
                }
                let host = facing.unwrap_or(0);
                let facing = match face {
                    None | Some(Face::Along) => host,
                    Some(Face::Bearing(b)) if end => b,
                    Some(Face::Bearing(b)) => via_num::normalise_facing(host + b),
                    Some(_) => {
                        self.diags.push(at_key(
                            codes::geom::FACE_SIDE,
                            top,
                            id,
                            "face",
                            "`toward` and `away` need a line and an offset",
                            "write a bearing, or `along`",
                        ));
                        return None;
                    }
                };
                Some((p, facing))
            }
        }
    }

    #[allow(clippy::too_many_arguments)] // one placement's facts, as `rules.md` 2 lists them
    fn plan_of(
        &mut self,
        top: &str,
        id: &str,
        asset: &str,
        place: &Place,
        face: Option<Face>,
        scale: f64,
        tint: f64,
        rank: i64,
    ) -> Option<Plan> {
        let (origin, facing) = self.locate(top, id, place, face, false)?;
        let on = match place {
            Place::On(o) => Some(o.reference.clone()),
            Place::Of(o) => Some(o.of.clone()),
            Place::At(_) => None,
        };
        self.plan_at(top, id, asset, origin, facing, scale, tint, on, rank)
    }

    /// A plan at a point and facing: the asset and its footprint there (`rules.md` 2).
    #[allow(clippy::too_many_arguments)] // one placement's facts, as `rules.md` 2 lists them
    pub(crate) fn plan_at(
        &mut self,
        top: &str,
        id: &str,
        asset: &str,
        origin: P,
        facing: i64,
        scale: f64,
        tint: f64,
        on: Option<String>,
        rank: i64,
    ) -> Option<Plan> {
        let Some(a) = self.pack.assets.get(asset) else {
            if self.pack.failed {
                return None;
            }
            self.diags.push(
                text::with_nearest(
                    at_key(
                        codes::asset::UNKNOWN,
                        top,
                        id,
                        "asset",
                        &format!("the pack binds no asset `{asset}`"),
                        &text::near_hint(
                            asset,
                            self.pack.assets.keys().map(String::as_str),
                            "name an asset the pack declares",
                        ),
                    ),
                    asset,
                    self.pack.assets.keys().map(String::as_str),
                    |n: &str| self.pack.declaration("asset", n),
                )
                .at(text::pos(origin.x, origin.y)),
            );
            return None;
        };
        let Some(fp) = footprint(a, origin, facing, scale) else {
            self.diags.push(at_key(
                codes::geom::RANGE,
                top,
                id,
                "asset",
                "the footprint leaves the lattice",
                "move it",
            ));
            return None;
        };
        Some(Plan {
            name: id.to_string(),
            asset: asset.to_string(),
            origin,
            facing,
            scale,
            tint,
            footprint: fp,
            on,
            rank,
        })
    }

    /// A hand instance's plan, and its footprint feature.
    pub fn instance(&mut self, id: &str) -> Option<Plan> {
        if let Some(p) = self.out.plans.get(id) {
            return Some(p.clone());
        }
        if self.failed.contains(id) {
            return None;
        }
        let i = self.d.instances.get(id)?;
        if !self.enter(id, "instance") {
            return None;
        }
        let plan = self.plan_of(
            "instance", id, &i.asset, &i.place, i.face, i.scale, i.tint, i.rank,
        );
        self.leave();
        let Some(plan) = plan else {
            self.failed.insert(id.to_string());
            self.out.missing.insert(id.to_string());
            return None;
        };
        if let Some(g) = &i.ground {
            if plan.footprint.is_empty() {
                self.diags.push(at_key(
                    codes::place::NO_FOOTPRINT,
                    "instance",
                    id,
                    "ground",
                    &format!("the asset `{}` has a point footprint", plan.asset),
                    "declare the asset's `footprint`, or remove `ground`",
                ));
            } else {
                match offset(id, &plan.footprint, i.margin, Join::Mitre(2.0)) {
                    Ok((r, _)) => {
                        let name = format!("{id}.ground");
                        let line = (r.pieces.len() == 1)
                            .then(|| Line::new(r.pieces[0].shell.clone(), true));
                        self.out
                            .shapes
                            .insert(name.clone(), Geo::Region { region: r, line });
                        self.out
                            .footprints
                            .insert(name, (id.to_string(), g.clone()));
                    }
                    Err(e) => self.geom_fault("instance", id, "margin", e),
                }
            }
        }
        self.out.plans.insert(id.to_string(), plan.clone());
        Some(plan)
    }

    /// A pin's instance plan (`identity.md` 4.2).
    pub fn pin(&mut self, id: &str) -> Option<Plan> {
        if let Some(p) = self.out.pins.get(id) {
            return Some(p.clone());
        }
        if self.failed.contains(id) {
            return None;
        }
        let pd = self.d.pins.get(id)?;
        if !self.enter(id, "pin") {
            return None;
        }
        let plan = self.plan_of("pin", id, &pd.asset, &pd.place, pd.face, 1.0, 1.0, pd.rank);
        self.leave();
        match plan {
            Some(p) => {
                self.out.pins.insert(id.to_string(), p.clone());
                Some(p)
            }
            None => {
                self.failed.insert(id.to_string());
                None
            }
        }
    }

    /// A path along another feature (`geometry.md` 3.5 and 4).
    fn follows_path(&mut self, f: &FeatureDecl, fl: &Follows) -> Option<Geo> {
        let reference = match self.spot("feature", &f.id, "follows", &fl.reference)? {
            Spot::Long(Geo::Path { line }) => Reference::Path(line.pts.clone()),
            Spot::Long(Geo::Region { region, .. }) if region.pieces.len() == 1 => {
                Reference::Ring(region.pieces[0].shell.clone())
            }
            Spot::Long(Geo::Region { region, .. }) => {
                let what = if region.is_empty() {
                    (
                        codes::geom::NO_LENGTH,
                        "an empty region, which has no outline",
                    )
                } else {
                    (
                        codes::geom::MANY_RINGS,
                        "a region of several rings, which has no one `t`",
                    )
                };
                self.diags.push(at_path(
                    what.0,
                    "feature",
                    &f.id,
                    &["follows", "ref"],
                    &format!("`{}` is {}", fl.reference, what.1),
                    "follow a path, or a region of one ring",
                ));
                return None;
            }
            Spot::Long(Geo::Point { .. }) | Spot::At { .. } => {
                self.diags.push(at_path(
                    codes::geom::NO_LENGTH,
                    "feature",
                    &f.id,
                    &["follows", "ref"],
                    &format!("`{}` has no length to follow", fl.reference),
                    "follow a path or a region, or place the feature with `on`",
                ));
                return None;
            }
        };
        let closed = matches!(reference, Reference::Ring(_)) && fl.span.0 == fl.span.1;
        match follow(&f.id, &reference, fl.span.0, fl.span.1, fl.offset) {
            Ok((mut pts, gd)) => {
                for g in gd {
                    self.geom_fault("feature", &f.id, "follows", g);
                }
                if closed && pts.len() > 1 && pts.first() == pts.last() {
                    pts.pop();
                }
                Some(Geo::Path {
                    line: Line::new(pts, closed),
                })
            }
            Err(e) => {
                self.geom_fault("feature", &f.id, "follows", e);
                None
            }
        }
    }

    /// Places every shape a shape formula reads, through the fields it reads too (a `where`
    /// region, a route's cost), before the formula is evaluated: dependency order, not id order
    /// (`graph.md` 2). False when the feature itself failed, as in a cycle through its own read.
    fn place_reads(&mut self, id: &str, formula: via_expr::FormulaId) -> bool {
        for sel in self.d.where_reads(formula).1 {
            match sel.strip_prefix("kind:") {
                Some(k) => {
                    let ids: Vec<String> = self
                        .d
                        .features
                        .iter()
                        .filter(|(_, x)| x.kind.as_deref() == Some(k))
                        .map(|(i, _)| i.clone())
                        .collect();
                    for i in ids {
                        self.feature(&i);
                    }
                }
                None => {
                    let base = sel
                        .strip_suffix(".ground")
                        .filter(|h| self.d.instances.contains_key(*h));
                    match base {
                        Some(inst) => {
                            let inst = inst.to_string();
                            self.instance(&inst);
                        }
                        None if self.d.features.contains_key(&sel) => {
                            self.feature(&sel);
                        }
                        None => {}
                    }
                }
            }
        }
        !self.failed.contains(id)
    }

    /// A region written as a condition (`geometry.md` 3.7): the formula in the head view at every
    /// post, masked, every piece kept and named by place (`identity.md` 2.3).
    fn where_region(&mut self, f: &FeatureDecl, formula: via_expr::FormulaId) -> Option<Geo> {
        let grid = self.d.grid?;
        // What the shape reads is resolved first; reading itself is a cycle (`formulas.md` 6.6).
        if !self.place_reads(&f.id, formula) {
            return None;
        }
        let snapshot = self.out.clone();
        let fields = Fields::new(self.d, &snapshot, grid);
        let values = fields.eval_where(formula, &f.id);
        self.diags.extend(fields.diags.borrow().iter().cloned());
        let values = values?;
        let mg = MaskGrid {
            x0: grid.extent[0],
            y0: grid.extent[1],
            r: grid.resolution,
            nx: usize::try_from(grid.nx).ok()?,
            ny: usize::try_from(grid.ny).ok()?,
        };
        let text = self.d.formulas.env.value(formula).unwrap_or("").to_string();
        let region = match f.holds {
            // `holds` (`derived.md` 2): the one piece holding the point.
            Some((x, y)) => {
                match crate::derived::where_region(&grid, &values, &text, Some(P::new(x, y))) {
                    Ok(found) => {
                        for g in found.diags {
                            let key = if matches!(
                                g.code,
                                via_geom::Code::DeriveDry | via_geom::Code::DeriveOpen
                            ) {
                                "holds"
                            } else {
                                "where"
                            };
                            self.geom_fault("feature", &f.id, key, g);
                        }
                        found.region
                    }
                    Err(e) => {
                        self.geom_fault("feature", &f.id, "holds", e);
                        return None;
                    }
                }
            }
            None => match mask(mg, &values, &text, None) {
                Ok((r, gd)) => {
                    for g in gd {
                        self.geom_fault("feature", &f.id, "where", g);
                    }
                    r
                }
                Err(e) => {
                    self.geom_fault("feature", &f.id, "where", e);
                    return None;
                }
            },
        };
        if region.is_empty() {
            return Some(Geo::Region { region, line: None });
        }
        self.name_pieces(&f.id, &region, grid);
        let line =
            (region.pieces.len() == 1).then(|| Line::new(region.pieces[0].shell.clone(), true));
        Some(Geo::Region { region, line })
    }

    /// Names every piece of a `where` region by place: the 32 m square holding its
    /// representative point, and an ordinal by that point, `x` then `y` (`identity.md` 2.3).
    fn name_pieces(&mut self, id: &str, region: &Region, grid: via_num::Grid) {
        let pg = PostGrid {
            extent: grid.extent,
            resolution: grid.resolution,
        };
        let mut found: Vec<((i64, i64), P, Region)> = region
            .pieces
            .iter()
            .map(|pc| {
                let r = Region {
                    pieces: vec![pc.clone()],
                };
                let point = representative(&r, pg);
                (
                    (
                        point.x.div_euclid(PIECE_SQUARE_MM),
                        point.y.div_euclid(PIECE_SQUARE_MM),
                    ),
                    point,
                    r,
                )
            })
            .collect();
        found.sort_by_key(|(sq, p, _)| (*sq, p.x, p.y));
        let mut out = Vec::new();
        let mut last: Option<(i64, i64)> = None;
        let mut ordinal = 0;
        for (sq, point, r) in found {
            ordinal = if last == Some(sq) { ordinal + 1 } else { 0 };
            last = Some(sq);
            out.push(PieceOut {
                name: format!(
                    "{id}#p{}_{}_{ordinal}",
                    crate::rules::key_int(sq.0),
                    crate::rules::key_int(sq.1)
                ),
                region: r,
                point,
            });
        }
        out.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
        self.out.pieces.insert(id.to_string(), out);
    }

    /// A selection by place (`identity.md` 3), resolved to the thing nearest `near`. A hand edit
    /// that selects a scatter's instance is deferred until the scatter has run (4.1).
    pub(crate) fn select(&mut self, top: &str, id: &str, sel: &Select) -> Option<Spot> {
        if self.d.is_maker_table(&sel.of) {
            return self.select_made(top, id, sel);
        }
        let key: &[&str] = if top == "scatter" {
            &["within", "of"]
        } else {
            &["on", "of"]
        };
        let near = pt(sel.near);
        // A step's returned features (`identity.md` 2.4): the class `feature` of a maker that is
        // a step, nearest `near`; ties to the bytewise-least name.
        if self.d.steps.contains_key(&sel.of) {
            if sel.class.as_deref().is_some_and(|c| c != "feature") {
                self.near_none(&sel.of, sel.class.as_deref().unwrap_or(""), &[(top, id)]);
                return None;
            }
            let prefix = format!("{}#", sel.of);
            let ids: Vec<String> = self
                .d
                .features
                .keys()
                .filter(|k| k.starts_with(&prefix))
                .cloned()
                .collect();
            // Ids in name order and a strict `<`: a tie keeps the bytewise-least name.
            let mut best: Option<(i128, P, String, Geo)> = None;
            for f in ids {
                let Some(g) = self.feature(&f) else { continue };
                let (d, at) = g.nearest_mm(near);
                if best.as_ref().is_none_or(|(b, ..)| d < *b) {
                    best = Some((d, at, f, g));
                }
            }
            let Some((_, at, name, geo)) = best else {
                self.near_none(&sel.of, "feature", &[(top, id)]);
                return None;
            };
            self.out
                .selections
                .insert(format!("{top}.{id}"), (name, at));
            return Some(match geo {
                Geo::Point { p, facing } => Spot::At {
                    p,
                    facing,
                    end: false,
                },
                g => Spot::Long(g),
            });
        }
        let is_where = self
            .d
            .features
            .get(&sel.of)
            .is_some_and(|f| matches!(f.shape, ShapeDecl::Where(_)));
        if self.d.scatters.contains_key(&sel.of) {
            if sel.class.as_deref().is_some_and(|c| c != "instance") {
                self.near_none(&sel.of, sel.class.as_deref().unwrap_or(""), &[(top, id)]);
                return None;
            }
            if matches!(top, "pin" | "clear") {
                self.out.deferred.insert(format!("{top}.{id}"), sel.clone());
                return None;
            }
            let why = if top == "scatter" {
                "`within` takes a region: an `area`, `cell`, `rest` or `piece`"
            } else {
                "a feature or an instance placed on a scatter's instance waits for the instance nodes of `graph.md`; in M1 only a pin or a clear selects a scatter's instance"
            };
            self.diags.push(at_path(
                if top == "scatter" {
                    codes::scatter::WITHIN
                } else {
                    codes::doc::NOT_YET
                },
                top,
                id,
                key,
                &format!("`{}` is a scatter: {why}", sel.of),
                "select a piece of a `where` region, or name an authored thing with `ref`",
            ));
            return None;
        }
        if !is_where {
            let mut makers: Vec<&str> = self.d.scatters.keys().map(String::as_str).collect();
            makers.extend(
                self.d
                    .features
                    .iter()
                    .filter(|(_, f)| matches!(f.shape, ShapeDecl::Where(_)))
                    .map(|(i, _)| i.as_str()),
            );
            let what = if self.d.features.contains_key(&sel.of)
                || self.d.instances.contains_key(&sel.of)
            {
                format!(
                    "`{}` is an authored thing, not a maker; name it with `ref`",
                    sel.of
                )
            } else {
                format!("`{}` names no maker", sel.of)
            };
            self.diags.push(text::with_nearest(
                at_path(
                    codes::identity::UNKNOWN_MAKER,
                    top,
                    id,
                    key,
                    &what,
                    &text::near_hint(
                        &sel.of,
                        makers.iter().copied(),
                        "name a scatter or a `where` feature",
                    ),
                ),
                &sel.of,
                makers.iter().copied(),
                text::header_of("scatter"),
            ));
            return None;
        }
        if sel.class.as_deref().is_some_and(|c| c != "piece") {
            self.near_none(&sel.of, sel.class.as_deref().unwrap_or(""), &[(top, id)]);
            return None;
        }
        self.feature(&sel.of)?;
        let pieces = self.out.pieces.get(&sel.of).cloned().unwrap_or_default();
        let zero = Dist2 {
            num: Wide::from_u128(0),
            den: Wide::from_u128(1),
        };
        let mut best: Option<(Dist2, &PieceOut)> = None;
        for pc in &pieces {
            let d = if pc.region.side(near) != Side::Out {
                zero
            } else {
                match dist2_rings(near, &pc.region.rings()) {
                    Some(d) => d,
                    None => continue,
                }
            };
            // Pieces are in name order, so a tie keeps the bytewise-least name.
            if best
                .as_ref()
                .is_none_or(|(b, _)| d.cmp_exact(b) == std::cmp::Ordering::Less)
            {
                best = Some((d, pc));
            }
        }
        let Some((_, pc)) = best else {
            self.near_none(&sel.of, "piece", &[(top, id)]);
            return None;
        };
        self.out
            .selections
            .insert(format!("{top}.{id}"), (pc.name.clone(), pc.point));
        let line = Line::new(pc.region.pieces[0].shell.clone(), true);
        let single = pc.region.pieces.len() == 1;
        Some(Spot::Long(Geo::Region {
            region: pc.region.clone(),
            line: single.then_some(line),
        }))
    }

    /// `identity.near_none`: a maker with no candidate of the class, at the maker, listing the
    /// selections it left with nothing.
    pub(crate) fn near_none(&mut self, maker: &str, class: &str, who: &[(&str, &str)]) {
        let top = if let Some(t) = self.d.maker_top(maker) {
            t
        } else if self.d.scatters.contains_key(maker) {
            "scatter"
        } else if self.d.steps.contains_key(maker) {
            "step"
        } else {
            "feature"
        };
        let key = (maker.to_string(), class.to_string());
        let mut list = self.near.get(&key).map(|n| n.1.clone()).unwrap_or_default();
        for (t, i) in who {
            let s = format!("{t}.{i}");
            if !list.contains(&s) {
                list.push(s);
            }
        }
        list.sort();
        let d = text::at_header(
            codes::identity::NEAR_NONE,
            top,
            maker,
            &format!(
                "`{maker}` has no `{class}` to select; left with nothing: {}",
                list.join(", ")
            ),
            "select from a maker that makes that class, or change what it makes",
        )
        .with("selections", DValue::texts(list.iter().map(String::as_str)));
        match self.near.get(&key).map(|n| n.0) {
            Some(i) => self.diags[i] = d,
            None => {
                self.near
                    .insert(key.clone(), (self.diags.len(), Vec::new()));
                self.diags.push(d);
            }
        }
        if let Some(n) = self.near.get_mut(&key) {
            n.1 = list;
        }
    }

    /// A clear's place (`identity.md` 4.3).
    fn clear(&mut self, id: &str) {
        let Some(c) = self.d.clears.get(id) else {
            return;
        };
        if let Some(r) = &c.region {
            let ring: Vec<P> = r.iter().map(|p| pt(*p)).collect();
            self.out
                .clears
                .insert(id.to_string(), ClearSpot::Inside(Region::from_ring(ring)));
            return;
        }
        let radius = c.radius;
        let (spot, t, offset, reference) = match &c.place {
            Some(Place::At(p)) => (
                Spot::At {
                    p: pt(*p),
                    facing: None,
                    end: false,
                },
                None,
                None,
                String::new(),
            ),
            Some(Place::On(o)) => match self.spot("clear", id, "on", &o.reference) {
                Some(s) => (s, o.t, o.offset, o.reference.clone()),
                None => return,
            },
            Some(Place::Of(sel)) => match self.select("clear", id, sel) {
                Some(s) => (s, sel.t, sel.offset, sel.of.clone()),
                None => return,
            },
            None => return,
        };
        let need_radius = |me: &mut Self| {
            me.diags.push(at_key(
                codes::doc::MISSING_KEY,
                "clear",
                id,
                "radius",
                "a clear at a point needs `radius`",
                "write `radius`",
            ));
        };
        let spot_out = match spot {
            Spot::At { p, .. } => {
                if t.is_some() || offset.is_some() {
                    self.diags.push(at_key(
                        codes::geom::NO_LENGTH,
                        "clear",
                        id,
                        "on",
                        &format!("`{reference}` has no length, so no `t` or `offset`"),
                        "drop `t` and `offset`",
                    ));
                    return;
                }
                let Some(r) = radius else {
                    need_radius(self);
                    return;
                };
                ClearSpot::Disc(p, r)
            }
            Spot::Long(Geo::Region { region, .. }) => {
                if radius.is_some() || t.is_some() {
                    self.diags.push(at_key(
                        codes::place::ON_FORM,
                        "clear",
                        id,
                        "on",
                        "a clear `on` a region takes neither `t` nor `radius`: it empties the region",
                        "remove `t` and `radius`, or clear `on` a point with `radius`",
                    ));
                    return;
                }
                ClearSpot::Inside(region)
            }
            Spot::Long(Geo::Path { line }) => {
                let Some(r) = radius else {
                    need_radius(self);
                    return;
                };
                match t {
                    Some(t) => {
                        match lineref::interpolate(&line, &reference, t, offset.unwrap_or(0)) {
                            Ok(a) => ClearSpot::Disc(a.point, r),
                            Err(e) => {
                                self.diags.push(e.to_diag(
                                    Subject::entity("clear", id),
                                    Site::Key(text::path(&["on", "t"])),
                                ));
                                return;
                            }
                        }
                    }
                    None => ClearSpot::Strip(line, r),
                }
            }
            Spot::Long(Geo::Point { p, .. }) => {
                let Some(r) = radius else {
                    need_radius(self);
                    return;
                };
                ClearSpot::Disc(p, r)
            }
        };
        self.out.clears.insert(id.to_string(), spot_out);
    }

    /// Name attributes (`authority.md` 1.2), resolved to the place they give.
    fn name_attributes(&mut self) {
        let d = self.d;
        for (fid, f) in &d.features {
            let Some(k) = f.kind.as_ref().and_then(|k| d.kinds.get(k)) else {
                continue;
            };
            for n in &k.names {
                let Some(crate::decl::Attr::Text(target)) = f.attrs.get(n) else {
                    continue;
                };
                if let Some(s) = self.spot("feature", fid, n, target) {
                    let p = match s {
                        Spot::At { p, .. } => p,
                        Spot::Long(g) => g.name_point(),
                    };
                    self.out
                        .names
                        .insert((fid.clone(), n.clone()), (target.clone(), p));
                }
            }
        }
    }
}

/// The declaration a place name reads (`identity.md` 1), for a not-found record's `related`
/// (`diagnostics.md` 1.3): the header of the feature, instance or pin it names, through a path's
/// end, an instance's mount or its `ground`.
pub(crate) fn declared_at(d: &Decls, name: &str) -> Option<(Subject, Site)> {
    let base = name
        .strip_suffix(".start")
        .or_else(|| name.strip_suffix(".end"))
        .unwrap_or(name);
    let head = base.split('.').next().unwrap_or(base);
    let top = *d.declared.get(base).or_else(|| d.declared.get(head))?;
    let id = if d.declared.contains_key(base) {
        base
    } else {
        head
    };
    Some((Subject::entity(top, id), Site::Header))
}
