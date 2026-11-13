//! The baseline a change report compares against (`build.md` 11.2): every build puts its
//! in-memory build into the store as one object, beside its manifest and files, and records which
//! object belongs to which manifest. The next build finds the baseline by the manifest digest
//! `.previous` names, or by the digest of the manifest in `--against DIR`.
//!
//! The object holds what the model's comparison reads (fields, features, footprints, instances,
//! asset facts, statements and their dependencies) and, for each statement key, the source path
//! and line that wrote it, so a cause found only in the baseline's text (a removed table) can
//! still name its line (`in: "baseline"`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use via_doc::World;
use via_graph::{Dec, Digest, Enc, Store};
use via_model::{Build, FeatureOut, FieldOut, FootprintOut, InstanceOut, Meeting, Tier};
use via_num::Grid;

const FORMAT: &str = "via-baseline-2";

/// Where a statement key was written: source path and line.
pub type Lines = BTreeMap<String, BTreeMap<String, (String, u32)>>;

/// A baseline: the build and where its statements were written.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub build: Build,
    pub lines: Lines,
    /// The layer of each feature and instance, as `feature.<name>` and `instance.<name>`.
    pub homes: BTreeMap<String, String>,
}

/// The layer of each written feature and instance of `world`.
pub fn homes_of(world: &World, build: &Build) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    for n in build.features.keys() {
        if let Some(h) = world.home("feature", n) {
            m.insert(format!("feature.{n}"), h.to_string());
        }
    }
    for n in build.instances.keys().chain(build.footprints.keys()) {
        if let Some(h) = world.home("instance", n).or_else(|| world.home("pin", n)) {
            m.insert(format!("instance.{n}"), h.to_string());
        }
    }
    m
}

/// Where each statement key of `build.statements` was written in `world`.
pub fn lines_of(world: &World, build: &Build) -> Lines {
    let mut out: Lines = BTreeMap::new();
    for (stmt, keys) in &build.statements {
        let mut m = BTreeMap::new();
        for k in keys.keys() {
            let entry = if stmt == "world" {
                world.root().get(k)
            } else if let Some((top, id)) = stmt.split_once('.') {
                if k.is_empty() {
                    world.entity_entry(top, id)
                } else {
                    world.entity(top, id).and_then(|t| t.get(k))
                }
            } else {
                None
            };
            if let Some(e) = entry {
                if let via_diag::Location::Span { file, line, .. } = world.sources().location(e.key)
                {
                    m.insert(k.clone(), (file, line));
                }
            }
        }
        if let Some((top, id)) = stmt.split_once('.') {
            if let Some(e) = world.entity_entry(top, id) {
                if let via_diag::Location::Span { file, line, .. } = world.sources().location(e.key)
                {
                    m.entry(String::new()).or_insert((file, line));
                }
            }
        }
        out.insert(stmt.clone(), m);
    }
    out
}

fn opt_str(e: &mut Enc, s: Option<&str>) {
    match s {
        Some(s) => {
            e.byte(1).str(s);
        }
        None => {
            e.byte(0);
        }
    }
}

fn pts(e: &mut Enc, p: &[(i64, i64)]) {
    e.len(p.len());
    for (x, y) in p {
        e.u64(*x as u64).u64(*y as u64);
    }
}

fn f32s(e: &mut Enc, v: &[f32]) {
    e.len(v.len());
    for x in v {
        e.u64(u64::from(x.to_bits()));
    }
}

fn str_map(e: &mut Enc, m: &BTreeMap<String, String>) {
    e.len(m.len());
    for (k, v) in m {
        e.str(k).str(v);
    }
}

