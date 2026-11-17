//! The repository checks that need only the repository's text, as `cargo test` tests, so
//! `cargo test --workspace` (CONTRIBUTING.md, "Checks before a commit") holds them.

fn assert_clean(name: &str, r: Result<xtask::report::Outcome, xtask::report::CheckError>) {
    let o = r.unwrap_or_else(|e| panic!("{name} could not run: {e}"));
    let lines: Vec<String> = o.findings.iter().map(|f| f.to_string()).collect();
    assert!(lines.is_empty(), "{name}:\n{}", lines.join("\n"));
}

#[test]
fn deny_list() {
    assert_clean("deny-list", xtask::denylist::check(&xtask::repo_root()));
}

#[test]
fn maths_crates_only_in_via_num() {
    assert_clean("maths-deps", xtask::mathdeps::check(&xtask::repo_root()));
}

#[test]
fn vocabulary_toml_equals_chapter() {
    assert_clean(
        "vocabulary-sync",
        xtask::denylist::check_sync(&xtask::repo_root()),
    );
}

/// At M0, the milestone being worked on; `xtask check --milestone N` runs it at later ones.
#[test]
fn cases_read_and_cover_m0() {
    assert_clean("cases", xtask::cases::check(&xtask::repo_root(), 0));
}

/// Every M0 and M1 code's `explain` has a `To fix:` line and a bad and a fixed example.
#[test]
fn explain_texts_finished_to_m1() {
    assert_clean("explain", xtask::explain::check(&xtask::repo_root(), 1));
}

#[test]
fn limitations_name_existing_cases() {
    assert_clean(
        "limitations",
        xtask::limitations::check(&xtask::repo_root()),
    );
}

/// Needs uv; on a machine without it the test says so and passes, since CI installs uv before
/// the conformance step and `xtask check` runs it there.
#[test]
fn python_imports() {
    if std::process::Command::new("uv")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("python_imports: uv not found, skipped");
        return;
    }
    assert_clean(
        "python-imports",
        xtask::pyimports::check(&xtask::repo_root()),
    );
}

/// The ledger's rows, pointers and counts (`tools/ledger_check.py`). Needs uv, as above.
#[test]
fn ledger_rows_and_pointers() {
    if std::process::Command::new("uv")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("ledger_rows_and_pointers: uv not found, skipped");
        return;
    }
    assert_clean(
        "ledger",
        xtask::record::check(&xtask::repo_root(), xtask::record::LEDGER),
    );
}

/// The layers page, the rings and the ledger's section 25 (`tools/layers_check.py`). Needs uv.
#[test]
fn layers_page_and_rings() {
    if std::process::Command::new("uv")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("layers_page_and_rings: uv not found, skipped");
        return;
    }
    assert_clean(
        "layers",
        xtask::record::check(&xtask::repo_root(), xtask::record::LAYERS),
    );
}
