//! `spec/formulas.md`'s Conformance cases and vectors that this crate decides alone, run against
//! the library (the command-line cases are the conformance runner's). Numbers in comments are
//! the chapter's case numbers. Names are neutral (`spec/vocabulary.md` 7).

use std::collections::BTreeMap;

use via_expr::{
    bind, evaluate, seed_shared, AttrType, Binding, Context, Domain, Env, ExprError, Failure,
    FieldInfo, Key, Measure, Program, ReadError, Reads, Scalar, Select, Setup, Shape, ShapeInfo,
    Stat, Window,
};
use via_num::Grid;

type Surface = fn(i64, i64) -> f64;
type Measures = fn(Measure, i64, i64) -> f64;

/// A small world: fields as functions of millimetres, features with fixed measures.
struct World {
    grid: Grid,
    fields: BTreeMap<&'static str, (Surface, Vec<&'static str>, bool)>,
    attrs: BTreeMap<String, Scalar>,
    members: BTreeMap<(String, String), Scalar>,
    shapes: Vec<(&'static str, &'static str, Shape, Measures)>,
    own: Option<Shape>,
    v: f64,
}

impl World {
    fn new(extent: [i64; 4], resolution: i64) -> World {
        World {
            grid: Grid::new(extent, resolution).expect("grid"),
            fields: BTreeMap::new(),
            attrs: BTreeMap::new(),
            members: BTreeMap::new(),
            shapes: Vec::new(),
            own: None,
            v: 0.0,
        }
    }

    fn inside(&self, x: i64, y: i64) -> bool {
        let [x0, y0, x1, y1] = self.grid.extent;
        (x0..=x1).contains(&x) && (y0..=y1).contains(&y)
    }
}

impl Reads for World {
    fn attribute(&self, feature: Option<&str>, name: &str) -> Result<Scalar, ReadError> {
        let v = match feature {
            None => self.attrs.get(name),
            Some(f) => self.members.get(&(f.to_string(), name.to_string())),
        };
        v.copied()
            .ok_or_else(|| ReadError::new(via_diag::codes::diag::INTERNAL, "no attribute", "-"))
    }
    fn field(&self, field: &str, domain: &Domain) -> Result<Vec<Option<f64>>, ReadError> {
        let (f, _, _) = self.fields[field];
        Ok(domain
            .points()
            .into_iter()
            .map(|(x, y)| self.inside(x, y).then(|| f(x, y)))
            .collect())
    }
    fn v(&self, domain: &Domain) -> Result<Vec<f64>, ReadError> {
        Ok(vec![self.v; domain.len()])
    }
    fn local(&self, m: Measure, domain: &Domain) -> Result<Vec<f64>, ReadError> {
        Ok(domain
            .points()
            .iter()
            .map(|&(x, _)| {
                if m == Measure::T {
                    0.0
                } else {
                    (x as f64) / 1000.0
                }
            })
            .collect())
    }
    fn measure(
        &self,
        feature: &str,
        m: Measure,
        domain: &Domain,
        _cap: Option<f64>,
    ) -> Result<Vec<f64>, ReadError> {
        let (_, _, _, f) = self
            .shapes
            .iter()
            .find(|s| s.0 == feature)
            .expect("feature");
        Ok(domain.points().iter().map(|&(x, y)| f(m, x, y)).collect())
    }
    fn under(&self, _field: Option<&str>, _stat: Stat) -> Result<f64, ReadError> {
        Err(ReadError::new(
            via_diag::codes::num::EMPTY_REDUCTION,
            "the feature has no samples",
            "make the region larger",
        ))
    }
}

impl Select for World {
    fn fields(&self, selector: &str) -> Vec<FieldInfo> {
        self.fields
            .iter()
            .filter(|(n, (_, tags, _))| match selector.strip_prefix("tag:") {
                Some(t) => tags.contains(&t),
                None => **n == selector,
            })
            .map(|(n, (_, _, w))| FieldInfo {
                name: n.to_string(),
                weights: *w,
            })
            .collect()
    }
    fn shapes(&self, selector: &str) -> Vec<ShapeInfo> {
        self.shapes
            .iter()
            .filter(|(id, kind, _, _)| match selector.strip_prefix("kind:") {
                Some(k) => *kind == k,
                None => *id == selector,
            })
            .map(|(id, _, s, _)| ShapeInfo {
                id: id.to_string(),
                shape: *s,
            })
            .collect()
    }
    fn own_shape(&self) -> Option<Shape> {
        self.own
    }
}

/// A world 5 by 3 posts at 1 m, x from 98 m to 102 m: case 41's `sqrt(x - 100)` fails at its
/// first post and holds at its last.
fn small() -> World {
    let mut w = World::new([98_000, 0, 102_000, 2_000], 1_000);
    w.fields.insert(
        "height",
        (|x, _| (x as f64) / 2000.0, vec!["ground"], false),
    );
    w
}

fn env_for(w: &World) -> Env {
    let mut env = Env::new();
    for n in w.fields.keys() {
        env.field(n);
    }
    for (id, kind, _, _) in &w.shapes {
        env.feature(id, Some(kind));
    }
    env
}

fn base() -> Context {
    Context::new(Key::FieldBase)
        .field("height")
        .noise(&["noise", "field", "height", "base"])
}

fn check(text: &str, ctx: &Context) -> Result<Program, ExprError> {
    let w = small();
    let mut env = env_for(&w);
    let id = env.text(text);
    env.check(id, ctx)
}

fn code(r: Result<Program, ExprError>) -> &'static str {
    match r {
        Ok(_) => "ok",
        Err(e) => e.code_str(),
    }
}

