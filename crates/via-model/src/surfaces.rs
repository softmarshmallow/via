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

/// How far past a quad's half diagonal a narrow cutting strip's band reaches (`composition.md`
/// 6.1, "plus 6 mm"): the arc's chord tolerance (`geometry.md` 3.4) and a millimetre of rounding.
const BAND_PAST_MM: i64 = 6;
const _: () = assert!(BAND_PAST_MM == via_geom::offset::ARC_TOLERANCE_MM as i64 + 1);

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
    /// For a surface brought from an import (`composition.md` 6.1): each import's frame, the
    /// outermost first, that takes a point of the build into the formula's own world, and the
    /// lifts added on the way out. Empty and 0 for a world's own surface.
    frames: Vec<via_num::Frame>,
    lift: f64,
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
        // A brought surface reads its formula at the point in its own world's frame, rounded to
        // the lattice through each frame in turn (`composition.md` 6.1), and adds the lifts.
        let own: Vec<(i64, i64)>;
        let points = if self.frames.is_empty() {
            points
        } else {
            own = points
                .iter()
                .map(|&p| {
                    self.frames.iter().fold(p, |q, f| {
                        let (x, y) = crate::import::to_import(f, q);
                        (x.round_ties_even() as i64, y.round_ties_even() as i64)
                    })
                })
                .collect();
            &own
        };
        let mut v = via_expr::evaluate(
            &self.program,
            &self.binding,
            &OwnerReads { e: self },
            &setup,
            &Domain::Points(points),
        )
        .map_err(|f| Box::new(f.error))?;
        if self.lift != 0.0 {
            for z in &mut v {
                *z += self.lift;
            }
        }
        Ok(v)
    }

    /// This surface as an import placed by `frame` and lifted by `lift` brings it (6.1).
    fn placed(&self, frame: via_num::Frame, lift: f64) -> SurfaceEval {
        let mut e = self.clone();
        e.frames.insert(0, frame);
        e.lift += lift;
        e
    }
}

/// One kind surface of one owner (1).
#[derive(Clone, Debug)]
pub struct SurfaceOut {
    /// `<owner>.<name>`.
    pub place: String,
    pub owner: String,
    pub name: String,
    /// The owner's kind: a skirt covers the cut cells of its own kind's surfaces only
    /// (`build.md` 10, "Skirt").
    pub kind: String,
    /// The bound, edges in (1.2).
    pub bound: Geo,
    pub base: Arc<SurfaceEval>,
    /// `cut` in metres, when written (1.3).
    pub cut: Option<f64>,
    /// A path owner's cutting strip grown by the reach of a cut quad past it (`build.md` 10,
    /// "Skirt"): the mesh covers the cut cells in it, so a cut quad is not left open past the
    /// bound. `None` for a region owner or a surface with no `cut`.
    pub skirt: Option<via_geom::Region>,
    /// A path owner's line and the strip's half width in millimetres, for a surface with `cut`:
    /// a world that brings it strokes its skirt again by its own posts (`composition.md` 6.1).
    pub(crate) strip: Option<(Arc<Vec<P>>, i64)>,
    /// A brought cutting strip narrower than half a quad's diagonal of the coarser world that
    /// brings it, widened to it: the quads whose centre it holds are cut, the formula read there
    /// past the bound (`composition.md` 6.1). `None`: the bound decides.
    pub(crate) cut_band: Option<via_geom::Region>,
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

    /// This surface of import `id`, placed by `frame` and lifted by `lift` (`composition.md`
    /// 6.1): its place and owner scoped (`<id>.<place>`), its bound and skirt taken through the
    /// frame point by point (every piece and hole), its `base` read in its own frame plus the
    /// lift. A skirt is stroked again from the brought line by `resolution`, the posts of the
    /// world that brings it, whose cut quads it covers (`build.md` 10, "Skirt"): the import's
    /// own skirt reached past the bound only as far as the import's finer quads (`coast` at
    /// 0.5 m in `atlas` at 2 m: the cliff's foot open, M6 close). `None` when a point of it
    /// leaves the lattice.
    ///
    /// On posts coarser than the import's (`own`, the import's resolution), a cutting strip
    /// narrower than half a quad's diagonal holds the centres of only some of the quads it crosses:
    /// a staircase of single cut quads whose uncut neighbours straddle the step and leave the
    /// hole's side open to a walker. There the strip cuts as if that wide (`cut_band`), round at
    /// its ends, and the skirt reaches a quad's half diagonal past the band.
    pub(crate) fn brought(
        &self,
        id: &str,
        frame: via_num::Frame,
        lift: f64,
        resolution: Option<i64>,
        own: Option<i64>,
    ) -> Option<SurfaceOut> {
        let pt = |q: &P| frame.to_world((q.x, q.y)).ok().map(|(x, y)| P::new(x, y));
        let ring = |r: &[P]| r.iter().map(pt).collect::<Option<Vec<P>>>();
        let region = |r: &via_geom::Region| -> Option<via_geom::Region> {
            let mut pieces = Vec::with_capacity(r.pieces.len());
            for pc in &r.pieces {
                pieces.push(via_geom::shape::Piece {
                    shell: ring(&pc.shell)?,
                    holes: pc.holes.iter().map(|h| ring(h)).collect::<Option<_>>()?,
                });
            }
            Some(via_geom::Region { pieces })
        };
        let line = |l: &via_geom::lineref::Line| {
            Some(via_geom::lineref::Line::new(ring(&l.pts)?, l.closed))
        };
        let bound = match &self.bound {
            Geo::Point { p, facing } => Geo::Point {
                p: pt(p)?,
                facing: facing.map(|f| frame.facing_to_world(f)),
            },
            Geo::Path { line: l } => Geo::Path { line: line(l)? },
            Geo::Region { region: r, line: l } => Geo::Region {
                region: region(r)?,
                line: match l {
                    Some(l) => Some(line(l)?),
                    None => None,
                },
            },
        };
        let strip = match &self.strip {
            Some((pts, mm)) => Some((Arc::new(ring(pts)?), *mm)),
            None => None,
        };
        let mut cut_band = None;
        let skirt = match (&strip, resolution, &self.skirt) {
            (Some((pts, mm)), Some(r), _) => {
                let reach = skirt_reach(r);
                let mut w = *mm;
                // Round at its ends too (M7 close): a quad that holds the line's end has its centre
                // within its half diagonal of it, past the butt end where two owners end at a jamb.
                // The band's arcs are chords inside the true arc (`geometry.md` 3.4, 5 mm) on
                // posts rounded to the millimetre: it reaches that much further, so a quad whose
                // centre lies exactly its half diagonal from the line's end (the line ending on
                // its corner, as `coast`'s west face at the cave's jamb) is held.
                let band = reach + BAND_PAST_MM;
                let mut cap = Cap::Butt;
                if own.is_some_and(|o| o < r) && w < band {
                    w = band;
                    cap = Cap::Round;
                    cut_band =
                        via_geom::offset::stroke(&self.owner, pts, w, w, Join::Round, cap).ok();
                }
                let w = w + reach;
                via_geom::offset::stroke(&self.owner, pts, w, w, Join::Round, cap).ok()
            }
            (_, _, Some(r)) => Some(region(r)?),
            _ => None,
        };
        Some(SurfaceOut {
            place: format!("{id}.{}", self.place),
            owner: format!("{id}.{}", self.owner),
            name: self.name.clone(),
            kind: self.kind.clone(),
            bound,
            base: Arc::new(self.base.placed(frame, lift)),
            cut: self.cut,
            skirt,
            strip,
            cut_band,
            material: self.material.clone(),
            said: Arc::new(AtomicBool::new(false)),
        })
    }
}

/// How far a cut quad of side `resolution` (millimetres) reaches past the strip that holds its
/// centre: half its diagonal, rounded up (`build.md` 10, "Skirt").
fn skirt_reach(resolution: i64) -> i64 {
    (resolution * 7072 + 9999) / 10000
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
        frames: Vec::new(),
        lift: 0.0,
    })
}

