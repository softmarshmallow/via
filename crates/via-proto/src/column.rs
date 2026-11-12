//! Columns in messages (`spec/protocol.md` 4.3), the probe's subset: numeric dtypes, inline or by
//! blob. Every call is a batch of columns (R23), so this is the whole payload of a call.

use serde_json::{json, Value};

use crate::store::Store;

/// Above this many values a column must travel as a blob (R20: large tables never as JSON).
pub const INLINE_MAX: usize = 4096;

#[derive(Clone, Debug, PartialEq)]
pub enum Column {
    I32(Vec<i32>),
    F64(Vec<f64>),
}

impl Column {
    pub fn len(&self) -> usize {
        match self {
            Column::I32(v) => v.len(),
            Column::F64(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A fault in a column, as the wire error it becomes (3.4).
#[derive(Debug, PartialEq)]
pub struct ColumnError {
    pub code: i64,
    pub message: String,
}

fn invalid(message: String) -> ColumnError {
    ColumnError {
        code: -32602,
        message,
    }
}

/// Reads a column of `name` from a message. Blob columns are mapped and checked: a size that does
/// not match `dtype` and `shape` is -32002; a non-finite value is -32004; `-0.0` reads as `+0.0`
/// (4.2), so a sender in another language need not normalise.
pub fn read(name: &str, v: &Value, store: &Store) -> Result<Column, ColumnError> {
    let dtype = v
        .get("dtype")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(format!("{name}: no dtype")))?;
    if let Some(values) = v.get("values") {
        let values = values
            .as_array()
            .ok_or_else(|| invalid(format!("{name}: values is not a list")))?;
        if values.len() > INLINE_MAX {
            return Err(invalid(format!(
                "{name}: {} values inline; more than {INLINE_MAX} must be a blob",
                values.len()
            )));
        }
        return match dtype {
            "i32" => values
                .iter()
                .map(|x| x.as_i64().and_then(|i| i32::try_from(i).ok()))
                .collect::<Option<Vec<_>>>()
                .map(Column::I32)
                .ok_or_else(|| invalid(format!("{name}: not all values are i32"))),
            "f64" => values
                .iter()
                .map(Value::as_f64)
                .collect::<Option<Vec<_>>>()
                .map(|v| Column::F64(v.into_iter().map(|x| x + 0.0).collect()))
                .ok_or_else(|| invalid(format!("{name}: not all values are numbers"))),
            other => Err(invalid(format!(
                "{name}: dtype {other} is not in the probe"
            ))),
        };
    }
    let blob = v
        .get("blob")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(format!("{name}: neither values nor blob")))?;
    let shape: Vec<usize> = v
        .get("shape")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid(format!("{name}: a blob column needs shape")))?
        .iter()
        .map(|d| d.as_u64().and_then(|d| usize::try_from(d).ok()))
        .collect::<Option<_>>()
        .ok_or_else(|| invalid(format!("{name}: shape is not integers that fit in memory")))?;
    let size: usize = match dtype {
        "i32" => 4,
        "f64" => 8,
        other => {
            return Err(invalid(format!(
                "{name}: dtype {other} is not in the probe"
            )))
        }
    };
    // The shape comes from the step, so its product is checked: an overflow is a shape no blob
    // can have, not a panic in the engine.
    let n = shape
        .iter()
        .try_fold(1usize, |acc, &d| acc.checked_mul(d))
        .ok_or_else(|| invalid(format!("{name}: shape {shape:?} is too large")))?;
    let want = n
        .checked_mul(size)
        .ok_or_else(|| invalid(format!("{name}: shape {shape:?} is too large")))?;
    let map = store.map(blob).map_err(|e| ColumnError {
        code: -32002,
        message: format!("{name}: {e}"),
    })?;
    if map.len() != want {
        return Err(ColumnError {
            code: -32002,
            message: format!("{name}: blob has {} bytes, shape gives {want}", map.len()),
        });
    }
    Ok(match dtype {
        "i32" => Column::I32(
            map.chunks_exact(4)
                .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect(),
        ),
        _ => {
            let mut out = Vec::with_capacity(n);
            for (row, c) in map.chunks_exact(8).enumerate() {
                let x = f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]);
                if !x.is_finite() {
                    return Err(ColumnError {
                        code: -32004,
                        message: format!("{name}: row {row} is not finite"),
                    });
                }
                out.push(x + 0.0);
            }
            Column::F64(out)
        }
    })
}

/// Writes a column for a message: inline up to `INLINE_MAX` values, else as a blob in the store.
pub fn write(col: &Column, store: &Store) -> std::io::Result<Value> {
    let (dtype, n) = match col {
        Column::I32(v) => ("i32", v.len()),
        Column::F64(v) => ("f64", v.len()),
    };
    if n <= INLINE_MAX {
        return Ok(match col {
            Column::I32(v) => json!({"dtype": dtype, "values": v}),
            Column::F64(v) => json!({"dtype": dtype, "values": v}),
        });
    }
    let bytes: Vec<u8> = match col {
        Column::I32(v) => v.iter().flat_map(|x| x.to_le_bytes()).collect(),
        Column::F64(v) => v.iter().flat_map(|x| (x + 0.0).to_le_bytes()).collect(),
    };
    let name = store.put(&bytes)?;
    Ok(json!({
        "dtype": dtype,
        "shape": [n],
        "blob": name,
        "path": crate::worker::wire_path(&store.path_of(&name)),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_and_blob_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path().join("objects")).unwrap();
        let small = Column::F64(vec![0.5, -0.0, 1.25]);
        let v = write(&small, &store).unwrap();
        assert!(v.get("values").is_some());
        let back = read("x", &v, &store).unwrap();
        assert_eq!(back, Column::F64(vec![0.5, 0.0, 1.25]));
        if let Column::F64(b) = back {
            assert!(b[1].is_sign_positive(), "-0.0 reads as +0.0");
        }
        let big = Column::I32((0..5000).collect());
        let v = write(&big, &store).unwrap();
        assert!(v.get("blob").is_some());
        assert_eq!(read("x", &v, &store).unwrap(), big);
    }

    #[test]
    fn faults_are_wire_errors() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path().join("objects")).unwrap();
        let nan = store.put(&f64::NAN.to_le_bytes()).unwrap();
        let v = json!({"dtype": "f64", "shape": [1], "blob": nan});
        assert_eq!(read("x", &v, &store).unwrap_err().code, -32004);
        let v = json!({"dtype": "f64", "shape": [2], "blob": nan});
        assert_eq!(read("x", &v, &store).unwrap_err().code, -32002);
        let v = json!({"dtype": "i32", "values": [1.5]});
        assert_eq!(read("x", &v, &store).unwrap_err().code, -32602);
        // A shape whose size overflows is refused as a value, not a panic.
        let huge = 1u64 << 40;
        let v = json!({"dtype": "f64", "shape": [huge, huge], "blob": nan});
        assert_eq!(read("x", &v, &store).unwrap_err().code, -32602);
        let v = json!({"dtype": "f64", "shape": [u64::MAX], "blob": nan});
        assert_eq!(read("x", &v, &store).unwrap_err().code, -32602);
    }
}
