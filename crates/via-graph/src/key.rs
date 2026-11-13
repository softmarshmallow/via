//! Keys (`spec/graph.md` 3.1) and the four versions (3.3).

use crate::digest::{Digest, Enc};

/// The four versions every key holds (3.3). A new value of any one changes every key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Versions {
    /// The document language, `documents.md` 2.3 (`via-doc-1`).
    pub document: String,
    /// The step protocol (`via-step-1`).
    pub protocol: String,
    /// The build format (`build.md`).
    pub build: String,
    /// The numerics epoch: raised by any engine change that may change the bytes of a value.
    pub epoch: u64,
}

impl Versions {
    /// The test versions of 3.1's vector.
    pub fn test() -> Versions {
        Versions {
            document: "test-doc".into(),
            protocol: "via-step-1".into(),
            build: "via-build-0".into(),
            epoch: 1,
        }
    }

    pub(crate) fn encode(&self, e: &mut Enc) {
        e.str(&self.document)
            .str(&self.protocol)
            .str(&self.build)
            .u64(self.epoch);
    }
}

/// What a key is computed from (3.1, items 1 to 5). Order is not significant: the encoding sorts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyParts {
    /// The node's family.
    pub family: String,
    /// The digests of the statements it is made from.
    pub statements: Vec<Digest>,
    /// Its own parameters, as (name, value).
    pub params: Vec<(String, String)>,
    /// Each input edge as (label, the input's value digest).
    pub edges: Vec<(String, Digest)>,
}

/// The canonical encoding of 3.1, whose digest is the key.
pub fn encode_key(versions: &Versions, parts: &KeyParts) -> Vec<u8> {
    let mut e = Enc::new();
    e.str("via-key");
    versions.encode(&mut e);
    e.str(&parts.family);
    let mut st = parts.statements.clone();
    st.sort();
    e.len(st.len());
    for d in &st {
        e.digest(d);
    }
    let mut params: Vec<&(String, String)> = parts.params.iter().collect();
    params.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()).then(a.1.cmp(&b.1)));
    e.len(params.len());
    for (n, v) in params {
        e.str(n).str(v);
    }
    let mut edges: Vec<&(String, Digest)> = parts.edges.iter().collect();
    edges.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()).then(a.1.cmp(&b.1)));
    e.len(edges.len());
    for (l, d) in edges {
        e.str(l).digest(d);
    }
    e.finish()
}

/// The key: the digest of [`encode_key`].
pub fn key(versions: &Versions, parts: &KeyParts) -> Digest {
    Digest::of(&encode_key(versions, parts))
}

/// A digest over named parts in a fixed encoding, for keys this chapter defines outside 3.1 (the
/// build key of 6.2, a step's static key of 5.1): a tag, the versions, then each (name, value)
/// pair sorted by name and value. The caller names the parts; nothing of the machine enters.
pub fn digest_of_parts(tag: &str, versions: &Versions, parts: &[(String, String)]) -> Digest {
    let mut e = Enc::new();
    e.str(tag);
    versions.encode(&mut e);
    let mut ps: Vec<&(String, String)> = parts.iter().collect();
    ps.sort_by(|a, b| {
        a.0.as_bytes()
            .cmp(b.0.as_bytes())
            .then(a.1.as_bytes().cmp(b.1.as_bytes()))
    });
    e.len(ps.len());
    for (n, v) in ps {
        e.str(n).str(v);
    }
    Digest::of(&e.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VECTOR: &str = concat!(
        "07000000000000007669612d6b65790800000000000000746573742d646f630a000000000000007669612d737465702d31",
        "0b000000000000007669612d6275696c642d3001000000000000000600000000000000736f7572636500000000000000",
        "0001000000000000000400000000000000706174680a00000000000000776f726c642e746f6d6c010000000000000005",
        "0000000000000062797465731111111111111111111111111111111111111111111111111111111111111111",
    );

    fn vector_parts() -> KeyParts {
        KeyParts {
            family: "source".into(),
            statements: vec![],
            params: vec![("path".into(), "world.toml".into())],
            edges: vec![("bytes".into(), Digest([0x11; 32]))],
        }
    }

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    #[test]
    fn key_vector_bytes_and_digest() {
        let enc = encode_key(&Versions::test(), &vector_parts());
        assert_eq!(enc.len(), 189);
        assert_eq!(hex(&enc), VECTOR);
        // The digest recorded in spec/vectors/graph/keys.toml.
        let text = include_str!("../../../spec/vectors/graph/keys.toml");
        let k = key(&Versions::test(), &vector_parts()).hex();
        assert!(text.contains(&format!("digest = \"{k}\"")), "{k}");
        assert!(text.contains(&format!("bytes = \"{VECTOR}\"")));
    }

    #[test]
    fn order_of_parts_does_not_matter_but_every_part_does() {
        let v = Versions::test();
        let a = KeyParts {
            family: "f".into(),
            statements: vec![Digest([2; 32]), Digest([1; 32])],
            params: vec![("w".into(), "1".into()), ("a".into(), "2".into())],
            edges: vec![("y".into(), Digest([3; 32])), ("x".into(), Digest([4; 32]))],
        };
        let mut b = a.clone();
        b.statements.reverse();
        b.params.reverse();
        b.edges.reverse();
        assert_eq!(key(&v, &a), key(&v, &b));
        let mut c = a.clone();
        c.edges.push(("x".into(), Digest([4; 32])));
        assert_ne!(
            key(&v, &a),
            key(&v, &c),
            "two edges to one node are two inputs"
        );
        let mut v2 = v.clone();
        v2.epoch = 2;
        assert_ne!(key(&v, &a), key(&v2, &a));
    }
}
