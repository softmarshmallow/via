//! A step's process tree, killed whole (`spec/protocol.md` 6.2).
//!
//! The protocol never uses a signal to ask a step anything. A worker is asked to stop by message;
//! otherwise its whole tree is killed at once, and dropping a `Tree` kills it, so no path out of
//! the engine (an early return, a panic unwinding) leaves a step running.
//!
//! | System | Start | Kill |
//! |---|---|---|
//! | Linux | Own process group; `PR_SET_PDEATHSIG(SIGKILL)` and a `getppid` check in `pre_exec`, from one long-lived spawning thread | `SIGKILL` to the group |
//! | macOS | Own process group (no parent-death signal: a worker ends when its protocol input ends) | `SIGKILL` to the group |
//! | Windows | Suspended, in a new process group with no console window, assigned to a kill-on-close Job Object, then resumed | `TerminateJobObject` |
//!
//! **When the group is killed.** On Linux and macOS the group id is the root's pid, and it stays
//! reserved only while some member of the group lives. So the group is killed once more at the
//! moment the root is reaped (by `wait`, `try_wait` or `kill`), which reaches any child the root
//! left behind, and never again after that: a later `kill` or drop could otherwise signal an
//! unrelated group that has since taken the same id. What remains is a window of one system call:
//! if the root was the group's last member, its id is free between the reap and that last
//! `killpg`, and a process started in exactly that window with the same id as its group would be
//! signalled. A pidfd (Linux) or a subreaper would close it; that is with the Open row on
//! grandchildren in `protocol.md`.

use std::io;
use std::process::{Child, Command, ExitStatus};

/// A started process and everything it started that stayed in its group or job.
pub struct Tree {
    child: Child,
    #[cfg(windows)]
    job: win::Job,
    reaped: Option<ExitStatus>,
}

impl Tree {
    /// Starts `cmd` as the root of a tree of its own.
    pub fn spawn(cmd: Command) -> io::Result<Tree> {
        imp::spawn(cmd)
    }

    pub fn id(&self) -> u32 {
        self.child.id()
    }

    pub fn child_mut(&mut self) -> &mut Child {
        &mut self.child
    }

    /// Kills the whole tree and reaps the root. Idempotent: once the root is reaped the tree was
    /// killed at that moment, and is not signalled again (see the module notes).
    pub fn kill(&mut self) -> io::Result<ExitStatus> {
        if let Some(s) = self.reaped {
            return Ok(s);
        }
        imp::kill_tree(self)?;
        // The group was signalled just now, so the reap need not signal it again.
        let s = self.child.wait()?;
        self.reaped = Some(s);
        Ok(s)
    }

    /// Waits for the root to exit. Grandchildren are reaped by the system once orphaned.
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        if let Some(s) = self.reaped {
            return Ok(s);
        }
        let s = self.child.wait()?;
        self.reaped(s);
        Ok(s)
    }

    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        if let Some(s) = self.reaped {
            return Ok(Some(s));
        }
        let s = self.child.try_wait()?;
        if let Some(s) = s {
            self.reaped(s);
        }
        Ok(s)
    }

    /// The root has just been reaped: kill what it left in its group now, the last time the
    /// group id is known to be ours.
    fn reaped(&mut self, s: ExitStatus) {
        let _ = imp::kill_tree(self);
        self.reaped = Some(s);
    }
}

impl Drop for Tree {
    /// Kill on drop: a step never outlives the engine's handle on it. A root already reaped had
    /// its group killed then, so a child it left behind is gone too.
    fn drop(&mut self) {
        let _ = self.kill();
    }
}

