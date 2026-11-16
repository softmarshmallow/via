//! Kind surfaces, kind mounts and the meetings between surfaces (`surfaces.md` 1, 3; M3 and M5).
//!
//! A kind surface is declared on its kind and owned by each feature of the kind: its bound is the
//! owner's region, or the owner's path stroked by `bound` (1.2); its height is the `base` formula
//! at the point asked (1.1). A kind mount is a named place on its owner (3.1). Meetings are path
//! ends at places and `meets` pairs (3.3); what must hold at each is 3.4. The words are the core's
//! own: surfaces, mounts, connectors; what a surface is for is a library's.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use via_diag::{codes, Diag, Site, Subject, Value as DValue};
use via_doc::Table;
use via_expr::{Binding, Domain, Program, ReadError, Reads, Scalar, Setup};
use via_geom::offset::{Cap, Join};
use via_geom::P;

use crate::decl::{Attr, Decls, KindMount, KindSurface};
use crate::place::Geo;
use crate::text::{self, at_path};

/// Names a kind mount or surface may not take (`surfaces.md` 1, rule 3).
const RESERVED: [&str; 3] = ["start", "end", "ground"];

/// Reads `[kind.<k>.surface.*]`, `[kind.<k>.mount.*]` and checks what the kind alone shows.
pub(crate) fn read_kind(d: &mut Decls, kind: &str, t: &Table, k: &mut crate::decl::Kind) {
    for (name, st) in crate::decl::table_of(t, "surface") {
        if RESERVED.contains(&name) {
            d.push(at_path(
                codes::identity::RESERVED,
                "kind",
                kind,
                &["surface", name],
                &format!("a surface may not be named `{name}`"),
                "choose another name",
            ));
            continue;
        }
        let get = |key: &str, d: &mut Decls| {
            st.value(key)
                .and_then(text::formula_text)
                .and_then(|r| r.ok())
                .map(|ft| {
                    d.formulas.add(
                        &ft,
                        Subject::entity("kind", kind),
                        text::path(&["surface", name, key]),
                    )
                })
        };
        let base = get("base", d);
        let bound = get("bound", d);
        let cut = get("cut", d);
        k.surfaces.insert(
            name.to_string(),
            KindSurface {
                name: name.to_string(),
                base,
                bound,
                cut,
                material: st.str("material").map(str::to_string),
            },
        );
    }
    for (name, mt) in crate::decl::table_of(t, "mount") {
        if RESERVED.contains(&name) {
            d.push(at_path(
                codes::identity::RESERVED,
                "kind",
                kind,
                &["mount", name],
                &format!("a mount may not be named `{name}`"),
                "choose another name",
            ));
            continue;
        }
        if k.surfaces.contains_key(name) {
            d.push(at_path(
                codes::surface::NAME_CLASH,
                "kind",
                kind,
                &["mount", name],
                &format!("`{name}` names both a surface and a mount of `{kind}`"),
                "rename one: a kind's mounts and surfaces share one namespace",
            ));
            continue;
        }
        let t_at = match mt.value("t").map(text::number) {
            Some(Some(Ok(x))) => x,
            _ => 0.0,
        };
        let offset = match mt.value("offset").and_then(text::length) {
            Some(Ok(mm)) => mm,
            _ => 0,
        };
        let face = match mt.value("face") {
            None => None,
            Some(via_doc::Value::Num(n)) => via_num::parse_facing(&n.norm).ok(),
            Some(_) => {
                d.push(at_path(
                    codes::surface::MOUNT_FACE,
                    "kind",
                    kind,
                    &["mount", name, "face"],
                    "a kind mount's `face` is a number of degrees from the owner's travel",
                    "write `face = <degrees>`: a kind mount has no `on` for a word to read",
                ));
                None
            }
        };
        k.mounts.insert(
            name.to_string(),
            KindMount {
                name: name.to_string(),
                t: t_at,
                offset,
                face,
                surface: mt.str("surface").map(str::to_string),
                meets: mt.str("meets").map(str::to_string),
                connector: mt.str("connector").map(str::to_string),
            },
        );
    }
    // `surface` and `meets` name things of this kind (3.1).
    let surfaces: Vec<&str> = k.surfaces.keys().map(String::as_str).collect();
    let mounts: Vec<&str> = k.mounts.keys().map(String::as_str).collect();
    let mut paired: BTreeMap<String, String> = BTreeMap::new();
    for m in k.mounts.values() {
        if let Some(s) = &m.surface {
            if !k.surfaces.contains_key(s) {
                d.push(text::with_nearest(
                    at_path(
                        codes::surface::UNKNOWN_SURFACE,
                        "kind",
                        kind,
                        &["mount", &m.name, "surface"],
                        &format!("`{kind}` has no surface `{s}`"),
                        &text::near_hint(
                            s,
                            surfaces.iter().copied(),
                            "name a surface the kind declares, or drop `surface` for the terrain",
                        ),
                    ),
                    s,
                    surfaces.iter().copied(),
                    text::undeclared,
                ));
            }
        }
        let Some(other) = &m.meets else { continue };
        if other == &m.name {
            d.push(at_path(
                codes::surface::MEETS_SELF,
                "kind",
                kind,
                &["mount", &m.name, "meets"],
                &format!("the mount `{}` meets itself", m.name),
                "name another mount of the kind",
            ));
            continue;
        }
        if !k.mounts.contains_key(other) {
            d.push(text::with_nearest(
                at_path(
                    codes::surface::UNKNOWN_MOUNT,
                    "kind",
                    kind,
                    &["mount", &m.name, "meets"],
                    &format!("`{kind}` has no mount `{other}`"),
                    &text::near_hint(
                        other,
                        mounts.iter().copied(),
                        "name a mount of the same kind",
                    ),
                ),
                other,
                mounts.iter().copied(),
                text::undeclared,
            ));
            continue;
        }
        // Both naming each other is one meeting; a mount in two pairs is an error.
        let (a, b) = if m.name < *other {
            (m.name.clone(), other.clone())
        } else {
            (other.clone(), m.name.clone())
        };
        for x in [&a, &b] {
            if let Some(p) = paired.get(x) {
                if *p != format!("{a}|{b}") {
                    d.push(at_path(
                        codes::surface::MEETS_TWICE,
                        "kind",
                        kind,
                        &["mount", &m.name, "meets"],
                        &format!("the mount `{x}` is in two `meets` pairs"),
                        "pair each mount once",
                    ));
                }
            }
        }
        paired.insert(a.clone(), format!("{a}|{b}"));
        paired.insert(b.clone(), format!("{a}|{b}"));
    }
}

