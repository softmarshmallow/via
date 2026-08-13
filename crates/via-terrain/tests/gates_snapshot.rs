//! Golden-seed snapshot of the gates report. Review diffs; never
//! blind-accept (CONTRIBUTING).

use via_terrain::{run, TerrainConfig};

/// The fitted gate values pass through libm transcendentals whose last ulp
/// may differ across platforms; rounding to 10 significant digits keeps the
/// snapshot portable while still catching real drift. The kernel itself
/// carries one libm call (the climate EMA's `exp`, a per-run constant);
/// perturbing it by ±1 ulp was verified not to change any artifact hash —
/// integer routing decisions and f32/cm quantization absorb it.
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
