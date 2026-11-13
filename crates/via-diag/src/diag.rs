//! The diagnostic as a node builds and stores it (`spec/diagnostics.md` 1, "Construction").

use crate::codes;
use crate::record::{Record, RelatedRecord};
use crate::registry::Code;
use crate::site::{Location, Resolve, Resolved, Site};
use crate::subject::Subject;
use crate::value::{Finite, Value};
use crate::Severity;
use std::collections::BTreeMap;
use std::fmt;

/// A message or hint: text that holds at least one letter, so neither blank nor a placeholder
/// such as `-` (1.3: "check your input" is not a hint, and no hint is not a hint either).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Text(String);

/// The text given for a message or hint was blank.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlankText;

impl fmt::Display for BlankText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "a diagnostic's message and hint must hold a letter, not be blank or a placeholder",
        )
    }
}

impl std::error::Error for BlankText {}

impl Text {
    /// The text, or `BlankText` when it holds no letter: empty, white space, or a placeholder
    /// such as `-` or `?`.
    pub fn new(s: impl Into<String>) -> Result<Text, BlankText> {
        let s = s.into();
        if !s.chars().any(char::is_alphabetic) {
            Err(BlankText)
        } else {
            Ok(Text(s))
        }
    }

    /// For the [`text!`](crate::text) macro, which has checked the literal at compile time.
    #[doc(hidden)]
    pub fn from_checked_static(s: &'static str) -> Text {
        Text(s.to_string())
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A place in the world, in the top world's frame (1.2). `x` and `y` are integer millimetres on
/// the lattice (R24); `z`, a height, is a double off the lattice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    x: i64,
    y: i64,
    z: Option<Finite>,
}

impl Position {
    /// A plan position in millimetres.
    pub fn plan(x_mm: i64, y_mm: i64) -> Position {
        Position {
            x: x_mm,
            y: y_mm,
            z: None,
        }
    }

    /// The same position with a height in metres.
    #[must_use]
    pub fn with_z(mut self, z: Finite) -> Position {
        self.z = Some(z);
        self
    }

    /// `x` in millimetres.
    pub fn x_mm(&self) -> i64 {
        self.x
    }

    /// `y` in millimetres.
    pub fn y_mm(&self) -> i64 {
        self.y
    }

    /// The height in metres, if given.
    pub fn z(&self) -> Option<Finite> {
        self.z
    }

    /// Section 5's position order: `y`, then `x`, then `z` (absent first).
    pub(crate) fn order(&self, other: &Position) -> std::cmp::Ordering {
        self.y
            .cmp(&other.y)
            .then(self.x.cmp(&other.x))
            .then_with(|| match (self.z, other.z) {
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (Some(_), None) => std::cmp::Ordering::Greater,
                (Some(a), Some(b)) => a.get().total_cmp(&b.get()),
            })
    }

    pub(crate) fn to_value_list(self) -> Vec<Value> {
        let mut v = vec![Value::Length(self.x), Value::Length(self.y)];
        if let Some(z) = self.z {
            v.push(Value::Double(z));
        }
        v
    }
}

/// Another thing involved (1, `related`): resolved at emission like the record's own site.
#[derive(Clone, Debug, PartialEq)]
pub struct Related {
    /// The other thing.
    pub subject: Subject,
    /// Where its text is.
    pub site: Site,
    /// Where it is in the world, if anywhere.
    pub position: Option<Position>,
    /// What it has to do with the fault.
    pub note: Text,
}

/// The code of a diagnostic: a registered core code, or a step's own `step.<id>.<name>`, which
/// only [`crate::step::arrive`] makes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagCode {
    /// A core code from the registry.
    Core(Code),
    /// A step's own code. Severity is the step's, held one per name per build (section 6).
    Step {
        /// The step's id.
        step: String,
        /// The `<name>` the step sent.
        name: String,
        /// The severity it sent.
        severity: Severity,
    },
}

impl DiagCode {
    /// The severity: the registry's for a core code.
    pub fn severity(&self) -> Severity {
        match self {
            DiagCode::Core(c) => c.severity(),
            DiagCode::Step { severity, .. } => *severity,
        }
    }
}

