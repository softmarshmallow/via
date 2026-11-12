//! Once, `--fail-on`, aggregation, order, exit status and the JSON document
//! (`spec/diagnostics.md` sections 3, 5 and 7).

use crate::codes;
use crate::diag::{Diag, Text};
use crate::json::Json;
use crate::nearest::{nearest, Nearest};
use crate::record::{Occurrence, Record};
use crate::registry::{self, lookup};
use crate::site::Site;
use crate::subject::Subject;
use crate::value::Value;
use crate::Severity;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

/// The `format` of the diagnostics document: this chapter's version.
pub const DOCUMENT_FORMAT: &str = "via-diagnostics-1";

/// The warning codes `--fail-on` named for this run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FailOn(BTreeSet<String>);

impl FailOn {
    /// No codes.
    pub fn none() -> FailOn {
        FailOn::default()
    }

    /// Adds a code already checked by [`fail_on_code`].
    pub fn insert(&mut self, code: String) {
        self.0.insert(code);
    }

    /// True when the code was named.
    pub fn contains(&self, code: &str) -> bool {
        self.0.contains(code)
    }
}

/// Checks one `--fail-on CODE` argument (section 3). A registered warning code, or any
/// well-formed step code, is accepted as written. A registered code that is not a warning is
/// `cli.usage`, naming the nearest warning code; anything else is `cli.unknown_code`, naming the
/// nearest core code. `argument` is the 1-based index of the code in the command line.
pub fn fail_on_code(written: &str, argument: u32) -> Result<String, Diag> {
    if let Some(code) = lookup(written) {
        if code.severity() == Severity::Warning {
            return Ok(written.to_string());
        }
        let warnings = registry::rows()
            .iter()
            .filter(|r| r.severity == Severity::Warning)
            .map(|r| r.code);
        let near = nearest(written, warnings);
        let diag = Diag::new(
            codes::cli::USAGE,
            Subject::Cli,
            Site::Argument(argument),
            text(format!(
                "`--fail-on {written}` names a code that is {}, not a warning; only a warning can be failed",
                code.severity()
            )),
            text(near.hint("name a warning code")),
        );
        return Err(near.with_data(diag));
    }
    if written.starts_with("step.") && registry::well_formed(written) {
        return Ok(written.to_string());
    }
    Err(unknown_code(
        written,
        argument,
        registry::rows().iter().map(|r| r.code),
    ))
}

/// `cli.unknown_code` for a code that names nothing among `known`, with the nearest one.
pub(crate) fn unknown_code<'a>(
    written: &str,
    argument: u32,
    known: impl IntoIterator<Item = &'a str>,
) -> Diag {
    let near = nearest(written, known);
    let diag = Diag::new(
        codes::cli::UNKNOWN_CODE,
        Subject::Cli,
        Site::Argument(argument),
        text(format!("`{written}` is not a diagnostic code")),
        text(near.hint("run `via explain` to list every code")),
    );
    near.with_data(diag)
}

/// Text the engine wrote itself, never blank by construction; a blank one would be a defect, so
/// it falls back to a fixed sentence rather than panicking.
pub(crate) fn text(s: String) -> Text {
    Text::new(s).unwrap_or_else(|_| crate::text!("(no text)"))
}

impl Nearest<'_> {
    fn with_data(&self, diag: Diag) -> Diag {
        match self {
            Nearest::Found(n) => diag.with("nearest", *n),
            Nearest::None(_) => diag,
        }
    }
}

/// Section 5 in its order: once, then `--fail-on`, then aggregation, then the total order. The
/// input order does not matter (nodes run in parallel); the output is the same for any.
pub fn finish(records: Vec<Record>, fail_on: &FailOn) -> Vec<Record> {
    // Once: a record whose canonical JSON equals one already raised is dropped. The produced
    // name is part of the comparison, since two occurrences with different names count twice.
    let mut seen = BTreeSet::new();
    let mut once = Vec::with_capacity(records.len());
    for r in records {
        let key = (r.to_canonical_json(), r.name.clone());
        if seen.insert(key) {
            once.push(r);
        }
    }
    for r in &mut once {
        r.failed_on = r.severity == Severity::Warning && fail_on.contains(&r.code);
    }
    let mut out = aggregate(once);
    out.sort_by(order);
    out
}

