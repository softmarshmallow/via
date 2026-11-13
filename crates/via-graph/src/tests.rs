//! Whole-graph tests with a toy plan: the M1 conformance cases of `spec/graph.md` that the graph
//! alone decides (null, threads, empty store, shuffle, early cutoff, why, toggle, cycle rest,
//! warning and error replay, step slots, failed step, rerun, corrupt, epoch, run report), and
//! section 7's invariant after every sequence.
//!
//! The toy world is a list of statements `id = value | reads`, where `value` is an integer, a
//! product `2*3`, `err` (an error) or `warn:<n>` (a warning). Each statement is an `attr` node
//! (digest only) whose value is its own plus the values it reads; three `field` chunks (kept) each
//! sum the attributes whose value falls in it, found by expansion; a plain lint step and a
//! gated build file read the chunks; the diagnostics file and the manifest are defined last.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use via_diag::{codes, Diag, KeyPath, Severity, Site, Subject};

use crate::*;

const CHUNKS: u64 = 3;

#[derive(Clone)]
struct World {
    /// (id, value, reads) in file order.
    lines: Vec<(String, String, Vec<String>)>,
    comment: String,
}

impl World {
    fn new(lines: &[(&str, &str, &[&str])]) -> World {
        World {
            lines: lines
                .iter()
                .map(|(i, v, r)| {
                    (
                        i.to_string(),
                        v.to_string(),
                        r.iter().map(|s| s.to_string()).collect(),
                    )
                })
                .collect(),
            comment: String::new(),
        }
    }

    fn text(&self) -> String {
        let mut s = self.comment.clone();
        for (i, v, r) in &self.lines {
            s.push_str(&format!("{i} = {v} | {}\n", r.join(",")));
        }
        s
    }

    fn set(&mut self, id: &str, value: &str) {
        for l in &mut self.lines {
            if l.0 == id {
                l.1 = value.to_string();
            }
        }
    }

    fn statement(&self, id: &str) -> Option<&(String, String, Vec<String>)> {
        self.lines.iter().find(|l| l.0 == id)
    }
}

struct Toy {
    world: World,
    fail_step: bool,
    unstable: AtomicU64,
    unstable_on: bool,
}

impl Toy {
    fn new(world: World) -> Toy {
        Toy {
            world,
            fail_step: false,
            unstable: AtomicU64::new(0),
            unstable_on: false,
        }
    }

    fn statement_digest(&self, id: &str) -> Digest {
        let (i, v, r) = self.world.statement(id).cloned().unwrap_or_default();
        Digest::of(format!("{i}={v}|{}", r.join(",")).as_bytes())
    }

    fn ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.world.lines.iter().map(|l| l.0.clone()).collect();
        ids.sort();
        ids
    }
}

fn id(s: &str) -> NodeId {
    NodeId::parse(s).unwrap()
}

fn chunk(i: u64) -> NodeId {
    NodeId::of("field", "sum")
        .view("build")
        .window(i, 0, i + 1, 1)
}

fn num(v: &Val) -> i64 {
    String::from_utf8_lossy(&v.bytes).parse().unwrap_or(0)
}

fn own(value: &str) -> Result<(i64, Option<bool>), ()> {
    if value == "err" {
        return Err(());
    }
    if let Some(n) = value.strip_prefix("warn:") {
        return Ok((n.parse().unwrap_or(0), Some(true)));
    }
    let n = value
        .split('*')
        .map(|p| p.trim().parse::<i64>().unwrap_or(0))
        .product();
    Ok((n, None))
}

