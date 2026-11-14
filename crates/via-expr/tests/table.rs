//! The function table against its machine copy and its chapter (`formulas.md` 8 and 11, R33):
//! `spec/functions.toml` and section 8 of `spec/formulas.md` name exactly the functions of
//! `via_expr::table::TABLE`, with equal columns, and every row dispatches: an M1 function checks
//! and evaluates in a context its Where column allows, a later one is `doc.not_yet`. A row
//! removed from the evaluator alone fails here.

use std::collections::BTreeSet;
use std::path::PathBuf;

use via_expr::table::TABLE;
use via_expr::{AttrType, Context, Env, Key};

fn spec(file: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec")
        .join(file);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

#[test]
fn table_equals_functions_toml() {
    let doc: toml::Table = toml::from_str(&spec("functions.toml")).expect("functions.toml parses");
    assert_eq!(
        doc.get("format").and_then(|v| v.as_str()),
        Some("via-functions-1")
    );
    let rows = doc
        .get("function")
        .and_then(|v| v.as_table())
        .expect("[function]");
    let file: BTreeSet<&str> = rows.keys().map(String::as_str).collect();
    let code: BTreeSet<&str> = TABLE.iter().map(|f| f.name).collect();
    assert_eq!(
        file, code,
        "functions.toml and the evaluator name different functions"
    );
    for f in TABLE {
        let r = rows[f.name].as_table().expect("a table");
        let s = |k: &str| {
            r.get(k)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string()
        };
        let i = |k: &str| r.get(k).and_then(|v| v.as_integer()).unwrap_or(-1);
        assert_eq!(i("order"), i64::from(f.order), "{}: order", f.name);
        assert_eq!(
            i("milestone"),
            i64::from(f.milestone),
            "{}: milestone",
            f.name
        );
        assert_eq!(s("section"), f.section, "{}: section", f.name);
        assert_eq!(s("rate"), f.rate_text, "{}: rate", f.name);
        assert_eq!(s("halo"), f.halo_text, "{}: halo", f.name);
        assert_eq!(s("where"), f.where_text, "{}: where", f.name);
        let sigs: Vec<String> = r["signatures"]
            .as_array()
            .expect("signatures")
            .iter()
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(sigs, f.signatures, "{}: signatures", f.name);
        // Every parameter named in a signature is a parameter of the row, in order.
        if f.milestone == 1 && !f.variadic {
            let names: Vec<&str> = f.params.iter().map(|p| p.name).collect();
            let longest = sigs
                .iter()
                .max_by_key(|s| s.len())
                .cloned()
                .unwrap_or_default();
            let inner =
                &longest[longest.find('(').unwrap_or(0) + 1..longest.rfind(')').unwrap_or(0)];
            let written: Vec<&str> = inner
                .split(',')
                .map(|p| p.split('=').next().unwrap_or("").trim())
                .collect();
            assert_eq!(written, names, "{}: parameters", f.name);
        }
    }
    let orders: Vec<u32> = TABLE.iter().map(|f| f.order).collect();
    assert_eq!(
        orders,
        (1..=TABLE.len() as u32).collect::<Vec<_>>(),
        "the table is in section 8's order"
    );
}

#[test]
fn table_equals_section_8() {
    let text = spec("formulas.md");
    let start = text.find("## 8. The function table").expect("section 8");
    let end = text.find("## 9. Evaluation").expect("section 9");
    let mut names = BTreeSet::new();
    // Rows of the tables headed `| Function |`; the vector and halo tables are not function rows.
    let mut in_table = false;
    for line in text[start..end].lines() {
        if line.starts_with("| Function |") {
            in_table = true;
            continue;
        }
        if !line.starts_with('|') {
            in_table = false;
        }
        if !in_table {
            continue;
        }
        let Some(first) = line.strip_prefix("| `") else {
            continue;
        };
        let cell = format!("`{}", first.split(" |").next().unwrap_or(""));
        for part in cell.split('`').skip(1).step_by(2) {
            if let Some(p) = part.find('(') {
                names.insert(part[..p].to_string());
            }
        }
    }
    let code: BTreeSet<String> = TABLE.iter().map(|f| f.name.to_string()).collect();
    assert_eq!(
        names, code,
        "section 8 and the evaluator name different functions"
    );
}

#[test]
fn every_row_dispatches() {
    let mut env = Env::new();
    env.field("h");
    env.feature("p", Some("k"));
    env.kind_attribute("k", "w", AttrType::Number);
    let calls = [
        ("abs", "abs(-1)"),
        ("sqrt", "sqrt(4)"),
        ("pow", "pow(2, 3)"),
        ("exp", "exp(0)"),
        ("floor", "floor(1.5)"),
        ("sin", "sin(90)"),
        ("cos", "cos(0)"),
        ("tan", "tan(45)"),
        ("min", "min(1, 2)"),
        ("max", "max(1, 2)"),
        ("clamp", "clamp(1, 0, 2)"),
        ("mix", "mix(0, 1, 0.5)"),
        ("smoothstep", "smoothstep(0, 1, 0.5)"),
        ("if", "if(1 > 0, 1, 0)"),
        ("fbm", "fbm(10)"),
        ("ridged", "ridged(10)"),
        ("f", "f('h')"),
        ("slope", "slope('h')"),
        ("dist", "dist('p', 5)"),
        ("sdist", "sdist('p', 5)"),
        ("near", "near('p', 5)"),
        ("inside", "if(inside('p'), 1, 0)"),
        ("along", "along('p')"),
        ("under", "under('mean')"),
        ("smoothed_along", "smoothed_along(4)"),
    ];
    let named: BTreeSet<&str> = calls.iter().map(|(n, _)| *n).collect();
    let code: BTreeSet<&str> = TABLE.iter().map(|f| f.name).collect();
    assert_eq!(named, code, "this test calls every row");
    for f in TABLE {
        let call = calls
            .iter()
            .find(|(n, _)| *n == f.name)
            .map(|(_, c)| *c)
            .unwrap_or("");
        let id = env.text(&format!("= {call}"));
        let ctx = Context::new(Key::EffectValue)
            .kind("k")
            .noise(&["noise", "t"]);
        let r = env.check(id, &ctx);
        // This engine reads M2: every row up to M2 checks; a later one is `doc.not_yet`.
        if f.milestone <= 2 {
            assert!(r.is_ok(), "{}: {:?}", f.name, r.err());
        } else {
            assert_eq!(
                r.err().map(|e| e.code_str()),
                Some("doc.not_yet"),
                "{}",
                f.name
            );
        }
    }
}
