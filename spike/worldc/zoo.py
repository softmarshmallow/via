"""Zoo: look at a pack by itself. Lists its assets with their facts, filters them, and lays the
matches out as a world, so that any loader can show them. It owns no renderer.

    python -m worldc.zoo PACK.toml [--match TEXT] [--taxon PATH] [--label L] [--tag TAG] [--tree] [--where "module_m == 5"] [--json] [--world NAME]
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

from .assets import load_pack
from .diagnostics import Diagnostics

SPIKE = Path(__file__).resolve().parent.parent


def kind_of(a: dict) -> str:
    return "model" if "source" in a else "recipe" if "recipe" in a else "primitive"


def main() -> int:
    ap = argparse.ArgumentParser(prog="worldc.zoo")
    ap.add_argument("pack", type=Path)
    ap.add_argument("--match", default="", help="text the asset id must contain")
    ap.add_argument("--tag", action="append", default=[])
    ap.add_argument("--where", default="", help="an expression over an asset's facts, e.g. \"module_m == 5 and solid\"")
    ap.add_argument("--taxon", default="", help="a place in the pack's taxonomy, e.g. building/row")
    ap.add_argument("--label", action="append", default=[])
    ap.add_argument("--tree", action="store_true", help="print the taxonomy with a count at every level")
    ap.add_argument("--builds", nargs="*", default=None, help="builds to count use in (names under build/); none named means every build")
    ap.add_argument("--unused", action="store_true", help="only assets no counted build places")
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--world", help="write worlds/NAME/world.toml with the matches laid out on a grid")
    args = ap.parse_args()
    diag = Diagnostics()
    pack = load_pack(args.pack.resolve(), diag)
    used: dict = {}
    counted = []
    if args.builds is not None or args.unused:      # use is read from what was built, so rules and scatters count too
        names = args.builds or sorted(p.name for p in (SPIKE / "build").iterdir() if (p / "world.json").is_file() and not p.name.startswith("."))
        for name in names:
            manifest = SPIKE / "build" / name / "world.json"
            if not manifest.is_file():
                print(f"  no build named {name}", file=sys.stderr)
                continue
            built = json.loads(manifest.read_text())
            if Path(built.get("pack", args.pack)).name != args.pack.name and "pack" in built:
                continue
            counted.append(name)
            for aid, chunks in built["instances"].items():
                used.setdefault(aid, {})[name] = sum(len(c) for c in chunks.values())
    rows = []
    for aid, a in sorted(pack["asset"].items()):
        if args.unused and aid in used:
            continue
        facts = {"id": aid, "source": kind_of(a), "across": a["size"][0], "depth": a["size"][1], "height": a["size"][2],
                 "footprint": a["footprint"], "module_m": a.get("module_m"), "solid": a["solid"], "tris": a["tris"],
                 "tags": a["tags"], "taxon": a.get("taxon", ""), "labels": a.get("labels", []), "avoid": a["avoid"], "max_relief": a["max_relief"]}
        if args.taxon and not (facts["taxon"] == args.taxon or facts["taxon"].startswith(args.taxon + "/")):
            continue
        if any(l not in facts["labels"] for l in args.label):
            continue
        if args.match not in aid or any(t not in a["tags"] for t in args.tag):
            continue
        if args.where and not eval(args.where, {"__builtins__": {}}, dict(facts)):
            continue
        if counted:
            facts["placed"] = sum(used.get(aid, {}).values())
            facts["in_builds"] = sorted(used.get(aid, {}))
        rows.append(facts)
    for item in diag.items:
        print(f"  {item['severity']} {item['code']} {item['subject']}: {item['message']}", file=sys.stderr)
    if args.tree:
        counts: dict = {}
        for r in rows:
            parts = (r["taxon"] or "(unfiled)").split("/")
            for k in range(1, len(parts) + 1):
                counts["/".join(parts[:k])] = counts.get("/".join(parts[:k]), 0) + 1
        for path in sorted(counts):
            print(f"{'  ' * path.count('/')}{path.split('/')[-1]:24} {counts[path]:4}")
        return 0
    if args.json:
        print(json.dumps(rows, indent=1))
    else:
        print(f"{'id':28} {'source':9} {'across':>6} {'depth':>6} {'height':>6} {'module':>6} {'solid':5} {'tris':>8}  tags")
        for r in rows:
            print(f"{r['id']:28} {r['source']:9} {r['across']:6.1f} {r['depth']:6.1f} {r['height']:6.1f} "
                  f"{(r['module_m'] or 0):6.1f} {str(r['solid']):5} {r['tris']:8,}  {r['taxon']} {','.join(r['labels'] + r['tags'])}")
        print(f"{len(rows)} of {len(pack['asset'])} assets")
        if counted:
            dead = [r["id"] for r in rows if not r["placed"]]
            print(f"use counted in {len(counted)} builds: {', '.join(counted)}")
            print(f"placed somewhere: {len(rows) - len(dead)}; placed nowhere: {len(dead)}")
            for r in sorted(rows, key=lambda r: -r["placed"])[:8]:
                if r["placed"]:
                    print(f"  {r['id']:28} {r['placed']:6,}  in {', '.join(r['in_builds'])}")
    if args.world and rows:
        gap = max(max(r["across"], r["depth"]) for r in rows) + 4.0
        cols = math.ceil(math.sqrt(len(rows)))
        side = max(64.0, math.ceil((cols + 1) * gap / 32) * 32.0)
        doc = [f'# Zoo of {args.pack.name}: written by `python -m worldc.zoo`. Each asset stands with its front to the south.\n'
               f'include = ["../../kinds/review.toml"]\n\n[world]\nname = "Zoo"\nextent = [0.0, 0.0, {side}, {side}]\nresolution = 1.0\nchunk = 32\n'
               f'pack = {json.dumps(str(args.pack.resolve()))}\n\n[field.height]\nunit = "m"\nbase = 0.0\n\n[field.ground]\ntype = "material"\n'
               f'layers = [{{ paint = "{next(iter(pack["ground"]))}" }}]\n']
        for n, r in enumerate(rows):
            x, y = (n % cols + 1) * gap, side - (n // cols + 1) * gap
            doc.append(f'[[instance]]\nid = "z{n}"\nasset = {json.dumps(r["id"])}\nat = [{x}, {y}]\nyaw = 270.0\ngroup = "zoo"\n')
        doc.append(f'[[feature]]\nid = "bm_zoo"\nkind = "bookmark"\nat = [{side / 2}, {-side * 0.25}]\nlook = [{side / 2}, {side * 0.55}]\neye = {side * 0.45}\nnote = "The pack, laid out."\n')
        out = SPIKE / "worlds" / args.world
        out.mkdir(parents=True, exist_ok=True)
        (out / "world.toml").write_text("\n".join(doc))
        print(f"wrote {out.relative_to(SPIKE)}/world.toml: {len(rows)} assets on a {cols}-wide grid")
    return 0


if __name__ == "__main__":
    sys.exit(main())
