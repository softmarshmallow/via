//! `via-terrain`: the terrain library's step program (`lib/terrain`).
//!
//! `when = "generate"`: settles the land under the stated facts and returns the settled ground,
//! standing water, wetness, silt and the courses the water took. `when = "lint"`: checks each fact
//! against the finished build, saying where and what would hold instead (R14, `derived.md` 5).
//!
//! With `VIA_STEP` set it runs once as a plain command (`protocol.md` 2.2); otherwise it is a
//! worker on the protocol's descriptors (3). A world names it as a configured program:
//! `via config program via-terrain <path>`, then `command = ["via-terrain"]`.

use std::path::Path;

use serde_json::{json, Value};
use via_proto::store::Store;
use via_step::{serve, take_stdio, Step, StepError, StepFn};
use via_terrain::facts::{self, Build, Finding};
use via_terrain::wire::{self, Fault, Feature, Targets};

fn broken(m: impl std::fmt::Display) -> StepError {
    StepError::Raised {
        exception: "TerrainError".into(),
        message: m.to_string(),
    }
}

/// What a call read, from either transport.
struct Read {
    surface: (String, Vec<f64>),
    water: Option<(String, Vec<f64>)>,
    /// The ground the generate step started from (its field tagged `settled_from`), for a lint.
    start: Option<Vec<f64>>,
    features: Vec<Feature>,
}

/// The lint over what it read.
fn lint(request: &Value, r: &Read) -> Result<Vec<Finding>, Fault> {
    let posts = wire::posts(request)?;
    let limits = wire::limits(&request["params"])?;
    let (steps, names) = match wire::checked(request) {
        Some((s, n)) => (Some(s), Some(n)),
        None => (None, None),
    };
    let stated: Vec<Feature> = r
        .features
        .iter()
        .filter(|f| names.as_ref().is_none_or(|n| n.contains(&f.name)))
        .cloned()
        .collect();
    let lakes = wire::lakes(&stated)?;
    let rivers = wire::rivers(&stated)?;
    let holds = wire::holds(&stated);
    let mut found = Vec::new();
    match &steps {
        Some(steps) => {
            for s in steps {
                found.extend(wire::found(&r.features, Some(s)));
            }
        }
        None => found = wire::found(&r.features, None),
    }
    let b = Build {
        posts,
        ground: &r.surface.1,
        water: r.water.as_ref().map(|w| w.1.as_slice()),
        lakes: &lakes,
        rivers: &rivers,
        holds: &holds,
        start: r.start.as_deref(),
        found: &found,
    };
    Ok(facts::check(&b, &limits))
}

// ---- the worker

struct Terrain;

fn field_answer(
    answer: &Value,
    posts: via_terrain::settle::Posts,
) -> Result<(String, Vec<f64>), StepError> {
    let (name, f) = answer
        .as_object()
        .and_then(|o| o.iter().next())
        .ok_or_else(|| broken("build.field answered no field"))?;
    let path = f["path"]
        .as_str()
        .ok_or_else(|| broken("build.field answered no path"))?;
    let v = wire::field_values(Path::new(path), posts).map_err(broken)?;
    // The answer is keyed by the entry as asked (a `role:` selector); the field's own name is in
    // it (`protocol.md` 3.3, `build.field`).
    let name = f["name"]
        .as_str()
        .map_or_else(|| name.clone(), str::to_string);
    Ok((name, v))
}

