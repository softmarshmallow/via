//! The M0 probe's engine services and harness (`spec/protocol.md`, What arrives when; case 0).
//!
//! Probe-only: `build.field` serves whole grids from a table the harness fills, and `expr.eval` is
//! a stub that knows one formula. Both are replaced in M2 by the graph's reads and `via-expr`. What
//! the probe proves is the transport: a field out and back through the store, a call back answered
//! with a column, and a killed worker reported and cleaned up.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};
use via_diag::{text, Diag};

use crate::column::{self, Column};
use crate::diag::StepSite;
use crate::store::Store;
use crate::worker::{
    wire_path, CancelToken, Deadlines, Outcome, RpcError, Services, Worker, WorkerSpec,
};

/// The world frame's grid, as the request carries it (2).
#[derive(Clone, Copy, Debug)]
pub struct Grid {
    pub extent_mm: [i64; 4],
    pub resolution_mm: i64,
    pub nx: usize,
    pub ny: usize,
}

impl Grid {
    pub fn to_json(&self) -> Value {
        json!({"extent_mm": self.extent_mm, "resolution_mm": self.resolution_mm,
               "nx": self.nx, "ny": self.ny})
    }
}

/// A scalar field held in the store as a field file.
#[derive(Clone, Debug)]
pub struct ProbeField {
    pub blob: String,
}

/// The engine's answers to call backs, for the probe.
pub struct ProbeServices {
    pub store: Arc<Store>,
    pub grid: Grid,
    pub fields: BTreeMap<String, ProbeField>,
    /// Call backs answered, by method, for the report.
    pub answered: BTreeMap<String, u64>,
}

/// The worker-to-engine methods the probe offers in `initialize`.
pub const METHODS: &[&str] = &["build.field", "expr.eval"];

fn bad_param(param: &str, message: String) -> RpcError {
    RpcError {
        code: -32602,
        message,
        data: Some(json!({"param": param})),
    }
}

impl Services for ProbeServices {
    fn handle(&mut self, method: &str, params: &Value) -> Result<Value, RpcError> {
        let r = match method {
            "build.field" => self.build_field(params),
            "expr.eval" => self.expr_eval(params),
            _ => Err(RpcError {
                code: -32601,
                message: format!("unknown method {method}"),
                data: Some(json!({"method": method})),
            }),
        };
        *self.answered.entry(method.to_owned()).or_default() += 1;
        r
    }
}

impl ProbeServices {
    fn build_field(&mut self, p: &Value) -> Result<Value, RpcError> {
        let names = p
            .get("fields")
            .and_then(Value::as_array)
            .ok_or_else(|| bad_param("fields", "fields must be a list of names".into()))?;
        if let Some(view) = p.get("view") {
            if !matches!(view.as_str(), Some("full" | "head")) {
                return Err(bad_param("view", "view is full or head".into()));
            }
        }
        if p.get("window").is_some_and(|w| !w.is_null()) {
            return Err(bad_param(
                "window",
                "windows arrive with M2; the probe serves whole grids".into(),
            ));
        }
        let mut out = serde_json::Map::new();
        for n in names {
            let n = n
                .as_str()
                .ok_or_else(|| bad_param("fields", "a field name is a string".into()))?;
            let f = self
                .fields
                .get(n)
                .ok_or_else(|| bad_param("fields", format!("no field {n}")))?;
            out.insert(
                n.to_owned(),
                json!({
                    "type": "scalar",
                    "blob": f.blob,
                    "path": wire_path(&self.store.path_of(&f.blob)),
                    "i0": 0, "j0": 0, "i1": self.grid.nx, "j1": self.grid.ny,
                }),
            );
        }
        Ok(Value::Object(out))
    }

    /// The stub: `x + y` in metres over point columns, as doubles. The real `expr.eval` evaluates
    /// any formula through `via-expr` (formulas.md 11).
    fn expr_eval(&mut self, p: &Value) -> Result<Value, RpcError> {
        let formula = p
            .get("formula")
            .and_then(Value::as_str)
            .ok_or_else(|| bad_param("formula", "formula must be a string".into()))?;
        if formula != "x + y" {
            return Err(bad_param(
                "formula",
                "the probe's stub evaluates only `x + y`".into(),
            ));
        }
        let wire = |e: column::ColumnError| RpcError {
            code: e.code,
            message: e.message,
            data: None,
        };
        let x = column::read("x", p.get("x").unwrap_or(&Value::Null), &self.store).map_err(wire)?;
        let y = column::read("y", p.get("y").unwrap_or(&Value::Null), &self.store).map_err(wire)?;
        let (Column::I32(x), Column::I32(y)) = (x, y) else {
            return Err(bad_param("x", "x and y are i32 millimetre columns".into()));
        };
        if x.len() != y.len() {
            return Err(bad_param("y", "x and y differ in length".into()));
        }
        let v: Vec<f64> = x
            .iter()
            .zip(&y)
            .map(|(&a, &b)| (f64::from(a) + f64::from(b)) / 1000.0)
            .collect();
        column::write(&Column::F64(v), &self.store).map_err(|e| RpcError {
            code: -32603,
            message: e.to_string(),
            data: None,
        })
    }
}