fn whole(w: &World, p: &Program, b: &Binding) -> Result<Vec<f64>, Box<Failure>> {
    let d = Domain::Window {
        grid: w.grid,
        window: Window::whole(&w.grid),
    };
    evaluate(
        p,
        b,
        w,
        &Setup {
            world_seed: 0,
            feature: Some("p1"),
        },
        &d,
    )
}

fn run_in(w: &World, env: &mut Env, text: &str, ctx: &Context) -> Result<Vec<f64>, Box<Failure>> {
    let id = env.text(text);
    let p = env.check(id, ctx).unwrap_or_else(|e| panic!("{text}: {e}"));
    let b = bind(&p, w).unwrap_or_else(|e| panic!("{text}: {e}"));
    whole(w, &p, &b)
}

fn run(text: &str) -> Result<Vec<f64>, Box<Failure>> {
    let w = small();
    let mut env = env_for(&w);
    run_in(&w, &mut env, text, &base())
}

fn value(text: &str) -> f64 {
    run(text).unwrap_or_else(|e| panic!("{text}: {}", e.error))[0]
}

fn fails(text: &str) -> ExprError {
    run(text)
        .err()
        .unwrap_or_else(|| panic!("{text} did not fail"))
        .error
}

/// The text a failure's span covers.
fn spanned(text: &str, e: &ExprError) -> String {
    let s = e.span().expect("a span");
    text[s.start as usize..s.end as usize].to_string()
}

// Text phase.

#[test]
fn case_01_unmarked() {
    let e = check("20 + y", &base()).unwrap_err();
    assert_eq!(e.code_str(), "formula.unmarked");
    assert_eq!(e.span(), Some(0..6));
    let e = check(" = 20", &base()).unwrap_err();
    assert_eq!(e.code_str(), "formula.unmarked");
    assert!(e.hint.contains("remove"), "{}", e.hint);
    assert_eq!(
        via_expr::marked(" = 1", via_expr::Mark::Attribute),
        via_expr::Marked::Unmarked
    );
    assert_eq!(
        via_expr::marked("text", via_expr::Mark::Attribute),
        via_expr::Marked::Text
    );
    assert_eq!(
        via_expr::marked("text", via_expr::Mark::Key),
        via_expr::Marked::Unmarked
    );
}

#[test]
fn case_02_constant_number() {
    let w = small();
    let mut env = env_for(&w);
    let id = env.number(20.0);
    let p = env.check(id, &base()).unwrap();
    let b = bind(&p, &w).unwrap();
    assert!(whole(&w, &p, &b).unwrap().iter().all(|&v| v == 20.0));
}

#[test]
fn case_03_empty() {
    let e = check("=", &base()).unwrap_err();
    assert_eq!(e.code_str(), "formula.syntax");
    assert!(e.message.contains("expected an expression"));
}

#[test]
fn case_04_operators_of_other_languages() {
    let wh = Context::new(Key::Where);
    let e = check("= (x > 1) & (y > 1)", &wh).unwrap_err();
    assert_eq!(
        (e.code_str(), e.hint.as_str()),
        ("formula.operator", "write `and`")
    );
    let e = check("= x = 1", &base()).unwrap_err();
    assert_eq!(
        (e.code_str(), e.hint.as_str()),
        ("formula.operator", "write `==`")
    );
    for t in [
        "= 2 ** 3",
        "= 7 % 2",
        "= x > 1 ? 1 : 0",
        "= x | 1",
        "= !x",
        "= 2 ^ 3",
    ] {
        assert_eq!(code(check(t, &base())), "formula.operator", "{t}");
    }
    let e = check("= f(\"height\")", &base()).unwrap_err();
    assert_eq!(
        (e.code_str(), e.hint.as_str()),
        ("formula.operator", "selectors take single quotes")
    );
}

#[test]
fn case_05_characters() {
    let e = check("= Sin(x)", &base()).unwrap_err();
    assert_eq!(
        (e.code_str(), e.hint.as_str()),
        ("formula.character", "names are lower case")
    );
    assert_eq!(e.span(), Some(2..3));
    assert_eq!(code(check("= x $ 1", &base())), "formula.character");
    assert_eq!(code(check("= sIn(x)", &base())), "formula.character");
    assert_eq!(code(check("= x + é", &base())), "formula.character");
}

#[test]
fn case_06_comparisons_do_not_chain() {
    let e = check("= 0 < x < 5", &base()).unwrap_err();
    assert_eq!(e.code_str(), "formula.syntax");
    assert_eq!(e.hint, "write `0 < x and x < 5`");
}

#[test]
fn case_07_number_literals() {
    for t in [".5", "5.", "1e", "2e5x", "1.e3", "1_0", "1e400", "1e-400"] {
        assert_eq!(
            code(check(&format!("= {t}"), &base())),
            "formula.syntax",
            "{t}"
        );
    }
    assert_eq!(check("= .5", &base()).unwrap_err().hint, "write `0.5`");
    let v = value("= 1e-3");
    assert_eq!(v.to_bits(), 0.001f64.to_bits());
    assert_eq!(
        via_expr::narrow(&[v]).unwrap()[0].to_bits(),
        0.001f32.to_bits()
    );
    assert_eq!(value("= 1E3"), 1000.0);
}

