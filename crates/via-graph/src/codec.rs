//! Values and their diagnostics in the store (`spec/graph.md` 3.2 and 3.4).
//!
//! A node's diagnostics are part of its value and its value digest. A value with none is stored
//! as its bytes, so a build file's object is the file itself; a value with some is stored as
//! `"via-value-1"`, its bytes and its diagnostics, in the encoding of 3.1. The key record holds
//! the value digest and the diagnostics again, so a node kept "digest only" replays them.
//!
//! A stored diagnostic holds its subject and site, never a file or line: [`Diag`] holds none.

use std::sync::Arc;

use via_diag::{registry, Diag, DiagCode, KeyPart, KeyPath, Position, Site, Subject, Text, Value};

use crate::digest::{Dec, Digest, Enc};

/// How a node's diagnostics are written into the store and read back.
///
/// `encode` returns `None` when it cannot write the diagnostics so that they read back equal;
/// the node's value is then used in this build but not stored, so it runs again next build and
/// a clean build still equals an incremental one.
pub trait DiagCodec: Sync {
    /// The diagnostics as bytes, or `None` when they would not read back equal.
    fn encode(&self, diags: &[Diag]) -> Option<Vec<u8>>;
    /// The diagnostics from bytes; `None` when the bytes are not an encoding.
    fn decode(&self, bytes: &[u8]) -> Option<Vec<Diag>>;
}

/// The codec for diagnostics with core codes. A step's own code (`step.<id>.<name>`), `related`
/// entries and a stated `count` are not readable outside `via-diag`, so a diagnostic with any of
/// them is refused (see [`DiagCodec::encode`]); the model crate supplies a codec that knows them.
pub struct CoreCodec;

impl DiagCodec for CoreCodec {
    fn encode(&self, diags: &[Diag]) -> Option<Vec<u8>> {
        let mut e = Enc::new();
        e.str("via-diags-1").len(diags.len());
        for d in diags {
            encode_diag(&mut e, d)?;
        }
        let bytes = e.finish();
        (self.decode(&bytes).as_deref() == Some(diags)).then_some(bytes)
    }

    fn decode(&self, bytes: &[u8]) -> Option<Vec<Diag>> {
        let mut d = Dec::new(bytes);
        if d.str()? != "via-diags-1" {
            return None;
        }
        let n = d.count()?;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            out.push(decode_diag(&mut d)?);
        }
        d.done().then_some(out)
    }
}

fn encode_diag(e: &mut Enc, d: &Diag) -> Option<()> {
    let code = match d.code() {
        DiagCode::Core(c) => c.as_str(),
        DiagCode::Step { .. } => return None,
    };
    e.str(code).str(&d.subject().to_string());
    encode_site(e, d.site());
    e.str(d.message().as_str()).str(d.hint().as_str());
    match d.position() {
        None => {
            e.byte(0);
        }
        Some(p) => {
            e.byte(1).u64(p.x_mm() as u64).u64(p.y_mm() as u64);
            match p.z() {
                None => e.byte(0),
                Some(z) => e.byte(1).u64(z.get().to_bits()),
            };
        }
    }
    e.len(d.data().len());
    for (k, v) in d.data() {
        e.str(k);
        encode_value(e, v);
    }
    match d.produced_name() {
        None => e.byte(0),
        Some(n) => e.byte(1).str(n),
    };
    Some(())
}

fn decode_diag(d: &mut Dec) -> Option<Diag> {
    let code = registry::lookup(&d.str()?)?;
    let subject = Subject::parse(&d.str()?).ok()?;
    let site = decode_site(d)?;
    let message = Text::new(d.str()?).ok()?;
    let hint = Text::new(d.str()?).ok()?;
    let mut diag = Diag::new(code, subject, site, message, hint);
    if d.byte()? == 1 {
        let x = d.u64()? as i64;
        let y = d.u64()? as i64;
        let mut p = Position::plan(x, y);
        if d.byte()? == 1 {
            p = p.with_z(via_diag::Finite::new(f64::from_bits(d.u64()?))?);
        }
        diag = diag.at(p);
    }
    for _ in 0..d.count()? {
        let k = d.str()?;
        diag = diag.with(k, decode_value(d, 0)?);
    }
    if d.byte()? == 1 {
        diag = diag.produced(d.str()?);
    }
    Some(diag)
}

