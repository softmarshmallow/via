//! The conformance vectors of `spec/documents.md`, as unit tests through the crate's public door.
//!
//! The chapter's cases run through the command line in `tools/conformance`; these hold the same
//! inputs and assertions at the crate, one test per case, named as the case. Names are neutral:
//! the deny-list reads string literals too (`vocabulary.md` 7).

use crate::{load, ChangeRequest, LoadOptions, Loaded, World};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use via_diag::{KeyPath, Location, Record, Resolve, Site, Subject};

static NEXT: AtomicU32 = AtomicU32::new(0);

/// A folder of files under the system's temporary folder, removed on drop.
struct Dir(PathBuf);

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn dir(files: &[(&str, &[u8])]) -> Dir {
    let n = NEXT.fetch_add(1, Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!("via-doc-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    for (p, text) in files {
        let path = root.join(p);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    Dir(root)
}

fn opts(d: &Dir, changes: &[&str], libs: &[&str], via_lib: &[&str]) -> LoadOptions {
    let via = (!via_lib.is_empty())
        .then(|| std::env::join_paths(via_lib.iter().map(|p| d.0.join(p))).unwrap());
    LoadOptions {
        lib_roots: libs.iter().map(|p| d.0.join(p)).collect(),
        via_lib: via,
        shipped_root: None,
        changes: changes
            .iter()
            .enumerate()
            .map(|(i, c)| {
                // A change file's path is given relative to the world folder, as a shell in it would.
                let w = if c.contains('.') || c.contains('/') {
                    d.0.join(c).to_string_lossy().into_owned()
                } else {
                    (*c).to_string()
                };
                ChangeRequest::new(w, u32::try_from(i).unwrap() + 3)
            })
            .collect(),
    }
}

fn merge_with(d: &Dir, o: &LoadOptions) -> Loaded {
    load(&d.0.join("world.toml"), 2, o)
}

fn merge(d: &Dir, changes: &[&str]) -> Loaded {
    merge_with(d, &opts(d, changes, &[], &[]))
}

fn codes(l: &Loaded) -> Vec<String> {
    let mut c: Vec<String> = l.records.iter().map(|r| r.code().to_string()).collect();
    c.sort();
    c
}

fn only<'a>(l: &'a Loaded, code: &str) -> Vec<&'a Record> {
    l.records.iter().filter(|r| r.code() == code).collect()
}

fn json(l: &Loaded) -> String {
    match &l.world {
        Some(w) => w.to_json(),
        None => panic!("no world: {:?}", codes(l)),
    }
}

fn world(l: &Loaded) -> &World {
    l.world
        .as_ref()
        .unwrap_or_else(|| panic!("no world: {:?}", codes(l)))
}

fn span(r: &Record) -> (String, u32, u32) {
    match r.location() {
        Location::Span {
            file, line, column, ..
        } => (file.clone(), *line, *column),
        other => panic!("not a span: {other:?}"),
    }
}

const KINDS: &[u8] = b"[kind.track]\nauthority = \"shaped\"\n[kind.track.defaults]\nwidth = 4\n\n[kind.mark]\nrequires = [\"toward\"]\nnames = [\"toward\"]\n";

