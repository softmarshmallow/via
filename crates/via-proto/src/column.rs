//! Columns in messages (`spec/protocol.md` 4.3): every dtype, inline or by blob. Every call is a
//! batch of columns (R23), so this is the whole payload of a call.
//!
//! A column is one of:
//!
//! ```json
//! {"dtype": "f64", "values": [0.5, 1.25]}
//! {"dtype": "i32", "shape": [4096, 2], "blob": "9f2c…", "path": "/…/objects/9f/9f2c…"}
//! {"dtype": "str", "values": ["beck", "ford"]}
//! {"dtype": "str", "blob": "<utf-8 bytes>", "offsets": "<i64 blob, n + 1 entries>"}
//! ```
//!
//! Inline values of a column with a `shape` of more than one dimension are flat, in C order (an
//! inline `i32` `(n, 2)` column is `[x0, y0, x1, y1, …]`), or nested lists of that shape; both
//! read the same.

use serde_json::{json, Value};

use crate::store::Store;

/// Above this many values a column must travel as a blob (R20: large tables never as JSON).
pub const INLINE_MAX: usize = 4096;

/// A column's values, by dtype. `Bool` holds 0 or 1 per row, one byte each (4.2).
#[derive(Clone, Debug, PartialEq)]
pub enum Column {
    U8(Vec<u8>),
    Bool(Vec<u8>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    Str(Vec<String>),
}

impl Column {
    /// The number of values (rows times the other dimensions).
    pub fn len(&self) -> usize {
        match self {
            Column::U8(v) | Column::Bool(v) => v.len(),
            Column::I32(v) => v.len(),
            Column::I64(v) => v.len(),
            Column::F32(v) => v.len(),
            Column::F64(v) => v.len(),
            Column::Str(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The dtype's name on the wire.
    pub fn dtype(&self) -> &'static str {
        match self {
            Column::U8(_) => "u8",
            Column::Bool(_) => "bool",
            Column::I32(_) => "i32",
            Column::I64(_) => "i64",
            Column::F32(_) => "f32",
            Column::F64(_) => "f64",
            Column::Str(_) => "str",
        }
    }

    /// The values as doubles, for a numeric column; `None` for strings.
    pub fn as_f64(&self) -> Option<Vec<f64>> {
        Some(match self {
            Column::U8(v) | Column::Bool(v) => v.iter().map(|x| f64::from(*x)).collect(),
            Column::I32(v) => v.iter().map(|x| f64::from(*x)).collect(),
            // An i64 beyond 2^53 is not a double exactly; such columns are never read as doubles.
            #[allow(clippy::cast_precision_loss)]
            Column::I64(v) => v.iter().map(|x| *x as f64).collect(),
            Column::F32(v) => v.iter().map(|x| f64::from(*x)).collect(),
            Column::F64(v) => v.clone(),
            Column::Str(_) => return None,
        })
    }

    /// The values as integers, for an integer column.
    pub fn as_i64(&self) -> Option<Vec<i64>> {
        Some(match self {
            Column::U8(v) | Column::Bool(v) => v.iter().map(|x| i64::from(*x)).collect(),
            Column::I32(v) => v.iter().map(|x| i64::from(*x)).collect(),
            Column::I64(v) => v.clone(),
            _ => return None,
        })
    }
}

/// A column with its shape: `[n]` for a plain column, `[n, k]` for `k` values per row.
#[derive(Clone, Debug, PartialEq)]
pub struct Shaped {
    pub shape: Vec<usize>,
    pub column: Column,
}

impl Shaped {
    /// A one-dimensional column.
    pub fn flat(column: Column) -> Shaped {
        Shaped {
            shape: vec![column.len()],
            column,
        }
    }

    /// The number of rows: the first dimension.
    pub fn rows(&self) -> usize {
        self.shape.first().copied().unwrap_or(0)
    }
}

/// A fault in a column, as the wire error it becomes (3.4).
#[derive(Debug, PartialEq)]
pub struct ColumnError {
    pub code: i64,
    pub message: String,
    /// The blob's size does not match the `dtype` and `shape` its message gives: the fault the
    /// engine reports itself as `protocol.blob_header` (4.2).
    pub header: bool,
}

fn invalid(message: String) -> ColumnError {
    ColumnError {
        code: -32602,
        message,
        header: false,
    }
}

fn blob_error(message: String) -> ColumnError {
    ColumnError {
        code: -32002,
        message,
        header: false,
    }
}

/// Reads a column of `name` from a message. Blob columns are mapped and checked: a size that does
/// not match `dtype` and `shape` is -32002; a non-finite value is -32004; `-0.0` reads as `+0.0`
/// (4.2), so a sender in another language need not normalise.
pub fn read(name: &str, v: &Value, store: &Store) -> Result<Column, ColumnError> {
    read_shaped(name, v, store).map(|s| s.column)
}

fn flatten<'a>(v: &'a Value, out: &mut Vec<&'a Value>) {
    match v {
        Value::Array(a) => {
            for x in a {
                flatten(x, out);
            }
        }
        x => out.push(x),
    }
}

fn shape_of(name: &str, v: &Value) -> Result<Option<Vec<usize>>, ColumnError> {
    let Some(s) = v.get("shape") else {
        return Ok(None);
    };
    s.as_array()
        .ok_or_else(|| invalid(format!("{name}: shape is not a list")))?
        .iter()
        .map(|d| d.as_u64().and_then(|d| usize::try_from(d).ok()))
        .collect::<Option<Vec<_>>>()
        .map(Some)
        .ok_or_else(|| invalid(format!("{name}: shape is not integers that fit in memory")))
}

fn product(name: &str, shape: &[usize]) -> Result<usize, ColumnError> {
    // The shape comes from the step, so its product is checked: an overflow is a shape no blob
    // can have, not a panic in the engine.
    shape
        .iter()
        .try_fold(1usize, |acc, &d| acc.checked_mul(d))
        .ok_or_else(|| invalid(format!("{name}: shape {shape:?} is too large")))
}

fn finite(name: &str, row: usize, x: f64) -> Result<f64, ColumnError> {
    if x.is_finite() {
        Ok(x + 0.0)
    } else {
        Err(ColumnError {
            code: -32004,
            message: format!("{name}: value {row} is not finite"),
            header: false,
        })
    }
}

/// [`read`] with the column's shape.
pub fn read_shaped(name: &str, v: &Value, store: &Store) -> Result<Shaped, ColumnError> {
    let dtype = v
        .get("dtype")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(format!("{name}: no dtype")))?;
    let shape = shape_of(name, v)?;
    if let Some(values) = v.get("values") {
        let mut flat = Vec::new();
        if !values.is_array() {
            return Err(invalid(format!("{name}: values is not a list")));
        }
        flatten(values, &mut flat);
        if flat.len() > INLINE_MAX {
            return Err(invalid(format!(
                "{name}: {} values inline; more than {INLINE_MAX} must be a blob",
                flat.len()
            )));
        }
        let shape = shape.unwrap_or_else(|| vec![flat.len()]);
        if product(name, &shape)? != flat.len() {
            return Err(invalid(format!(
                "{name}: {} values, shape {shape:?}",
                flat.len()
            )));
        }
        let bad = |what: &str| invalid(format!("{name}: not all values are {what}"));
        let column = match dtype {
            "u8" => Column::U8(
                flat.iter()
                    .map(|x| x.as_u64().and_then(|i| u8::try_from(i).ok()))
                    .collect::<Option<_>>()
                    .ok_or_else(|| bad("u8"))?,
            ),
            "bool" => Column::Bool(
                flat.iter()
                    .map(|x| match x {
                        Value::Bool(b) => Some(u8::from(*b)),
                        Value::Number(n) => n.as_u64().filter(|i| *i <= 1).map(|i| i as u8),
                        _ => None,
                    })
                    .collect::<Option<_>>()
                    .ok_or_else(|| bad("bool"))?,
            ),
            "i32" => Column::I32(
                flat.iter()
                    .map(|x| x.as_i64().and_then(|i| i32::try_from(i).ok()))
                    .collect::<Option<_>>()
                    .ok_or_else(|| bad("i32"))?,
            ),
            "i64" => Column::I64(
                flat.iter()
                    .map(|x| x.as_i64())
                    .collect::<Option<_>>()
                    .ok_or_else(|| bad("i64"))?,
            ),
            "f32" | "f64" => {
                let mut out = Vec::with_capacity(flat.len());
                for (row, x) in flat.iter().enumerate() {
                    let x = x.as_f64().ok_or_else(|| bad("numbers"))?;
                    out.push(finite(name, row, x)?);
                }
                if dtype == "f32" {
                    // Narrowed as the wire's f32 is meant: the sender wrote an f32's value.
                    #[allow(clippy::cast_possible_truncation)]
                    Column::F32(out.into_iter().map(|x| (x as f32) + 0.0).collect())
                } else {
                    Column::F64(out)
                }
            }
            "str" => Column::Str(
                flat.iter()
                    .map(|x| x.as_str().map(str::to_string))
                    .collect::<Option<_>>()
                    .ok_or_else(|| bad("strings"))?,
            ),
            other => return Err(invalid(format!("{name}: dtype {other} is not in 4.3"))),
        };
        return Ok(Shaped { shape, column });
    }
    let blob = v
        .get("blob")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(format!("{name}: neither values nor blob")))?;
    if dtype == "str" {
        let offs = v
            .get("offsets")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid(format!("{name}: a str blob needs offsets")))?;
        let bytes = store
            .map(blob)
            .map_err(|e| blob_error(format!("{name}: {e}")))?;
        let om = store
            .map(offs)
            .map_err(|e| blob_error(format!("{name}: offsets: {e}")))?;
        if om.len() % 8 != 0 || om.is_empty() {
            return Err(blob_error(format!(
                "{name}: offsets of {} bytes are not i64 entries",
                om.len()
            )));
        }
        let o: Vec<i64> = om
            .chunks_exact(8)
            .map(|c| i64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
            .collect();
        let mut out = Vec::with_capacity(o.len() - 1);
        for w in o.windows(2) {
            let (a, b) = (usize::try_from(w[0]), usize::try_from(w[1]));
            let (Ok(a), Ok(b)) = (a, b) else {
                return Err(blob_error(format!("{name}: a negative offset")));
            };
            if a > b || b > bytes.len() {
                return Err(blob_error(format!("{name}: offsets leave the blob")));
            }
            let s = std::str::from_utf8(&bytes[a..b])
                .map_err(|_| blob_error(format!("{name}: a string is not UTF-8")))?;
            out.push(s.to_string());
        }
        let n = out.len();
        return Ok(Shaped {
            shape: shape.unwrap_or_else(|| vec![n]),
            column: Column::Str(out),
        });
    }
    let shape = shape.ok_or_else(|| invalid(format!("{name}: a blob column needs shape")))?;
    let size: usize = match dtype {
        "u8" | "bool" => 1,
        "i32" | "f32" => 4,
        "i64" | "f64" => 8,
        other => return Err(invalid(format!("{name}: dtype {other} is not in 4.3"))),
    };
    let n = product(name, &shape)?;
    let want = n
        .checked_mul(size)
        .ok_or_else(|| invalid(format!("{name}: shape {shape:?} is too large")))?;
    let map = store
        .map(blob)
        .map_err(|e| blob_error(format!("{name}: {e}")))?;
    if map.len() != want {
        return Err(ColumnError {
            header: true,
            ..blob_error(format!(
                "{name}: blob has {} bytes, shape gives {want}",
                map.len()
            ))
        });
    }
    let column = match dtype {
        "u8" => Column::U8(map.to_vec()),
        "bool" => {
            if let Some(row) = map.iter().position(|b| *b > 1) {
                return Err(invalid(format!("{name}: value {row} is not 0 or 1")));
            }
            Column::Bool(map.to_vec())
        }
        "i32" => Column::I32(
            map.chunks_exact(4)
                .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect(),
        ),
        "i64" => Column::I64(
            map.chunks_exact(8)
                .map(|c| i64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
                .collect(),
        ),
        "f32" => {
            let mut out = Vec::with_capacity(n);
            for (row, c) in map.chunks_exact(4).enumerate() {
                let x = f32::from_le_bytes([c[0], c[1], c[2], c[3]]);
                finite(name, row, f64::from(x))?;
                out.push(x + 0.0);
            }
            Column::F32(out)
        }
        _ => {
            let mut out = Vec::with_capacity(n);
            for (row, c) in map.chunks_exact(8).enumerate() {
                let x = f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]);
                out.push(finite(name, row, x)?);
            }
            Column::F64(out)
        }
    };
    Ok(Shaped { shape, column })
}

