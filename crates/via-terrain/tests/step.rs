//! The terrain step through its program, as a plain command (`protocol.md` 2.2): a request on disk,
//! field files in a store, the answer read back from `out`. The same program serves the engine as
//! a worker; these tests need no engine, so they run by default.
//!
//! The world is a small basin written for these tests (not the spike's): land rising north from a
//! sea in the south, a hollow with a lake stated in it, and a river stated from the hills to the
//! sea. What must hold is the doctrine's, not a recording: the run is the same bytes at 1 and 8
//! threads; the water runs from the source to the sea near the stated mouth; a moved mouth moves the
//! course; a lake that cannot hold and a river that runs uphill are each reported with what would
//! hold instead; a good world draws no error.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use via_num as num;
use via_proto::npy;

use std::cell::Cell;

thread_local! {
    /// The grid of the test running on this thread: posts per side and their spacing in mm.
    static GRID: Cell<(usize, i64)> = const { Cell::new((201, 8000)) }; // 1600 m at 8 m
}

fn n() -> usize {
    GRID.with(|g| g.get().0)
}

fn res_mm() -> i64 {
    GRID.with(|g| g.get().1)
}

thread_local! {
    /// The ground of the test running on this thread; `None` for the default basin.
    static GROUND: std::cell::RefCell<Option<Vec<f64>>> = const { std::cell::RefCell::new(None) };
}

fn ground() -> Vec<f64> {
    if let Some(g) = GROUND.with(|g| g.borrow().clone()) {
        return g;
    }
    let mut g = vec![0.0; n() * n()];
    for j in 0..n() {
        for i in 0..n() {
            let (x, y) = (i as f64 * 8.0, j as f64 * 8.0);
            let hills = 6.0 * num::sin(x / 170.0) * num::sin(y / 130.0);
            let (dx, dy) = (x - 500.0, y - 1100.0);
            let hollow = 20.0 * num::exp(-(dx * dx + dy * dy) / (120.0 * 120.0));
            g[j * n() + i] = -12.0 + 0.03 * y + hills - hollow;
        }
    }
    g
}

struct Feat {
    name: &'static str,
    kind: &'static str,
    geometry: u8,
    ring: Vec<(i64, i64)>,
    attrs: Vec<(&'static str, f64)>,
}

/// A sixteen-sided ring about `c`, counter-clockwise, millimetres.
fn round(c: (i64, i64), r: i64) -> Vec<(i64, i64)> {
    (0..16)
        .map(|k| {
            let a = num::PI * 2.0 * k as f64 / 16.0;
            let mm = |v: f64| v.round_ties_even() as i64;
            (
                c.0 + mm(r as f64 * num::cos(a)),
                c.1 + mm(r as f64 * num::sin(a)),
            )
        })
        .collect()
}

fn lake(level: f64) -> Feat {
    Feat {
        name: "tarn",
        kind: "terrain_lake",
        geometry: 2,
        ring: round((500_000, 1_100_000), 60_000),
        attrs: vec![("level", level)],
    }
}

fn river(mouth: (i64, i64)) -> Feat {
    Feat {
        name: "river",
        kind: "terrain_river",
        geometry: 1,
        ring: vec![(1_200_000, 1_520_000), (1_100_000, 900_000), mouth],
        attrs: vec![],
    }
}

/// A feature list (`protocol.md` 4.3), inline, features in name order.
fn feature_list(fs: &mut [Feat]) -> Value {
    fs.sort_by_key(|f| f.name);
    let mut coords = Vec::new();
    let (mut pieces, mut rings, mut starts) = (vec![0i64], vec![0i64], vec![0i64]);
    let mut attr_names: Vec<&str> = fs
        .iter()
        .flat_map(|f| f.attrs.iter().map(|a| a.0))
        .collect();
    attr_names.sort();
    attr_names.dedup();
    for f in fs.iter() {
        for &(x, y) in &f.ring {
            coords.push(x);
            coords.push(y);
        }
        starts.push(starts.last().unwrap() + f.ring.len() as i64);
        rings.push(rings.last().unwrap() + 1);
        pieces.push(pieces.last().unwrap() + 1);
    }
    let mut list = json!({
        "coords": {"dtype": "i32", "shape": [coords.len() / 2, 2], "values": coords},
        "geometry": {"dtype": "u8", "values": fs.iter().map(|f| f.geometry).collect::<Vec<_>>()},
        "pieces": {"dtype": "i64", "values": pieces},
        "rings": {"dtype": "i64", "values": rings},
        "ring_starts": {"dtype": "i64", "values": starts},
        "name": {"dtype": "str", "values": fs.iter().map(|f| f.name).collect::<Vec<_>>()},
        "kind": {"dtype": "str", "values": fs.iter().map(|f| f.kind).collect::<Vec<_>>()},
        "authority": {"dtype": "str", "values": fs.iter().map(|_| "anchored").collect::<Vec<_>>()},
        "layer": {"dtype": "str", "values": fs.iter().map(|_| "facts").collect::<Vec<_>>()},
    });
    for a in attr_names {
        let vals: Vec<f64> = fs
            .iter()
            .map(|f| f.attrs.iter().find(|x| x.0 == a).map_or(0.0, |x| x.1))
            .collect();
        let has: Vec<u8> = fs
            .iter()
            .map(|f| u8::from(f.attrs.iter().any(|x| x.0 == a)))
            .collect();
        list[a] = json!({"dtype": "f64", "values": vals});
        list[format!("{a}?")] = json!({"dtype": "u8", "values": has});
    }
    list
}

struct Call {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Call {
    fn new() -> Call {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::create_dir_all(root.join("objects")).unwrap();
        std::fs::create_dir_all(root.join("out")).unwrap();
        Call { _dir: dir, root }
    }

