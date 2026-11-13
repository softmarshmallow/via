//! The `via kernel` files (`spec/geometry.md` 7) through `via_geom::kernel::run`, and once
//! through the `kernel` example binary, as the conformance runner will call it.

use via_geom::json::{parse, Json};
use via_geom::kernel::run;

const CROSS: &str = r#"{"@format":"via-kernel-1","grid":{"extent":[0,0,100000,100000],"resolution":500},
"shapes":[{"source":"frame_a","ring":[[0,0],[100000,0],[100000,100000],[0,100000]]},
{"source":"cross_ew","path":[[0,50000],[100000,50000]]},{"source":"cross_ns","path":[[50000,0],[50000,100000]]}]}"#;

const CROSS_SPACED: &str = r#"{ "shapes" : [
  { "path" : [ [50000, 0], [50000, 100000] ], "source" : "cross_ns" },
  { "source" : "cross_ew", "path" : [ [0, 50000], [100000, 50000] ] },
  { "ring" : [ [0, 0], [100000, 0], [100000, 100000], [0, 100000] ], "source" : "frame_a" } ],
  "grid" : { "resolution" : 500, "extent" : [0, 0, 100000, 100000] },
  "@format" : "via-kernel-1" }"#;

#[test]
fn usage_errors() {
    let o = run("frobnicate", CROSS, 3);
    assert_eq!((o.status, o.output.is_none()), (2, true));
    let o = run("network", &CROSS.replace("via-kernel-1", "via-kernel-0"), 3);
    assert_eq!((o.status, o.usage.as_deref()), (2, Some("@format")));
}

#[test]
fn cross_twice_same_bytes() {
    let a = run("network", CROSS, 3);
    let b = run("network", CROSS_SPACED, 3);
    assert_eq!(a.status, 0);
    assert_eq!(a.output, b.output);
    let out = a.output.unwrap();
    assert!(out.ends_with("}\n") && !out.contains(' '));
    let j = parse(&out).unwrap();
    assert_eq!(j.get("areas").unwrap().arr().unwrap().len(), 4);
    assert_eq!(j.get("components").unwrap().int(), Some(1));
    // Re-writing the parsed output gives the same bytes: canonical.
    assert_eq!(j.to_text(), out);
}

#[test]
fn errors_exit_one() {
    let bow = r#"{"@format":"via-kernel-1","shapes":[{"source":"b","ring":[[0,0],[10000,10000],[10000,0],[0,10000]]}]}"#;
    let o = run("read", bow, 3);
    assert_eq!(o.status, 1);
    let j = parse(o.output.as_deref().unwrap()).unwrap();
    let d = &j.get("diagnostics").unwrap().arr().unwrap()[0];
    assert_eq!(
        d.get("code").and_then(Json::str),
        Some("geom.self_crossing")
    );
    assert!(j.get("shape").is_none());
    let dup = r#"{"@format":"via-kernel-1","shapes":[{"source":"a","path":[[0,0],[1,1]]},{"source":"a","path":[[0,1],[1,0]]}]}"#;
    assert_eq!(run("noding", dup, 3).status, 1);
}

/// Case 48 through the command: a clear on one cell removes exactly its instance.
#[test]
fn names_clear_one_cell() {
    let base = parse(CROSS).unwrap();
    let mut input = match base {
        Json::Obj(m) => m,
        _ => unreachable!(),
    };
    let params = parse(
        r#"{"of":"cells","producer":"maker_a","area":[0,0],"cell_length":10000,"band_depth":10000,
        "min_cell_length":5000,"min_cell_area":40000000,
        "frontage":[{"source":"cross_ns","edge":[[50000,0],[50000,50000]],"side":"left"},
                    {"source":"cross_ew","edge":[[0,50000],[50000,50000]],"side":"right"}]}"#,
    )
    .unwrap();
    input.insert("params".into(), params);
    let first = run("names", &Json::Obj(input.clone()).to_text(), 3);
    assert_eq!(first.status, 0, "{:?}", first.output);
    let out = parse(first.output.as_deref().unwrap()).unwrap();
    let cells = out.get("cells").unwrap().arr().unwrap();
    assert_eq!(cells.len(), 10);
    // One instance per cell at its point, the cell as its footprint.
    let insts: Vec<Json> = cells
        .iter()
        .map(|c| {
            Json::obj(vec![
                (
                    "name",
                    Json::s(&format!(
                        "homes#{}",
                        c.get("name")
                            .unwrap()
                            .str()
                            .unwrap()
                            .split('#')
                            .nth(1)
                            .unwrap()
                    )),
                ),
                ("producer", Json::s("homes")),
                ("asset", Json::s("house")),
                ("origin", c.get("point").unwrap().clone()),
                ("face", Json::from_i(0)),
                ("footprint", c.get("region").unwrap().clone()),
            ])
        })
        .collect();
    input.insert("instances".into(), Json::Arr(insts));
    // The station 2 cell of the north-running side lies at x 40..50, y 20..30: near (45, 25).
    let edits =
        parse(r#"{"clear":{"one":{"on":{"of":"maker_a","class":"cell","near":[45.0,25.0]}}}}"#)
            .unwrap();
    input.insert("edits".into(), edits);
    let o = run("names", &Json::Obj(input).to_text(), 3);
    assert_eq!(o.status, 0, "{:?}", o.output);
    let out = parse(o.output.as_deref().unwrap()).unwrap();
    assert_eq!(out.get("held").unwrap().arr().unwrap().len(), 9);
}

#[test]
fn example_binary() {
    let dir = std::env::temp_dir().join(format!("via-geom-kernel-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let inp = dir.join("in.json");
    let out = dir.join("out.json");
    std::fs::write(&inp, CROSS).unwrap();
    let exe = env!("CARGO_BIN_EXE_kernel_cmd");
    let st = std::process::Command::new(exe)
        .args([
            "network",
            "--in",
            inp.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert_eq!(st.code(), Some(0));
    assert_eq!(
        std::fs::read_to_string(&out).unwrap(),
        run("network", CROSS, 3).output.unwrap()
    );
    let _ = std::fs::remove_dir_all(&dir);
}