/// `surface.fold` (1.2): the strip of `pts` at `half` mm overlaps itself, found as `geometry.md`
/// 3.5 finds `geom.follow_broken`: the curve at `half` on either side is not one piece. Gives
/// the position and the largest bound that holds on both sides.
fn fold(id: &str, pts: &[P], half: i64, subject: &Subject) -> Option<Diag> {
    use via_geom::follow::{follow, Reference};
    let reference = Reference::Path(pts.to_vec());
    let mut broken: Option<(P, i64)> = None;
    for side in [half, -half] {
        let Err(g) = follow(id, &reference, 0.0, 1.0, side) else {
            continue;
        };
        if g.code != via_geom::Code::FollowBroken {
            continue;
        }
        let widest = match g.data.get("widest") {
            Some(via_diag::Value::Length(w)) => w.abs(),
            _ => 0,
        };
        let at = g.position.unwrap_or(pts[0]);
        broken = Some(match broken {
            Some((p, w)) if w <= widest => (p, w),
            _ => (at, widest),
        });
    }
    let (at, widest) = broken?;
    Some(
        text::diag(
            codes::surface::FOLD,
            subject.clone(),
            Site::Header,
            &format!(
                "the strip of `{id}` at a bound of {} m overlaps itself near [{}, {}]; the largest bound that holds is {} m",
                via_num::write_length(half),
                via_num::write_length(at.x),
                via_num::write_length(at.y),
                via_num::write_length(widest)
            ),
            "write a smaller `bound`, or ease the bend",
        )
        .at(text::pos(at.x, at.y))
        .with("bound", DValue::Length(half))
        .with("widest", DValue::Length(widest)),
    )
}

/// A path owner's strip clipped to the extent (1.2): `surface.clipped` (info) with the share
/// of the strip's area lost, exact integer areas and one f64 division.
fn clipped(
    d: &Decls,
    id: &str,
    region: via_geom::Region,
    subject: &Subject,
    diags: &mut Vec<Diag>,
) -> via_geom::Region {
    use via_geom::overlay::{boolean, BoolOp};
    let Some([x0, y0, x1, y1]) = d.grid.map(|g| g.extent) else {
        return region;
    };
    let Some((lo, hi)) = region.bbox() else {
        return region;
    };
    if lo.x >= x0 && lo.y >= y0 && hi.x <= x1 && hi.y <= y1 {
        return region;
    }
    let extent = via_geom::Region::from_ring(vec![
        P::new(x0, y0),
        P::new(x1, y0),
        P::new(x1, y1),
        P::new(x0, y1),
    ]);
    let Ok(inside) = boolean(BoolOp::Intersection, &[region.clone(), extent]) else {
        return region;
    };
    let (a, ai) = (region.twice_area(), inside.twice_area());
    if a <= 0 || ai >= a {
        return region;
    }
    let share = (a - ai) as f64 / a as f64;
    let mut dg = text::diag(
        codes::surface::CLIPPED,
        subject.clone(),
        Site::Header,
        &format!(
            "the bound of `{id}` reaches beyond the extent; {} of it is clipped",
            text::dbl(share)
        ),
        "nothing if it may reach past the edge; otherwise move it inside, or widen the extent",
    );
    if let Some(v) = DValue::double(share) {
        dg = dg.with("share", v);
    }
    diags.push(dg);
    inside
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
            let mut skirt: Option<via_geom::Region> = None;
            let mut strip: Option<(Arc<Vec<P>>, i64)> = None;
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
                    if let Some(dg) = fold(id, &line.pts, mm, &subject) {
                        diags.push(dg);
                        continue;
                    }
                    match via_geom::offset::stroke(id, &line.pts, mm, mm, Join::Round, Cap::Butt) {
                        Ok(region) => {
                            // A cut quad has its centre in the strip, so it reaches at most half
                            // a quad's diagonal past it: the skirt is the strip grown by that.
                            if s.cut.is_some() {
                                strip = Some((Arc::new(line.pts.clone()), mm));
                                if let Some(g) = d.grid {
                                    let reach = skirt_reach(g.resolution);
                                    skirt = via_geom::offset::stroke(
                                        id,
                                        &line.pts,
                                        mm + reach,
                                        mm + reach,
                                        Join::Round,
                                        Cap::Butt,
                                    )
                                    .ok()
                                    .map(|r| clipped(d, id, r, &subject, &mut Vec::new()));
                                }
                            }
                            Geo::Region {
                                region: clipped(d, id, region, &subject, diags),
                                line: None,
                            }
                        }
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
                    kind: f.kind.clone().unwrap_or_default(),
                    bound,
                    base: Arc::new(base),
                    skirt: if cut.is_some() { skirt } else { None },
                    strip: if cut.is_some() { strip } else { None },
                    cut_band: None,
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
    /// For an asset mount, the instance it is a mount of (3.2): what a joint reads (3.3, R76).
    pub owner: Option<String>,
    /// For an asset mount, its `rise` in this instance, millimetres (3.2, R75): already in `z`.
    pub rise: i64,
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
                    ..Default::default()
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

/// How far a mount that nothing answers looks for the mount it missed (3.4): 2 m in plan.
pub(crate) const NEAR_JOINT: i64 = 2_000;

/// The joints of 3.3 (R76): which instances' mounts answer each other, and which share a point
/// and a height with more than one other.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Joints {
    /// Each joint, its two mounts ordered bytewise.
    pub pairs: Vec<(String, String)>,
    /// Each set of three or more mounts at one point and one height, sorted.
    pub crowded: Vec<Vec<String>>,
}

/// The joints among `mounts` (3.3). The mounts that take part are asset mounts with a connector,
/// a plan point and a height. Those at one plan point are taken together. First by height, to
/// the millimetre, and connector: exactly two of one height and one connector, of two instances,
/// are a joint; three or more that are not all one instance's are joined to nothing and
/// reported. Then what is left at the point, whatever its heights and connectors: exactly two,
/// of two instances, are a joint for `surface.gap` or `surface.connector` to report; any other
/// number is left unjoined.
pub(crate) fn joints(mounts: &BTreeMap<String, Side>, round_mm: &dyn Fn(f64) -> i64) -> Joints {
    type Member<'a> = (&'a str, &'a str, i64, &'a str);
    let mut at: BTreeMap<(i64, i64), Vec<Member<'_>>> = BTreeMap::new();
    for (name, s) in mounts {
        let (Some(owner), Some(c), Some(p), Some(z)) = (&s.owner, &s.connector, s.p, s.z) else {
            continue;
        };
        at.entry((p.x, p.y)).or_default().push((
            name.as_str(),
            owner.as_str(),
            round_mm(z),
            c.as_str(),
        ));
    }
    let two = |a: &Member<'_>, b: &Member<'_>| -> (String, String) {
        if a.0.as_bytes() < b.0.as_bytes() {
            (a.0.to_string(), b.0.to_string())
        } else {
            (b.0.to_string(), a.0.to_string())
        }
    };
    let mut out = Joints::default();
    for group in at.values() {
        let mut alike: BTreeMap<(i64, &str), Vec<&Member<'_>>> = BTreeMap::new();
        for m in group {
            alike.entry((m.2, m.3)).or_default().push(m);
        }
        let mut left: Vec<&Member<'_>> = Vec::new();
        for level in alike.values() {
            let one_owner = level.iter().all(|m| m.1 == level[0].1);
            if level.len() == 2 && !one_owner {
                out.pairs.push(two(level[0], level[1]));
            } else if level.len() >= 3 && !one_owner {
                let mut names: Vec<String> = level.iter().map(|m| m.0.to_string()).collect();
                names.sort();
                out.crowded.push(names);
            } else {
                left.extend(level.iter().copied());
            }
        }
        if left.len() == 2 && left[0].1 != left[1].1 {
            out.pairs.push(two(left[0], left[1]));
        }
    }
    out.pairs.sort();
    out.crowded.sort();
    out
}

/// The asset mounts of other instances, with a connector, at `s`'s own plan point (3.4): what a
/// mount left unjoined at a point stands beside. Sorted by name.
fn beside<'a>(
    name: &str,
    s: &Side,
    mounts: &'a BTreeMap<String, Side>,
) -> Vec<(&'a str, &'a Side)> {
    let (Some(owner), Some(p)) = (s.owner.as_ref(), s.p) else {
        return Vec::new();
    };
    mounts
        .iter()
        .filter(|(other, o)| {
            other.as_str() != name
                && o.mount
                && o.connector.is_some()
                && o.owner.as_ref().is_some_and(|oo| oo != owner)
                && o.p == Some(p)
        })
        .map(|(n, o)| (n.as_str(), o))
        .collect()
}