fn worker_read(step: &mut Step<'_>, generate: bool) -> Result<Read, StepError> {
    let request = step.request.clone();
    let posts = wire::posts(&request).map_err(broken)?;
    let paths = wire::paths(&request).map_err(broken)?;
    let store = Store::open(&paths.objects).map_err(broken)?;
    // A generate step returns a row on the surface, so it reads the head view (authority.md 4.4);
    // a lint reads the finished build.
    let view = if generate { "head" } else { "full" };
    let a = step.engine(
        "build.field",
        json!({"fields": ["role:surface"], "view": view}),
    )?;
    let surface = field_answer(&a, posts)?;
    // A world without a field tagged `water_level` answers an error here: then no row on water.
    let water = match step.engine(
        "build.field",
        json!({"fields": ["tag:water_level"], "view": view}),
    ) {
        Ok(a) => Some(field_answer(&a, posts)?),
        Err(StepError::Cancelled) => return Err(StepError::Cancelled),
        Err(_) => None,
    };
    // A lint checks a hold against the ground the step started from (R14).
    let start = if generate {
        None
    } else {
        match step.engine(
            "build.field",
            json!({"fields": ["tag:settled_from"], "view": "full"}),
        ) {
            Ok(a) => Some(field_answer(&a, posts)?.1),
            Err(StepError::Cancelled) => return Err(StepError::Cancelled),
            Err(_) => None,
        }
    };
    let mut features = Vec::new();
    let mut read = |select: Value, step: &mut Step<'_>| -> Result<(), StepError> {
        let list = step.engine("build.features", json!({"select": select, "columns": "*"}))?;
        features.extend(wire::features(&list, &store).map_err(broken)?);
        Ok(())
    };
    if generate {
        for k in [wire::LAKE, wire::RIVER, wire::HOLD] {
            read(json!(format!("kind:{k}")), step)?;
        }
    } else {
        match wire::checked(&request) {
            Some((_, names)) => {
                if !names.is_empty() {
                    read(json!(names), step)?;
                }
            }
            None => {
                for k in [wire::LAKE, wire::RIVER] {
                    read(json!(format!("kind:{k}")), step)?;
                }
            }
        }
        read(json!(format!("kind:{}", wire::COURSE)), step)?;
    }
    Ok(Read {
        surface,
        water,
        start,
        features,
    })
}

/// Sends findings through `diag.emit`, one batch (R23).
fn emit(step: &mut Step<'_>, findings: &[Finding]) -> Result<(), StepError> {
    if findings.is_empty() {
        return Ok(());
    }
    let mut seen = std::collections::BTreeSet::new();
    let s = |f: fn(&Finding) -> String| -> Value {
        json!({"dtype": "str", "values": findings.iter().map(f).collect::<Vec<_>>()})
    };
    let explain: Vec<&str> = findings
        .iter()
        .map(|f| {
            if seen.insert(f.code) {
                facts::explain(f.code)
            } else {
                ""
            }
        })
        .collect();
    let mut params = json!({
        "severity": s(|f| f.severity.to_string()),
        "code": s(|f| f.code.to_string()),
        "subject": s(|f| f.subject.clone()),
        "message": s(|f| f.message.clone()),
        "hint": s(|f| f.hint.clone()),
        "data": s(|f| f.data.clone()),
        "explain": {"dtype": "str", "values": explain},
    });
    if findings.iter().all(|f| f.position.is_some()) {
        let xy: Vec<f64> = findings
            .iter()
            .flat_map(|f| {
                let p = f.position.unwrap_or((0.0, 0.0));
                [p.0, p.1]
            })
            .collect();
        params["position"] = json!({"dtype": "f64", "shape": [findings.len(), 2], "values": xy});
    }
    step.engine("diag.emit", params)?;
    Ok(())
}

/// Findings with and without a position go in two batches: a column holds every row or none.
fn emit_all(step: &mut Step<'_>, findings: &[Finding]) -> Result<(), StepError> {
    let (with, without): (Vec<Finding>, Vec<Finding>) =
        findings.iter().cloned().partition(|f| f.position.is_some());
    emit(step, &with)?;
    emit(step, &without)
}