/// A kind surface's height, evaluable at any lattice point after the build (2.2).
#[derive(Clone, Debug)]
pub struct SurfaceEval {
    program: Program,
    binding: Binding,
    attrs: BTreeMap<String, Attr>,
    geo: Geo,
    owner: String,
    seed: u64,
}

struct OwnerReads<'a> {
    e: &'a SurfaceEval,
}

impl Reads for OwnerReads<'_> {
    fn attribute(&self, feature: Option<&str>, name: &str) -> Result<Scalar, ReadError> {
        if feature.is_some_and(|f| f != self.e.owner) {
            return Err(ReadError::new(
                codes::diag::INTERNAL,
                "a kind surface reads only its owner's attributes",
                "report it: this is a defect of the engine",
            ));
        }
        match self.e.attrs.get(name) {
            Some(Attr::Number(x)) => Ok(Scalar::Number(*x)),
            Some(Attr::YesNo(b)) => Ok(Scalar::YesNo(*b)),
            _ => Err(ReadError::new(
                codes::formula::TYPE,
                format!("`{name}` is not a number here"),
                "write a number for the attribute",
            )),
        }
    }

    fn local(&self, measure: via_expr::Measure, domain: &Domain) -> Result<Vec<f64>, ReadError> {
        let pj = self.e.geo.projector();
        Ok(domain
            .points()
            .into_iter()
            .map(|(x, y)| {
                let m = pj.project(P::new(x, y));
                match measure {
                    via_expr::Measure::D => m.d,
                    via_expr::Measure::Sd => m.sd,
                    via_expr::Measure::T => m.t.unwrap_or(0.0),
                    via_expr::Measure::Inside => {
                        f64::from(u8::from(self.e.geo.contains(P::new(x, y))))
                    }
                }
            })
            .collect())
    }
}

impl SurfaceEval {
    /// The formula at each point, or the failure's diagnostic.
    pub fn eval(&self, points: &[(i64, i64)]) -> Result<Vec<f64>, Box<via_expr::ExprError>> {
        let setup = Setup {
            world_seed: self.seed,
            feature: Some(&self.owner),
        };
        via_expr::evaluate(
            &self.program,
            &self.binding,
            &OwnerReads { e: self },
            &setup,
            &Domain::Points(points),
        )
        .map_err(|f| Box::new(f.error))
    }
}