#[test]
fn case_08_size_and_depth() {
    let long = format!("= {}11", "1+".repeat(2047));
    assert_eq!(long.len() - 1, 4097);
    assert_eq!(code(check(&long, &base())), "formula.too_long");
    let ok = format!("= {}1", "1+".repeat(2047));
    assert_eq!(ok.len() - 1, 4096);
    assert_eq!(code(check(&ok, &base())), "ok");
    let deep = format!("= {}1{}", "(".repeat(65), ")".repeat(65));
    assert_eq!(code(check(&deep, &base())), "formula.too_deep");
    let deep = format!("= {}1{}", "(".repeat(64), ")".repeat(64));
    assert_eq!(code(check(&deep, &base())), "ok");
    let deep = format!("= {}1{}", "abs(".repeat(65), ")".repeat(65));
    assert_eq!(code(check(&deep, &base())), "formula.too_deep");
    assert_eq!(
        code(check(&format!("= {}1", "-".repeat(65)), &base())),
        "formula.too_deep"
    );
    assert_eq!(code(check("= +1", &base())), "formula.syntax");
}

#[test]
fn case_09_precedence() {
    assert!(run("= -2 * 3 + 4 / 2 - 1")
        .unwrap()
        .iter()
        .all(|&v| v == -5.0));
    assert_eq!(value("= 2 - 3 - 4"), -5.0);
    assert_eq!(value("= if(not 1 > 2 and 2 > 1 or 1 > 2, 1, 0)"), 1.0);
}

#[test]
fn case_10_shadowing() {
    let mut env = Env::new();
    env.kind_attribute("k", "d", AttrType::Number);
    env.kind_attribute("k", "width", AttrType::Number);
    let one = env.text("= 1");
    env.kind_let("k", "width", one);
    env.field("height");
    env.world_let("height", one);
    let found: Vec<(&str, String)> = env
        .shadows()
        .iter()
        .map(|e| (e.code_str(), e.message.clone()))
        .collect();
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(found.iter().all(|(c, _)| *c == "formula.shadow"));
    assert_eq!(
        found
            .iter()
            .filter(|(_, m)| m.contains("name of a field"))
            .count(),
        1
    );
    // A kind attribute named like a function is legal.
    let mut env = Env::new();
    env.kind_attribute("k", "floor", AttrType::Number);
    assert!(env.shadows().is_empty());
    let id = env.text("= floor(floor)");
    assert!(env
        .check(id, &Context::new(Key::EffectValue).kind("k"))
        .is_ok());
}

#[test]
fn case_11_inner_scope_hides_the_world() {
    let mut w = small();
    w.attrs.insert("width".into(), Scalar::Number(4.0));
    let mut env = env_for(&w);
    let one = env.text("= 1");
    env.world_let("width", one);
    env.kind_attribute("k", "width", AttrType::Number);
    let ctx = Context::new(Key::EffectValue).kind("k");
    assert!(run_in(&w, &mut env, "= width", &ctx)
        .unwrap()
        .iter()
        .all(|&v| v == 4.0));
    // The field base sees the world's.
    assert!(run_in(&w, &mut env, "= width", &base())
        .unwrap()
        .iter()
        .all(|&v| v == 1.0));
}

#[test]
fn case_12_names_by_context() {
    let e = check("= d", &base()).unwrap_err();
    assert_eq!(e.code_str(), "formula.unknown_name");
    assert!(
        e.message.contains("exists in a kind effect"),
        "{}",
        e.message
    );
    let e = check("= heigth", &base()).unwrap_err();
    assert_eq!(e.code_str(), "formula.unknown_name");
    let e = check("= height", &base()).unwrap_err();
    assert!(e.hint.contains("f('height')"), "{}", e.hint);
    assert_eq!(code(check("= fbm", &base())), "formula.unknown_name");
    assert_eq!(code(check("= v", &base())), "formula.unknown_name");
    let then = Context::new(Key::ThenValue).field("height");
    assert_eq!(code(check("= v + 1", &then)), "ok");
    assert_eq!(
        code(check("= v + 1", &then.clone().weights(true))),
        "formula.unknown_name"
    );
    let mc = Context::new(Key::MaxChange).kind("k");
    assert_eq!(code(check("= t", &mc)), "formula.unknown_name");
    assert_eq!(code(check("= z", &base())), "doc.not_yet");
    // A kind surface's base (`surfaces.md` 1.1, M3): `t`, `d`, `sd`, never a field.
    let surf = Context::new(Key::SurfaceBase);
    assert_eq!(code(check("= 1 + t + d + sd", &surf)), "ok");
    assert_eq!(code(check("= f('height')", &surf)), "formula.unknown_name");
    let bound = Context::new(Key::SurfaceBound);
    // `t`, `d`, `sd`, `x`, `y` in `bound` are `formula.rate` (`surfaces.md` 1.1).
    for n in ["d", "t", "sd", "x", "y"] {
        assert_eq!(
            code(check(&format!("= {n}"), &bound)),
            "formula.rate",
            "{n}"
        );
    }
}

#[test]
fn case_13_unknown_functions() {
    let e = check("= clip(x, 0, 1)", &base()).unwrap_err();
    assert_eq!(
        (e.code_str(), e.hint.as_str()),
        ("formula.unknown_function", "write `clamp`")
    );
    assert_eq!(e.span(), Some(2..6));
    assert_eq!(
        check("= lerp(0, 1, 0.5)", &base()).unwrap_err().hint,
        "write `mix`"
    );
    let e = check("= smoothstp(0, 1, x)", &base()).unwrap_err();
    assert_eq!(
        e.data.get("nearest"),
        Some(&via_diag::Value::text("smoothstep"))
    );
    // Outside its Where column.
    assert_eq!(
        code(check("= under('mean')", &base())),
        "formula.unknown_function"
    );
    // `ridged` arrives in M2, which this engine reads.
    assert!(check("= ridged(10)", &base()).is_ok());
}

#[test]
fn case_14_arguments() {
    for t in [
        "= fbm(60, 3, octaves = 4)",
        "= fbm(wavelength = 60, 3)",
        "= fbm(60, foo = 1)",
        "= clamp(x, 0)",
        "= min(1)",
        "= abs(1, 2)",
        "= min(a = 1, 2)",
    ] {
        assert_eq!(code(check(t, &base())), "formula.arguments", "{t}");
    }
    assert_eq!(
        code(check("= fbm(60, lacunarity = 3, octaves = 2)", &base())),
        "ok"
    );
}

