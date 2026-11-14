//! What a step returns (`protocol.md` 5): its `out` folder and document, checked before anything
//! from it is used, every fault located in `step:<id>/document.toml` where it has a line.
//!
//! The checks here are the protocol's: which tables a document may hold, where its files may
//! point, the field file format, keys that shadow a name the step can see, rows on a field of a
//! higher layer (`field.step_layer`) and a field that already exists (`field.step_base`). The
//! document's words and values are the reader's to check, once it is joined to its layer.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use via_diag::{codes, Diag, Site, Subject};
use via_doc::World;
use via_graph::{Digest, Store};
use via_num::Grid;

use crate::txt;

/// What a call returned, checked.
#[derive(Debug, Default)]
pub struct Returned {
    /// Each file of `out` as (path, digest), sorted by path; field files with `-0.0` written as
    /// `+0.0` (4.2).
    pub files: Vec<(String, Digest)>,
    /// The document's text (a generate step), as returned.
    pub document: Option<String>,
    pub diags: Vec<Diag>,
    /// Whether it returned rows on a field, and on which fields.
    pub row_fields: BTreeSet<String>,
    /// The keys of what it returned, by table: `feature`, `instance`, `field`, ...
    pub keys: BTreeMap<String, Vec<String>>,
    /// Whether nothing kept it from being used.
    pub ok: bool,
}

impl Returned {
    /// Whether it returned rows or features (`derived.md` 5: what `authority.unchecked` asks).
    pub fn returns_rows_or_features(&self) -> bool {
        !self.row_fields.is_empty() || self.keys.get("feature").is_some_and(|k| !k.is_empty())
    }

    /// Whether the document holds any table.
    pub fn has_tables(&self) -> bool {
        self.keys.values().any(|k| !k.is_empty()) || !self.row_fields.is_empty()
    }
}

/// The tables a returned document may hold (5).
const ALLOWED: [&str; 6] = ["field", "feature", "instance", "set", "run", "scatter"];

/// The file a returned document's records name (`documents.md` 2.1).
pub fn document_file(step: &str) -> String {
    format!("step:{step}/document.toml")
}

/// The line, column and bytes of the first line holding a table header for `path`, or a
/// top-level `key =` when `path` has one part and `top_key` is set.
fn locate(text: &str, path: &[&str], top_key: bool) -> (u32, u32, [u32; 2]) {
    let mut offset = 0usize;
    let mut in_header_zone = true;
    for (n, line) in text.split_inclusive('\n').enumerate() {
        let t = line.trim();
        let len = line.trim_end_matches(['\n', '\r']).len();
        let span = (n as u32 + 1, 1, [offset as u32, (offset + len) as u32]);
        if t.starts_with('[') {
            in_header_zone = false;
            let inner: String = t
                .trim_start_matches('[')
                .split(']')
                .next()
                .unwrap_or("")
                .chars()
                .filter(|c| *c != '"' && *c != '\'' && !c.is_whitespace())
                .collect();
            let parts: Vec<&str> = inner.split('.').collect();
            if !top_key && parts.len() >= path.len() && parts[..path.len()] == *path {
                return span;
            }
        } else if in_header_zone && top_key {
            let k = t.split('=').next().unwrap_or("").trim().trim_matches('"');
            if !t.starts_with('#') && k == path[0] {
                return span;
            }
        } else if !t.starts_with('#') && !top_key {
            // A dotted key in the root, `field.x.base = 1`.
            let k: String = t
                .split('=')
                .next()
                .unwrap_or("")
                .chars()
                .filter(|c| *c != '"' && !c.is_whitespace())
                .collect();
            if in_header_zone && k.split('.').take(path.len()).eq(path.iter().copied()) {
                return span;
            }
        }
        offset += line.len();
    }
    (1, 1, [0, 0])
}

struct Ctx<'a> {
    step: &'a str,
    text: &'a str,
    diags: Vec<Diag>,
}

impl Ctx<'_> {
    fn at(&mut self, code: via_diag::Code, path: &[&str], top: bool, msg: String, hint: String) {
        let (line, column, bytes) = locate(self.text, path, top);
        self.diags.push(Diag::new(
            code,
            Subject::entity("step", self.step),
            Site::Text {
                file: document_file(self.step),
                line,
                column,
                bytes,
            },
            txt(msg),
            txt(hint),
        ));
    }
}

