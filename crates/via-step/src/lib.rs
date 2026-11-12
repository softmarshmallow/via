//! Steps written in Rust.
//!
//! Responsibility: a worker for steps whose code is Rust (the erosion step over `via-terrain`),
//! speaking `via-step-1` through `via-proto`. It reaches the engine by the same protocol a
//! Python step uses, so a Rust step has no private door (R19b).
//!
//! Governed by `spec/protocol.md`.
//!
//! M0 holds the worker loop (3.2) and the descriptor rule of 3.1 for Rust: the protocol takes the
//! standard descriptors before the step's code runs, so a `println!` in a step reaches the log, not
//! the wire. As in the Python SDK, the protocol is read on a thread of its own (3.1): a
//! `$/cancelRequest` is seen at once ([`Step::cancelled`]), and when the input ends, which is when
//! the engine has died, the process ends even while a step is computing. macOS has no parent-death
//! signal, so this is what keeps a Rust step from running on as an orphan there (6.2).

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{self, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

use serde_json::{json, Map, Value};
use via_proto::frame::{read_frame, write_frame};
use via_proto::PROTOCOL;

/// Every point the worker loop dispatches, for `capabilities.when` (protocol.md 3.2). A step
/// lacking one of them is `protocol.no_entry` at `step.call`, so `protocol.when_unserved` never
/// comes from this crate: one condition, one code.
pub const POINTS: &[&str] = &["generate", "lint", "export"];

/// How a step function ends without a document.
#[derive(Debug)]
pub enum StepError {
    /// Answered -32010 (`protocol.raised`), with the error's type name as `data.exception`.
    Raised { exception: String, message: String },
    /// Answered -32800.
    Cancelled,
}

/// An error the engine answered to a call (3.4). Not a diagnostic by itself.
#[derive(Debug, Clone, PartialEq)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

impl From<RpcError> for StepError {
    fn from(e: RpcError) -> Self {
        if e.code == -32800 {
            return StepError::Cancelled;
        }
        StepError::Raised {
            exception: "RpcError".into(),
            message: format!("{} (code {})", e.message, e.code),
        }
    }
}

/// A step written in Rust.
pub trait StepFn {
    /// The points this step has an entry for. A `step.call` at another point is answered
    /// `protocol.no_entry`, as a script missing its function is in the Python SDK; it is not what
    /// `capabilities.when` lists, which is every point the worker loop can dispatch (3.2).
    fn when(&self) -> &'static [&'static str];
    /// One `step.call`. For a generate step, `Ok(Some(text))` is the returned document and
    /// `Ok(Some(String::new()))` an empty one ("nothing to return", protocol.md 5); `Ok(None)`
    /// returns no document, which the engine reports as `protocol.no_output` unless the step wrote
    /// `document.toml`. Lint and export steps return `Ok(None)`.
    fn call(&mut self, step: &mut Step<'_>) -> Result<Option<String>, StepError>;
}

/// State the reader thread and the worker loop share.
#[derive(Default)]
struct Shared {
    /// Ids of `step.call`s the engine asked to stop.
    cancelled: Mutex<BTreeSet<i64>>,
    /// A `step.call` is running: if the input ends now, nothing can be answered, so the process
    /// ends at once (3.1).
    busy: AtomicBool,
    /// The input has ended.
    closed: AtomicBool,
}

impl Shared {
    fn is_cancelled(&self, call: i64) -> bool {
        self.cancelled
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .contains(&call)
    }
}

/// The protocol's two descriptors, taken from the standard ones (3.1), with the thread that reads
/// one of them.
pub struct Conn {
    w: File,
    next: i64,
    /// The engine's requests and notifications other than `$/cancelRequest`, in order.
    inbox: Receiver<Value>,
    /// Responses to this worker's own requests.
    responses: Receiver<Value>,
    shared: Arc<Shared>,
}

/// The engine is gone and nothing can be answered: end the process now (3.1).
fn engine_gone() -> ! {
    std::process::exit(0)
}

/// Duplicates descriptors 0 and 1 for the protocol (close-on-exec), then points 0 at the null
/// device and 1 at standard error. Call it first in `main`, before any step code runs.
#[cfg(unix)]
pub fn take_stdio() -> io::Result<Conn> {
    use std::os::fd::FromRawFd;
    // SAFETY: plain descriptor calls; each returned descriptor is checked and owned exactly once.
    let (pin, pout) = unsafe {
        let pin = libc::fcntl(0, libc::F_DUPFD_CLOEXEC, 3);
        let pout = libc::fcntl(1, libc::F_DUPFD_CLOEXEC, 3);
        if pin < 0 || pout < 0 {
            return Err(io::Error::last_os_error());
        }
        let null = libc::open(c"/dev/null".as_ptr(), libc::O_RDONLY);
        if null < 0 || libc::dup2(null, 0) < 0 || libc::dup2(2, 1) < 0 {
            return Err(io::Error::last_os_error());
        }
        libc::close(null);
        (File::from_raw_fd(pin), File::from_raw_fd(pout))
    };
    Conn::new(pin, pout)
}

/// The same move with Win32 calls: the standard input and output handles are duplicated for the
/// protocol (not inheritable, the originals closed by the duplication), then standard input is
/// set to a handle on `NUL` and standard output to standard error. Rust's `std::io::stdout` asks
/// for the standard handle on each write, so a `println!` reaches the log. Only the duplicates are
/// wrapped in `File`, so dropping the `Conn` closes nothing std still uses.
#[cfg(windows)]
pub fn take_stdio() -> io::Result<Conn> {
    use std::os::windows::io::FromRawHandle;
    use windows_sys::Win32::Foundation::{
        DuplicateHandle, DUPLICATE_CLOSE_SOURCE, DUPLICATE_SAME_ACCESS, GENERIC_READ, HANDLE,
        INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Console::{
        GetStdHandle, SetStdHandle, STD_ERROR_HANDLE, STD_HANDLE, STD_INPUT_HANDLE,
        STD_OUTPUT_HANDLE,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    // SAFETY: every handle is checked before use; each duplicate is owned exactly once by a File;
    // the wide string is NUL-terminated and outlives the call.
    unsafe {
        let me = GetCurrentProcess();
        let take = |std: STD_HANDLE| -> io::Result<HANDLE> {
            let h = GetStdHandle(std);
            if h.is_null() || h == INVALID_HANDLE_VALUE {
                return Err(io::Error::other("a standard handle is missing"));
            }
            let mut dup: HANDLE = std::ptr::null_mut();
            let ok = DuplicateHandle(
                me,
                h,
                me,
                &mut dup,
                0,
                0,
                DUPLICATE_SAME_ACCESS | DUPLICATE_CLOSE_SOURCE,
            );
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(dup)
        };
        let pin = take(STD_INPUT_HANDLE)?;
        let pout = take(STD_OUTPUT_HANDLE)?;
        let nul: Vec<u16> = "NUL\0".encode_utf16().collect();
        let null = CreateFileW(
            nul.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );
        if null == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let err = GetStdHandle(STD_ERROR_HANDLE);
        if SetStdHandle(STD_INPUT_HANDLE, null) == 0 || SetStdHandle(STD_OUTPUT_HANDLE, err) == 0 {
            return Err(io::Error::last_os_error());
        }
        Conn::new(File::from_raw_handle(pin), File::from_raw_handle(pout))
    }
}

impl Conn {
    /// Starts the reader thread on `r`; `w` is written by the worker loop and the step.
    pub fn new(r: File, w: File) -> io::Result<Conn> {
        let (inbox_tx, inbox) = channel();
        let (resp_tx, responses) = channel();
        let shared = Arc::new(Shared::default());
        let sh = Arc::clone(&shared);
        std::thread::Builder::new()
            .name("via-step-protocol".into())
            .spawn(move || {
                let mut r = BufReader::with_capacity(1 << 16, r);
                // Any fault on the wire ends the input, as a closed pipe does.
                while let Ok(Some(body)) = read_frame(&mut r) {
                    let Ok(m) = serde_json::from_slice::<Value>(&body) else {
                        eprintln!("via-step: a protocol frame is not JSON");
                        break;
                    };
                    if m.get("method").and_then(Value::as_str) == Some("$/cancelRequest") {
                        if let Some(c) = m.pointer("/params/id").and_then(Value::as_i64) {
                            sh.cancelled
                                .lock()
                                .unwrap_or_else(|p| p.into_inner())
                                .insert(c);
                        }
                        continue;
                    }
                    let to = if m.get("method").is_some() {
                        &inbox_tx
                    } else {
                        &resp_tx
                    };
                    if to.send(m).is_err() {
                        break;
                    }
                }
                // The input has ended: the engine is gone, or it sent its last message (quit).
                // Messages already queued are still served, then the loop ends. A step that is
                // running, or waiting on an answer that can no longer come, ends the process now.
                sh.closed.store(true, Ordering::SeqCst);
                if sh.busy.load(Ordering::SeqCst) {
                    engine_gone();
                }
            })?;
        Ok(Conn {
            w,
            next: 0,
            inbox,
            responses,
            shared,
        })
    }

    fn send(&mut self, msg: &Value) -> io::Result<()> {
        let body = serde_json::to_vec(msg).map_err(io::Error::other)?;
        write_frame(&mut self.w, &body).map_err(|e| io::Error::other(e.to_string()))
    }

    fn respond(&mut self, id: &Value, outcome: Result<Value, Value>) {
        let msg = match outcome {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err(e) => json!({"jsonrpc": "2.0", "id": id, "error": e}),
        };
        let _ = self.send(&msg);
    }

    /// Marks a step.call as running or done. Set while the input has already ended, the call can
    /// never be answered, so the process ends (as the reader thread does in the other order).
    fn busy_with(&self, flag: bool) {
        self.shared.busy.store(flag, Ordering::SeqCst);
        if flag && self.shared.closed.load(Ordering::SeqCst) {
            engine_gone();
        }
    }
}

/// One call of a step: its request, and the door back to the engine.
pub struct Step<'a> {
    pub request: Value,
    conn: &'a mut Conn,
    call: i64,
}

/// An engine call's params with `call` set: an object, or null for none. Anything else is the
/// step's own mistake, answered as a raised error rather than a panic.
fn with_call(params: Value, call: i64) -> Result<Value, StepError> {
    let mut obj = match params {
        Value::Null => Map::new(),
        Value::Object(o) => o,
        other => {
            return Err(StepError::Raised {
                exception: "ParamsError".into(),
                message: format!("engine call params must be an object or null, not {other}"),
            })
        }
    };
    obj.insert("call".into(), json!(call));
    Ok(Value::Object(obj))
}

impl Step<'_> {
    /// True once the engine has asked this call to stop (`$/cancelRequest`). A long computation
    /// polls it and returns `Err(StepError::Cancelled)`; an engine call checks it itself.
    pub fn cancelled(&self) -> bool {
        self.conn.shared.is_cancelled(self.call)
    }

    /// One engine call (3.3), `call` set to this step.call's id.
    pub fn engine(&mut self, method: &str, params: Value) -> Result<Value, StepError> {
        if self.cancelled() {
            return Err(StepError::Cancelled);
        }
        let params = with_call(params, self.call)?;
        self.conn.next += 1;
        let id = self.conn.next;
        let msg = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        if self.conn.send(&msg).is_err() {
            engine_gone();
        }
        loop {
            // Disconnected: the reader thread has ended, so the answer can never come.
            let Ok(m) = self.conn.responses.recv() else {
                engine_gone();
            };
            if m.get("id").and_then(Value::as_i64) != Some(id) {
                continue; // A stray response to nothing pending: ignored, as JSON-RPC does.
            }
            if let Some(e) = m.get("error") {
                return Err(RpcError {
                    code: e.get("code").and_then(Value::as_i64).unwrap_or(0),
                    message: e
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .into(),
                    data: e.get("data").cloned(),
                }
                .into());
            }
            return Ok(m.get("result").cloned().unwrap_or(Value::Null));
        }
    }
}

const CANCELLED: i64 = -32800;

/// The answer to one `step.call`, from what the step returned.
fn answer(
    when: &str,
    r: Result<Option<String>, StepError>,
    cancelled: bool,
) -> Result<Value, Value> {
    match r {
        _ if cancelled => Err(json!({"code": CANCELLED, "message": "cancelled"})),
        Err(StepError::Cancelled) => Err(json!({"code": CANCELLED, "message": "cancelled"})),
        Err(StepError::Raised { exception, message }) => {
            eprintln!("{exception}: {message}");
            Err(json!({"code": -32010, "message": message,
                       "data": {"exception": exception, "traceback": message}}))
        }
        // None stays null, so "returned neither" (protocol.no_output) is not mistaken for "wrote
        // an empty document" (protocol.md 5).
        Ok(doc) if when == "generate" => Ok(json!({ "document": doc })),
        Ok(_) => Ok(Value::Null),
    }
}

/// The worker loop: answers `initialize`, each `step.call`, `shutdown` and `quit`. Returns the
/// process exit status.
pub fn serve(conn: &mut Conn, step: &mut dyn StepFn) -> i32 {
    loop {
        // Disconnected: the input has ended and every queued message is served.
        let Ok(msg) = conn.inbox.recv() else {
            return 0;
        };
        let method = msg
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let id = msg.get("id").cloned();
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        match (method.as_str(), id) {
            ("initialize", Some(id)) => {
                if params.get("protocol").and_then(Value::as_str) != Some(PROTOCOL) {
                    conn.respond(
                        &id,
                        Err(json!({"code": -32001, "message": format!("this worker speaks {PROTOCOL}"),
                                   "data": {"protocol": PROTOCOL}})),
                    );
                    return 1;
                }
                conn.respond(
                    &id,
                    Ok(json!({"protocol": PROTOCOL,
                              "sdk": {"name": "via-step", "version": env!("CARGO_PKG_VERSION")},
                              "capabilities": {"when": POINTS}})),
                );
            }
            ("step.call", Some(id)) => {
                let call = id.as_i64().unwrap_or(0);
                let when = params
                    .get("when")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned();
                let outcome = if !step.when().contains(&when.as_str()) {
                    Err(
                        json!({"code": -32601, "message": format!("no entry for {when}"),
                               "data": {"code": "protocol.no_entry", "when": when}}),
                    )
                } else {
                    conn.busy_with(true);
                    let mut s = Step {
                        request: params,
                        conn: &mut *conn,
                        call,
                    };
                    let r = step.call(&mut s);
                    conn.busy_with(false);
                    answer(&when, r, conn.shared.is_cancelled(call))
                };
                conn.respond(&id, outcome);
            }
            ("shutdown", Some(id)) => conn.respond(&id, Ok(Value::Null)),
            ("quit", _) => return 0,
            (_, Some(id)) => conn.respond(
                &id,
                Err(
                    json!({"code": -32601, "message": format!("unknown method {method}"),
                           "data": {"method": method}}),
                ),
            ),
            (_, None) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_is_no_document_and_empty_is_an_empty_one() {
        assert_eq!(
            answer("generate", Ok(None), false),
            Ok(json!({"document": null}))
        );
        assert_eq!(
            answer("generate", Ok(Some(String::new())), false),
            Ok(json!({"document": ""}))
        );
        assert_eq!(answer("lint", Ok(None), false), Ok(Value::Null));
        let e = answer("generate", Ok(Some("x".into())), true).unwrap_err();
        assert_eq!(e["code"], CANCELLED);
    }

    #[test]
    fn params_must_be_an_object_or_null() {
        assert_eq!(with_call(Value::Null, 3).unwrap(), json!({"call": 3}));
        assert_eq!(
            with_call(json!({"fields": ["h"]}), 3).unwrap(),
            json!({"fields": ["h"], "call": 3})
        );
        for bad in [json!([1, 2]), json!("x"), json!(1)] {
            assert!(matches!(
                with_call(bad, 3),
                Err(StepError::Raised { exception, .. }) if exception == "ParamsError"
            ));
        }
    }
}