#[test]
fn case_15_types() {
    assert_eq!(
        code(check("= 1 and x > 0", &Context::new(Key::Where))),
        "formula.type"
    );
    assert_eq!(
        code(check("= if(x > 1, 1, x > 2)", &base())),
        "formula.type"
    );
    assert_eq!(code(check("= if(x, 1, 0)", &base())), "formula.type");
    assert_eq!(
        code(check("= 1 > 0", &Context::new(Key::Attribute))),
        "formula.type"
    );
    assert_eq!(code(check("= 'height' + 1", &base())), "formula.type");
    assert_eq!(code(check("= x > 1", &base())), "formula.type");
    let mut env = Env::new();
    env.kind_attribute("k", "label", AttrType::Text);
    let id = env.text("= label");
    assert_eq!(
        code(env.check(id, &Context::new(Key::EffectValue).kind("k"))),
        "formula.type"
    );
    // A bool attribute reads as yes/no.
    env.kind_attribute("k", "on", AttrType::YesNo);
    let id = env.text("= if(on, 1, 0)");
    assert_eq!(
        code(env.check(id, &Context::new(Key::EffectValue).kind("k"))),
        "ok"
    );
    // A `where` takes yes/no or a number.
    assert_eq!(code(check("= x > 1", &Context::new(Key::Where))), "ok");
}

#[test]
fn case_16_rates() {
    let e = check("= f('height')", &Context::new(Key::Attribute)).unwrap_err();
    assert_eq!(e.code_str(), "formula.rate");
    assert!(
        e.message.starts_with("`f('height')` is per post"),
        "{}",
        e.message
    );
    let t = "= 1 + 2 * f('height')";
    let e = check(t, &Context::new(Key::Attribute)).unwrap_err();
    assert_eq!(spanned(t, &e), "f('height')");
    let e = check("= fbm(x, 3)", &base()).unwrap_err();
    assert_eq!(e.code_str(), "formula.rate");
    assert!(e.message.contains("`wavelength` of `fbm`"), "{}", e.message);
    assert_eq!(
        code(check(
            "= f('height')",
            &Context::new(Key::MaxChange).kind("k")
        )),
        "formula.rate"
    );
    // A per-post world let read by an attribute is a rate error; a constant one is not.
    let mut env = Env::new();
    let px = env.text("= x * 2");
    let c = env.text("= 3");
    env.world_let("px", px);
    env.world_let("c", c);
    let a = env.text("= c + 1");
    assert_eq!(code(env.check(a, &Context::new(Key::Attribute))), "ok");
    let a = env.text("= px + 1");
    assert_eq!(
        code(env.check(a, &Context::new(Key::Attribute))),
        "formula.rate"
    );
}

#[test]
fn case_17_constant_parameter_ranges() {
    for t in [
        "= fbm(60, 3.5)",
        "= fbm(60, 13)",
        "= fbm(60, 3, gain = 0)",
        "= fbm(60, 3, lacunarity = 1)",
        "= fbm(-1)",
        "= fbm(0)",
    ] {
        assert_eq!(code(check(t, &base())), "formula.range", "{t}");
    }
    assert_eq!(code(check("= fbm(60, 12, gain = 1)", &base())), "ok");
}

#[test]
fn case_18_seed_is_a_name() {
    assert_eq!(
        code(check("= fbm(60, 3, seed = 2)", &base())),
        "formula.seed"
    );
    assert_eq!(
        code(check("= fbm(60, 3, seed = 'Two')", &base())),
        "formula.seed"
    );
    assert_eq!(code(check("= fbm(60, 3, seed = 'two')", &base())), "ok");
}

#[test]
fn case_19_cap() {
    let mut w = small();
    w.shapes.push(("brook", "k", Shape::Path, |_, _, _| 1.0));
    let mut env = env_for(&w);
    let id = env.text("= dist('brook')");
    assert_eq!(code(env.check(id, &base())), "formula.cap");
    let effect = Context::new(Key::EffectValue).kind("k");
    assert_eq!(code(env.check(id, &effect)), "formula.cap");
    assert_eq!(code(env.check(id, &effect.radius(12.0))), "ok");
}

#[test]
fn case_20_selector_form_and_class() {
    let mut w = small();
    w.shapes.push(("brook", "k", Shape::Path, |_, _, _| 1.0));
    let mut env = env_for(&w);
    let mut c = |t: &str| {
        let id = env.text(t);
        env.check(id, &base())
            .err()
            .map(|e| (e.code_str(), e.hint.clone()))
    };
    let (code1, hint) = c("= f('role:surface')").unwrap();
    assert_eq!(code1, "formula.selector");
    assert!(hint.contains("tag:"));
    assert_eq!(
        c("= near('height', 5)").map(|e| e.0),
        Some("formula.selector")
    );
    assert_eq!(
        c("= dist('maker#e3', 10)").map(|e| e.0),
        Some("identity.key_in_text")
    );
    assert_eq!(c("= f('brook')").map(|e| e.0), Some("formula.selector"));
    assert_eq!(c("= f('hieght')").map(|e| e.0), Some("formula.selector"));
    assert_eq!(c("= dist('a.b.c', 10)").map(|e| e.0), Some("doc.not_yet"));
    assert_eq!(c("= f(height)").map(|e| e.0), Some("formula.unknown_name"));
    assert_eq!(c("= f(1)").map(|e| e.0), Some("formula.type"));
    assert_eq!(c("= dist('kind:k', 10) + f('tag:ground')"), None);
}