/// A call folder, `<store>/calls/<nonce>/out` (6.3), deleted when dropped: after the result is
/// stored, and on every failure path, so a killed worker leaves nothing behind.
pub struct CallFolder {
    root: PathBuf,
}

static NONCE: AtomicU64 = AtomicU64::new(0);

impl CallFolder {
    pub fn new(store_root: &Path) -> std::io::Result<CallFolder> {
        let n = NONCE.fetch_add(1, Ordering::Relaxed);
        let root = store_root
            .join("calls")
            .join(format!("{}-{n}", std::process::id()));
        fs::create_dir_all(root.join("out"))?;
        Ok(CallFolder { root })
    }

    pub fn out(&self) -> PathBuf {
        self.root.join("out")
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    /// Stores every file of `out` as an object; name to digest, in bytewise name order.
    pub fn collect(
        &self,
        store: &Store,
    ) -> Result<BTreeMap<String, String>, crate::store::StoreError> {
        let mut names: Vec<_> = fs::read_dir(self.out())?
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .map(|e| e.file_name())
            .collect();
        names.sort();
        let mut out = BTreeMap::new();
        for n in names {
            let d = store.adopt(&self.out().join(&n))?;
            out.insert(n.to_string_lossy().into_owned(), d);
        }
        Ok(out)
    }
}

impl Drop for CallFolder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// The probe's fixed setting: a store, an interpreter, the SDK and the step script.
pub struct Probe {
    pub store_root: PathBuf,
    pub store: Arc<Store>,
    pub python: PathBuf,
    pub sdk: PathBuf,
    pub script: PathBuf,
    pub grid: Grid,
}

/// The probe's step: declared, for its diagnostics, by the header `[step.probe]` (12 bytes) at
/// line 1 of a world the harness names.
pub fn site() -> StepSite {
    StepSite {
        id: "probe".into(),
        file: "probe/world.toml".into(),
        line: 1,
        column: 1,
        bytes: [0, 12],
    }
}

/// The probe's field: 1281 by 1281 posts (1 m over 1280 m), 6.5 MB of f32. Every 1000th value is
/// zero, so the step's negation makes `-0.0`, which must come back as `+0.0` (4.2).
pub fn probe_field(nx: usize, ny: usize) -> Vec<f32> {
    let mut v = Vec::with_capacity(nx * ny);
    for j in 0..ny {
        for i in 0..nx {
            v.push(((i * 7 + j * 13) % 1000) as f32 * 0.25 - 100.0);
        }
    }
    v
}

/// The repository's `sdk/python/`, which the probe passes as `--sdk DIR` would (8.1).
pub fn repo_sdk() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sdk/python")
}

/// The probe's step script.
pub fn fixture_script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/probe_step.py")
}

/// How to make the SDK's uv environment, the same command on every system (uv takes the venv
/// folder for `--python`).
pub const VENV_SETUP: &str =
    "uv venv sdk/python/.venv --python 3.12 && uv pip install --python sdk/python/.venv numpy";

/// The interpreter of the SDK's uv environment, `sdk/python/.venv`, made by [`VENV_SETUP`]. The
/// probe still reaches it through `config.toml` or an explicit flag, never through `PATH`.
pub fn venv_python() -> PathBuf {
    let venv = repo_sdk().join(".venv");
    if cfg!(windows) {
        venv.join("Scripts/python.exe")
    } else {
        venv.join("bin/python")
    }
}

/// `[program.<name>]` with `path`, as `via config program` will write it: the path through the
/// TOML serializer, so a backslash or a non-ASCII name is escaped as TOML escapes it.
pub fn program_config(name: &str, path: &Path) -> String {
    let p = toml::Value::String(path.to_string_lossy().into_owned());
    format!("[program.{name}]\npath = {p}\n")
}

/// Whether a process of that id is alive, for the harness's checks that no process remains.
#[cfg(unix)]
pub fn alive(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return false;
    };
    // SAFETY: signal 0 only checks that the process exists.
    unsafe { libc::kill(pid, 0) == 0 }
}