/// One kind surface of one owner (1).
#[derive(Clone, Debug)]
pub struct SurfaceOut {
    /// `<owner>.<name>`.
    pub place: String,
    pub owner: String,
    pub name: String,
    /// The bound, edges in (1.2).
    pub bound: Geo,
    pub base: Arc<SurfaceEval>,
    /// `cut` in metres, when written (1.3).
    pub cut: Option<f64>,
    pub material: Option<String>,
    /// Whether this surface's `formula.not_finite` was said in this build (2.7: once, at the
    /// first failing point asked).
    said: Arc<AtomicBool>,
}

impl SurfaceOut {
    /// `sample(surface, p).z`: `Ok(None)` outside the bound, the formula's fault where `base`
    /// fails or is not finite there (2.7).
    pub fn z(&self, p: (i64, i64)) -> Result<Option<f64>, Box<via_expr::ExprError>> {
        if !self.bound.contains(P::new(p.0, p.1)) {
            return Ok(None);
        }
        Ok(self.base.eval(&[p])?.first().copied())
    }

    /// [`SurfaceOut::z`] in a build: a fault is `formula.not_finite` (or the formula's own code)
    /// on the owner, said once per surface (2.7), and gives `None`; `Ok(None)` is outside.
    pub(crate) fn z_in_build(
        &self,
        p: (i64, i64),
        diags: &mut Vec<Diag>,
    ) -> Result<Option<f64>, ()> {
        match self.z(p) {
            Ok(z) => Ok(z),
            Err(e) => {
                if !self.said.swap(true, Ordering::Relaxed) {
                    diags.push(self.fault(&e));
                }
                Err(())
            }
        }
    }

    /// A fault of this surface's formula as a diagnostic on its owner (2.7).
    pub fn fault(&self, e: &via_expr::ExprError) -> Diag {
        let owner = self.owner.clone();
        e.to_diag(&|_, _| Subject::entity("feature", owner.as_str()), &|_| {
            Site::Header
        })
        .with("surface", DValue::text(self.place.clone()))
    }

    /// Whether the plan point lies in the bound, edges in.
    pub fn inside(&self, p: (i64, i64)) -> bool {
        self.bound.contains(P::new(p.0, p.1))
    }
}

/// The evaluator of formula `f` for feature `owner` (a kind surface's key).
fn evaluator(d: &Decls, f: via_expr::FormulaId, owner: &str, geo: &Geo) -> Option<SurfaceEval> {
    let program = d.formulas.programs.get(&f)?.clone();
    let binding = via_expr::bind(&program, &crate::decl::NoSelect).ok()?;
    let attrs = d.features.get(owner)?.attrs.clone();
    Some(SurfaceEval {
        program,
        binding,
        attrs,
        geo: geo.clone(),
        owner: owner.to_string(),
        seed: d.seed,
    })
}