/// The mount nearest to `name` that could have answered it (3.4): an asset mount of another
/// instance, with the same connector, that nothing answers either, within [`NEAR_JOINT`] in plan.
/// Nearest in plan, then in height, then by name. Returns it with the miss in plan and in
/// height, millimetres.
fn nearest_open<'a>(
    name: &str,
    s: &Side,
    mounts: &'a BTreeMap<String, Side>,
    answered: &BTreeSet<&str>,
    round_mm: &dyn Fn(f64) -> i64,
) -> Option<(&'a str, i64, i64)> {
    let (owner, c, p, z) = (s.owner.as_ref()?, s.connector.as_ref()?, s.p?, s.z?);
    let mut best: Option<(i64, i64, &str)> = None;
    for (other, o) in mounts {
        if other == name || answered.contains(other.as_str()) {
            continue;
        }
        let (Some(oo), Some(oc), Some(q), Some(oz)) = (&o.owner, &o.connector, o.p, o.z) else {
            continue;
        };
        if oo == owner || oc != c {
            continue;
        }
        let d2 = via_num::distance_sq_mm((p.x, p.y), (q.x, q.y));
        if d2 > i128::from(NEAR_JOINT) * i128::from(NEAR_JOINT) {
            continue;
        }
        let plan = via_num::round_to_lattice((d2 as f64).sqrt()).unwrap_or(NEAR_JOINT);
        let up = (round_mm(oz) - round_mm(z)).abs();
        let cand = (plan, up, other.as_str());
        if best.is_none_or(|b| cand < b) {
            best = Some(cand);
        }
    }
    best.map(|(plan, up, other)| (other, plan, up))
}

/// The meetings' checks (3.4) and `surface.unmet`, as diagnostics.
pub(crate) fn unmet(
    d: &Decls,
    mounts: &BTreeMap<String, Side>,
    meetings: &[crate::build::Meeting],
    crowded: &[Vec<String>],
    round_mm: &dyn Fn(f64) -> i64,
    diags: &mut Vec<Diag>,
) {
    // A path end at a place answers it; a `meets` pair or a joint answers each side only when
    // both have the same connector (3.4: a partner with no connector does not answer a mount).
    // A mount among three or more at one point is `surface.joint_many`'s (3.3): said there,
    // not here, and it is no mount another one missed.
    let mut answered: BTreeSet<&str> = crowded.iter().flatten().map(String::as_str).collect();
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
        let mut message = format!(
            "the mount `{name}` has connector `{c}` and no path end or paired mount answers it"
        );
        let mut hint =
            format!("end a path whose kind has connector `{c}` here: `ends_at = \"{name}\"`");
        let near = nearest_open(name, s, mounts, &answered, round_mm);
        if let Some((other, plan, up)) = near {
            // The joint it missed (R76): the nearest mount that could have answered it.
            message.push_str(&format!(
                "; the nearest `{c}` mount of another instance, `{other}`, is {plan} mm away in plan and {up} mm in height"
            ));
            hint = format!(
                "bring `{name}` and `{other}` to one point and one height, facing opposite ways: \
                 they miss by {plan} mm in plan and {up} mm in height. Or {hint}"
            );
        }
        // What it stands beside (3.4): a mount left over at a point where others stand is told
        // which, and what each is, so a third piece or a piece of another connector is named.
        let others = beside(name, s, mounts);
        if !others.is_empty() {
            let list: Vec<String> = others
                .iter()
                .map(|(n, o)| {
                    let z = o.z.map_or_else(String::new, |z| {
                        format!(", at {} m", via_num::write_length(round_mm(z)))
                    });
                    format!("`{n}` (`{}`{z})", o.connector.as_deref().unwrap_or(""))
                })
                .collect();
            message.push_str(&format!("; at its own point: {}", list.join(", ")));
        }
        let mut diag = text::diag(
            codes::surface::UNMET,
            place_subject(d, name),
            Site::Header,
            &message,
            &hint,
        )
        .with("connector", DValue::text(c.clone()))
        .with("place", DValue::text(name.clone()));
        if !others.is_empty() {
            let names: Vec<&str> = others.iter().map(|(n, _)| *n).collect();
            diag = diag.with("beside", DValue::text(names.join(" ")));
        }
        if let Some((other, plan, up)) = near {
            diag = diag
                .with("near", DValue::text(other.to_string()))
                .with("miss", DValue::Int(plan))
                .with("miss_z", DValue::Int(up));
        }
        diags.push(diag);
    }
}

/// A place name's subject (`diagnostics.md` 1.1): the entity its first part names.
pub(crate) fn place_subject(d: &Decls, name: &str) -> Subject {
    let head = name.split('.').next().unwrap_or(name);
    // A run's member (`<run>#r<k>`, `rules.md` 5) is its run's, and a scatter's its scatter's:
    // a member is no entity of the text, so a record named after it had nowhere to be found
    // (`diag.internal`, K9's review: `surface.unmet` of a run of pieces with a connector).
    if !d.features.contains_key(head) && !d.instances.contains_key(head) {
        if let Some(r) = d.run_of(head) {
            return Subject::entity("run", r);
        }
        if let Some((m, _)) = head.split_once('#') {
            if d.scatters.contains_key(m) {
                return Subject::entity("scatter", m);
            }
        }
    }
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
    /// The third number of its `at` and the point it was written at (R65): `surface` is then
    /// what the height chose, `None` where it chose nothing.
    height: Option<(crate::decl::Height, (i64, i64))>,
}

