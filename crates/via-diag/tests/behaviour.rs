//! The record, its JSON, once, aggregation, order, `--fail-on`, exit status, step arrival and
//! `via explain` (`spec/diagnostics.md` sections 1 to 8). Names are neutral, so the deny-list
//! (`spec/vocabulary.md` 7) passes this file too.

use std::collections::BTreeMap;
use via_diag::step::{self, Mapped, RawValue, StepDiagIn, StepWorld};
use via_diag::{
    codes, document, exit_status, fail_on_code, finish, text, ChainEntry, Diag, FailOn, Finite,
    KeyPath, Lint, Location, Position, Record, Resolve, Resolved, Site, Subject, Text, Through,
    Value,
};

/// A resolver with a few fixed spans, standing in for the document reader.
struct Spans;

impl Resolve for Spans {
    fn resolve(&self, subject: &Subject, site: &Site) -> Option<Resolved> {
        let s = subject.to_string();
        match (s.as_str(), site) {
            ("scatter.shrubs.density", Site::Expr(_, r)) if r.start == 65 => Some(Resolved {
                location: Location::Span {
                    file: "changes/bad_typo.toml".into(),
                    line: 2,
                    column: 67,
                    bytes: [82, 84],
                },
                chain: vec![ChainEntry {
                    through: Through::Change,
                    name: "bad_typo".into(),
                    file: "changes/bad_typo.toml".into(),
                    line: 2,
                    owner: None,
                }],
            }),
            (_, Site::Header) | (_, Site::Key(_)) => Some(Resolved {
                location: Location::Span {
                    file: "world.toml".into(),
                    line: 77,
                    column: 1,
                    bytes: [2210, 2223],
                },
                chain: Vec::new(),
            }),
            _ => None,
        }
    }

    fn world_file(&self) -> String {
        "world.toml".into()
    }
}

fn t(s: &str) -> Text {
    Text::new(s).unwrap()
}

fn subj(s: &str) -> Subject {
    Subject::parse(s).unwrap()
}

#[test]
fn worked_example_one() {
    let d = Diag::new(
        codes::formula::UNKNOWN_NAME,
        subj("scatter.shrubs.density"),
        Site::Expr(KeyPath::key("density"), 65..67),
        t("no field, let or attribute named `xx`"),
        text!("write `x`"),
    )
    .with("name", "xx")
    .with("nearest", "x");
    let r = d.emit(&Spans);
    assert_eq!(
        r.to_canonical_json(),
        concat!(
            r#"{"bytes":[82,84],"chain":[{"file":"changes/bad_typo.toml","line":2,"name":"bad_typo","through":"change"}],"#,
            r#""code":"formula.unknown_name","column":67,"data":{"name":"xx","nearest":"x"},"#,
            r#""file":"changes/bad_typo.toml","hint":"write `x`","line":2,"#,
            r#""message":"no field, let or attribute named `xx`","severity":"error","subject":"scatter.shrubs.density"}"#
        )
    );
    // The chapter's own example carries `data.name`, and its row now names it (section 2).
    assert!(d.undeclared_data().is_empty());
    assert_eq!(exit_status(&[r]), 1);
}

fn relief(name: &str, x: i64, y: i64, relief: f64) -> Record {
    Diag::new(
        codes::place::RELIEF,
        subj("step.infill"),
        Site::Header,
        t("instances stand on ground that varies more than their asset allows"),
        t("move them"),
    )
    .at(Position::plan(x, y))
    .with("relief", Value::double(relief).unwrap())
    .produced(name)
    .emit(&Spans)
}

/// Every permutation of `items` (Heap's algorithm), in a fixed order.
fn permutations<T: Clone>(items: &[T]) -> Vec<Vec<T>> {
    fn go<T: Clone>(k: usize, a: &mut Vec<T>, out: &mut Vec<Vec<T>>) {
        if k <= 1 {
            out.push(a.clone());
            return;
        }
        for i in 0..k - 1 {
            go(k - 1, a, out);
            if k.is_multiple_of(2) {
                a.swap(i, k - 1);
            } else {
                a.swap(0, k - 1);
            }
        }
        go(k - 1, a, out);
    }
    let mut a = items.to_vec();
    let mut out = Vec::new();
    go(a.len(), &mut a, &mut out);
    out
}

