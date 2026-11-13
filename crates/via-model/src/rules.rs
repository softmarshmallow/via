//! Scatter (`rules.md` 7), hand edits on producers' output (`identity.md` 4.1 to 4.3), grounding
//! (`rules.md` 6, `surfaces.md` 4) and the checks on placed things (`rules.md` 8).

use std::collections::{BTreeMap, BTreeSet};

use via_diag::{codes, Diag, Site, Subject, Value as DValue};
use via_expr::{evaluate, Domain, Setup};
use via_geom::held::{self, Clear, ClearPlace, Pin};
use via_geom::lineref::Line;
use via_geom::overlay::{boolean, BoolOp};
use via_geom::pred::{dist2_segment, Dist2, Wide};
use via_geom::{Region, P};
use via_raster::raster::read_at;

use crate::decl::Select;
use crate::decl::{Choice, Decls, Ranged, ScatterDecl};
use crate::pack::Pack;
use crate::place::{footprint, ClearSpot, Geo, Placed, Plan};
use crate::resolve::{Fields, View};
use crate::text::{self, at_header, at_key};

/// A placed instance, any producer, before grounding.
#[derive(Clone, Debug)]
pub struct Placement {
    pub name: String,
    /// The producer, `None` for a hand instance or a pin.
    pub maker: Option<String>,
    /// For a pin: the pin's id.
    pub pin: bool,
    pub asset: String,
    pub origin: P,
    pub facing: i64,
    pub scale: f64,
    pub tint: f64,
    pub footprint: Region,
    /// Metres, after `sink`.
    pub z: f64,
}

pub(crate) fn key_int(i: i64) -> String {
    if i < 0 {
        format!("n{}", -i)
    } else {
        i.to_string()
    }
}

/// The produced name of a scatter's lattice cell (`identity.md` 2.3).
pub fn scatter_name(id: &str, spacing: i64, i: i64, j: i64) -> String {
    format!("{id}#s{spacing}_{}_{}", key_int(i), key_int(j))
}

fn overlaps(a: &Region, b: &Region) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    let (Some((amin, amax)), Some((bmin, bmax))) = (a.bbox(), b.bbox()) else {
        return false;
    };
    if amax.x <= bmin.x || bmax.x <= amin.x || amax.y <= bmin.y || bmax.y <= amin.y {
        return false;
    }
    boolean(BoolOp::Intersection, &[a.clone(), b.clone()]).is_ok_and(|r| r.twice_area() > 0)
}

/// `relief(surface, region)` (`surfaces.md` 2.3) on the terrain: least and greatest height over
/// the region's posts, vertices and crossings with grid lines. The turning points inside a cell
/// are not taken: (left) the exact set of 2.3 owes them.
pub fn relief(f: &Fields<'_>, surface: &str, region: &Region) -> Option<(f64, f64)> {
    let g = f.grid;
    let r = f.get(surface, View::Build)?;
    let s = r.scalar.as_ref()?;
    let [x0, y0, x1, y1] = g.extent;
    let mut pts: Vec<P> = Vec::new();
    for ring in region.rings() {
        for k in 0..ring.len() {
            let (a, b) = (ring[k], ring[(k + 1) % ring.len()]);
            pts.push(a);
            // Crossings with vertical grid lines.
            let (lo, hi) = (a.x.min(b.x), a.x.max(b.x));
            let mut gx = x0 + (lo - x0).div_euclid(g.resolution) * g.resolution;
            while gx <= hi {
                if gx > lo && gx < hi {
                    let y = a.y
                        + (((gx - a.x) as i128 * (b.y - a.y) as i128) / (b.x - a.x) as i128) as i64;
                    pts.push(P::new(gx, y));
                }
                gx += g.resolution;
            }
            let (lo, hi) = (a.y.min(b.y), a.y.max(b.y));
            let mut gy = y0 + (lo - y0).div_euclid(g.resolution) * g.resolution;
            while gy <= hi {
                if gy > lo && gy < hi {
                    let x = a.x
                        + (((gy - a.y) as i128 * (b.x - a.x) as i128) / (b.y - a.y) as i128) as i64;
                    pts.push(P::new(x, gy));
                }
                gy += g.resolution;
            }
        }
    }
    if let Some((bmin, bmax)) = region.bbox() {
        if let Ok(Some(w)) =
            via_raster::window::effect_window(&g, [bmin.x, bmin.y, bmax.x, bmax.y], 0)
        {
            for (i, j) in w.posts() {
                let (x, y) = via_raster::window::post_mm(&g, i, j);
                if region.side(P::new(x, y)) != via_geom::pred::Side::Out {
                    pts.push(P::new(x, y));
                }
            }
        }
    }
    let vals: Vec<f64> = pts
        .iter()
        .filter(|p| p.x >= x0 && p.x <= x1 && p.y >= y0 && p.y <= y1)
        .filter_map(|p| read_at(&g, s, p.x, p.y).ok())
        .collect();
    let lo = via_num::minimum(&vals).ok()?;
    let hi = via_num::maximum(&vals).ok()?;
    Some((lo, hi))
}