/// Every kind surface of every placed feature, by place name (1); faults pushed. The second map
/// holds the surfaces a fault of their own left out, so a reader of one says nothing more.
pub(crate) fn kind_surfaces(
    d: &Decls,
    shapes: &BTreeMap<String, Geo>,
    diags: &mut Vec<Diag>,
) -> (BTreeMap<String, SurfaceOut>, BTreeSet<String>) {
    let mut out = BTreeMap::new();
    let mut failed = BTreeSet::new();
    let fault = |e: &via_expr::ExprError, id: &str| {
        e.to_diag(&|_, _| Subject::entity("feature", id), &|_| Site::Header)
    };
    for (id, f) in &d.features {
        let Some(k) = f.kind.as_ref().and_then(|k| d.kinds.get(k)) else {
            continue;
        };
        if k.surfaces.is_empty() {
            continue;
        }
        let Some(geo) = shapes.get(id) else { continue };
        for s in k.surfaces.values() {
            let subject = Subject::entity("feature", id.as_str());
            let place = format!("{id}.{}", s.name);
            // A formula that did not pass its check was said there; this surface is left out.
            failed.insert(place.clone());
            let bound = match (geo, s.bound) {
                (Geo::Path { line }, Some(bf)) => {
                    let Some(e) = evaluator(d, bf, id, geo) else {
                        continue;
                    };
                    let half = match e.eval(&[(0, 0)]) {
                        Ok(v) => v[0],
                        Err(err) => {
                            diags.push(fault(&err, id));
                            continue;
                        }
                    };
                    let mm = via_num::round_to_lattice(half * 1000.0).unwrap_or(0);
                    if !(half.is_finite() && mm > 0) {
                        diags.push(text::diag(
                            codes::surface::BOUND,
                            subject.clone(),
                            Site::Header,
                            &format!("`bound` is {half} for `{id}`; it must be above 0 and finite"),
                            "write a half width above 0",
                        ));
                        continue;
                    }
                    match via_geom::offset::stroke(id, &line.pts, mm, mm, Join::Round, Cap::Butt) {
                        Ok(region) => Geo::Region { region, line: None },
                        Err(_) => {
                            diags.push(text::diag(
                                codes::surface::BOUND,
                                subject.clone(),
                                Site::Header,
                                &format!("the path of `{id}` stroked by `bound` gives no region"),
                                "write a wider `bound`, or a path with length",
                            ));
                            continue;
                        }
                    }
                }
                (Geo::Region { .. }, None) => geo.clone(),
                _ => {
                    diags.push(text::diag(
                        codes::surface::OWNER_SHAPE,
                        subject.clone(),
                        Site::Header,
                        &format!(
                            "`{id}` is {} and its kind `{}` has the surface `{}`{}",
                            geo.word(),
                            k.id,
                            s.name,
                            if s.bound.is_some() {
                                " with a `bound`"
                            } else {
                                " with no `bound`"
                            }
                        ),
                        "a path owner needs `bound`; a region owner must not have one; a point owns no surface",
                    ));
                    continue;
                }
            };
            let Some(base) = s.base.and_then(|b| evaluator(d, b, id, geo)) else {
                continue;
            };
            let cut = match s.cut.and_then(|c| evaluator(d, c, id, geo)) {
                None => None,
                Some(e) => match e.eval(&[(0, 0)]) {
                    Ok(v) if v[0].is_finite() && v[0] >= 0.0 => Some(v[0]),
                    Ok(v) => {
                        diags.push(text::diag(
                            codes::surface::CUT,
                            subject.clone(),
                            Site::Header,
                            &format!(
                                "`cut` is {} for `{id}`; it must be 0 or more and finite",
                                v[0]
                            ),
                            "write a height of 0 or more",
                        ));
                        None
                    }
                    Err(err) => {
                        diags.push(fault(&err, id));
                        None
                    }
                },
            };
            failed.remove(&place);
            out.insert(
                place.clone(),
                SurfaceOut {
                    place,
                    owner: id.clone(),
                    name: s.name.clone(),
                    bound,
                    base: Arc::new(base),
                    cut,
                    material: s.material.clone(),
                    said: Arc::new(AtomicBool::new(false)),
                },
            );
        }
    }
    (out, failed)
}

/// A kind mount's plan point and world face (3.1): `on = { ref = owner, t, offset }`, its face
/// added to the bearing of the travel of the segment that holds `t`.
pub(crate) fn mount_point(geo: &Geo, m: &KindMount, source: &str) -> Option<(P, Option<i64>)> {
    let line = match geo {
        Geo::Path { line } => line,
        Geo::Region {
            line: Some(line), ..
        } => line,
        _ => return None,
    };
    let at = via_geom::lineref::interpolate(line, source, m.t, m.offset).ok()?;
    let face = m.face.map(|f| (at.bearing + f).rem_euclid(360_000));
    Some((at.point, face))
}

/// The place a name gives a meeting (3.3): its default mount.
#[derive(Clone, Debug, Default)]
pub(crate) struct Side {
    pub p: Option<P>,
    pub z: Option<f64>,
    pub face: Option<i64>,
    pub connector: Option<String>,
    /// A kind mount or an asset mount: what `surface.unmet` reads.
    pub mount: bool,
}