// Selection phase.

fn region(m: Measure, x: i64, _y: i64) -> f64 {
    match m {
        Measure::Inside => {
            if x <= 100_000 {
                1.0
            } else {
                0.0
            }
        }
        Measure::Sd => (100_000 - x) as f64 / 1000.0,
        Measure::D => via_num::max((x - 100_000) as f64 / 1000.0, 0.0),
        Measure::T => 0.25,
    }
}

fn path(m: Measure, x: i64, _y: i64) -> f64 {
    match m {
        Measure::Sd => (x - 99_000) as f64 / 1000.0,
        Measure::D => ((x - 99_000) as f64 / 1000.0).abs(),
        _ => 0.5,
    }
}

fn shaped() -> World {
    let mut w = small();
    w.fields
        .insert("other", (|_, _| 7.0, vec!["ground"], false));
    w.fields.insert("cover", (|_, _| 1.0, vec![], true));
    w.shapes
        .push(("pond", "basin", Shape::Region { rings: 1 }, region));
    w.shapes.push(("a1", "track", Shape::Path, path));
    w.shapes.push(("a2", "track", Shape::Path, path));
    w
}

fn bind_code(w: &World, text: &str, ctx: &Context) -> &'static str {
    let mut env = env_for(w);
    let id = env.text(text);
    let p = env.check(id, ctx).unwrap_or_else(|e| panic!("{text}: {e}"));
    match bind(&p, w) {
        Ok(_) => "ok",
        Err(e) => e.code_str(),
    }
}

#[test]
fn case_22_selector_count() {
    let w = shaped();
    assert_eq!(
        bind_code(&w, "= f('tag:ground')", &base()),
        "formula.selector_count"
    );
    assert_eq!(
        bind_code(&w, "= sdist('kind:track', 20)", &base()),
        "formula.selector_count"
    );
    assert_eq!(
        bind_code(&w, "= along('kind:track')", &base()),
        "formula.selector_count"
    );
    assert_eq!(
        bind_code(&w, "= f('tag:none')", &base()),
        "formula.selector_count"
    );
    assert_eq!(bind_code(&w, "= f('cover')", &base()), "formula.type");
}

#[test]
fn case_23_empty_selection() {
    let w = shaped();
    let mut env = env_for(&w);
    for (t, v) in [
        ("= dist('kind:none', 50)", 50.0),
        ("= sdist('kind:none', 50)", -50.0),
        ("= near('kind:none', 50)", 0.0),
        ("= if(inside('kind:none'), 1, 0)", 0.0),
    ] {
        let id = env.text(t);
        let p = env.check(id, &base()).unwrap();
        let b = bind(&p, &w).unwrap();
        assert_eq!(b.warnings.len(), 1, "{t}");
        assert_eq!(b.warnings[0].code_str(), "formula.empty_selection");
        assert!(whole(&w, &p, &b).unwrap().iter().all(|&x| x == v), "{t}");
    }
    let id = env.text("= dist('kind:none', 50) + dist('kind:none', 50)");
    let p = env.check(id, &base()).unwrap();
    assert_eq!(bind(&p, &w).unwrap().warnings.len(), 1, "once per formula");
}

#[test]
fn case_24_shapes() {
    let mut w = shaped();
    assert_eq!(
        bind_code(&w, "= if(inside('a1'), 1, 0)", &base()),
        "formula.shape"
    );
    assert_eq!(bind_code(&w, "= if(inside('pond'), 1, 0)", &base()), "ok");
    w.shapes.push(("mixed", "basin", Shape::Path, path));
    assert_eq!(
        bind_code(&w, "= sdist('kind:basin', 20)", &base()),
        "formula.shape"
    );
    w.own = Some(Shape::Region { rings: 1 });
    let ctx = Context::new(Key::EffectValue).kind("basin");
    assert_eq!(bind_code(&w, "= smoothed_along(24)", &ctx), "formula.shape");
    w.own = Some(Shape::Path);
    assert_eq!(bind_code(&w, "= smoothed_along(24)", &ctx), "ok");
    // `'edge_min'` takes a region (8.5): on a path it is `formula.shape`, told at selection; on
    // a region it waits for milestone 2.
    assert_eq!(bind_code(&w, "= under('edge_min')", &ctx), "formula.shape");
    w.own = Some(Shape::Region { rings: 1 });
    assert_eq!(bind_code(&w, "= under('edge_min')", &ctx), "doc.not_yet");
}

// Values.

#[test]
fn case_25_sdist_equals_sd_within_cap() {
    let w = shaped();
    let mut env = env_for(&w);
    let got = run_in(&w, &mut env, "= sdist('pond', 20)", &base()).unwrap();
    let d = Domain::Window {
        grid: w.grid,
        window: Window::whole(&w.grid),
    };
    let want: Vec<f64> = d
        .points()
        .iter()
        .map(|&(x, y)| region(Measure::Sd, x, y))
        .collect();
    assert_eq!(got, want);
    let capped = run_in(&w, &mut env, "= sdist('pond', 1)", &base()).unwrap();
    assert!(capped.iter().all(|v| (-1.0..=1.0).contains(v)));
    let t = run_in(
        &w,
        &mut env,
        "= t",
        &Context::new(Key::EffectValue).kind("k"),
    )
    .unwrap();
    assert!(t.iter().all(|&v| v.to_bits() == 0));
}

#[test]
fn case_26_inside_on_the_outline() {
    let w = shaped();
    let mut env = env_for(&w);
    let got = run_in(&w, &mut env, "= if(inside('pond'), 1, 0)", &base()).unwrap();
    // Posts at x = 98, 99, 100 (the outline) are inside.
    assert_eq!(&got[..5], &[1.0, 1.0, 1.0, 0.0, 0.0]);
}

