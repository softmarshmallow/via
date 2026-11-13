//! The `via` binary, end to end, where the conformance runner cannot reach: it passes
//! `--diagnostics` itself (`conformance.md` 3.5) and sends no signal.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A fresh working folder holding `world.toml`.
fn work(name: &str, world: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("via-cli-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("world.toml"), world).unwrap();
    d
}

fn via(dir: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_via"));
    c.current_dir(dir)
        .env("VIA_LIB", repo().join("lib"))
        .env("VIA_CONFIG_DIR", dir.join(".config"));
    c
}

const SMALL: &str = "name       = \"w\"\nextent     = [0, 0, 8, 8]\nresolution = 1\n";

/// `build.md` 14 and `diagnostics.md` 7: a `--diagnostics FILE` inside the build directory is
/// `cli.usage`, exit 2, and the directory is neither read nor written, nor its run report.
#[test]
fn diagnostics_inside_out_is_usage_and_touches_nothing() {
    for (label, out, file) in [
        ("given", Some("bx"), "bx/d.json"),
        ("dotted", Some("./sub/../bx"), "bx/d.json"),
        ("default", None, "build/w/d.json"),
    ] {
        let d = work(&format!("inside-{label}"), SMALL);
        // An existing build directory, so "untouched" means something.
        let dir = d.join(if out.is_some() { "bx" } else { "build/w" });
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("keep.txt"), b"keep").unwrap();
        let mut c = via(&d);
        c.arg("build").arg("world.toml");
        if let Some(o) = out {
            c.args(["--out", o]);
        }
        c.args(["--diagnostics", file]);
        let o = c.output().unwrap();
        let err = String::from_utf8_lossy(&o.stderr);
        assert_eq!(o.status.code(), Some(2), "{label}: {err}");
        assert!(err.contains("cli.usage"), "{label}: {err}");
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(
            names,
            ["keep.txt"],
            "{label}: the build directory was written"
        );
        let report = PathBuf::from(format!("{}.run.json", dir.display()));
        assert!(!report.exists(), "{label}: a run report was written");
        assert!(!d.join(".via").exists(), "{label}: the store was written");
        let _ = std::fs::remove_dir_all(&d);
    }
}

fn python() -> Option<String> {
    let o = Command::new("sh")
        .args(["-c", "command -v python3"])
        .output()
        .ok()?;
    let p = String::from_utf8(o.stdout).ok()?.trim().to_string();
    (!p.is_empty()).then_some(p)
}

fn alive(pid: &str) -> bool {
    Command::new("kill")
        .args(["-0", pid])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// `protocol.md` 6.2: Ctrl+C reaches the engine, which kills the step's tree, removes its call
/// folder and ends by the signal. The step's pid is gone afterwards.
#[test]
fn ctrl_c_kills_the_step() {
    let Some(py) = python() else {
        eprintln!("skipped: no python3 on PATH");
        return;
    };
    let world = format!(
        "{SMALL}\n[field.height]\nrole = \"surface\"\n\n[step.look]\nwhen    = \"lint\"\ncommand = [\"python\", \"steps/lint.py\"]\n"
    );
    let d = work("sigint", &world);
    std::fs::create_dir_all(d.join("steps")).unwrap();
    let pid_file = d.join("pid.txt");
    std::fs::write(
        d.join("steps/lint.py"),
        format!(
            "import os, time\nopen({:?}, 'w').write(str(os.getpid()))\ntime.sleep(60)\n",
            pid_file.display().to_string()
        ),
    )
    .unwrap();
    assert!(via(&d)
        .args(["config", "program", "python", &py])
        .status()
        .unwrap()
        .success());
    assert!(via(&d)
        .args(["trust", "world.toml", "--yes"])
        .stdout(Stdio::null())
        .status()
        .unwrap()
        .success());
    let mut engine = via(&d)
        .args(["build", "world.toml", "--out", "build/w"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let start = Instant::now();
    let step_pid = loop {
        if let Ok(p) = std::fs::read_to_string(&pid_file) {
            if !p.is_empty() {
                break p;
            }
        }
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "the step never started"
        );
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(alive(&step_pid));
    let sent = Command::new("kill")
        .args(["-INT", &engine.id().to_string()])
        .status()
        .unwrap();
    assert!(sent.success());
    let start = Instant::now();
    let status = loop {
        if let Some(s) = engine.try_wait().unwrap() {
            break s;
        }
        assert!(
            start.elapsed() < Duration::from_secs(20),
            "the engine did not end on SIGINT"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(status.signal(), Some(2), "ended by SIGINT: {status:?}");
    // The kill is SIGKILL to the group; give the kernel a moment to reap.
    let start = Instant::now();
    while alive(&step_pid) && start.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!alive(&step_pid), "the step outlived the engine");
    let calls = d.join(".via/store/calls");
    let left: Vec<_> = std::fs::read_dir(&calls)
        .map(|r| r.flatten().map(|e| e.file_name()).collect())
        .unwrap_or_default();
    assert!(left.is_empty(), "call folders left: {left:?}");
    let _ = std::fs::remove_dir_all(&d);
}