/// The assets a scatter's choices resolve to, with diagnostics for unknown ids and taxa.
fn choice_lists(
    s: &ScatterDecl,
    pack: &Pack,
    diags: &mut Vec<Diag>,
) -> Option<Vec<(Vec<String>, i64)>> {
    if pack.failed {
        // The pack's own error stands for every read of it (`diagnostics.md` 4).
        return None;
    }
    let mut out = Vec::new();
    let mut ok = true;
    for Choice { what, weight } in &s.choices {
        if let Some(taxon) = what.strip_prefix('@') {
            if !pack.taxa.contains(taxon) {
                diags.push(at_key(
                    codes::pack::UNKNOWN_TAXON,
                    "scatter",
                    &s.id,
                    "assets",
                    &format!("the taxon `{taxon}` is not declared in the pack"),
                    &text::near_hint(
                        taxon,
                        pack.taxa.iter().map(String::as_str),
                        "declare `[taxon.\"<path>\"]` in the pack",
                    ),
                ));
                ok = false;
                continue;
            }
            let ids: Vec<String> = pack
                .select(taxon)
                .into_iter()
                .map(|a| a.id.clone())
                .collect();
            if ids.is_empty() {
                let with: Vec<&str> = pack
                    .taxa
                    .iter()
                    .filter(|t| !pack.select(t).is_empty())
                    .map(String::as_str)
                    .collect();
                diags.push(at_key(
                    codes::asset::EMPTY_SELECTION,
                    "scatter",
                    &s.id,
                    "assets",
                    &format!("no asset is filed at or under `{taxon}`"),
                    &text::near_hint(taxon, with, "file an asset there"),
                ));
                ok = false;
                continue;
            }
            out.push((ids, *weight));
        } else if pack.assets.contains_key(what) {
            out.push((vec![what.clone()], *weight));
        } else {
            diags.push(text::with_nearest(
                at_key(
                    codes::asset::UNKNOWN,
                    "scatter",
                    &s.id,
                    "assets",
                    &format!("the pack binds no asset `{what}`"),
                    &text::near_hint(
                        what,
                        pack.assets.keys().map(String::as_str),
                        "name an asset the pack declares",
                    ),
                ),
                what,
                pack.assets.keys().map(String::as_str),
            ));
            ok = false;
        }
    }
    ok.then_some(out)
}

/// `rules.md` 7.4: the asset a draw picks.
fn choose(lists: &[(Vec<String>, i64)], u: u64) -> String {
    let total: i128 = lists.iter().map(|(_, w)| i128::from(*w)).sum();
    let target = ((u128::from(u) * total as u128) >> 64) as i128;
    let mut start: i128 = 0;
    for (ids, w) in lists {
        let w = i128::from(*w);
        if target < start + w {
            let k = ids.len() as i128;
            let idx = ((target - start) * k) / w;
            return ids[idx as usize].clone();
        }
        start += w;
    }
    lists
        .last()
        .map(|(ids, _)| ids[ids.len() - 1].clone())
        .unwrap_or_default()
}

fn ranged(r: Ranged, u: u64) -> f64 {
    match r {
        Ranged::Fixed(v) => v,
        Ranged::Range(lo, hi) => lo + via_num::unit(u) * (hi - lo),
    }
}

/// What every producer placed, held, and the hand edits' diagnostics.
#[derive(Debug, Default)]
pub struct Produced {
    pub instances: Vec<Placement>,
    /// Pins that select a scatter's instance (`identity.md` 4.1), placed once it has run.
    pub pins: Vec<Placement>,
    /// What each such selection resolved to: `<table>.<id>` to (produced name, point).
    pub selections: BTreeMap<String, (String, P)>,
    /// Per producer: the names before hand edits, for `place.gave_way` and the change report.
    pub makers: BTreeSet<String>,
    pub diags: Vec<Diag>,
}

/// A hand edit's selection of a scatter's instance (`identity.md` 3), over the run stage.
struct SelectRaw<'r> {
    raw: &'r [held::Instance],
    /// Selections that found nothing, by maker: `<table>.<id>`.
    left: BTreeMap<String, Vec<String>>,
    chosen: BTreeMap<String, (String, P)>,
}