impl Plan for Toy {
    fn define(&self, nid: &NodeId, known: &Known<'_>) -> Defined {
        let s = nid.as_str();
        let mut def = Def::default();
        match nid.family() {
            "source" => {
                def.params = vec![("path".into(), "world.toml".into())];
                def.direct = vec![("bytes".into(), Digest::of(self.world.text().as_bytes()))];
            }
            "merge" => def.edges = vec![("source".into(), id("source:world.toml"))],
            "select" => def.params = vec![("ids".into(), self.ids().join(","))],
            "attr" => {
                let name = &s["attr:".len()..];
                def.subject = Some(Subject::entity("feature", name));
                def.statements = vec![(format!("feature.{name}"), self.statement_digest(name))];
                if let Some((_, _, reads)) = self.world.statement(name) {
                    def.edges = reads
                        .iter()
                        .map(|r| (format!("reads {r}"), id(&format!("attr:{r}"))))
                        .collect();
                }
            }
            "field" => {
                let sel = id("select:all/v:build");
                let Some(list) = known.value(&sel) else {
                    return Defined::Need(vec![sel]);
                };
                let attrs: Vec<NodeId> = String::from_utf8_lossy(&list.bytes)
                    .split(',')
                    .filter(|x| !x.is_empty())
                    .map(|x| id(&format!("attr:{x}")))
                    .collect();
                if attrs.iter().any(|a| known.value(a).is_none()) {
                    let mut need = vec![sel];
                    need.extend(attrs);
                    return Defined::Need(need);
                }
                let i: u64 = s
                    .split("/w:")
                    .nth(1)
                    .unwrap()
                    .split('.')
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap();
                def.subject = Some(Subject::entity("field", "sum"));
                def.params = vec![("window".into(), format!("{i}"))];
                for a in attrs {
                    let v = known.value(&a).map(num).unwrap_or(0);
                    if v.rem_euclid(CHUNKS as i64) as u64 == i {
                        def.edges.push((format!("effect {}", &a.as_str()[5..]), a));
                    }
                }
            }
            "step" => {
                def.subject = Some(Subject::entity("step", "lint"));
                def.statements = vec![("step.lint".into(), Digest::of(b"lint"))];
                def.edges = (0..CHUNKS)
                    .map(|i| (format!("data {i}"), chunk(i)))
                    .collect();
                def.step = Some(StepDef {
                    step: "lint".into(),
                    command: vec!["toy".into(), "lint".into()],
                });
            }
            "write" => match s {
                "write:out.txt" => {
                    def.edges = (0..CHUNKS)
                        .map(|i| (format!("chunk {i}"), chunk(i)))
                        .collect();
                    def.gate.skip_on_error = true;
                }
                "write:diagnostics.json" => {
                    let Some(world) = known.world_diags() else {
                        return Defined::Last;
                    };
                    def.edges = vec![("merge".into(), id("merge"))];
                    def.direct =
                        vec![("world".into(), Digest::of(format!("{world:?}").as_bytes()))];
                }
                "write:manifest.json" => {
                    if known.world_diags().is_none() {
                        return Defined::Last;
                    }
                    def.gate.skip_on_error = true;
                    def.edges = vec![
                        ("file out.txt".into(), id("write:out.txt")),
                        ("file diagnostics.json".into(), id("write:diagnostics.json")),
                        ("source world.toml".into(), id("source:world.toml")),
                    ];
                }
                _ => return Defined::Def(def),
            },
            _ => {}
        }
        Defined::Def(def)
    }

