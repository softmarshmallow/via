//! Ids (`spec/vocabulary.md` 2.1, `spec/identity.md`'s `identity.bad_id`).

use crate::report::{At, Fault};
use via_diag::{codes, Subject};

/// The longest id, in bytes.
pub const MAX_ID: usize = 48;

/// `[a-z][a-z0-9_]*`, at most 48 bytes.
pub fn is_id(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && b.len() <= MAX_ID
        && b[0].is_ascii_lowercase()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_')
}

/// The repaired id of `identity.bad_id`: lowercased, each invalid character `_`, `x` in front if
/// the first is not a letter, cut to 48 bytes.
pub fn repair(s: &str) -> String {
    let mut out: String = s
        .chars()
        .map(|c| {
            let c = c.to_ascii_lowercase();
            if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if !out.starts_with(|c: char| c.is_ascii_lowercase()) {
        out.insert(0, 'x');
    }
    out.truncate(MAX_ID);
    out
}

/// `identity.bad_id` for a written id, or `None` when it is one.
pub(crate) fn check(id: &str, subject: Subject, at: At, what: &str) -> Option<Fault> {
    if is_id(id) {
        return None;
    }
    let fixed = repair(id);
    Some(Fault::new(
        codes::identity::BAD_ID,
        subject,
        at,
        format!(
            "the {what} `{id}` is not an id: an id is `[a-z][a-z0-9_]*`, at most {MAX_ID} bytes"
        ),
        format!("write `{fixed}`"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repairs_as_identity_says() {
        assert_eq!(repair("mill.a"), "mill_a");
        assert_eq!(repair("9x"), "x9x");
        assert_eq!(repair("Mill"), "mill");
        assert!(is_id("a_1"));
        assert!(!is_id("Winter"));
        assert!(!is_id(&"a".repeat(49)));
    }
}
