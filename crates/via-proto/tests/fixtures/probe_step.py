"""The M0 probe's step (spec/protocol.md, conformance case 0). One script, several modes by param.

- field:  read `height` by memmap, return its negation (-0.0 must come back +0.0) and an
          unchanged copy, and make one call back (expr.eval) whose column is checked here.
- calls:  time `n` empty batch calls (expr.eval with zero-length columns), worker side.
- maps:   time `n` blob maps: a build.field round trip plus numpy.load(mmap_mode="r") and one read;
          and, apart, `n` bare maps of the same file.
- hang:   start a child that stays in the process group, write both process ids, then wait to be
          killed by the harness mid-call.
- spin:   make empty calls until cancelled.
- print:  print to standard output and run a `cat` (stdin copied to stdout, written in Python so it
          exists on Windows too) with no stdin argument; neither may touch the wire.

On Windows a venv's python.exe is a launcher that runs the base interpreter as its child, so the
pid `hang` writes is that child's, not the engine's tree root.
"""

import os
import subprocess
import sys
import time

import numpy as np

print("probe step imported", flush=True)  # reaches the log, never the wire


def generate(step):
    mode = step.params["mode"]
    out = step.out.folder
    if mode == "field":
        (h,) = step.fields(["height"])
        assert isinstance(h, np.memmap) and not h.flags.writeable, "mapped read-only"
        assert h.shape == (step.grid["ny"], step.grid["nx"]) and h.dtype == np.float32
        step.out.field("neg", -h)
        step.out.field("same", h)
        x = np.array([0, 1000, 2500], dtype=np.int32)
        y = np.array([0, 0, 500], dtype=np.int32)
        col = step.eval("x + y", x=x, y=y)
        assert col.dtype == np.float64 and col.tolist() == [0.0, 1.0, 3.0], col
    elif mode == "calls":
        n = int(step.params["n"])
        empty = np.zeros(0, dtype=np.int32)
        t = np.empty(n, dtype=np.float64)
        for k in range(n):
            t0 = time.perf_counter_ns()
            step.eval("x + y", x=empty, y=empty)
            t[k] = time.perf_counter_ns() - t0
        t.tofile(os.path.join(out, "calls.f64"))
    elif mode == "maps":
        n = int(step.params["n"])
        rt = np.empty(n, dtype=np.float64)
        bare = np.empty(n, dtype=np.float64)
        acc = 0.0
        for k in range(n):
            t0 = time.perf_counter_ns()
            (h,) = step.fields(["height"])
            acc += float(h[k % h.shape[0], 0])
            rt[k] = time.perf_counter_ns() - t0
            del h
        path = step.call("build.field", fields=["height"])["height"]["path"]
        for k in range(n):
            t0 = time.perf_counter_ns()
            h = np.load(path, mmap_mode="r")
            acc += float(h[k % h.shape[0], 0])
            bare[k] = time.perf_counter_ns() - t0
            del h
        rt.tofile(os.path.join(out, "maps.f64"))
        bare.tofile(os.path.join(out, "bare_maps.f64"))
        print(f"checksum {acc}")
    elif mode == "hang":
        child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(600)"])
        with open(os.path.join(step.params["pids"]), "w") as f:
            f.write(f"{os.getpid()} {child.pid}\n")
        time.sleep(600)
    elif mode == "spin":
        empty = np.zeros(0, dtype=np.int32)
        while True:
            step.eval("x + y", x=empty, y=empty)
    elif mode == "print":
        print("hello from the step")
        os.write(1, b"raw write to descriptor 1\n")
        cat = [sys.executable, "-c", "import sys; sys.stdout.buffer.write(sys.stdin.buffer.read())"]
        got = subprocess.run(cat, stdout=subprocess.PIPE, timeout=10)
        assert got.stdout == b"", got.stdout
    elif mode == "raise":
        raise ValueError("as asked")
    else:
        raise KeyError(mode)