#[cfg(unix)]
mod imp {
    use super::Tree;
    use std::io;
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    pub fn spawn(mut cmd: Command) -> io::Result<Tree> {
        // Its own group, led by the child, so one `killpg` reaches everything that stayed in it and
        // a terminal's Ctrl+C reaches the engine, not the step.
        cmd.process_group(0);
        #[cfg(target_os = "linux")]
        {
            let parent = std::process::id() as libc::pid_t;
            // SAFETY: only async-signal-safe calls between fork and exec.
            unsafe {
                cmd.pre_exec(move || {
                    if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) == -1 {
                        return Err(io::Error::last_os_error());
                    }
                    // The engine died between fork and prctl: the signal will never come.
                    if libc::getppid() != parent {
                        libc::_exit(1);
                    }
                    Ok(())
                });
            }
            return linux_spawner::spawn(cmd);
        }
        #[cfg(not(target_os = "linux"))]
        {
            let child = cmd.spawn()?;
            Ok(Tree {
                child,
                reaped: None,
            })
        }
    }

    pub fn kill_tree(tree: &mut Tree) -> io::Result<()> {
        let pgid = tree.child.id() as libc::pid_t;
        // A group id is not reused while any member of the group lives, so this reaches only our
        // tree even after the root has exited, as long as some member is left to kill.
        // SAFETY: plain system call with no pointers.
        let r = unsafe { libc::killpg(pgid, libc::SIGKILL) };
        if r == -1 {
            let e = io::Error::last_os_error();
            // ESRCH: nothing left in the group. EPERM on macOS: only zombies are left.
            if !matches!(e.raw_os_error(), Some(libc::ESRCH) | Some(libc::EPERM)) {
                return Err(e);
            }
        }
        Ok(())
    }

    /// The parent-death signal fires when the thread that forked ends, not the process (6.2), so
    /// every step is forked from one thread that lives as long as the engine.
    #[cfg(target_os = "linux")]
    mod linux_spawner {
        use super::Tree;
        use std::io;
        use std::process::{Child, Command};
        use std::sync::mpsc::{channel, Sender};
        use std::sync::{Mutex, OnceLock};

        type Job = (Command, Sender<io::Result<Child>>);
        static SPAWNER: OnceLock<Mutex<Sender<Job>>> = OnceLock::new();

        pub fn spawn(cmd: Command) -> io::Result<Tree> {
            let tx = SPAWNER.get_or_init(|| {
                let (tx, rx) = channel::<Job>();
                std::thread::Builder::new()
                    .name("via-spawner".into())
                    .spawn(move || {
                        for (mut cmd, reply) in rx {
                            let _ = reply.send(cmd.spawn());
                        }
                    })
                    .expect("the spawner thread starts");
                Mutex::new(tx)
            });
            let (rtx, rrx) = channel();
            tx.lock()
                .unwrap_or_else(|p| p.into_inner())
                .send((cmd, rtx))
                .map_err(|_| io::Error::other("spawner thread is gone"))?;
            let child = rrx
                .recv()
                .map_err(|_| io::Error::other("spawner thread is gone"))??;
            Ok(Tree {
                child,
                reaped: None,
            })
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::{win, Tree};
    use std::io;
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    use windows_sys::Win32::System::Threading::{
        CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW, CREATE_SUSPENDED,
    };

    pub fn spawn(mut cmd: Command) -> io::Result<Tree> {
        // Suspended, so it cannot start a child before it is in the job. No console window: an
        // engine started by an editor plugin has no console, and each worker would open one. The
        // flag does not touch piped standard handles or the job.
        cmd.creation_flags(CREATE_SUSPENDED | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
        let job = win::Job::new()?;
        let mut child = cmd.spawn()?;
        if let Err(e) = job.assign(&child).and_then(|()| win::resume(child.id())) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
        Ok(Tree {
            child,
            job,
            reaped: None,
        })
    }

    pub fn kill_tree(tree: &mut Tree) -> io::Result<()> {
        tree.job.terminate()
    }
}

/// The Windows code path: written to the chapter, not yet run (no Windows runner until a push is
/// allowed; `protocol.md`, Open).
#[cfg(windows)]
mod win {
    use std::io;
    use std::os::windows::io::AsRawHandle;
    use std::process::Child;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
    };
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

    /// A Job Object with kill-on-close and no breakaway: when the engine dies, its handle closes
    /// and the whole tree dies with it.
    pub struct Job(HANDLE);

    // SAFETY: a job handle may be used from any thread.
    unsafe impl Send for Job {}

    impl Job {
        pub fn new() -> io::Result<Job> {
            // SAFETY: null attributes and name are allowed; the struct is plain data.
            unsafe {
                let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if h.is_null() {
                    return Err(io::Error::last_os_error());
                }
                let job = Job(h);
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                let ok = SetInformationJobObject(
                    job.0,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const core::ffi::c_void,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                if ok == 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(job)
            }
        }

        pub fn assign(&self, child: &Child) -> io::Result<()> {
            // SAFETY: both handles are live for the call.
            let ok = unsafe { AssignProcessToJobObject(self.0, child.as_raw_handle() as HANDLE) };
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }

        pub fn terminate(&self) -> io::Result<()> {
            // SAFETY: the handle is live until drop.
            let ok = unsafe { TerminateJobObject(self.0, 1) };
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            // SAFETY: closed once; with kill-on-close this also ends anything left in the job.
            unsafe { CloseHandle(self.0) };
        }
    }

    /// Resumes the suspended process's threads. `std::process` does not hand out the primary
    /// thread's handle, so its threads are found by snapshot; a suspended process has only one.
    pub fn resume(pid: u32) -> io::Result<()> {
        // SAFETY: the snapshot handle is closed on every path; the entry is plain data whose size
        // is set before use.
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
            if snap == INVALID_HANDLE_VALUE {
                return Err(io::Error::last_os_error());
            }
            let mut entry: THREADENTRY32 = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
            let mut resumed = 0;
            let mut more = Thread32First(snap, &mut entry) != 0;
            while more {
                if entry.th32OwnerProcessID == pid {
                    let t = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID);
                    if !t.is_null() {
                        if ResumeThread(t) != u32::MAX {
                            resumed += 1;
                        }
                        CloseHandle(t);
                    }
                }
                more = Thread32Next(snap, &mut entry) != 0;
            }
            CloseHandle(snap);
            if resumed == 0 {
                return Err(io::Error::other("no thread of the step could be resumed"));
            }
            Ok(())
        }
    }
}
