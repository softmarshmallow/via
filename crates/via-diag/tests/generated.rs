//! The registry's generated files are current, and the registry matches the chapters
//! (`spec/diagnostics.md` section 2 and the workspace checks of its Conformance section).

#[path = "../tools/gen.rs"]
mod gen;

fn registry() -> Vec<gen::Row> {
    let text = std::fs::read_to_string(gen::root().join(gen::REGISTRY))
        .unwrap_or_else(|e| panic!("registry: {e}"));
    gen::parse_registry(&text).unwrap_or_else(|e| panic!("{e}"))
}

fn assert_current(rel: &str, want: &str) {
    let have = std::fs::read_to_string(gen::root().join(rel)).unwrap_or_default();
    assert!(
        have == want,
        "{rel} is stale; run `cargo run -p via-diag --example gen_reference`"
    );
}

#[test]
fn generated_rust_is_current() {
    assert_current(gen::GENERATED_RUST, &gen::generated_rust(&registry()));
}

#[test]
fn reference_page_is_current() {
    assert_current(gen::REFERENCE, &gen::reference_page(&registry()));
}

#[test]
fn registry_matches_every_chapter_table() {
    let derived = gen::derive_rows(&gen::root().join("spec")).unwrap_or_else(|e| panic!("{e}"));
    let diffs = gen::compare(&derived, &registry());
    assert!(
        diffs.is_empty(),
        "the registry differs from the chapters (run the generator with --sync):\n{}",
        diffs.join("\n")
    );
}

#[test]
fn compiled_table_equals_the_registry() {
    let reg = registry();
    let rows = via_diag::registry::rows();
    assert_eq!(rows.len(), reg.len());
    for (a, b) in rows.iter().zip(&reg) {
        assert_eq!(a.code, b.code);
        assert_eq!(a.severity.as_str(), b.severity);
        assert_eq!(a.explain, b.explain);
    }
}

#[test]
fn derivation_rules() {
    let rows = gen::chapter_rows(
        "x",
        "## Diagnostics\n\n| Code | Severity | Says | Hint |\n|---|---|---|---|\n\
         | `a.one` | error | Bad; `data.limit`, worst first by data.limit. (M3) | The nearest name. |\n\
         | `a.internal` | error | Ours. | Report it. |\n\
         | `load.x` | warning | Reader. | The nearest value. |\n\n\
         ## Other\n\n| Code | Severity | Says |\n|---|---|---|\n| `b.no` | info | Not read. |\n",
    )
    .unwrap();
    assert_eq!(rows.len(), 3);
    let one = &rows[0].row;
    assert_eq!(one.milestone, 3);
    assert_eq!(one.data, ["limit", "nearest"]);
    assert_eq!(one.worst.as_deref(), Some("limit"));
    assert_eq!(one.covered_by, "conformance");
    assert_eq!(rows[1].row.covered_by, "workspace");
    assert_eq!(rows[2].row.covered_by, "reader");
    assert!(
        rows[2].row.data.is_empty(),
        "the nearest value is not a nearest name"
    );
}