#[test]
fn case_27_noise_streams() {
    let w = small();
    let mut env = env_for(&w);
    let a = run_in(&w, &mut env, "= fbm(80, 3, seed = 'a')", &base()).unwrap();
    let other = Context::new(Key::FieldBase)
        .field("other")
        .noise(&["noise", "field", "other", "base"]);
    let b = run_in(&w, &mut env, "= fbm(80, 3, seed = 'a')", &other).unwrap();
    assert_eq!(a, b);
    let field = Context::new(Key::FieldBase).noise(&["noise", "field", "trees", "base"]);
    let scatter = Context::new(Key::Density).noise(&["noise", "scatter", "trees", "density"]);
    let f = run_in(&w, &mut env, "= fbm(80, 3) * 0.5 + 0.5", &field).unwrap();
    let s = run_in(&w, &mut env, "= fbm(80, 3) * 0.5 + 0.5", &scatter).unwrap();
    assert_ne!(f, s);
    // The default stream is the stream of the parts (formulas.md 8.2's stream vectors).
    let d = Domain::Points(&[(37_000, -11_000)]);
    let id = env.text("= fbm(13, 2)");
    let p = env.check(id, &base()).unwrap();
    let got = evaluate(
        &p,
        &bind(&p, &w).unwrap(),
        &w,
        &Setup {
            world_seed: 0,
            feature: None,
        },
        &d,
    )
    .unwrap();
    let want = via_expr::fbm(0x2B7A_1CEA_060E_DEB6, 37.0, -11.0, 13.0, 2, 0.5, 2.0).unwrap();
    assert_eq!(got[0].to_bits(), want.to_bits());
}

#[test]
fn case_28_seed_shared() {
    let mut env = Env::new();
    let one = env.text("= fbm(80, 3) + fbm(40, 2)");
    let p = env.check(one, &base()).unwrap();
    let warn = seed_shared(&[&p]);
    assert_eq!(warn.len(), 1);
    assert_eq!(warn[0].code_str(), "formula.seed_shared");
    let b = env.text("= fbm(80, 3)");
    let r = env.text("= v + fbm(80, 3)");
    let pb = env.check(b, &base()).unwrap();
    let then = Context::new(Key::ThenValue)
        .field("height")
        .noise(&["noise", "field", "height", "then", "r1", "value"]);
    let pr = env.check(r, &then).unwrap();
    assert!(seed_shared(&[&pb, &pr]).is_empty());
    // A `let` read twice is one noise call.
    let n = env.text("= fbm(80, 3)");
    env.world_let("n", n);
    let twice = env.text("= n + n * 2");
    let pt = env.check(twice, &base()).unwrap();
    assert!(seed_shared(&[&pt]).is_empty());
}

#[test]
fn case_30_degrees() {
    for (t, v) in [
        ("sin(180)", 0.0),
        ("cos(90)", 0.0),
        ("sin(360)", 0.0),
        ("sin(90)", 1.0),
        ("sin(450)", 1.0),
        ("cos(180)", -1.0),
        ("sin(-90)", -1.0),
    ] {
        let got = value(&format!("= {t}"));
        assert_eq!(got.to_bits(), f64::to_bits(v), "{t}");
    }
    let t = "= tan(90)";
    let e = fails(t);
    assert_eq!(e.code_str(), "formula.not_finite");
    assert_eq!(spanned(t, &e), "tan(90)");
}

#[test]
fn case_31_min_max() {
    assert_eq!(value("= min(3, 1, 2)"), 1.0);
    assert_eq!(value("= max(3, 1, 2)"), 3.0);
    let t = "= 1 / min(0, -0)";
    let e = fails(t);
    assert_eq!(e.code_str(), "formula.not_finite");
    assert_eq!(spanned(t, &e), "1 / min(0, -0)");
    assert!(
        e.message.starts_with("`/` gave -infinity of 1, -0"),
        "{}",
        e.message
    );
}

#[test]
fn case_32_slope_and_windows() {
    let mut w = World::new([0, 0, 6_000, 4_000], 1_000);
    w.fields.insert(
        "height",
        (|x, _| 0.5 * ((x as f64) / 1000.0), vec![], false),
    );
    let mut env = env_for(&w);
    let id = env.text("= slope('height')");
    let p = env
        .check(id, &Context::new(Key::FieldBase).field("other"))
        .unwrap();
    assert_eq!(p.halo_posts, 1);
    let b = bind(&p, &w).unwrap();
    let all = whole(&w, &p, &b).unwrap();
    let want = via_num::atan(0.5) * (180.0 / via_num::PI);
    assert!(all.iter().all(|v| v.to_bits() == want.to_bits()), "{all:?}");
    assert_windows_equal(&w, &p, &b);
}

/// Case 46 in small: every window of every size gives the same cut of the whole.
fn assert_windows_equal(w: &World, p: &Program, b: &Binding) {
    let g = w.grid;
    let all = whole(w, p, b).unwrap();
    for (tw, th) in [(1, 1), (2, 3), (3, 2), (g.nx, 1), (1, g.ny)] {
        for j0 in (0..g.ny).step_by(th as usize) {
            for i0 in (0..g.nx).step_by(tw as usize) {
                let win = Window {
                    i0,
                    j0,
                    i1: (i0 + tw).min(g.nx),
                    j1: (j0 + th).min(g.ny),
                };
                let d = Domain::Window {
                    grid: g,
                    window: win,
                };
                let got = evaluate(
                    p,
                    b,
                    w,
                    &Setup {
                        world_seed: 0,
                        feature: Some("p1"),
                    },
                    &d,
                )
                .unwrap();
                for (k, v) in got.iter().enumerate() {
                    let gi = d.order(k) as usize;
                    assert_eq!(v.to_bits(), all[gi].to_bits(), "window {win:?} post {gi}");
                }
            }
        }
    }
}

