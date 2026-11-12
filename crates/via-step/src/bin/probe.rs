//! The probe's step in Rust: reads `height` through `build.field` and maps its blob, returns its
//! negation as a field file, and makes one `expr.eval` call back. The Python probe step's `field`
//! mode, by the same door, so the engine cannot tell the two apart (R19b). With `mode = "spin"`
//! it computes without engine calls until cancelled, for the reader-thread tests.

use serde_json::json;
use via_proto::npy;
use via_proto::store::Store;
use via_step::{serve, take_stdio, Step, StepError, StepFn};

struct Probe;

fn raised(what: impl std::fmt::Display) -> StepError {
    StepError::Raised {
        exception: "ProbeError".into(),
        message: what.to_string(),
    }
}

impl StepFn for Probe {
    fn when(&self) -> &'static [&'static str] {
        &["generate"]
    }

    fn call(&mut self, step: &mut Step<'_>) -> Result<Option<String>, StepError> {
        if step.request["params"]["mode"] == "spin" {
            // A computation that makes no engine call: it sees a cancel only through the reader
            // thread, and an engine that dies ends it the same way (protocol.md 3.1).
            while !step.cancelled() {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            return Err(StepError::Cancelled);
        }
        println!("a println! from the Rust step reaches the log");
        let objects = step.request["objects"]
            .as_str()
            .ok_or_else(|| raised("no objects"))?;
        let out = step.request["out"]
            .as_str()
            .ok_or_else(|| raised("no out"))?
            .to_owned();
        let store = Store::open(objects).map_err(raised)?;
        let got = step.engine("build.field", json!({"fields": ["height"]}))?;
        let blob = got["height"]["blob"]
            .as_str()
            .ok_or_else(|| raised("no blob"))?;
        let map = store.map(blob).map_err(raised)?;
        let h = npy::parse(&map).map_err(|e| raised(e.0))?;
        let neg: Vec<f32> = map[h.data_offset..]
            .chunks_exact(4)
            .map(|c| -f32::from_le_bytes([c[0], c[1], c[2], c[3]]) + 0.0)
            .collect();
        std::fs::write(
            std::path::Path::new(&out).join("neg.npy"),
            npy::f32_file(&h.shape, &neg),
        )
        .map_err(raised)?;
        let col = step.engine(
            "expr.eval",
            json!({"formula": "x + y",
                   "x": {"dtype": "i32", "values": [0, 1000, 2500]},
                   "y": {"dtype": "i32", "values": [0, 0, 500]}}),
        )?;
        if col != json!({"dtype": "f64", "values": [0.0, 1.0, 3.0]}) {
            return Err(raised(format!("expr.eval gave {col}")));
        }
        Ok(Some("[field.neg]\nfile = \"neg.npy\"\n".into()))
    }
}

fn main() {
    let mut conn = match take_stdio() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("via-step-probe: cannot take the protocol descriptors: {e}");
            std::process::exit(1);
        }
    };
    std::process::exit(serve(&mut conn, &mut Probe));
}