/// A fixed shuffle (a multiplicative step through the indices), so a test's input is not already
/// in the order the code sorts into.
fn shuffled<T: Clone>(items: &[T]) -> Vec<T> {
    let n = items.len();
    let step = (1..n).rev().find(|s| gcd(*s, n) == 1).unwrap_or(1);
    (0..n).map(|i| items[(i * step + 3) % n].clone()).collect()
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

#[test]
fn a_producer_folds_into_one_record_with_five_occurrences() {
    let ordered: Vec<Record> = (0..166)
        .map(|i| {
            relief(
                &format!("infill#h{i}"),
                1000 * (i % 7),
                64_000 + i,
                0.5 + i as f64 / 100.0,
            )
        })
        .collect();
    let records = shuffled(&ordered);
    assert_ne!(records, ordered, "the input is not already sorted");
    let out = finish(records, &FailOn::none());
    assert_eq!(
        out,
        finish(ordered.iter().rev().cloned().collect(), &FailOn::none())
    );
    assert_eq!(out.len(), 1);
    let r = &out[0];
    assert_eq!(r.count(), 166);
    assert_eq!(r.occurrences().len(), 5);
    // No `worst` field in place.relief's row, so the first five by position: y, then x.
    let names: Vec<&str> = r
        .occurrences()
        .iter()
        .filter_map(|o| o.name.as_deref())
        .collect();
    assert_eq!(
        names,
        [
            "infill#h0",
            "infill#h1",
            "infill#h2",
            "infill#h3",
            "infill#h4"
        ]
    );
    let json = r.to_canonical_json();
    assert!(json.contains(r#""count":166"#), "{json}");
    assert!(
        json.contains(r#"{"name":"infill#h0","position":[0,64]}"#),
        "{json}"
    );
    assert!(json.contains(r#""position":[0,64]"#));
}

#[test]
fn records_tied_on_position_and_name_fold_the_same_in_any_arrival_order() {
    // Equal position and name, different data: only the canonical JSON tells them apart. Seven
    // records, so which five become occurrences depends on the order too.
    let tied = vec![
        relief("infill#a", 0, 0, 2.0),
        relief("infill#a", 0, 0, 1.0),
        relief("infill#a", 0, 0, 3.0),
        relief("infill#b", 0, 0, 1.0),
        relief("infill#b", 0, 0, 0.5),
        relief("infill#a", 1000, 0, 1.0),
        relief("infill#a", 1000, 0, 4.0),
    ];
    let lints = [Lint {
        step: "infill".into(),
        ran: true,
    }];
    let first = finish(tied.clone(), &FailOn::none());
    let expected = document(&first, &lints, exit_status(&first));
    let all = permutations(&tied);
    assert_eq!(all.len(), 5040);
    for input in all {
        let out = finish(input, &FailOn::none());
        assert_eq!(out, first);
        assert_eq!(document(&out, &lints, exit_status(&out)), expected);
    }
    // The fold keeps the lowest record of the total order: position, name, then canonical JSON,
    // where `"relief":1` sorts before `"relief":2`.
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].count(), 7);
    let json = first[0].to_canonical_json();
    assert!(json.contains(r#""data":{"relief":1}"#), "{json}");
}

#[test]
fn hand_instances_are_never_folded() {
    let records: Vec<Record> = (0..10)
        .map(|i| {
            Diag::new(
                codes::place::RELIEF,
                Subject::entity("instance", format!("i{i}")),
                Site::Header,
                t("steep"),
                t("move it"),
            )
            .at(Position::plan(0, 0))
            .emit(&Spans)
        })
        .collect();
    assert_eq!(finish(records, &FailOn::none()).len(), 10);
}

#[test]
fn once_drops_identical_records_and_keeps_distinct_names() {
    let a = relief("infill#a", 0, 0, 1.0);
    let b = relief("infill#b", 0, 0, 1.0);
    let out = finish(vec![a.clone(), a, b], &FailOn::none());
    assert_eq!(out[0].count(), 2, "two names at one position count twice");
}

#[test]
fn order_is_total_and_independent_of_input_order() {
    let err = Diag::new(
        codes::num::RESOLUTION,
        Subject::World,
        Site::World,
        t("m"),
        t("h"),
    )
    .emit(&Spans);
    let cli = Diag::new(
        codes::cli::USAGE,
        Subject::Cli,
        Site::Argument(2),
        t("m"),
        t("h"),
    )
    .emit(&Spans);
    let warn = relief("infill#a", 5, 5, 1.0);
    let info = Diag::new(
        codes::set::UNUSED,
        subj("set.s"),
        Site::Header,
        t("m"),
        t("h"),
    )
    .emit(&Spans);
    // Two more warnings tied with `warn` on position and name, differing only in data.
    let warn2 = relief("infill#a", 5, 5, 2.0);
    let warn3 = relief("infill#a", 5, 5, 0.5);
    let inputs = [info, warn, err, cli, warn2, warn3];
    let one = finish(inputs.to_vec(), &FailOn::none());
    let one_doc = document(&one, &[], exit_status(&one));
    for p in permutations(&inputs) {
        let out = finish(p, &FailOn::none());
        assert_eq!(out, one);
        assert_eq!(document(&out, &[], exit_status(&out)), one_doc);
    }
    let two = finish(inputs.iter().rev().cloned().collect(), &FailOn::none());
    assert_eq!(one, two);
    let codes: Vec<&str> = one.iter().map(Record::code).collect();
    assert_eq!(
        codes,
        ["cli.usage", "num.resolution", "place.relief", "set.unused"]
    );
    let doc = document(&one, &[], exit_status(&one));
    assert_eq!(doc, document(&two, &[], exit_status(&two)));
    assert!(doc.ends_with("}\n") && !doc.ends_with("\n\n"));
    assert!(doc.starts_with(
        r#"{"counts":{"error":2,"failed_on":0,"info":1,"warning":1},"diagnostics":["#
    ));
    assert!(doc.ends_with("\"format\":\"via-diagnostics-1\",\"lints\":[],\"status\":2}\n"));
}

#[test]
fn fail_on_marks_warnings_only() {
    assert_eq!(fail_on_code("place.relief", 3).unwrap(), "place.relief");
    assert_eq!(
        fail_on_code("step.checks.too_steep", 3).unwrap(),
        "step.checks.too_steep"
    );
    let usage = fail_on_code("num.resolution", 3).unwrap_err();
    assert_eq!(usage.code().to_string(), "cli.usage");
    let unknown = fail_on_code("place.relif", 3).unwrap_err();
    assert_eq!(unknown.code().to_string(), "cli.unknown_code");
    assert_eq!(unknown.data()["nearest"], Value::from("place.relief"));

    let mut f = FailOn::none();
    f.insert("place.relief".into());
    let out = finish(vec![relief("infill#a", 0, 0, 1.0)], &f);
    assert!(out[0].failed_on());
    assert_eq!(out[0].severity().as_str(), "warning");
    assert_eq!(exit_status(&out), 1);
    let doc = document(&out, &[], 1);
    assert!(doc.contains(r#""counts":{"error":0,"failed_on":1,"info":0,"warning":1}"#));
    assert!(doc.contains(r#""failed_on":true"#));
    assert_eq!(
        exit_status(&finish(
            vec![relief("infill#a", 0, 0, 1.0)],
            &FailOn::none()
        )),
        0
    );
}

#[test]
fn exit_status_ranks_internal_over_cli_over_error() {
    let internal = Diag::new(
        codes::geom::INTERNAL,
        Subject::World,
        Site::World,
        t("m"),
        t("h"),
    )
    .emit(&Spans);
    let cli = Diag::new(
        codes::cli::USAGE,
        Subject::Cli,
        Site::Argument(1),
        t("m"),
        t("h"),
    )
    .emit(&Spans);
    let err = Diag::new(
        codes::num::RESOLUTION,
        Subject::World,
        Site::World,
        t("m"),
        t("h"),
    )
    .emit(&Spans);
    assert_eq!(exit_status(&[err.clone(), internal]), 3);
    assert_eq!(exit_status(&[err.clone(), cli]), 2);
    assert_eq!(exit_status(&[err]), 1);
    assert_eq!(exit_status(&[]), 0);
}

#[test]
fn an_unresolved_site_becomes_diag_internal() {
    let r = Diag::new(
        codes::formula::UNKNOWN_NAME,
        subj("scatter.shrubs.density"),
        Site::Expr(KeyPath::key("density"), 1..2),
        t("m"),
        t("h"),
    )
    .emit(&Spans);
    assert_eq!(r.code(), "diag.internal");
    assert_eq!(r.subject(), "scatter.shrubs.density");
    assert_eq!(*r.location(), Location::world_start("world.toml"));
    assert_eq!(exit_status(&[r]), 3);
}

#[test]
fn positions_write_exact_metres_and_shortest_heights() {
    let r = Diag::new(
        codes::place::KEEP_OUT,
        subj("instance.a"),
        Site::Header,
        t("m"),
        t("h"),
    )
    .at(Position::plan(181_500, 64_000).with_z(Finite::new(12.25).unwrap()))
    .emit(&Spans);
    assert!(r
        .to_canonical_json()
        .contains(r#""position":[181.5,64,12.25]"#));
}

#[test]
fn file_sites_merge_their_locator_into_data() {
    let mut locator = BTreeMap::new();
    locator.insert("pointer".to_string(), Value::from("/meshes/0"));
    let r = Diag::new(
        codes::pack::BAD_MODEL,
        subj("asset.a"),
        Site::File {
            file: "pack:models/a.glb".into(),
            locator,
        },
        t("m"),
        t("h"),
    )
    .emit(&Spans);
    let j = r.to_canonical_json();
    assert!(j.contains(r#""data":{"pointer":"/meshes/0"}"#), "{j}");
    assert!(j.contains(r#""file":"pack:models/a.glb""#) && !j.contains("\"line\""));
}

#[test]
fn skipped_has_a_stated_count() {
    let d = via_diag::poison::skipped(
        &subj("scatter.shrubs.density"),
        [Some(&subj("step.checks"))],
    )
    .unwrap();
    let j = d.emit(&Spans).to_canonical_json();
    assert!(j.contains(r#""count":1"#), "{j}");
    assert!(j.contains(r#""data":{"first":["step.checks"]}"#), "{j}");
    assert!(j.contains(r#""subject":"scatter.shrubs""#));
    let e = via_diag::poison::exports_skipped(2)
        .unwrap()
        .emit(&Spans)
        .to_canonical_json();
    assert!(e.contains(r#""data":{"stage":"export"}"#) && e.contains(r#""subject":"world""#));
}

#[test]
fn explain_finds_codes_and_names_the_nearest() {
    use via_diag::explain;
    let e = explain::lookup("formula.unknown_name", &[], 2).unwrap();
    assert!(explain::text(e)
        .starts_with("formula.unknown_name (error, spec/formulas.md, milestone 1)\n"));
    let j = explain::json(e);
    assert!(j.starts_with(r#"{"chapter":"formulas","code":"formula.unknown_name","covered_by":"conformance","data":["name","nearest"]"#), "{j}");
    let err = explain::lookup("formula.unknwn_name", &[], 2).unwrap_err();
    assert_eq!(err.code().to_string(), "cli.unknown_code");
    assert_eq!(err.data()["nearest"], Value::from("formula.unknown_name"));
    assert_eq!(
        *err.emit(&via_diag::NoWorld).location(),
        Location::Argument(2)
    );
    let steps = [explain::StepCode {
        code: "step.checks.too_steep".into(),
        severity: via_diag::Severity::Warning,
        explain: Some("Too steep.\n".into()),
    }];
    let s = explain::lookup("step.checks.too_steep", &steps, 2).unwrap();
    assert!(explain::text(s).contains("Too steep."));
    let listed = explain::list_text();
    assert!(
        listed.contains("formula.unknown_name") && listed.starts_with("asset (spec/assets.md)")
    );
}

/// A world for step arrival: `feature.a` exists, `shrubs#k1` is produced by `scatter.shrubs`,
/// and the extent is 0 to 100 m.
struct World;

impl StepWorld for World {
    fn map_subject(&self, written: &str) -> Option<Mapped> {
        match written {
            "feature.a" | "a" => Some(Mapped {
                subject: subj("feature.a"),
                produced: None,
                place: None,
            }),
            "shrubs#k1" => Some(Mapped {
                subject: subj("scatter.shrubs"),
                produced: Some("shrubs#k1".into()),
                place: None,
            }),
            _ => None,
        }
    }
    fn to_lattice(&self, metres: f64) -> Option<i64> {
        // Test stand-in for via-num's rounding: exact for the values used here.
        metres.is_finite().then_some((metres * 1000.0) as i64)
    }
    fn in_extent(&self, x: i64, y: i64) -> bool {
        (0..=100_000).contains(&x) && (0..=100_000).contains(&y)
    }
    fn selector_site(&self, _: &str, _: &str) -> Site {
        step::for_each_site()
    }
}

fn sent() -> StepDiagIn {
    StepDiagIn {
        severity: Some("warning".into()),
        code: Some("unknown_name".into()),
        subject: Some("a".into()),
        message: Some("slope 0.4 over 0.3".into()),
        hint: Some("lower the slope".into()),
        ..StepDiagIn::default()
    }
}

#[test]
fn step_codes_are_prefixed_and_subjects_mapped() {
    let d = step::arrive("checks", &sent(), &World);
    assert_eq!(d.code().to_string(), "step.checks.unknown_name");
    assert_eq!(d.subject().to_string(), "feature.a");
    let mut p = sent();
    p.subject = Some("shrubs#k1".into());
    p.position = Some(vec![10.5, 20.0]);
    let r = step::arrive("checks", &p, &World).emit(&Spans);
    let out = finish(vec![r], &FailOn::none());
    assert_eq!(out[0].subject(), "scatter.shrubs");
    assert_eq!(out[0].occurrences()[0].name.as_deref(), Some("shrubs#k1"));
}

/// One way to spoil a step diagnostic.
type Edit = Box<dyn Fn(&mut StepDiagIn)>;

#[test]
fn malformed_step_diagnostics_are_replaced() {
    let cases: Vec<Edit> = vec![
        Box::new(|d| d.hint = None),
        Box::new(|d| d.hint = Some("  ".into())),
        Box::new(|d| d.message = Some(String::new())),
        Box::new(|d| d.message = Some(" \t".into())),
        Box::new(|d| d.severity = Some("fatal".into())),
        Box::new(|d| d.subject = Some("feature.nowhere".into())),
        Box::new(|d| d.code = Some("Bad-Code".into())),
        Box::new(|d| d.position = Some(vec![500.0, 1.0])),
        Box::new(|d| d.position = Some(vec![f64::NAN, 1.0])),
        Box::new(|d| d.data = Some(RawValue::Text("x".into()))),
        Box::new(|d| {
            let mut t = BTreeMap::new();
            t.insert("k".to_string(), RawValue::Null);
            d.data = Some(RawValue::Table(t));
        }),
    ];
    for (i, f) in cases.iter().enumerate() {
        let mut d = sent();
        f(&mut d);
        let out = step::arrive("checks", &d, &World);
        assert_eq!(out.code().to_string(), "diag.malformed", "case {i}");
        assert_eq!(out.subject().to_string(), "step.checks");
        assert!(out.data().contains_key("missing"));
    }
}

#[test]
fn one_code_with_two_severities_is_one_malformed() {
    let mut e = sent();
    e.severity = Some("error".into());
    let ds = vec![
        step::arrive("checks", &sent(), &World),
        step::arrive("checks", &e, &World),
        step::arrive("checks", &sent(), &World),
    ];
    let out = step::settle(ds);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].code().to_string(), "diag.malformed");
}

#[test]
fn explain_texts_from_steps() {
    let mut book = step::Explains::new();
    assert!(book.observe("checks", "a", Some("A.")).is_none());
    assert!(book.observe("checks", "a", Some("A.")).is_none());
    let d = book.observe("checks", "a", Some("B.")).unwrap();
    assert_eq!(d.code().to_string(), "protocol.explain_differs");
    assert!(
        book.observe("checks", "a", Some("C.")).is_none(),
        "said once"
    );
    let n = book.observe("checks", "b", None).unwrap();
    assert_eq!(n.code().to_string(), "protocol.no_explain");
    assert_eq!(book.get("step.checks.a"), Some("A."));
}

#[test]
fn explain_is_due_with_the_first_emit_of_a_code_in_each_call() {
    let mut book = step::Explains::new();
    assert!(book.observe("each", "a", Some("A.")).is_none());
    assert!(
        book.observe("each", "a", None).is_none(),
        "later in the call"
    );
    book.begin_call();
    let d = book.observe("each", "a", None).unwrap();
    assert_eq!(d.code().to_string(), "protocol.no_explain");
    assert!(book.observe("each", "a", Some("A.")).is_none());
    book.begin_call();
    assert!(book.observe("each", "a", Some("A.")).is_none());
    assert_eq!(book.get("step.each.a"), Some("A."));
    // A text that comes only in a later call is still recorded.
    let mut late = step::Explains::new();
    assert!(late.observe("each", "b", None).is_some());
    late.begin_call();
    assert!(late.observe("each", "b", Some("B.")).is_none());
    assert_eq!(late.get("step.each.b"), Some("B."));
}

#[test]
fn a_lint_counts_its_malformed_and_protocol_records() {
    let mut d = sent();
    d.hint = None;
    let malformed = step::arrive("checks", &d, &World).emit(&Spans);
    let own = step::arrive("checks", &sent(), &World).emit(&Spans);
    let protocol = Diag::new(
        codes::protocol::NO_EXPLAIN,
        Subject::entity("step", "checks"),
        Site::Header,
        t("m"),
        t("h"),
    )
    .emit(&Spans);
    let other = step::arrive("checksx", &sent(), &World).emit(&Spans);
    let out = finish(vec![malformed, own, protocol, other], &FailOn::none());
    let doc = document(
        &out,
        &[Lint {
            step: "checks".into(),
            ran: true,
        }],
        exit_status(&out),
    );
    assert!(
        doc.contains(r#""lints":[{"error":1,"info":0,"ran":true,"step":"checks","warning":2}]"#),
        "{doc}"
    );
}

#[test]
fn terminal_limits_and_summary() {
    let records: Vec<Record> = (0..12)
        .map(|i| {
            Diag::new(
                codes::place::RELIEF,
                Subject::entity("instance", format!("i{i:02}")),
                Site::Header,
                t("steep\u{1b}[2J"),
                t("move it"),
            )
            .emit(&Spans)
        })
        .collect();
    let out = finish(records, &FailOn::none());
    let text = via_diag::terminal::render(
        &out,
        &[
            Lint {
                step: "checks".into(),
                ran: true,
            },
            Lint {
                step: "other".into(),
                ran: false,
            },
        ],
        via_diag::terminal::Options::default(),
    );
    assert_eq!(text.matches("warning[place.relief]").count(), 10);
    assert!(text.contains("... and 2 more place.relief\n"));
    assert!(text.contains("lint checks: 0 errors, 0 warnings, 0 infos\nlint other: skipped\n"));
    assert!(text.ends_with("0 errors, 12 warnings, 0 infos\n"));
    assert!(!text.contains('\u{1b}'), "control characters are escaped");
    assert!(text.contains("  --> world.toml:77:1\n  hint: move it\n"));
}

#[test]
fn blank_text_is_refused() {
    assert!(Text::new("").is_err());
    assert!(Text::new(" \n\t").is_err());
    assert!(Text::new("x").is_ok());
    // A placeholder is not a hint (1.3).
    assert!(Text::new("-").is_err());
    assert!(Text::new(" ? ").is_err());
    assert!(Text::new("é").is_ok());
}