    fn compute(&self, nid: &NodeId, def: &Def, inputs: &Inputs<'_>) -> Output {
        let s = nid.as_str();
        match nid.family() {
            "source" => Output::of(Val::plain(def.direct[0].1.bytes().to_vec())),
            "merge" => Output::of(Val::plain(self.world.text().into_bytes())),
            "select" => Output::of(Val::plain(self.ids().join(",").into_bytes())),
            "attr" => {
                let name = &s["attr:".len()..];
                let (_, value, _) = self.world.statement(name).cloned().unwrap_or_default();
                let reads: i64 = inputs.all().iter().map(|(_, _, v)| num(v)).sum();
                let subject = Subject::entity("feature", name);
                match own(&value) {
                    Err(()) => Output::of(Val {
                        bytes: Arc::new(Vec::new()),
                        diags: Arc::new(vec![Diag::new(
                            codes::graph::INPUT_MISSING,
                            subject,
                            Site::Key(KeyPath::key("value")),
                            via_diag::text!("the value is an error"),
                            via_diag::text!("write a number"),
                        )]),
                    }),
                    Ok((n, warn)) => {
                        let mut diags = Vec::new();
                        if warn.is_some() {
                            diags.push(Diag::new(
                                codes::identity::CLEAR_EMPTY,
                                subject,
                                Site::Header,
                                via_diag::text!("a warning"),
                                via_diag::text!("drop the warning"),
                            ));
                        }
                        Output::of(Val {
                            bytes: Arc::new((n + reads).to_string().into_bytes()),
                            diags: Arc::new(diags),
                        })
                    }
                }
            }
            "field" => {
                let sum: i64 = inputs.all().iter().map(|(_, _, v)| num(v)).sum();
                Output::of(Val::plain(sum.to_string().into_bytes()))
            }
            "step" => {
                let data: Vec<String> = inputs
                    .all()
                    .iter()
                    .map(|(_, _, v)| String::from_utf8_lossy(&v.bytes).to_string())
                    .collect();
                let request = Digest::of(data.join(";").as_bytes());
                let mut text = format!("ok {}", data.join(";"));
                if self.unstable_on {
                    text.push_str(&format!(
                        " t{}",
                        self.unstable.fetch_add(1, Ordering::SeqCst)
                    ));
                }
                let run = StepRun {
                    reads: vec![],
                    data: Some(Read {
                        method: "data".into(),
                        request,
                        answer: Digest::of(text.as_bytes()),
                    }),
                    exit: Some(if self.fail_step { 1 } else { 0 }),
                    files: vec![("report.txt".into(), Digest::of(text.as_bytes()))],
                    log: None,
                };
                if self.fail_step {
                    return Output {
                        value: Val {
                            bytes: Arc::new(Vec::new()),
                            diags: Arc::new(vec![Diag::new(
                                codes::protocol::FAILED,
                                Subject::entity("step", "lint"),
                                Site::Header,
                                via_diag::text!("the step exited with status 1"),
                                via_diag::text!("read the step's log"),
                            )]),
                        },
                        complete: false,
                        step: Some(run),
                    };
                }
                Output {
                    value: Val::plain(text.into_bytes()),
                    complete: true,
                    step: Some(run),
                }
            }
            _ => match s {
                "write:out.txt" => {
                    let parts: Vec<String> = inputs
                        .all()
                        .iter()
                        .map(|(_, _, v)| String::from_utf8_lossy(&v.bytes).to_string())
                        .collect();
                    Output::of(Val::plain(parts.join("\n").into_bytes()))
                }
                "write:diagnostics.json" => {
                    let lines: Vec<String> = inputs
                        .world_diags()
                        .unwrap_or_default()
                        .iter()
                        .map(|d| format!("{} {}", d.code(), d.subject()))
                        .collect();
                    Output::of(Val::plain(lines.join("\n").into_bytes()))
                }
                _ => {
                    let mut b = Vec::new();
                    for (l, _, v) in inputs.all() {
                        b.extend_from_slice(l.as_bytes());
                        b.extend_from_slice(Digest::of(&v.bytes).hex().as_bytes());
                    }
                    Output::of(Val::plain(b))
                }
            },
        }
    }
}

fn roots() -> Vec<NodeId> {
    vec![
        id("step:lint"),
        id("write:diagnostics.json"),
        id("write:manifest.json"),
        id("write:out.txt"),
    ]
}