/// The object's bytes.
pub fn encode(s: &Snapshot) -> Vec<u8> {
    let b = &s.build;
    let mut e = Enc::new();
    e.str(FORMAT);
    match b.grid {
        Some(g) => {
            e.byte(1);
            for v in g.extent {
                e.u64(v as u64);
            }
            e.u64(g.resolution as u64);
        }
        None => {
            e.byte(0);
        }
    }
    e.len(b.fields.len());
    for (n, f) in &b.fields {
        e.str(n);
        opt_str(&mut e, f.role.as_deref());
        opt_str(&mut e, f.unit.as_deref());
        e.len(f.tags.len());
        for t in &f.tags {
            e.str(t);
        }
        match &f.values {
            Some(v) => {
                e.byte(1);
                f32s(&mut e, v);
            }
            None => {
                e.byte(0);
            }
        }
        e.len(f.weights.len());
        for (m, v) in &f.weights {
            e.str(m);
            f32s(&mut e, v);
        }
    }
    e.len(b.features.len());
    for (n, f) in &b.features {
        e.str(n);
        opt_str(&mut e, f.kind.as_deref());
        e.byte(match f.tier {
            None => 0,
            Some(Tier::Derived) => 1,
            Some(Tier::Shaped) => 2,
            Some(Tier::Anchored) => 3,
        });
        e.str(&f.shape);
        pts(&mut e, &f.points);
        match f.facing {
            Some(v) => {
                e.byte(1).u64(v as u64);
            }
            None => {
                e.byte(0);
            }
        }
        str_map(&mut e, &f.attrs);
        e.len(f.names.len());
        for (a, (place, (x, y))) in &f.names {
            e.str(a).str(place).u64(*x as u64).u64(*y as u64);
        }
        opt_str(&mut e, f.output.as_deref());
        e.len(f.rings.len());
        for pc in &f.rings {
            e.len(pc.len());
            for r in pc {
                pts(&mut e, r);
            }
        }
    }
    e.len(b.footprints.len());
    for (n, f) in &b.footprints {
        e.str(n).str(&f.kind);
        pts(&mut e, &f.ring);
    }
    e.len(b.instances.len());
    for (n, i) in &b.instances {
        e.str(n);
        opt_str(&mut e, i.maker.as_deref());
        e.str(&i.asset)
            .u64(i.origin.0 as u64)
            .u64(i.origin.1 as u64)
            .u64(i.z.to_bits())
            .u64(i.facing as u64)
            .u64(i.scale.to_bits())
            .u64(i.tint.to_bits());
        pts(&mut e, &i.footprint);
    }
    e.len(b.meetings.len());
    for m in &b.meetings {
        e.str(&m.a)
            .str(&m.b)
            .u64(m.at.0 as u64)
            .u64(m.at.1 as u64)
            .u64(m.z_a.to_bits())
            .u64(m.z_b.to_bits())
            .u64(m.gap as u64);
    }
    e.len(b.assets.len());
    for (a, f) in &b.assets {
        e.str(a);
        str_map(&mut e, f);
    }
    e.len(b.statements.len());
    for (s, m) in &b.statements {
        e.str(s);
        str_map(&mut e, m);
    }
    e.len(b.deps.len());
    for ((t, n), set) in &b.deps {
        e.str(t).str(n).len(set.len());
        for x in set {
            e.str(x);
        }
    }
    e.len(s.lines.len());
    for (st, m) in &s.lines {
        e.str(st).len(m.len());
        for (k, (file, line)) in m {
            e.str(k).str(file).u64(u64::from(*line));
        }
    }
    str_map(&mut e, &s.homes);
    e.len(b.selections.len());
    for (who, (name, (x, y))) in &b.selections {
        e.str(who).str(name).u64(*x as u64).u64(*y as u64);
    }
    e.finish()
}

fn d_opt(d: &mut Dec) -> Option<Option<String>> {
    Some(match d.byte()? {
        1 => Some(d.str()?),
        _ => None,
    })
}

fn d_pts(d: &mut Dec) -> Option<Vec<(i64, i64)>> {
    let n = d.count()?;
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        v.push((d.u64()? as i64, d.u64()? as i64));
    }
    Some(v)
}

fn d_f32s(d: &mut Dec) -> Option<Vec<f32>> {
    let n = d.count()?;
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        v.push(f32::from_bits(u32::try_from(d.u64()?).ok()?));
    }
    Some(v)
}

fn d_map(d: &mut Dec) -> Option<BTreeMap<String, String>> {
    let mut m = BTreeMap::new();
    for _ in 0..d.count()? {
        let k = d.str()?;
        m.insert(k, d.str()?);
    }
    Some(m)
}

