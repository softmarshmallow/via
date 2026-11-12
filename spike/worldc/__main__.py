"""worldc: compile a world document into what an engine loads.

    python -m worldc WORLD.toml [--pack PACK.toml] [--overlay FILE]... [--variant NAME]... [--out NAME]
                                [--lint reach]... [--map]

This module is the command line only: it reads the arguments, calls the pipeline and prints the report.
"""
from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

from .diagnostics import Diagnostics
from .lints import LINTS
from .pipeline import compile_world

SPIKE = Path(__file__).resolve().parent.parent


def main() -> int:
    ap = argparse.ArgumentParser(prog="worldc")
    ap.add_argument("world", type=Path)
    ap.add_argument("--pack", type=Path)
    ap.add_argument("--overlay", type=Path, action="append", default=[])
    ap.add_argument("--variant", action="append", default=[])
    ap.add_argument("--out")
    ap.add_argument("--lint", action="append", default=[], help="on-demand checks: " + ", ".join(sorted(LINTS)))
    ap.add_argument("--map", action="store_true", help="also emit the player-facing map's layers")
    args = ap.parse_args()
    unknown = [name for name in args.lint if name not in LINTS]
    if unknown:
        ap.error(f"unknown lint {', '.join(repr(n) for n in unknown)}; available: {', '.join(sorted(LINTS))}")

    started = time.time()
    name = args.out or "_".join([args.world.parent.name] + [p.stem for p in args.overlay] + args.variant)
    out = SPIKE / "build" / name
    done = compile_world(args.world, overlays=args.overlay, variants=args.variant, pack_path=args.pack,
                         lints=args.lint, out=out, with_map=args.map)
    if done.budget is not None:      # nothing was emitted if the world lacks a required field
        total, grid = done.budget["total"], done.world.grid
        print(f"{out.relative_to(SPIKE)}: {grid.nx}x{grid.ny} posts, {len(done.world.materials)} ground materials, "
              f"{total['instances']:,} instances ({len(done.explicit)} explicit), {total['source_tris']:,} source triangles")
    _report(done.diag, started)
    return 1 if done.diag.count("error") else 0


def _report(diag: Diagnostics, started: float) -> None:
    for item in diag.items:
        if item["severity"] == "info":
            continue
        where = f" at ({item['where'][0]:g}, {item['where'][1]:g})" if "where" in item else ""
        print(f"  {item['severity']:7s} {item['code']:22s} {item['subject']}{where}: {item['message']}")
        if "hint" in item:
            print(f"  {'':7s} {'':22s} hint: {item['hint']}")
    print(f"{diag.count('error')} errors, {diag.count('warning')} warnings, {time.time() - started:.1f} s")


if __name__ == "__main__":
    sys.exit(main())