/// Folds records of one producer that agree on code, subject, location and chain, have a
/// position and no `related`; gives every record with a produced name its `occurrences`.
fn aggregate(records: Vec<Record>) -> Vec<Record> {
    let mut groups: BTreeMap<String, Vec<Record>> = BTreeMap::new();
    let mut out = Vec::new();
    for r in records {
        let foldable =
            r.position.is_some() && r.related.is_empty() && r.producer && !r.count_stated;
        if foldable {
            groups.entry(fold_key(&r)).or_default().push(r);
        } else {
            out.push(single(r));
        }
    }
    for (_, group) in groups {
        // Each record's canonical JSON, computed once, is the last tiebreak of both sorts below:
        // two records at one position with one name (or none) but different data or message
        // would otherwise compare equal and keep arrival order, which is the order parallel nodes
        // finished in, so the folded record and its occurrences would vary with thread count.
        let mut keyed: Vec<(String, Record)> = group
            .into_iter()
            .map(|r| (r.to_canonical_json(), r))
            .collect();
        keyed.sort_by(|a, b| occurrence_order(&a.1, &b.1).then_with(|| a.0.cmp(&b.0)));
        if keyed.len() == 1 {
            out.extend(keyed.pop().map(|(_, r)| single(r)));
            continue;
        }
        let worst = lookup(&keyed[0].1.code).and_then(|c| c.row().worst);
        let mut occ: Vec<&(String, Record)> = keyed.iter().collect();
        if let Some(w) = worst {
            // Largest first; a record without the value ranks last. Ties keep the total
            // occurrence order (position, name, canonical JSON).
            occ.sort_by(|(ja, a), (jb, b)| {
                let va = a.data.get(w).and_then(Value::rank);
                let vb = b.data.get(w).and_then(Value::rank);
                match (va, vb) {
                    (Some(x), Some(y)) => y.total_cmp(&x),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => Ordering::Equal,
                }
                .then_with(|| occurrence_order(a, b))
                .then_with(|| ja.cmp(jb))
            });
        }
        let occurrences: Vec<Occurrence> = occ
            .iter()
            .take(5)
            .map(|(_, r)| Occurrence {
                name: r.name.clone(),
                position: r.position,
                value: worst.and_then(|w| r.data.get(w).cloned()),
            })
            .collect();
        let count = u32::try_from(keyed.len()).unwrap_or(u32::MAX);
        let (_, mut first) = keyed.swap_remove(0);
        first.count = count;
        first.occurrences = occurrences;
        first.name = None;
        out.push(first);
    }
    out
}

/// A record that is not folded: its produced name, if any, moves into `occurrences`.
fn single(mut r: Record) -> Record {
    if let Some(name) = r.name.take() {
        let worst = lookup(&r.code).and_then(|c| c.row().worst);
        r.occurrences = vec![Occurrence {
            name: Some(name),
            position: r.position,
            value: worst.and_then(|w| r.data.get(w).cloned()),
        }];
    }
    r
}

fn fold_key(r: &Record) -> String {
    let full = r.to_json();
    let mut key = Json::obj();
    if let Json::Obj(members) = full {
        for (k, v) in members {
            if matches!(
                k.as_str(),
                "code"
                    | "subject"
                    | "file"
                    | "line"
                    | "column"
                    | "bytes"
                    | "argument"
                    | "chain"
                    | "failed_on"
            ) {
                key.insert(k, v);
            }
        }
    }
    Json::Obj(key).to_canonical()
}

/// Occurrence order: position (`y`, `x`, `z`), then name bytewise. Not total on its own; callers
/// break ties on the canonical JSON.
fn occurrence_order(a: &Record, b: &Record) -> Ordering {
    let pa = a.position;
    let pb = b.position;
    match (pa, pb) {
        (Some(x), Some(y)) => x.order(&y),
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
    .then_with(|| a.name.cmp(&b.name))
}

/// The total order of section 5.
pub(crate) fn order(a: &Record, b: &Record) -> Ordering {
    a.severity
        .cmp(&b.severity)
        .then(a.location.form_rank().cmp(&b.location.form_rank()))
        .then_with(|| match (&a.location, &b.location) {
            (crate::Location::Argument(x), crate::Location::Argument(y)) => x.cmp(y),
            (la, lb) => la
                .file()
                .cmp(&lb.file())
                .then_with(|| line_col(la).cmp(&line_col(lb))),
        })
        .then_with(|| a.code.as_bytes().cmp(b.code.as_bytes()))
        .then_with(|| a.subject.as_bytes().cmp(b.subject.as_bytes()))
        .then_with(|| match (a.position, b.position) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(x), Some(y)) => x.order(&y),
        })
        .then_with(|| a.to_canonical_json().cmp(&b.to_canonical_json()))
}

fn line_col(l: &crate::Location) -> (u32, u32) {
    match l {
        crate::Location::Span { line, column, .. } => (*line, *column),
        _ => (0, 0),
    }
}

