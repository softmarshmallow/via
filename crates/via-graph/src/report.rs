//! The run report (`spec/graph.md` 6.3): one canonical JSON object (`documents.md` 8), written
//! beside the build directory as `<directory>.run.json`, last, to a temporary name and renamed
//! over the old one.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use crate::digest::Digest;
use crate::engine::{NeededBy, NodeOutcome, Run, Status};
use crate::id::NodeId;
use crate::record::BuildRecord;

/// The report's format; changes when a reader of the old format would misread the new one.
pub const RUN_FORMAT: &str = "via-run-0";

/// A canonical JSON value: members ordered bytewise, only `"`, `\` and U+0000 to U+001F
/// escaped (`\u00xx` lowercase), no whitespace outside strings.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// An unsigned integer.
    Int(u64),
    /// A string.
    Str(String),
    /// A list.
    Arr(Vec<Json>),
    /// An object.
    Obj(BTreeMap<String, Json>),
    /// Canonical JSON written elsewhere (a `diagnostics.md` record), inserted as it is.
    Raw(String),
}

impl Json {
    /// The canonical text, without a final line feed.
    pub fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Int(i) => {
                let _ = write!(out, "{i}");
            }
            Json::Str(s) => write_str(s, out),
            Json::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Json::Obj(m) => {
                out.push('{');
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_str(k, out);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
            Json::Raw(s) => out.push_str(s),
        }
    }
}