#[test]
fn case_33_v() {
    let mut w = small();
    w.v = 5.0;
    let mut env = env_for(&w);
    let ctx = Context::new(Key::EffectValue).kind("k");
    assert!(run_in(&w, &mut env, "= v + 1", &ctx)
        .unwrap()
        .iter()
        .all(|&x| x == 6.0));
}

#[test]
fn case_38_let_chain_is_shared() {
    let w = small();
    let mut env = env_for(&w);
    let first = env.text("= x * 0.5 + 1");
    env.world_let("l0", first);
    for k in 1..40 {
        let t = env.text(&format!("= l{} + l{} * 0.25", k - 1, k - 1));
        env.world_let(&format!("l{k}"), t);
    }
    let id = env.text("= l39");
    let p = env.check(id, &base()).unwrap();
    // Five nodes for `l0`, then four per `let` (two reads share one node): once per post each.
    assert_eq!(p.len(), 5 + 39 * 3);
    let b = bind(&p, &w).unwrap();
    let got = whole(&w, &p, &b).unwrap();
    let mut want: f64 = 98.0 * 0.5 + 1.0;
    for _ in 1..40 {
        want = want + want * 0.25;
    }
    assert_eq!(got[0].to_bits(), want.to_bits());
}

#[test]
fn case_39_member() {
    let mut w = small();
    w.members
        .insert(("west".into(), "width".into()), Scalar::Number(3.5));
    let mut env = env_for(&w);
    env.feature("west", Some("k"));
    env.feature_attribute("west", "width", AttrType::Number);
    let ctx = Context::new(Key::Attribute).feature("east");
    let id = env.text("= west.width + 0.5");
    let p = env.check(id, &ctx).unwrap();
    assert_eq!(p.rate(), via_expr::Rate::Constant);
    assert!(p
        .reads
        .members
        .contains(&("west".to_string(), "width".to_string())));
    assert!(whole(&w, &p, &bind(&p, &w).unwrap())
        .unwrap()
        .iter()
        .all(|&v| v == 4.0));
    // The worked example's typo.
    let t = "= west.widht + 0.5";
    let id = env.text(t);
    let e = env.check(id, &ctx).unwrap_err();
    assert_eq!(e.code_str(), "formula.unknown_name");
    assert_eq!(
        e.message,
        "`widht` is not an attribute of `west` (kind `k`)"
    );
    assert_eq!(spanned(t, &e), "widht");
    assert_eq!(e.data.get("nearest"), Some(&via_diag::Value::text("width")));
}

#[test]
fn case_40_points_outside_the_field() {
    let w = small();
    let mut env = env_for(&w);
    let id = env.text("= f('height')");
    let p = env.check(id, &Context::new(Key::Where)).unwrap();
    let b = bind(&p, &w).unwrap();
    let pts = [(99_500, 500), (200_000, 0)];
    let setup = Setup {
        world_seed: 0,
        feature: None,
    };
    let e = evaluate(&p, &b, &w, &setup, &Domain::Points(&pts)).unwrap_err();
    assert_eq!(e.error.code_str(), "formula.outside_field");
    assert_eq!(e.error.position, Some((200_000, 0)));
    let id = env.text("= if(x < 150, f('height'), 0)");
    let p = env.check(id, &Context::new(Key::FieldBase)).unwrap();
    assert!(evaluate(
        &p,
        &bind(&p, &w).unwrap(),
        &w,
        &setup,
        &Domain::Points(&pts)
    )
    .is_ok());
}

// Failure.

#[test]
fn case_41_first_post_in_post_order() {
    let t = "= sqrt(x - 100)";
    let e = fails(t);
    assert_eq!(e.code_str(), "formula.not_finite");
    assert_eq!(spanned(t, &e), "sqrt(x - 100)");
    assert_eq!(e.position, Some((98_000, 0)));
    assert_eq!(e.hint, "guard it: `sqrt(max(a, 0))`");
}

#[test]
fn case_42_branches_not_taken() {
    assert!(run("= if(x > 100, sqrt(x - 100), 0)").is_ok());
    assert!(run("= if(x > 50, if(x > 100, sqrt(x - 100), 0), 0)").is_ok());
    assert!(
        run("= if(x < 100, 0, 1 / (x - 100))").is_err(),
        "x = 100 takes the division"
    );
    assert!(run("= if(x <= 100, 0, 1 / (x - 100))").is_ok());
}

#[test]
fn case_43_per_operation() {
    for (t, at) in [
        ("= if(sqrt(x - 100) > 1, 1, 0)", "sqrt(x - 100)"),
        ("= 1 / exp(1000)", "exp(1000)"),
        ("= if(x > 1 or sqrt(x - 100) > 0, 1, 0)", "sqrt(x - 100)"),
    ] {
        let e = fails(t);
        assert_eq!(e.code_str(), "formula.not_finite", "{t}");
        assert_eq!(spanned(t, &e), at, "{t}");
    }
}