fn step_diag(step: &str, code: via_diag::Code, msg: String, hint: &str) -> Diag {
    Diag::new(
        code,
        Subject::entity("step", step),
        Site::Header,
        txt(msg),
        txt(hint.to_string()),
    )
}

/// What a step may see by name: the world's ids at or below its layer.
pub struct Names<'a> {
    pub world: &'a World,
    /// The step's layer rank; `None` for a lint or export.
    pub rank: Option<i64>,
}

impl Names<'_> {
    fn sees(&self, id: &str) -> bool {
        match self.world.find(id) {
            Some((_, r)) => self.rank.is_none_or(|own| r <= own),
            None => false,
        }
    }
}

/// Reads and checks a call's `out` folder. `document` is a worker's returned text, if any.
pub fn collect(
    out: &Path,
    step: &str,
    when: &str,
    document: Option<String>,
    grid: &Grid,
    names: &Names<'_>,
    store: &Store,
) -> Returned {
    let mut r = Returned::default();
    let (files, links) = crate::dir::listing_with_links(out);
    for l in &links {
        r.diags.push(step_diag(
            step,
            codes::protocol::OUTPUT_FILE,
            format!(
                "step `{step}` left `out/{l}`, a symbolic link; a step returns files, not links"
            ),
            "write the file itself into `out`",
        ));
    }
    let on_disk = files.iter().any(|f| f == "document.toml");
    if when != "generate" {
        if on_disk {
            r.diags.push(step_diag(
                step,
                codes::protocol::OUTPUT_TABLE,
                format!("{when} step `{step}` wrote a document; a {when} step returns no document"),
                "remove `document.toml` from what the step writes",
            ));
        }
        for f in files {
            if let Ok(b) = std::fs::read(out.join(&f)) {
                r.files.push((f, store.put_object(&b)));
            }
        }
        // A lint's stray document is reported and its call still kept; a link is not.
        r.ok = links.is_empty();
        return r;
    }
    let text = match (document, on_disk) {
        (Some(_), true) => {
            r.diags.push(step_diag(
                step,
                codes::protocol::OUTPUT_TWICE,
                format!(
                    "step `{step}` returned its document in the call and wrote `document.toml`"
                ),
                "return the document one way: the call's result or `out/document.toml`",
            ));
            None
        }
        (Some(t), false) => Some(t),
        (None, true) => match std::fs::read(out.join("document.toml")) {
            Ok(b) => match String::from_utf8(b) {
                Ok(t) => Some(t),
                Err(_) => {
                    r.diags.push(step_diag(
                        step,
                        codes::doc::SYNTAX,
                        format!("step `{step}`: `document.toml` is not UTF-8"),
                        "write the document as UTF-8",
                    ));
                    None
                }
            },
            Err(_) => None,
        },
        (None, false) => {
            r.diags.push(step_diag(
                step,
                codes::protocol::NO_OUTPUT,
                format!("generate step `{step}` returned no document"),
                "write `out/document.toml`, empty when there is nothing to return",
            ));
            None
        }
    };
    let mut field_files: BTreeMap<String, (String, bool)> = BTreeMap::new();
    if let Some(text) = &text {
        let mut c = Ctx {
            step,
            text,
            diags: Vec::new(),
        };
        check_document(&mut c, text, grid, names, &mut r, &mut field_files);
        r.diags.extend(c.diags);
    }
    // Field files named by the document: inside `out`, in the subset, of the grid's shape.
    for (field, (file, weights)) in &field_files {
        let mut c = Ctx {
            step,
            text: text.as_deref().unwrap_or(""),
            diags: Vec::new(),
        };
        let inside = !file.is_empty()
            && Path::new(file)
                .components()
                .all(|p| matches!(p, std::path::Component::Normal(_)));
        let p = out.join(file);
        let is_link = std::fs::symlink_metadata(&p).is_ok_and(|m| m.file_type().is_symlink());
        if !inside || is_link || !p.is_file() {
            c.at(
                codes::protocol::OUTPUT_FILE,
                &["field", field],
                false,
                format!("step `{step}`: the field `{field}` names `{file}`, which is no file inside `out`"),
                "write the field file into `out` and name it relative to `out`".into(),
            );
        } else if let Ok(bytes) = std::fs::read(&p) {
            match check_field_file(&bytes, grid, *weights) {
                Ok(()) => {}
                Err(FieldFault::Format(why)) => c.at(
                    codes::protocol::FIELD_FILE,
                    &["field", field],
                    false,
                    format!("step `{step}`: `{file}` for field `{field}` is outside the field file format: {why}"),
                    "write it with `step.out.field`, or `numpy.save` of a C-order float32 array of the grid's shape".into(),
                ),
                Err(FieldFault::NotFinite(k)) => c.at(
                    codes::num::NOT_FINITE_RESULT,
                    &["field", field],
                    false,
                    format!("step `{step}`: `{file}` holds a value that is not finite at sample {k}"),
                    "return finite values only".into(),
                ),
            }
        }
        r.diags.extend(c.diags);
    }
    let named: BTreeSet<&str> = field_files.values().map(|(f, _)| f.as_str()).collect();
    for f in files {
        let Ok(mut b) = std::fs::read(out.join(&f)) else {
            continue;
        };
        if named.contains(f.as_str()) {
            positive_zero(&mut b);
        }
        r.files.push((f, store.put_object(&b)));
    }
    r.document = text;
    r.ok = !r
        .diags
        .iter()
        .any(|d| d.severity() == via_diag::Severity::Error);
    r
}

