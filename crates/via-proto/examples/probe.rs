//! The M0 probe with its measurements (spec/protocol.md, case 0; plan.md M0).
//!
//! ```text
//! cargo run -p via-proto --release --example probe -- [--python PATH | --program NAME]
//!     [--runs 5] [--calls 10000] [--maps 1000]
//! ```
//!
//! The interpreter is `--python PATH`, or the program `--program NAME` (default `python`) from
//! `config.toml` in the user config folder (`VIA_CONFIG_DIR` replaces it), never `PATH`.
//!
//! Each run starts a fresh worker and does, in order: initialize; the 6.5 MB field out and back
//! with one call back; `--calls` empty batch calls; `--maps` blob maps (a `build.field` round trip
//! plus `numpy.load(mmap_mode="r")`), and as many bare maps; then shutdown. A last worker is
//! killed mid-call. Times are wall clock; the per-call ones are taken in the worker with
//! `time.perf_counter_ns`, so they are what a step sees.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde_json::json;
use via_proto::npy;
use via_proto::probe::{self, Probe};
use via_proto::store::digest;
use via_proto::worker::{CancelToken, Deadlines, Outcome};

struct Args {
    python: Option<PathBuf>,
    program: String,
    runs: usize,
    calls: usize,
    maps: usize,
}

fn args() -> Args {
    let mut a = Args {
        python: None,
        program: "python".into(),
        runs: 5,
        calls: 10_000,
        maps: 1_000,
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let v = it
            .next()
            .unwrap_or_else(|| fail(&format!("{k} needs a value")));
        match k.as_str() {
            "--python" => a.python = Some(PathBuf::from(v)),
            "--program" => a.program = v,
            "--runs" => a.runs = v.parse().unwrap_or_else(|_| fail("--runs takes a count")),
            "--calls" => a.calls = v.parse().unwrap_or_else(|_| fail("--calls takes a count")),
            "--maps" => a.maps = v.parse().unwrap_or_else(|_| fail("--maps takes a count")),
            _ => fail(&format!("unknown argument {k}")),
        }
    }
    a
}

fn fail(msg: &str) -> ! {
    eprintln!("probe: {msg}");
    std::process::exit(2)
}

fn f64s(path: &std::path::Path) -> Vec<f64> {
    std::fs::read(path)
        .unwrap_or_else(|e| fail(&format!("{}: {e}", path.display())))
        .chunks_exact(8)
        .map(|c| f64::from_le_bytes(c.try_into().expect("8 bytes")))
        .collect()
}