fn write_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn obj<const N: usize>(members: [(&str, Json); N]) -> Json {
    Json::Obj(
        members
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

fn digest(d: Option<&Digest>) -> Json {
    d.map_or(Json::Null, |d| Json::Str(d.hex()))
}

/// What the report says about the build, beyond the nodes (6.3, `build`, `engine`, `threads`).
#[derive(Clone, Debug)]
pub struct ReportHead {
    /// The numerics epoch.
    pub epoch: u64,
    /// The release of the binary.
    pub version: String,
    /// The thread count the build used.
    pub threads: u64,
    /// The build key.
    pub key: Digest,
    /// The manifest's `id`, or `None` when no manifest was written.
    pub id: Option<String>,
    /// Whether the null build applied.
    pub null: bool,
    /// The previous build's key, if any.
    pub previous: Option<Digest>,
    /// The build's wall time in whole milliseconds.
    pub ms: u64,
    /// Whole milliseconds the plan spent outside its nodes' own computations (6.3
    /// `totals.memo_ms`).
    pub memo_ms: u64,
    /// The run diagnostics as `diagnostics.md` 1 records in canonical JSON, already in the
    /// order of `diagnostics.md` 5 (`via_diag::Record::to_canonical_json`).
    pub run_records: Vec<String>,
}

fn node_json(id: &NodeId, n: &NodeOutcome) -> Json {
    let cause = n.cause.as_ref().map_or(Json::Null, |c| {
        let mut m = BTreeMap::new();
        m.insert("reason".to_string(), Json::Str(c.reason().to_string()));
        if let crate::engine::Cause::Edge { label, node } = c {
            m.insert("edge".to_string(), Json::Str(label.clone()));
            m.insert("node".to_string(), Json::Str(node.to_string()));
        }
        if let crate::engine::Cause::Trace { method, request } = c {
            m.insert("method".to_string(), Json::Str(method.clone()));
            m.insert("request".to_string(), Json::Str(request.hex()));
        }
        Json::Obj(m)
    });
    let needed = match &n.needed_by {
        None => Json::Null,
        Some(NeededBy::Expansion) => Json::Str("expansion".into()),
        Some(NeededBy::Node(r)) => Json::Str(r.to_string()),
    };
    obj([
        ("cause", cause),
        ("family", Json::Str(id.family().to_string())),
        ("failed", Json::Bool(n.failed)),
        ("id", Json::Str(id.to_string())),
        ("key", digest(n.key.as_ref())),
        ("ms", Json::Int(n.ms)),
        ("needed_by", needed),
        ("status", Json::Str(n.status.as_str().to_string())),
        ("value", digest(n.value.as_ref())),
    ])
}

fn step_json(id: &NodeId, n: &NodeOutcome) -> Option<Json> {
    let s = n.step.as_ref()?;
    let reads = s
        .reads
        .iter()
        .map(|r| {
            obj([
                ("answer", Json::Str(r.answer.hex())),
                ("method", Json::Str(r.method.clone())),
                ("request", Json::Str(r.request.hex())),
            ])
        })
        .collect();
    let returned = s.returned.as_ref().map_or(Json::Null, |(v, files)| {
        let mut fs: Vec<&(String, Digest)> = files.iter().collect();
        fs.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
        obj([
            (
                "files",
                Json::Arr(
                    fs.into_iter()
                        .map(|(p, d)| {
                            obj([
                                ("digest", Json::Str(d.hex())),
                                ("path", Json::Str(p.clone())),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("value", Json::Str(v.hex())),
        ])
    });
    Some(obj([
        (
            "command",
            Json::Arr(s.command.iter().map(|c| Json::Str(c.clone())).collect()),
        ),
        (
            "exit",
            s.exit.map_or(Json::Null, |e| Json::Raw(e.to_string())),
        ),
        ("id", Json::Str(id.to_string())),
        ("log", digest(s.log.as_ref())),
        ("reads", Json::Arr(reads)),
        ("returned", returned),
        ("static_key", digest(s.static_key.as_ref())),
        ("trace", digest(s.trace.as_ref())),
    ]))
}

/// The run report of a run: `complete` is the run's.
pub fn report(head: &ReportHead, run: &Run) -> String {
    let nodes: Vec<(&NodeId, &NodeOutcome)> = run.nodes.iter().collect();
    assemble(head, run.complete, &nodes)
}

/// The run report of a null build (6.2): every node of the record `reused`.
pub fn null_report(head: &ReportHead, record: &BuildRecord) -> String {
    let outcomes: Vec<(NodeId, NodeOutcome)> = record
        .nodes
        .iter()
        .map(|(id, n)| {
            (
                id.clone(),
                NodeOutcome {
                    status: Status::Reused,
                    key: n.key,
                    value: n.value,
                    failed: false,
                    cause: None,
                    needed_by: None,
                    ms: 0,
                    step: record.steps.get(id).cloned(),
                },
            )
        })
        .collect();
    let refs: Vec<(&NodeId, &NodeOutcome)> = outcomes.iter().map(|(a, b)| (a, b)).collect();
    assemble(head, record.complete, &refs)
}

fn assemble(head: &ReportHead, complete: bool, nodes: &[(&NodeId, &NodeOutcome)]) -> String {
    let mut counts: BTreeMap<&str, u64> = BTreeMap::new();
    for s in ["ran", "recomputed", "reused", "skipped"] {
        counts.insert(s, 0);
    }
    for (_, n) in nodes {
        *counts.entry(n.status.as_str()).or_default() += 1;
    }
    let steps: Vec<Json> = nodes
        .iter()
        .filter(|(id, _)| id.family() == "step")
        .filter_map(|(id, n)| step_json(id, n))
        .collect();
    let report = obj([
        (
            "build",
            obj([
                ("complete", Json::Bool(complete)),
                ("id", head.id.clone().map_or(Json::Null, Json::Str)),
                ("key", Json::Str(head.key.hex())),
                ("null", Json::Bool(head.null)),
                ("previous", digest(head.previous.as_ref())),
            ]),
        ),
        (
            "diagnostics",
            Json::Arr(head.run_records.iter().cloned().map(Json::Raw).collect()),
        ),
        (
            "engine",
            obj([
                ("epoch", Json::Int(head.epoch)),
                ("version", Json::Str(head.version.clone())),
            ]),
        ),
        ("format", Json::Str(RUN_FORMAT.into())),
        (
            "nodes",
            Json::Arr(nodes.iter().map(|(id, n)| node_json(id, n)).collect()),
        ),
        ("steps", Json::Arr(steps)),
        (
            "system",
            obj([
                ("arch", Json::Str(std::env::consts::ARCH.into())),
                ("os", Json::Str(std::env::consts::OS.into())),
            ]),
        ),
        ("threads", Json::Int(head.threads)),
        (
            "totals",
            obj([
                ("memo_ms", Json::Int(head.memo_ms)),
                ("ms", Json::Int(head.ms)),
                ("nodes", Json::Int(nodes.len() as u64)),
                ("ran", Json::Int(counts["ran"])),
                ("recomputed", Json::Int(counts["recomputed"])),
                ("reused", Json::Int(counts["reused"])),
                ("skipped", Json::Int(counts["skipped"])),
            ]),
        ),
    ]);
    let mut out = String::new();
    report.write(&mut out);
    out.push('\n');
    out
}

/// `<directory>.run.json` beside the build directory.
pub fn report_path(directory: &Path) -> PathBuf {
    let mut name = directory
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(".run.json");
    directory.with_file_name(name)
}

/// Writes the report to a temporary name beside it and renames it over the old one, so a reader
/// never sees half a report.
pub fn write_report(directory: &Path, text: &str) -> io::Result<()> {
    let path = report_path(directory);
    let mut tmp_name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    tmp_name.push(format!(".tmp-{}", std::process::id()));
    let tmp = path.with_file_name(tmp_name);
    let mut f = std::fs::File::create(&tmp)?;
    f.write_all(text.as_bytes())?;
    f.sync_all()?;
    drop(f);
    std::fs::rename(&tmp, &path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_strings_and_order() {
        let j = obj([
            ("b", Json::Str("q\"\\\u{1}é".into())),
            (
                "a",
                Json::Arr(vec![Json::Null, Json::Int(3), Json::Bool(false)]),
            ),
        ]);
        let mut s = String::new();
        j.write(&mut s);
        assert_eq!(s, "{\"a\":[null,3,false],\"b\":\"q\\\"\\\\\\u0001é\"}");
    }

    #[test]
    fn report_path_is_beside_the_directory() {
        assert_eq!(
            report_path(Path::new("/x/build/alpha")),
            PathBuf::from("/x/build/alpha.run.json")
        );
    }
}
