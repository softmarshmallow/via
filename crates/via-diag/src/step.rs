//! A step's diagnostics, checked on arrival (`spec/diagnostics.md` section 6).
//!
//! The protocol crate decodes `diag.emit` into [`StepDiagIn`] (every field optional, as it came
//! off the wire) and calls [`arrive`] with a [`StepWorld`] that knows the world's names. What
//! comes back is always a well-formed [`Diag`]: the step's own, under `step.<id>.<name>`, or one
//! `diag.malformed` saying what was wrong. After the build, [`settle`] replaces every record of a
//! name that arrived with two severities.

use crate::codes;
use crate::collect::text;
use crate::diag::{Diag, DiagCode, Position, Related, Text};
use crate::registry::ident;
use crate::site::Site;
use crate::subject::{KeyPath, Subject};
use crate::value::{Finite, Value};
use crate::Severity;
use std::collections::{BTreeMap, BTreeSet};

/// A JSON value as the step sent it, before checking.
#[derive(Clone, Debug, PartialEq)]
pub enum RawValue {
    /// `null`.
    Null,
    /// A boolean.
    Bool(bool),
    /// A number (any JSON number, as a double).
    Number(f64),
    /// A string.
    Text(String),
    /// A list.
    List(Vec<RawValue>),
    /// A table.
    Table(BTreeMap<String, RawValue>),
}

/// One related thing as the step sent it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StepRelatedIn {
    /// The subject, in any form 1.1 maps.
    pub subject: Option<String>,
    /// Position in f64 metres.
    pub position: Option<Vec<f64>>,
    /// What it has to do with the fault.
    pub note: Option<String>,
}

/// One `diag.emit` as the step sent it (`protocol.md` 3.3).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StepDiagIn {
    /// `error`, `warning` or `info`.
    pub severity: Option<String>,
    /// The `<name>` part of `step.<id>.<name>`.
    pub code: Option<String>,
    /// An entity address, key path, bare id, place name, produced name or selector.
    pub subject: Option<String>,
    /// What is wrong.
    pub message: Option<String>,
    /// What to change.
    pub hint: Option<String>,
    /// `[x, y]` or `[x, y, z]` in f64 metres.
    pub position: Option<Vec<f64>>,
    /// Other things involved.
    pub related: Vec<StepRelatedIn>,
    /// The code's facts; must be a table of strings, numbers, booleans or lists of those.
    pub data: Option<RawValue>,
    /// The text `via explain` prints for this code; sent with the first `diag.emit` of a code in
    /// a call.
    pub explain: Option<String>,
}

/// A subject as 1.1 maps it.
#[derive(Clone, Debug, PartialEq)]
pub struct Mapped {
    /// The canonical subject: an entity address with an optional key path, or a selector.
    pub subject: Subject,
    /// For a produced name: the name, which goes into `occurrences`.
    pub produced: Option<String>,
    /// For a place name: the rest of the place name, which goes in `data.place`.
    pub place: Option<String>,
}

/// What arrival needs to know of the world. Implemented by the model crate.
pub trait StepWorld {
    /// Maps a subject the step wrote to its canonical form (1.1); `None` when it names nothing
    /// in this world.
    fn map_subject(&self, written: &str) -> Option<Mapped>;
    /// Metres from a step to integer millimetres, rounded once, ties to even (`numbers.md` 3.1);
    /// `None` when not finite or out of range. Owned by `via-num`, so this crate does no rounding.
    fn to_lattice(&self, metres: f64) -> Option<i64>;
    /// True when the plan point lies inside the world's extent.
    fn in_extent(&self, x_mm: i64, y_mm: i64) -> bool;
    /// The site for a selector subject: the key in the step's table that wrote it
    /// (`for_each`), else the step's header.
    fn selector_site(&self, step_id: &str, selector: &str) -> Site;
}

/// Checks one step diagnostic and gives the record to keep: the step's, or `diag.malformed`.
pub fn arrive(step_id: &str, d: &StepDiagIn, world: &dyn StepWorld) -> Diag {
    match check(step_id, d, world) {
        Ok(diag) => diag,
        Err(missing) => malformed(step_id, d.message.as_deref().unwrap_or(""), &missing),
    }
}