enum FieldFault {
    Format(String),
    NotFinite(usize),
}

fn check_field_file(bytes: &[u8], grid: &Grid, weights: bool) -> Result<(), FieldFault> {
    let h = via_proto::npy::parse(bytes).map_err(|e| FieldFault::Format(e.0))?;
    if h.descr != via_proto::npy::Descr::F4 {
        return Err(FieldFault::Format("its descr is not <f4".into()));
    }
    let (ny, nx) = (grid.ny as usize, grid.nx as usize);
    let ok = match h.shape.as_slice() {
        [a, b] => !weights && *a == ny && *b == nx,
        [_, a, b] => weights && *a == ny && *b == nx,
        _ => false,
    };
    if !ok {
        return Err(FieldFault::Format(format!(
            "its shape {:?} is not the grid's ({ny}, {nx}){}",
            h.shape,
            if weights {
                " with a plane per material"
            } else {
                ""
            }
        )));
    }
    for (k, c) in bytes[h.data_offset..].chunks_exact(4).enumerate() {
        if !f32::from_le_bytes([c[0], c[1], c[2], c[3]]).is_finite() {
            return Err(FieldFault::NotFinite(k));
        }
    }
    Ok(())
}

/// Every `-0.0` sample of a field file as `+0.0` (4.2): the engine reads them so, whatever wrote
/// them.
fn positive_zero(bytes: &mut [u8]) {
    let Ok(h) = via_proto::npy::parse(bytes) else {
        return;
    };
    if h.descr != via_proto::npy::Descr::F4 {
        return;
    }
    for c in bytes[h.data_offset..].chunks_exact_mut(4) {
        if c == [0, 0, 0, 0x80] {
            c[3] = 0;
        }
    }
}