fn scratch(name: &str) -> std::path::PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let p = std::env::temp_dir().join(format!(
        "via-graph-it-{name}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&p);
    p
}

/// A store and the record of the last build in it, as `.previous` and the run report would.
struct Bench {
    root: std::path::PathBuf,
    previous: Option<BuildRecord>,
    versions: Versions,
    threads: usize,
}

impl Bench {
    fn new(name: &str) -> Bench {
        Bench {
            root: scratch(name),
            previous: None,
            versions: Versions::test(),
            threads: 1,
        }
    }

    fn build_with(&mut self, toy: &Toy, rerun: &[&str], cycles: Vec<Cycle>) -> Run {
        let store = Store::open(&self.root);
        let mut opts = Options::new(self.versions.clone());
        opts.threads = self.threads;
        opts.previous = self.previous.clone();
        opts.rerun = rerun.iter().map(|s| s.to_string()).collect();
        opts.cycles = cycles;
        let run = run(toy, &store, &opts, &roots());
        let files = files(&run, &store, toy);
        self.previous = Some(run.record(&self.versions, Vec::new(), files));
        run
    }

    fn build(&mut self, toy: &Toy) -> Run {
        self.build_with(toy, &[], Vec::new())
    }

    fn store(&self) -> Store {
        Store::open(&self.root)
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// The build files: every `write` node's bytes.
fn files(run: &Run, store: &Store, toy: &Toy) -> BTreeMap<String, Digest> {
    run.nodes
        .iter()
        .filter(|(i, n)| i.family() == "write" && n.status != Status::Skipped)
        .filter_map(|(i, _)| {
            let v = run.value(i, store, toy)?;
            Some((
                i.as_str()["write:".len()..].to_string(),
                Digest::of(&v.bytes),
            ))
        })
        .collect()
}

fn bytes_of(run: &Run, store: &Store, toy: &Toy) -> BTreeMap<String, Vec<u8>> {
    run.nodes
        .iter()
        .filter(|(i, n)| i.family() == "write" && n.status != Status::Skipped)
        .filter_map(|(i, _)| Some((i.to_string(), run.value(i, store, toy)?.bytes.to_vec())))
        .collect()
}

fn ran(run: &Run) -> Vec<String> {
    run.ran().iter().map(|i| i.to_string()).collect()
}

fn world() -> World {
    World::new(&[
        ("alpha", "4", &[]),
        ("beta", "2", &["alpha"]),
        ("gamma", "1", &[]),
        ("delta", "5", &["gamma"]),
    ])
}

/// Section 7: the same text built clean, into a fresh store, gives the same build files and
/// world diagnostics.
fn assert_clean_equals(run: &Run, store: &Store, toy: &Toy) {
    let mut fresh = Bench::new("fresh");
    let clean = fresh.build(toy);
    assert_eq!(
        bytes_of(run, store, toy),
        bytes_of(&clean, &fresh.store(), toy),
        "clean build differs from incremental"
    );
    assert_eq!(run.world_diags, clean.world_diags);
    for (i, n) in &clean.nodes {
        assert_eq!(
            n.status == Status::Skipped,
            run.nodes[i].status == Status::Skipped,
            "{i}"
        );
    }
}

#[test]
fn clean_then_unchanged() {
    let toy = Toy::new(world());
    let mut b = Bench::new("null");
    let first = b.build(&toy);
    assert!(
        first.nodes.values().all(|n| n.status == Status::Ran),
        "{first:?}"
    );
    assert!(first
        .nodes
        .values()
        .all(|n| n.cause == Some(Cause::NoRecord)));
    assert_eq!(first.nodes.len(), 14);
    let second = b.build(&toy);
    assert!(ran(&second).is_empty(), "{:?}", ran(&second));
    // Expansion needed the digest-only attributes; nothing else was computed.
    for (i, n) in &second.nodes {
        match n.status {
            Status::Recomputed => match &n.needed_by {
                Some(NeededBy::Expansion) => {}
                Some(NeededBy::Node(r)) => {
                    assert_eq!(second.nodes[r].status, Status::Recomputed, "{i} by {r}")
                }
                None => panic!("{i} recomputed for no one"),
            },
            s => assert_eq!(s, Status::Reused, "{i}"),
        }
    }
    let store = b.store();
    assert_eq!(
        bytes_of(&first, &store, &toy),
        bytes_of(&second, &store, &toy)
    );
    assert_clean_equals(&second, &store, &toy);
}

#[test]
fn null_build_touches_nothing() {
    let toy = Toy::new(world());
    let mut b = Bench::new("nullbuild");
    let run = b.build(&toy);
    let store = b.store();
    let bk = build_key(
        &b.versions,
        &BuildKeyParts {
            files: vec![("world.toml".into(), Digest::of(toy.world.text().as_bytes()))],
            ..Default::default()
        },
    );
    let rec = run.record(&b.versions, b"[]".to_vec(), files(&run, &store, &toy));
    rec.put(&store, &bk);
    let null = null_build(&store, &bk, Some(&bk), false).expect("applies");
    let head = ReportHead {
        epoch: 1,
        version: "0.1.0".into(),
        threads: 1,
        key: bk,
        id: Some("x".into()),
        null: true,
        previous: Some(bk),
        ms: 0,
        run_records: vec![],
    };
    let text = null_report(&head, &null);
    assert!(
        text.contains("\"ran\":0,\"recomputed\":0,\"reused\":14,\"skipped\":0"),
        "{text}"
    );
}

#[test]
fn threads_change_nothing() {
    let toy = Toy::new(world());
    let mut one = Bench::new("t1");
    let mut eight = Bench::new("t8");
    eight.threads = 8;
    let a = one.build(&toy);
    let b = eight.build(&toy);
    assert_eq!(ran(&a), ran(&b));
    for (i, n) in &a.nodes {
        assert_eq!(n.key, b.nodes[i].key, "{i}");
        assert_eq!(n.value, b.nodes[i].value, "{i}");
    }
    assert_eq!(
        bytes_of(&a, &one.store(), &toy),
        bytes_of(&b, &eight.store(), &toy)
    );
    let mut toy2 = Toy::new(world());
    toy2.world.set("gamma", "7");
    let a2 = one.build(&toy2);
    let b2 = eight.build(&toy2);
    assert_eq!(ran(&a2), ran(&b2));
    let st: Vec<_> = a2
        .nodes
        .values()
        .map(|n| (n.status, n.cause.clone()))
        .collect();
    let st8: Vec<_> = b2
        .nodes
        .values()
        .map(|n| (n.status, n.cause.clone()))
        .collect();
    assert_eq!(st, st8);
}

#[test]
fn empty_store_runs_everything_with_same_bytes() {
    let toy = Toy::new(world());
    let mut b = Bench::new("empty");
    let first = b.build(&toy);
    let before = bytes_of(&first, &b.store(), &toy);
    let _ = std::fs::remove_dir_all(&b.root);
    let again = b.build(&toy);
    assert!(again.nodes.values().all(|n| n.status == Status::Ran));
    assert_eq!(bytes_of(&again, &b.store(), &toy), before);
}

#[test]
fn shuffle_runs_only_the_text_nodes() {
    let toy = Toy::new(world());
    let mut b = Bench::new("shuffle");
    b.build(&toy);
    let mut w = world();
    w.lines.reverse();
    w.comment = "# moved\n".into();
    let toy2 = Toy::new(w);
    let run = b.build(&toy2);
    assert_eq!(
        ran(&run),
        vec![
            "merge",
            "source:world.toml",
            "write:diagnostics.json",
            "write:manifest.json"
        ]
    );
    assert_eq!(
        run.nodes[&id("source:world.toml")].cause,
        Some(Cause::Edge {
            label: "bytes".into(),
            node: id("source:world.toml")
        })
    );
    assert_clean_equals(&run, &b.store(), &toy2);
}

#[test]
fn early_cutoff_stops_at_an_equal_value() {
    let mut w = world();
    w.set("gamma", "2*3");
    let toy = Toy::new(w.clone());
    let mut b = Bench::new("cutoff");
    b.build(&toy);
    w.set("gamma", "6");
    let toy2 = Toy::new(w);
    let run = b.build(&toy2);
    let r = ran(&run);
    assert!(r.contains(&"attr:gamma".to_string()));
    assert!(!r.contains(&"attr:delta".to_string()), "{r:?}");
    assert_eq!(run.nodes[&id("write:out.txt")].status, Status::Reused);
    assert_eq!(run.nodes[&id("attr:gamma")].cause, Some(Cause::Statements));
    assert_clean_equals(&run, &b.store(), &toy2);
}

#[test]
fn a_change_runs_what_read_it_and_says_why() {
    let toy = Toy::new(world());
    let mut b = Bench::new("why");
    let base = b.build(&toy);
    let base_rec = b.previous.clone().unwrap();
    let mut w = world();
    w.set("alpha", "5");
    let toy2 = Toy::new(w);
    let run = b.build(&toy2);
    let r = ran(&run);
    assert!(r.contains(&"attr:alpha".to_string()) && r.contains(&"attr:beta".to_string()));
    assert!(!r.contains(&"attr:gamma".to_string()));
    for (i, n) in &run.nodes {
        assert_eq!(n.status == Status::Ran, n.cause.is_some(), "{i}");
    }
    assert_eq!(run.nodes[&id("attr:alpha")].cause, Some(Cause::Statements));
    assert_eq!(
        run.nodes[&id("attr:beta")].cause,
        Some(Cause::Edge {
            label: "reads alpha".into(),
            node: id("attr:alpha")
        })
    );
    // 6.4: the change to out.txt is caused by alpha's statement, through beta and a chunk.
    let cs = causes(&run, &base_rec, &id("write:out.txt"));
    assert_eq!(cs.len(), 1);
    assert_eq!(cs[0].statement, "feature.alpha");
    assert_eq!(cs[0].path.first(), Some(&id("attr:alpha")));
    assert_eq!(cs[0].path.last(), Some(&id("write:out.txt")));
    assert!(base.nodes.len() == run.nodes.len());
    assert_clean_equals(&run, &b.store(), &toy2);
}

#[test]
fn toggle_and_step_slots_reuse_every_state() {
    let toy = Toy::new(world());
    let mut w = world();
    w.set("delta", "9");
    let changed = Toy::new(w);
    let mut b = Bench::new("toggle");
    b.build(&toy);
    let with = b.build(&changed);
    assert!(with.nodes[&id("step:lint")].status == Status::Ran);
    for n in 0..4 {
        let t = if n % 2 == 0 { &toy } else { &changed };
        let run = b.build(t);
        assert!(ran(&run).is_empty(), "round {n}: {:?}", ran(&run));
        assert_eq!(run.nodes[&id("step:lint")].status, Status::Reused);
    }
}

#[test]
fn warnings_and_errors_replay() {
    let mut w = world();
    w.set("gamma", "warn:1");
    let toy = Toy::new(w);
    let mut b = Bench::new("warn");
    let first = b.build(&toy);
    assert!(first
        .world_diags
        .iter()
        .any(|d| d.severity() == Severity::Warning));
    let second = b.build(&toy);
    assert!(ran(&second).is_empty());
    assert_eq!(first.world_diags, second.world_diags);

    let mut w = world();
    w.set("alpha", "err");
    let bad = Toy::new(w);
    let mut b = Bench::new("error");
    let first = b.build(&bad);
    assert!(first.has_error);
    assert_eq!(first.nodes[&id("attr:alpha")].status, Status::Ran);
    assert!(first.nodes[&id("attr:alpha")].failed);
    assert_eq!(first.nodes[&id("attr:beta")].status, Status::Skipped);
    assert_eq!(first.nodes[&id("write:out.txt")].status, Status::Skipped);
    assert_eq!(
        first.nodes[&id("write:manifest.json")].status,
        Status::Skipped
    );
    assert_eq!(
        first.nodes[&id("write:diagnostics.json")].status,
        Status::Ran
    );
    assert_eq!(first.nodes[&id("step:lint")].status, Status::Skipped);
    let codes: Vec<String> = first
        .world_diags
        .iter()
        .map(|d| d.code().to_string())
        .collect();
    assert!(
        codes.contains(&"graph.input_missing".to_string()),
        "{codes:?}"
    );
    assert!(codes.contains(&"diag.skipped".to_string()), "{codes:?}");
    let second = b.build(&bad);
    assert!(ran(&second).is_empty(), "{:?}", ran(&second));
    assert_eq!(first.world_diags, second.world_diags);
    assert_clean_equals(&second, &b.store(), &bad);
}

#[test]
fn a_failed_step_runs_again_and_skips_its_readers() {
    let mut toy = Toy::new(world());
    toy.fail_step = true;
    let mut b = Bench::new("failed");
    let first = b.build(&toy);
    let n = &first.nodes[&id("step:lint")];
    assert!(n.failed && n.value.is_none() && n.status == Status::Ran);
    assert!(!first.complete);
    assert_eq!(n.step.as_ref().unwrap().exit, Some(1));
    let second = b.build(&toy);
    assert_eq!(second.nodes[&id("step:lint")].status, Status::Ran);
    assert_eq!(second.nodes[&id("step:lint")].cause, Some(Cause::NotStored));
}

#[test]
fn rerun_finds_an_unstable_step() {
    let mut toy = Toy::new(world());
    toy.unstable_on = true;
    let mut b = Bench::new("rerun");
    b.build(&toy);
    let plain = b.build(&toy);
    assert_eq!(plain.nodes[&id("step:lint")].status, Status::Reused);
    assert!(plain.run_diags.is_empty());
    let forced = b.build_with(&toy, &["lint"], Vec::new());
    assert_eq!(forced.nodes[&id("step:lint")].status, Status::Ran);
    assert_eq!(forced.nodes[&id("step:lint")].cause, Some(Cause::Rerun));
    assert_eq!(forced.run_diags.len(), 1);
    assert_eq!(
        forced.run_diags[0].code().to_string(),
        "graph.step_unstable"
    );
    assert!(!forced
        .world_diags
        .iter()
        .any(|d| d.code().to_string() == "graph.step_unstable"));
}

#[test]
fn a_corrupt_object_is_dropped_and_rebuilt() {
    let toy = Toy::new(world());
    let mut b = Bench::new("corrupt");
    let first = b.build(&toy);
    // Damage a chunk the change does not touch: alpha 4 -> 7 moves alpha and beta.
    let mut w = world();
    w.set("alpha", "7");
    let toy2 = Toy::new(w);
    let untouched = (0..CHUNKS)
        .map(chunk)
        .find(|c| {
            let mut probe = Bench::new("probe");
            probe.build(&toy);
            let r = probe.build(&toy2);
            r.nodes[c].status == Status::Reused
        })
        .expect("a chunk the change does not touch");
    let vd = first.nodes[&untouched].value.unwrap();
    let store = b.store();
    let path = store.object_path(&vd);
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[0] ^= 1;
    std::fs::write(&path, bytes).unwrap();
    let run = b.build(&toy2);
    assert_eq!(run.nodes[&untouched].status, Status::Ran);
    assert_eq!(run.nodes[&untouched].cause, Some(Cause::NotStored));
    assert!(run
        .run_diags
        .iter()
        .any(|d| d.code().to_string() == "graph.store_corrupt"));
    assert_clean_equals(&run, &b.store(), &toy2);
}

#[test]
fn a_new_epoch_runs_everything() {
    let toy = Toy::new(world());
    let mut b = Bench::new("epoch");
    let first = b.build(&toy);
    b.versions.epoch += 1;
    let run = b.build(&toy);
    assert!(run.nodes.values().all(|n| n.status == Status::Ran));
    assert!(run.nodes.values().all(|n| n.cause == Some(Cause::Versions)));
    let store = b.store();
    assert_eq!(bytes_of(&first, &store, &toy), bytes_of(&run, &store, &toy));
}

fn cycle_of(members: &[&str]) -> Vec<Cycle> {
    let mut g = RefGraph::new();
    for (n, m) in members.iter().enumerate() {
        g.member(
            format!("feature.{m}"),
            Member {
                subject: Subject::entity("feature", *m),
                order: SourceOrder {
                    rank: Some(0),
                    file: "world.toml".into(),
                    line: n as u32 + 1,
                },
            },
        );
        g.vertex(format!("attr:{m}"), format!("feature.{m}"));
    }
    for w in 0..members.len() {
        let a = members[w];
        let b = members[(w + 1) % members.len()];
        g.edge(RefEdge {
            from: format!("attr:{a}"),
            to: format!("attr:{b}"),
            label: format!("reads {b}"),
            key: KeyPath::key("reads"),
        });
    }
    g.cycles()
}

#[test]
fn a_cycle_fails_its_group_and_the_rest_is_kept() {
    let looped = World::new(&[
        ("alpha", "1", &["beta"]),
        ("beta", "1", &["alpha"]),
        ("gamma", "3", &[]),
        ("delta", "5", &["gamma"]),
    ]);
    let toy = Toy::new(looped);
    let mut b = Bench::new("cycle");
    let first = b.build_with(&toy, &[], cycle_of(&["alpha", "beta"]));
    assert!(first.has_error);
    assert_eq!(first.nodes[&id("attr:alpha")].status, Status::Skipped);
    assert_eq!(first.nodes[&id("attr:alpha")].key, None);
    assert_eq!(first.nodes[&id("attr:gamma")].status, Status::Ran);
    assert_eq!(first.nodes[&id("write:out.txt")].status, Status::Skipped);
    let cycles: Vec<_> = first
        .world_diags
        .iter()
        .filter(|d| d.code().to_string() == "graph.cycle")
        .collect();
    assert_eq!(cycles.len(), 1);
    let fixed = Toy::new(World::new(&[
        ("alpha", "1", &[]),
        ("beta", "1", &["alpha"]),
        ("gamma", "3", &[]),
        ("delta", "5", &["gamma"]),
    ]));
    let second = b.build(&fixed);
    assert_eq!(second.nodes[&id("attr:gamma")].status, Status::Recomputed);
    assert!(!ran(&second).contains(&"attr:delta".to_string()));
    assert_clean_equals(&second, &b.store(), &fixed);
}

#[test]
fn the_run_report_has_every_member() {
    let toy = Toy::new(world());
    let mut b = Bench::new("report");
    let run = b.build(&toy);
    let head = ReportHead {
        epoch: 1,
        version: "0.1.0".into(),
        threads: 1,
        key: Digest::of(b"k"),
        id: None,
        null: false,
        previous: None,
        ms: 3,
        run_records: vec![],
    };
    let text = report(&head, &run);
    assert!(text.ends_with("}\n") && !text[..text.len() - 1].contains('\n'));
    for m in [
        "\"build\":{\"complete\":true,\"id\":null,",
        "\"format\":\"via-run-0\"",
        "\"engine\":{\"epoch\":1,\"version\":\"0.1.0\"}",
        "\"system\":{\"arch\":",
        "\"totals\":{\"ms\":3,\"nodes\":14,\"ran\":14,\"recomputed\":0,\"reused\":0,\"skipped\":0}",
        "\"cause\":{\"reason\":\"no record\"}",
        "\"command\":[\"toy\",\"lint\"]",
        "\"reads\":[{\"answer\":",
        "\"trace\":null",
        "\"returned\":{\"files\":[{\"digest\":",
    ] {
        assert!(text.contains(m), "{m} missing from {text}");
    }
    let ids: Vec<usize> = run
        .nodes
        .keys()
        .map(|i| text.find(&format!("\"id\":\"{i}\"")).unwrap())
        .collect();
    assert!(ids.windows(2).all(|w| w[0] < w[1]), "nodes in id order");
    let dir = b.root.join("out");
    write_report(&dir, &text).unwrap();
    assert_eq!(std::fs::read_to_string(report_path(&dir)).unwrap(), text);
}