/// The surface a path's own end lies on (3.3): its kind's one surface, else the terrain.
pub(crate) fn own_surface<'s>(
    d: &Decls,
    surfaces: &'s BTreeMap<String, SurfaceOut>,
    path: &str,
) -> Result<Option<&'s SurfaceOut>, Vec<String>> {
    let Some(k) = d
        .features
        .get(path)
        .and_then(|f| f.kind.as_ref())
        .and_then(|k| d.kinds.get(k))
    else {
        return Ok(None);
    };
    match k.surfaces.len() {
        0 => Ok(None),
        1 => Ok(k
            .surfaces
            .keys()
            .next()
            .and_then(|n| surfaces.get(&format!("{path}.{n}")))),
        _ => Err(k.surfaces.keys().cloned().collect()),
    }
}

/// The connector of a path's ends: its kind's (3.4).
pub(crate) fn path_connector(d: &Decls, path: &str) -> Option<String> {
    d.features
        .get(path)
        .and_then(|f| f.kind.as_ref())
        .and_then(|k| d.kinds.get(k))
        .and_then(|k| k.connector.clone())
}

/// Whether `name` is `<feature>.<surface>`, a kind surface named as a place (3.3, M5).
pub(crate) fn is_kind_surface(d: &Decls, name: &str) -> bool {
    let Some((owner, part)) = name.split_once('.') else {
        return false;
    };
    d.features
        .get(owner)
        .and_then(|f| f.kind.as_ref())
        .and_then(|k| d.kinds.get(k))
        .is_some_and(|k| k.surfaces.contains_key(part))
}

/// The kind mount `name` names, with its owner.
pub(crate) fn kind_mount<'d>(d: &'d Decls, name: &str) -> Option<(&'d str, &'d KindMount)> {
    let (owner, part) = name.split_once('.')?;
    let f = d.features.get(owner)?;
    let k = d.kinds.get(f.kind.as_ref()?)?;
    let (h, _) = d.features.get_key_value(owner)?;
    Some((h.as_str(), k.mounts.get(part)?))
}

/// Every kind mount of every placed feature: `<owner>.<mount>` to its side.
pub(crate) fn kind_mounts(
    d: &Decls,
    shapes: &BTreeMap<String, Geo>,
    surfaces: &BTreeMap<String, SurfaceOut>,
    terrain: &dyn Fn(P) -> Option<f64>,
    diags: &mut Vec<Diag>,
) -> BTreeMap<String, Side> {
    let mut out = BTreeMap::new();
    for (id, f) in &d.features {
        let Some(k) = f.kind.as_ref().and_then(|k| d.kinds.get(k)) else {
            continue;
        };
        let Some(geo) = shapes.get(id) else { continue };
        for m in k.mounts.values() {
            let name = format!("{id}.{}", m.name);
            let Some((p, face)) = mount_point(geo, m, &name) else {
                continue;
            };
            let z = match &m.surface {
                Some(s) => surfaces
                    .get(&format!("{id}.{s}"))
                    .and_then(|s| s.z_in_build((p.x, p.y), diags).ok().flatten()),
                None => terrain(p),
            };
            out.insert(
                name,
                Side {
                    p: Some(p),
                    z,
                    face,
                    connector: m.connector.clone(),
                    mount: true,
                },
            );
        }
    }
    out
}

/// `turn` (3.4): the unsigned angle between the arrival bearing and the place's face.
pub fn turn(arrival: i64, face: i64) -> i64 {
    let e = (arrival - face).rem_euclid(360_000);
    e.min(360_000 - e)
}

/// The meetings' checks (3.4) and `surface.unmet`, as diagnostics.
pub(crate) fn unmet(
    d: &Decls,
    mounts: &BTreeMap<String, Side>,
    meetings: &[crate::build::Meeting],
    diags: &mut Vec<Diag>,
) {
    // A path end at a place answers it; a `meets` pair answers each side only when both have
    // the same connector (3.4: a partner with no connector does not answer a mount).
    let mut answered: BTreeSet<&str> = BTreeSet::new();
    for m in meetings {
        if !m.pair {
            answered.insert(m.b.as_str());
        } else if m.connector_a.is_some() && m.connector_a == m.connector_b {
            answered.insert(m.a.as_str());
            answered.insert(m.b.as_str());
        }
    }
    for (name, s) in mounts {
        if !s.mount || answered.contains(name.as_str()) {
            continue;
        }
        let Some(c) = &s.connector else { continue };
        diags.push(
            text::diag(
                codes::surface::UNMET,
                place_subject(d, name),
                Site::Header,
                &format!(
                    "the mount `{name}` has connector `{c}` and no path end or paired mount answers it"
                ),
                &format!(
                    "end a path whose kind has connector `{c}` here: `ends_at = \"{name}\"`"
                ),
            )
            .with("connector", DValue::text(c.clone()))
            .with("place", DValue::text(name.clone())),
        );
    }
}

