//! Writes the files generated from the diagnostic registry (`spec/diagnostics.md` section 2):
//! `crates/via-diag/src/generated.rs` (the code constants and `via explain`'s table) and
//! `docs/reference/diagnostics.md` (R33). `cargo test -p via-diag` fails when either is stale.
//!
//! With `--sync`, it first rewrites `spec/diagnostics.toml` from the chapters' Diagnostics tables,
//! keeping every hand-written `explain`; run it after a chapter's table changes.
//!
//! With `--check`, it writes nothing: it exits non-zero naming each generated file that differs
//! from what it would write, so `cargo run -p xtask -- check` can prove the committed copies fresh.
//!
//! Usage: `cargo run -p via-diag --example gen_reference [-- --sync | --check]`

#[path = "../tools/gen.rs"]
mod gen;

use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let root = gen::root();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let sync = args.iter().any(|a| a == "--sync");
    let check = args.iter().any(|a| a == "--check");
    if sync && check {
        return Err("--sync and --check cannot be combined".to_string());
    }
    let reg_path = root.join(gen::REGISTRY);
    if sync {
        let derived = gen::derive_rows(&root.join("spec"))?;
        let existing = match std::fs::read_to_string(&reg_path) {
            Ok(t) => gen::parse_registry(&t)?,
            Err(_) => Vec::new(),
        };
        write(&reg_path, &gen::sync_registry(&derived, &existing))?;
    }
    let text =
        std::fs::read_to_string(&reg_path).map_err(|e| format!("{}: {e}", reg_path.display()))?;
    let rows = gen::parse_registry(&text)?;
    let outputs = [
        (root.join(gen::GENERATED_RUST), gen::generated_rust(&rows)),
        (root.join(gen::REFERENCE), gen::reference_page(&rows)),
    ];
    if check {
        let stale: Vec<String> = outputs
            .iter()
            .filter(|(p, t)| std::fs::read_to_string(p).ok().as_deref() != Some(t.as_str()))
            .map(|(p, _)| p.display().to_string())
            .collect();
        if !stale.is_empty() {
            return Err(format!(
                "stale: {}; rewrite with `cargo run -p via-diag --example gen_reference`",
                stale.join(", ")
            ));
        }
    } else {
        for (p, t) in &outputs {
            write(p, t)?;
        }
    }
    let derived = gen::derive_rows(&root.join("spec"))?;
    let diffs = gen::compare(&derived, &rows);
    for d in &diffs {
        eprintln!("registry differs from the chapters: {d}");
    }
    if !diffs.is_empty() {
        return Err(format!(
            "{} differences; run with --sync to rewrite the derived keys",
            diffs.len()
        ));
    }
    Ok(())
}

fn write(path: &std::path::Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}