impl SelectRaw<'_> {
    /// The instance of `sel.of` nearest `near`, exactly; a tie goes to the bytewise-least name.
    fn pick(&mut self, top: &str, id: &str, sel: &Select) -> Option<held::Instance> {
        let near = P::new(sel.near.0, sel.near.1);
        let mut best: Option<(i128, &held::Instance)> = None;
        for i in self.raw.iter().filter(|i| i.producer == sel.of) {
            let (dx, dy) = (
                i128::from(i.origin.x - near.x),
                i128::from(i.origin.y - near.y),
            );
            let d2 = dx * dx + dy * dy;
            let better = best.as_ref().is_none_or(|(b, bi)| {
                d2 < *b || (d2 == *b && i.name.as_bytes() < bi.name.as_bytes())
            });
            if better {
                best = Some((d2, i));
            }
        }
        match best {
            Some((_, i)) => {
                self.chosen
                    .insert(format!("{top}.{id}"), (i.name.clone(), i.origin));
                Some(i.clone())
            }
            None => {
                self.left
                    .entry(sel.of.clone())
                    .or_default()
                    .push(format!("{top}.{id}"));
                None
            }
        }
    }
}

/// Runs every scatter and applies hand edits.
pub fn scatter_all(
    d: &Decls,
    pack: &Pack,
    placed: &Placed,
    f: &Fields<'_>,
    hand: &[Placement],
) -> Produced {
    let mut out = Produced::default();
    let grid = f.grid;
    let [ex0, ey0, ex1, ey1] = grid.extent;
    let surface = f.surface();
    let keep_out = f.keep_out(View::Build);
    let mut raw: Vec<held::Instance> = Vec::new();
    let mut facts: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    for s in d.scatters.values() {
        out.makers.insert(s.id.clone());
        let Some(lists) = choice_lists(s, pack, &mut out.diags) else {
            continue;
        };
        let within = match &s.within {
            None if s.within_of.is_some() => match placed.within.get(&s.id) {
                Some(r) => Some(r.clone()),
                // The selection said why it found nothing.
                None => continue,
            },
            None => None,
            Some(w) => match placed.shapes.get(w) {
                Some(Geo::Region { region, .. }) => Some(region.clone()),
                Some(g) => {
                    out.diags.push(at_key(
                        codes::scatter::WITHIN,
                        "scatter",
                        &s.id,
                        "within",
                        &format!("`{w}` is {}, not a region", g.word()),
                        "name a region feature or a footprint feature",
                    ));
                    continue;
                }
                None => {
                    out.diags.push(at_key(
                        codes::identity::UNKNOWN_PLACE,
                        "scatter",
                        &s.id,
                        "within",
                        &format!("`{w}` names no region"),
                        &text::near_hint(
                            w,
                            placed.shapes.keys().map(String::as_str),
                            "name a region feature",
                        ),
                    ));
                    continue;
                }
            },
        };
        let (mut bx0, mut by0, mut bx1, mut by1) = (ex0, ey0, ex1, ey1);
        if let Some((a, b)) = within.as_ref().and_then(Region::bbox) {
            bx0 = bx0.max(a.x);
            by0 = by0.max(a.y);
            bx1 = bx1.min(b.x);
            by1 = by1.min(b.y);
        }
        let sp = s.spacing;
        let st = |purpose: &str| via_num::stream(d.seed, &["rule", s.seed.as_str(), purpose]);
        let (sx, sy, sk, sa, sf, ss, stt) = (
            st("jitter_x"),
            st("jitter_y"),
            st("keep"),
            st("asset"),
            st("face"),
            st("scale"),
            st("tint"),
        );
        let w = (i128::from(sp) * i128::from(s.jitter)) / 1_000_000;
        // Candidates (step 1) and confinement (step 2).
        let mut cands: Vec<(i64, i64, (i64, i64))> = Vec::new();
        if bx0 <= bx1 && by0 <= by1 {
            for j in by0.div_euclid(sp)..=by1.div_euclid(sp) {
                for i in bx0.div_euclid(sp)..=bx1.div_euclid(sp) {
                    let off = |u: u64| -> i64 {
                        ((i128::from(sp) - w).div_euclid(2)
                            + ((u128::from(u) * w as u128) >> 64) as i128)
                            as i64
                    };
                    let x = i * sp + off(via_num::draw(sx, &[i, j]));
                    let y = j * sp + off(via_num::draw(sy, &[i, j]));
                    if x < ex0 || x > ex1 || y < ey0 || y > ey1 {
                        continue;
                    }
                    if let Some(r) = &within {
                        if r.side(P::new(x, y)) == via_geom::pred::Side::Out {
                            continue;
                        }
                    }
                    cands.push((i, j, (x, y)));
                }
            }
        }
        if cands.is_empty() {
            out.diags.push(at_header(
                codes::scatter::EMPTY,
                "scatter",
                &s.id,
                "the scatter placed nothing: no candidate lies in the extent and `within`",
                "widen `within` or the extent",
            ));
            continue;
        }
        // Keep (step 3): density at each candidate, times 1 - keep_out.
        let pts: Vec<(i64, i64)> = cands.iter().map(|c| c.2).collect();
        let density = match s.density {
            None => vec![1.0; pts.len()],
            Some(fid) => {
                let Some(p) = d.formulas.programs.get(&fid) else {
                    continue;
                };
                let sel = DensitySel { f };
                let b = match via_expr::bind(p, &sel) {
                    Ok(b) => b,
                    Err(e) => {
                        out.diags.push(d.formulas.diag(&e));
                        continue;
                    }
                };
                for (_, sel) in &p.reads.selectors {
                    if d.fields
                        .get(sel.as_str())
                        .is_some_and(|fd| fd.role == Some(crate::decl::Role::KeepOut))
                    {
                        out.diags.push(at_key(codes::scatter::KEEP_OUT_TWICE, "scatter", &s.id, "density", &format!("`density` reads the `keep_out` field `{sel}`, which the scatter already applies"), "remove the `keep_out` factor from `density`"));
                    }
                }
                let reads = DensityReads { f };
                match evaluate(
                    p,
                    &b,
                    &reads,
                    &Setup {
                        world_seed: d.seed,
                        feature: None,
                    },
                    &Domain::Points(&pts),
                ) {
                    Ok(v) => v,
                    Err(fail) if fail.error.code == codes::diag::SKIPPED => continue,
                    Err(fail) => {
                        out.diags.push(d.formulas.diag(&fail.error));
                        continue;
                    }
                }
            }
        };
        let mut kept = 0usize;
        let mut gave = (0u64, 0u64);
        let mut gave_box: Option<(P, P)> = None;
        for (k, (i, j, (x, y))) in cands.iter().enumerate() {
            let ko = keep_out
                .as_ref()
                .and_then(|r| read_at(&grid, r, *x, *y).ok())
                .unwrap_or(0.0);
            let p = density[k] * (1.0 - ko);
            if via_num::unit(via_num::draw(sk, &[*i, *j])) >= p {
                continue;
            }
            kept += 1;
            let asset = choose(&lists, via_num::draw(sa, &[*i, *j]));
            let facing = ((u128::from(via_num::draw(sf, &[*i, *j])) * 360_000) >> 64) as i64;
            let scale = ranged(s.scale, via_num::draw(ss, &[*i, *j]));
            let tint = ranged(s.tint, via_num::draw(stt, &[*i, *j]));
            let Some(a) = pack.assets.get(&asset) else {
                continue;
            };
            let origin = P::new(*x, *y);
            let Some(fp) = footprint(a, origin, facing, scale) else {
                continue;
            };
            // Give way (7.5): solid hand instances, then relief.
            // Neither asset `none` (rules.md 7.5, as 8.1 and 3.4's edges read).
            // Pins are hand edits, applied to the held output, not given way to (identity.md 4.1).
            let overlap = a.solid
                && hand.iter().filter(|h| !h.pin).any(|h| {
                    pack.assets.get(&h.asset).is_some_and(|ha| ha.solid)
                        && overlaps(&h.footprint, &fp)
                });
            let mut relief_bad = false;
            if !overlap {
                if let (Some(limit), Some(sf)) = (a.max_relief, surface) {
                    if let Some((lo, hi)) = relief(f, sf, &fp) {
                        relief_bad = hi - lo > limit * scale;
                    }
                }
            }
            if overlap || relief_bad {
                if overlap {
                    gave.0 += 1;
                } else {
                    gave.1 += 1;
                }
                gave_box = Some(match gave_box {
                    None => (origin, origin),
                    Some((a, b)) => (
                        P::new(a.x.min(origin.x), a.y.min(origin.y)),
                        P::new(b.x.max(origin.x), b.y.max(origin.y)),
                    ),
                });
                continue;
            }
            let name = scatter_name(&s.id, sp, *i, *j);
            facts.insert(name.clone(), (scale, tint));
            raw.push(held::Instance {
                name,
                producer: s.id.clone(),
                asset,
                origin,
                face: facing,
                footprint: fp,
            });
        }
        if gave.0 + gave.1 > 0 {
            let (a, b) = gave_box.unwrap_or((P::new(0, 0), P::new(0, 0)));
            out.diags.push(
                at_header(codes::place::GAVE_WAY, "scatter", &s.id, &format!("{} candidates gave way: {} by overlap, {} by relief", gave.0 + gave.1, gave.0, gave.1), "widen the space or change the hand instances it gives way to; nothing if intended")
                    .at(text::pos(a.x, a.y))
                    .with("overlap", DValue::Int(gave.0 as i64))
                    .with("relief", DValue::Int(gave.1 as i64))
                    .with("box", DValue::List(vec![DValue::Length(a.x), DValue::Length(a.y), DValue::Length(b.x), DValue::Length(b.y)])),
            );
        }
        if kept == 0 {
            let stage = if s.density.is_some() {
                "density"
            } else {
                "keep_out"
            };
            out.diags.push(at_key(
                codes::scatter::EMPTY,
                "scatter",
                &s.id,
                stage,
                &format!("the scatter placed nothing: `{stage}` removed every candidate"),
                &format!("change `{stage}`"),
            ));
        }
    }
    // Hand edits (identity.md 4.1): clears, then pins. A hand edit that selects a scatter's
    // instance reads the run stage, `raw`, before any hand edit (section 3).
    let mut clears = Vec::new();
    let mut strips: Vec<(String, Line, i64, Option<Vec<String>>)> = Vec::new();
    let mut selector = SelectRaw {
        raw: &raw,
        left: BTreeMap::new(),
        chosen: BTreeMap::new(),
    };
    for c in d.clears.values() {
        if let Some(by) = &c.by {
            for b in by {
                if !d.scatters.contains_key(b) {
                    out.diags.push(by_fault(d, "clear", &c.id, b));
                }
            }
        }
        let place = match placed.clears.get(&c.id) {
            Some(ClearSpot::Disc(p, r)) => ClearPlace::Disc(*p, *r),
            Some(ClearSpot::Inside(r)) => ClearPlace::Inside(r.clone()),
            Some(ClearSpot::Strip(line, r)) => {
                strips.push((c.id.clone(), line.clone(), *r, c.by.clone()));
                continue;
            }
            None => match placed.deferred.get(&format!("clear.{}", c.id)) {
                Some(sel) => {
                    let Some(inst) = selector.pick("clear", &c.id, sel) else {
                        continue;
                    };
                    if sel.t.is_some() || sel.offset.is_some() {
                        out.diags.push(at_key(
                            codes::geom::NO_LENGTH,
                            "clear",
                            &c.id,
                            "on",
                            "an instance has no length, so no `t` or `offset`",
                            "drop `t` and `offset`",
                        ));
                        continue;
                    }
                    // Without `radius`, the selected instance alone: origins at its origin.
                    ClearPlace::Disc(inst.origin, c.radius.unwrap_or(0))
                }
                // Its place said why it could not be resolved.
                None => continue,
            },
        };
        clears.push(Clear {
            id: c.id.clone(),
            place,
            by: c.by.clone(),
        });
    }
    let mut pin_plans: Vec<(String, Plan)> = placed
        .pins
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    for (key, sel) in &placed.deferred {
        let Some(id) = key.strip_prefix("pin.") else {
            continue;
        };
        let Some(pd) = d.pins.get(id) else { continue };
        let Some(inst) = selector.pick("pin", id, sel) else {
            continue;
        };
        let mut placer = crate::place::Placer::new(d, pack);
        let spot = crate::place::Spot::At {
            p: inst.origin,
            facing: Some(inst.face),
            end: false,
        };
        let plan = placer
            .on_spot("pin", id, &sel.of, spot, sel.t, sel.offset, pd.face)
            .and_then(|(origin, facing)| {
                placer.plan_at(
                    "pin",
                    id,
                    &pd.asset,
                    origin,
                    facing,
                    1.0,
                    1.0,
                    Some(sel.of.clone()),
                    pd.rank,
                )
            });
        out.diags.extend(placer.diags);
        if let Some(plan) = plan {
            out.pins.push(Placement {
                name: plan.name.clone(),
                maker: None,
                pin: true,
                asset: plan.asset.clone(),
                origin: plan.origin,
                facing: plan.facing,
                scale: plan.scale,
                tint: plan.tint,
                footprint: plan.footprint.clone(),
                z: 0.0,
            });
            pin_plans.push((id.to_string(), plan));
        }
    }
    pin_plans.sort_by(|a, b| a.0.cmp(&b.0));
    out.selections = std::mem::take(&mut selector.chosen);
    for (maker, who) in std::mem::take(&mut selector.left) {
        let refs: Vec<(&str, &str)> = who.iter().filter_map(|w| w.split_once('.')).collect();
        let mut placer = crate::place::Placer::new(d, pack);
        placer.near_none(&maker, "instance", &refs);
        out.diags.extend(placer.diags);
    }
    drop(selector);
    // Strips (identity.md 4.3, a clear `on` a path without `t`): origins within `radius` of the
    // nearest point of the whole path, tested exactly.
    for (id, line, r, by) in &strips {
        let r2 = Dist2 {
            num: Wide::from_u128((i128::from(*r) * i128::from(*r)) as u128),
            den: Wide::from_u128(1),
        };
        let before = raw.len();
        raw.retain(|i| {
            let speaks = by.as_ref().is_none_or(|b| b.contains(&i.producer));
            let near = (0..line.segments()).any(|k| {
                let (a, b) = line.seg(k);
                dist2_segment(i.origin, a, b).0.cmp_exact(&r2) != std::cmp::Ordering::Greater
            });
            !(speaks && near)
        });
        if raw.len() == before {
            out.diags.push(at_header(
                codes::identity::CLEAR_EMPTY,
                "clear",
                id,
                &format!("the clear `{id}` removed nothing"),
                "delete the clear, or move it onto what it should remove",
            ));
        }
    }
    let mut pins = Vec::new();
    for (id, p) in &pin_plans {
        let by = d.pins.get(id).and_then(|x| x.by.clone());
        if let Some(by) = &by {
            for b in by {
                if !d.scatters.contains_key(b) {
                    out.diags.push(by_fault(d, "pin", id, b));
                }
            }
        }
        // A pin inside a clear (R31) and in keep_out.
        for c in &clears {
            let inside = match &c.place {
                ClearPlace::Disc(q, r) => {
                    let (dx, dy) = (i128::from(p.origin.x - q.x), i128::from(p.origin.y - q.y));
                    dx * dx + dy * dy <= i128::from(*r) * i128::from(*r)
                }
                ClearPlace::Inside(reg) => reg.side(p.origin) != via_geom::pred::Side::Out,
            };
            if inside {
                out.diags.push(
                    at_header(
                        codes::identity::PIN_IN_CLEAR,
                        "pin",
                        id,
                        &format!(
                            "the pin `{id}` stands inside the clear `{}`; the pin wins",
                            c.id
                        ),
                        "leave it if the pin is what the spot is for; otherwise move the pin",
                    )
                    .at(text::pos(p.origin.x, p.origin.y))
                    .with("clear", DValue::text(c.id.clone())),
                );
            }
        }
        if let Some(k) = keep_out
            .as_ref()
            .and_then(|r| read_at(&grid, r, p.origin.x, p.origin.y).ok())
        {
            if k > 0.0 {
                out.diags.push(
                    at_header(
                        codes::identity::PIN_IN_KEEP_OUT,
                        "pin",
                        id,
                        &format!("the pin `{id}` stands where `keep_out` is above 0"),
                        "move the pin, or accept it here",
                    )
                    .at(text::pos(p.origin.x, p.origin.y)),
                );
            }
        }
        pins.push(Pin {
            id: id.clone(),
            asset: p.asset.clone(),
            origin: p.origin,
            footprint: p.footprint.clone(),
            by,
        });
    }
    match held::hold(raw, &clears, &pins) {
        Ok((live, gd)) => {
            for g in gd {
                let src = g.source.clone().unwrap_or_default();
                let top = if d.pins.contains_key(&src) {
                    "pin"
                } else {
                    "clear"
                };
                out.diags
                    .push(g.to_diag(Subject::entity(top, src), Site::Header));
            }
            for i in live {
                let (scale, tint) = facts.get(&i.name).copied().unwrap_or((1.0, 1.0));
                out.instances.push(Placement {
                    name: i.name,
                    maker: Some(i.producer),
                    pin: false,
                    asset: i.asset,
                    origin: i.origin,
                    facing: i.face,
                    scale,
                    tint,
                    footprint: i.footprint,
                    z: 0.0,
                });
            }
        }
        Err(g) => out.diags.push(g.to_diag(Subject::World, Site::World)),
    }
    out
}