impl StepFn for Terrain {
    fn when(&self) -> &'static [&'static str] {
        &["generate", "lint"]
    }

    fn call(&mut self, step: &mut Step<'_>) -> Result<Option<String>, StepError> {
        let request = step.request.clone();
        let generate = request["when"] == "generate";
        let r = worker_read(step, generate)?;
        if generate {
            let posts = wire::posts(&request).map_err(broken)?;
            let paths = wire::paths(&request).map_err(broken)?;
            let seed = wire::seed(&request).map_err(broken)?;
            let targets = Targets {
                surface: r.surface.0.clone(),
                water: r.water.as_ref().map(|w| w.0.clone()),
                order: request["params"]["order"].as_i64(),
            };
            match wire::generate(
                posts,
                r.surface.1,
                &r.features,
                &request["params"],
                seed,
                &targets,
                &paths.out,
            ) {
                Ok((doc, warnings)) => {
                    emit_all(step, &warnings)?;
                    Ok(Some(doc))
                }
                Err(Fault::Said(f)) => {
                    emit_all(step, &[*f])?;
                    Ok(Some(String::new()))
                }
                Err(Fault::Broken(m)) => Err(broken(m)),
            }
        } else {
            match lint(&request, &r) {
                Ok(findings) => emit_all(step, &findings)?,
                Err(Fault::Said(f)) => emit_all(step, &[*f])?,
                Err(Fault::Broken(m)) => return Err(broken(m)),
            }
            Ok(None)
        }
    }
}

// ---- the plain command

fn plain(request_path: &Path) -> Result<(), String> {
    let text = std::fs::read(request_path)
        .map_err(|e| format!("cannot read {}: {e}", request_path.display()))?;
    let request: Value =
        serde_json::from_slice(&text).map_err(|e| format!("the request is not JSON: {e}"))?;
    let posts = wire::posts(&request)?;
    let paths = wire::paths(&request)?;
    let store = Store::open(&paths.objects).map_err(|e| e.to_string())?;
    let data = &request["data"];
    let mut surface = None;
    let mut water = None;
    let mut start = None;
    for (name, f) in data["fields"].as_object().into_iter().flatten() {
        let path = || {
            f["path"]
                .as_str()
                .map(Path::new)
                .ok_or("a field has no path")
        };
        if f["role"] == "surface" {
            surface = Some((name.clone(), wire::field_values(path()?, posts)?));
        }
        let tagged = f["tags"]
            .as_array()
            .is_some_and(|t| t.iter().any(|x| x == "water_level"));
        if tagged && water.is_none() {
            water = Some((name.clone(), wire::field_values(path()?, posts)?));
        }
        let from = f["tags"]
            .as_array()
            .is_some_and(|t| t.iter().any(|x| x == "settled_from"));
        if from && start.is_none() {
            start = Some(wire::field_values(path()?, posts)?);
        }
    }
    let surface = surface.ok_or("no field in the step's view has the role `surface`")?;
    let features = wire::features(&data["features"], &store)?;
    let r = Read {
        surface,
        water,
        start,
        features,
    };
    let mut findings = Vec::new();
    if request["when"] == "generate" {
        let targets = Targets {
            surface: r.surface.0.clone(),
            water: r.water.as_ref().map(|w| w.0.clone()),
            order: request["params"]["order"].as_i64(),
        };
        let doc = match wire::generate(
            posts,
            r.surface.1,
            &r.features,
            &request["params"],
            wire::seed(&request)?,
            &targets,
            &paths.out,
        ) {
            Ok((doc, warnings)) => {
                findings.extend(warnings);
                doc
            }
            Err(Fault::Said(f)) => {
                findings.push(*f);
                String::new()
            }
            Err(Fault::Broken(m)) => return Err(m),
        };
        std::fs::write(paths.out.join("document.toml"), doc).map_err(|e| e.to_string())?;
    } else {
        match lint(&request, &r) {
            Ok(f) => findings = f,
            Err(Fault::Said(f)) => findings.push(*f),
            Err(Fault::Broken(m)) => return Err(m),
        }
    }
    if !findings.is_empty() {
        std::fs::write(
            paths.out.join("diagnostics.json"),
            wire::diagnostics_json(&findings),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn main() {
    if let Some(p) = std::env::var_os("VIA_STEP") {
        if let Err(m) = plain(Path::new(&p)) {
            eprintln!("via-terrain: {m}");
            std::process::exit(1);
        }
        return;
    }
    let mut conn = match take_stdio() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("via-terrain: cannot take the protocol descriptors: {e}");
            std::process::exit(1);
        }
    };
    std::process::exit(serve(&mut conn, &mut Terrain));
}