/// The exit status records give (section 7): 3 with an `internal` error, else 2 with a `cli.*`
/// error, else 1 with any error or `failed_on` record, else 0.
pub fn exit_status(records: &[Record]) -> u8 {
    if records
        .iter()
        .any(|r| r.fails() && registry::is_internal(&r.code))
    {
        3
    } else if records
        .iter()
        .any(|r| r.fails() && r.code.starts_with("cli."))
    {
        2
    } else if records.iter().any(Record::fails) {
        1
    } else {
        0
    }
}

/// Splits records into world diagnostics (for `diagnostics.json`) and run diagnostics
/// (`build.md` 11.1), each keeping its order.
pub fn split_world_run(records: Vec<Record>) -> (Vec<Record>, Vec<Record>) {
    records.into_iter().partition(|r| !r.is_run())
}

/// A lint step of the world, for the `lints` list and the terminal summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lint {
    /// The step's id.
    pub step: String,
    /// False when the step was skipped.
    pub ran: bool,
}

/// Records counted by severity (section 7): records, not occurrences; a `failed_on` record
/// counts under `warning` and under `failed_on`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Error records.
    pub error: u64,
    /// Warning records, `failed_on` ones included.
    pub warning: u64,
    /// Info records.
    pub info: u64,
    /// Warning records failed by `--fail-on`.
    pub failed_on: u64,
}

impl Counts {
    /// Counts these records.
    pub fn of<'a>(records: impl IntoIterator<Item = &'a Record>) -> Counts {
        let mut c = Counts::default();
        for r in records {
            match r.severity {
                Severity::Error => c.error += 1,
                Severity::Warning => c.warning += 1,
                Severity::Info => c.info += 1,
            }
            if r.failed_on {
                c.failed_on += 1;
            }
        }
        c
    }

    fn to_json(self) -> Json {
        let mut o = Json::obj();
        for (k, v) in [
            ("error", self.error),
            ("failed_on", self.failed_on),
            ("info", self.info),
            ("warning", self.warning),
        ] {
            o.insert(k.into(), Json::Int(i64::try_from(v).unwrap_or(i64::MAX)));
        }
        Json::Obj(o)
    }
}

/// The diagnostics document (section 7), canonical, ending in one line feed. For
/// `diagnostics.json`, pass the world records and their own [`exit_status`]; for `--json` and
/// `--diagnostics`, world and run records together and the process's exit status. The records
/// are put in section 5's order here, so a caller cannot write them unsorted.
pub fn document(records: &[Record], lints: &[Lint], status: u8) -> String {
    let mut sorted: Vec<&Record> = records.iter().collect();
    sorted.sort_by(|a, b| order(a, b));
    let mut o = Json::obj();
    o.insert(
        "counts".into(),
        Counts::of(sorted.iter().copied()).to_json(),
    );
    o.insert(
        "diagnostics".into(),
        Json::Arr(sorted.iter().map(|r| r.to_json()).collect()),
    );
    o.insert("format".into(), Json::Str(DOCUMENT_FORMAT.into()));
    let mut lint_list: Vec<&Lint> = lints.iter().collect();
    lint_list.sort_by(|a, b| a.step.as_bytes().cmp(b.step.as_bytes()));
    o.insert(
        "lints".into(),
        Json::Arr(
            lint_list
                .iter()
                .map(|l| {
                    let c = lint_counts(records, &l.step);
                    let mut m = Json::obj();
                    m.insert("step".into(), Json::Str(l.step.clone()));
                    m.insert("ran".into(), Json::Bool(l.ran));
                    m.insert("error".into(), json_count(c.error));
                    m.insert("warning".into(), json_count(c.warning));
                    m.insert("info".into(), json_count(c.info));
                    Json::Obj(m)
                })
                .collect(),
        ),
    );
    o.insert("status".into(), Json::Int(i64::from(status)));
    let mut s = Json::Obj(o).to_canonical();
    s.push('\n');
    s
}

fn json_count(n: u64) -> Json {
    Json::Int(i64::try_from(n).unwrap_or(i64::MAX))
}

/// A lint step's records (section 7): those with its own codes, `step.<id>.*`, and those whose
/// subject is the step, `step.<id>` or a key path in it, so its `diag.malformed` and `protocol.*`
/// records count against it too.
pub(crate) fn lint_counts(records: &[Record], step: &str) -> Counts {
    let subject = format!("step.{step}");
    let prefix = format!("{subject}.");
    Counts::of(records.iter().filter(|r| {
        r.code.starts_with(&prefix) || r.subject == subject || r.subject.starts_with(&prefix)
    }))
}
