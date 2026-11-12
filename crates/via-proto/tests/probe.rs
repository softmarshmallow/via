//! The M0 probe as tests (spec/protocol.md, conformance case 0, and the M2 cases it previews).
//!
//! They need the SDK's uv environment with NumPy, so they are ignored by default:
//!
//! ```text
//! uv venv sdk/python/.venv --python 3.12 && uv pip install --python sdk/python/.venv numpy
//! cargo test -p via-proto -- --include-ignored
//! ```
//!
//! The interpreter is reached through a `config.toml` written for each test, as `via` reaches a
//! named program (6.1), never through `PATH`.
//!
//! On Windows a venv's `Scripts\python.exe` is a launcher that starts the base interpreter as its
//! child, so the process id a fixture writes (`os.getpid()`) is that child, not `Tree::id()`. The
//! kill tests are written to hold either way: they kill and watch the ids the fixture wrote.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::json;
use via_diag::{codes, Code, DiagCode, Location, Value};
use via_proto::diag::Diag;
use via_proto::npy;
use via_proto::probe::{self, Probe, ProbeServices};
use via_proto::process::Tree;
use via_proto::program;
use via_proto::store::digest;
use via_proto::worker::{CancelToken, Deadlines, Outcome, Worker, WorkerSpec};

struct Setup {
    _dir: tempfile::TempDir,
    root: PathBuf,
    probe: Probe,
}

fn setup() -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let venv = probe::venv_python();
    assert!(
        venv.exists(),
        "{} is missing: {}",
        venv.display(),
        probe::VENV_SETUP
    );
    // The interpreter by name, through config.toml (6.1, 6.3).
    let config = root.join("config");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(
        config.join("config.toml"),
        probe::program_config("python", &venv),
    )
    .unwrap();
    let python = program::resolve("python", &config).unwrap();
    let probe = Probe::new(
        root.join("store"),
        python,
        probe::repo_sdk(),
        probe::fixture_script(),
    )
    .unwrap();
    Setup {
        _dir: dir,
        root,
        probe,
    }
}

fn services_with_field(p: &Probe) -> (ProbeServices, Vec<f32>, String) {
    let (field, values) = p.seed_field().unwrap();
    let blob = field.blob.clone();
    let mut s = p.services();
    s.fields.insert("height".into(), field);
    (s, values, blob)
}

