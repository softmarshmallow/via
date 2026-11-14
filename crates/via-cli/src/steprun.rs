//! One call of a worker step (`protocol.md` 3): the request without `data`, the call with its
//! calls back answered from the step's view, and what it returned, checked (5).

use std::ffi::OsString;
use std::path::PathBuf;

use serde_json::Value;
use via_diag::{codes, Diag};
use via_proto::diag::StepSite;
use via_proto::worker::{Deadlines, Outcome, WorkerSpec};

use crate::json::{obj, J};
use crate::steps::{
    resolve_program, sdk_folder, slash, step_diag, write_failed, Call, Code, Mapper, Ran, StepDecl,
};
use crate::stepview::View;
use crate::stepwork::{self, Svc};

/// Runs one call of a trusted worker step.
pub fn run_worker(s: &StepDecl, code: &Code, call: &Call<'_>) -> Ran {
    let mut ran = Ran::new();
    let Some(grid) = call.build.grid else {
        return ran;
    };
    let Some(pool) = call.pool else {
        return ran;
    };
    let program = match s.command.first() {
        Some(p) => match resolve_program(p, s, call) {
            Ok(p) => p,
            Err(d) => {
                ran.diags.push(d);
                return ran;
            }
        },
        None => {
            ran.diags.push(step_diag(
                &s.id,
                codes::protocol::START,
                format!("step `{}` could not be started: its `command` is empty", s.id),
                "write the program and its arguments, `command = [\"python\", \"-m\", \"viasdk\", \"steps/s.py\"]`".into(),
            ).with("kind", "empty"));
            return ran;
        }
    };
    let objects_dir = call.store.root().join("objects");
    let objects = match via_proto::store::Store::open(&objects_dir) {
        Ok(o) => o,
        Err(e) => {
            ran.diags.push(write_failed(&s.id, &objects_dir, &e));
            return ran;
        }
    };
    // The call folder (6.3): `out` alone; a worker gets its request on the wire.
    let nonce = crate::steps::call_nonce();
    let folder = call
        .store
        .root()
        .join("calls")
        .join(format!("{}-{nonce}", s.id));
    let _live = crate::interrupt::Live::enter(folder.clone());
    let out = folder.join("out");
    if let Err(e) = std::fs::create_dir_all(&out) {
        ran.diags.push(write_failed(&s.id, &out, &e));
        let _ = std::fs::remove_dir_all(&folder);
        return ran;
    }
    for i in &s.inputs {
        if crate::steps::input_digest(&s.home.join(i)).is_none() {
            ran.diags.push(
                step_diag(
                    &s.id,
                    codes::graph::INPUT_MISSING,
                    format!("step `{}`: the input `{i}` does not exist", s.id),
                    "create it, or remove it from `inputs`".into(),
                )
                .with("path", i.clone()),
            );
        }
    }
    if !ran.diags.is_empty() {
        let _ = std::fs::remove_dir_all(&folder);
        return ran;
    }
    let request =
        crate::steps::request_for_worker(s, call, &grid, slash(&objects_dir), slash(&out));
    let request: Value = serde_json::from_slice(&request.file()).unwrap_or(Value::Null);
    let sdk = match sdk_folder(call.store, call.sdk) {
        Ok(p) => p,
        Err((path, e)) => {
            ran.diags.push(write_failed(&s.id, &path, &e));
            let _ = std::fs::remove_dir_all(&folder);
            return ran;
        }
    };
    let args: Vec<OsString> = s
        .command
        .iter()
        .skip(1)
        .map(|a| match a.strip_prefix("lib:") {
            Some(rest) => {
                let (lib, path) = rest.split_once('/').unwrap_or((rest, ""));
                call.roots
                    .library(lib)
                    .map(|d| d.join(path))
                    .unwrap_or_default()
                    .into_os_string()
            }
            None => OsString::from(a),
        })
        .collect();
    let cwd: PathBuf = code
        .folder
        .clone()
        .filter(|_| s.command.iter().any(|c| c.starts_with("lib:")))
        .unwrap_or_else(|| s.home.clone());
    let start = || {
        stepwork_start(WorkerSpec {
            program: program.clone(),
            args: args.clone(),
            cwd: cwd.clone(),
            sdk: Some(sdk.clone()),
            objects: objects_dir.clone(),
            site: StepSite {
                id: s.id.clone(),
                file: String::new(),
                line: 1,
                column: 1,
                bytes: [0, 0],
            },
            when: s.when.clone(),
            methods: stepwork::METHODS.iter().map(|m| (*m).to_string()).collect(),
            deadlines: Deadlines {
                call: call.deadline,
                ..Deadlines::default()
            },
        })
    };
    let view = View {
        world: call.world,
        build: call.build,
        grid,
        objects: &objects,
        pack: call.pack,
        step: s,
        rank: call.rank(s),
        seed: via_num::world_seed(i64::try_from(call.seed).unwrap_or(0)).unwrap_or(0),
        stratum: call.stratum,
        bad: call.bad,
    };
    let mapper = Mapper {
        world: call.world,
        grid: call.build.grid,
    };
    let mut svc = Svc::new(&view, call.store, &mapper);
    let slot = pool.slot(&s.id);
    let (outcome, log) = stepwork::call(&slot, &start, request, &mut svc);
    ran.ran = true;
    let served = svc.served;
    if served.bad_read {
        // `diagnostics.md` 4: the call is cancelled and the lint skipped; what it found is
        // dropped, and no other fault of the call is reported.
        ran.skipped = true;
        ran.ran = false;
        ran.diags.clear();
        let _ = std::fs::remove_dir_all(&folder);
        let _ = outcome;
        let _ = log;
        return ran;
    }
    let refused = served.refused;
    ran.emitted.clone_from(&served.diags);
    ran.diags.extend(served.diags);
    ran.explains = served
        .explains
        .entries()
        .map(|(c, e)| (c.to_string(), e.map(str::to_string)))
        .collect();
    let log_digest = call.store.put_object(log.unwrap_or_default().as_bytes());
    let mut finished = false;
    let mut document: Option<String> = None;
    match outcome {
        Ok(Outcome::Done(v)) => {
            finished = true;
            match v.get("document") {
                Some(Value::String(t)) => document = Some(t.clone()),
                Some(Value::Object(o)) => {
                    let text = o
                        .get("blob")
                        .and_then(Value::as_str)
                        .and_then(|b| objects.map(b).ok())
                        .and_then(|m| String::from_utf8(m.to_vec()).ok());
                    match text {
                        Some(t) => document = Some(t),
                        None => {
                            finished = false;
                            ran.diags.push(step_diag(
                                &s.id,
                                codes::protocol::BLOB_DIGEST,
                                format!("step `{}` returned a document blob that cannot be read as named", s.id),
                                "return the document's text, or a blob named by its BLAKE3 digest".into(),
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(Outcome::Cancelled) => {}
        // A step that ends by the exception a refusal raised has one report, the engine's own
        // (3.3: one fault, one message).
        Err(d) if *d.code() == via_diag::DiagCode::Core(codes::protocol::RAISED) && refused => {
            let _ = d;
        }
        Err(d) => ran.diags.push(d),
    }
    let mut files: Vec<J> = Vec::new();
    if finished {
        let returned = crate::stepout::collect(
            &out,
            &s.id,
            &s.when,
            document,
            &grid,
            &crate::stepout::Names {
                world: call.world,
                rank: call.rank(s),
            },
            call.store,
        );
        ran.diags.extend(returned.diags.iter().cloned());
        // R21 with `authority.md` 4.4: a step that returns rows on a field reads its head.
        for f in returned.row_fields.intersection(&served.full) {
            ran.diags.push(step_diag(
                &s.id,
                codes::authority::STEP_VIEW,
                format!(
                    "step `{}` read `{f}` in its full view and returned a row on it, so it would read its own effects back",
                    s.id
                ),
                format!("read `{f}` with `view = \"head\"`"),
            ));
        }
        if s.when == "generate" {
            ran.rows_or_features = returned.returns_rows_or_features();
            ran.keys = returned.keys.clone();
            ran.unjoined = returned.ok && returned.has_tables();
            ran.document = returned.document.clone();
            let mut facts: Vec<String> = served.facts.into_iter().collect();
            facts.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
            ran.facts = facts;
        }
        files = returned
            .files
            .iter()
            .map(|(p, d)| obj([("digest", J::s(d.hex())), ("path", J::s(p.clone()))]))
            .collect();
        ran.complete = returned.ok;
    }
    ran.reads = served.reads;
    ran.record = obj([
        ("command", J::strs(s.command.iter().map(String::as_str))),
        ("exit", J::Null),
        ("id", J::s(format!("step:{}", s.id))),
        ("log", J::s(log_digest.hex())),
        ("returned", J::Arr(files)),
        ("trust", J::s(code.trust.hex())),
    ]);
    let _ = std::fs::remove_dir_all(&folder);
    ran
}

fn stepwork_start(spec: WorkerSpec) -> Result<via_proto::worker::Worker, Diag> {
    via_proto::worker::Worker::start(spec)
}
