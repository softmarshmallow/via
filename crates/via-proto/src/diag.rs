//! The step's place in the text, for the protocol's diagnostics.
//!
//! Every fault of a worker is reported as one `via_diag::Diag` on the subject `step.<id>`, at
//! `Site::Header`: the `[step.<id>]` header an author would edit (`spec/diagnostics.md` 1 and
//! 1.2). Severity comes from the code and codes are the registry's generated constants
//! (`via_diag::codes::protocol`), so an unregistered code does not compile.
//!
//! In M2 the document reader resolves the header's span. The probe runs outside the parser, so
//! its harness names the span itself, and [`StepSite`] is the resolver for that one header.

pub use via_diag::Diag;
use via_diag::{terminal, Code, Location, Record, Resolve, Resolved, Site, Subject, Text};

/// Where a step is declared: the span of its `[step.<id>]` header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepSite {
    pub id: String,
    pub file: String,
    pub line: u32,
    pub column: u32,
    /// `[start, end)` in bytes of the header (1.2 requires bytes with every line).
    pub bytes: [u32; 2],
}

impl StepSite {
    /// The diagnostic subject of the step: `step.<id>` (`diagnostics.md` 1.1).
    pub fn subject(&self) -> Subject {
        Subject::entity("step", self.id.clone())
    }

    /// A diagnostic of this step at its header. `message` is built from facts, so it is never
    /// blank; should it be, a fixed text stands in rather than a panic.
    pub fn diag(&self, code: Code, message: String, hint: Text) -> Diag {
        let message = Text::new(message).unwrap_or_else(|_| via_diag::text!("(no message)"));
        Diag::new(code, self.subject(), Site::Header, message, hint)
    }

    /// The located record, resolved against this header alone.
    pub fn record(&self, d: &Diag) -> Record {
        d.emit(self)
    }

    /// The diagnostic as the terminal shows it (`diagnostics.md` 7), for the probe's harness.
    pub fn render(&self, d: &Diag) -> String {
        let opts = terminal::Options {
            info: true,
            colour: false,
        };
        terminal::render(&[self.record(d)], &[], opts)
    }
}

impl Resolve for StepSite {
    fn resolve(&self, subject: &Subject, site: &Site) -> Option<Resolved> {
        if *subject != self.subject() || *site != Site::Header {
            return None;
        }
        Some(Resolved {
            location: Location::Span {
                file: self.file.clone(),
                line: self.line,
                column: self.column,
                bytes: self.bytes,
            },
            chain: Vec::new(),
        })
    }

    fn world_file(&self) -> String {
        self.file.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use via_diag::codes;

    #[test]
    fn a_step_diag_is_located_at_its_header_with_bytes() {
        let site = StepSite {
            id: "probe".into(),
            file: "probe/world.toml".into(),
            line: 1,
            column: 1,
            bytes: [0, 13],
        };
        let d = site
            .diag(
                codes::protocol::DEADLINE,
                "late".into(),
                via_diag::text!("be faster"),
            )
            .with("phase", "cancel")
            .with("deadline", via_diag::Value::double(600.0).unwrap());
        assert!(d.undeclared_data().is_empty(), "{:?}", d.undeclared_data());
        let r = site.record(&d);
        let json = r.to_canonical_json();
        assert!(json.contains(r#""bytes":[0,13]"#), "{json}");
        assert!(
            json.contains(r#""deadline":600,"#),
            "numbers.md 3.4: {json}"
        );
        assert!(json.contains(r#""subject":"step.probe""#), "{json}");
        assert!(json.contains(r#""severity":"error""#), "{json}");
    }
}
