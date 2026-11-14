//! Running a step as a worker (`protocol.md` 3): one process per step per build, serving every
//! call of the step, answering its calls back from the view, recording what it read.
//!
//! Each traced call is recorded as its normalised request, kept in the store by digest so a
//! later build can answer it again without the step ([`replay`]), and the digest of its answer
//! (`graph.md` 5.1). A compute that reads a field is also recorded as a `build.field` read of
//! that field's whole grid in its view (`derived.md` 4), so a changed field runs the step again
//! even when the compute's answer would not change.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use via_diag::step::{Explains, StepDiagIn, StepRelatedIn};
use via_diag::{codes, Diag, Site, Subject};
use via_graph::{Digest, Read, Store};
use via_proto::column::{self, Column};
use via_proto::worker::{CancelToken, Outcome, RpcError, Services, Worker};

use crate::stepview::{self, View};

/// The worker methods offered in `initialize` (3.2).
pub const METHODS: [&str; 12] = [
    "build.field",
    "build.features",
    "build.instances",
    "input.open",
    "pack.query",
    "expr.eval",
    "derive.where",
    "derive.route",
    "derive.descend",
    "derive.spill",
    "surface.meetings",
    "diag.emit",
];

/// The running workers of a build, one per step, started at a step's first call and shut down
/// when the build ends (3.2: workers do not outlive a build).
#[derive(Default)]
pub struct Pool {
    slots: Mutex<BTreeMap<String, Arc<Mutex<Option<Worker>>>>>,
}

impl Pool {
    /// The slot of a step's worker; one call at a time holds it.
    pub fn slot(&self, step: &str) -> Arc<Mutex<Option<Worker>>> {
        self.slots
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry(step.to_string())
            .or_default()
            .clone()
    }
}