struct DensitySel<'f, 'a> {
    f: &'f Fields<'a>,
}

impl via_expr::Select for DensitySel<'_, '_> {
    fn fields(&self, selector: &str) -> Vec<via_expr::FieldInfo> {
        self.f
            .d
            .fields
            .values()
            .filter(|fd| match selector.strip_prefix("tag:") {
                Some(t) => fd.tags.iter().any(|x| x == t),
                None => fd.name == selector,
            })
            .map(|fd| via_expr::FieldInfo {
                name: fd.name.clone(),
                weights: fd.weights,
            })
            .collect()
    }
    fn shapes(&self, selector: &str) -> Vec<via_expr::ShapeInfo> {
        let shape = |g: &Geo| match g {
            Geo::Point { .. } => via_expr::Shape::Point,
            Geo::Path { .. } => via_expr::Shape::Path,
            Geo::Region { region, .. } => via_expr::Shape::Region {
                rings: region.ring_count() as u32,
            },
        };
        match selector.strip_prefix("kind:") {
            Some(k) => self
                .f
                .writers
                .iter()
                .filter(|w| w.kind == k)
                .filter_map(|w| {
                    self.f
                        .placed
                        .shapes
                        .get(&w.name)
                        .map(|g| via_expr::ShapeInfo {
                            id: w.name.clone(),
                            shape: shape(g),
                        })
                })
                .collect(),
            None => self
                .f
                .placed
                .shapes
                .get(selector)
                .map(|g| {
                    vec![via_expr::ShapeInfo {
                        id: selector.to_string(),
                        shape: shape(g),
                    }]
                })
                .unwrap_or_default(),
        }
    }
}

