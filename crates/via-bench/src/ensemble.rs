//! Ensembles, not runs (ADR 0008 D5): a benchmark statistic is reported
//! as a distribution over at least five seeds, never a single value. The
//! spike reported five-seed numbers from tooling that was never committed;
//! this module is that tooling, committed.

use serde_json::{Map, Value};

/// Collect per-seed Fabric values (as JSON) into `{mean, sd, n_valid, n}`
/// per numeric field. A field NaN in any run serializes to null and cannot
/// honestly enter a mean; it is aggregated over the runs where it is
/// numeric, with the valid-run count reported per field so a dropped-seed
/// character is distinguishable from one never measured (ADR 0008 D5's
/// "or the full set" is carried by `per_seed` either way).
pub fn aggregate(runs: &[Value]) -> Value {
    // Fields are collected from every run, not the first: the first seed
    // may be the one where a character went null.
    let mut all_fields: Vec<String> = Vec::new();
    for r in runs {
        if let Value::Object(o) = r {
            for (k, v) in o {
                if v.is_number() && !all_fields.contains(k) {
                    all_fields.push(k.clone());
                }
            }
        }
    }
    let mut mean = Map::new();
    let mut sd = Map::new();
    let mut n_valid = Map::new();
    for f in &all_fields {
        let vals: Vec<f64> = runs
            .iter()
            .filter_map(|r| r.get(f).and_then(Value::as_f64))
            .collect();
        n_valid.insert(f.clone(), Value::from(vals.len()));
        if vals.is_empty() {
            continue;
        }
        let n = vals.len() as f64;
        let m = vals.iter().sum::<f64>() / n;
        let var = vals.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / n;
        mean.insert(f.clone(), json_num(m));
        sd.insert(f.clone(), json_num(var.sqrt()));
    }
    Value::Object(Map::from_iter([
        ("n".to_string(), Value::from(runs.len())),
        ("n_valid".to_string(), Value::Object(n_valid)),
        ("mean".to_string(), Value::Object(mean)),
        ("sd".to_string(), Value::Object(sd)),
    ]))
}

fn json_num(x: f64) -> Value {
    serde_json::Number::from_f64(x)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mean_and_sd_by_hand() {
        let runs = vec![
            json!({"a": 1.0, "b": 10.0, "label": "x"}),
            json!({"a": 3.0, "b": 10.0, "label": "y"}),
        ];
        let agg = aggregate(&runs);
        assert_eq!(agg["n"], 2);
        assert!((agg["mean"]["a"].as_f64().unwrap() - 2.0).abs() < 1e-12);
        assert!((agg["sd"]["a"].as_f64().unwrap() - 1.0).abs() < 1e-12);
        assert!((agg["sd"]["b"].as_f64().unwrap()).abs() < 1e-12);
        assert!(agg["mean"].get("label").is_none());
        assert_eq!(agg["n_valid"]["a"], 2);
    }

    #[test]
    fn null_fields_are_counted_not_hidden() {
        // A character NaN in one seed (serialized null): aggregated over
        // the valid runs, with the shortfall visible in n_valid — and
        // found even when the FIRST run is the null one.
        let runs = vec![
            json!({"a": null, "b": 1.0}),
            json!({"a": 4.0, "b": 3.0}),
            json!({"a": 6.0, "b": 5.0}),
        ];
        let agg = aggregate(&runs);
        assert_eq!(agg["n"], 3);
        assert_eq!(agg["n_valid"]["a"], 2);
        assert_eq!(agg["n_valid"]["b"], 3);
        assert!((agg["mean"]["a"].as_f64().unwrap() - 5.0).abs() < 1e-12);
        assert!((agg["mean"]["b"].as_f64().unwrap() - 3.0).abs() < 1e-12);
    }
}