/// The worked example of section 10, in neutral words.
fn worked() -> Dir {
    dir(&[
        (
            "world.toml",
            b"name       = \"mill\"
extent     = [0, 0, 128, 128]
resolution = 1
include    = [\"paths\"]
pack       = \"primitive\"

[layer.dressing]
file  = \"layers/dressing.toml\"
owner = \"props\"
rank  = 1

[field.height]
role = \"surface\"
base = 20

[feature.track]
kind = \"track\"
path = [[0, 40], [128, 60]]

[change.wide.feature.track]
width = 6
",
        ),
        (
            "layers/dressing.toml",
            b"[instance.cart]\nasset = \"box_small\"\non    = { ref = \"track\", t = 0.3, offset = 4 }\n",
        ),
        ("changes/no_cart.toml", b"[instance.cart]\nremove = true\n"),
        ("lib/paths/kinds.toml", KINDS),
    ])
}

fn simple(world_text: &str) -> Dir {
    dir(&[
        ("world.toml", world_text.as_bytes()),
        ("lib/paths/kinds.toml", KINDS),
    ])
}

const HEAD: &str = "name = \"w\"\nextent = [0, 0, 64, 64]\ninclude = [\"paths\"]\n";

#[test]
fn worked_example() {
    let d = worked();
    let l = merge(&d, &["wide", "changes/no_cart.toml"]);
    let j = json(&l);
    assert!(
        j.contains(r#""feature":{"track":{"@layer":"world","kind":"track","path":[[0,40],[128,60]],"width":6}}"#),
        "{j}"
    );
    assert!(!j.contains("\"cart\""), "{j}");
    assert!(
        j.starts_with(r#"{"@changes":["changes/no_cart.toml","wide"],"@format":"via-merged-1""#),
        "{j}"
    );
    assert!(j.ends_with("}\n"));
    // Requesting the two in the other order prints the same bytes.
    let l2 = merge(&d, &["changes/no_cart.toml", "wide"]);
    assert_eq!(json(&l2), j);
}

#[test]
fn change_inline() {
    let d = worked();
    let l = merge(&d, &["wide"]);
    let w = world(&l);
    let o = w.origins_json();
    assert!(
        o.contains(r#""width":{"chain":[{"file":"world.toml","line":21,"name":"wide","through":"change"}],"column":1,"file":"world.toml","line":21}"#),
        "{o}"
    );
    let r = w
        .resolve(
            &Subject::entity("feature", "track"),
            &Site::Key(KeyPath::key("width")),
        )
        .unwrap();
    assert_eq!(r.chain.last().unwrap().name, "wide");
}

#[test]
fn change_file() {
    let d = worked();
    let l = merge(&d, &["changes/no_cart.toml"]);
    assert!(world(&l).entity("instance", "cart").is_none());
}

#[test]
fn change_twice() {
    let d = worked();
    let once = json(&merge(&d, &["wide"]));
    let l = merge(&d, &["wide", "wide"]);
    assert!(l.records.is_empty(), "{:?}", codes(&l));
    assert_eq!(json(&l), once);
    let l = merge(
        &d,
        &["changes/no_cart.toml", "changes/../changes/no_cart.toml"],
    );
    assert!(l.records.is_empty(), "{:?}", codes(&l));
}

#[test]
fn upward_and_layer_chain() {
    // The worked example's last sentence: `starts_at = "cart"` on the rank 0 feature.
    let d = worked();
    let text = std::fs::read_to_string(d.0.join("world.toml")).unwrap();
    std::fs::write(
        d.0.join("world.toml"),
        text.replace(
            "kind = \"track\"\npath",
            "kind = \"track\"\nstarts_at = \"cart\"\npath",
        ),
    )
    .unwrap();
    let l = merge(&d, &[]);
    let up = only(&l, "doc.upward_ref");
    assert_eq!(up.len(), 1, "{:?}", codes(&l));
    assert!(
        up[0].message().contains("owner `props`"),
        "{}",
        up[0].message()
    );
    assert!(up[0].message().contains("no owner"));
    assert_eq!(up[0].subject(), "feature.track.starts_at");
}

#[test]
fn shuffle() {
    let a = simple(&format!(
        "{HEAD}[feature.b]\nkind = \"track\"\nat = [1, 2]\n\n[feature.a]\nat = [3, 4]\nkind = \"track\"\n"
    ));
    let b = simple(
        "[feature.a]\nkind = \"track\"\nat = [3, 4]\n[feature.b]\nat = [1, 2]\nkind = \"track\"\n",
    );
    let mut text = std::fs::read_to_string(b.0.join("world.toml")).unwrap();
    text.push_str(
        &HEAD
            .lines()
            .rev()
            .map(|l| format!("{l}\n"))
            .collect::<String>(),
    );
    // Top-level keys after a table would belong to it; write them first, reordered.
    let text = format!(
        "include = [\"paths\"]\nextent = [0, 0, 64, 64]\nname = \"w\"\n{}",
        text.split("include").next().unwrap()
    );
    std::fs::write(b.0.join("world.toml"), text).unwrap();
    assert_eq!(json(&merge(&a, &[])), json(&merge(&b, &[])));
}

#[test]
fn syntax_forms() {
    let a = simple(&format!("{HEAD}[feature.m]\nkind = \"track\"\non = {{ ref = \"n\", t = 0.5 }}\n[feature.n]\npath = [[0, 0], [1, 1]]\n"));
    let b = simple(&format!("{HEAD}[feature.m]\nkind = \"track\"\n[feature.m.on]\nref = \"n\"\nt = 0.5\n[feature.n]\npath = [[0, 0], [1, 1]]\n"));
    assert_eq!(json(&merge(&a, &[])), json(&merge(&b, &[])));
}

#[test]
fn crlf_bom() {
    let text = format!("{HEAD}[feature.m]\nkind = \"track\"\nat = [1, 2]\nwidth = 5\n");
    let a = simple(&text);
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend(text.replace('\n', "\r\n").into_bytes());
    let b = dir(&[("world.toml", &bytes), ("lib/paths/kinds.toml", KINDS)]);
    let (la, lb) = (merge(&a, &[]), merge(&b, &[]));
    assert_eq!(json(&la), json(&lb));
    assert_eq!(world(&la).origins_json(), world(&lb).origins_json());
}

#[test]
fn syntax_v11() {
    let d = simple(&format!(
        "{HEAD}[feature.m]\non = {{ ref = \"n\",\n t = 0.5 }}\n"
    ));
    let l = merge(&d, &[]);
    assert_eq!(codes(&l), ["doc.syntax"]);
    assert!(l.world.is_none());
}

#[test]
fn too_deep() {
    let deep = |n: usize| {
        format!(
            "{HEAD}[step.s.params]\nx = {}1{}\n",
            "[".repeat(n),
            "]".repeat(n)
        )
    };
    // `[step.s.params]` is depth 3, so `x`'s outer list is depth 4 and the n-th is 3 + n.
    let l = merge(&simple(&deep(61)), &[]);
    assert!(
        !codes(&l).contains(&"doc.too_deep".to_string()),
        "{:?}",
        codes(&l)
    );
    let l = merge(&simple(&deep(62)), &[]);
    assert_eq!(codes(&l), ["doc.too_deep"]);
    let l = merge(
        &simple(&format!("{HEAD}[{}]\n", vec!["a"; 65].join("."))),
        &[],
    );
    assert_eq!(codes(&l), ["doc.too_deep"]);
}

#[test]
fn bad_utf8() {
    let d = dir(&[
        (
            "world.toml",
            format!("{HEAD}[layer.d]\nfile = \"d.toml\"\nowner = \"o\"\nrank = 1\n").as_bytes(),
        ),
        ("d.toml", b"[feature.x]\nat = [1, 2] # \xFF\n"),
        ("lib/paths/kinds.toml", KINDS),
    ]);
    let l = merge(&d, &[]);
    assert_eq!(codes(&l), ["doc.syntax"]);
    let r = &l.records[0];
    assert_eq!(
        r.location(),
        &Location::File {
            file: "d.toml".into()
        }
    );
    assert_eq!(r.data().get("offset"), Some(&via_diag::Value::Int(26)));
}

#[test]
fn array_table() {
    let d = simple(&format!(
        "{HEAD}[[feature]]\nid = \"mill\"\n[[feature]]\nid = \"cart\"\n"
    ));
    let l = merge(&d, &[]);
    assert_eq!(codes(&l), ["doc.syntax"]);
    assert_eq!(
        l.records[0].data().get("rewrite"),
        Some(&via_diag::Value::text("[feature.mill]"))
    );
    assert_eq!(via_diag::exit_status(&l.records), 1);
}

#[test]
fn inline_list() {
    let d = simple(&format!(
        "{HEAD}[scatter.s]\nassets = [{{ id = \"a\", weight = 2 }}, \"b\"]\nspacing = 7\n"
    ));
    let l = merge(&d, &[]);
    assert!(l.records.is_empty(), "{:?}", codes(&l));
    assert!(json(&l).contains(r#""assets":[{"id":"a","weight":2},"b"]"#));
}

#[test]
fn unknown_key() {
    let d = simple(&format!(
        "{HEAD}\n[feature.m]\nkind = \"track\"\nwidht = 6\n"
    ));
    let l = merge(&d, &[]);
    assert_eq!(codes(&l), ["doc.unknown_key"]);
    let r = &l.records[0];
    assert_eq!(span(r), ("world.toml".into(), 7, 1));
    assert_eq!(
        r.data().get("nearest"),
        Some(&via_diag::Value::text("width"))
    );
    assert_eq!(r.hint(), "write `width`");
}

#[test]
fn fixed_hints() {
    let d = simple(&format!(
        "{HEAD}[feature.m]\nover = 1\non = {{ feature = \"x\", t = 0.5 }}\n[params]\na = 1\n"
    ));
    let l = merge(&d, &[]);
    let hints: Vec<&str> = l.records.iter().map(Record::hint).collect();
    assert!(hints.contains(&"write `[let]`"), "{hints:?}");
    let d = simple(&format!(
        "{HEAD}[feature.m]\nover = 1\non = {{ feature = \"x\", t = 0.5 }}\n"
    ));
    let l = merge(&d, &[]);
    let hints: Vec<&str> = l.records.iter().map(Record::hint).collect();
    assert!(hints.contains(&"write `ref`"), "{hints:?}");
    assert!(
        hints.iter().any(|h| h.starts_with("`over` was dropped")),
        "{hints:?}"
    );
}

#[test]
fn not_yet() {
    // `type = "volume"` is no longer a later word (`surfaces.md` 1.3): the model says
    // `field.type_unknown`, so the document reads it as a string with nothing to say.
    let l = merge(
        &simple(&format!("{HEAD}[field.a]\ntype = \"volume\"\nbase = 0\n")),
        &[],
    );
    assert!(codes(&l).is_empty(), "{:?}", codes(&l));
    // `chunk` and `[budget]` arrived in M6 (`build.md` 5, 5.1): read with nothing to say.
    let l = merge(
        &simple(&format!(
            "chunk = 64\n{HEAD}[budget]\ntriangles = 1000\n[budget.waive.a]\nat = [0, 0]\ntriangles = 2000\nreason = \"r\"\n"
        )),
        &[],
    );
    assert!(codes(&l).is_empty(), "{:?}", codes(&l));
    // A waiver says why (`doc.missing_key`), and a budget's limits are integers.
    let l = merge(
        &simple(&format!(
            "{HEAD}[budget]\ntriangles = 1.5\n[budget.waive.a]\nat = [0, 0]\n"
        )),
        &[],
    );
    assert_eq!(codes(&l), ["doc.missing_key", "doc.type"]);
    // M2's words are read: a worker generate step is no longer `doc.not_yet`; nor, from M6, is an
    // export step, which may name its `output` (`protocol.md` 1).
    let l = merge(
        &simple(&format!(
            "{HEAD}[step.s]\nwhen = \"generate\"\ncommand = [\"x\"]\nworker = true\n"
        )),
        &[],
    );
    assert!(codes(&l).is_empty(), "{:?}", codes(&l));
    let l = merge(
        &simple(&format!(
            "{HEAD}[step.s]\nwhen = \"export\"\ncommand = [\"x\"]\noutput = \"player_map\"\n"
        )),
        &[],
    );
    assert!(codes(&l).is_empty(), "{:?}", codes(&l));
}

#[test]
fn datetime() {
    // `num.seed_type` is a string or a float (`numbers.md` 7); a date is the general fault.
    let l = merge(&simple(&format!("seed = 1979-05-27\n{HEAD}")), &[]);
    assert_eq!(codes(&l), ["doc.type"]);
}

#[test]
fn integer_key_and_hex_literal() {
    let w = |rank: &str| {
        dir(&[
            (
                "world.toml",
                format!("{HEAD}[layer.d]\nfile = \"d.toml\"\nowner = \"o\"\nrank = {rank}\n")
                    .as_bytes(),
            ),
            ("d.toml", b""),
            ("lib/paths/kinds.toml", KINDS),
        ])
    };
    let l = merge(&w("1.0"), &[]);
    assert_eq!(codes(&l), ["doc.type"]);
    let l = merge(&w("0x10"), &[]);
    assert!(
        json(&l).contains(r#""d":{"file":"d.toml","owner":"o","rank":16}"#),
        "{}",
        json(&l)
    );
}

#[test]
fn missing_key() {
    let d = dir(&[
        (
            "world.toml",
            b"name = \"w\"\n[layer.d]\nfile = \"d.toml\"\nrank = 1\n",
        ),
        ("d.toml", b""),
    ]);
    let l = merge(&d, &[]);
    assert_eq!(codes(&l), ["doc.missing_key", "doc.missing_key"]);
    let layer = l.records.iter().find(|r| r.subject() == "layer.d").unwrap();
    assert_eq!(span(layer), ("world.toml".into(), 2, 1));
    let top = l.records.iter().find(|r| r.subject() == "world").unwrap();
    assert_eq!(span(top), ("world.toml".into(), 1, 1));
}

fn two_docs(world_tail: &str, layer: &str) -> Dir {
    dir(&[
        (
            "world.toml",
            format!("{HEAD}[layer.d]\nfile = \"d.toml\"\nowner = \"o\"\nrank = 1\n{world_tail}")
                .as_bytes(),
        ),
        ("d.toml", layer.as_bytes()),
        ("lib/paths/kinds.toml", KINDS),
    ])
}

#[test]
fn dup_layers_and_foreign_patch() {
    let l = merge(
        &two_docs(
            "[feature.x]\nat = [1, 2]\n",
            "[feature.x]\nkind = \"track\"\n",
        ),
        &[],
    );
    assert_eq!(codes(&l), ["identity.duplicate_id"]);
    let r = &l.records[0];
    assert_eq!(span(r).0, "d.toml");
    assert_eq!(span(&Record::clone(r)).1, 1);
    assert!(r.message().contains("world.toml") && r.message().contains("d.toml"));
    assert!(r
        .hint()
        .contains("edit it where it is declared, in `world.toml`, or write a change"));
    assert_eq!(r.related().len(), 1);
}

#[test]
fn dup_cross_and_world_id() {
    let l = merge(
        &simple(&format!(
            "{HEAD}[feature.m]\nat = [1, 2]\n[instance.m]\nasset = \"a\"\nat = [1, 2]\n"
        )),
        &[],
    );
    assert_eq!(codes(&l), ["identity.duplicate_id"]);
    let l = merge(
        &simple(&format!("{HEAD}[feature.world]\nat = [1, 2]\n")),
        &[],
    );
    assert_eq!(codes(&l), ["identity.reserved"]);
    let l = merge(
        &simple(&format!("{HEAD}[feature.Mill]\nat = [1, 2]\n")),
        &[],
    );
    assert_eq!(codes(&l), ["identity.bad_id"]);
    assert_eq!(l.records[0].hint(), "write `mill`");
}

#[test]
fn dup_let_field() {
    // `formula.shadow` is `formulas.md`'s; this chapter raises nothing.
    let l = merge(
        &simple(&format!(
            "{HEAD}[let]\nheight = 2\n[field.height]\nbase = 1\n"
        )),
        &[],
    );
    assert!(l.records.is_empty(), "{:?}", codes(&l));
}

#[test]
fn dup_kind() {
    let d = dir(&[
        (
            "world.toml",
            b"name = \"w\"\nextent = [0, 0, 8, 8]\ninclude = [\"a\", \"b\"]\n",
        ),
        ("lib/a/kinds.toml", b"[kind.k]\n"),
        ("lib/b/kinds.toml", b"[kind.k]\n"),
    ]);
    let l = merge(&d, &[]);
    assert_eq!(codes(&l), ["identity.duplicate_id"]);
    assert!(l.records[0].message().contains("lib:a/kinds.toml"));
    assert!(l.records[0].message().contains("lib:b/kinds.toml"));
}

#[test]
fn layer_rank() {
    let d = dir(&[
        ("world.toml", b"name = \"w\"\nextent = [0, 0, 8, 8]\n[layer.a]\nfile = \"a.toml\"\nowner = \"o\"\nrank = 1\n[layer.b]\nfile = \"b.toml\"\nowner = \"o\"\nrank = 1\n"),
        ("a.toml", b""),
        ("b.toml", b""),
    ]);
    assert_eq!(codes(&merge(&d, &[])), ["doc.layer_rank"]);
    let d = dir(&[
        ("world.toml", b"name = \"w\"\nextent = [0, 0, 8, 8]\n[layer.a]\nfile = \"a.toml\"\nowner = \"o\"\nrank = 0\n"),
        ("a.toml", b""),
    ]);
    assert_eq!(codes(&merge(&d, &[])), ["doc.layer_rank"]);
}

#[test]
fn layer_file_twice() {
    let d = dir(&[
        ("world.toml", b"name = \"w\"\nextent = [0, 0, 8, 8]\n[layer.a]\nfile = \"a.toml\"\nowner = \"o\"\nrank = 1\n[layer.b]\nfile = \"./a.toml\"\nowner = \"o\"\nrank = 2\n"),
        ("a.toml", b""),
    ]);
    let l = merge(&d, &[]);
    assert_eq!(codes(&l), ["doc.layer_file_twice", "doc.layer_file_twice"]);
}

fn lib_world(include: &str) -> String {
    format!("name = \"w\"\nextent = [0, 0, 8, 8]\ninclude = [{include}]\n")
}

#[test]
fn include_lookup() {
    let d = dir(&[
        ("w/world.toml", lib_world("\"paths\"").as_bytes()),
        ("w/lib/paths/kinds.toml", b"[kind.near]\n"),
        ("root/paths/kinds.toml", b"[kind.far]\n"),
    ]);
    let l = load(&d.0.join("w/world.toml"), 2, &opts(&d, &[], &["root"], &[]));
    assert_eq!(codes(&l), ["doc.include_shadowed"]);
    assert!(l.records[0].message().contains("beside is used"));
    assert!(world(&l).entity("kind", "near").is_some());
}

#[test]
fn include_env() {
    let d = dir(&[
        ("world.toml", lib_world("\"paths\", \"land\"").as_bytes()),
        (
            "cli/paths/kinds.toml",
            b"[kind.p]\n[kind.p.defaults]\nwidht = 1\n",
        ),
        ("env/paths/kinds.toml", b"[kind.q]\n"),
        ("env/land/kinds.toml", b"[kind.l]\nauthority = \"shaped\"\n"),
    ]);
    let l = merge_with(&d, &opts(&d, &[], &["cli"], &["env"]));
    assert_eq!(codes(&l), ["doc.include_shadowed"]);
    assert!(l.records[0].message().contains("--lib 1"));
    assert!(l.records[0].message().contains("VIA_LIB 1"));
    let w = world(&l);
    assert!(w.entity("kind", "p").is_some() && w.entity("kind", "l").is_some());
    assert!(w
        .origins_json()
        .contains(r#""file":"lib:paths/kinds.toml""#));
    assert!(w.origins_json().contains(r#""file":"lib:land/kinds.toml""#));
}

#[test]
fn include_missing_roots() {
    let d = dir(&[
        ("world.toml", lib_world("\"wyas\"").as_bytes()),
        ("cli/other/kinds.toml", b""),
        ("env/ways/kinds.toml", b""),
    ]);
    let l = merge_with(&d, &opts(&d, &[], &["cli"], &["env"]));
    assert_eq!(codes(&l), ["doc.include_missing"]);
    let r = &l.records[0];
    for root in ["beside", "--lib 1", "VIA_LIB 1", "shipped"] {
        assert!(r.message().contains(root), "{}", r.message());
    }
    assert_eq!(
        r.data().get("nearest"),
        Some(&via_diag::Value::text("ways"))
    );
    let out = via_diag::document(&l.records, &[], 1);
    assert!(!out.contains(&d.0.to_string_lossy().into_owned()), "{out}");
}

#[test]
fn include_code_only() {
    let d = dir(&[
        ("world.toml", lib_world("\"steps\"").as_bytes()),
        ("lib/steps/run.py", b"print()\n"),
    ]);
    let l = merge(&d, &[]);
    assert!(l.records.is_empty(), "{:?}", codes(&l));
    assert!(world(&l).ids("kind").is_empty());
}

#[test]
fn include_case() {
    let d = dir(&[
        ("world.toml", lib_world("\"paths\"").as_bytes()),
        ("lib/Paths/kinds.toml", b""),
    ]);
    assert_eq!(codes(&merge(&d, &[])), ["doc.path_case"]);
}

#[test]
fn include_cycle_and_once() {
    let d = dir(&[
        ("world.toml", lib_world("\"a\"").as_bytes()),
        ("lib/a/kinds.toml", b"include = [\"b\"]\n"),
        ("lib/b/kinds.toml", b"include = [\"a\"]\n"),
    ]);
    let l = merge(&d, &[]);
    assert_eq!(codes(&l), ["doc.include_cycle"]);
    assert!(l.records[0].message().contains("a -> b -> a"));
    let d = dir(&[
        ("world.toml", lib_world("\"a\", \"b\"").as_bytes()),
        ("lib/a/kinds.toml", b"include = [\"land\"]\n"),
        ("lib/b/kinds.toml", b"include = [\"land\"]\n"),
        ("lib/land/kinds.toml", b"[kind.k]\n"),
    ]);
    let l = merge(&d, &[]);
    assert!(l.records.is_empty(), "{:?}", codes(&l));
}

#[test]
fn span_portable() {
    let make = || {
        dir(&[
            ("world.toml", lib_world("\"paths\"").as_bytes()),
            ("root/paths/kinds.toml", b"[kind.k]\nwidht = 1\n"),
        ])
    };
    let (a, b) = (make(), make());
    let la = merge_with(&a, &opts(&a, &[], &["root"], &[]));
    let lb = merge_with(&b, &opts(&b, &[], &["root"], &[]));
    let (ja, jb) = (
        via_diag::document(&la.records, &[], 1),
        via_diag::document(&lb.records, &[], 1),
    );
    assert_eq!(ja, jb);
    assert!(ja.contains("\"file\":\"lib:paths/kinds.toml\""), "{ja}");
}

fn changes_world(changes: &str) -> Dir {
    simple(&format!(
        "{HEAD}[feature.x]\nkind = \"track\"\npath = [[0, 0], [9, 9]]\nends_at = \"y\"\nsmooth = true\n[feature.y]\nat = [9, 9]\n[instance.i]\nasset = \"a\"\non = {{ ref = \"x\", t = 0.5 }}\n{changes}"
    ))
}

#[test]
fn change_empty_nested_layer_kind_pack_fixed() {
    let cases: [(&str, &[&str]); 7] = [
        ("[change.c]\n", &["doc.change_empty"]),
        (
            "[change.c.change.b]\nfeature.x.width = 1\n",
            &["doc.change_nested"],
        ),
        (
            "[change.c.layer.extra]\nfile = \"e.toml\"\n",
            &["doc.not_yet"],
        ),
        (
            "[change.c.kind.fresh]\nauthority = \"shaped\"\n",
            &["doc.kind_outside_library"],
        ),
        (
            "[change.c.asset.cart]\nprimitive = \"box\"\n",
            &["doc.unknown_key"],
        ),
        (
            "[change.c]\ninclude = [\"x\"]\nname = \"n\"\n",
            &["doc.change_fixed", "doc.change_fixed"],
        ),
        ("[change.c]\nremove = true\n", &["doc.remove_above_entity"]),
    ];
    for (text, want) in cases {
        let l = merge(&changes_world(text), &["c"]);
        assert_eq!(codes(&l), want, "{text}");
    }
    let l = merge(
        &changes_world("[change.c.asset.cart]\nprimitive = \"box\"\n"),
        &["c"],
    );
    assert_eq!(
        l.records[0].hint(),
        "a change reaches a pack only by swapping it: write `pack`"
    );
}

#[test]
fn commute_and_conflicts() {
    let d = changes_world(
        "[change.a.feature.x]\nwidth = 5\n[change.b.feature.y]\nat = [1, 1]\n[change.e.feature.x]\nwidth = 5\n\
         [change.n1.feature.fresh]\nat = [1, 1]\n[change.n2.feature.fresh]\nkind = \"track\"\n\
         [change.r.feature.x]\nremove = true\n[change.k.feature.x]\nkind = \"mark\"\n\
         [change.pa.feature.y]\nat = [2, 2]\n[change.po.feature.y]\non = { ref = \"x\", t = 0.1 }\n",
    );
    assert_eq!(json(&merge(&d, &["a", "b"])), json(&merge(&d, &["b", "a"])));
    for pair in [["a", "e"], ["n1", "n2"], ["r", "k"], ["pa", "po"]] {
        let l = merge(&d, &pair);
        assert_eq!(codes(&l), ["doc.change_conflict"], "{pair:?}");
        assert!(
            l.records[0].message().contains(pair[0]) && l.records[0].message().contains(pair[1])
        );
        assert_eq!(l.records[0].related().len(), 1);
    }
}

#[test]
fn placement_clear_and_twice() {
    let d = changes_world("[change.c.feature.x]\non = { ref = \"y\", t = 0.5 }\n");
    let l = merge(&d, &["c"]);
    let w = world(&l);
    let x = w.entity("feature", "x").unwrap();
    assert!(x.get("on").is_some());
    for gone in ["path", "ends_at", "smooth"] {
        assert!(x.get(gone).is_none(), "{gone}");
    }
    let l = merge(
        &simple(&format!(
            "{HEAD}[feature.x]\nat = [1, 1]\non = {{ ref = \"y\" }}\n"
        )),
        &[],
    );
    assert_eq!(codes(&l), ["doc.placement_twice"]);
}

#[test]
fn unset_default() {
    let d = changes_world("[feature.z]\nkind = \"track\"\nat = [1, 1]\nwidth = 3\n[change.c.feature.z]\nunset = [\"width\"]\n");
    let l = merge(&d, &["c"]);
    let j = json(&l);
    assert!(
        j.contains(r#""z":{"@layer":"world","at":[1,1],"kind":"track"}"#),
        "{j}"
    );
    assert!(j.contains(r#""defaults":{"width":4}"#), "{j}");
}

#[test]
fn removals() {
    let cases: [(&str, &str); 3] = [
        (
            "[change.c.feature.x]\nunset = [\"nope\"]\n",
            "doc.remove_missing",
        ),
        (
            "[change.c.feature.x]\nremove = true\nkind = \"track\"\n",
            "doc.remove_with_keys",
        ),
        ("[feature.q]\nremove = true\n", "doc.remove_outside_change"),
    ];
    for (text, want) in cases {
        let l = merge(&changes_world(text), &["c"]);
        let l = if text.starts_with("[feature") {
            merge(&changes_world(text), &[])
        } else {
            l
        };
        assert_eq!(codes(&l), [want], "{text}");
    }
}

#[test]
fn dangling() {
    let l = merge(
        &changes_world("[change.c.feature.x]\nremove = true\n"),
        &["c"],
    );
    assert_eq!(codes(&l), ["doc.dangling"]);
    let r = &l.records[0];
    assert_eq!(r.subject(), "instance.i.on.ref");
    assert!(r.message().contains("`c`"));
}

#[test]
fn adds_near() {
    let l = merge(
        &changes_world("[change.c.feature.yy]\nat = [1, 1]\n"),
        &["c"],
    );
    assert_eq!(codes(&l), ["doc.change_adds_near"]);
    assert_eq!(
        l.records[0].data().get("nearest"),
        Some(&via_diag::Value::text("y"))
    );
    assert!(l.world.is_some());
    // A warning of a finished world is still located in its file.
    assert_eq!(span(&l.records[0]).0, "world.toml");
}

#[test]
fn on_partial() {
    let l = merge(
        &changes_world("[change.c.instance.i.on]\nt = 0.8\n"),
        &["c"],
    );
    assert_eq!(codes(&l), ["doc.missing_key"]);
    assert!(
        l.records[0].message().contains("`ref`"),
        "{}",
        l.records[0].message()
    );
    assert!(l.records[0].hint().contains("write the whole table"));
}

#[test]
fn idle_change() {
    let l = merge(&changes_world("[change.c.feature.x]\nwidht = 1\n"), &[]);
    assert_eq!(codes(&l), ["doc.idle_change"]);
    assert_eq!(
        l.records[0].data().get("code"),
        Some(&via_diag::Value::text("doc.unknown_key"))
    );
    assert_eq!(via_diag::exit_status(&l.records), 0);
    assert_eq!(span(&l.records[0]).0, "world.toml");
}

#[test]
fn pack_swap() {
    let d = dir(&[
        (
            "world.toml",
            format!("{HEAD}pack = \"primitive\"\n[feature.x]\nat = [1, 1]\n").as_bytes(),
        ),
        ("lib/paths/kinds.toml", KINDS),
        ("changes/bigger.toml", b"pack = \"other\"\n"),
    ]);
    let plain = json(&merge(&d, &[]));
    let swapped = json(&merge(&d, &["changes/bigger.toml"]));
    assert!(swapped.contains(r#""pack":"other""#));
    assert_eq!(
        swapped
            .replace(r#""pack":"other""#, r#""pack":"primitive""#)
            .replace(r#""@changes":["changes/bigger.toml"]"#, r#""@changes":[]"#),
        plain
    );
}

#[test]
fn unknown_change_and_change_files() {
    let d = changes_world("[change.downstream.feature.x]\nwidth = 1\n");
    let l = merge(&d, &["dowstream"]);
    assert_eq!(codes(&l), ["doc.unknown_change"]);
    assert_eq!(l.records[0].hint(), "write `downstream`");
    assert_eq!(l.records[0].location(), &Location::Argument(3));
    std::fs::create_dir_all(d.0.join("changes")).unwrap();
    for f in ["Winter.toml", "no_cart.v2.toml", "winter.tml"] {
        std::fs::write(d.0.join("changes").join(f), "pack = \"x\"\n").unwrap();
    }
    let l = merge(&d, &["changes/Winter.toml", "changes/no_cart.v2.toml"]);
    assert_eq!(codes(&l), ["identity.bad_id", "identity.bad_id"]);
    let l = merge(&d, &["changes/winter.tml"]);
    assert_eq!(codes(&l), ["doc.path_form"]);
}

#[test]
fn paths_in_text() {
    let w = |file: &str| {
        dir(&[
            ("world.toml", format!("name = \"w\"\nextent = [0, 0, 8, 8]\n[layer.d]\nfile = '{file}'\nowner = \"o\"\nrank = 1\n").as_bytes()),
            ("layers/d.toml", b""),
        ])
    };
    assert_eq!(codes(&merge(&w("Layers/d.toml"), &[])), ["doc.path_case"]);
    for bad in ["layers\\d.toml", "C:x.toml", "aux.toml"] {
        assert_eq!(codes(&merge(&w(bad), &[])), ["doc.path_form"], "{bad}");
    }
    let l = merge(&w("layers/e.toml"), &[]);
    assert_eq!(codes(&l), ["doc.file_missing"]);
    assert_eq!(l.records[0].hint(), "write `d.toml`");
}

#[test]
fn file_missing() {
    let d = dir(&[]);
    let l = load(&d.0.join("nowhere.toml"), 2, &LoadOptions::default());
    assert_eq!(codes(&l), ["doc.file_missing"]);
    assert_eq!(l.records[0].location(), &Location::Argument(2));
}

#[test]
fn formula_span() {
    // An unknown name in a basic string with an escape before it: the column a sub-expression
    // reports is the name's column in the file. `formulas.md` finds the name; this is the map.
    let line = "base = \"= \\u0031 + nme\"";
    let d = simple(&format!("{HEAD}[field.h]\n{line}\n"));
    let l = merge(&d, &[]);
    let w = world(&l);
    let f = w.formulas();
    let at = f
        .iter()
        .find(|f| f.subject.to_string() == "field.h.base")
        .unwrap();
    let start = at.text.text.find("nme").unwrap() as u32;
    let r = w
        .resolve(
            &Subject::entity("field", "h"),
            &Site::Expr(KeyPath::key("base"), start..start + 3),
        )
        .unwrap();
    let Location::Span {
        line: ln, column, ..
    } = r.location
    else {
        panic!()
    };
    assert_eq!((ln, column as usize), (5, line.find("nme").unwrap() + 1));
}

#[test]
fn params_free_and_json_escape() {
    let d = simple(
        "name = \"é\\t\\\"\"\nextent = [0, 0, 8, 8]\ninclude = [\"paths\"]\n[step.s]\nwhen = \"lint\"\ncommand = [\"x\"]\n[step.s.params]\nany = { deep = [1.50, 0x10], word = \"w\" }\n",
    );
    let l = merge(&d, &[]);
    assert!(l.records.is_empty(), "{:?}", codes(&l));
    let j = json(&l);
    assert!(
        j.contains(r#""params":{"any":{"deep":[1.50,16],"word":"w"}}"#),
        "{j}"
    );
    assert!(j.contains("\"name\":\"é\\u0009\\\"\""), "{j}");
}

#[test]
fn numbers_print_by_what_they_take() {
    let d = simple(&format!(
        "seed = 7\nresolution = 0.5\n{HEAD}[feature.m]\nkind = \"track\"\nat = [1.25, 2]\nface = -90\nwidth = 1e1\n"
    ));
    let l = merge(&d, &[]);
    let j = json(&l);
    assert!(j.contains(r#""seed":"0000000000000007""#), "{j}");
    assert!(j.contains(r#""resolution":0.5"#), "{j}");
    assert!(j.contains(r#""at":[1.25,2]"#), "{j}");
    assert!(j.contains(r#""face":270"#), "{j}");
    assert!(j.contains(r#""width":10"#), "{j}");
    let l = merge(
        &simple(&format!("{HEAD}[feature.m]\nat = [1.2345, 2]\n")),
        &[],
    );
    assert_eq!(codes(&l), ["num.too_precise"]);
    let l = merge(&simple(&format!("{HEAD}[field.h]\nbase = \"20\"\n")), &[]);
    assert_eq!(codes(&l), ["formula.unmarked"]);
    let l = merge(
        &simple(&format!("{HEAD}[field.h]\noutside = \"= 2\"\n")),
        &[],
    );
    assert_eq!(codes(&l), ["doc.type"]);
    assert!(l.records[0].hint().contains("takes no formula"));
}

#[test]
fn reserved_attribute_and_names() {
    let d = dir(&[
        ("world.toml", lib_world("\"k\"").as_bytes()),
        ("lib/k/kinds.toml", b"[kind.k.defaults]\nat = 1\n"),
    ]);
    assert_eq!(codes(&merge(&d, &[])), ["doc.reserved_attribute"]);
    // A name attribute is checked for direction like `on.ref`.
    let d = two_docs(
        "[feature.m]\nkind = \"mark\"\nat = [1, 1]\ntoward = \"up\"\n",
        "[feature.up]\nat = [2, 2]\n",
    );
    assert_eq!(codes(&merge(&d, &[])), ["doc.upward_ref"]);
}

#[test]
fn layer_values_carry_their_layer() {
    let d = two_docs("", "[feature.up]\nat = [2, 2]\n");
    let l = merge(&d, &[]);
    let w = world(&l);
    assert_eq!(w.home("feature", "up"), Some("d"));
    assert_eq!(w.rank("feature", "up"), Some(1));
    let r = w
        .resolve(
            &Subject::entity("feature", "up"),
            &Site::Key(KeyPath::key("at")),
        )
        .unwrap();
    assert_eq!(r.chain.len(), 1);
    assert_eq!(r.chain[0].owner.as_deref(), Some("o"));
    assert_eq!(r.chain[0].file, "d.toml");
}

#[test]
fn world_paths_never_absolute() {
    let d = worked();
    let l = merge(&d, &["changes/no_cart.toml"]);
    let w = world(&l);
    for s in w.sources().iter() {
        assert!(!Path::new(&s.name).is_absolute(), "{}", s.name);
    }
}

/// The plan's M1 world, read from the example folder every M1 case copies, at run time
/// so no library word is written here. Its libraries are stood in for by kinds generated from what it uses; each change
/// file and inline change reads, and the four bad edits pass reading (their faults are later
/// nodes': authority, formulas, the graph, the pack).
#[test]
fn m1_world_reads() {
    // The example world with all four bad edits, found by its contents.
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let case = std::fs::read_dir(&examples)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.join("changes/bad_cycle.toml").is_file() && p.join("world.toml").is_file())
        .unwrap();
    let world_text = std::fs::read_to_string(case.join("world.toml")).unwrap();
    let mut files: Vec<(String, Vec<u8>)> =
        vec![("world.toml".into(), world_text.clone().into_bytes())];
    let mut change_files = Vec::new();
    for e in std::fs::read_dir(case.join("changes")).unwrap() {
        let e = e.unwrap();
        let name = e.file_name().to_string_lossy().into_owned();
        files.push((format!("changes/{name}"), std::fs::read(e.path()).unwrap()));
        change_files.push(format!("changes/{name}"));
    }
    change_files.sort();
    // Kinds: every `kind` a feature names and every `ground` an instance names, with the
    // attributes their features write; `toward` is a name attribute.
    let src = crate::SrcId(0);
    let root = crate::read::read(src, &world_text).unwrap_or_else(|_| panic!());
    let mut kinds: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
        Default::default();
    for (_, e) in root.table("feature").unwrap().iter() {
        let crate::Value::Table(t) = &e.item.value else {
            panic!()
        };
        let attrs = kinds.entry(t.str("kind").unwrap().to_string()).or_default();
        for k in t.keys() {
            if !crate::schema::FEATURE_RESERVED.contains(&k) {
                attrs.insert(k.to_string());
            }
        }
    }
    for g in ["ground"] {
        for (_, e) in root.table("instance").unwrap().iter() {
            let crate::Value::Table(t) = &e.item.value else {
                panic!()
            };
            kinds.entry(t.str(g).unwrap().to_string()).or_default();
            kinds
                .entry(format!("firm_{}", t.str(g).unwrap()))
                .or_default();
        }
    }
    let mut kinds_text = String::new();
    for (k, attrs) in &kinds {
        kinds_text.push_str(&format!("[kind.{k}]\n"));
        if !attrs.is_empty() {
            let list: Vec<String> = attrs.iter().map(|a| format!("\"{a}\"")).collect();
            kinds_text.push_str(&format!(
                "requires = [{}]\nnames = [{}]\n",
                list.join(", "),
                list.join(", ")
            ));
        }
    }
    let include = root.value("include").unwrap();
    let crate::Value::List(libs) = include else {
        panic!()
    };
    for (i, lib) in libs.iter().enumerate() {
        let crate::Value::Str(s) = &lib.value else {
            panic!()
        };
        let text = if i == 0 {
            kinds_text.clone()
        } else {
            String::new()
        };
        files.push((format!("libs/{}/kinds.toml", s.text), text.into_bytes()));
    }
    let refs: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(p, b)| (p.as_str(), b.as_slice()))
        .collect();
    let d = dir(&refs);
    let base = merge_with(&d, &opts(&d, &[], &["libs"], &[]));
    assert!(base.records.is_empty(), "{:?}", base.records);
    let base_json = json(&base);
    let inline: Vec<String> = root
        .table("change")
        .unwrap()
        .keys()
        .map(str::to_string)
        .collect();
    assert_eq!(inline.len(), 3);
    for c in inline.iter().chain(&change_files) {
        let l = merge_with(&d, &opts(&d, &[c.as_str()], &["libs"], &[]));
        assert!(l.records.is_empty(), "{c}: {:?}", l.records);
        assert_ne!(json(&l), base_json, "{c}");
    }
    let all: Vec<&str> = change_files
        .iter()
        .filter(|c| c.contains("bad_"))
        .map(String::as_str)
        .collect();
    assert_eq!(all.len(), 4);
    let l = merge_with(&d, &opts(&d, &all, &["libs"], &[]));
    assert!(l.records.is_empty(), "{:?}", l.records);
    let both: Vec<&str> = inline.iter().map(String::as_str).collect();
    let l = merge_with(&d, &opts(&d, &both, &["libs"], &[]));
    assert!(l.records.is_empty(), "{:?}", l.records);
    // The world file's own text is untouched, and the merged world prints within the reader's
    // own rules: line 1's `name` is the first top-level key.
    assert!(base_json.contains("\"layer\":{\"world\":{\"rank\":0}}"));
}

#[test]
fn change_file_adds_in_the_layer_it_names_down_from() {
    // documents.md 6.1, case `change_file_layer`: a clear naming a scatter of layer `d` (rank 1)
    // is declared in `d`, not `world`, so it is no upward reference; a pin naming nothing above
    // rank 0 stays in `world`; an inline change of the world file still adds in `world`, and the
    // same clear written there is `doc.upward_ref`.
    let clear = "on = { of = \"trees\", near = [5, 5] }\nby = [\"trees\"]\n";
    let make = |tail: &str| {
        dir(&[
            (
                "world.toml",
                format!("{HEAD}[layer.d]\nfile = \"d.toml\"\nowner = \"o\"\nrank = 1\n{tail}")
                    .as_bytes(),
            ),
            (
                "d.toml",
                b"[scatter.trees]\nwithin = \"plot\"\nassets = [\"tree\"]\nspacing = 4\n\n\
                  [feature.plot]\nregion = [[0, 0], [10, 0], [10, 10], [0, 10]]\n",
            ),
            ("lib/paths/kinds.toml", KINDS),
            (
                "edits.toml",
                format!("[clear.c1]\n{clear}\n[pin.p1]\nasset = \"tree\"\nat = [2, 2]\n")
                    .as_bytes(),
            ),
        ])
    };
    let d = make("");
    let l = merge(&d, &["edits.toml"]);
    assert_eq!(codes(&l), Vec::<String>::new());
    let w = world(&l);
    assert_eq!(w.home("clear", "c1"), Some("d"));
    assert_eq!(w.rank("clear", "c1"), Some(1));
    assert_eq!(w.home("pin", "p1"), Some("world"));
    let d = make(&format!("[change.inline.clear.c2]\n{clear}"));
    // Both names (`on.of` and `by`) reach up.
    assert_eq!(
        codes(&merge(&d, &["inline"])),
        ["doc.upward_ref", "doc.upward_ref"]
    );
}

#[test]
fn change_file_rehome_follows_its_own_adds() {
    // documents.md 6.1, case `change_file_chain`: a clear naming a scatter the same file adds,
    // itself within a feature of layer `d`, rises to `d` on the second pass (the clear is
    // applied first, so one pass would leave it in `world`, an upward reference). The formula
    // sentence of 6.1 is checked in via-model, case `change_file_formula`.
    let make = |edits: &str| {
        dir(&[
            (
                "world.toml",
                format!("{HEAD}[layer.d]\nfile = \"d.toml\"\nowner = \"o\"\nrank = 1\n").as_bytes(),
            ),
            (
                "d.toml",
                b"[feature.copse]\nregion = [[0, 0], [10, 0], [10, 10], [0, 10]]\n\n[let]\nrise = 2\n",
            ),
            ("lib/paths/kinds.toml", KINDS),
            ("edits.toml", edits.as_bytes()),
        ])
    };
    let d = make(
        "[clear.c1]\non = { of = \"under\", near = [5, 5] }\nby = [\"under\"]\n\n\
         [scatter.under]\nwithin = \"copse\"\nassets = [\"tree\"]\nspacing = 4\n",
    );
    let l = merge(&d, &["edits.toml"]);
    assert_eq!(codes(&l), Vec::<String>::new());
    let w = world(&l);
    assert_eq!(w.home("scatter", "under"), Some("d"));
    assert_eq!(w.home("clear", "c1"), Some("d"));
}

#[test]
fn a_run_names_a_list_of_assets() {
    // `rules.md` 5 (R68): `assets` on a run is a list of asset ids, one for each member, and the
    // merged world keeps it as written. A scatter's choices (`{ id, weight }`) are not a run's:
    // a table there, or a single name, is no list of names.
    let l = merge(
        &simple(&format!(
            "{HEAD}[feature.line]\npath = [[0, 8], [32, 8]]\n\n\
             [run.row]\nassets = [\"box\", \"post\", \"box\"]\non = {{ ref = \"line\" }}\n"
        )),
        &[],
    );
    assert_eq!(codes(&l), Vec::<String>::new());
    let j = json(&l);
    let want = r#""assets":["box","post","box"]"#;
    assert!(j.contains(want), "{want} in {j}");
    for bad in [
        "assets = [{ id = \"box\", weight = 2 }]",
        "assets = \"box\"",
        "assets = [1, 2]",
    ] {
        let l = merge(
            &simple(&format!(
                "{HEAD}[feature.line]\npath = [[0, 8], [32, 8]]\n\n\
                 [run.row]\n{bad}\non = {{ ref = \"line\" }}\n"
            )),
            &[],
        );
        assert_eq!(codes(&l), ["doc.type"], "{bad}");
    }
}

#[test]
fn at_with_a_height() {
    // `surfaces.md` 4.1 (R65), `vocabulary.md` 2.4: `at` on an instance, a pin and a point
    // feature takes two lengths, or two lengths and a number, the height; the merged world
    // writes the height as a number. A set item's `at` and a clear's take two numbers only, and
    // four numbers are no point anywhere.
    let l = merge(
        &simple(&format!(
            "{HEAD}[instance.a]\nasset = \"box\"\nat = [1, 2.5, 13.3004]\n\n\
             [pin.p]\nasset = \"box\"\nat = [3, 4, 10]\n\n\
             [feature.f]\nat = [5, 6, -0.5]\n\n\
             [instance.b]\nasset = \"box\"\nat = [7, 8]\n"
        )),
        &[],
    );
    assert_eq!(codes(&l), Vec::<String>::new());
    let j = json(&l);
    for want in [
        r#""at":[1,2.5,13.3004]"#,
        r#""at":[3,4,10]"#,
        r#""at":[5,6,-0.5]"#,
        r#""at":[7,8]"#,
    ] {
        assert!(j.contains(want), "{want} in {j}");
    }
    for bad in [
        "[set.s.item.i]\nasset = \"box\"\nat = [1, 2, 3]\n",
        "[clear.c]\nat = [1, 2, 3]\nradius = 2\n",
        "[instance.a]\nasset = \"box\"\nat = [1, 2, 3, 4]\n",
        "[instance.a]\nasset = \"box\"\nat = [1, 2, \"high\"]\n",
    ] {
        let l = merge(&simple(&format!("{HEAD}{bad}")), &[]);
        assert_eq!(codes(&l), ["doc.type"], "{bad}");
    }
    // The plan point stays on the lattice: a fourth decimal there is too precise, in the
    // height it is not.
    let l = merge(
        &simple(&format!(
            "{HEAD}[instance.a]\nasset = \"box\"\nat = [1.0001, 2, 3]\n"
        )),
        &[],
    );
    assert_eq!(codes(&l), ["num.too_precise"]);
}