struct DensityReads<'f, 'a> {
    f: &'f Fields<'a>,
}

impl via_expr::Reads for DensityReads<'_, '_> {
    fn field(&self, field: &str, domain: &Domain) -> Result<Vec<Option<f64>>, via_expr::ReadError> {
        let g = self.f.grid;
        let r = self.f.get(field, View::Build).ok_or_else(|| {
            via_expr::ReadError::new(
                // The field failed and said so; its readers say nothing (`graph.md` 3.4).
                codes::diag::SKIPPED,
                format!("`{field}` could not be resolved"),
                "see the field's own diagnostic",
            )
        })?;
        let s = r.scalar.clone().ok_or_else(|| {
            via_expr::ReadError::new(
                codes::formula::TYPE,
                "a weights field is not a number",
                "read a scalar field",
            )
        })?;
        Ok(domain
            .points()
            .into_iter()
            .map(|(x, y)| read_at(&g, &s, x, y).ok())
            .collect())
    }
    fn measure(
        &self,
        feature: &str,
        m: via_expr::Measure,
        domain: &Domain,
        _cap: Option<f64>,
    ) -> Result<Vec<f64>, via_expr::ReadError> {
        let g = self.f.placed.shapes.get(feature).ok_or_else(|| {
            via_expr::ReadError::new(codes::diag::INTERNAL, "no shape", "report it")
        })?;
        Ok(domain
            .points()
            .into_iter()
            .map(|(x, y)| {
                let p = P::new(x, y);
                let pr = g.project(p);
                match m {
                    via_expr::Measure::D => pr.d,
                    via_expr::Measure::Sd => pr.sd,
                    via_expr::Measure::T => pr.t.unwrap_or(0.0),
                    via_expr::Measure::Inside => f64::from(u8::from(g.contains(p))),
                }
            })
            .collect())
    }
}