#[test]
fn case_44_ranges() {
    let w = small();
    let mut env = env_for(&w);
    let density = Context::new(Key::Density).noise(&["noise", "scatter", "s", "density"]);
    let e = run_in(&w, &mut env, "= 1.2", &density).unwrap_err().error;
    assert_eq!(
        (e.code_str(), e.hint.as_str()),
        ("formula.range", "write `clamp(..., 0, 1)`")
    );
    let mask = Context::new(Key::ThenMask).field("height");
    assert_eq!(
        run_in(&w, &mut env, "= 1.5", &mask)
            .unwrap_err()
            .error
            .code_str(),
        "formula.range"
    );
    let mc = Context::new(Key::MaxChange).kind("k");
    assert_eq!(
        run_in(&w, &mut env, "= -1", &mc)
            .unwrap_err()
            .error
            .code_str(),
        "formula.range"
    );
    for t in ["= clamp(x, 1, 0)", "= smoothstep(1, 1, x)"] {
        assert_eq!(fails(t).code_str(), "formula.range", "{t}");
    }
    assert!(
        run("= if(x > 1000, clamp(x, 1, 0), 0)").is_ok(),
        "liveness governs ranges too"
    );
}

#[test]
fn case_45_failure_is_the_same_by_window() {
    let w = small();
    let mut env = env_for(&w);
    for t in [
        "= sqrt(x - 100)",
        "= if(y > 0, 1 / (x - 101), sqrt(x - 99))",
    ] {
        let id = env.text(t);
        let p = env.check(id, &base()).unwrap();
        let b = bind(&p, &w).unwrap();
        let all = whole(&w, &p, &b).unwrap_err();
        let g = w.grid;
        let mut least: Option<Failure> = None;
        for j0 in 0..g.ny {
            for i0 in 0..g.nx {
                let d = Domain::Window {
                    grid: g,
                    window: Window {
                        i0,
                        j0,
                        i1: i0 + 1,
                        j1: j0 + 1,
                    },
                };
                if let Err(f) = evaluate(
                    &p,
                    &b,
                    &w,
                    &Setup {
                        world_seed: 0,
                        feature: None,
                    },
                    &d,
                ) {
                    if least.as_ref().is_none_or(|l| f.order < l.order) {
                        least = Some(*f);
                    }
                }
            }
        }
        let least = least.expect("a failure");
        assert_eq!(least.order, all.order, "{t}");
        assert_eq!(least.error, all.error, "{t}");
    }
}

#[test]
fn case_47_and_50_failures_inside_a_let() {
    let mut w = small();
    w.attrs.insert("width".into(), Scalar::Number(3.0));
    let mut env = env_for(&w);
    env.kind_attribute("k", "width", AttrType::Number);
    let lt = "= sqrt(width - 10)";
    let l = env.text(lt);
    env.kind_let("k", "w2", l);
    let ctx = Context::new(Key::EffectValue).kind("k");
    let reader = "= 1 + w2";
    let e = run_in(&w, &mut env, reader, &ctx).unwrap_err().error;
    assert_eq!(e.code_str(), "formula.not_finite");
    assert_eq!(
        e.at,
        via_expr::At::Expr {
            formula: l,
            span: 2..lt.len() as u32
        }
    );
    assert_eq!(e.related.len(), 1);
    assert!(matches!(&e.related[0].0, via_expr::At::Expr { span, .. } if *span == (6..8)));
    assert_ne!(e.subject, Some(l), "the subject is the reading formula");
    // Case 47: a world let read by two formulas: one location, the let's.
    let mut env = env_for(&w);
    let bad = env.text("= sqrt(-1)");
    env.world_let("bad", bad);
    for t in ["= bad", "= 2 * bad"] {
        let e = run_in(&w, &mut env, t, &base()).unwrap_err().error;
        assert!(
            matches!(e.at, via_expr::At::Expr { formula, .. } if formula == bad),
            "{t}"
        );
    }
    // A let cycle.
    let a = env.text("= b2 + 1");
    let b2 = env.text("= a2 * 2");
    env.world_let("a2", a);
    env.world_let("b2", b2);
    assert_eq!(code(env.check(a, &base())), "graph.cycle");
    let errors = env.check_lets();
    assert!(errors.iter().any(|e| e.code_str() == "graph.cycle"));
}

#[test]
fn per_feature_read_failures_follow_liveness() {
    let w = small();
    let mut env = env_for(&w);
    let ctx = Context::new(Key::EffectValue).kind("k");
    let e = run_in(&w, &mut env, "= under('mean')", &ctx)
        .unwrap_err()
        .error;
    assert_eq!(e.code_str(), "num.empty_reduction");
    assert!(run_in(&w, &mut env, "= if(x > 1000, under('mean'), 0)", &ctx).is_ok());
}

#[test]
fn case_52_noise_vectors() {
    let w = small();
    let mut env = env_for(&w);
    let id = env.text("= fbm(1, 1, seed = 'hills')");
    let p = env.check(id, &base()).unwrap();
    let b = bind(&p, &w).unwrap();
    let pts = [(500, 500), (250, 750), (-500, 500), (3_000, 2_000)];
    let got = evaluate(
        &p,
        &b,
        &w,
        &Setup {
            world_seed: 0,
            feature: None,
        },
        &Domain::Points(&pts),
    )
    .unwrap();
    assert_eq!(got[0], 0.375);
    assert_eq!(got[1].to_bits(), 0x3FAB_DF20_0000_0000);
    assert_eq!(got[2], -0.25);
    assert_eq!(got[3].to_bits(), 0);
}

#[test]
fn diagnostic_from_an_error() {
    use via_diag::{KeyPath, Site, Subject};
    let t = "= sqrt(x - 100)";
    let e = fails(t);
    let d = e.to_diag(
        &|_, _| Subject::entity("field", "height").with_keys(&KeyPath::key("base")),
        &|at| match at {
            via_expr::At::Expr { span, .. } => Site::Expr(KeyPath::key("base"), span.clone()),
            _ => Site::Header,
        },
    );
    assert_eq!(d.code().to_string(), "formula.not_finite");
    assert_eq!(d.position().map(|p| p.x_mm()), Some(98_000));
    assert!(d.undeclared_data().is_empty());
}