/// The stance of a placement: a hand instance and its set items by the instance's declaration, a
/// run member and its items by the run's `on`, a scattered instance by its scatter's `surface`.
fn stance(d: &Decls, p: &crate::rules::Placement) -> Option<Stance> {
    let at = |h: &Option<crate::decl::Height>, pl: &crate::decl::Place| match (h, pl) {
        (Some(h), crate::decl::Place::At(q)) => Some((h.clone(), *q)),
        _ => None,
    };
    if p.pin {
        // A pin stands on the terrain unless its `at` carries a height (`identity.md` 4.2).
        let pin = d.pins.get(&p.name)?;
        let height = at(&pin.height, &pin.place)?;
        return Some(Stance {
            subject: Subject::entity("pin", p.name.as_str()),
            key: format!("pin.{}", p.name),
            surface: pin.surface.clone(),
            on: None,
            ground: false,
            height: Some(height),
        });
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
                height: at(&decl.height, &decl.place),
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
                    height: None,
                });
            }
            let s = d.scatters.get(m)?;
            Some(Stance {
                subject: Subject::entity("scatter", m),
                key: format!("scatter.{m}"),
                surface: Some(s.surface.clone()?),
                on: None,
                ground: false,
                height: None,
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
        if let (Some((h, at)), None) = (&st.height, &st.surface) {
            if h.among.is_empty() {
                fault(none_at(subject, &st.key, *at), &mut said, &mut g, diags);
            } else {
                // A candidate's formula failed at the point: its fault is said on its owner
                // (2.7), once, and this thing is not placed.
                for s in h.among.iter().filter_map(|n| surfaces.get(n)) {
                    let _ = s.z_in_build(*at, diags);
                }
                said.insert(st.key.clone(), true);
                g.gone.insert(p.name.clone());
            }
            continue;
        }
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
                // Hand instances and set items only (4.2): a step's returned instance by its
                // name, as its `maker` is set only when the world has a hand edit.
                // A pin is checked by `identity.md` 4.2 only (`rules.md` 8).
                if p.maker.is_none() && !p.pin && !d.produced_instance(&p.name) {
                    footprint_on(d, pack, s, p, &subject, diags);
                }
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

/// `relief(surface, [region])` on a kind surface (2.3): `(low, high, outside)`. Sampled, not
/// exact: the formula at the lattice posts inside `R ∩ B`, its vertices, and its edges'
/// crossings with grid lines (rounded to the lattice); `outside` from exact integer areas and one
/// f64 division. `R` wholly outside gives `(0, 0, 1)`; `None` for a region of zero area, or where
/// the formula fails (its fault said on the owner once, 2.7). Its callers pass footprints, which
/// never have zero area, so `surface.empty_region` (2.3, a step's call) is not said here.
pub(crate) fn kind_relief(
    s: &SurfaceOut,
    grid: &via_num::Grid,
    region: &via_geom::Region,
    diags: &mut Vec<Diag>,
) -> Option<(f64, f64, f64)> {
    use via_geom::overlay::{boolean, BoolOp};
    let area = region.twice_area();
    if area <= 0 {
        return None;
    }
    let bound = s.region()?;
    let inter = boolean(BoolOp::Intersection, &[region.clone(), bound.clone()]).unwrap_or_default();
    let ai = inter.twice_area();
    let outside = (area - ai) as f64 / area as f64;
    if inter.is_empty() || ai <= 0 {
        return Some((0.0, 0.0, 1.0));
    }
    let mut pts: Vec<(i64, i64)> = Vec::new();
    if let Some((lo, hi)) = inter.bbox() {
        if let Ok(Some(w)) = via_raster::window::effect_window(grid, [lo.x, lo.y, hi.x, hi.y], 0) {
            for (i, j) in w.posts() {
                let (x, y) = via_raster::window::post_mm(grid, i, j);
                if inter.side(P::new(x, y)) != via_geom::pred::Side::Out {
                    pts.push((x, y));
                }
            }
        }
    }
    let [x0, y0, _, _] = grid.extent;
    let r = grid.resolution;
    let lines = |p: i64, q: i64, o: i64| -> Vec<i64> {
        let (lo, hi) = (p.min(q), p.max(q));
        let mut v = Vec::new();
        let mut g = o + (lo - o).div_euclid(r) * r + r;
        while g < hi {
            v.push(g);
            g += r;
        }
        v
    };
    for ring in inter.rings() {
        for k in 0..ring.len() {
            let (a, b) = (ring[k], ring[(k + 1) % ring.len()]);
            pts.push((a.x, a.y));
            let along = |num: i64, den: i64, from: i64, span: i64| -> i64 {
                from + via_num::round_to_lattice(span as f64 * num as f64 / den as f64).unwrap_or(0)
            };
            if a.x != b.x {
                for gx in lines(a.x, b.x, x0) {
                    pts.push((gx, along(gx - a.x, b.x - a.x, a.y, b.y - a.y)));
                }
            }
            if a.y != b.y {
                for gy in lines(a.y, b.y, y0) {
                    pts.push((along(gy - a.y, b.y - a.y, a.x, b.x - a.x), gy));
                }
            }
        }
    }
    pts.sort_unstable();
    pts.dedup();
    let vals = match s.base.eval(&pts) {
        Ok(v) => v,
        Err(e) => {
            if !s.said.swap(true, Ordering::Relaxed) {
                diags.push(s.fault(&e));
            }
            return None;
        }
    };
    let low = via_num::minimum(&vals).ok()?;
    let high = via_num::maximum(&vals).ok()?;
    Some((low, high, outside))
}

/// After grounding a hand instance on a kind surface (4.2): `relief` over its footprint;
/// `outside > 0` is `surface.off_bound` with the share, and `high - low` past the asset's
/// `max_relief` is `place.relief`.
fn footprint_on(
    d: &Decls,
    pack: &crate::pack::Pack,
    s: &SurfaceOut,
    p: &crate::rules::Placement,
    subject: &Subject,
    diags: &mut Vec<Diag>,
) {
    let Some(grid) = d.grid.as_ref() else { return };
    let Some((low, high, outside)) = kind_relief(s, grid, &p.footprint, diags) else {
        return;
    };
    if outside > 0.0 {
        let mut dg = text::diag(
            codes::surface::OFF_BOUND,
            subject.clone(),
            Site::Header,
            &format!(
                "{} of the footprint of `{}` lies outside the bound of `{}`",
                text::dbl(outside),
                p.name,
                s.place
            ),
            &format!("move it onto `{}`", s.place),
        )
        .at(text::pos(p.origin.x, p.origin.y))
        .with("surface", DValue::text(s.place.clone()));
        if let Some(v) = DValue::double(outside) {
            dg = dg.with("share", v);
        }
        diags.push(dg);
    }
    if let Some(limit) = pack.assets.get(&p.asset).and_then(|a| a.max_relief) {
        if high - low > limit * p.scale {
            diags.push(
                text::diag(
                    codes::place::RELIEF,
                    subject.clone(),
                    Site::Header,
                    &format!(
                        "relief of `{}` under `{}` is {} m; its asset allows {} m",
                        s.place,
                        p.name,
                        text::dbl(high - low),
                        text::dbl(limit * p.scale)
                    ),
                    "move it, or choose an asset with a larger `max_relief`",
                )
                .at(text::pos(p.origin.x, p.origin.y)),
            );
        }
    }
}

/// `surface.through` (4.1, warning): a hand instance on the terrain whose height reaches
/// through a kind surface above its origin, lying above its `z` and below its top (`z` plus its
/// bounds' top, scaled). Said once per instance, naming the lowest such surface.
pub(crate) fn through(
    d: &Decls,
    pack: &crate::pack::Pack,
    surfaces: &BTreeMap<String, SurfaceOut>,
    all: &[crate::rules::Placement],
    stands: &BTreeMap<String, String>,
    diags: &mut Vec<Diag>,
) {
    let mut said: BTreeSet<&str> = BTreeSet::new();
    for p in all {
        // A rule, run, scatter or step instance is not tested (4.1): a step's by its name, as
        // its `maker` is set only when the world has a hand edit.
        if p.maker.is_some()
            || p.pin
            || stands.contains_key(&p.name)
            || d.produced_instance(&p.name)
        {
            continue;
        }
        let head = p.name.split('.').next().unwrap_or(&p.name);
        if said.contains(head) {
            continue;
        }
        let Some(a) = pack.assets.get(&p.asset) else {
            continue;
        };
        let top = p.z + via_num::metres(a.bounds[5]) * p.scale;
        let o = (p.origin.x, p.origin.y);
        let mut hit: Option<(f64, &str)> = None;
        for s in surfaces.values() {
            if let Ok(Some(z)) = s.z(o) {
                if z > p.z && z < top && hit.is_none_or(|(hz, _)| z < hz) {
                    hit = Some((z, s.place.as_str()));
                }
            }
        }
        let Some((z, place)) = hit else { continue };
        said.insert(head);
        let owner = place.split('.').next().unwrap_or(place);
        diags.push(
            text::diag(
                codes::surface::THROUGH,
                Subject::entity("instance", head),
                Site::Header,
                &format!(
                    "`{}` stands on the terrain and reaches {} m, through `{place}` at {} m",
                    p.name,
                    text::dbl(top),
                    text::dbl(z)
                ),
                &format!("place it `on` {owner} to stand on that surface, or move it"),
            )
            .at(text::pos(p.origin.x, p.origin.y))
            .with("surface", DValue::text(place)),
        );
    }
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
        let (lo, hi) = match s.cut_band.as_ref().and_then(via_geom::Region::bbox) {
            Some(b) => b,
            None => s.bound.bbox(),
        };
        // The quads whose centre can lie in the bound's box.
        let i0 = ((lo.x - half - x0).div_euclid(r)).max(0) as usize;
        let j0 = ((lo.y - half - y0).div_euclid(r)).max(0) as usize;
        let i1 = (((hi.x - half - x0).div_euclid(r)).max(-1) + 1).min(qx as i64) as usize;
        let j1 = (((hi.y - half - y0).div_euclid(r)).max(-1) + 1).min(qy as i64) as usize;
        for j in j0..j1 {
            for i in i0..i1 {
                let c = (x0 + i as i64 * r + half, y0 + j as i64 * r + half);
                let z = match &s.cut_band {
                    // A widened band (`composition.md` 6.1) reads the formula past the bound.
                    Some(band) if band.side(P::new(c.0, c.1)) == via_geom::pred::Side::Out => {
                        continue
                    }
                    // Past the bound the surface does not stand: a fault there cuts nothing and
                    // is not the owner's (`composition.md` 6.1). Inside, as any surface (2.7).
                    Some(_) if !s.inside(c) => Ok(s
                        .base
                        .eval(&[c])
                        .ok()
                        .and_then(|v| v.first().copied())
                        .filter(|z| z.is_finite())),
                    _ => s.z_in_build(c, diags),
                };
                let z = match z {
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
///
/// A path owner's surface with `cut` (its `skirt`) also covers, in each cell the terrain's cut
/// file marks (`cut`, indexed as `build.md` 6.5), the cell's part within the skirt and outside the
/// bound, its height the formula's there: so a sheer face's mesh closes the slivers its cut quads
/// open past its strip, its strip's edges still exact ("Skirt", build.md 10).
pub(crate) fn meshes(
    grid: &via_num::Grid,
    surfaces: &BTreeMap<String, SurfaceOut>,
    cut: Option<&[u8]>,
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
        let qx = (grid.nx as i64 - 1).max(0);
        let quads = cut.filter(|c| c.len() as i64 == qx * (grid.ny as i64 - 1).max(0));
        let skirt = s.skirt.as_ref().filter(|_| quads.is_some());
        let Some((lo, hi)) = skirt.unwrap_or(region).bbox() else {
            continue;
        };
        let (lo, hi) = (
            P::new(lo.x.max(x0), lo.y.max(y0)),
            P::new(hi.x.min(x1), hi.y.min(y1)),
        );
        if lo.x >= hi.x || lo.y >= hi.y {
            continue;
        }
        let ring_edges = |r: &via_geom::Region| -> Vec<(P, P)> {
            r.rings()
                .iter()
                .flat_map(|ring| {
                    (0..ring.len()).map(move |k| (ring[k], ring[(k + 1) % ring.len()]))
                })
                .collect()
        };
        let bound_edges = ring_edges(region);
        let skirt_edges = skirt.map(ring_edges).unwrap_or_default();
        // The cut cells a skirt covers: those whose centre lies in the bound of a surface of the
        // owner's kind and name that cuts (its own, or a piece it meets at a joint), never a
        // neighbour's floor or pad (M5 close: a face's skirt over a cave floor's cut z-fought).
        let kin: Vec<&via_geom::Region> = if skirt.is_some() {
            surfaces
                .values()
                .filter(|o| o.kind == s.kind && o.name == s.name && o.cut.is_some())
                .filter_map(|o| o.cut_band.as_ref().or_else(|| o.region()))
                .collect()
        } else {
            Vec::new()
        };
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
                // Every cell is clipped to the bound; a cut cell of a skirted surface also gives
                // its part within the skirt and outside the bound, so the bound's outline stays a
                // constraint (the face's edges exact) and the skirt lies beside it.
                let is_cut = quads.is_some_and(|c| {
                    i >= 0 && j >= 0 && i < qx && c.get((j * qx + i) as usize) == Some(&1)
                });
                let touched = bound_edges.iter().any(|(a, b)| {
                    a.x.max(b.x) >= cx0
                        && a.x.min(b.x) <= cx1
                        && a.y.max(b.y) >= cy0
                        && a.y.min(b.y) <= cy1
                });
                let cell = via_geom::Region::from_ring(square.clone());
                let mut tris: Vec<[P; 3]> = Vec::new();
                if !touched {
                    let c = P::new(cx0 + (cx1 - cx0) / 2, cy0 + (cy1 - cy0) / 2);
                    if region.side(c) != via_geom::pred::Side::Out {
                        tris.push([square[0], square[1], square[2]]);
                        tris.push([square[0], square[2], square[3]]);
                    }
                } else if let Ok(piece) =
                    boolean(BoolOp::Intersection, &[cell.clone(), region.clone()])
                {
                    if !piece.is_empty() {
                        if let Ok(t) = via_geom::triangulate::triangulate(&piece) {
                            tris.extend(t);
                        }
                    }
                }
                let kin_cut = is_cut && {
                    let c = P::new(cx0 + (cx1 - cx0) / 2, cy0 + (cy1 - cy0) / 2);
                    kin.iter().any(|g| g.side(c) != via_geom::pred::Side::Out)
                };
                if let Some(k) = skirt.filter(|_| kin_cut) {
                    let near = skirt_edges.iter().any(|(a, b)| {
                        a.x.max(b.x) >= cx0
                            && a.x.min(b.x) <= cx1
                            && a.y.max(b.y) >= cy0
                            && a.y.min(b.y) <= cy1
                    });
                    let within = if near {
                        boolean(BoolOp::Intersection, &[cell, k.clone()]).ok()
                    } else {
                        Some(cell)
                    };
                    if let Some(w) = within.filter(|w| !w.is_empty()) {
                        if let Ok(rest) = boolean(BoolOp::Difference, &[w, region.clone()]) {
                            if !rest.is_empty() {
                                if let Ok(t) = via_geom::triangulate::triangulate(&rest) {
                                    tris.extend(t);
                                }
                            }
                        }
                    }
                }
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
                            // overlay put a hair outside, or on a skirt, takes the formula's
                            // value there.
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
        let subject = Subject::entity("feature", id.as_str());
        let Some(Geo::Point { p, .. }) = placed.shapes.get(id) else {
            continue;
        };
        if let (Some(h), None) = (&f.height, &f.surface) {
            // The height chose nothing (4.1, R65): no candidate, or one whose formula failed
            // there, said on its owner (2.7).
            if h.among.is_empty() {
                diags.push(none_at(subject, &format!("feature.{id}"), (p.x, p.y)));
            } else {
                for s in h.among.iter().filter_map(|n| surfaces.get(n)) {
                    let _ = s.z_in_build((p.x, p.y), diags);
                }
            }
            continue;
        }
        let Some(sn) = &f.surface else {
            // A point `on` a thing stands where an instance `on` it would (4.1, row 1 to 3):
            // the one kind surface of a feature, or a mount's surface. A point laid on the
            // owner's line within one lattice unit of the bound is on it (1.2); one an `offset`
            // lays farther out stands on the terrain, as a target does, and nothing is said
            // (4.1, decided in slice 3: a view beside a deck that moves with it).
            let crate::decl::ShapeDecl::Point(crate::decl::Place::On(o)) = &f.shape else {
                continue;
            };
            match on_target(d, &o.reference, 0) {
                Ok(Some(n)) => {
                    let Some(s) = surfaces.get(&n) else { continue };
                    let near = (-1..=1).any(|dx| (-1..=1).any(|dy| s.inside((p.x + dx, p.y + dy))));
                    if near {
                        out.insert(id.clone(), s.place.clone());
                    }
                }
                Ok(None) => {}
                Err(names) => diags.push(
                    text::diag(
                        codes::surface::WHICH,
                        subject,
                        Site::Key(text::path(&["on"])),
                        &format!(
                            "`{id}` is placed on `{}`, which owns {}",
                            o.reference,
                            names.join(", ")
                        ),
                        &format!(
                            "place it `on` a mount of `{}`, or by `at` with `surface`",
                            o.reference
                        ),
                    )
                    .with("surfaces", DValue::texts(names.into_iter())),
                ),
            }
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
        } else if let [one] = declared(d, place).as_slice() {
            // A feature that owns one kind surface, named as a target: its point on that
            // surface where the bound holds it (else the terrain, as `xyz_of` falls back).
            Some(one.clone())
        } else if let Some(i) = d.instances.get(place) {
            // An instance named as a target stands where it was grounded (4.1): its `surface`,
            // or what its `on` stands on (else the terrain).
            match (&i.surface, &i.place) {
                (Some(s), _) => Some(s.clone()),
                (None, crate::decl::Place::On(o)) => on_target(d, &o.reference, 0).ok().flatten(),
                _ => None,
            }
        } else if let Some(t) = path_end_meets(d, place) {
            // A path's end named as a target, where the path `starts_at`/`ends_at` a kind
            // surface (or a thing standing on one): that surface (4.4, the end meets it).
            if surfaces.contains_key(&t) {
                Some(t)
            } else {
                on_target(d, &t, 0).ok().flatten()
            }
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

/// What `<path>.start` or `<path>.end` meets: the path's `starts_at` or `ends_at`.
pub(crate) fn path_end_meets(d: &Decls, place: &str) -> Option<String> {
    let (head, end) = place.rsplit_once('.')?;
    let f = d.features.get(head)?;
    match end {
        "start" => f.starts_at.clone(),
        "end" => f.ends_at.clone(),
        _ => None,
    }
}

/// `surface.none_at` (4.1, R65): a height in `at` where no kind surface holds the plan point.
fn none_at(subject: Subject, what: &str, at: (i64, i64)) -> Diag {
    text::diag(
        codes::surface::NONE_AT,
        subject,
        Site::Key(text::path(&["at"])),
        &format!(
            "`{what}` writes a height at [{}, {}], and no kind surface holds that point",
            via_num::write_length(at.0),
            via_num::write_length(at.1)
        ),
        "drop the third number to stand on the terrain, or move it onto a surface",
    )
    .at(text::pos(at.0, at.1))
}

/// A height in millimetres on the lattice (`numbers.md` 3.1), as a meeting's gap rounds one
/// (2.4); one past the lattice's range stands at its end, so "nearest" keeps its order.
fn height_mm(z: f64) -> i64 {
    via_num::round_to_lattice(z * 1000.0).unwrap_or(if z > 0.0 {
        via_num::LATTICE_MAX
    } else {
        -via_num::LATTICE_MAX
    })
}

/// The choice of 4.1 (R65) for one plan point: among the kind surfaces whose bound holds it and
/// whose owner lies at `rank` or below, the one whose height there is nearest `z`, in whole
/// millimetres; a tie to the lower, then by place name. Gives the candidates by place name and
/// the chosen one; `None` chosen where there is no candidate or a candidate's formula fails
/// there.
fn nearest_surface(
    d: &Decls,
    kinds: &BTreeMap<String, SurfaceOut>,
    at: (i64, i64),
    z: f64,
    rank: i64,
) -> (Vec<String>, Option<String>) {
    let want = height_mm(z);
    let mut among = Vec::new();
    let mut best: Option<(i64, f64, &str)> = None;
    let mut failed = false;
    // Place names in bytewise order, so the first of equals is the tie's.
    for s in kinds.values() {
        let seen = d.features.get(&s.owner).is_some_and(|f| f.rank <= rank);
        if !seen || !s.inside(at) {
            continue;
        }
        among.push(s.place.clone());
        let Some(h) = s.base.eval(&[at]).ok().and_then(|v| v.first().copied()) else {
            failed = true;
            continue;
        };
        let apart = (height_mm(h) - want).abs();
        if best.is_none_or(|(a, low, _)| apart < a || (apart == a && h < low)) {
            best = Some((apart, h, s.place.as_str()));
        }
    }
    let chosen = if failed {
        None
    } else {
        best.map(|(_, _, n)| n.to_string())
    };
    (among, chosen)
}

/// Resolves every height written in `at` (4.1, R65) to the kind surface it chooses, on plans:
/// the shapes are placed, and a kind surface's `base` reads no field (1.1). The choice is
/// written to the thing's `surface`, so from here it is a thing placed with `surface`: its
/// level, its grounding, what stands on it and its graph edges read that one key. A hand
/// instance (a set it places included), a pin and a point feature; each sees the surfaces of
/// its own layer and below.
pub(crate) fn choose_by_height(d: &mut Decls, shapes: &BTreeMap<String, Geo>) {
    use crate::decl::{Place, ShapeDecl};
    let any = d.instances.values().any(|i| i.height.is_some())
        || d.pins.values().any(|p| p.height.is_some())
        || d.features.values().any(|f| f.height.is_some());
    if !any {
        return;
    }
    let kinds = kind_surfaces(d, shapes, &mut Vec::new()).0;
    // A run that runs no step, in a world that declares a generate step (`surfaces.md` 7): the
    // surface a height would choose may be one a step returns, which is not here. A height that
    // finds no candidate is then dropped: the thing stands on the terrain, as with two numbers.
    let may_miss = d.steps_unrun && d.steps.values().any(|s| s.when == "generate");
    let mut chosen: Vec<(&'static str, String, Vec<String>, Option<String>)> = Vec::new();
    for (id, i) in &d.instances {
        if let (Some(h), Place::At(at)) = (&i.height, &i.place) {
            let (among, to) = nearest_surface(d, &kinds, *at, h.z, i.rank);
            chosen.push(("instance", id.clone(), among, to));
        }
    }
    for (id, p) in &d.pins {
        if let (Some(h), Place::At(at)) = (&p.height, &p.place) {
            let (among, to) = nearest_surface(d, &kinds, *at, h.z, p.rank);
            chosen.push(("pin", id.clone(), among, to));
        }
    }
    for (id, f) in &d.features {
        if let (Some(h), ShapeDecl::Point(Place::At(at))) = (&f.height, &f.shape) {
            let (among, to) = nearest_surface(d, &kinds, *at, h.z, f.rank);
            chosen.push(("feature", id.clone(), among, to));
        }
    }
    for (top, id, among, to) in chosen {
        let (height, surface) = match top {
            "instance" => match d.instances.get_mut(&id) {
                Some(i) => (&mut i.height, &mut i.surface),
                None => continue,
            },
            "pin" => match d.pins.get_mut(&id) {
                Some(p) => (&mut p.height, &mut p.surface),
                None => continue,
            },
            _ => match d.features.get_mut(&id) {
                Some(f) => (&mut f.height, &mut f.surface),
                None => continue,
            },
        };
        if may_miss && among.is_empty() {
            *height = None;
            *surface = None;
            continue;
        }
        if let Some(h) = height {
            h.among = among;
        }
        *surface = to;
    }
}

/// What a thing stands on and how high it reaches above it (`rules.md` 8.1): plan only, read
/// from the declarations and the asset's facts, never from a field (4.3 step 1).
#[derive(Clone, Debug)]
pub(crate) struct Level {
    /// The place name of the kind surface the thing's declaration names; `""` for the terrain.
    pub surface: String,
    /// Its span above that surface in metres: `-sink` to `-sink` plus its bounds' height, scaled.
    pub lo: f64,
    pub hi: f64,
    /// Its own plan point, where a surface's height is read for it (8.1, R70).
    pub at: (i64, i64),
}

impl Level {
    pub(crate) fn new(surface: String, span: (f64, f64), at: P) -> Level {
        Level {
            surface,
            lo: span.0,
            hi: span.1,
            at: (at.x, at.y),
        }
    }
}

/// The [`Level`] of a placed thing: what its declaration says it stands on, its asset's span
/// and its origin.
pub(crate) fn level(d: &Decls, pack: &crate::pack::Pack, p: &crate::rules::Placement) -> Level {
    let surface = match stance(d, p) {
        Some(st) => match (&st.surface, &st.on) {
            (Some(s), _) => s.clone(),
            (None, Some(r)) => on_target(d, r, 0).ok().flatten().unwrap_or_default(),
            _ => String::new(),
        },
        None => String::new(),
    };
    Level::new(surface, span(pack, &p.asset, p.scale), p.origin)
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

/// The least gap, in millimetres of height, at which one path passes over another and does not
/// meet it (`geometry.md` 2.7, R71). Ours: half a metre, less than any storey and more than a
/// kerb or a step, so two decks a hand apart are one junction and a flight under a flight is
/// not.
const APART_MIN: i64 = 500;
/// The finest lattice, in millimetres, the heights of two paths' surfaces are read on (R71).
const APART_STEP: i64 = 250;
/// The most steps a side of that lattice (R71).
const APART_STEPS: i64 = 96;

/// A height in whole millimetres at each plan point asked so far; `None` where it cannot be read.
type HeightsAt = BTreeMap<(i64, i64), Option<i64>>;

/// The kind surfaces the level test reads (`rules.md` 8.1): their bounds, `cut` and heights,
/// plan only (made from the owners' shapes, 4.3 step 3; a `base` reads no field, 1.1).
pub(crate) struct Levels {
    kinds: BTreeMap<String, SurfaceOut>,
    /// Each surface's height at a plan point, as read once; `None` where its `base` fails
    /// there (2.7).
    heights: std::cell::RefCell<BTreeMap<String, HeightsAt>>,
    /// The height of the lowest surface with `cut` whose bound holds a plan point, as read once;
    /// `None` where none holds it with a height that can be read.
    lowest: std::cell::RefCell<HeightsAt>,
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
            heights: Default::default(),
            lowest: Default::default(),
        }
    }

    /// A kind surface's height at a plan point, on plans (8.1, R70): its `base` there, rounded
    /// to the lattice in whole millimetres as a height in `at` is (4.1). `None` where it cannot
    /// be read: no such surface, or a `base` that fails or is not finite there (2.7).
    fn height(&self, surface: &str, at: (i64, i64)) -> Option<i64> {
        if let Some(h) = self.heights.borrow().get(surface).and_then(|m| m.get(&at)) {
            return *h;
        }
        let h = self
            .kinds
            .get(surface)
            .and_then(|s| s.base.eval(&[at]).ok())
            .and_then(|v| v.first().copied())
            .filter(|z| z.is_finite())
            .map(height_mm);
        self.heights
            .borrow_mut()
            .entry(surface.to_string())
            .or_default()
            .insert(at, h);
        h
    }

    /// The height of the lowest kind surface with `cut` whose bound holds the plan point, among
    /// those whose height can be read there (8.1, R70).
    fn lowest_cut(&self, at: (i64, i64)) -> Option<i64> {
        if let Some(l) = self.lowest.borrow().get(&at) {
            return *l;
        }
        let l = self
            .kinds
            .values()
            .filter(|s| s.cut.is_some() && s.inside(at))
            .filter_map(|s| self.height(&s.place, at))
            .min();
        self.lowest.borrow_mut().insert(at, l);
        l
    }

    /// Whether two paths whose lines cross in plan pass one over the other and do not meet
    /// (`geometry.md` 2.7, R71). Each must own a kind surface. Their heights are read on plans
    /// over the ground the two bounds share: at `at`, where the lines cross, and on a lattice
    /// over the box the bounds have in common, at most [`APART_STEPS`] steps a side and no
    /// finer than [`APART_STEP`]. One path's surfaces span a range of heights there, and the
    /// other's another; the paths are apart where the gap between the two ranges is
    /// [`APART_MIN`] or more. A path that owns no surface, or none whose height can be read
    /// there, is told apart from nothing.
    pub(crate) fn apart(&self, a: &str, b: &str, at: (i64, i64)) -> bool {
        let of = |owner: &str| -> Vec<&SurfaceOut> {
            self.kinds
                .values()
                .filter(|s| {
                    s.place
                        .strip_prefix(owner)
                        .is_some_and(|rest| rest.starts_with('.'))
                })
                .collect()
        };
        let (sa, sb) = (of(a), of(b));
        if sa.is_empty() || sb.is_empty() {
            return false;
        }
        let mut points = vec![at];
        for x in &sa {
            for y in &sb {
                let (Some((alo, ahi)), Some((blo, bhi))) = (
                    x.region().and_then(via_geom::Region::bbox),
                    y.region().and_then(via_geom::Region::bbox),
                ) else {
                    continue;
                };
                let (x0, y0) = (alo.x.max(blo.x), alo.y.max(blo.y));
                let (x1, y1) = (ahi.x.min(bhi.x), ahi.y.min(bhi.y));
                if x0 > x1 || y0 > y1 {
                    continue;
                }
                let step = APART_STEP.max((x1 - x0).max(y1 - y0) / APART_STEPS);
                let mut py = y0;
                loop {
                    let mut px = x0;
                    loop {
                        if x.inside((px, py)) && y.inside((px, py)) {
                            points.push((px, py));
                        }
                        if px >= x1 {
                            break;
                        }
                        px = (px + step).min(x1);
                    }
                    if py >= y1 {
                        break;
                    }
                    py = (py + step).min(y1);
                }
            }
        }
        let range = |surfaces: &[&SurfaceOut]| -> Option<(i64, i64)> {
            let mut out: Option<(i64, i64)> = None;
            for s in surfaces {
                for p in points.iter().filter(|p| s.inside(**p)) {
                    if let Some(h) = self.height(&s.place, *p) {
                        out = Some(out.map_or((h, h), |(lo, hi)| (lo.min(h), hi.max(h))));
                    }
                }
            }
            out
        };
        match (range(&sa), range(&sb)) {
            (Some((alo, ahi)), Some((blo, bhi))) => {
                blo - ahi >= APART_MIN || alo - bhi >= APART_MIN
            }
            _ => false,
        }
    }

    /// Whether the terrain and the kind surface a thing stands on are one level (8.1): the
    /// surface stands on the ground. Its kind writes `cut`, or its bound meets the footprint of
    /// the thing on the terrain (R40); and no surface with `cut` holds the plan point of the
    /// thing on it and stands lower there (R70: only the lowest is the ground). A surface whose
    /// own height cannot be read keeps the first clause alone.
    fn ground(&self, terrain: &via_geom::Region, on: &Level) -> bool {
        let Some(s) = self.kinds.get(&on.surface) else {
            return false;
        };
        if s.cut.is_none()
            && !s
                .region()
                .is_some_and(|r| crate::rules::overlaps(terrain, r))
        {
            return false;
        }
        match (self.height(&on.surface, on.at), self.lowest_cut(on.at)) {
            (Some(h), Some(low)) => low >= h,
            _ => true,
        }
    }

    /// Whether two things can clash (`rules.md` 8.1): one level, and spans whose insides meet
    /// (a span of no height meets one it touches). One surface is one level; so are two kind
    /// surfaces whose heights, each at its own thing's plan point, are equal to the millimetre
    /// (R70), and the terrain with a kind surface that stands on the ground ([`Levels::ground`]).
    pub(crate) fn clash(
        &self,
        a: &Level,
        fa: &via_geom::Region,
        b: &Level,
        fb: &via_geom::Region,
    ) -> bool {
        if a.surface != b.surface {
            let one = match (a.surface.is_empty(), b.surface.is_empty()) {
                (true, false) => self.ground(fa, b),
                (false, true) => self.ground(fb, a),
                _ => match (self.height(&a.surface, a.at), self.height(&b.surface, b.at)) {
                    (Some(x), Some(y)) => x == y,
                    _ => false,
                },
            };
            if !one {
                return false;
            }
        }
        let (lo, hi) = (via_num::max(a.lo, b.lo), via_num::min(a.hi, b.hi));
        lo < hi || ((a.lo == a.hi || b.lo == b.hi) && lo <= hi)
    }
}

#[cfg(test)]
mod joint_tests {
    use super::*;

    fn round(z: f64) -> i64 {
        via_num::round_to_lattice(z * 1000.0).unwrap_or(0)
    }

    /// An asset mount of `owner` at `(x, y)` mm and `z` m, with a connector unless `c` is empty.
    fn mount(owner: &str, x: i64, y: i64, z: f64, c: &str) -> Side {
        Side {
            p: Some(P::new(x, y)),
            z: Some(z),
            face: Some(0),
            connector: (!c.is_empty()).then(|| c.to_string()),
            mount: true,
            owner: Some(owner.to_string()),
            rise: 0,
        }
    }

    fn map(items: &[(&str, Side)]) -> BTreeMap<String, Side> {
        items
            .iter()
            .map(|(n, s)| (n.to_string(), s.clone()))
            .collect()
    }

    fn pair(a: &str, b: &str) -> (String, String) {
        (a.to_string(), b.to_string())
    }

    #[test]
    fn two_instances_at_one_point_join_and_are_ordered_bytewise() {
        let m = map(&[
            ("b.in", mount("b", 1000, 2000, 15.0, "pipe")),
            ("a.out", mount("a", 1000, 2000, 15.0, "pipe")),
        ]);
        let j = joints(&m, &round);
        assert_eq!(j.pairs, vec![pair("a.out", "b.in")]);
        assert!(j.crowded.is_empty());
    }

    #[test]
    fn a_millimetre_apart_in_plan_is_no_joint() {
        let m = map(&[
            ("a.out", mount("a", 1000, 2000, 15.0, "pipe")),
            ("b.in", mount("b", 1001, 2000, 15.0, "pipe")),
        ]);
        assert_eq!(joints(&m, &round), Joints::default());
    }

    #[test]
    fn no_connector_no_owner_no_height_take_no_part() {
        let mut kind = mount("x", 0, 0, 15.0, "pipe");
        kind.owner = None;
        let mut blind = mount("d", 0, 0, 15.0, "pipe");
        blind.z = None;
        let m = map(&[
            ("a.out", mount("a", 0, 0, 15.0, "pipe")),
            ("b.in", mount("b", 0, 0, 15.0, "")),
            ("c.head", kind),
            ("d.in", blind),
        ]);
        assert_eq!(joints(&m, &round), Joints::default());
    }

    #[test]
    fn two_mounts_of_one_instance_never_join() {
        let m = map(&[
            ("a.low", mount("a", 0, 0, 13.0, "pipe")),
            ("a.high", mount("a", 0, 0, 13.0, "pipe")),
        ]);
        assert_eq!(joints(&m, &round), Joints::default());
        // Nor at two heights, as the two left over.
        let m = map(&[
            ("a.low", mount("a", 0, 0, 13.0, "pipe")),
            ("a.high", mount("a", 0, 0, 15.0, "pipe")),
        ]);
        assert_eq!(joints(&m, &round), Joints::default());
    }

    #[test]
    fn another_connector_still_joins_for_the_check_to_say_so() {
        let m = map(&[
            ("a.out", mount("a", 0, 0, 15.0, "pipe")),
            ("b.in", mount("b", 0, 0, 15.0, "steam")),
        ]);
        assert_eq!(joints(&m, &round).pairs, vec![pair("a.out", "b.in")]);
    }

    #[test]
    fn four_at_a_point_join_in_twos_by_height() {
        let m = map(&[
            ("wall.low", mount("wall", 0, 0, 13.0, "pipe")),
            ("wall.high", mount("wall", 0, 0, 15.0, "pipe")),
            ("lo.c", mount("lo", 0, 0, 13.0, "pipe")),
            ("hi.c", mount("hi", 0, 0, 15.0, "pipe")),
        ]);
        let j = joints(&m, &round);
        assert_eq!(
            j.pairs,
            vec![pair("hi.c", "wall.high"), pair("lo.c", "wall.low")]
        );
        assert!(j.crowded.is_empty());
    }

    #[test]
    fn the_two_left_over_join_at_two_heights() {
        let m = map(&[
            ("wall.low", mount("wall", 0, 0, 13.0, "pipe")),
            ("wall.high", mount("wall", 0, 0, 15.0, "pipe")),
            ("lo.c", mount("lo", 0, 0, 14.0, "pipe")),
            ("hi.c", mount("hi", 0, 0, 15.0, "pipe")),
        ]);
        assert_eq!(
            joints(&m, &round).pairs,
            vec![pair("hi.c", "wall.high"), pair("lo.c", "wall.low")]
        );
        // Alone at the point, two at two heights are a joint too: its gap is the check's.
        let m = map(&[
            ("a.out", mount("a", 0, 0, 15.0, "pipe")),
            ("b.in", mount("b", 0, 0, 16.0, "pipe")),
        ]);
        assert_eq!(joints(&m, &round).pairs, vec![pair("a.out", "b.in")]);
    }

    #[test]
    fn three_left_over_join_nothing() {
        let m = map(&[
            ("wall.low", mount("wall", 0, 0, 13.0, "pipe")),
            ("wall.high", mount("wall", 0, 0, 15.0, "pipe")),
            ("lo.c", mount("lo", 0, 0, 14.0, "pipe")),
        ]);
        assert_eq!(joints(&m, &round), Joints::default());
    }

    #[test]
    fn three_at_one_height_are_crowded_and_join_nothing() {
        let m = map(&[
            ("c.end", mount("c", 0, 0, 15.0, "pipe")),
            ("a.out", mount("a", 0, 0, 15.0, "pipe")),
            ("b.in", mount("b", 0, 0, 15.0, "pipe")),
            // Another height at the same point is its own level: alone, it joins nothing.
            ("d.in", mount("d", 0, 0, 12.0, "pipe")),
        ]);
        let j = joints(&m, &round);
        assert!(j.pairs.is_empty());
        assert_eq!(
            j.crowded,
            vec![vec![
                "a.out".to_string(),
                "b.in".to_string(),
                "c.end".to_string()
            ]]
        );
        // Three of which two are one instance's: still not all one instance's.
        let m = map(&[
            ("a.out", mount("a", 0, 0, 15.0, "pipe")),
            ("a.side", mount("a", 0, 0, 15.0, "pipe")),
            ("b.in", mount("b", 0, 0, 15.0, "pipe")),
        ]);
        assert_eq!(joints(&m, &round).crowded.len(), 1);
    }

    #[test]
    fn mounts_at_a_point_join_within_one_connector_first() {
        // Two pipes and an end of another connector at one height: the pipes are a joint, and the third is
        // left (as first built the three were crowded and the joint was lost; K9's review).
        let m = map(&[
            ("a.out", mount("a", 0, 0, 15.0, "pipe")),
            ("b.in", mount("b", 0, 0, 15.0, "pipe")),
            ("c.end", mount("c", 0, 0, 15.0, "other")),
        ]);
        let j = joints(&m, &round);
        assert_eq!(j.pairs, vec![pair("a.out", "b.in")]);
        assert!(j.crowded.is_empty());
        // Two of each: two joints.
        let mut m = m;
        m.insert("d.end".into(), mount("d", 0, 0, 15.0, "other"));
        assert_eq!(
            joints(&m, &round).pairs,
            vec![pair("a.out", "b.in"), pair("c.end", "d.end")]
        );
        // One of each and nothing else: the two left over are a joint, for
        // `surface.connector` to report.
        let m = map(&[
            ("a.out", mount("a", 0, 0, 15.0, "pipe")),
            ("c.end", mount("c", 0, 0, 15.0, "other")),
        ]);
        assert_eq!(joints(&m, &round).pairs, vec![pair("a.out", "c.end")]);
    }

    #[test]
    fn a_mount_left_at_a_point_is_told_what_stands_beside_it() {
        let m = map(&[
            ("a.out", mount("a", 0, 0, 15.0, "pipe")),
            ("b.in", mount("b", 0, 0, 15.0, "pipe")),
            ("c.end", mount("c", 0, 0, 16.0, "other")),
            ("c.side", mount("c", 0, 0, 15.0, "other")), // its own: never named
            ("d.far", mount("d", 1, 0, 15.0, "pipe")),   // a millimetre off: another point
            ("e.plain", mount("e", 0, 0, 15.0, "")),     // no connector: no part
        ]);
        let got: Vec<&str> = beside("c.end", &m["c.end"], &m)
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        assert_eq!(got, vec!["a.out", "b.in"]);
        assert!(beside("d.far", &m["d.far"], &m).is_empty());
    }

    #[test]
    fn heights_are_compared_to_the_millimetre() {
        let m = map(&[
            ("a.out", mount("a", 0, 0, 15.0004, "pipe")),
            ("b.in", mount("b", 0, 0, 15.0, "pipe")),
            ("c.in", mount("c", 0, 0, 15.001, "pipe")),
        ]);
        // 15.0004 rounds to 15.000: a joint with b at one height; c is left alone.
        let j = joints(&m, &round);
        assert_eq!(j.pairs, vec![pair("a.out", "b.in")]);
        assert!(j.crowded.is_empty());
    }

    #[test]
    fn the_nearest_open_mount_is_another_instances_with_the_same_connector() {
        let m = map(&[
            ("r1.in", mount("r1", 0, 0, 10.0, "slot")),
            ("r1.out", mount("r1", 50, 0, 10.0, "slot")), // its own: never named
            ("r4.out", mount("r4", 128, 0, 10.0, "slot")),
            ("r3.out", mount("r3", 100, 0, 10.0, "slot")), // nearer, and answered
            ("r2.out", mount("r2", 60, 0, 10.0, "other")), // another connector
            ("r5.out", mount("r5", 0, 2001, 10.0, "slot")), // past 2 m
        ]);
        let answered: BTreeSet<&str> = ["r3.out"].into_iter().collect();
        let got = nearest_open("r1.in", &m["r1.in"], &m, &answered, &round);
        assert_eq!(got, Some(("r4.out", 128, 0)));
        // At exactly 2 m it is still named; nothing in reach gives none.
        let m = map(&[
            ("a.out", mount("a", 0, 0, 10.0, "slot")),
            ("b.in", mount("b", 0, 2000, 11.5, "slot")),
        ]);
        let none = BTreeSet::new();
        assert_eq!(
            nearest_open("a.out", &m["a.out"], &m, &none, &round),
            Some(("b.in", 2000, 1500))
        );
        let alone = map(&[("a.out", mount("a", 0, 0, 10.0, "slot"))]);
        assert_eq!(
            nearest_open("a.out", &alone["a.out"], &alone, &none, &round),
            None
        );
    }

    #[test]
    fn nearest_is_by_plan_then_height_then_name() {
        let none = BTreeSet::new();
        let m = map(&[
            ("a.out", mount("a", 0, 0, 14.0, "pipe")),
            ("w.low", mount("w", 0, 0, 13.0, "pipe")),
            ("w.high", mount("w", 0, 0, 15.0, "pipe")),
            ("x.far", mount("x", 300, 400, 14.0, "pipe")),
        ]);
        // Both of w's are at the point and 1 m off in height: the first by name. x is at the
        // same height and 500 mm off in plan: plan comes first.
        assert_eq!(
            nearest_open("a.out", &m["a.out"], &m, &none, &round),
            Some(("w.high", 0, 1000))
        );
        let m = map(&[
            ("a.out", mount("a", 0, 0, 14.0, "pipe")),
            ("w.low", mount("w", 0, 0, 13.5, "pipe")),
            ("w.high", mount("w", 0, 0, 15.0, "pipe")),
        ]);
        assert_eq!(
            nearest_open("a.out", &m["a.out"], &m, &none, &round),
            Some(("w.low", 0, 500))
        );
    }
}