/// Hand instances and pins as placements, with the extent check (`rules.md` 8.4).
pub fn hand(
    d: &Decls,
    placed: &Placed,
    grid: &via_num::Grid,
    diags: &mut Vec<Diag>,
) -> Vec<Placement> {
    let [x0, y0, x1, y1] = grid.extent;
    let mut out = Vec::new();
    let mut add = |plan: &Plan, pin: bool, diags: &mut Vec<Diag>| {
        let o = plan.origin;
        if o.x < x0 || o.x > x1 || o.y < y0 || o.y > y1 {
            let top = if pin { "pin" } else { "instance" };
            diags.push(
                at_header(
                    codes::place::OUTSIDE,
                    top,
                    &plan.name,
                    &format!("`{}` stands outside the extent; not placed", plan.name),
                    "move it inside the extent, or widen the extent",
                )
                .at(text::pos(o.x, o.y)),
            );
            return;
        }
        out.push(Placement {
            name: plan.name.clone(),
            maker: None,
            pin,
            asset: plan.asset.clone(),
            origin: o,
            facing: plan.facing,
            scale: plan.scale,
            tint: plan.tint,
            footprint: plan.footprint.clone(),
            z: 0.0,
        });
    };
    for p in placed.plans.values() {
        add(p, false, diags);
    }
    for p in placed.pins.values() {
        add(p, true, diags);
    }
    let _ = d;
    out
}