impl Pool {
    /// Shuts every worker down (`protocol.md` 6.2), in step order: what a worker killed past the
    /// deadline gave, `protocol.slow_exit`, a run diagnostic (it is about this run's timing, so
    /// it is never recorded or replayed, `graph.md` 3.4).
    pub fn stop(&self) -> Vec<Diag> {
        let slots = std::mem::take(
            &mut *self
                .slots
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
        let mut out = Vec::new();
        for (_, s) in slots {
            if let Some(w) = s
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take()
            {
                out.extend(w.shutdown());
            }
        }
        out
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// What the calls back of one `step.call` gave.
pub struct Served {
    pub reads: Vec<Read>,
    /// Stated facts named in the answers of its `build.features` reads (`derived.md` 5).
    pub facts: BTreeSet<String>,
    /// Fields it read in their full view (`authority.step_view`).
    pub full: BTreeSet<String>,
    /// Diagnostics: the engine's own refusals, and what the step sent with `diag.emit`.
    pub diags: Vec<Diag>,
    pub explains: Explains,
    /// Whether the engine refused a call with a diagnostic of its own.
    pub refused: bool,
    /// Whether a read reached a failed or skipped node (`diagnostics.md` 4).
    pub bad_read: bool,
}

/// The engine's side of one `step.call`'s calls back.
pub struct Svc<'v, 'a> {
    pub view: &'v View<'a>,
    pub store: &'v Store,
    pub mapper: &'v dyn via_diag::step::StepWorld,
    pub served: Served,
}

impl<'v, 'a> Svc<'v, 'a> {
    pub fn new(
        view: &'v View<'a>,
        store: &'v Store,
        mapper: &'v dyn via_diag::step::StepWorld,
    ) -> Self {
        let mut explains = Explains::new();
        explains.begin_call();
        Svc {
            view,
            store,
            mapper,
            served: Served {
                reads: Vec::new(),
                facts: BTreeSet::new(),
                full: BTreeSet::new(),
                diags: Vec::new(),
                explains,
                refused: false,
                bad_read: false,
            },
        }
    }

    fn record(&mut self, method: &str, params: &Value, answer: &Value) {
        let req = stepview::normalise(method, params, &self.view.grid);
        let bytes = serde_json::to_vec(&req).unwrap_or_default();
        let request = self.store.put_object(&bytes);
        self.served.reads.push(Read {
            method: method.to_string(),
            request,
            answer: stepview::answer_digest(answer),
        });
    }

    /// The fields a compute reads, with its view.
    fn compute_fields(&self, method: &str, params: &Value) -> Vec<String> {
        match method {
            "derive.descend" | "derive.spill" => params
                .get("field")
                .and_then(Value::as_str)
                .and_then(|s| self.view.field_values(s, "full").ok().map(|(n, _)| vec![n]))
                .unwrap_or_default(),
            "expr.eval" | "derive.route" | "derive.where" => {
                let mut texts: Vec<String> = Vec::new();
                for k in ["formula", "cost"] {
                    if let Some(s) = params.get(k).and_then(Value::as_str) {
                        texts.push(s.to_string());
                    }
                }
                if let Some(f) = params.get("formulas") {
                    if let Ok(Column::Str(v)) = column::read("formulas", f, self.view.objects) {
                        texts.extend(v);
                    }
                }
                let mut out: BTreeSet<String> = BTreeSet::new();
                for t in texts {
                    out.extend(self.view.formula_fields(&t));
                }
                out.into_iter().collect()
            }
            _ => Vec::new(),
        }
    }

    fn emit(&mut self, params: &Value) -> Result<Value, RpcError> {
        let bad = |m: String| RpcError {
            code: -32602,
            message: m,
            data: None,
        };
        let objects = self.view.objects;
        let strs = |k: &str| -> Result<Option<Vec<String>>, RpcError> {
            match params.get(k) {
                None | Some(Value::Null) => Ok(None),
                Some(v) => match column::read(k, v, objects) {
                    Ok(Column::Str(s)) => Ok(Some(s)),
                    Ok(_) => Err(bad(format!("{k}: a str column"))),
                    Err(e) => Err(RpcError {
                        code: e.code,
                        message: e.message,
                        data: None,
                    }),
                },
            }
        };
        let need = |k: &str| -> Result<Vec<String>, RpcError> {
            strs(k)?.ok_or_else(|| bad(format!("{k}: a str column")))
        };
        let severity = need("severity")?;
        let code = need("code")?;
        let subject = need("subject")?;
        let message = need("message")?;
        let hint = need("hint")?;
        let related = strs("related")?;
        let data = strs("data")?;
        let explain = strs("explain")?;
        let n = severity.len();
        if [code.len(), subject.len(), message.len(), hint.len()]
            .iter()
            .any(|l| *l != n)
        {
            return Err(bad("diag.emit: columns of one length".into()));
        }
        // A non-finite position replaces the record by `diag.malformed` (4.2), so it is read
        // without the column check that refuses it.
        let position: Option<Vec<f64>> = match params.get("position") {
            None | Some(Value::Null) => None,
            Some(v) => match v.get("values") {
                Some(vals) => {
                    let mut flat = Vec::new();
                    flatten(vals, &mut flat);
                    Some(flat)
                }
                None => column::read("position", v, objects)
                    .ok()
                    .and_then(|c| c.as_f64()),
            },
        };
        for k in 0..n {
            let parse = |s: Option<&String>| -> Option<Value> {
                s.filter(|t| !t.is_empty())
                    .and_then(|t| serde_json::from_str(t).ok())
            };
            let rel: Vec<StepRelatedIn> = parse(related.as_ref().and_then(|r| r.get(k)))
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default()
                .iter()
                .map(|r| StepRelatedIn {
                    subject: r.get("subject").and_then(Value::as_str).map(str::to_string),
                    position: r
                        .get("position")
                        .and_then(Value::as_array)
                        .map(|a| a.iter().map(|x| x.as_f64().unwrap_or(f64::NAN)).collect()),
                    note: r.get("note").and_then(Value::as_str).map(str::to_string),
                })
                .collect();
            let d = StepDiagIn {
                severity: Some(severity[k].clone()),
                code: Some(code[k].clone()),
                subject: Some(subject[k].clone()),
                message: Some(message[k].clone()),
                hint: Some(hint[k].clone()),
                position: position
                    .as_ref()
                    .and_then(|p| Some(vec![*p.get(2 * k)?, *p.get(2 * k + 1)?])),
                related: rel,
                data: parse(data.as_ref().and_then(|d| d.get(k))).map(|v| raw(&v)),
                explain: explain
                    .as_ref()
                    .and_then(|e| e.get(k))
                    .filter(|e| !e.is_empty())
                    .cloned(),
            };
            let step = &self.view.step.id;
            if let Some(extra) = self
                .served
                .explains
                .observe(step, &code[k], d.explain.as_deref())
            {
                self.served.diags.push(extra);
            }
            self.served
                .diags
                .push(via_diag::step::arrive(step, &d, self.mapper));
        }
        Ok(Value::Null)
    }
}

fn flatten(v: &Value, out: &mut Vec<f64>) {
    match v {
        Value::Array(a) => a.iter().for_each(|x| flatten(x, out)),
        x => out.push(x.as_f64().unwrap_or(f64::NAN)),
    }
}

fn raw(v: &Value) -> via_diag::step::RawValue {
    use via_diag::step::RawValue;
    match v {
        Value::Null => RawValue::Null,
        Value::Bool(b) => RawValue::Bool(*b),
        Value::Number(n) => RawValue::Number(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => RawValue::Text(s.clone()),
        Value::Array(a) => RawValue::List(a.iter().map(raw).collect()),
        Value::Object(m) => RawValue::Table(m.iter().map(|(k, v)| (k.clone(), raw(v))).collect()),
    }
}

impl Services for Svc<'_, '_> {
    fn handle(&mut self, method: &str, params: &Value) -> Result<Value, RpcError> {
        if method == "diag.emit" {
            return self.emit(params);
        }
        match self.view.answer(method, params) {
            Ok(answer) => {
                if stepview::traced(method) {
                    self.record(method, params, &answer);
                    let view = params.get("view").and_then(Value::as_str).unwrap_or("full");
                    if method == "build.field" {
                        if view == "full" {
                            if let Some(m) = answer.as_object() {
                                for e in m.values() {
                                    if let Some(n) = e.get("name").and_then(Value::as_str) {
                                        self.served.full.insert(n.to_string());
                                    }
                                }
                            }
                        }
                    } else if method != "build.features"
                        && method != "build.instances"
                        && method != "input.open"
                        && method != "pack.query"
                    {
                        for f in self.compute_fields(method, params) {
                            let p = json!({"fields": [f.clone()], "view": view});
                            if let Ok(a) = self.view.answer("build.field", &p) {
                                self.record("build.field", &p, &a);
                            }
                            if view == "full" {
                                self.served.full.insert(f);
                            }
                        }
                    }
                    if method == "build.features" {
                        if let Some(Ok(Column::Str(names))) = answer
                            .get("name")
                            .map(|c| column::read("name", c, self.view.objects))
                        {
                            self.served.facts.extend(self.view.stated(&names));
                        }
                    }
                }
                Ok(answer)
            }
            Err(r) => {
                if r.code == stepview::BAD_READ {
                    self.served.bad_read = true;
                } else if stepview::traced(method) {
                    // A refusal is an answer the step read (`graph.md` 5.1): a step that
                    // catches it and goes on depends on the call still being refused.
                    let req = stepview::normalise(method, params, &self.view.grid);
                    let bytes = serde_json::to_vec(&req).unwrap_or_default();
                    let request = self.store.put_object(&bytes);
                    self.served.reads.push(Read {
                        method: method.to_string(),
                        request,
                        answer: refusal_digest(r.code, &r.message, r.data.as_ref()),
                    });
                }
                if let Some(d) = r.diag {
                    self.served.diags.push(d);
                    self.served.refused = true;
                }
                Err(RpcError {
                    code: r.code,
                    message: r.message,
                    data: r.data,
                })
            }
        }
    }
}

/// The answer digest of a refused traced call: its normalised refusal, the code and the message
/// and data (`graph.md` 5.1). A trace holding it stays valid only while the call is still refused
/// the same way.
pub fn refusal_digest(code: i64, message: &str, data: Option<&Value>) -> Digest {
    stepview::answer_digest(&json!({
        "refused": {"code": code, "message": path_free(message), "data": data.cloned().unwrap_or(Value::Null)}
    }))
}

/// A refusal's message with every absolute path left out (`graph.md` 5.1), so the answer a trace
/// records is the same on every machine: each run of text that starts with `/` (or a drive
/// letter and `:\`) after a space, quote or bracket, up to the next one, becomes `…`.
pub fn path_free(message: &str) -> String {
    let stop =
        |c: char| c.is_whitespace() || matches!(c, '`' | '\'' | '"' | '(' | ')' | '[' | ']' | ',');
    let chars: Vec<char> = message.chars().collect();
    let mut out = String::with_capacity(message.len());
    let mut k = 0;
    while k < chars.len() {
        let after_stop = k == 0 || stop(chars[k - 1]);
        let drive = chars.get(k).is_some_and(char::is_ascii_alphabetic)
            && chars.get(k + 1) == Some(&':')
            && matches!(chars.get(k + 2), Some('\\' | '/'));
        if after_stop && (chars[k] == '/' || drive) {
            while k < chars.len() && !stop(chars[k]) {
                k += 1;
            }
            out.push('…');
            continue;
        }
        out.push(chars[k]);
        k += 1;
    }
    out
}

/// Answers a recorded read again from the current view (`graph.md` 5.1): the digest of the
/// answer, the digest of the refusal when the view refuses it (as recorded), or `None` when the
/// request is gone from the store or the read reaches a failed node.
pub fn replay(view: &View<'_>, store: &Store, read: &Read) -> Option<Digest> {
    let bytes = store.get_object(&read.request)?;
    let req: Value = serde_json::from_slice(&bytes).ok()?;
    let method = req.get("method")?.as_str()?.to_string();
    match view.answer(&method, &req) {
        Ok(a) => Some(stepview::answer_digest(&a)),
        Err(r) if r.code == stepview::BAD_READ => None,
        Err(r) => Some(refusal_digest(r.code, &r.message, r.data.as_ref())),
    }
}

/// The diagnostic when a worker's diagnostic leaves the worker usable: an exception or a missing
/// entry (3.4). Any other failure killed it.
pub fn worker_survives(d: &Diag) -> bool {
    matches!(
        d.code(),
        via_diag::DiagCode::Core(c)
            if *c == codes::protocol::RAISED || *c == codes::protocol::NO_ENTRY
    )
}

/// Runs one `step.call` on the step's worker, starting it if needed.
#[allow(clippy::too_many_arguments)]
pub fn call(
    slot: &Mutex<Option<Worker>>,
    start: &dyn Fn() -> Result<Worker, Diag>,
    request: Value,
    svc: &mut Svc<'_, '_>,
) -> (Result<Outcome, Diag>, Option<String>) {
    let mut guard = slot
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if guard.is_none() {
        match start() {
            Ok(w) => *guard = Some(w),
            Err(d) => return (Err(d), None),
        }
    }
    let Some(w) = guard.as_mut() else {
        return (
            Err(Diag::new(
                codes::diag::INTERNAL,
                Subject::entity("step", svc.view.step.id.clone()),
                Site::Header,
                crate::txt("the step's worker is gone".into()),
                crate::txt("report this as a bug in via".into()),
            )),
            None,
        );
    };
    let cancel = CancelToken::default();
    let r = w.call(request, svc, &cancel);
    let log = Some(w.log());
    if let Err(d) = &r {
        if !worker_survives(d) {
            if let Some(mut dead) = guard.take() {
                dead.kill();
            }
        }
    }
    (r, log)
}

#[cfg(test)]
mod tests {
    use super::path_free;

    #[test]
    fn a_refusal_message_keeps_no_machine_path() {
        assert_eq!(
            path_free("blob `/Users/a/.via/objects/ab/abcd` cannot be mapped: No such file"),
            "blob `…` cannot be mapped: No such file"
        );
        assert_eq!(path_free("at C:\\via\\objects x"), "at … x");
        assert_eq!(path_free("a/b and 1/2 stay"), "a/b and 1/2 stay");
    }
}
