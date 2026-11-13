//! Faults found while reading, located directly (`spec/diagnostics.md` 1.2).
//!
//! A fault found in reading is not stored with a node and replayed: the reader runs on every
//! build. So each one is located as it is found, from its span, and emitted through `via-diag`'s
//! one constructor with a resolver that knows only that fault's spans ([`Spot`]).

use crate::source::{Sources, Span};
use crate::tree::Link;
use via_diag::{
    ChainEntry, Diag, KeyPath, Record, Related, Resolve, Resolved, Site, Subject, Text, Value,
};

/// Where a fault is.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum At {
    /// Text.
    Span(Span),
    /// A command-line argument, 1-based after the program name.
    Argument(u32),
    /// A file that is not text, or not readable as text, with facts locating inside it.
    File(String, Vec<(String, Value)>),
    /// The world file at 1:1.
    World,
}

/// A fault before it is emitted.
#[derive(Clone, Debug)]
pub(crate) struct Fault {
    pub code: via_diag::Code,
    pub subject: Subject,
    pub at: At,
    pub message: String,
    pub hint: String,
    pub data: Vec<(String, Value)>,
    pub related: Vec<(Subject, At, String)>,
    pub chain: Vec<Link>,
}

impl Fault {
    pub(crate) fn new(
        code: via_diag::Code,
        subject: Subject,
        at: At,
        message: impl Into<String>,
        hint: impl Into<String>,
    ) -> Fault {
        Fault {
            code,
            subject,
            at,
            message: message.into(),
            hint: hint.into(),
            data: Vec::new(),
            related: Vec::new(),
            chain: Vec::new(),
        }
    }

    #[must_use]
    pub(crate) fn with(mut self, key: &str, value: impl Into<Value>) -> Fault {
        self.data.push((key.to_string(), value.into()));
        self
    }

    #[must_use]
    pub(crate) fn related(mut self, subject: Subject, at: At, note: impl Into<String>) -> Fault {
        self.related.push((subject, at, note.into()));
        self
    }

    #[must_use]
    pub(crate) fn chain(mut self, chain: &[Link]) -> Fault {
        self.chain = chain.to_vec();
        self
    }

    /// The code as written.
    pub(crate) fn code_str(&self) -> &'static str {
        self.code.as_str()
    }

    /// The record, located against `sources`.
    pub(crate) fn emit(&self, sources: &Sources, world_file: &str) -> Record {
        let mut spot = Spot {
            spots: Vec::new(),
            world_file: world_file.to_string(),
        };
        let site = spot.place(
            &self.subject,
            &self.at,
            0,
            chain_entries(sources, &self.chain),
            sources,
        );
        let mut diag = Diag::new(
            self.code,
            self.subject.clone(),
            site,
            text(&self.message),
            text(&self.hint),
        );
        for (k, v) in &self.data {
            diag = diag.with(k.clone(), v.clone());
        }
        for (i, (subject, at, note)) in self.related.iter().enumerate() {
            let site = spot.place(subject, at, i + 1, Vec::new(), sources);
            diag = diag.related(Related {
                subject: subject.clone(),
                site,
                position: None,
                note: text(note),
            });
        }
        diag.emit(&spot)
    }
}

/// A message or hint as `Text`; a blank one (a defect of ours) becomes a fixed word, so a fault
/// is never lost for want of text.
pub(crate) fn text(s: &str) -> Text {
    Text::new(s).unwrap_or_else(|_| via_diag::text!("(no text)"))
}

pub(crate) fn chain_entries(sources: &Sources, chain: &[Link]) -> Vec<ChainEntry> {
    chain
        .iter()
        .map(|l| ChainEntry {
            through: l.through,
            name: l.name.clone(),
            file: sources.name(l.span.src).to_string(),
            line: sources.line(l.span),
            owner: l.owner.clone(),
        })
        .collect()
}

/// A resolver that knows the spans of one fault and its related things, each under its own
/// site, so two related things with one subject still resolve to their own places.
struct Spot {
    spots: Vec<(Subject, Site, Resolved)>,
    world_file: String,
}

impl Spot {
    fn place(
        &mut self,
        subject: &Subject,
        at: &At,
        index: usize,
        chain: Vec<ChainEntry>,
        sources: &Sources,
    ) -> Site {
        match at {
            At::Argument(n) => Site::Argument(*n),
            At::File(file, locator) => Site::File {
                file: file.clone(),
                locator: locator.iter().cloned().collect(),
            },
            At::World => Site::World,
            At::Span(span) => {
                // A site private to this resolver: `Header` for the record, a numbered key for
                // each related thing.
                let site = if index == 0 {
                    Site::Header
                } else {
                    Site::Key(KeyPath::key(format!("related{index}")))
                };
                self.spots.push((
                    subject.clone(),
                    site.clone(),
                    Resolved {
                        location: sources.location(*span),
                        chain,
                    },
                ));
                site
            }
        }
    }
}

impl Resolve for Spot {
    fn resolve(&self, subject: &Subject, site: &Site) -> Option<Resolved> {
        self.spots
            .iter()
            .find(|(s, t, _)| s == subject && t == site)
            .map(|(_, _, r)| r.clone())
    }

    fn world_file(&self) -> String {
        self.world_file.clone()
    }
}

/// Faults gathered by one step of reading.
#[derive(Debug, Default)]
pub(crate) struct Faults {
    pub list: Vec<Fault>,
}

impl Faults {
    pub(crate) fn push(&mut self, f: Fault) {
        self.list.push(f);
    }

    pub(crate) fn extend(&mut self, fs: impl IntoIterator<Item = Fault>) {
        self.list.extend(fs);
    }

    /// True when any fault is an error.
    pub(crate) fn has_error(&self) -> bool {
        self.list
            .iter()
            .any(|f| f.code.severity() == via_diag::Severity::Error)
    }
}