impl fmt::Display for DiagCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagCode::Core(c) => f.write_str(c.as_str()),
            DiagCode::Step { step, name, .. } => write!(f, "step.{step}.{name}"),
        }
    }
}

/// A diagnostic as a node raises and stores it. It holds no file, line or byte offset: those
/// come at emission, so a stored diagnostic replays correctly after text moves.
///
/// The fields sit behind one box, so a `Result<_, Diag>` stays a pointer wide on its error side
/// and errors-as-values cost nothing on the success path.
#[derive(Clone, Debug, PartialEq)]
pub struct Diag(Box<DiagInner>);

#[derive(Clone, Debug, PartialEq)]
struct DiagInner {
    code: DiagCode,
    subject: Subject,
    site: Site,
    message: Text,
    hint: Text,
    position: Option<Position>,
    related: Vec<Related>,
    data: BTreeMap<String, Value>,
    name: Option<String>,
    count: Option<u32>,
}

impl Diag {
    /// The one constructor: code, subject, site, message and hint are all required, so a
    /// diagnostic without them does not compile. Severity is the code's.
    pub fn new(code: Code, subject: Subject, site: Site, message: Text, hint: Text) -> Diag {
        Diag::with_code(DiagCode::Core(code), subject, site, message, hint)
    }

    pub(crate) fn with_code(
        code: DiagCode,
        subject: Subject,
        site: Site,
        message: Text,
        hint: Text,
    ) -> Diag {
        // 1.3's nearest name is set by the lookup that found it ([`Diag::nearest_name`]),
        // never read back from the hint's wording.
        let data = BTreeMap::new();
        Diag(Box::new(DiagInner {
            code,
            subject,
            site,
            message,
            hint,
            position: None,
            related: Vec::new(),
            data,
            name: None,
            count: None,
        }))
    }

    /// Where in the world (1.2). Any fault with a place gives one.
    #[must_use]
    pub fn at(mut self, position: Position) -> Diag {
        self.0.position = Some(position);
        self
    }

    /// One `data` fact. The registry row lists the names a code's records carry; see
    /// [`Diag::undeclared_data`].
    #[must_use]
    pub fn with(mut self, key: impl Into<String>, value: impl Into<Value>) -> Diag {
        self.0.data.insert(key.into(), value.into());
        self
    }

    /// Another thing involved.
    #[must_use]
    pub fn related(mut self, related: Related) -> Diag {
        self.0.related.push(related);
        self
    }

    /// The nearest name of a not-found record (1.3), as the lookup found it: `data.nearest`
    /// holds it, and when the world declares it in text, `related` points at that declaration
    /// (`declared` is its subject and site) with the note "declared here".
    #[must_use]
    pub fn nearest_name(self, name: &str, declared: Option<(Subject, Site)>) -> Diag {
        let d = self.with("nearest", Value::text(name.to_string()));
        match declared {
            Some((subject, site)) => d.related(Related {
                subject,
                site,
                position: None,
                note: crate::text!("declared here"),
            }),
            None => d,
        }
    }

    /// The produced name this occurrence is about (1.1): the subject is its producer, and the
    /// name goes into `occurrences`.
    #[must_use]
    pub fn produced(mut self, name: impl Into<String>) -> Diag {
        self.0.name = Some(name.into());
        self
    }

    /// A `count` the record states itself rather than by aggregation (`diag.skipped`: the
    /// skipped subjects). It is always written, even when 1.
    #[must_use]
    pub fn counted(mut self, count: u32) -> Diag {
        self.0.count = Some(count);
        self
    }

    /// The code.
    pub fn code(&self) -> &DiagCode {
        &self.0.code
    }

    /// The severity, fixed by the code.
    pub fn severity(&self) -> Severity {
        self.0.code.severity()
    }

    /// The subject.
    pub fn subject(&self) -> &Subject {
        &self.0.subject
    }

    /// The site.
    pub fn site(&self) -> &Site {
        &self.0.site
    }

    /// The message.
    pub fn message(&self) -> &Text {
        &self.0.message
    }

    /// The hint.
    pub fn hint(&self) -> &Text {
        &self.0.hint
    }

