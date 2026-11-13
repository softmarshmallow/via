//! Ctrl+C and `SIGTERM` while steps run (`spec/protocol.md` 6.2).
//!
//! A step runs in its own process group, so a terminal's Ctrl+C reaches the engine alone. With no
//! step running, the signal keeps its default action: the engine ends at once, and the directory
//! lock goes with it. With steps running, the handler only records the signal; each step's wait
//! loop sees it, kills its process tree, removes its call folder (6.3), and the engine then ends
//! by the same signal once no step is left, so the shell sees an interrupted process.

use std::path::PathBuf;
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// The signal received while steps ran; 0 for none.
static SIGNAL: AtomicI32 = AtomicI32::new(0);
/// Steps started and not yet cleaned up.
static LIVE: AtomicUsize = AtomicUsize::new(0);

/// How long the last step to clean up is waited for before the engine ends anyway.
const CLEANUP_WAIT: Duration = Duration::from_secs(5);

/// Ends the process by `sig`'s default action. Async-signal-safe.
fn die_by(sig: libc::c_int) {
    // SAFETY: `signal` and `raise` are async-signal-safe, and take no pointers.
    unsafe {
        libc::signal(sig, libc::SIG_DFL);
        libc::raise(sig);
    }
}

extern "C" fn on_signal(sig: libc::c_int) {
    if LIVE.load(Ordering::SeqCst) == 0 {
        die_by(sig);
        return;
    }
    SIGNAL.store(sig, Ordering::SeqCst);
}

/// Installs the handler for `SIGINT` and `SIGTERM`. Called once, before any step can start.
pub fn install() {
    for sig in [libc::SIGINT, libc::SIGTERM] {
        // SAFETY: a zeroed `sigaction` is valid; the handler only touches atomics and calls
        // async-signal-safe functions.
        unsafe {
            let mut sa: libc::sigaction = std::mem::zeroed();
            sa.sa_sigaction = on_signal as extern "C" fn(libc::c_int) as libc::sighandler_t;
            sa.sa_flags = libc::SA_RESTART;
            libc::sigemptyset(&mut sa.sa_mask);
            libc::sigaction(sig, &sa, std::ptr::null_mut());
        }
    }
}

/// The signal received while steps ran, if any.
pub fn pending() -> Option<i32> {
    match SIGNAL.load(Ordering::SeqCst) {
        0 => None,
        s => Some(s),
    }
}

/// One running step and its call folder, counted while it lives so a signal is held for its
/// cleanup. Dropped after the step's process tree (declare it first): with a signal pending, the
/// drop removes the call folder and ends the process.
pub struct Live {
    folder: PathBuf,
}

impl Live {
    /// Counts a step whose call folder is `folder`.
    pub fn enter(folder: PathBuf) -> Live {
        LIVE.fetch_add(1, Ordering::SeqCst);
        Live { folder }
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        let sig = pending();
        if sig.is_some() {
            let _ = std::fs::remove_dir_all(&self.folder);
        }
        LIVE.fetch_sub(1, Ordering::SeqCst);
        if let Some(sig) = sig {
            end(sig);
        }
    }
}

/// After a step has been killed and its call folder removed: waits for the other steps to do the
/// same, then ends the process by `sig`.
fn end(sig: i32) -> ! {
    let start = Instant::now();
    while LIVE.load(Ordering::SeqCst) > 0 && start.elapsed() < CLEANUP_WAIT {
        std::thread::sleep(Duration::from_millis(5));
    }
    die_by(sig);
    // Not reached: the default action of `SIGINT` and `SIGTERM` ends the process.
    std::process::exit(128 + sig)
}