fn check(step_id: &str, d: &StepDiagIn, world: &dyn StepWorld) -> Result<Diag, String> {
    let hint = d
        .hint
        .as_deref()
        .and_then(|h| Text::new(h).ok())
        .ok_or("the hint is missing or blank")?;
    let message = d
        .message
        .as_deref()
        .and_then(|m| Text::new(m).ok())
        .ok_or("the message is missing, empty or only whitespace")?;
    let name = d.code.as_deref().ok_or("the code is missing")?;
    if !ident(name) {
        return Err(format!("the code `{name}` does not match [a-z][a-z0-9_]*"));
    }
    let severity = d
        .severity
        .as_deref()
        .and_then(Severity::parse)
        .ok_or("the severity is not error, warning or info")?;
    let written = d.subject.as_deref().ok_or("the subject is missing")?;
    let mapped = world
        .map_subject(written)
        .ok_or_else(|| format!("the subject `{written}` names nothing in this world"))?;
    let site = site_for(step_id, &mapped.subject, world);
    let mut diag = Diag::with_code(
        DiagCode::Step {
            step: step_id.to_string(),
            name: name.to_string(),
            severity,
        },
        mapped.subject,
        site,
        message,
        hint,
    );
    if let Some(p) = &d.position {
        diag = diag.at(position(p, world)?);
    }
    for rel in &d.related {
        let w = rel
            .subject
            .as_deref()
            .ok_or("a related subject is missing")?;
        let m = world
            .map_subject(w)
            .ok_or_else(|| format!("the related subject `{w}` names nothing in this world"))?;
        let pos = match &rel.position {
            Some(p) => Some(position(p, world)?),
            None => None,
        };
        let note = rel
            .note
            .as_deref()
            .and_then(|n| Text::new(n).ok())
            .unwrap_or_else(|| crate::text!("named by the step"));
        let site = site_for(step_id, &m.subject, world);
        diag = diag.related(Related {
            subject: m.subject,
            site,
            position: pos,
            note,
        });
    }
    match &d.data {
        None => {}
        Some(RawValue::Table(t)) => {
            for (k, v) in t {
                let value = data_value(v, true).ok_or_else(|| {
                    format!("`data.{k}` is not a string, number, boolean or list of those")
                })?;
                diag = diag.with(k.clone(), value);
            }
        }
        Some(_) => return Err("`data` is not a table".to_string()),
    }
    if let Some(place) = mapped.place {
        diag = diag.with("place", place);
    }
    if let Some(name) = mapped.produced {
        diag = diag.produced(name);
    }
    Ok(diag)
}

/// The site of a step's subject: its key path if it has one, else its header; for a selector,
/// what the world says.
fn site_for(step_id: &str, subject: &Subject, world: &dyn StepWorld) -> Site {
    match subject {
        Subject::Selector(s) => world.selector_site(step_id, s),
        s => s.key_path().map_or(Site::Header, Site::Key),
    }
}

fn position(p: &[f64], world: &dyn StepWorld) -> Result<Position, String> {
    let bad = || format!("the position {p:?} is not finite or lies outside the world's extent");
    if p.len() != 2 && p.len() != 3 {
        return Err(format!("the position has {} numbers, not 2 or 3", p.len()));
    }
    let x = world.to_lattice(p[0]).ok_or_else(bad)?;
    let y = world.to_lattice(p[1]).ok_or_else(bad)?;
    if !world.in_extent(x, y) {
        return Err(bad());
    }
    let mut pos = Position::plan(x, y);
    if let Some(z) = p.get(2) {
        pos = pos.with_z(Finite::new(*z).ok_or_else(bad)?);
    }
    Ok(pos)
}

fn data_value(v: &RawValue, list_allowed: bool) -> Option<Value> {
    match v {
        RawValue::Bool(b) => Some(Value::Bool(*b)),
        RawValue::Number(n) => Value::double(*n),
        RawValue::Text(s) => Some(Value::Text(s.clone())),
        RawValue::List(items) if list_allowed => items
            .iter()
            .map(|i| data_value(i, false))
            .collect::<Option<Vec<_>>>()
            .map(Value::List),
        _ => None,
    }
}

/// One `diag.malformed` for the step: subject the step, at its header, with the step's message
/// and what was wrong.
fn malformed(step_id: &str, message: &str, missing: &str) -> Diag {
    Diag::new(
        codes::diag::MALFORMED,
        Subject::entity("step", step_id),
        Site::Header,
        text(format!("step `{step_id}` sent a malformed diagnostic: {missing}")),
        text(format!(
            "fix the step `{step_id}` so that every diagnostic it sends has a code, a subject this world has, a message and a hint"
        )),
    )
    .with("message", message)
    .with("missing", missing)
}