fn encode_path(e: &mut Enc, p: &KeyPath) {
    e.len(p.parts().len());
    for part in p.parts() {
        match part {
            KeyPart::Key(k) => e.byte(0).str(k),
            KeyPart::Index(i) => e.byte(1).u64(u64::from(*i)),
        };
    }
}

fn decode_path(d: &mut Dec) -> Option<KeyPath> {
    let n = d.count()?;
    let mut parts = Vec::with_capacity(n);
    for _ in 0..n {
        parts.push(match d.byte()? {
            0 => KeyPart::Key(d.str()?),
            1 => KeyPart::Index(u32::try_from(d.u64()?).ok()?),
            _ => return None,
        });
    }
    KeyPath::from_parts(parts)
}

fn encode_site(e: &mut Enc, s: &Site) {
    match s {
        Site::Key(p) => {
            e.byte(0);
            encode_path(e, p);
        }
        Site::Expr(p, r) => {
            e.byte(1);
            encode_path(e, p);
            e.u64(u64::from(r.start)).u64(u64::from(r.end));
        }
        Site::Header => {
            e.byte(2);
        }
        Site::Selector(p) => {
            e.byte(3);
            encode_path(e, p);
        }
        Site::File { file, locator } => {
            e.byte(4).str(file).len(locator.len());
            for (k, v) in locator {
                e.str(k);
                encode_value(e, v);
            }
        }
        Site::Argument(n) => {
            e.byte(5).u64(u64::from(*n));
        }
        Site::World => {
            e.byte(6);
        }
    }
}

fn decode_site(d: &mut Dec) -> Option<Site> {
    Some(match d.byte()? {
        0 => Site::Key(decode_path(d)?),
        1 => {
            let p = decode_path(d)?;
            let a = u32::try_from(d.u64()?).ok()?;
            let b = u32::try_from(d.u64()?).ok()?;
            Site::Expr(p, a..b)
        }
        2 => Site::Header,
        3 => Site::Selector(decode_path(d)?),
        4 => {
            let file = d.str()?;
            let mut locator = std::collections::BTreeMap::new();
            for _ in 0..d.count()? {
                let k = d.str()?;
                locator.insert(k, decode_value(d, 0)?);
            }
            Site::File { file, locator }
        }
        5 => Site::Argument(u32::try_from(d.u64()?).ok()?),
        6 => Site::World,
        _ => return None,
    })
}

fn encode_value(e: &mut Enc, v: &Value) {
    match v {
        Value::Bool(b) => e.byte(0).byte(u8::from(*b)),
        Value::Int(i) => e.byte(1).u64(*i as u64),
        Value::Double(f) => e.byte(2).u64(f.get().to_bits()),
        Value::Length(mm) => e.byte(3).u64(*mm as u64),
        Value::Text(s) => e.byte(4).str(s),
        Value::List(items) => {
            e.byte(5).len(items.len());
            for i in items {
                encode_value(e, i);
            }
            e
        }
    };
}

/// Lists nest at most this deep in a stored value; deeper input is refused as corrupt.
const MAX_DEPTH: u32 = 64;

fn decode_value(d: &mut Dec, depth: u32) -> Option<Value> {
    if depth > MAX_DEPTH {
        return None;
    }
    Some(match d.byte()? {
        0 => Value::Bool(match d.byte()? {
            0 => false,
            1 => true,
            _ => return None,
        }),
        1 => Value::Int(d.u64()? as i64),
        2 => Value::Double(via_diag::Finite::new(f64::from_bits(d.u64()?))?),
        3 => Value::Length(d.u64()? as i64),
        4 => Value::Text(d.str()?),
        5 => {
            let n = d.count()?;
            let mut items = Vec::with_capacity(n);
            for _ in 0..n {
                items.push(decode_value(d, depth + 1)?);
            }
            Value::List(items)
        }
        _ => return None,
    })
}

/// A node's value: its bytes and its diagnostics.
#[derive(Clone, Debug, PartialEq)]
pub struct Val {
    /// The value's bytes, as the node kind defines them.
    pub bytes: Arc<Vec<u8>>,
    /// Its diagnostics, as raised (no file or line).
    pub diags: Arc<Vec<Diag>>,
}

