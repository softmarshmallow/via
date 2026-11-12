//! The worker transport (`spec/protocol.md` 3 and 6.2): start, `initialize`, `step.call` with call
//! backs, `$/cancelRequest`, `shutdown` and `quit`, and every way a worker can fail, each reported
//! as one located diagnostic.
//!
//! One reader thread turns the worker's output into messages and one drains its standard error into
//! the log, so a chatty worker never blocks and a dead one is seen at once (its output ends). The
//! calling thread owns the conversation: it sends, receives and answers call backs in order, so the
//! engine side needs no locks around the protocol state.

use std::collections::{BTreeSet, VecDeque};
use std::ffi::OsString;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use via_diag::{codes, text, Diag};

use crate::diag::StepSite;
use crate::frame::{read_frame, write_frame, FrameError};
use crate::process::Tree;
use crate::PROTOCOL;

/// The deadlines of 6.2. Defaults until measured (`protocol.md`, Open).
#[derive(Clone, Copy, Debug)]
pub struct Deadlines {
    pub initialize: Duration,
    pub call: Duration,
    pub cancel: Duration,
    pub exit: Duration,
}

impl Default for Deadlines {
    fn default() -> Self {
        Deadlines {
            initialize: Duration::from_secs(10),
            call: Duration::from_secs(600),
            cancel: Duration::from_secs(5),
            exit: Duration::from_secs(2),
        }
    }
}

/// How to start one worker.
pub struct WorkerSpec {
    /// The resolved program (6.1): from `config.toml` or an explicit path, never from `PATH`.
    pub program: PathBuf,
    pub args: Vec<OsString>,
    /// The folder of the declaring document.
    pub cwd: PathBuf,
    /// The SDK's folder, put first on `PYTHONPATH` (8.1).
    pub sdk: Option<PathBuf>,
    pub objects: PathBuf,
    /// The step's declaration: every diagnostic of this worker is located there.
    pub site: StepSite,
    /// The point the step runs at; the worker must serve it.
    pub when: String,
    /// The worker-to-engine methods offered, for `initialize`'s `capabilities.methods`.
    pub methods: Vec<String>,
    pub deadlines: Deadlines,
}

/// An error answered to a call back (3.4). Not by itself a diagnostic: the SDK raises it.
#[derive(Clone, Debug, PartialEq)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

/// What the engine answers call backs with.
pub trait Services {
    fn handle(&mut self, method: &str, params: &Value) -> Result<Value, RpcError>;
}

/// Asks a pending `step.call` to stop, from any thread.
#[derive(Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    fn is_set(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// How a `step.call` ended, when no diagnostic is owed.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    /// The call's result: `{ "document": ... }` or `null`.
    Done(Value),
    /// Cancelled by the engine's token and answered -32800. Its output is discarded.
    Cancelled,
}

enum Incoming {
    Message(Value),
    Framing(FrameError),
    NotJson(Vec<u8>),
    Closed,
}

enum Event {
    Message(Value),
    Framing(Diag),
    Closed,
    Timeout,
}

pub struct Worker {
    tree: Tree,
    stdin: Option<ChildStdin>,
    rx: Receiver<Incoming>,
    next_id: i64,
    log: Arc<Mutex<Log>>,
    site: StepSite,
    deadlines: Deadlines,
    sdk: Value,
}

/// A path as the wire carries it: absolute, UTF-8, `/` as separator on every system (2).
pub fn wire_path(p: &Path) -> String {
    let s = p.to_string_lossy();
    if cfg!(windows) {
        s.replace('\\', "/")
    } else {
        s.into_owned()
    }
}