fn check_document(
    c: &mut Ctx<'_>,
    text: &str,
    grid: &Grid,
    names: &Names<'_>,
    r: &mut Returned,
    field_files: &mut BTreeMap<String, (String, bool)>,
) {
    let _ = grid;
    let step = c.step.to_string();
    let doc = match text.parse::<toml::Table>() {
        Ok(t) => t,
        Err(e) => {
            let start = e.span().map_or(0, |s| s.start);
            let line = text[..start.min(text.len())].matches('\n').count() as u32 + 1;
            c.diags.push(Diag::new(
                codes::doc::SYNTAX,
                Subject::entity("step", step.clone()),
                Site::Text {
                    file: document_file(&step),
                    line,
                    column: 1,
                    bytes: [start as u32, start as u32],
                },
                txt(format!(
                    "step `{step}` returned a document that is not TOML: {}",
                    e.message()
                )),
                txt("write the document with the SDK's `step.out`".into()),
            ));
            return;
        }
    };
    for (top, v) in &doc {
        let toml::Value::Table(t) = v else {
            c.at(
                codes::protocol::OUTPUT_TABLE,
                &[top.as_str()],
                true,
                format!("step `{step}` returned the top-level key `{top}`; a returned document holds tables only"),
                "return only `field`, `feature`, `instance`, `set`, `run` and `scatter` tables".into(),
            );
            continue;
        };
        if !ALLOWED.contains(&top.as_str()) {
            c.at(
                codes::protocol::OUTPUT_TABLE,
                &[top.as_str()],
                false,
                format!("step `{step}` returned `[{top}]`, which a step may not return"),
                "return only `field`, `feature`, `instance`, `set`, `run` and `scatter` tables"
                    .into(),
            );
            continue;
        }
        let keys = r.keys.entry(top.clone()).or_default();
        for (key, body) in t {
            // A returned key is an id (`identity.md` 2.2): the build names it `<step>#<key>`.
            if !via_doc::is_id(key) {
                c.at(
                    codes::identity::BAD_KEY,
                    &[top.as_str(), key.as_str()],
                    false,
                    format!("step `{step}` returned the key `{key}`, which is not an id: `[a-z][a-z0-9_]*`, at most 48 bytes"),
                    format!("write `{}`", via_doc::repair_id(key)),
                );
                continue;
            }
            // A step finds; the author states (R14): what a step returns is never anchored.
            if top == "feature"
                && body.get("authority").and_then(toml::Value::as_str) == Some("anchored")
            {
                c.at(
                    codes::authority::STEP_ANCHORED,
                    &["feature", key.as_str()],
                    false,
                    format!("step `{step}` returned the feature `{key}` as anchored; a step's features are shaped or derived"),
                    "return it as shaped or derived; a fact the author holds is written in text".into(),
                );
            }
            if top == "field" {
                let Some(ft) = body.as_table() else { continue };
                let own: Vec<&String> = ft.keys().filter(|k| *k != "then").collect();
                let exists = names.world.entity("field", key).is_some();
                if !own.is_empty() {
                    if exists {
                        c.at(
                            codes::field::STEP_BASE,
                            &["field", key],
                            false,
                            format!("step `{step}` returned `[field.{key}]`, but the field `{key}` exists; a step never replaces a field"),
                            format!("return rows on `{key}` with `[field.{key}.then.<id>]`, or a field of a new name"),
                        );
                    } else {
                        keys.push(key.clone());
                        if names.sees(key) {
                            c.at(
                                codes::protocol::KEY_SHADOWS,
                                &["field", key],
                                false,
                                format!(
                                    "step `{step}` returned the key `{key}`, a name it can see"
                                ),
                                format!("name the returned field other than `{key}`"),
                            );
                        }
                        if let Some(f) = ft.get("file").and_then(toml::Value::as_str) {
                            let weights =
                                ft.get("type").and_then(toml::Value::as_str) == Some("weights");
                            field_files.insert(key.clone(), (f.to_string(), weights));
                        }
                    }
                }
                if let Some(rows) = ft.get("then").and_then(toml::Value::as_table) {
                    if exists {
                        let fr = names.world.rank("field", key);
                        if let (Some(own), Some(fr)) = (names.rank, fr) {
                            if fr > own {
                                let layer = names.world.home("field", key).unwrap_or("");
                                c.at(
                                    codes::field::STEP_LAYER,
                                    &["field", key, "then"],
                                    false,
                                    format!("step `{step}` returned a row on `{key}`, a field of the higher layer `{layer}`"),
                                    format!("declare the step in layer `{layer}` or above, or return a field of its own"),
                                );
                                continue;
                            }
                        }
                    }
                    if !rows.is_empty() {
                        r.row_fields.insert(key.clone());
                    }
                }
                continue;
            }
            keys.push(key.clone());
            if names.sees(key) {
                c.at(
                    codes::protocol::KEY_SHADOWS,
                    &[top.as_str(), key.as_str()],
                    false,
                    format!("step `{step}` returned the key `{key}`, a name it can see"),
                    format!("name the returned {top} other than `{key}`"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locate_finds_headers_and_top_keys() {
        let t = "x = 1\n\n[feature.a]\nkind = \"k\"\n\n[pin.p]\nat = [0, 0]\n";
        assert_eq!(locate(t, &["x"], true).0, 1);
        assert_eq!(locate(t, &["feature", "a"], false).0, 3);
        assert_eq!(locate(t, &["pin"], false).0, 6);
        let (_, _, b) = locate(t, &["pin"], false);
        assert_eq!(&t[b[0] as usize..b[1] as usize], "[pin.p]");
    }

    #[test]
    fn negative_zero_reads_as_positive() {
        let mut f = via_proto::npy::f32_file(&[1, 2], &[-0.0, 1.0]);
        positive_zero(&mut f);
        let h = via_proto::npy::parse(&f).unwrap();
        assert_eq!(&f[h.data_offset..h.data_offset + 4], &[0, 0, 0, 0]);
    }
}