impl Val {
    /// A value with no diagnostics.
    pub fn plain(bytes: Vec<u8>) -> Val {
        Val {
            bytes: Arc::new(bytes),
            diags: Arc::new(Vec::new()),
        }
    }

    /// True when a diagnostic of the value is an error: the node failed and its readers are
    /// skipped.
    pub fn has_error(&self) -> bool {
        self.diags
            .iter()
            .any(|d| d.severity() == via_diag::Severity::Error)
    }
}

/// A value as the store holds it, with its digest and whether its diagnostics are storable.
pub(crate) struct Encoded {
    pub object: Vec<u8>,
    pub digest: Digest,
    /// The diagnostics' encoding, or `None` when the codec refused them.
    pub diags: Option<Vec<u8>>,
}

pub(crate) fn encode_val(v: &Val, codec: &dyn DiagCodec) -> Encoded {
    if v.diags.is_empty() {
        let digest = Digest::of(&v.bytes);
        return Encoded {
            object: v.bytes.as_ref().clone(),
            digest,
            diags: CoreCodec.encode(&[]),
        };
    }
    let diags = codec.encode(&v.diags);
    // When the codec refuses, the digest still has to tell values apart: the debug text holds
    // every member of a diagnostic. Such a value is never stored, so this text is never read.
    let for_digest = diags
        .clone()
        .unwrap_or_else(|| format!("unstorable {:?}", v.diags).into_bytes());
    let mut e = Enc::new();
    e.str("via-value-1").bytes(&v.bytes).bytes(&for_digest);
    let object = e.finish();
    Encoded {
        digest: Digest::of(&object),
        object,
        diags,
    }
}

/// Reads an object back, given the diagnostics from its key record.
pub(crate) fn decode_object(object: &[u8], diags: &[Diag], codec: &dyn DiagCodec) -> Option<Val> {
    if diags.is_empty() {
        return Some(Val::plain(object.to_vec()));
    }
    let mut d = Dec::new(object);
    if d.str()? != "via-value-1" {
        return None;
    }
    let bytes = d.bytes()?.to_vec();
    let stored = codec.decode(d.bytes()?)?;
    (d.done() && stored == diags).then(|| Val {
        bytes: Arc::new(bytes),
        diags: Arc::new(stored),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use via_diag::codes;

    fn sample() -> Diag {
        Diag::new(
            codes::graph::INPUT_MISSING,
            Subject::parse("step.checks").unwrap(),
            Site::Expr(KeyPath::key("inputs").index(2), 1..4),
            via_diag::text!("an input is missing"),
            via_diag::text!("restore the file"),
        )
        .at(Position::plan(-5, 12_000).with_z(via_diag::Finite::new(0.1).unwrap()))
        .with("path", "data/a.csv")
        .with(
            "n",
            Value::List(vec![Value::Int(-3), Value::Length(1005), Value::Bool(true)]),
        )
        .produced("scatter_x#k")
    }

    #[test]
    fn core_diagnostics_round_trip() {
        let ds = vec![sample(), sample().with("x", Value::double(2.5).unwrap())];
        let b = CoreCodec.encode(&ds).expect("storable");
        assert_eq!(CoreCodec.decode(&b), Some(ds));
        assert!(CoreCodec.decode(&b[..b.len() - 1]).is_none());
    }

    #[test]
    fn related_and_count_are_refused_not_lost() {
        let d = sample().counted(3);
        assert!(CoreCodec.encode(&[d]).is_none());
    }

    #[test]
    fn values_with_and_without_diagnostics() {
        let plain = Val::plain(b"abc".to_vec());
        let e = encode_val(&plain, &CoreCodec);
        assert_eq!(
            e.digest,
            Digest::of(b"abc"),
            "a build file's object is the file"
        );
        let v = Val {
            bytes: Arc::new(b"abc".to_vec()),
            diags: Arc::new(vec![sample()]),
        };
        let e2 = encode_val(&v, &CoreCodec);
        assert_ne!(
            e.digest, e2.digest,
            "diagnostics are part of the value digest"
        );
        let back = decode_object(&e2.object, &v.diags, &CoreCodec).unwrap();
        assert_eq!(back, v);
        assert!(v.has_error());
    }
}
