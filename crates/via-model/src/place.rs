//! Shapes and placements (`geometry.md` 2, `rules.md` 2 and 3, `identity.md` 1): every feature's
//! resolved geometry, every hand instance's and pin's plan, and place names (`beck.end` style:
//! `<path>.start`, `<path>.end`, `<instance>`, `<instance>.<mount>`, `<instance>.ground`).
//!
//! Resolution is lazy and memoised: a feature placed `on` an instance waits for that instance's
//! plan, which may wait for a feature. A name met again while it is being resolved is a cycle,
//! `graph.cycle`, naming every member.

use std::collections::{BTreeMap, BTreeSet};

use via_diag::{codes, Diag, KeyPath, Site, Subject};
use via_geom::lineref::{self, Line};
use via_geom::offset::{offset, Join};
use via_geom::shape::{read_path, read_ring};
use via_geom::{Region, P};
use via_num::Frame;

use crate::decl::{Decls, Face, FeatureDecl, On, Place, ShapeDecl};
use crate::pack::{Asset, Pack};
use crate::text::{self, at_key, at_path};

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
#[derive(Debug, Default)]
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
}

pub struct Placer<'a> {
    d: &'a Decls,
    pack: &'a Pack,
    pub out: Placed,
    visiting: Vec<String>,
    failed: BTreeSet<String>,
    pub diags: Vec<Diag>,
    cycles: BTreeSet<Vec<String>>,
}

fn pt(p: (i64, i64)) -> P {
    P::new(p.0, p.1)
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
    pub fn new(d: &'a Decls, pack: &'a Pack) -> Placer<'a> {
        Placer {
            d,
            pack,
            out: Placed::default(),
            visiting: Vec::new(),
            failed: BTreeSet::new(),
            diags: Vec::new(),
            cycles: BTreeSet::new(),
        }
    }

    /// Resolves every feature, instance, pin and footprint, in id order.
    pub fn run(mut self) -> (Placed, Vec<Diag>) {
        for id in self.d.features.keys() {
            self.feature(id);
        }
        for id in self.d.instances.keys() {
            self.instance(id);
        }
        for id in self.d.pins.keys() {
            self.pin(id);
        }
        self.name_attributes();
        (self.out, self.diags)
    }

    fn enter(&mut self, name: &str, top: &str) -> bool {
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

    fn leave(&mut self) {
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
                let mut gd = Vec::new();
                let path = read_path(&f.id, &v, &mut gd);
                for g in gd {
                    self.geom_fault("feature", &f.id, "path", g);
                }
                Some(Geo::Path {
                    line: Line::new(path?, false),
                })
            }
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
                        Place::On(_) => Some(facing),
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
                        self.diags.push(at_key(
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
                        ));
                        None
                    }
                };
            }
        }
        let mut names: Vec<String> = self.d.features.keys().cloned().collect();
        names.extend(self.d.instances.keys().cloned());
        for (fid, f) in &self.d.features {
            if matches!(f.shape, ShapeDecl::Path(_)) {
                names.push(format!("{fid}.start"));
                names.push(format!("{fid}.end"));
            }
        }
        self.diags.push(at_key(
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
        ));
        None
    }

    fn host_missing(&mut self, top: &str, id: &str, key: &str, host: &str) {
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
                match spot {
                    Spot::Long(g) => {
                        let line = match &g {
                            Geo::Path { line } => line.clone(),
                            Geo::Region { line: Some(l), .. } => l.clone(),
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
                        let Some(t) = *t else {
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
                                let on_line =
                                    lineref::interpolate(&line, reference, t, 0).ok()?.point;
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
        let Some(a) = self.pack.assets.get(asset) else {
            self.diags.push(
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
                )
                .at(text::pos(origin.x, origin.y)),
            );
            if let Some(d) = self.diags.pop() {
                self.diags.push(text::with_nearest(
                    d,
                    asset,
                    self.pack.assets.keys().map(String::as_str),
                ));
            }
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
        let on = match place {
            Place::On(o) => Some(o.reference.clone()),
            Place::At(_) => None,
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
                        let line = (r.ring_count() == 1)
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