/// A place name's subject (`diagnostics.md` 1.1): the entity its first part names.
pub(crate) fn place_subject(d: &Decls, name: &str) -> Subject {
    let head = name.split('.').next().unwrap_or(name);
    let top = if d.features.contains_key(head) {
        "feature"
    } else if d.instances.contains_key(head) {
        "instance"
    } else if d.pins.contains_key(head) {
        "pin"
    } else {
        "feature"
    };
    Subject::entity(top, head)
}

/// What grounding on kind surfaces found (`surfaces.md` 4.1).
#[derive(Debug, Default)]
pub(crate) struct Grounded {
    /// The names not placed (`surface.outside`, `surface.which`, the faults of a `surface`).
    pub gone: BTreeSet<String>,
    /// Each placed hand instance that stands on a kind surface, to that surface's place name.
    pub stands: BTreeMap<String, String>,
}

/// The kind surfaces a feature's kind declares, by place name (whether built or not).
fn declared(d: &Decls, feature: &str) -> Vec<String> {
    d.features
        .get(feature)
        .and_then(|f| f.kind.as_ref())
        .and_then(|k| d.kinds.get(k))
        .map(|k| {
            k.surfaces
                .keys()
                .map(|n| format!("{feature}.{n}"))
                .collect()
        })
        .unwrap_or_default()
}

/// What `on = <r>` stands on, from the declarations (4.1): a kind mount's surface, the one
/// surface of a feature, or, for a mount of an instance (3.2), the surface that instance stands
/// on. `Err` lists a feature's two or more surfaces (`surface.which`); `Ok(None)` is the terrain.
fn on_target(d: &Decls, r: &str, depth: u8) -> Result<Option<String>, Vec<String>> {
    if let Some((owner, m)) = kind_mount(d, r) {
        return Ok(m.surface.as_ref().map(|sn| format!("{owner}.{sn}")));
    }
    let names = declared(d, r);
    match names.len() {
        0 => {}
        1 => return Ok(names.into_iter().next()),
        _ => return Err(names),
    }
    if let Some((h, _)) = r.split_once('.') {
        if depth < 16 {
            if let Some(i) = d.instances.get(h) {
                return Ok(match (&i.surface, &i.place) {
                    (Some(sn), _) => Some(sn.clone()),
                    (None, crate::decl::Place::On(o)) => {
                        on_target(d, &o.reference, depth + 1).ok().flatten()
                    }
                    _ => None,
                });
            }
        }
    }
    Ok(None)
}

