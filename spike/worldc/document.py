"""The source document: loading a world's TOML and merging everything that is laid over it.

Owns `deep_merge`, `load_doc` (the includes of the world and of its layers, layers, overlays,
variants) and `check_layers`, the one check that reads the document alone. Nothing here knows about grids, fields or assets.
"""
from __future__ import annotations

import json
import tomllib
from pathlib import Path

from .diagnostics import Diagnostics


def deep_merge(base: dict, over: dict) -> dict:
    """Overlay `over` on `base`. Tables merge; lists of tables that carry `id` merge by id; else replace."""
    for key, val in over.items():
        cur = base.get(key)
        if isinstance(val, dict) and isinstance(cur, dict):
            deep_merge(cur, val)
        elif (isinstance(val, list) and isinstance(cur, list) and val
              and all(isinstance(i, dict) and "id" in i for i in val)):
            by_id = {i.get("id"): i for i in cur if isinstance(i, dict)}
            for item in val:
                if item.get("remove"):
                    cur[:] = [e for e in cur if e.get("id") != item["id"]]
                elif item["id"] in by_id:
                    deep_merge(by_id[item["id"]], item)
                else:
                    cur.append(item)
        else:
            base[key] = val
    return base


def _read(file: Path) -> dict:
    """A document is TOML when a person writes it and may be JSON when a program does."""
    return json.loads(file.read_text()) if file.suffix == ".json" else tomllib.loads(file.read_text())


def _anchor(part: dict, home: Path) -> None:
    """A field read from a file names it relative to the document that says so."""
    for spec in part.get("field", {}).values():
        if isinstance(spec, dict) and "file" in spec:
            spec["file"] = str((home / spec["file"]).resolve())


def load_doc(path: Path, overlays: list[Path], variants: list[str], *, generated: list | tuple = (),
             upto: str | None = None) -> dict:
    """One merged document: the includes, the world's own text, its layers, then the overlays and the
    named variants, each over what came before.

    The world's file and every layer file may `include` shared libraries of kinds, each with paths
    relative to the file that names them. All of them go in first, the world's before the layers'
    and a file named twice only once, so whatever a world or a layer writes itself wins."""
    own = tomllib.loads(path.read_text())
    _anchor(own, path.parent)
    layers = [(layer["id"], path.parent / layer["file"]) for layer in own.get("layer", [])]
    # What a generate step wrote is a layer like any other: lowest of all when it ran before the
    # world was compiled, else directly above the layer it ran after. `upto` stops at a layer, which
    # is how the layers below a step are compiled alone to be handed to it.
    for gid, gfile, after in generated:
        at = 0 if after is None else next(n for n, (lid, _) in enumerate(layers) if lid == after) + 1
        while after is None and at < len(layers) and layers[at][0] in {g[0] for g in generated if g[2] is None}:
            at += 1
        layers.insert(at, (gid, Path(gfile)))
    if upto is not None:
        layers = layers[:next(n for n, (lid, _) in enumerate(layers) if lid == upto) + 1]
    texts = {lid: _read(file) for lid, file in layers}
    for lid, file in layers:
        _anchor(texts[lid], file.parent)
    doc: dict = {}
    included: set = set()
    for home, incs in [(path.parent, own.get("include", []))] + [(file.parent, texts[lid].pop("include", [])) for lid, file in layers]:
        for inc in incs:
            lib = (home / inc).resolve()
            if lib not in included:
                included.add(lib)
                deep_merge(doc, tomllib.loads(lib.read_text()))
    deep_merge(doc, own)
    # layers: separately owned documents for the same world, lowest first. Their lists are joined,
    # not overridden, and every item remembers the layer it came from.
    doc["_layers"] = []
    seen: dict = {}
    for lid, _ in layers:
        doc["_layers"].append(lid)
        part = texts[lid]
        for key, val in part.items():
            if isinstance(val, list) and all(isinstance(i, dict) for i in val):
                for item in val:
                    item["_layer"] = lid
                    if "id" in item:
                        if (key, item["id"]) in seen:
                            raise SystemExit(f"layer '{lid}' defines {key} '{item['id']}', already defined in layer '{seen[(key, item['id'])]}'")
                        seen[(key, item["id"])] = lid
                doc.setdefault(key, []).extend(val)
            elif isinstance(val, dict) and isinstance(doc.get(key), dict):
                deep_merge(doc[key], val)
            else:
                doc[key] = val
    for over in overlays:
        deep_merge(doc, tomllib.loads(over.read_text()))
    for name in variants:
        if name not in doc.get("variant", {}):
            raise SystemExit(f"unknown variant '{name}'")
        deep_merge(doc, doc["variant"][name])
    return doc


def check_layers(doc: dict, diag: Diagnostics) -> None:
    """A layer may refer to things in its own layer or a lower one, never a higher one."""
    order = {lid: n for n, lid in enumerate(doc.get("_layers", []))}
    if not order:
        return
    home: dict = {}
    for key in ("feature", "instance", "run", "kitrun", "dress", "flank", "scatter"):
        for item in doc.get(key, []):
            if "id" in item and "_layer" in item:
                home[item["id"]] = item["_layer"]

    def owner(ref):
        return home.get(ref) or home.get(str(ref).split(".")[0])      # a run's item belongs to its run

    for key in ("feature", "instance", "run", "kitrun", "dress", "flank", "expect"):
        for item in doc.get(key, []):
            mine = item.get("_layer")
            if mine is None:
                continue
            refs = [item.get("beside"), item.get("atop"), item.get("in"), item.get("from") if key == "expect" else None]
            on = item.get("on")
            refs.append(on.get("feature") if isinstance(on, dict) else on)
            look = item.get("look")
            refs.append(look.get("feature") if isinstance(look, dict) else None)
            for group in (item.get("joins", []), item.get("hosts", []), item.get("to", []) if key == "expect" else []):
                refs.extend(group if isinstance(group, list) else [])
            for key2 in ("starts_at", "ends_at"):      # a path that meets a mount refers to the mount's owner
                if isinstance(item.get(key2), str):
                    refs.append(item[key2].rpartition(".")[0])
            for ref in refs:
                theirs = owner(ref) if isinstance(ref, str) else None
                if theirs and order[theirs] > order[mine]:
                    diag.error("layer.direction", str(item.get("id", key)),
                               f"it is in layer '{mine}' and refers to '{ref}' in the higher layer '{theirs}'",
                               hint="move it up a layer, or move what it refers to down")