/// Writes a one-dimensional column for a message: inline up to `INLINE_MAX` values, else as a
/// blob in the store.
pub fn write(col: &Column, store: &Store) -> std::io::Result<Value> {
    write_shaped(&Shaped::flat(col.clone()), store)
}

fn json_f64(x: f64) -> Value {
    serde_json::Number::from_f64(x + 0.0).map_or(Value::Null, Value::Number)
}

/// Writes a column with its shape. The shape is written whenever it has more than one dimension,
/// and on every blob.
pub fn write_shaped(col: &Shaped, store: &Store) -> std::io::Result<Value> {
    let n = col.column.len();
    let dtype = col.column.dtype();
    let shape_needed = col.shape.len() > 1;
    if n <= INLINE_MAX {
        let values: Vec<Value> = match &col.column {
            Column::U8(v) => v.iter().map(|x| json!(x)).collect(),
            Column::Bool(v) => v.iter().map(|x| json!(*x == 1)).collect(),
            Column::I32(v) => v.iter().map(|x| json!(x)).collect(),
            Column::I64(v) => v.iter().map(|x| json!(x)).collect(),
            Column::F32(v) => v.iter().map(|x| json_f64(f64::from(*x))).collect(),
            Column::F64(v) => v.iter().map(|x| json_f64(*x)).collect(),
            Column::Str(v) => v.iter().map(|x| json!(x)).collect(),
        };
        let mut m = json!({"dtype": dtype, "values": values});
        if shape_needed {
            m["shape"] = json!(col.shape);
        }
        return Ok(m);
    }
    if let Column::Str(v) = &col.column {
        let mut bytes = Vec::new();
        let mut offs: Vec<u8> = 0i64.to_le_bytes().to_vec();
        for s in v {
            bytes.extend_from_slice(s.as_bytes());
            offs.extend_from_slice(&i64::try_from(bytes.len()).unwrap_or(i64::MAX).to_le_bytes());
        }
        let b = store.put(&bytes)?;
        let o = store.put(&offs)?;
        return Ok(json!({"dtype": "str", "blob": b, "offsets": o}));
    }
    let bytes: Vec<u8> = match &col.column {
        Column::U8(v) | Column::Bool(v) => v.clone(),
        Column::I32(v) => v.iter().flat_map(|x| x.to_le_bytes()).collect(),
        Column::I64(v) => v.iter().flat_map(|x| x.to_le_bytes()).collect(),
        Column::F32(v) => v.iter().flat_map(|x| (x + 0.0).to_le_bytes()).collect(),
        Column::F64(v) => v.iter().flat_map(|x| (x + 0.0).to_le_bytes()).collect(),
        Column::Str(_) => Vec::new(),
    };
    let name = store.put(&bytes)?;
    Ok(json!({
        "dtype": dtype,
        "shape": col.shape,
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
    fn every_dtype_round_trips_inline_and_as_a_blob() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path().join("objects")).unwrap();
        for n in [3usize, 5000] {
            let cols = [
                Column::U8((0..n).map(|i| (i % 251) as u8).collect()),
                Column::Bool((0..n).map(|i| (i % 2) as u8).collect()),
                Column::I32((0..n).map(|i| i as i32 - 7).collect()),
                Column::I64((0..n).map(|i| (i as i64) << 40).collect()),
                Column::F32((0..n).map(|i| i as f32 * 0.25).collect()),
                Column::F64((0..n).map(|i| i as f64 / 3.0).collect()),
                Column::Str((0..n).map(|i| format!("é{i}")).collect()),
            ];
            for c in cols {
                let v = write(&c, &store).unwrap();
                assert_eq!(v.get("blob").is_some(), n > INLINE_MAX, "{}", c.dtype());
                assert_eq!(read("x", &v, &store).unwrap(), c, "{}", c.dtype());
            }
        }
        // (n, 2) inline, flat or nested.
        let flat = json!({"dtype": "i32", "shape": [2, 2], "values": [1, 2, 3, 4]});
        let nested = json!({"dtype": "i32", "shape": [2, 2], "values": [[1, 2], [3, 4]]});
        let a = read_shaped("c", &flat, &store).unwrap();
        assert_eq!(a, read_shaped("c", &nested, &store).unwrap());
        assert_eq!(a.shape, vec![2, 2]);
        assert_eq!(a.rows(), 2);
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
        let v = json!({"dtype": "i32", "shape": [3], "values": [1, 2]});
        assert_eq!(read("x", &v, &store).unwrap_err().code, -32602);
        // A shape whose size overflows is refused as a value, not a panic.
        let huge = 1u64 << 40;
        let v = json!({"dtype": "f64", "shape": [huge, huge], "blob": nan});
        assert_eq!(read("x", &v, &store).unwrap_err().code, -32602);
        let v = json!({"dtype": "f64", "shape": [u64::MAX], "blob": nan});
        assert_eq!(read("x", &v, &store).unwrap_err().code, -32602);
        // A blob whose bytes do not match its name.
        let good = store.put(&1.0f64.to_le_bytes()).unwrap();
        std::fs::write(store.path_of(&good), 2.0f64.to_le_bytes()).unwrap();
        let fresh = Store::open(dir.path().join("objects")).unwrap();
        let v = json!({"dtype": "f64", "shape": [1], "blob": good});
        assert_eq!(read("x", &v, &fresh).unwrap_err().code, -32002);
    }
}
