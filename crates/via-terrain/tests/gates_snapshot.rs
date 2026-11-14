//! Golden-seed snapshot of the gates report. Review diffs; never
//! blind-accept (CONTRIBUTING).

use via_terrain::{run, TerrainConfig};

/// The fitted gate values are rounded to 10 significant digits so the snapshot reads as the
/// science it records, not its last bits. The bits themselves are held elsewhere: every float
/// operation of the model goes through `via-num` (R26), so a run is the same bits at any thread
/// count (tests/determinism.rs) and on every system; the M2 port to `via-num` moved one value,
/// `sediment_mass_closure` (0 to 1.5e-16, a summation residual, inside its 1e-9 gate).
fn round_floats(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Number(n) => {
            if n.is_f64() {
                let f = n.as_f64().unwrap();
                let rounded: f64 = format!("{f:.9e}").parse().unwrap();
                *v = serde_json::Value::from(rounded);
            }
        }
        serde_json::Value::Array(a) => a.iter_mut().for_each(round_floats),
        serde_json::Value::Object(o) => o.values_mut().for_each(round_floats),
        _ => {}
    }
}

#[test]
fn gates_snapshot_seed7() {
    let cfg = TerrainConfig {
        seed: 7,
        size: 128,
        steps: 60,
        ..TerrainConfig::default()
    };
    let out = run(&cfg, &mut |_, _| {});
    let mut gates = serde_json::to_value(&out.gates).unwrap();
    round_floats(&mut gates);
    insta::assert_json_snapshot!("gates_seed7_128", gates);
}