/// Grounds hand instances on kind surfaces (`surfaces.md` 4.1, 4.2): by `surface` (M5), by `on`
/// a mount that lies on one (a kind mount, or an asset mount of an instance on one), or by `on` a
/// feature that owns exactly one. A surface left out by a fault of its own (`failed`) says
/// nothing more: its instances are not placed.
pub(crate) fn ground_on_surfaces(
    d: &Decls,
    pack: &crate::pack::Pack,
    surfaces: &BTreeMap<String, SurfaceOut>,
    failed: &BTreeSet<String>,
    all: &mut [crate::rules::Placement],
    diags: &mut Vec<Diag>,
) -> Grounded {
    let mut g = Grounded::default();
    for p in all.iter_mut() {
        if p.maker.is_some() || p.pin {
            continue;
        }
        let Some(decl) = d.instances.get(&p.name) else {
            continue;
        };
        let subject = Subject::entity("instance", p.name.as_str());
        let key = |k: &str| Site::Key(text::path(&[k]));
        let on = match &decl.place {
            crate::decl::Place::On(o) => Some(o.reference.clone()),
            _ => None,
        };
        let name = if let Some(name) = &decl.surface {
            if on.is_some() {
                diags.push(text::diag(
                    codes::surface::PLACED_TWICE,
                    subject,
                    key("surface"),
                    &format!("`{}` writes both `on` and `surface`", p.name),
                    "drop `surface`, or place it by `at`",
                ));
                g.gone.insert(p.name.clone());
                continue;
            }
            name.clone()
        } else if let Some(r) = &on {
            match on_target(d, r, 0) {
                Ok(Some(n)) => n,
                Ok(None) => continue,
                Err(names) => {
                    diags.push(
                        text::diag(
                            codes::surface::WHICH,
                            subject,
                            key("on"),
                            &format!(
                                "`{}` is placed on `{r}`, which owns {}",
                                p.name,
                                names.join(", ")
                            ),
                            &format!("place it `on` a mount of `{r}`, or by `at` with `surface`"),
                        )
                        .with("surfaces", DValue::texts(names.into_iter())),
                    );
                    g.gone.insert(p.name.clone());
                    continue;
                }
            }
        } else {
            continue;
        };
        if failed.contains(&name) {
            // Its owner's fault was said where the surface is declared (2.7).
            g.gone.insert(p.name.clone());
            continue;
        }
        let Some(s) = surfaces.get(&name) else {
            if decl.surface.is_none() {
                // A kind mount naming a surface its kind lacks: `surface.unknown_surface` was
                // said on the kind.
                g.gone.insert(p.name.clone());
                continue;
            }
            let head = name.split('.').next().unwrap_or(&name);
            let owned = declared(d, head);
            diags.push(
                text::diag(
                    codes::surface::NOT_SURFACE,
                    subject,
                    key("surface"),
                    &format!("`{name}` is not a kind surface"),
                    &if owned.is_empty() {
                        format!("`{head}` owns no surface; name `<feature>.<surface>`")
                    } else {
                        format!("name one of {}", owned.join(", "))
                    },
                )
                .with("surfaces", DValue::texts(owned.into_iter())),
            );
            g.gone.insert(p.name.clone());
            continue;
        };
        if decl.ground.is_some() {
            diags.push(text::diag(
                codes::surface::GROUND_OFF_TERRAIN,
                subject,
                key("ground"),
                &format!(
                    "`{}` stands on `{}`, and `ground` writes the terrain only",
                    p.name, s.place
                ),
                "drop `ground`, or place it on the terrain",
            ));
            g.gone.insert(p.name.clone());
            continue;
        }
        match s.z_in_build((p.origin.x, p.origin.y), diags) {
            Ok(Some(z)) => {
                let sink = pack.assets.get(&p.asset).map_or(0.0, |a| a.sink);
                p.z = z - sink * p.scale;
                g.stands.insert(p.name.clone(), s.place.clone());
            }
            Ok(None) => {
                diags.push(
                    text::diag(
                        codes::surface::OUTSIDE,
                        subject,
                        Site::Header,
                        &format!(
                            "`{}` at [{}, {}] lies outside the bound of `{}`",
                            p.name,
                            via_num::write_length(p.origin.x),
                            via_num::write_length(p.origin.y),
                            s.place
                        ),
                        &format!("move it onto `{}`", s.place),
                    )
                    .with("surface", DValue::text(s.place.clone())),
                );
                g.gone.insert(p.name.clone());
            }
            Err(()) => {
                g.gone.insert(p.name.clone());
            }
        }
    }
    g
}

/// What stands on a thing grounding left unplaced is not placed either (4.1, "A host that was not
/// placed"): `surface.host_missing` on each, naming its host, down the chain.
pub(crate) fn host_missing(
    d: &Decls,
    all: &[crate::rules::Placement],
    gone: &mut BTreeSet<String>,
    diags: &mut Vec<Diag>,
) {
    loop {
        let mut grew = false;
        for p in all {
            if p.maker.is_some() || p.pin || gone.contains(&p.name) {
                continue;
            }
            let Some(crate::decl::Place::On(o)) = d.instances.get(&p.name).map(|i| &i.place) else {
                continue;
            };
            let h = o.reference.split('.').next().unwrap_or(&o.reference);
            if !(d.instances.contains_key(h) && gone.contains(h)) {
                continue;
            }
            diags.push(
                text::diag(
                    codes::surface::HOST_MISSING,
                    Subject::entity("instance", p.name.as_str()),
                    Site::Header,
                    &format!(
                        "`{}` stands on `{h}`, which was not placed; not placed",
                        p.name
                    ),
                    &format!("fix `{h}` first: what stands on it is placed with it"),
                )
                .related(via_diag::Related {
                    subject: Subject::entity("instance", h),
                    site: Site::Header,
                    position: None,
                    note: via_diag::text!("the host, not placed"),
                }),
            );
            gone.insert(p.name.clone());
            grew = true;
        }
        if !grew {
            break;
        }
    }
}
