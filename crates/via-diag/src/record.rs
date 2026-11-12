//! The emitted record: a diagnostic with its location, ready to sort, fold and write.

use crate::diag::Position;
use crate::json::Json;
use crate::site::{ChainEntry, Location, Resolved};
use crate::subject::Subject;
use crate::value::Value;
use crate::Severity;
use std::collections::BTreeMap;

/// A related thing, resolved.
#[derive(Clone, Debug, PartialEq)]
pub struct RelatedRecord {
    pub(crate) subject: String,
    pub(crate) location: Location,
    pub(crate) position: Option<Position>,
    pub(crate) note: String,
}

impl RelatedRecord {
    /// The related thing's subject.
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Its location.
    pub fn location(&self) -> &Location {
        &self.location
    }

    fn to_json(&self) -> Json {
        let mut o = Json::obj();
        o.insert("subject".into(), Json::Str(self.subject.clone()));
        o.insert("note".into(), Json::Str(self.note.clone()));
        location_json(&self.location, &mut o);
        if let Some(p) = self.position {
            o.insert("position".into(), position_json(p));
        }
        Json::Obj(o)
    }
}

/// One occurrence of an aggregated record (section 5).
#[derive(Clone, Debug, PartialEq)]
pub struct Occurrence {
    /// The produced name, if there is one.
    pub name: Option<String>,
    /// Its position.
    pub position: Option<Position>,
    /// The value of the code's `worst` field, if the code has one.
    pub value: Option<Value>,
}

impl Occurrence {
    fn to_json(&self) -> Json {
        let mut o = Json::obj();
        if let Some(n) = &self.name {
            o.insert("name".into(), Json::Str(n.clone()));
        }
        if let Some(p) = self.position {
            o.insert("position".into(), position_json(p));
        }
        if let Some(v) = &self.value {
            o.insert("value".into(), v.to_json());
        }
        Json::Obj(o)
    }
}

/// A located diagnostic. Only [`crate::Diag::emit`] makes one, so every record has a code, a
/// subject, a location and a non-blank hint.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub(crate) severity: Severity,
    pub(crate) code: String,
    pub(crate) subject: String,
    pub(crate) producer: bool,
    pub(crate) message: String,
    pub(crate) hint: String,
    pub(crate) location: Location,
    pub(crate) chain: Vec<ChainEntry>,
    pub(crate) position: Option<Position>,
    pub(crate) related: Vec<RelatedRecord>,
    pub(crate) data: BTreeMap<String, Value>,
    /// The produced name of this occurrence, before aggregation moves it into `occurrences`.
    pub(crate) name: Option<String>,
    /// The record's count: occurrences after aggregation, or a count the code states itself.
    pub(crate) count: u32,
    /// True when the count was stated by the code (`diag.skipped`) and is always written.
    pub(crate) count_stated: bool,
    pub(crate) occurrences: Vec<Occurrence>,
    pub(crate) failed_on: bool,
}