/// Reads the object back; `None` when it is not one.
pub fn decode(bytes: &[u8]) -> Option<Snapshot> {
    let mut d = Dec::new(bytes);
    if d.str()? != FORMAT {
        return None;
    }
    let mut b = Build::default();
    if d.byte()? == 1 {
        let mut ext = [0i64; 4];
        for v in &mut ext {
            *v = d.u64()? as i64;
        }
        b.grid = Grid::new(ext, d.u64()? as i64).ok();
    }
    for _ in 0..d.count()? {
        let n = d.str()?;
        let role = d_opt(&mut d)?;
        let unit = d_opt(&mut d)?;
        let mut tags = Vec::new();
        for _ in 0..d.count()? {
            tags.push(d.str()?);
        }
        let values = match d.byte()? {
            1 => Some(d_f32s(&mut d)?),
            _ => None,
        };
        let mut weights = BTreeMap::new();
        for _ in 0..d.count()? {
            let m = d.str()?;
            weights.insert(m, d_f32s(&mut d)?);
        }
        b.fields.insert(
            n,
            FieldOut {
                role,
                unit,
                tags,
                values,
                weights,
            },
        );
    }
    for _ in 0..d.count()? {
        let n = d.str()?;
        let kind = d_opt(&mut d)?;
        let tier = match d.byte()? {
            1 => Some(Tier::Derived),
            2 => Some(Tier::Shaped),
            3 => Some(Tier::Anchored),
            _ => None,
        };
        let shape = d.str()?;
        let points = d_pts(&mut d)?;
        let facing = match d.byte()? {
            1 => Some(d.u64()? as i64),
            _ => None,
        };
        let attrs = d_map(&mut d)?;
        let mut names = BTreeMap::new();
        for _ in 0..d.count()? {
            let a = d.str()?;
            let place = d.str()?;
            names.insert(a, (place, (d.u64()? as i64, d.u64()? as i64)));
        }
        let output = d_opt(&mut d)?;
        let mut rings = Vec::new();
        for _ in 0..d.count()? {
            let mut pc = Vec::new();
            for _ in 0..d.count()? {
                pc.push(d_pts(&mut d)?);
            }
            rings.push(pc);
        }
        b.features.insert(
            n,
            FeatureOut {
                kind,
                tier,
                shape,
                points,
                facing,
                attrs,
                names,
                output,
                rings,
            },
        );
    }
    for _ in 0..d.count()? {
        let n = d.str()?;
        let kind = d.str()?;
        let ring = d_pts(&mut d)?;
        b.footprints.insert(n, FootprintOut { kind, ring });
    }
    for _ in 0..d.count()? {
        let n = d.str()?;
        let maker = d_opt(&mut d)?;
        let asset = d.str()?;
        let origin = (d.u64()? as i64, d.u64()? as i64);
        let z = f64::from_bits(d.u64()?);
        let facing = d.u64()? as i64;
        let scale = f64::from_bits(d.u64()?);
        let tint = f64::from_bits(d.u64()?);
        let footprint = d_pts(&mut d)?;
        b.instances.insert(
            n,
            InstanceOut {
                maker,
                asset,
                origin,
                z,
                facing,
                scale,
                tint,
                footprint,
            },
        );
    }
    for _ in 0..d.count()? {
        b.meetings.push(Meeting {
            a: d.str()?,
            b: d.str()?,
            at: (d.u64()? as i64, d.u64()? as i64),
            z_a: f64::from_bits(d.u64()?),
            z_b: f64::from_bits(d.u64()?),
            gap: d.u64()? as i64,
        });
    }
    for _ in 0..d.count()? {
        let a = d.str()?;
        b.assets.insert(a, d_map(&mut d)?);
    }
    for _ in 0..d.count()? {
        let s = d.str()?;
        b.statements.insert(s, d_map(&mut d)?);
    }
    for _ in 0..d.count()? {
        let t = d.str()?;
        let n = d.str()?;
        let mut set = BTreeSet::new();
        for _ in 0..d.count()? {
            set.insert(d.str()?);
        }
        b.deps.insert((t, n), set);
    }
    let mut lines: Lines = BTreeMap::new();
    for _ in 0..d.count()? {
        let st = d.str()?;
        let mut m = BTreeMap::new();
        for _ in 0..d.count()? {
            let k = d.str()?;
            let f = d.str()?;
            m.insert(k, (f, u32::try_from(d.u64()?).ok()?));
        }
        lines.insert(st, m);
    }
    let homes = d_map(&mut d)?;
    for _ in 0..d.count()? {
        let who = d.str()?;
        let name = d.str()?;
        let at = (d.u64()? as i64, d.u64()? as i64);
        b.selections.insert(who, (name, at));
    }
    d.done().then_some(Snapshot {
        build: b,
        lines,
        homes,
    })
}

/// The file in the store that names a manifest's baseline object.
fn index_path(store: &Store, manifest: &Digest) -> PathBuf {
    store
        .root()
        .join("baselines")
        .join(manifest.fan())
        .join(manifest.hex())
}

/// Puts a baseline into the store under its manifest's digest.
pub fn put(store: &Store, manifest: &Digest, s: &Snapshot) {
    let obj = store.put_object(&encode(s));
    let p = index_path(store, manifest);
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(p, obj.hex());
}

/// The baseline of a manifest, when the store holds it.
pub fn get(store: &Store, manifest: &Digest) -> Option<Snapshot> {
    let hex = std::fs::read_to_string(index_path(store, manifest)).ok()?;
    let d = Digest::from_hex(hex.trim())?;
    decode(&store.get_object(&d)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let mut b = Build {
            grid: Grid::new([0, 0, 4_000, 2_000], 1_000).ok(),
            ..Build::default()
        };
        b.fields.insert(
            "height".into(),
            FieldOut {
                role: Some("surface".into()),
                unit: Some("m".into()),
                tags: vec![],
                values: Some(vec![1.5, -0.0, 3.0]),
                weights: BTreeMap::new(),
            },
        );
        b.instances.insert(
            "hut".into(),
            InstanceOut {
                maker: None,
                asset: "box_small".into(),
                origin: (-5, 7),
                z: 2.25,
                facing: 90_000,
                scale: 1.0,
                tint: 0.0,
                footprint: vec![(0, 0), (1, 1)],
            },
        );
        b.deps.insert(
            ("instance".into(), "hut".into()),
            BTreeSet::from(["world".to_string()]),
        );
        let s = Snapshot {
            build: b,
            lines: BTreeMap::from([(
                "world".to_string(),
                BTreeMap::from([("pack".to_string(), ("world.toml".to_string(), 4))]),
            )]),
            homes: BTreeMap::from([("instance.hut".to_string(), "world".to_string())]),
        };
        let back = decode(&encode(&s)).unwrap();
        assert_eq!(back.build.fields, s.build.fields);
        assert_eq!(back.build.instances, s.build.instances);
        assert_eq!(back.build.deps, s.build.deps);
        assert_eq!(back.lines, s.lines);
        assert_eq!(back.homes, s.homes);
        assert_eq!(back.build.grid, s.build.grid);
    }
}
