//! Python under `lib/` imports only `viasdk` and the standard library (R19b), and a library step
//! (not a `test_` file) neither imports `tomllib` nor names `kinds.toml` outside a docstring: it
//! does not read its kinds file (`spec/protocol.md` 7).
//!
//! The reading is done by `py/check_imports.py` with Python's own `ast`, run through uv with an
//! isolated interpreter: Python's parser is the only one that reads every import form exactly,
//! and its `sys.stdlib_module_names` is the standard library's own list. This side finds the
//! files, plants a self-test, and turns the script's JSON lines into findings.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::report::{rel, CheckError, Outcome};

const SCRIPT: &str = "crates/xtask/py/check_imports.py";
/// The interpreter uv runs; its standard-library list is the one held. Overridden by
/// `VIA_CHECK_PYTHON` until the SDK's `pyproject.toml` states the supported range.
const DEFAULT_PYTHON: &str = "3.12";

fn python_files(dir: &Path) -> Result<Vec<PathBuf>, CheckError> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d)? {
            let p = e?.path();
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if p.is_dir() {
                if name != "__pycache__" && !name.starts_with('.') {
                    stack.push(p);
                }
            } else if name.ends_with(".py") {
                out.push(p);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// One finding of the script: (absolute path, line, message).
type PyFinding = (String, usize, String);

fn run_script(root: &Path, lib: &Path, files: &[PathBuf]) -> Result<Vec<PyFinding>, CheckError> {
    let python = std::env::var("VIA_CHECK_PYTHON").unwrap_or_else(|_| DEFAULT_PYTHON.to_string());
    let output = Command::new("uv")
        .args([
            "run",
            "--no-project",
            "--quiet",
            "--python",
            &python,
            "python",
            "-I",
        ])
        .arg(root.join(SCRIPT))
        .arg(lib)
        .args(files)
        .current_dir(root)
        .output()
        .map_err(|e| CheckError(format!("cannot run uv (needed for the import check): {e}")))?;
    if !output.status.success() {
        return Err(CheckError(format!(
            "{SCRIPT} failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let mut out = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let v: serde_json::Value = serde_json::from_str(line)
            .map_err(|e| CheckError(format!("{SCRIPT} printed a line that is not JSON: {e}")))?;
        out.push((
            v["path"].as_str().unwrap_or("").to_string(),
            v["line"].as_u64().unwrap_or(0) as usize,
            v["message"].as_str().unwrap_or("").to_string(),
        ));
    }
    Ok(out)
}

/// Plants a library in a temporary folder whose bad imports must each be found and whose good
/// ones must not.
fn self_test(root: &Path) -> Result<Vec<String>, CheckError> {
    let dir = std::env::temp_dir().join(format!("via-xtask-imports-{}", std::process::id()));
    let lib = dir.join("lib");
    let pkg = lib.join("demo");
    std::fs::create_dir_all(&pkg)?;
    let good = pkg.join("good.py");
    let bad = pkg.join("bad.py");
    std::fs::write(
        &good,
        "import os, json\nfrom viasdk import step\nimport viasdk.fields as f\nfrom . import good\nfrom __future__ import annotations\n",
    )?;
    std::fs::write(
        &bad,
        "import numpy\ndef f():\n    from requests import get\n    import importlib\n    importlib.import_module('x')\nfrom .. import other\n",
    )?;
    // A step that reads its kinds file (protocol.md 7): `tomllib` and the name, not in a
    // docstring; the same lines in a test are not a step's.
    let kinds_text =
        "\"\"\"Not read: kinds.toml in a docstring.\"\"\"\nimport tomllib\nP = 'kinds.toml'\n";
    let kinds = pkg.join("reads_kinds.py");
    let test = pkg.join("test_kinds.py");
    std::fs::write(&kinds, kinds_text)?;
    std::fs::write(&test, kinds_text)?;
    let result = run_script(root, &lib, &[good, bad, kinds, test]);
    let _ = std::fs::remove_dir_all(&dir);
    let found = result?;
    let mut lines: Vec<usize> = found
        .iter()
        .filter(|(p, _, _)| p.ends_with("bad.py"))
        .map(|(_, l, _)| *l)
        .collect();
    lines.sort();
    let mut problems = Vec::new();
    if found
        .iter()
        .any(|(p, _, _)| p.ends_with("good.py") || p.ends_with("test_kinds.py"))
    {
        problems.push(format!("self-test: good imports were reported: {found:?}"));
    }
    let mut kinds_lines: Vec<usize> = found
        .iter()
        .filter(|(p, _, _)| p.ends_with("reads_kinds.py"))
        .map(|(_, l, _)| *l)
        .collect();
    kinds_lines.sort();
    if kinds_lines != [2, 3] {
        problems.push(format!(
            "self-test: planted kinds-file reads at lines 2, 3; found {kinds_lines:?}"
        ));
    }
    if lines != [1, 3, 5, 6] {
        problems.push(format!(
            "self-test: planted bad imports at lines 1, 3, 5, 6; found {lines:?}"
        ));
    }
    Ok(problems)
}

pub fn check(root: &Path) -> Result<Outcome, CheckError> {
    let mut o = Outcome::default();
    let lib = root.join("lib");
    let files = if lib.is_dir() {
        python_files(&lib)?
    } else {
        Vec::new()
    };
    for p in self_test(root)? {
        o.finding("crates/xtask", 0, p);
    }
    o.note("self-test: a planted library's bad imports and kinds-file reads found, its good ones passed");
    if files.is_empty() {
        o.note("no Python under lib/ yet: nothing to check");
        return Ok(o);
    }
    for (path, line, message) in run_script(root, &lib, &files)? {
        o.finding(rel(root, Path::new(&path)), line, message);
    }
    o.note(format!(
        "{} Python files under lib/ read with ast",
        files.len()
    ));
    Ok(o)
}