/// Section 6: when one step code arrives with two severities in one build, every record of that
/// name is replaced by one `diag.malformed`. Other diagnostics pass through in order.
pub fn settle(diags: Vec<Diag>) -> Vec<Diag> {
    let mut severities: BTreeMap<(String, String), BTreeSet<Severity>> = BTreeMap::new();
    for d in &diags {
        if let DiagCode::Step {
            step,
            name,
            severity,
        } = d.code()
        {
            severities
                .entry((step.clone(), name.clone()))
                .or_default()
                .insert(*severity);
        }
    }
    let mut reported = BTreeSet::new();
    let mut out = Vec::with_capacity(diags.len());
    for d in diags {
        let key = match d.code() {
            DiagCode::Step { step, name, .. } => Some((step.clone(), name.clone())),
            DiagCode::Core(_) => None,
        };
        match key {
            Some(k) if severities.get(&k).is_some_and(|s| s.len() > 1) => {
                if reported.insert(k.clone()) {
                    let words: Vec<&str> = severities[&k].iter().map(|s| s.as_str()).collect();
                    out.push(malformed(
                        &k.0,
                        "",
                        &format!("the code `{}` arrived as {}", k.1, words.join(" and ")),
                    ));
                }
            }
            _ => out.push(d),
        }
    }
    out
}

/// The `explain` texts steps sent, one per code, and the protocol warnings about them
/// (`protocol.md` 3.3: the first `diag.emit` of a code in a call carries the text). The book
/// lives for the build, so two calls that send different texts are caught; which codes a call
/// has sent is reset by [`Explains::begin_call`], so each call of a `for_each` step must send it.
#[derive(Clone, Debug, Default)]
pub struct Explains {
    texts: BTreeMap<String, Option<String>>,
    in_call: BTreeSet<String>,
    warned_differs: BTreeSet<String>,
}

impl Explains {
    /// An empty book.
    pub fn new() -> Explains {
        Explains::default()
    }

    /// Starts a new call of a step: the next `diag.emit` of each code is again its first.
    pub fn begin_call(&mut self) {
        self.in_call.clear();
    }

    /// Records one arrival of `step.<id>.<name>` and gives `protocol.no_explain` when the code's
    /// first arrival in this call has no text, or `protocol.explain_differs` once per build when
    /// a text differs from the first one recorded.
    pub fn observe(&mut self, step_id: &str, name: &str, explain: Option<&str>) -> Option<Diag> {
        let code = format!("step.{step_id}.{name}");
        let explain = explain.filter(|e| !e.trim().is_empty());
        let first_in_call = self.in_call.insert(code.clone());
        let recorded = match (self.texts.get(&code), explain) {
            (None, _) | (Some(None), Some(_)) => {
                self.texts.insert(code.clone(), explain.map(str::to_string));
                None
            }
            (Some(r), _) => r.clone(),
        };
        if first_in_call && explain.is_none() {
            return Some(Diag::new(
                codes::protocol::NO_EXPLAIN,
                Subject::entity("step", step_id),
                Site::Header,
                text(format!("step `{step_id}` sent `{code}` with no `explain` text")),
                text(format!(
                    "send `explain` with the first `diag.emit` of `{name}` in each call: what it means, what to change, an example"
                )),
            ));
        }
        match (recorded, explain) {
            (Some(a), Some(b)) if a != b && self.warned_differs.insert(code.clone()) => {
                Some(Diag::new(
                    codes::protocol::EXPLAIN_DIFFERS,
                    Subject::entity("step", step_id),
                    Site::Header,
                    text(format!(
                        "step `{step_id}` sent two `explain` texts for `{code}`"
                    )),
                    text(format!("send one fixed `explain` text for `{name}`")),
                ))
            }
            _ => None,
        }
    }

    /// The text recorded for a step code, if one came.
    pub fn get(&self, code: &str) -> Option<&str> {
        self.texts.get(code).and_then(|t| t.as_deref())
    }

    /// Every step code seen, bytewise, with its text if one came.
    pub fn entries(&self) -> impl Iterator<Item = (&str, Option<&str>)> {
        self.texts.iter().map(|(k, v)| (k.as_str(), v.as_deref()))
    }
}

/// Keeps a key path type in this module's public signatures discoverable for implementers of
/// [`StepWorld::selector_site`], which usually return `Site::Selector(KeyPath::key("for_each"))`.
pub fn for_each_site() -> Site {
    Site::Selector(KeyPath::key("for_each"))
}