/// Grounds every placement and checks hand instances (`rules.md` 8, `surfaces.md` 4.2).
pub fn ground_and_check(
    d: &Decls,
    pack: &Pack,
    placed: &Placed,
    f: &Fields<'_>,
    all: &mut [Placement],
    diags: &mut Vec<Diag>,
) {
    let Some(surface) = f.surface() else {
        if let Some(first) = all.first() {
            let (top, id) = match &first.maker {
                Some(m) => ("scatter", m.clone()),
                None if first.pin => ("pin", first.name.clone()),
                None => ("instance", first.name.clone()),
            };
            diags.push(at_header(
                codes::field::ROLE_MISSING,
                top,
                &id,
                "something stands on the ground, and no field has `role = \"surface\"`",
                "add a field with `role = \"surface\"`",
            ));
        }
        return;
    };
    let grid = f.grid;
    let nofp = f.keep_out(View::NoFootprint);
    let ko_names: Vec<String> = d
        .fields
        .values()
        .filter(|x| x.role == Some(crate::decl::Role::KeepOut))
        .map(|x| x.name.clone())
        .collect();
    for p in all.iter_mut() {
        let Some(a) = pack.assets.get(&p.asset) else {
            continue;
        };
        let Some(z) = f.read(surface, View::Build, p.origin) else {
            continue;
        };
        p.z = z - a.sink * p.scale;
        if p.maker.is_some() || p.pin {
            continue;
        }
        // Relief (8.2) and keep_out (8.3) for hand instances.
        if let Some(limit) = a.max_relief {
            if let Some((lo, hi)) = relief(f, surface, &p.footprint) {
                if hi - lo > limit * p.scale {
                    diags.push(at_header(codes::place::RELIEF, "instance", &p.name, &format!("relief under `{}` is {} m; its asset allows {} m", p.name, text::dbl(hi - lo), text::dbl(limit * p.scale)), "move it, give it `ground` with a kind that levels, or choose an asset with a larger `max_relief`").at(text::pos(p.origin.x, p.origin.y)));
                }
            }
        }
        if let Some(k) = &nofp {
            let mut hit = read_at(&grid, k, p.origin.x, p.origin.y)
                .ok()
                .is_some_and(|v| v > 0.0);
            if !hit {
                if let Some((bmin, bmax)) = p.footprint.bbox() {
                    if let Ok(Some(w)) = via_raster::window::effect_window(
                        &grid,
                        [bmin.x, bmin.y, bmax.x, bmax.y],
                        0,
                    ) {
                        hit = w.posts().any(|(i, j)| {
                            let (x, y) = via_raster::window::post_mm(&grid, i, j);
                            p.footprint.side(P::new(x, y)) != via_geom::pred::Side::Out
                                && k.get(i, j).is_some_and(|v| v > 0.0)
                        });
                    }
                }
            }
            if hit {
                diags.push(
                    at_header(
                        codes::place::KEEP_OUT,
                        "instance",
                        &p.name,
                        &format!("`{}` stands where `keep_out` is above 0", p.name),
                        "move it, or change the kind whose effect writes `keep_out` there",
                    )
                    .at(text::pos(p.origin.x, p.origin.y))
                    .with("fields", DValue::texts(ko_names.iter().cloned())),
                );
            }
        }
    }
    // Overlap between hand instances (8.1).
    let hands: Vec<&Placement> = all.iter().filter(|p| p.maker.is_none() && !p.pin).collect();
    for (k, a) in hands.iter().enumerate() {
        for b in &hands[k + 1..] {
            let solid = |x: &Placement| pack.assets.get(&x.asset).is_some_and(|s| s.solid);
            if !solid(a) || !solid(b) {
                continue;
            }
            let on = |x: &Placement, y: &Placement| {
                placed
                    .plans
                    .get(&x.name)
                    .and_then(|pl| pl.on.as_deref())
                    .is_some_and(|o| o == y.name || o.starts_with(&format!("{}.", y.name)))
            };
            if on(a, b) || on(b, a) {
                continue;
            }
            if overlaps(&a.footprint, &b.footprint) {
                diags.push(
                    at_header(
                        codes::place::OVERLAP,
                        "instance",
                        &b.name,
                        &format!("`{}` and `{}` overlap", a.name, b.name),
                        "move one",
                    )
                    .at(text::pos(b.origin.x, b.origin.y))
                    .related(via_diag::Related {
                        subject: Subject::entity("instance", a.name.clone()),
                        site: Site::Header,
                        position: None,
                        note: via_diag::text!("the other instance"),
                    }),
                );
            }
        }
    }
}

/// A `by` name that is no scatter (`identity.md` 4.1): `identity.not_a_producer` when it names
/// something that places no instances (a feature, an instance, an edit), else
/// `identity.unknown_producer` with the nearest scatter.
fn by_fault(d: &Decls, top: &str, id: &str, b: &str) -> Diag {
    let names_something = d.features.contains_key(b)
        || d.instances.contains_key(b)
        || d.pins.contains_key(b)
        || d.clears.contains_key(b);
    if names_something {
        return at_key(
            codes::identity::NOT_A_PRODUCER,
            top,
            id,
            "by",
            &format!("`{b}` places no instances, so nothing of it can be edited"),
            "name the producers that place on it",
        );
    }
    at_key(
        codes::identity::UNKNOWN_PRODUCER,
        top,
        id,
        "by",
        &format!("`{b}` names no producer"),
        &text::near_hint(b, d.scatters.keys().map(String::as_str), "name a scatter"),
    )
}