/// Median and 99th percentile, in microseconds, of nanosecond samples.
fn stats(mut ns: Vec<f64>) -> (f64, f64) {
    ns.sort_by(f64::total_cmp);
    let q = |p: f64| ns[((ns.len() - 1) as f64 * p) as usize] / 1000.0;
    (q(0.5), q(0.99))
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn main() {
    let a = args();
    let python = match a.python {
        Some(p) => p,
        None => {
            let dir = via_proto::program::config_dir()
                .unwrap_or_else(|| fail("no user config folder; pass --python PATH"));
            via_proto::program::resolve(&a.program, &dir).unwrap_or_else(|e| fail(&e.to_string()))
        }
    };
    let tmp = std::env::temp_dir().join(format!("via-probe-{}", std::process::id()));
    let p = Probe::new(
        tmp.join("store"),
        python,
        probe::repo_sdk(),
        probe::fixture_script(),
    )
    .unwrap_or_else(|e| fail(&e.to_string()));
    let (field, values) = p.seed_field().unwrap_or_else(|e| fail(&e.to_string()));
    let neg: Vec<f32> = values.iter().map(|v| -v + 0.0).collect();
    let expect_neg = digest(&npy::f32_file(&[p.grid.ny, p.grid.nx], &neg));
    println!(
        "field: {} x {} f32, {} bytes as a field file",
        p.grid.nx,
        p.grid.ny,
        std::fs::metadata(p.store.path_of(&field.blob))
            .map(|m| m.len())
            .unwrap_or(0)
    );

    let mut rows: Vec<[f64; 8]> = Vec::new();
    for run in 0..a.runs {
        let mut s = p.services();
        s.fields.insert("height".into(), field.clone());
        let none = CancelToken::default();

        let t = Instant::now();
        let mut w = p
            .start(Deadlines::default())
            .unwrap_or_else(|d| fail(&probe::site().render(&d)));
        let start = ms(t.elapsed());

        let t = Instant::now();
        let r = p
            .call(&mut w, json!({"mode": "field"}), &mut s, &none)
            .unwrap_or_else(|d| fail(&format!("{}\n{}", probe::site().render(&d), w.log())));
        let field_ms = ms(t.elapsed());
        assert_eq!(r.files["same.npy"], field.blob, "bytes out and back differ");
        assert_eq!(r.files["neg.npy"], expect_neg, "negated field differs");
        assert!(matches!(r.outcome, Outcome::Done(_)));

        // The worker writes its timings into out, which the engine stores as objects.
        let r = p
            .call(
                &mut w,
                json!({"mode": "calls", "n": a.calls}),
                &mut s,
                &none,
            )
            .unwrap_or_else(|d| fail(&probe::site().render(&d)));
        let calls = f64s(&p.store.path_of(&r.files["calls.f64"]));
        let r = p
            .call(&mut w, json!({"mode": "maps", "n": a.maps}), &mut s, &none)
            .unwrap_or_else(|d| fail(&probe::site().render(&d)));
        let maps = f64s(&p.store.path_of(&r.files["maps.f64"]));
        let bare = f64s(&p.store.path_of(&r.files["bare_maps.f64"]));

        // Engine-side maps of the same blob (checked once, on first map).
        let mut eng = Vec::with_capacity(a.maps);
        let mut acc = 0u64;
        for k in 0..a.maps {
            let t = Instant::now();
            let m = p
                .store
                .map(&field.blob)
                .unwrap_or_else(|e| fail(&e.to_string()));
            acc = acc.wrapping_add(u64::from(m[128 + (k * 4096) % (m.len() - 128)]));
            eng.push(t.elapsed().as_nanos() as f64);
        }
        std::hint::black_box(acc);

        let t = Instant::now();
        let slow = w.shutdown();
        let stop = ms(t.elapsed());
        assert!(slow.is_none(), "slow exit");

        let (c50, c99) = stats(calls);
        let (m50, m99) = stats(maps);
        let (b50, _) = stats(bare);
        let (e50, _) = stats(eng);
        println!(
            "run {run}: start+initialize {start:.1} ms; field out and back {field_ms:.1} ms; \
             empty call median {c50:.1} us (p99 {c99:.1}); build.field+map median {m50:.1} us \
             (p99 {m99:.1}); bare numpy map {b50:.1} us; engine map {e50:.1} us; shutdown {stop:.1} ms"
        );
        rows.push([start, field_ms, c50, c99, m50, m99, b50, e50]);
    }
    let col = |i: usize| median(rows.iter().map(|r| r[i]).collect());
    println!("\nmedian of {} runs:", a.runs);
    println!("  start + initialize          {:8.1} ms", col(0));
    println!(
        "  6.5 MB field out and back   {:8.1} ms  (incl. 1 call back, 2 field files stored)",
        col(1)
    );
    println!(
        "  empty batch call, round trip{:8.1} us  (p99 {:.1} us; n = {})",
        col(2),
        col(3),
        a.calls
    );
    println!(
        "  build.field + numpy map     {:8.1} us  (p99 {:.1} us; n = {})",
        col(4),
        col(5),
        a.maps
    );
    println!("  numpy.load(mmap) alone      {:8.1} us", col(6));
    println!("  engine Store::map           {:8.1} us", col(7));

    // The kill: a worker killed mid-call is reported, located, and its tree and call folder go.
    let mut s = p.services();
    let mut w = p
        .start(Deadlines::default())
        .unwrap_or_else(|d| fail(&probe::site().render(&d)));
    let pids = tmp.join("pids");
    let pids_w = pids.clone();
    // Kills the interpreter the fixture names (on Windows, the venv launcher's child), on every
    // system, so the call ends by the crash and not by the 600 s deadline.
    let killer = std::thread::spawn(move || loop {
        if let Ok(t) = std::fs::read_to_string(&pids_w) {
            let v: Vec<u32> = t
                .split_whitespace()
                .filter_map(|x| x.parse().ok())
                .collect();
            if v.len() == 2 {
                probe::kill_pid(v[0]);
                return v;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    });
    let t = Instant::now();
    let err = p
        .call(
            &mut w,
            json!({"mode": "hang", "pids": pids.to_string_lossy()}),
            &mut s,
            &CancelToken::default(),
        )
        .expect_err("the call must fail");
    let pids = killer.join().unwrap_or_default();
    let site = probe::site();
    println!(
        "\nkilled mid-call after {:.1} ms:\n{}",
        ms(t.elapsed()),
        site.render(&err)
    );
    println!(
        "diagnostics.json record: {}",
        site.record(&err).to_canonical_json()
    );
    std::thread::sleep(Duration::from_millis(100));
    for pid in pids {
        println!("pid {pid} alive after cleanup: {}", probe::alive(pid));
    }
    let left = std::fs::read_dir(tmp.join("store/calls"))
        .map(|d| d.count())
        .unwrap_or(0);
    println!("call folders left: {left}");
    let _ = std::fs::remove_dir_all(&tmp);
}