/// Whether a process of that id is alive, for the harness's checks that no process remains.
#[cfg(windows)]
pub fn alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    // SAFETY: the handle is checked and closed; the exit code is a plain out value.
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return false;
        }
        let mut code = 0u32;
        let ok = GetExitCodeProcess(h, &mut code);
        CloseHandle(h);
        ok != 0 && code == STILL_ACTIVE as u32
    }
}

/// Kills one process by id, as a fixture's outside killer would: the harness's "worker killed
/// mid-call". Not how the engine kills (it kills the whole tree, `process::Tree`).
#[cfg(unix)]
pub fn kill_pid(pid: u32) {
    if let Ok(pid) = libc::pid_t::try_from(pid) {
        // SAFETY: a plain signal to a process id the harness was handed.
        unsafe { libc::kill(pid, libc::SIGKILL) };
    }
}

/// Kills one process by id, as a fixture's outside killer would: the harness's "worker killed
/// mid-call". Not how the engine kills (it kills the whole tree, `process::Tree`).
#[cfg(windows)]
pub fn kill_pid(pid: u32) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    // SAFETY: the handle is checked and closed.
    unsafe {
        let h = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if !h.is_null() {
            TerminateProcess(h, 9);
            CloseHandle(h);
        }
    }
}

impl Probe {
    /// Writes the probe field into the store as a field file. Returns it and its values.
    pub fn seed_field(&self) -> std::io::Result<(ProbeField, Vec<f32>)> {
        let values = probe_field(self.grid.nx, self.grid.ny);
        let file = crate::npy::f32_file(&[self.grid.ny, self.grid.nx], &values);
        let blob = self.store.put(&file)?;
        Ok((ProbeField { blob }, values))
    }

    pub fn new(
        store_root: PathBuf,
        python: PathBuf,
        sdk: PathBuf,
        script: PathBuf,
    ) -> std::io::Result<Probe> {
        let store = Arc::new(Store::open(store_root.join("objects"))?);
        Ok(Probe {
            store_root,
            store,
            python,
            sdk,
            script,
            grid: Grid {
                extent_mm: [0, 0, 1_280_000, 1_280_000],
                resolution_mm: 1000,
                nx: 1281,
                ny: 1281,
            },
        })
    }

    pub fn services(&self) -> ProbeServices {
        ProbeServices {
            store: Arc::clone(&self.store),
            grid: self.grid,
            fields: BTreeMap::new(),
            answered: BTreeMap::new(),
        }
    }

    /// Starts the worker: `<python> -m viasdk <script>`, the SDK first on its path.
    pub fn start(&self, deadlines: Deadlines) -> Result<Worker, Diag> {
        let cwd = self
            .script
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        Worker::start(WorkerSpec {
            program: self.python.clone(),
            args: vec![
                OsString::from("-m"),
                OsString::from("viasdk"),
                self.script.clone().into_os_string(),
            ],
            cwd,
            sdk: Some(self.sdk.clone()),
            objects: self.store.objects().to_path_buf(),
            site: site(),
            when: "generate".into(),
            methods: METHODS.iter().map(|s| s.to_string()).collect(),
            deadlines,
        })
    }

    /// One `step.call` with `params`, its out folder stored as objects afterwards.
    pub fn call(
        &self,
        worker: &mut Worker,
        params: Value,
        services: &mut ProbeServices,
        cancel: &CancelToken,
    ) -> Result<CallResult, Diag> {
        let folder = CallFolder::new(&self.store_root).map_err(|e| io_diag(&e))?;
        let request = json!({
            "protocol": crate::PROTOCOL,
            "step": "probe",
            "when": "generate",
            "for_each_item": null,
            "params": params,
            "grid": self.grid.to_json(),
            "seed": "0000000000000000",
            "out": wire_path(&folder.out()),
            "objects": wire_path(self.store.objects()),
        });
        let outcome = worker.call(request, services, cancel)?;
        let files = match outcome {
            Outcome::Done(_) => folder.collect(&self.store).map_err(|e| {
                crate::worker::internal(&site(), &format!("storing the step's output failed: {e}"))
            })?,
            Outcome::Cancelled => BTreeMap::new(),
        };
        Ok(CallResult {
            outcome,
            files,
            call_folder: folder.path().to_path_buf(),
        })
    }
}

fn io_diag(e: &std::io::Error) -> Diag {
    let s = site();
    s.diag(
        via_diag::codes::diag::INTERNAL,
        format!("The call folder could not be made: {e}."),
        text!("Check that the store folder is writable."),
    )
    .with("message", e.to_string())
}

/// What one probe call gave.
#[derive(Debug)]
pub struct CallResult {
    pub outcome: Outcome,
    /// Files of `out`, name to blob digest.
    pub files: BTreeMap<String, String>,
    /// Already deleted when this is returned; kept to show that it was.
    pub call_folder: PathBuf,
}