fn wait_gone(pid: u32) -> bool {
    let until = Instant::now() + Duration::from_secs(5);
    while Instant::now() < until {
        if !probe::alive(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

fn wait_pids(path: &Path) -> (u32, u32) {
    let until = Instant::now() + Duration::from_secs(20);
    loop {
        if let Ok(t) = std::fs::read_to_string(path) {
            let v: Vec<u32> = t
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if v.len() == 2 {
                return (v[0], v[1]);
            }
        }
        assert!(Instant::now() < until, "the step never wrote its pids");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The code, located at the step's header with bytes (diagnostics.md 1.2), and only the `data`
/// the registry row declares (section 2).
fn assert_located(d: &Diag, code: Code) {
    let site = probe::site();
    assert_eq!(d.code(), &DiagCode::Core(code), "{}", site.render(d));
    assert!(d.undeclared_data().is_empty(), "{:?}", d.undeclared_data());
    let r = site.record(d);
    assert_eq!(r.subject(), "step.probe");
    assert_eq!(
        r.location(),
        &Location::Span {
            file: "probe/world.toml".into(),
            line: 1,
            column: 1,
            bytes: [0, 12],
        }
    );
    assert!(!r.hint().is_empty());
}

fn text(s: &str) -> Value {
    Value::text(s)
}

#[test]
#[ignore = "needs sdk/python/.venv with numpy (uv); run with --include-ignored"]
fn field_out_and_back_with_a_call_back() {
    let s = setup();
    let (mut services, values, height) = services_with_field(&s.probe);
    let mut w = s.probe.start(Deadlines::default()).unwrap();
    assert_eq!(w.sdk()["name"], "viasdk");
    let r = s
        .probe
        .call(
            &mut w,
            json!({"mode": "field"}),
            &mut services,
            &CancelToken::default(),
        )
        .unwrap();

    // Out unchanged: the very bytes, so the very name.
    assert_eq!(r.files["same.npy"], height);
    // Negated, with -0.0 written as +0.0 by the SDK.
    let neg: Vec<f32> = values.iter().map(|v| -v + 0.0).collect();
    assert!(values.contains(&0.0));
    let expect = npy::f32_file(&[1281, 1281], &neg);
    assert_eq!(expect.len(), 6_563_972);
    assert_eq!(r.files["neg.npy"], digest(&expect));
    let back = s.probe.store.map(&r.files["neg.npy"]).unwrap();
    assert_eq!(npy::parse(&back).unwrap().shape, vec![1281, 1281]);

    // The call back was answered with a column, checked inside the step.
    assert_eq!(services.answered["expr.eval"], 1);
    assert_eq!(services.answered["build.field"], 1);
    let Outcome::Done(result) = r.outcome else {
        panic!("not done")
    };
    let doc = result["document"].as_str().unwrap();
    assert!(doc.contains("[field.neg]\nfile = \"neg.npy\""), "{doc}");
    assert!(!r.call_folder.exists(), "the call folder is deleted");
    assert!(w.log().contains("probe step imported"));
    assert_eq!(w.shutdown(), None);
}

#[test]
#[ignore = "needs sdk/python/.venv with numpy (uv); run with --include-ignored"]
fn a_worker_killed_mid_call_is_crashed_and_cleaned_up() {
    let s = setup();
    let mut services = s.probe.services();
    let mut w = s.probe.start(Deadlines::default()).unwrap();
    let pids = s.root.join("pids");
    let pids_w = pids.clone();
    let killer = std::thread::spawn(move || {
        let (worker, child) = wait_pids(&pids_w);
        probe::kill_pid(worker);
        (worker, child)
    });
    let started = Instant::now();
    let err = s
        .probe
        .call(
            &mut w,
            json!({"mode": "hang", "pids": pids.to_string_lossy()}),
            &mut services,
            &CancelToken::default(),
        )
        .unwrap_err();
    let (worker, child) = killer.join().unwrap();
    assert_located(&err, codes::protocol::CRASHED);
    assert!(err.data().is_empty(), "protocol.crashed has no data (6.2)");
    assert!(started.elapsed() < Duration::from_secs(10));
    assert!(wait_gone(worker));
    assert!(
        wait_gone(child),
        "the child left in the group is killed with it"
    );
    assert_eq!(
        std::fs::read_dir(s.root.join("store/calls"))
            .unwrap()
            .count(),
        0,
        "no call folder is left"
    );
}

#[test]
#[ignore = "needs sdk/python/.venv with numpy (uv); run with --include-ignored"]
fn a_deadline_cancels_then_kills_the_tree() {
    let s = setup();
    let mut services = s.probe.services();
    let deadlines = Deadlines {
        call: Duration::from_millis(500),
        cancel: Duration::from_millis(500),
        ..Deadlines::default()
    };
    let mut w = s.probe.start(deadlines).unwrap();
    let pids = s.root.join("pids");
    let err = s
        .probe
        .call(
            &mut w,
            json!({"mode": "hang", "pids": pids.to_string_lossy()}),
            &mut services,
            &CancelToken::default(),
        )
        .unwrap_err();
    assert_located(&err, codes::protocol::DEADLINE);
    assert_eq!(err.data()["phase"], text("cancel"));
    assert_eq!(err.data()["deadline"], Value::double(0.5).unwrap());
    let (worker, child) = wait_pids(&pids);
    assert!(wait_gone(worker));
    assert!(wait_gone(child));
}

#[test]
#[ignore = "needs sdk/python/.venv with numpy (uv); run with --include-ignored"]
fn cancel_then_raise_then_shutdown() {
    let s = setup();
    let mut services = s.probe.services();
    let mut w = s.probe.start(Deadlines::default()).unwrap();
    let token = CancelToken::default();
    let t = token.clone();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        t.cancel();
    });
    let r = s
        .probe
        .call(&mut w, json!({"mode": "spin"}), &mut services, &token)
        .unwrap();
    canceller.join().unwrap();
    assert_eq!(r.outcome, Outcome::Cancelled);
    assert!(services.answered["expr.eval"] > 0);

    // The worker stays up after a cancel and after a raise (3.4).
    let err = s
        .probe
        .call(
            &mut w,
            json!({"mode": "raise"}),
            &mut services,
            &CancelToken::default(),
        )
        .unwrap_err();
    assert_located(&err, codes::protocol::RAISED);
    assert_eq!(err.data()["exception"], text("ValueError"));
    assert!(w.log().contains("ValueError: as asked"));
    let r = s
        .probe
        .call(
            &mut w,
            json!({"mode": "print"}),
            &mut services,
            &CancelToken::default(),
        )
        .unwrap();
    assert!(matches!(r.outcome, Outcome::Done(_)));
    let log = w.log();
    assert!(log.contains("hello from the step"), "{log}");
    assert!(log.contains("raw write to descriptor 1"), "{log}");
    assert_eq!(w.shutdown(), None);
}

/// A worker that does not use the launcher, for the faults the launcher prevents.
fn fake(s: &Setup, mode: &str) -> Result<Worker, Diag> {
    Worker::start(WorkerSpec {
        program: s.probe.python.clone(),
        args: vec![
            probe::fixture_script()
                .with_file_name("fake_worker.py")
                .into_os_string(),
            mode.into(),
        ],
        cwd: s.root.clone(),
        sdk: Some(probe::repo_sdk()),
        objects: s.probe.store.objects().to_path_buf(),
        site: probe::site(),
        when: "generate".into(),
        methods: vec![],
        deadlines: Deadlines {
            initialize: Duration::from_secs(5),
            ..Deadlines::default()
        },
    })
}

#[test]
#[ignore = "needs sdk/python/.venv with numpy (uv); run with --include-ignored"]
fn version_and_framing_faults() {
    let s = setup();
    let err = fake(&s, "version").err().expect("refused");
    assert_located(&err, codes::protocol::VERSION);
    assert_eq!(err.data()["worker"], text("via-step-2"));
    assert_eq!(err.data()["engine"], text("via-step-1"));

    let err = fake(&s, "when").err().expect("refused");
    assert_located(&err, codes::protocol::WHEN_UNSERVED);
    assert_eq!(err.data()["when"], text("generate"));

    let mut w = fake(&s, "hello").unwrap();
    let mut services = s.probe.services();
    let err = s
        .probe
        .call(&mut w, json!({}), &mut services, &CancelToken::default())
        .unwrap_err();
    assert_located(&err, codes::protocol::FRAMING);

    let err = fake(&s, "silent").err().expect("deadline");
    assert_located(&err, codes::protocol::DEADLINE);
    assert_eq!(err.data()["phase"], text("initialize"));
    // numbers.md 3.4: the deadline is written `5`, not `5.0`.
    let json = probe::site().record(&err).to_canonical_json();
    assert!(json.contains(r#""deadline":5,"#), "{json}");
}

#[test]
#[ignore = "needs sdk/python/.venv with numpy (uv); run with --include-ignored"]
fn dropping_a_tree_kills_its_group() {
    let s = setup();
    let pids = s.root.join("pids");
    let mut cmd = std::process::Command::new(&s.probe.python);
    // The pids file as an argument, so no path is quoted into Python source.
    cmd.arg("-c")
        .arg(
            "import os, subprocess, sys, time\n\
             c = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(600)'])\n\
             open(sys.argv[1], 'w').write(f'{os.getpid()} {c.pid}')\n\
             time.sleep(600)\n",
        )
        .arg(&pids);
    let tree = Tree::spawn(cmd).unwrap();
    let (root, child) = wait_pids(&pids);
    // On Windows the venv launcher is the root and the interpreter its child (module notes).
    #[cfg(unix)]
    assert_eq!(root, tree.id());
    drop(tree);
    assert!(wait_gone(root));
    assert!(wait_gone(child));

    // And a worker: dropped while idle, its tree goes with it.
    let w = s.probe.start(Deadlines::default()).unwrap();
    let pid = w.pid();
    drop(w);
    assert!(wait_gone(pid));
}
