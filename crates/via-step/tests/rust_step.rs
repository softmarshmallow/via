//! A Rust step through the engine's door (R19b): the probe's field out and back and its call back,
//! with the same bytes as the Python step gives. Needs no Python, so it runs by default.

use std::ffi::OsString;
use std::path::PathBuf;

use serde_json::json;
use via_proto::npy;
use via_proto::probe::{self, Probe};
use via_proto::store::digest;
use via_proto::worker::{CancelToken, Deadlines, Outcome, Worker, WorkerSpec};

#[test]
fn a_rust_step_speaks_the_same_protocol() {
    let dir = tempfile::tempdir().unwrap();
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_via-step-probe"));
    // The probe harness is reused for its store, field and services; only the program differs.
    let p = Probe::new(
        dir.path().join("store"),
        exe.clone(),
        probe::repo_sdk(),
        exe.clone(),
    )
    .unwrap();
    let (field, values) = p.seed_field().unwrap();
    let mut services = p.services();
    services.fields.insert("height".into(), field);
    let mut w = Worker::start(WorkerSpec {
        program: exe,
        args: Vec::<OsString>::new(),
        cwd: dir.path().to_path_buf(),
        sdk: None,
        objects: p.store.objects().to_path_buf(),
        site: probe::site(),
        when: "generate".into(),
        methods: probe::METHODS.iter().map(|s| s.to_string()).collect(),
        deadlines: Deadlines::default(),
    })
    .unwrap();
    assert_eq!(w.sdk()["name"], "via-step");
    let r = p
        .call(&mut w, json!({}), &mut services, &CancelToken::default())
        .unwrap();
    let neg: Vec<f32> = values.iter().map(|v| -v + 0.0).collect();
    assert_eq!(
        r.files["neg.npy"],
        digest(&npy::f32_file(&[1281, 1281], &neg))
    );
    assert!(matches!(r.outcome, Outcome::Done(_)));
    assert_eq!(services.answered["expr.eval"], 1);
    assert!(w.log().contains("println! from the Rust step"));
    assert_eq!(w.shutdown(), None);
}

fn rust_worker(dir: &std::path::Path, p: &Probe, deadlines: Deadlines) -> Worker {
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_via-step-probe"));
    Worker::start(WorkerSpec {
        program: exe,
        args: Vec::<OsString>::new(),
        cwd: dir.to_path_buf(),
        sdk: None,
        objects: p.store.objects().to_path_buf(),
        site: probe::site(),
        when: "generate".into(),
        methods: probe::METHODS.iter().map(|s| s.to_string()).collect(),
        deadlines,
    })
    .unwrap()
}

/// A Rust step computing with no engine call sees `$/cancelRequest` through its reader thread.
#[test]
fn a_rust_step_sees_a_cancel_without_calling_the_engine() {
    let dir = tempfile::tempdir().unwrap();
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_via-step-probe"));
    let p = Probe::new(
        dir.path().join("store"),
        exe.clone(),
        probe::repo_sdk(),
        exe,
    )
    .unwrap();
    let mut services = p.services();
    let deadlines = Deadlines {
        call: std::time::Duration::from_millis(300),
        ..Deadlines::default()
    };
    let mut w = rust_worker(dir.path(), &p, deadlines);
    let token = CancelToken::default();
    let t = token.clone();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        t.cancel();
    });
    let r = p
        .call(&mut w, json!({"mode": "spin"}), &mut services, &token)
        .unwrap();
    canceller.join().unwrap();
    assert_eq!(r.outcome, Outcome::Cancelled);
    assert_eq!(w.shutdown(), None);
}

/// The engine dies while a Rust step computes: its input ends, and it exits rather than run on
/// as an orphan (macOS has no parent-death signal; protocol.md 3.1, 6.2).
#[test]
fn a_rust_step_ends_when_its_input_ends_mid_call() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    use via_proto::frame::write_frame;

    let dir = tempfile::tempdir().unwrap();
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_via-step-probe"));
    let mut child = Command::new(&exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for msg in [
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
               "params": {"protocol": via_proto::PROTOCOL}}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "step.call",
               "params": {"when": "generate", "params": {"mode": "spin"},
                          "out": dir.path().to_string_lossy()}}),
    ] {
        write_frame(&mut stdin, &serde_json::to_vec(&msg).unwrap()).unwrap();
    }
    stdin.flush().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200));
    drop(stdin); // The engine "dies": the step's protocol input ends.
    let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "{status:?}");
            break;
        }
        if std::time::Instant::now() > until {
            let _ = child.kill();
            panic!("the Rust step ran on after its input ended");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
