//! Ensembles, not runs (ADR 0008 D5): a benchmark statistic is reported
//! as a distribution over at least five seeds, never a single value. The
//! spike reported five-seed numbers from tooling that was never committed;
//! this module is that tooling, committed.

use serde_json::{Map, Value};

/// Collect per-seed Fabric values (as JSON) into `{mean, sd, n}` per
/// numeric field. Non-numeric and null fields are skipped; a field must
/// be numeric in every run to aggregate.
pub fn aggregate(runs: &[Value]) -> Value {
    let mut fields: Vec<String> = Vec::new();
    if let Some(Value::Object(first)) = runs.first() {
        for (k, v) in first {
            if v.is_number() {
                fields.push(k.clone());
            }
        }
    }
    let mut mean = Map::new();
    let mut sd = Map::new();
    for f in &fields {
        let vals: Vec<f64> = runs
            .iter()
            .filter_map(|r| r.get(f).and_then(Value::as_f64))
            .collect();
        if vals.len() != runs.len() {
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
    }
}
