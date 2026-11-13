//! Hand edits on a producer's instances (`spec/identity.md` 4.1 to 4.3): clears, then pins. The
//! one routine a build and `via kernel names` both use. Every test is exact, in integer
//! millimetres.

use crate::diag::{Code, GeomDiag};
use crate::overlay::{boolean, BoolOp};
use crate::pred::Side;
use crate::shape::{Region, P};
use via_diag::Value;

/// An instance a producer placed.
#[derive(Clone, Debug)]
pub struct Instance {
    /// Its produced name.
    pub name: String,
    /// Its producer.
    pub producer: String,
    /// Its asset.
    pub asset: String,
    /// Its origin.
    pub origin: P,
    /// Its facing, millidegrees.
    pub face: i64,
    /// Its footprint in the world frame.
    pub footprint: Region,
}

/// Where a clear acts (4.3).
#[derive(Clone, Debug)]
pub enum ClearPlace {
    /// Origins within `radius` mm of a point.
    Disc(P, i64),
    /// Origins inside a region (outline included).
    Inside(Region),
}

/// A clear.
#[derive(Clone, Debug)]
pub struct Clear {
    /// Its id.
    pub id: String,
    /// Where.
    pub place: ClearPlace,
    /// The producers it speaks to; `None` for every producer.
    pub by: Option<Vec<String>>,
}

/// A pin.
#[derive(Clone, Debug)]
pub struct Pin {
    /// Its id, which names its instance.
    pub id: String,
    /// Its asset.
    pub asset: String,
    /// Where it stands.
    pub origin: P,
    /// Its footprint in the world frame.
    pub footprint: Region,
    /// The producers whose instances it removes; `None` for every producer.
    pub by: Option<Vec<String>>,
}

fn speaks_to(by: &Option<Vec<String>>, producer: &str) -> bool {
    by.as_ref().is_none_or(|b| b.iter().any(|p| p == producer))
}

fn overlaps(a: &Region, b: &Region) -> Result<bool, GeomDiag> {
    let i = boolean(BoolOp::Intersection, &[a.clone(), b.clone()])?;
    Ok(i.twice_area() > 0)
}

fn inside(place: &ClearPlace, p: P) -> bool {
    match place {
        ClearPlace::Disc(c, r) => {
            let (dx, dy) = (i128::from(p.x - c.x), i128::from(p.y - c.y));
            dx * dx + dy * dy <= i128::from(*r) * i128::from(*r)
        }
        ClearPlace::Inside(reg) => reg.side(p) != Side::Out,
    }
}

/// The held output: the producers' instances after clears and pins, and diagnostics.
pub fn hold(
    instances: Vec<Instance>,
    clears: &[Clear],
    pins: &[Pin],
) -> Result<(Vec<Instance>, Vec<GeomDiag>), GeomDiag> {
    let mut diags = Vec::new();
    let mut live = instances;
    let mut clears: Vec<&Clear> = clears.iter().collect();
    clears.sort_by(|a, b| a.id.cmp(&b.id));
    for c in clears {
        let before = live.len();
        live.retain(|i| !(speaks_to(&c.by, &i.producer) && inside(&c.place, i.origin)));
        if live.len() == before {
            diags.push(
                GeomDiag::new(
                    Code::ClearEmpty,
                    format!("the clear `{}` removed nothing", c.id),
                )
                .source(&c.id),
            );
        }
    }
    let mut pins: Vec<&Pin> = pins.iter().collect();
    pins.sort_by(|a, b| a.id.cmp(&b.id));
    for (i, a) in pins.iter().enumerate() {
        for b in &pins[i + 1..] {
            if overlaps(&a.footprint, &b.footprint)? {
                diags.push(
                    GeomDiag::new(
                        Code::PinsOverlap,
                        format!("pins `{}` and `{}` overlap", a.id, b.id),
                    )
                    .source(&a.id)
                    .at(a.origin)
                    .with("other", b.id.as_str()),
                );
            }
        }
    }
    for p in &pins {
        let mut removed: Vec<String> = Vec::new();
        let mut keep = Vec::with_capacity(live.len());
        for inst in live {
            if overlaps(&p.footprint, &inst.footprint)? {
                if speaks_to(&p.by, &inst.producer) {
                    removed.push(inst.name.clone());
                    continue;
                }
                diags.push(
                    GeomDiag::new(
                        Code::PinOverlap,
                        format!("the pin `{}` overlaps `{}`", p.id, inst.name),
                    )
                    .source(&p.id)
                    .at(p.origin)
                    .with("instance", inst.name.as_str()),
                );
            }
            keep.push(inst);
        }
        live = keep;
        if !removed.is_empty() {
            removed.sort();
            diags.push(
                GeomDiag::new(
                    Code::PinRemoved,
                    format!("the pin `{}` removed {}", p.id, removed.len()),
                )
                .source(&p.id)
                .at(p.origin)
                .with("count", removed.len() as i64)
                .with("names", Value::texts(removed)),
            );
        }
        // The pin's own instance is a written thing, in no producer's held output (4.2).
    }
    live.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    Ok((live, diags))
}