    /// The other things involved.
    pub fn related_list(&self) -> &[Related] {
        &self.0.related
    }

    /// The position, if any.
    pub fn position(&self) -> Option<Position> {
        self.0.position
    }

    /// The `data` facts.
    pub fn data(&self) -> &BTreeMap<String, Value> {
        &self.0.data
    }

    /// The produced name, if any.
    pub fn produced_name(&self) -> Option<&str> {
        self.0.name.as_deref()
    }

    /// The count the record states itself ([`Diag::counted`]), if any.
    pub fn stated_count(&self) -> Option<u32> {
        self.0.count
    }

    /// `data` names that the code's registry row does not list (section 2: a registry defect when
    /// non-empty). Empty for a step code, whose fields the step chooses. Conformance and tests
    /// call this; the engine does not refuse such a record, since refusing would lose the fault.
    pub fn undeclared_data(&self) -> Vec<&str> {
        match &self.0.code {
            DiagCode::Core(c) => {
                let row = c.row();
                let mut names: Vec<&str> = self
                    .0
                    .data
                    .keys()
                    .map(String::as_str)
                    .filter(|k| !row.data.contains(k))
                    .collect();
                if let Site::File { locator, .. } = &self.0.site {
                    names.retain(|k| !locator.contains_key(*k));
                }
                names
            }
            DiagCode::Step { .. } => Vec::new(),
        }
    }

    /// Emission: resolves the site and every related site against the current merge. A site
    /// that does not resolve is a defect of ours, so the record becomes `diag.internal` naming
    /// the code and the site, at the world file's start.
    pub fn emit(&self, resolver: &dyn Resolve) -> Record {
        let resolved = match resolve(resolver, &self.0.subject, &self.0.site) {
            Some(r) => r,
            None => return self.unresolved(resolver, &self.0.site),
        };
        let mut related = Vec::with_capacity(self.0.related.len());
        for rel in &self.0.related {
            match resolve(resolver, &rel.subject, &rel.site) {
                Some(r) => related.push(RelatedRecord {
                    subject: rel.subject.to_string(),
                    location: r.location,
                    position: rel.position,
                    note: rel.note.as_str().to_string(),
                }),
                None => return self.unresolved(resolver, &rel.site),
            }
        }
        let mut data = self.0.data.clone();
        if let Site::File { locator, .. } = &self.0.site {
            for (k, v) in locator {
                data.entry(k.clone()).or_insert_with(|| v.clone());
            }
        }
        Record::new(
            self.severity(),
            self.0.code.to_string(),
            &self.0.subject,
            self.0.message.as_str().to_string(),
            self.0.hint.as_str().to_string(),
            resolved,
            self.0.position,
            related,
            data,
            self.0.name.clone(),
            self.0.count,
        )
    }

    fn unresolved(&self, resolver: &dyn Resolve, site: &Site) -> Record {
        let message = format!(
            "{} on `{}` was raised at {}, which has no span in this merge",
            self.0.code,
            self.0.subject,
            site.describe()
        );
        let internal = Diag::new(
            codes::diag::INTERNAL,
            self.0.subject.clone(),
            Site::World,
            Text(format!("a diagnostic could not be located: {message}")),
            Text("report it: this is a defect of the engine, not of the text".to_string()),
        )
        .with("message", message);
        internal.emit(resolver)
    }
}

fn resolve(resolver: &dyn Resolve, subject: &Subject, site: &Site) -> Option<Resolved> {
    match site {
        Site::Argument(n) => Some(Resolved {
            location: Location::Argument(*n),
            chain: Vec::new(),
        }),
        Site::File { file, .. } => Some(Resolved {
            location: Location::File { file: file.clone() },
            chain: Vec::new(),
        }),
        Site::Text {
            file,
            line,
            column,
            bytes,
        } => Some(Resolved {
            location: Location::Span {
                file: file.clone(),
                line: *line,
                column: *column,
                bytes: *bytes,
            },
            chain: Vec::new(),
        }),
        Site::World => Some(Resolved {
            location: Location::world_start(resolver.world_file()),
            chain: Vec::new(),
        }),
        _ => resolver.resolve(subject, site),
    }
}