impl Worker {
    /// Starts the worker and runs `initialize`. On any failure the tree is already killed.
    pub fn start(spec: WorkerSpec) -> Result<Worker, Diag> {
        let site = spec.site.clone();
        let mut cmd = Command::new(&spec.program);
        cmd.args(&spec.args)
            .current_dir(&spec.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // The launcher tells the transports apart by VIA_STEP alone (8): never set for a worker.
            .env_remove("VIA_STEP")
            .env("VIA_PROTOCOL", PROTOCOL)
            .env("PYTHONHASHSEED", "0")
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .env("PYTHONSAFEPATH", "1")
            .env("PYTHONUTF8", "1");
        if let Some(sdk) = &spec.sdk {
            let mut paths = vec![sdk.clone()];
            if let Some(p) = std::env::var_os("PYTHONPATH") {
                paths.extend(std::env::split_paths(&p));
            }
            if let Ok(joined) = std::env::join_paths(paths) {
                cmd.env("PYTHONPATH", joined);
            }
        }
        let mut tree = Tree::spawn(cmd).map_err(|e| {
            let kind = match e.kind() {
                std::io::ErrorKind::NotFound => "not_found",
                std::io::ErrorKind::PermissionDenied => "denied",
                _ => "other",
            };
            site.diag(
                codes::protocol::START,
                format!("The program of step {} could not be started.", site.id),
                text!("Check the program named in config.toml (via config program) and the step's command."),
            )
            .with("kind", kind)
        })?;
        let child = tree.child_mut();
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().expect("stdout is piped");
        let stderr = child.stderr.take().expect("stderr is piped");

        let log = Arc::new(Mutex::new(Log::default()));
        let log_w = Arc::clone(&log);
        std::thread::Builder::new()
            .name("via-step-log".into())
            .spawn(move || drain(stderr, &log_w))
            .map_err(|_| internal(&site, "the log thread could not start"))?;

        let (tx, rx) = channel();
        std::thread::Builder::new()
            .name("via-step-read".into())
            .spawn(move || {
                let mut r = BufReader::with_capacity(1 << 16, stdout);
                loop {
                    let msg = match read_frame(&mut r) {
                        Ok(Some(body)) => match serde_json::from_slice::<Value>(&body) {
                            Ok(v) => Incoming::Message(v),
                            Err(_) => Incoming::NotJson(body),
                        },
                        Ok(None) | Err(FrameError::Io(_)) => Incoming::Closed,
                        Err(e) => Incoming::Framing(e),
                    };
                    let last = !matches!(msg, Incoming::Message(_));
                    if tx.send(msg).is_err() || last {
                        return;
                    }
                }
            })
            .map_err(|_| internal(&site, "the reader thread could not start"))?;

        let mut w = Worker {
            tree,
            stdin,
            rx,
            next_id: 0,
            log,
            site,
            deadlines: spec.deadlines,
            sdk: Value::Null,
        };
        w.initialize(&spec.objects, &spec.methods, &spec.when)?;
        Ok(w)
    }

    pub fn pid(&self) -> u32 {
        self.tree.id()
    }

    /// What the worker said its SDK is, for the step result (never checked, 8).
    pub fn sdk(&self) -> &Value {
        &self.sdk
    }

    /// The step's log so far: its standard error, invalid UTF-8 replaced (6.2). A log past
    /// [`LOG_HEAD`] + [`LOG_TAIL`] bytes keeps its head and tail and says how much was dropped.
    pub fn log(&self) -> String {
        self.log.lock().unwrap_or_else(|p| p.into_inner()).text()
    }

    fn initialize(&mut self, objects: &Path, methods: &[String], when: &str) -> Result<(), Diag> {
        let id = self.request(
            "initialize",
            json!({
                "protocol": PROTOCOL,
                "engine": {"name": "via", "version": env!("CARGO_PKG_VERSION")},
                "objects": wire_path(objects),
                "capabilities": {"methods": methods},
            }),
        );
        let until = Instant::now() + self.deadlines.initialize;
        let answer = loop {
            match self.next_event(until) {
                Event::Message(m) if is_response_to(&m, id) => break m,
                Event::Message(m) => self.refuse(&m, -32005, "no step.call is pending"),
                Event::Framing(d) => return Err(self.fail(d)),
                Event::Closed => {
                    let d = self.crashed("before answering initialize");
                    return Err(self.fail(d));
                }
                Event::Timeout => {
                    let d = self.deadline("initialize", self.deadlines.initialize);
                    return Err(self.fail(d));
                }
            }
        };
        if let Some(err) = answer.get("error") {
            let d = if err.get("code").and_then(Value::as_i64) == Some(-32001) {
                let theirs = err
                    .pointer("/data/protocol")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                self.version(theirs)
            } else {
                self.wire_error_diag(err)
            };
            self.notify("quit", Value::Null);
            return Err(self.fail(d));
        }
        let result = answer.get("result").cloned().unwrap_or(Value::Null);
        let theirs = result.get("protocol").and_then(Value::as_str).unwrap_or("");
        if theirs != PROTOCOL {
            let d = self.version(if theirs.is_empty() { "none" } else { theirs });
            self.notify("quit", Value::Null);
            return Err(self.fail(d));
        }
        let serves: BTreeSet<&str> = result
            .pointer("/capabilities/when")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        if !serves.contains(when) {
            let d = self
                .site
                .diag(
                    codes::protocol::WHEN_UNSERVED,
                    format!(
                        "Step {} runs at {when}, but its worker serves only {:?}.",
                        self.site.id,
                        serves.iter().collect::<Vec<_>>()
                    ),
                    // viasdk and via-step list every point, so this is a worker program built for
                    // fewer; a script missing its function is protocol.no_entry at step.call (3.2).
                    text!("Run the step with a worker program that dispatches its `when`, such as viasdk."),
                )
                .with("when", when);
            self.notify("quit", Value::Null);
            return Err(self.fail(d));
        }
        self.sdk = result.get("sdk").cloned().unwrap_or(Value::Null);
        Ok(())
    }

    /// Runs one `step.call`, answering the worker's call backs from `services` while it is
    /// pending. Every failure is one located diagnostic, and after a failure the tree is killed.
    pub fn call(
        &mut self,
        request: Value,
        services: &mut dyn Services,
        cancel: &CancelToken,
    ) -> Result<Outcome, Diag> {
        let id = self.request("step.call", request);
        let call_deadline = Instant::now() + self.deadlines.call;
        // Some(by_token) once $/cancelRequest is sent; then the 5 s cancel deadline runs.
        let mut cancelling: Option<(bool, Instant)> = None;
        loop {
            let now = Instant::now();
            if cancelling.is_none() && (cancel.is_set() || now >= call_deadline) {
                self.notify("$/cancelRequest", json!({"id": id}));
                cancelling = Some((cancel.is_set(), now + self.deadlines.cancel));
            }
            let until = match cancelling {
                Some((_, d)) => d,
                // Wake now and then to see the token; a message wakes us at once.
                None => call_deadline.min(now + Duration::from_millis(20)),
            };
            match self.next_event(until) {
                Event::Message(m) if is_response_to(&m, id) => {
                    return self.finish_call(m, cancelling.map(|c| c.0));
                }
                Event::Message(m) if m.get("method").is_some() => {
                    if cancelling.is_some() {
                        self.refuse(&m, -32800, "the step.call is being cancelled");
                    } else {
                        self.serve(&m, id, services);
                    }
                }
                // A stray response to nothing we sent: ignored, as JSON-RPC does.
                Event::Message(_) => {}
                Event::Framing(d) => return Err(self.fail(d)),
                Event::Closed => {
                    let d = self.crashed("during step.call");
                    return Err(self.fail(d));
                }
                Event::Timeout => {
                    if let Some((_, d)) = cancelling {
                        if Instant::now() >= d {
                            let diag = self.deadline("cancel", self.deadlines.cancel);
                            return Err(self.fail(diag));
                        }
                    }
                }
            }
        }
    }

    fn finish_call(&mut self, m: Value, cancelled: Option<bool>) -> Result<Outcome, Diag> {
        match cancelled {
            Some(true) => return Ok(Outcome::Cancelled),
            Some(false) => {
                let d = self.deadline("step.call", self.deadlines.call);
                return Err(self.fail(d));
            }
            None => {}
        }
        if let Some(result) = m.get("result") {
            return Ok(Outcome::Done(result.clone()));
        }
        let err = m.get("error").cloned().unwrap_or(Value::Null);
        // The worker stays up after a raised step (3.4); only the call failed.
        Err(self.wire_error_diag(&err))
    }

    /// Asks the worker to stop: `shutdown`, then `quit`, then waits; killed past the deadline,
    /// which is `protocol.slow_exit` (info).
    pub fn shutdown(mut self) -> Option<Diag> {
        let until = Instant::now() + self.deadlines.exit;
        let id = self.request("shutdown", Value::Null);
        loop {
            match self.next_event(until) {
                Event::Message(m) if is_response_to(&m, id) => break,
                Event::Message(m) if m.get("method").is_some() => {
                    self.refuse(&m, -32005, "no step.call is pending")
                }
                Event::Message(_) => {}
                Event::Closed | Event::Framing(_) | Event::Timeout => break,
            }
        }
        self.notify("quit", Value::Null);
        self.stdin = None;
        while Instant::now() < until {
            if let Ok(Some(_)) = self.tree.try_wait() {
                // Exited: the reap killed anything it left in its group.
                return None;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let _ = self.tree.kill();
        Some(self.site.diag(
            codes::protocol::SLOW_EXIT,
            format!(
                "The worker of step {} did not exit within {} s of shutdown and was killed.",
                self.site.id,
                self.deadlines.exit.as_secs()
            ),
            text!("Return from the step function promptly; do not keep threads or children alive."),
        ))
    }

    /// Kills the tree now (a deadline, a crash, or the engine stopping).
    pub fn kill(&mut self) {
        self.stdin = None;
        let _ = self.tree.kill();
    }

    fn next_event(&mut self, until: Instant) -> Event {
        let wait = until.saturating_duration_since(Instant::now());
        match self.rx.recv_timeout(wait) {
            Ok(Incoming::Message(v)) => Event::Message(v),
            Ok(Incoming::Framing(FrameError::TooLarge(n))) => Event::Framing(
                self.site
                    .diag(
                        codes::protocol::MESSAGE_TOO_LARGE,
                        format!(
                            "The worker of step {} sent a frame of {n} bytes; the limit is 16 MiB.",
                            self.site.id
                        ),
                        text!("Send large arrays as blobs (field files or column blobs), not inline JSON."),
                    )
                    .with("bytes", i64::try_from(n).unwrap_or(i64::MAX)),
            ),
            Ok(Incoming::Framing(e)) => Event::Framing(self.framing(&e.to_string())),
            Ok(Incoming::NotJson(body)) => {
                let head = String::from_utf8_lossy(&body[..body.len().min(200)]).into_owned();
                Event::Framing(self.framing(&format!("a frame is not JSON: {head:?}")))
            }
            Ok(Incoming::Closed) | Err(RecvTimeoutError::Disconnected) => Event::Closed,
            Err(RecvTimeoutError::Timeout) => Event::Timeout,
        }
    }

    fn serve(&mut self, m: &Value, pending: i64, services: &mut dyn Services) {
        let Some(id) = m.get("id").cloned() else {
            return; // A notification from the worker: none is defined yet; ignored.
        };
        let method = m.get("method").and_then(Value::as_str).unwrap_or("");
        let params = m.get("params").cloned().unwrap_or(Value::Null);
        if params.get("call").and_then(Value::as_i64) != Some(pending) {
            self.refuse(m, -32005, "`call` does not name the pending step.call");
            return;
        }
        let reply = match services.handle(method, &params) {
            Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
            Err(e) => {
                let mut err = json!({"code": e.code, "message": e.message});
                if let Some(d) = e.data {
                    err["data"] = d;
                }
                json!({"jsonrpc": "2.0", "id": id, "error": err})
            }
        };
        self.send(&reply);
    }

    fn refuse(&mut self, m: &Value, code: i64, message: &str) {
        if let Some(id) = m.get("id").cloned() {
            self.send(&json!({"jsonrpc": "2.0", "id": id,
                              "error": {"code": code, "message": message}}));
        }
    }

    fn request(&mut self, method: &str, params: Value) -> i64 {
        self.next_id += 1;
        let id = self.next_id;
        let mut msg = json!({"jsonrpc": "2.0", "id": id, "method": method});
        if !params.is_null() {
            msg["params"] = params;
        }
        self.send(&msg);
        id
    }

    fn notify(&mut self, method: &str, params: Value) {
        let mut msg = json!({"jsonrpc": "2.0", "method": method});
        if !params.is_null() {
            msg["params"] = params;
        }
        self.send(&msg);
    }

    /// A failed write means the worker is gone; the reader thread reports that as `Closed`, so the
    /// error is not reported twice.
    fn send(&mut self, msg: &Value) {
        if let Some(stdin) = self.stdin.as_mut() {
            let body = serde_json::to_vec(msg).expect("a JSON value serialises");
            if write_frame(stdin, &body).is_err() {
                self.stdin = None;
            }
        }
    }

    /// Kills the tree and passes the diagnostic on: every failure ends the same way.
    fn fail(&mut self, d: Diag) -> Diag {
        self.kill();
        d
    }

    fn crashed(&mut self, when: &str) -> Diag {
        // Kill first: if the root died, its children may still hold the group.
        self.kill();
        let status = self.tree.wait().ok();
        self.note(&format!("[via] worker exited {when}: {status:?}\n"));
        self.site.diag(
            codes::protocol::CRASHED,
            format!("The worker of step {} exited {when}.", self.site.id),
            text!("Read the step's log for the cause; the step's process tree has been killed."),
        )
    }

    fn deadline(&self, phase: &str, limit: Duration) -> Diag {
        let d = self
            .site
            .diag(
                codes::protocol::DEADLINE,
                format!(
                    "The worker of step {} did not answer {phase} within {} s.",
                    self.site.id,
                    limit.as_secs_f64()
                ),
                text!("Make the step faster, or raise the deadline with --step-deadline."),
            )
            .with("phase", phase);
        // A Duration is finite, so this always holds. Through via-diag's Value, 600 s is written
        // `600`, as numbers.md 3.4 writes numbers.
        match via_diag::Value::double(limit.as_secs_f64()) {
            Some(v) => d.with("deadline", v),
            None => d,
        }
    }

    fn version(&self, theirs: &str) -> Diag {
        self.site
            .diag(
                codes::protocol::VERSION,
                format!(
                    "The worker of step {} speaks {theirs}; this engine speaks {PROTOCOL}.",
                    self.site.id
                ),
                text!("Use the viasdk that came with this engine (via build --sdk only for SDK work)."),
            )
            .with("engine", PROTOCOL)
        .with("worker", theirs)
    }

    fn framing(&self, what: &str) -> Diag {
        self.note(&format!("[via] framing: {what}\n"));
        self.site.diag(
            codes::protocol::FRAMING,
            format!(
                "The worker of step {} wrote something that is not a protocol frame.",
                self.site.id
            ),
            text!(
                "Start Python steps through the launcher (python -m viasdk script.py), which moves \
                 print and standard output off the protocol descriptor (protocol.md 3.1)."
            ),
        )
    }

    /// A wire error answering `step.call` or `initialize`, as its diagnostic (3.4, 6.2).
    fn wire_error_diag(&self, err: &Value) -> Diag {
        let code = err.get("code").and_then(Value::as_i64).unwrap_or(0);
        match code {
            -32010 => {
                let exception = err
                    .pointer("/data/exception")
                    .and_then(Value::as_str)
                    .unwrap_or("Exception")
                    .to_owned();
                if let Some(tb) = err.pointer("/data/traceback").and_then(Value::as_str) {
                    self.note(tb);
                }
                self.site
                    .diag(
                        codes::protocol::RAISED,
                        format!("Step {} raised {exception}.", self.site.id),
                        text!("Read the traceback in the step's log."),
                    )
                    .with("exception", exception)
            }
            -32601 => self.site.diag(
                codes::protocol::NO_ENTRY,
                format!(
                    "The script of step {} has no entry for its point.",
                    self.site.id
                ),
                text!("Define a top-level function named after `when`: generate, lint or export."),
            ),
            _ => internal(
                &self.site,
                &format!("the worker answered with error {code}: {err}"),
            ),
        }
    }

    fn note(&self, text: &str) {
        self.log
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(text.as_bytes());
    }
}

/// A fault of ours, not the author's; still located at the step so it is never lost.
pub(crate) fn internal(site: &StepSite, what: &str) -> Diag {
    site.diag(
        codes::diag::INTERNAL,
        format!("Internal error running step {}: {what}.", site.id),
        text!("Report this as a bug in via, with the step's log."),
    )
    .with("message", what)
}

/// Bytes of a step's log kept from its start.
pub const LOG_HEAD: usize = 1 << 20;
/// Bytes of a step's log kept from its end.
pub const LOG_TAIL: usize = 3 << 20;

/// A step's log in memory, bounded, so a runaway step cannot make the engine grow without limit.
/// The head says how the step began and the tail how it ended, which is what a reader needs; the
/// middle is counted, not kept.
#[derive(Default)]
struct Log {
    head: Vec<u8>,
    tail: VecDeque<u8>,
    dropped: u64,
}

impl Log {
    fn push(&mut self, mut bytes: &[u8]) {
        if self.head.len() < LOG_HEAD {
            let n = bytes.len().min(LOG_HEAD - self.head.len());
            self.head.extend_from_slice(&bytes[..n]);
            bytes = &bytes[n..];
        }
        self.tail.extend(bytes);
        if self.tail.len() > LOG_TAIL {
            let excess = self.tail.len() - LOG_TAIL;
            self.tail.drain(..excess);
            self.dropped += excess as u64;
        }
    }

    fn text(&self) -> String {
        let mut bytes = self.head.clone();
        if self.dropped > 0 {
            bytes.extend_from_slice(
                format!("\n[via] {} bytes of the log dropped here\n", self.dropped).as_bytes(),
            );
        }
        bytes.extend(self.tail.iter());
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

fn is_response_to(m: &Value, id: i64) -> bool {
    m.get("method").is_none() && m.get("id").and_then(Value::as_i64) == Some(id)
}

fn drain(mut r: impl Read, log: &Mutex<Log>) {
    let mut buf = [0u8; 8192];
    loop {
        match r.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => log
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .push(&buf[..n]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_keeps_head_and_tail_and_counts_the_rest() {
        let mut log = Log::default();
        let chunk = vec![b'x'; 1 << 16];
        for _ in 0..100 {
            log.push(&chunk);
        }
        log.push(b"the end");
        assert_eq!(log.head.len(), LOG_HEAD);
        assert_eq!(log.tail.len(), LOG_TAIL);
        let total = 100 * (1u64 << 16) + 7;
        assert_eq!(log.dropped, total - (LOG_HEAD + LOG_TAIL) as u64);
        let t = log.text();
        assert!(t.ends_with("the end"));
        assert!(t.contains("bytes of the log dropped here"));
    }
}