    fn field(&self, name: &str, v: &[f64]) -> PathBuf {
        let f: Vec<f32> = v.iter().map(|&x| x as f32).collect();
        let p = self.root.join("objects").join(format!("{name}.npy"));
        std::fs::write(&p, npy::f32_file(&[n(), n()], &f)).unwrap();
        p
    }

    /// Runs the program on a request; returns `out`.
    fn run(&self, request: Value, threads: usize) -> PathBuf {
        let req = self.root.join("request.json");
        std::fs::write(&req, serde_json::to_vec(&request).unwrap()).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_via-terrain"))
            .env("VIA_STEP", &req)
            .env("RAYON_NUM_THREADS", threads.to_string())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "via-terrain failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        self.root.join("out")
    }
}

fn request(call: &Call, when: &str, fields: Value, features: Value, params: Value) -> Value {
    let span = (n() as i64 - 1) * res_mm();
    json!({
        "protocol": "via-step-1",
        "step": "terrain",
        "when": when,
        "for_each_item": null,
        "params": params,
        "grid": {"extent_mm": [0, 0, span, span], "resolution_mm": res_mm(), "nx": n(), "ny": n()},
        "seed": "00000000000000b1",
        "out": call.root.join("out").to_str().unwrap(),
        "objects": call.root.join("objects").to_str().unwrap(),
        "checks": null,
        "data": {"fields": fields, "features": features, "instances": {}, "inputs": []},
    })
}

fn field_entry(path: &Path, role: Option<&str>, tags: &[&str]) -> Value {
    json!({"type": "scalar", "role": role, "tags": tags, "unit": "m", "blob": "x",
           "path": path.to_str().unwrap(), "i0": 0, "j0": 0, "i1": n(), "j1": n()})
}

/// One generate call; its document, its fields by name, and every byte of `out`.
struct Generated {
    doc: toml::Table,
    fields: BTreeMap<String, Vec<f64>>,
    bytes: BTreeMap<String, Vec<u8>>,
}

fn generate(fs: &mut [Feat], threads: usize) -> Generated {
    let call = Call::new();
    let h = call.field("height", &ground());
    let w = call.field("water", &ground());
    let fields = json!({
        "height": field_entry(&h, Some("surface"), &[]),
        "water": field_entry(&w, None, &["water_level"]),
    });
    let out = call.run(
        request(&call, "generate", fields, feature_list(fs), json!({})),
        threads,
    );
    let mut bytes = BTreeMap::new();
    let mut names: Vec<_> = std::fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    names.sort();
    for p in names {
        bytes.insert(
            p.file_name().unwrap().to_string_lossy().into_owned(),
            std::fs::read(&p).unwrap(),
        );
    }
    assert!(
        !bytes.contains_key("diagnostics.json"),
        "{:?}",
        bytes.keys()
    );
    let doc: toml::Table = std::str::from_utf8(&bytes["document.toml"])
        .unwrap()
        .parse()
        .unwrap();
    let mut fields = BTreeMap::new();
    for name in ["eroded", "standing", "wet", "silt"] {
        let b = &bytes[&format!("{name}.npy")];
        let hd = npy::parse(b).unwrap();
        assert_eq!(hd.shape, vec![n(), n()]);
        let v: Vec<f64> = b[hd.data_offset..]
            .chunks_exact(4)
            .map(|c| f64::from(f32::from_le_bytes([c[0], c[1], c[2], c[3]])))
            .collect();
        fields.insert(name.to_string(), v);
    }
    Generated { doc, fields, bytes }
}

fn course(g: &Generated) -> Vec<(f64, f64)> {
    let f = g.doc["feature"]["river_course"].as_table().unwrap();
    assert_eq!(f["kind"].as_str(), Some("terrain_course"));
    assert_eq!(f["fact"].as_str(), Some("river"));
    assert!(f.get("at").is_none());
    let n = |v: &toml::Value| v.as_float().or(v.as_integer().map(|i| i as f64)).unwrap();
    let pt = |p: &toml::Value| {
        let p = p.as_array().unwrap();
        (n(&p[0]), n(&p[1]))
    };
    match f.get("path") {
        Some(path) => path.as_array().unwrap().iter().map(pt).collect(),
        None => vec![pt(&f["at"])],
    }
}

/// Distance from a point to the stated line.
fn off_line(p: (f64, f64), line: &[(f64, f64)]) -> f64 {
    let mut best = f64::INFINITY;
    for s in line.windows(2) {
        let (a, b) = (s[0], s[1]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let t = num::clamp(
            ((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy),
            0.0,
            1.0,
        );
        best = num::min(best, dist(p, (a.0 + t * dx, a.1 + t * dy)));
    }
    best
}

fn line_m(f: &Feat) -> Vec<(f64, f64)> {
    f.ring
        .iter()
        .map(|&(x, y)| (num::metres(x), num::metres(y)))
        .collect()
}

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0) * (a.0 - b.0) + (a.1 - b.1) * (a.1 - b.1)).sqrt()
}

/// The lint over a generate call's result, with the build made as the document says: the row on
/// the surface is `f('eroded')`, the row on water raises it to the standing water.
fn lint(fs: &mut [Feat], g: &Generated) -> Vec<Value> {
    let call = Call::new();
    let eroded = &g.fields["eroded"];
    let standing = &g.fields["standing"];
    let water: Vec<f64> = eroded
        .iter()
        .zip(standing)
        .map(|(&e, &s)| if s > 0.0 { e + s } else { e })
        .collect();
    let h = call.field("height", eroded);
    let w = call.field("water", &water);
    let fields = json!({
        "height": field_entry(&h, Some("surface"), &[]),
        "water": field_entry(&w, None, &["water_level"]),
    });
    let mm = |v: f64| (v * 1000.0).round_ties_even() as i64;
    let mut list = feature_list(fs);
    // The returned course, as the build names it, appended to the stated facts.
    let returned = g.doc.get("feature").and_then(|t| t.get("river_course"));
    if let Some(f) = returned.and_then(|f| f.as_table()) {
        let c = course(g);
        let mut cl = feature_list(&mut [Feat {
            name: "terrain#river_course",
            kind: "terrain_course",
            geometry: 1,
            ring: c.iter().map(|&(x, y)| (mm(x), mm(y))).collect(),
            attrs: vec![("catchment", f["catchment"].as_float().unwrap())],
        }]);
        cl["fact"] = json!({"dtype": "str", "values": ["river"]});
        cl["fact?"] = json!({"dtype": "u8", "values": [1]});
        list = join(list, cl);
    }
    let mut req = request(&call, "lint", fields, list, json!({}));
    req["checks"] = json!([{"step": "terrain", "facts": ["river", "tarn"]}]);
    let out = call.run(req, 1);
    match std::fs::read(out.join("diagnostics.json")) {
        Ok(b) => serde_json::from_slice::<Vec<Value>>(&b).unwrap(),
        Err(_) => vec![],
    }
}

/// Two feature lists in one: columns concatenated, offsets shifted, attributes padded.
fn join(a: Value, b: Value) -> Value {
    let rows = |l: &Value| l["name"]["values"].as_array().unwrap().len();
    let (na, nb) = (rows(&a), rows(&b));
    let vals = |l: &Value, k: &str| l[k]["values"].as_array().cloned();
    let shift = |v: Vec<Value>, by: i64| -> Vec<Value> {
        v.into_iter()
            .map(|x| json!(x.as_i64().unwrap() + by))
            .collect()
    };
    let mut out = a.clone();
    let coords_a = vals(&a, "coords").unwrap();
    let coords_b = vals(&b, "coords").unwrap();
    let v_a = coords_a.len() as i64 / 2;
    out["coords"] = json!({"dtype": "i32", "shape": [(coords_a.len() + coords_b.len()) / 2, 2],
                           "values": ([coords_a, coords_b].concat())});
    for (k, last) in [
        ("pieces", na as i64),
        ("rings", na as i64),
        ("ring_starts", v_a),
    ] {
        let mut x = vals(&a, k).unwrap();
        let y = shift(vals(&b, k).unwrap(), last);
        x.extend(y.into_iter().skip(1));
        out[k]["values"] = json!(x);
    }
    let keys: std::collections::BTreeSet<String> = a
        .as_object()
        .unwrap()
        .keys()
        .chain(b.as_object().unwrap().keys())
        .cloned()
        .collect();
    for k in keys {
        if ["coords", "pieces", "rings", "ring_starts"].contains(&k.as_str()) {
            continue;
        }
        let dtype = a[&k]["dtype"]
            .as_str()
            .or(b[&k]["dtype"].as_str())
            .unwrap()
            .to_string();
        let pad = |n: usize| -> Vec<Value> {
            match dtype.as_str() {
                "str" => vec![json!(""); n],
                _ => vec![json!(0); n],
            }
        };
        let x = vals(&a, &k).unwrap_or_else(|| pad(na));
        let y = vals(&b, &k).unwrap_or_else(|| pad(nb));
        out[&k] = json!({"dtype": dtype, "values": ([x, y].concat())});
    }
    out
}

fn codes(d: &[Value]) -> Vec<(String, String)> {
    d.iter()
        .map(|x| {
            (
                x["severity"].as_str().unwrap().to_string(),
                x["code"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

const MOUTH: (i64, i64) = (1_000_000, 120_000);

/// Same request, same bytes, at 1 and 8 threads (R26; the old determinism test's pattern).
#[test]
fn same_bytes_at_any_thread_count() {
    let a = generate(&mut [lake(7.0), river(MOUTH)], 1);
    let b = generate(&mut [lake(7.0), river(MOUTH)], 8);
    assert_eq!(a.bytes, b.bytes);
}

/// The water runs from the source to the sea near the stated mouth, the lake holds at its level,
/// and the facts lint finds nothing to say.
#[test]
fn stated_facts_hold_and_say_so() {
    let mut fs = vec![lake(7.0), river(MOUTH)];
    let g = generate(&mut fs, 4);
    let c = course(&g);
    let f = g.doc["feature"]["river_course"].as_table().unwrap();
    assert_eq!(f["ends"].as_str(), Some("sea"), "{c:?}");
    assert!(dist(c[0], (1200.0, 1520.0)) < 1.0);
    // It keeps to the stated line all the way to the sea.
    let line = line_m(&river(MOUTH));
    assert!(c.iter().all(|&p| off_line(p, &line) < 50.0), "{c:?}");
    // On a coastal plain this flat the water meets the sea some way up from the stated mouth.
    let end = *c.last().unwrap();
    assert!(end.1 < 600.0, "ends at {end:?}");
    // The rows the document writes, on the world's own names.
    let field = g.doc["field"].as_table().unwrap();
    assert_eq!(
        field["height"]["then"]["erosion"]["value"].as_str(),
        Some("= f('eroded')")
    );
    assert!(field["water"]["then"]["erosion"].is_table());
    for k in ["eroded", "standing", "wet", "silt"] {
        assert_eq!(field[k]["file"].as_str(), Some(format!("{k}.npy").as_str()));
        assert!(field[k]["outside"].is_integer());
    }
    // Water stands in the lake.
    let at = |x: f64, y: f64| {
        let (i, j) = ((x / 8.0) as usize, (y / 8.0) as usize);
        j * n() + i
    };
    assert!(g.fields["standing"][at(500.0, 1100.0)] > 1.0);
    let d = lint(&mut fs, &g);
    assert!(
        codes(&d).iter().all(|(s, _)| s != "error"),
        "{:#}",
        json!(d)
    );
}

/// The mouth moves 400 m east along the shore and nothing else is edited: the course follows.
#[test]
fn a_moved_mouth_moves_the_course() {
    let a = generate(&mut [lake(7.0), river(MOUTH)], 4);
    let b = generate(&mut [lake(7.0), river((1_400_000, 120_000))], 4);
    let (ca, cb) = (course(&a), course(&b));
    let moved = line_m(&river((1_400_000, 120_000)));
    assert!(cb.iter().all(|&p| off_line(p, &moved) < 50.0), "{cb:?}");
    let (ea, eb) = (*ca.last().unwrap(), *cb.last().unwrap());
    assert_eq!(
        b.doc["feature"]["river_course"]["ends"].as_str(),
        Some("sea")
    );
    assert!(eb.0 - ea.0 > 100.0, "{ea:?} to {eb:?}");
    assert_ne!(a.bytes["eroded.npy"], b.bytes["eroded.npy"]);
}

/// What cannot hold is said, with what would: a lake far above its hollow, a river from the sea
/// to the hills.
#[test]
fn what_cannot_hold_is_reported_with_what_would() {
    let mut up = river(MOUTH);
    up.ring.reverse();
    let mut fs = vec![lake(30.0), up];
    let g = generate(&mut fs, 4);
    let d = lint(&mut fs, &g);
    let c = codes(&d);
    assert!(
        c.contains(&("error".into(), "lake_perched".into())),
        "{:#}",
        json!(d)
    );
    assert!(
        c.contains(&("error".into(), "river_uphill".into())),
        "{c:?}"
    );
    let perched = d.iter().find(|x| x["code"] == "lake_perched").unwrap();
    assert_eq!(perched["subject"], "feature.tarn");
    assert!(perched["data"]["would_hold"].as_f64().unwrap() < 30.0);
    assert!(perched["explain"].is_string() && perched["position"].is_array());
}

// ---- the worker transport

use via_proto::diag::StepSite;
use via_proto::worker::{CancelToken, Deadlines, Outcome, RpcError, Services, Worker, WorkerSpec};

/// The engine's side for one worker call: fields by selector, features by kind or name, and the
/// diagnostics the step sends, recorded.
struct Engine {
    fields: Vec<(String, Option<String>, Vec<String>, PathBuf)>,
    features: Vec<Feat>,
    emitted: Vec<Value>,
    reads: Vec<String>,
}

fn feat_clone(f: &Feat) -> Feat {
    Feat {
        name: f.name,
        kind: f.kind,
        geometry: f.geometry,
        ring: f.ring.clone(),
        attrs: f.attrs.clone(),
    }
}

impl Services for Engine {
    fn handle(&mut self, method: &str, p: &Value) -> Result<Value, RpcError> {
        let bad = |m: &str| RpcError {
            code: -32602,
            message: m.into(),
            data: None,
        };
        match method {
            "build.field" => {
                let sel = p["fields"][0].as_str().unwrap_or("");
                self.reads.push(format!("field {sel} {}", p["view"]));
                let hit = self.fields.iter().find(|(_, role, tags, _)| {
                    sel.strip_prefix("role:")
                        .is_some_and(|r| role.as_deref() == Some(r))
                        || sel
                            .strip_prefix("tag:")
                            .is_some_and(|t| tags.iter().any(|x| x == t))
                });
                let (name, _, _, path) = hit.ok_or_else(|| bad("no field matches"))?;
                Ok(
                    json!({name: {"type": "scalar", "blob": "x", "path": path.to_str().unwrap(),
                                 "i0": 0, "j0": 0, "i1": n(), "j1": n()}}),
                )
            }
            "build.features" => {
                let sel = &p["select"];
                self.reads.push(format!("features {sel}"));
                let mut picked: Vec<Feat> = self
                    .features
                    .iter()
                    .filter(|f| match sel.as_str() {
                        Some(s) => s.strip_prefix("kind:") == Some(f.kind),
                        None => sel
                            .as_array()
                            .is_some_and(|a| a.iter().any(|n| n == f.name)),
                    })
                    .map(feat_clone)
                    .collect();
                Ok(feature_list(&mut picked))
            }
            "diag.emit" => {
                self.emitted.push(p.clone());
                Ok(Value::Null)
            }
            _ => Err(RpcError {
                code: -32601,
                message: format!("unknown method {method}"),
                data: None,
            }),
        }
    }
}

/// One request, two transports, the same bytes (R22): the worker's document and field files equal
/// the plain command's, and it reads the surface and water in their head views and the facts by
/// kind, nothing more.
#[test]
fn the_worker_returns_the_plain_commands_bytes() {
    let plain = generate(&mut [lake(7.0), river(MOUTH)], 2);

    let call = Call::new();
    let h = call.field("height", &ground());
    let w = call.field("water", &ground());
    let mut engine = Engine {
        fields: vec![
            ("height".into(), Some("surface".into()), vec![], h),
            ("water".into(), None, vec!["water_level".into()], w),
        ],
        features: vec![lake(7.0), river(MOUTH)],
        emitted: vec![],
        reads: vec![],
    };
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_via-terrain"));
    let mut worker = Worker::start(WorkerSpec {
        program: exe,
        args: vec![],
        cwd: call.root.clone(),
        sdk: None,
        objects: call.root.join("objects"),
        site: StepSite {
            id: "terrain".into(),
            file: "world.toml".into(),
            line: 1,
            column: 1,
            bytes: [0, 14],
        },
        when: "generate".into(),
        methods: ["build.field", "build.features", "diag.emit"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        deadlines: Deadlines::default(),
    })
    .unwrap();
    let mut req = request(&call, "generate", json!(null), json!(null), json!({}));
    req.as_object_mut().unwrap().remove("data");
    let outcome = worker
        .call(req, &mut engine, &CancelToken::default())
        .unwrap();
    assert_eq!(worker.shutdown(), None);
    let Outcome::Done(result) = outcome else {
        panic!("cancelled")
    };
    let doc = result["document"].as_str().unwrap();
    assert_eq!(doc.as_bytes(), plain.bytes["document.toml"].as_slice());
    for name in ["eroded", "standing", "wet", "silt"] {
        let f = format!("{name}.npy");
        assert_eq!(
            std::fs::read(call.root.join("out").join(&f)).unwrap(),
            plain.bytes[&f],
            "{f}"
        );
    }
    assert!(engine.emitted.is_empty());
    assert_eq!(
        engine.reads,
        [
            "field role:surface \"head\"",
            "field tag:water_level \"head\"",
            "features \"kind:terrain_lake\"",
            "features \"kind:terrain_river\"",
            "features \"kind:terrain_hold\"",
        ]
    );
}

// ---- the world of the conformance cases (spec/cases/derived/terrain_*), less its fbm ripple

fn case_world() {
    GRID.with(|g| g.set((161, 4000)));
    let mut g = vec![0.0; 161 * 161];
    for j in 0..161 {
        for i in 0..161 {
            let (x, y) = (i as f64 * 4.0, j as f64 * 4.0);
            let (dx, dy) = (x - 200.0, y - 560.0);
            g[j * 161 + i] = -4.0 + 0.03 * y - 20.0 * num::exp(-(dx * dx + dy * dy) / 6400.0);
        }
    }
    GROUND.with(|c| *c.borrow_mut() = Some(g));
}

fn case_facts(level: f64, reversed: bool) -> Vec<Feat> {
    let mut ring = vec![(520_000, 620_000), (480_000, 320_000), (440_000, 40_000)];
    if reversed {
        ring.reverse();
    }
    vec![
        Feat {
            name: "tarn",
            kind: "terrain_lake",
            geometry: 2,
            ring: round((200_000, 560_000), 60_000),
            attrs: vec![("level", level)],
        },
        Feat {
            name: "river",
            kind: "terrain_river",
            geometry: 1,
            ring,
            attrs: vec![],
        },
    ]
}

#[test]
fn the_case_world_holds() {
    case_world();
    let mut fs = case_facts(2.5, false);
    let g = generate(&mut fs, 4);
    assert_eq!(
        g.doc["feature"]["river_course"]["ends"].as_str(),
        Some("sea")
    );
    let d = lint(&mut fs, &g);
    assert!(
        codes(&d).iter().all(|(s, _)| s != "error"),
        "{:#}",
        json!(d)
    );
}

#[test]
fn the_case_world_cannot_hold() {
    case_world();
    let mut fs = case_facts(12.0, true);
    let g = generate(&mut fs, 4);
    let d = lint(&mut fs, &g);
    let c = codes(&d);
    assert!(
        c.contains(&("error".into(), "lake_perched".into())),
        "{c:?}"
    );
    let up = d.iter().find(|x| x["code"] == "river_uphill").unwrap();
    assert_eq!(up["position"], json!([520.0, 620.0]));
}
