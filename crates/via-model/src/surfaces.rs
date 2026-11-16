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

    /// The bound as a region (1.2): a region owner's, or a path owner's strip.
    pub fn region(&self) -> Option<&via_geom::Region> {
        match &self.bound {
            Geo::Region { region, .. } => Some(region),
            _ => None,
        }
    }
}

/// `surface.not_surface` (4.1) on `subject`, whose `surface` names `name`; `None` when the
/// name is a kind surface its kind declares (a fault of its own was said where it is declared).
pub(crate) fn not_surface(d: &Decls, name: &str, subject: Subject) -> Option<Diag> {
    if is_kind_surface(d, name) {
        return None;
    }
    let head = name.split('.').next().unwrap_or(name);
    let owned = declared(d, head);
    Some(
        text::diag(
            codes::surface::NOT_SURFACE,
            subject,
            Site::Key(text::path(&["surface"])),
            &format!("`{name}` is not a kind surface"),
            &if owned.is_empty() {
                format!("`{head}` owns no surface; name `<feature>.<surface>`")
            } else {
                format!("name one of {}", owned.join(", "))
            },
        )
        .with("surfaces", DValue::texts(owned)),
    )
}

/// The evaluator of formula `f` for feature `owner` (a kind surface's key). `None` when the
/// formula failed its own check (said there) or does not bind, which is said here: never a
/// silent skip (2.7).
fn evaluator(
    d: &Decls,
    f: via_expr::FormulaId,
    owner: &str,
    geo: &Geo,
    diags: &mut Vec<Diag>,
) -> Option<SurfaceEval> {
    let program = d.formulas.programs.get(&f)?.clone();
    let binding = match via_expr::bind(&program, &crate::decl::NoSelect) {
        Ok(b) => b,
        Err(e) => {
            diags.push(d.formulas.diag(&e));
            return None;
        }
    };
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
                    let Some(e) = evaluator(d, bf, id, geo, diags) else {
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
            let Some(base) = s.base.and_then(|b| evaluator(d, b, id, geo, diags)) else {
                continue;
            };
            let cut = match s.cut.and_then(|c| evaluator(d, c, id, geo, diags)) {
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

/// How a placement says what it stands on (4.1): its subject, `surface`, the reference of its
/// `on`, and whether it writes `ground`.
struct Stance {
    subject: Subject,
    /// The key the subject's diagnostics are said once under.
    key: String,
    surface: Option<String>,
    on: Option<String>,
    ground: bool,
}

/// The stance of a placement: a hand instance and its set items by the instance's declaration, a
/// run member and its items by the run's `on`, a scattered instance by its scatter's `surface`.
fn stance(d: &Decls, p: &crate::rules::Placement) -> Option<Stance> {
    if p.pin {
        return None;
    }
    let on_ref = |pl: &crate::decl::Place| match pl {
        crate::decl::Place::On(o) => Some(o.reference.clone()),
        _ => None,
    };
    match p.maker.as_deref() {
        None => {
            let head = p.name.split('.').next().unwrap_or(&p.name);
            let decl = d.instances.get(head)?;
            Some(Stance {
                subject: Subject::entity("instance", head),
                key: format!("instance.{head}"),
                surface: decl.surface.clone(),
                on: on_ref(&decl.place),
                ground: decl.ground.is_some(),
            })
        }
        Some(m) => {
            if let Some(r) = d.runs.get(m) {
                return Some(Stance {
                    subject: Subject::entity("run", m),
                    key: format!("run.{m}"),
                    surface: None,
                    on: on_ref(&r.place),
                    ground: r.ground.is_some(),
                });
            }
            let s = d.scatters.get(m)?;
            Some(Stance {
                subject: Subject::entity("scatter", m),
                key: format!("scatter.{m}"),
                surface: Some(s.surface.clone()?),
                on: None,
                ground: false,
            })
        }
    }
}

/// Grounds placements on kind surfaces (`surfaces.md` 4.1, 4.2): by `surface` (M5), by `on` a
/// mount that lies on one (a kind mount, or an asset mount of an instance on one), or by `on` a
/// feature that owns exactly one; a set's items as their instance, a run's members as their run
/// (M5). A surface left out by a fault of its own (`failed`) says nothing more: what stands on it
/// is not placed.
pub(crate) fn ground_on_surfaces(
    d: &Decls,
    pack: &crate::pack::Pack,
    surfaces: &BTreeMap<String, SurfaceOut>,
    failed: &BTreeSet<String>,
    all: &mut [crate::rules::Placement],
    diags: &mut Vec<Diag>,
) -> Grounded {
    let mut g = Grounded::default();
    // A declaration's own fault is said once, for all its items or members; each is not placed.
    let mut said: BTreeMap<String, bool> = BTreeMap::new();
    for p in all.iter_mut() {
        let Some(st) = stance(d, p) else { continue };
        if said.get(&st.key) == Some(&true) {
            g.gone.insert(p.name.clone());
            continue;
        }
        let key = |k: &str| Site::Key(text::path(&[k]));
        let subject = st.subject.clone();
        let fault = |dg: Diag,
                     said: &mut BTreeMap<String, bool>,
                     g: &mut Grounded,
                     diags: &mut Vec<Diag>| {
            diags.push(dg);
            said.insert(st.key.clone(), true);
            g.gone.insert(p.name.clone());
        };
        let name = if let Some(name) = &st.surface {
            if st.on.is_some() {
                fault(
                    text::diag(
                        codes::surface::PLACED_TWICE,
                        subject,
                        key("surface"),
                        &format!("`{}` writes both `on` and `surface`", st.key),
                        "drop `surface`, or place it by `at`",
                    ),
                    &mut said,
                    &mut g,
                    diags,
                );
                continue;
            }
            name.clone()
        } else if let Some(r) = &st.on {
            match on_target(d, r, 0) {
                Ok(Some(n)) => n,
                Ok(None) => continue,
                Err(names) => {
                    fault(
                        text::diag(
                            codes::surface::WHICH,
                            subject,
                            key("on"),
                            &format!(
                                "`{}` is placed on `{r}`, which owns {}",
                                st.key,
                                names.join(", ")
                            ),
                            &format!("place it `on` a mount of `{r}`, or by `at` with `surface`"),
                        )
                        .with("surfaces", DValue::texts(names.into_iter())),
                        &mut said,
                        &mut g,
                        diags,
                    );
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
            match st
                .surface
                .as_ref()
                .and_then(|n| not_surface(d, n, subject.clone()))
            {
                Some(dg) => fault(dg, &mut said, &mut g, diags),
                // A kind mount naming a surface its kind lacks: `surface.unknown_surface` was
                // said on the kind.
                None => {
                    g.gone.insert(p.name.clone());
                }
            }
            continue;
        };
        if st.ground {
            fault(
                text::diag(
                    codes::surface::GROUND_OFF_TERRAIN,
                    subject,
                    key("ground"),
                    &format!(
                        "`{}` stands on `{}`, and `ground` writes the terrain only",
                        st.key, s.place
                    ),
                    "drop `ground`, or place it on the terrain",
                ),
                &mut said,
                &mut g,
                diags,
            );
            continue;
        }
        said.entry(st.key.clone()).or_insert(false);
        let mut at = s.z_in_build((p.origin.x, p.origin.y), diags);
        // A member a run lays along its host's line (`rules.md` 5) at a span end lies on the
        // strip's end edge, which is inside (1.2); the member's point and the strip's corner
        // are rounded to the lattice apart, so one unit off the bound is still on it (coast:
        // the parapet's first post, 1 mm past the deck's end).
        if matches!(at, Ok(None)) && st.surface.is_none() && st.on.is_some() {
            let (x, y) = (p.origin.x, p.origin.y);
            let near = (-1..=1).any(|dx| (-1..=1).any(|dy| s.inside((x + dx, y + dy))));
            if near {
                at = match s.base.eval(&[(x, y)]) {
                    Ok(v) => Ok(v.first().copied()),
                    Err(e) => {
                        if !s.said.swap(true, Ordering::Relaxed) {
                            diags.push(s.fault(&e));
                        }
                        Err(())
                    }
                };
            }
        }
        match at {
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
                // The formula's fault was said on the owner (2.7); this one is not placed.
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
                    note: via_diag::text!("the owner, not placed"),
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

/// The terrain quads the kind surfaces' `cut` removes from export (`surfaces.md` 1.3,
/// `build.md` 6.5): one byte per quad, `[j * (nx - 1) + i]` for the quad whose south-west post is
/// `(i, j)`, 1 where cut. A quad is cut when its centre (its south-west post plus `floor(r / 2)`
/// on each axis) lies in a surface's bound and its lowest post is below that surface's `z` there
/// plus `cut`. Ours.
pub(crate) fn cut_quads(
    grid: &via_num::Grid,
    terrain: &[f32],
    surfaces: &BTreeMap<String, SurfaceOut>,
    diags: &mut Vec<Diag>,
) -> Vec<u8> {
    let (nx, ny) = (grid.nx as usize, grid.ny as usize);
    let (qx, qy) = (nx.saturating_sub(1), ny.saturating_sub(1));
    let mut out = vec![0u8; qx * qy];
    if terrain.len() != nx * ny {
        return out;
    }
    let [x0, y0, ..] = grid.extent;
    let r = grid.resolution;
    let half = r.div_euclid(2);
    for s in surfaces.values() {
        let Some(cut) = s.cut else { continue };
        let (lo, hi) = s.bound.bbox();
        // The quads whose centre can lie in the bound's box.
        let i0 = ((lo.x - half - x0).div_euclid(r)).max(0) as usize;
        let j0 = ((lo.y - half - y0).div_euclid(r)).max(0) as usize;
        let i1 = (((hi.x - half - x0).div_euclid(r)).max(-1) + 1).min(qx as i64) as usize;
        let j1 = (((hi.y - half - y0).div_euclid(r)).max(-1) + 1).min(qy as i64) as usize;
        for j in j0..j1 {
            for i in i0..i1 {
                let c = (x0 + i as i64 * r + half, y0 + j as i64 * r + half);
                let z = match s.z_in_build(c, diags) {
                    Ok(Some(z)) => z,
                    Ok(None) => continue,
                    // `formula.not_finite` was said on the owner (2.7); the build fails.
                    Err(()) => return out,
                };
                let post = |a: usize, b: usize| f64::from(terrain[b * nx + a]);
                let low = [
                    post(i, j),
                    post(i + 1, j),
                    post(i, j + 1),
                    post(i + 1, j + 1),
                ]
                .into_iter()
                .fold(f64::INFINITY, via_num::min);
                if low < z + cut {
                    out[j * qx + i] = 1;
                }
            }
        }
    }
    out
}

/// A kind surface's mesh (`build.md` 10), before it is written: vertices in integer millimetres,
/// world frame, `z` rounded to the lattice; triangles counter-clockwise seen from above.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshOut {
    /// `<owner>.<surface>`.
    pub place: String,
    pub owner: String,
    pub surface: String,
    pub material: Option<String>,
    /// The south-west corner of the mesh's box, on the lattice.
    pub origin: (i64, i64),
    pub vertices: Vec<[i64; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

/// Every kind surface's mesh (`build.md` 10): the posts inside the bound, every vertex of the
/// bound's outline and every crossing of it with a grid line. Each grid cell is clipped to the
/// bound and each piece triangulated by the kernel's triangulation (`geometry.md` 3.10), so the
/// grid lines and the outline are constraints and no point is added. Ours.
pub(crate) fn meshes(
    grid: &via_num::Grid,
    surfaces: &BTreeMap<String, SurfaceOut>,
    pack: &crate::pack::Pack,
    diags: &mut Vec<Diag>,
) -> Vec<MeshOut> {
    use via_geom::overlay::{boolean, BoolOp};
    let mut out = Vec::new();
    let [x0, y0, x1, y1] = grid.extent;
    let r = grid.resolution;
    'surface: for s in surfaces.values() {
        if let Some(m) = &s.material {
            if !pack.materials.contains_key(m) {
                diags.push(
                    text::diag(
                        codes::pack::UNKNOWN_MATERIAL,
                        Subject::entity("feature", s.owner.as_str()),
                        Site::Header,
                        &format!(
                            "the surface `{}` names the material `{m}`, which the pack lacks",
                            s.place
                        ),
                        "name a material of the pack, or drop `material`",
                    )
                    .with("material", DValue::text(m.clone())),
                );
                continue;
            }
        }
        let Some(region) = s.region() else { continue };
        let Some((lo, hi)) = region.bbox() else {
            continue;
        };
        let (lo, hi) = (
            P::new(lo.x.max(x0), lo.y.max(y0)),
            P::new(hi.x.min(x1), hi.y.min(y1)),
        );
        if lo.x >= hi.x || lo.y >= hi.y {
            continue;
        }
        let edges: Vec<(P, P)> = region
            .rings()
            .iter()
            .flat_map(|ring| (0..ring.len()).map(move |k| (ring[k], ring[(k + 1) % ring.len()])))
            .collect();
        let i0 = (lo.x - x0).div_euclid(r);
        let j0 = (lo.y - y0).div_euclid(r);
        let i1 = (hi.x - x0 + r - 1).div_euclid(r);
        let j1 = (hi.y - y0 + r - 1).div_euclid(r);
        let mut index: BTreeMap<(i64, i64), u32> = BTreeMap::new();
        let mut vertices: Vec<[i64; 3]> = Vec::new();
        let mut triangles: Vec<[u32; 3]> = Vec::new();
        for j in j0..j1 {
            for i in i0..i1 {
                let (cx0, cy0) = (x0 + i * r, y0 + j * r);
                let (cx1, cy1) = ((cx0 + r).min(x1), (cy0 + r).min(y1));
                let square = vec![
                    P::new(cx0, cy0),
                    P::new(cx1, cy0),
                    P::new(cx1, cy1),
                    P::new(cx0, cy1),
                ];
                let touched = edges.iter().any(|(a, b)| {
                    a.x.max(b.x) >= cx0
                        && a.x.min(b.x) <= cx1
                        && a.y.max(b.y) >= cy0
                        && a.y.min(b.y) <= cy1
                });
                let tris: Vec<[P; 3]> = if !touched {
                    let c = P::new(cx0 + (cx1 - cx0) / 2, cy0 + (cy1 - cy0) / 2);
                    if !s.inside((c.x, c.y)) {
                        continue;
                    }
                    vec![
                        [square[0], square[1], square[2]],
                        [square[0], square[2], square[3]],
                    ]
                } else {
                    let cell = via_geom::Region::from_ring(square);
                    let Ok(piece) = boolean(BoolOp::Intersection, &[cell, region.clone()]) else {
                        continue;
                    };
                    if piece.is_empty() {
                        continue;
                    }
                    match via_geom::triangulate::triangulate(&piece) {
                        Ok(t) => t,
                        Err(_) => continue,
                    }
                };
                for t in tris {
                    let mut k = [0u32; 3];
                    for (n, q) in t.iter().enumerate() {
                        if let Some(&v) = index.get(&(q.x, q.y)) {
                            k[n] = v;
                            continue;
                        }
                        let z = match s.z_in_build((q.x, q.y), diags) {
                            Ok(Some(z)) => z,
                            // The bound's own vertices are in it (edges in); a point the
                            // overlay put a hair outside takes the formula's value there.
                            Ok(None) => match s.base.eval(&[(q.x, q.y)]) {
                                Ok(v) => v[0],
                                Err(e) => {
                                    if !s.said.swap(true, Ordering::Relaxed) {
                                        diags.push(s.fault(&e));
                                    }
                                    continue 'surface;
                                }
                            },
                            // `z_in_build` has said the formula's fault (2.7).
                            Err(()) => continue 'surface,
                        };
                        // A formula's value is finite (2.7 refuses the rest), so only a height
                        // past the lattice's range (2^53 mm) fails here: said as the formula's.
                        let Ok(zmm) = via_num::round_to_lattice(z * 1000.0) else {
                            diags.push(
                                text::diag(
                                    codes::num::NOT_FINITE_RESULT,
                                    Subject::entity("feature", s.owner.as_str()),
                                    Site::Header,
                                    &format!("the surface `{}` stands out of range", s.place),
                                    "keep `base` within the lattice's range",
                                )
                                .with("surface", DValue::text(s.place.clone())),
                            );
                            continue 'surface;
                        };
                        let v = vertices.len() as u32;
                        index.insert((q.x, q.y), v);
                        vertices.push([q.x, q.y, zmm]);
                        k[n] = v;
                    }
                    triangles.push(k);
                }
            }
        }
        if triangles.is_empty() {
            continue;
        }
        out.push(MeshOut {
            place: s.place.clone(),
            owner: s.owner.clone(),
            surface: s.name.clone(),
            material: s.material.clone(),
            origin: (lo.x, lo.y),
            vertices,
            triangles,
        });
    }
    out
}

/// The kind surface each point feature with `surface` and each named place stands on
/// (`surfaces.md` 4.1, 3.3), for the `z` that `features.json` writes: a point feature by its
/// `surface`, a kind surface named as a place by itself, a kind mount by its surface. Faults of a
/// point's `surface`: `surface.placed_twice` with `on`, `surface.not_surface`, `surface.outside`.
pub(crate) fn place_surfaces(
    d: &Decls,
    placed: &crate::place::Placed,
    surfaces: &BTreeMap<String, SurfaceOut>,
    diags: &mut Vec<Diag>,
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (id, f) in &d.features {
        let Some(sn) = &f.surface else { continue };
        let subject = Subject::entity("feature", id.as_str());
        let Some(Geo::Point { p, .. }) = placed.shapes.get(id) else {
            continue;
        };
        if let crate::decl::ShapeDecl::Point(crate::decl::Place::On(_)) = &f.shape {
            diags.push(text::diag(
                codes::surface::PLACED_TWICE,
                subject,
                Site::Key(text::path(&["surface"])),
                &format!("`{id}` writes both `on` and `surface`"),
                "drop `surface`, or place it by `at`",
            ));
            continue;
        }
        let Some(s) = surfaces.get(sn) else {
            if let Some(dg) = not_surface(d, sn, subject) {
                diags.push(dg);
            }
            continue;
        };
        if !s.inside((p.x, p.y)) {
            diags.push(
                text::diag(
                    codes::surface::OUTSIDE,
                    subject,
                    Site::Header,
                    &format!(
                        "`{id}` at [{}, {}] lies outside the bound of `{}`",
                        via_num::write_length(p.x),
                        via_num::write_length(p.y),
                        s.place
                    ),
                    &format!("move it onto `{}`", s.place),
                )
                .with("surface", DValue::text(s.place.clone())),
            );
            continue;
        }
        out.insert(id.clone(), s.place.clone());
    }
    for (place, _) in placed.names.values() {
        let on = if surfaces.contains_key(place) {
            Some(place.clone())
        } else if let Some(s) = out.get(place) {
            Some(s.clone())
        } else {
            kind_mount(d, place)
                .and_then(|(owner, m)| m.surface.as_ref().map(|sn| format!("{owner}.{sn}")))
        };
        if let Some(s) = on.filter(|s| surfaces.contains_key(s)) {
            out.insert(place.clone(), s);
        }
    }
    out
}

/// What a thing stands on and how high it reaches above it (`rules.md` 8.1, M5): the place name
/// of the kind surface its declaration names (`""` for the terrain), and its span above that
/// surface in metres, `-sink` to `-sink` plus its bounds' height, all scaled. Plan only: read from
/// the declarations and the asset's facts, never from a field (4.3 step 1).
pub(crate) fn level(
    d: &Decls,
    pack: &crate::pack::Pack,
    p: &crate::rules::Placement,
) -> (String, f64, f64) {
    let surface = match stance(d, p) {
        Some(st) => match (&st.surface, &st.on) {
            (Some(s), _) => s.clone(),
            (None, Some(r)) => on_target(d, r, 0).ok().flatten().unwrap_or_default(),
            _ => String::new(),
        },
        None => String::new(),
    };
    let (lo, hi) = span(pack, &p.asset, p.scale);
    (surface, lo, hi)
}

/// An asset's height span above the surface it stands on, metres (`rules.md` 8.1).
pub(crate) fn span(pack: &crate::pack::Pack, asset: &str, scale: f64) -> (f64, f64) {
    match pack.assets.get(asset) {
        Some(a) => (
            (via_num::metres(a.bounds[2]) - a.sink) * scale,
            (via_num::metres(a.bounds[5]) - a.sink) * scale,
        ),
        None => (0.0, 0.0),
    }
}

/// The kind surfaces the level test reads (`rules.md` 8.1): their bounds and `cut`, plan only
/// (made from the owners' shapes, 4.3 step 3).
pub(crate) struct Levels {
    kinds: BTreeMap<String, SurfaceOut>,
}

impl Levels {
    pub(crate) fn new(d: &Decls, shapes: &BTreeMap<String, Geo>) -> Levels {
        let any = d.kinds.values().any(|k| !k.surfaces.is_empty());
        Levels {
            kinds: if any {
                kind_surfaces(d, shapes, &mut Vec::new()).0
            } else {
                BTreeMap::new()
            },
        }
    }

    /// Whether two things can clash (`rules.md` 8.1): one level, and spans whose insides meet
    /// (a span of no height meets one it touches). The terrain is one level with a kind surface
    /// that has `cut` (a ground floor let into it) or whose bound the terrain thing's footprint
    /// meets; two different kind surfaces never are.
    pub(crate) fn clash(
        &self,
        a: &(String, f64, f64),
        fa: &via_geom::Region,
        b: &(String, f64, f64),
        fb: &via_geom::Region,
    ) -> bool {
        if a.0 != b.0 {
            let (ground, on) = match (a.0.is_empty(), b.0.is_empty()) {
                (true, false) => (fa, &b.0),
                (false, true) => (fb, &a.0),
                _ => return false,
            };
            let Some(s) = self.kinds.get(on) else {
                return false;
            };
            let meets = s
                .region()
                .is_some_and(|r| crate::rules::overlaps(ground, r));
            if s.cut.is_none() && !meets {
                return false;
            }
        }
        let (lo, hi) = (via_num::max(a.1, b.1), via_num::min(a.2, b.2));
        lo < hi || ((a.1 == a.2 || b.1 == b.2) && lo <= hi)
    }
}