impl Record {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        severity: Severity,
        code: String,
        subject: &Subject,
        message: String,
        hint: String,
        resolved: Resolved,
        position: Option<Position>,
        related: Vec<RelatedRecord>,
        data: BTreeMap<String, Value>,
        name: Option<String>,
        count: Option<u32>,
    ) -> Record {
        Record {
            severity,
            code,
            subject: subject.to_string(),
            producer: subject.is_producer(),
            message,
            hint,
            location: resolved.location,
            chain: resolved.chain,
            position,
            related,
            data,
            name,
            count: count.unwrap_or(1),
            count_stated: count.is_some(),
            occurrences: Vec::new(),
            failed_on: false,
        }
    }

    /// The registered severity (never changed by `--fail-on`).
    pub fn severity(&self) -> Severity {
        self.severity
    }

    /// The code.
    pub fn code(&self) -> &str {
        &self.code
    }

    /// The subject in canonical form.
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// The message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The hint.
    pub fn hint(&self) -> &str {
        &self.hint
    }

    /// The location.
    pub fn location(&self) -> &Location {
        &self.location
    }

    /// The chain.
    pub fn chain(&self) -> &[ChainEntry] {
        &self.chain
    }

    /// The position.
    pub fn position(&self) -> Option<Position> {
        self.position
    }

    /// The related things.
    pub fn related(&self) -> &[RelatedRecord] {
        &self.related
    }

    /// The `data` facts.
    pub fn data(&self) -> &BTreeMap<String, Value> {
        &self.data
    }

    /// The count (1 unless aggregated or stated).
    pub fn count(&self) -> u32 {
        self.count
    }

    /// The occurrences, at most five.
    pub fn occurrences(&self) -> &[Occurrence] {
        &self.occurrences
    }

    /// True when `--fail-on` named this warning's code.
    pub fn failed_on(&self) -> bool {
        self.failed_on
    }

    /// True for a run diagnostic (`build.md` 11.1), never written to `diagnostics.json`.
    pub fn is_run(&self) -> bool {
        crate::registry::is_run_code(&self.code)
    }

    /// True when it counts as an error for the exit status and for writing: an error, or a
    /// warning failed by `--fail-on`.
    pub fn fails(&self) -> bool {
        self.severity == Severity::Error || self.failed_on
    }

    pub(crate) fn to_json(&self) -> Json {
        let mut o = Json::obj();
        o.insert("severity".into(), Json::Str(self.severity.as_str().into()));
        o.insert("code".into(), Json::Str(self.code.clone()));
        o.insert("subject".into(), Json::Str(self.subject.clone()));
        o.insert("message".into(), Json::Str(self.message.clone()));
        o.insert("hint".into(), Json::Str(self.hint.clone()));
        location_json(&self.location, &mut o);
        if !self.chain.is_empty() {
            o.insert(
                "chain".into(),
                Json::Arr(self.chain.iter().map(chain_json).collect()),
            );
        }
        if let Some(p) = self.position {
            o.insert("position".into(), position_json(p));
        }
        if !self.related.is_empty() {
            o.insert(
                "related".into(),
                Json::Arr(self.related.iter().map(RelatedRecord::to_json).collect()),
            );
        }
        if !self.data.is_empty() {
            let mut d = Json::obj();
            for (k, v) in &self.data {
                d.insert(k.clone(), v.to_json());
            }
            o.insert("data".into(), Json::Obj(d));
        }
        if self.count > 1 || self.count_stated {
            o.insert("count".into(), Json::Int(i64::from(self.count)));
        }
        if !self.occurrences.is_empty() {
            o.insert(
                "occurrences".into(),
                Json::Arr(self.occurrences.iter().map(Occurrence::to_json).collect()),
            );
        }
        if self.failed_on {
            o.insert("failed_on".into(), Json::Bool(true));
        }
        Json::Obj(o)
    }

    /// The record as one canonical JSON object (no final line feed).
    pub fn to_canonical_json(&self) -> String {
        self.to_json().to_canonical()
    }
}

fn location_json(loc: &Location, o: &mut BTreeMap<String, Json>) {
    match loc {
        Location::Argument(n) => {
            o.insert("argument".into(), Json::Int(i64::from(*n)));
        }
        Location::File { file } => {
            o.insert("file".into(), Json::Str(file.clone()));
        }
        Location::Span {
            file,
            line,
            column,
            bytes,
        } => {
            o.insert("file".into(), Json::Str(file.clone()));
            o.insert("line".into(), Json::Int(i64::from(*line)));
            o.insert("column".into(), Json::Int(i64::from(*column)));
            o.insert(
                "bytes".into(),
                Json::Arr(vec![
                    Json::Int(i64::from(bytes[0])),
                    Json::Int(i64::from(bytes[1])),
                ]),
            );
        }
    }
}

fn chain_json(c: &ChainEntry) -> Json {
    let mut o = Json::obj();
    o.insert("through".into(), Json::Str(c.through.as_str().into()));
    o.insert("name".into(), Json::Str(c.name.clone()));
    o.insert("file".into(), Json::Str(c.file.clone()));
    o.insert("line".into(), Json::Int(i64::from(c.line)));
    if let Some(owner) = &c.owner {
        o.insert("owner".into(), Json::Str(owner.clone()));
    }
    Json::Obj(o)
}

fn position_json(p: Position) -> Json {
    Json::Arr(p.to_value_list().iter().map(Value::to_json).collect())
}
