"""The pipeline: the one order in which a world is compiled, from its source file to its outputs.

Owns `compile_world` and the `Compiled` result it returns. The command line calls it for the world
it was given, and `parts` calls it for every part that world brings in, so there is one pipeline
and both see the same steps in the same order.
"""
from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

from .assets import load_pack
from .checks import check_explicit, check_joins
from .decks import build_decks
from .diagnostics import Diagnostics
from .document import check_layers, load_doc
from .interior import build_interiors
from .lints import LINTS
from .outputs import bookmarks, budgets, emit, water_bodies
from .parts import take_given, apply_fields, apply_height, apply_late, bring, check_exits, check_named, load_parts, part_exits, part_mounts, part_paths
from .placement import finish, place_explicit
from .playmap import emit_map
from .producers import dress, expand, scatter
from .steps import run_generate, run_lints, steps_of
from .volume import build_volumes
from .world import World


@dataclass
class Compiled:
    """What one compile leaves behind. `explicit` are the hand-placed instances of this world alone;
    `instances` is everything placed, scattered and brought in by parts. `marks`, `budget` and
    `water` are the bookmarks, the per-chunk budgets and the water measurements of a master that
    reached the end; they stay unset for a part and for a world that lacks a required field. `sound` is false for a part one of whose fields did not resolve: its
    diagnostics say why, and the rest of it is not to be used."""
    doc: dict
    world: World
    diag: Diagnostics
    explicit: list[dict]
    instances: list[dict]
    volumes: list[dict]
    built: dict
    marks: list | None = None
    budget: dict | None = None
    water: list | None = None
    sound: bool = True


def compile_world(path: Path, *, overlays: list[Path] | tuple = (), variants: list[str] | tuple = (),
                  pack_path: Path | None = None, as_part: bool = False, lints: list[str] | tuple = (),
                  out: Path | None = None, with_map: bool = False, generated: list | None = None,
                  upto: str | None = None, given: dict | None = None) -> Compiled:
    """Compile the world at `path`. With `out`, also run the named lints and write every output file
    there (and the player-facing map's layers under it, `with_map`). `pack_path` overrides the pack
    the document names. Every diagnostic except a lint's is raised before the first file is written,
    so what the compiler says does not depend on whether it was asked to write.

    `as_part` compiles a world that is going to be placed inside another one. Such a world is compiled
    alone, so it differs from a master in exactly these ways:
      - its own `[[part]]` rows are not loaded: parts do not nest;
      - a field that cannot be resolved is reported as it is for a master, and the compile then
        stops with `sound` unset, because a part that did not fully resolve has nothing sound to
        hand to its master. The master puts the part's name on the diagnostic and leaves it out;
      - the checks on what is emitted are skipped (the required `height` and `ground` fields, the
        ground materials against the pack and the limit of eight, bookmarks, budgets and water
        bodies): the master makes them on the merged result.
    """
    unknown = [name for name in lints if name not in LINTS]
    if unknown:      # refused before any work is done: a misspelt lint would otherwise look like a clean run
        raise ValueError(f"unknown lint {', '.join(repr(n) for n in unknown)}; available: {', '.join(sorted(LINTS))}")
    doc = load_doc(path, list(overlays), list(variants))
    diag = Diagnostics()
    take_given(doc, given, diag)
    # The user's generate steps run first. Each writes a layer, and the document is then read again
    # with those layers in place. A compile that was handed its layers (`generated`) is the one made
    # beneath a step, and runs no steps of its own. A part runs its generate steps like any world;
    # its lint steps are not run, since it writes no build for them to read.
    own_steps = generated is None and bool(doc.get("step"))
    if own_steps:
        def compile_lower(layer: str, so_far: list, where: Path) -> Compiled:
            return compile_world(path, overlays=overlays, variants=variants, pack_path=pack_path,
                                 out=where, generated=so_far, upto=layer, given=given)
        generated = run_generate(path, doc, diag, compile_lower)
    if generated or upto:
        doc = load_doc(path, list(overlays), list(variants), generated=generated or (), upto=upto)
        take_given(doc, given, Diagnostics())      # the same document read again: what there was to say has been said
    pack_path = (pack_path or (path.parent / doc["world"]["pack"])).resolve()
    pack = load_pack(pack_path, diag)
    check_layers(doc, diag)
    held = Diagnostics()         # parts are compiled first, since the master may name places in them; what they report
    parts = [] if as_part else load_parts(doc, path, pack_path, held, pack)      # is still listed after the master's own
    exits = part_exits(parts)
    world = World(doc, pack, diag, outer=part_mounts(parts), outer_paths=part_paths(parts), outer_faces={r: e["faces"] for r, e in exits.items()})
    check_exits(world, exits)
    diag.items.extend(held.items)
    world.lock_dir = path.parent / "locks"
    done = Compiled(doc, world, diag, explicit=[], instances=[], volumes=[], built={})

    unresolved: set = set()      # fields already reported, so one that fails twice is said once
    specs = expand(world)
    explicit = done.explicit = place_explicit(world, specs)
    try:
        world.field("height")
        apply_height(world, parts)                # seam parts bring their own ground
        check_named(world, parts)
        apply_late(world, parts, ["height"])      # the master's own word on the joined ground
        done.volumes = build_volumes(world)       # rewrites `height` with the top of each volume
        world.derive()
    except NameError as exc:
        diag.error("field.unresolved", "height", str(exc))
        unresolved.add("height")
    explicit += place_explicit(world, specs, deferred=True)
    explicit += dress(world, explicit)
    for name, fspec in doc.get("field", {}).items():
        if fspec.get("type") == "volume" or name in unresolved:
            continue
        try:
            world.field(name)
        except NameError as exc:
            diag.error("field.unresolved", name, str(exc))
            unresolved.add(name)
    if as_part and unresolved:
        done.sound = False
        return done
    if not as_part:
        if "height" not in world.fields or "ground" not in world.fields:
            diag.error("field.required", "world", "the spike viewer needs a `height` and a `ground` field")
            return done
        apply_fields(world, parts)
        _check_ground(world)
    apply_late(world, parts, [n for n in world.fields if n not in ("height", "bed", "hole")])
    scattered = scatter(world, explicit)
    check_explicit(world, explicit)
    check_joins(world)
    instances = done.instances = explicit + scattered
    finish(world, instances)

    built = done.built = build_interiors(world, explicit)
    built.setdefault("meshes", []).extend(build_decks(world))      # ways that leave the ground
    brought = bring(world, parts, built, done.volumes)
    finish(world, brought)
    instances += brought
    # A bookmark in a room stands on that room's floor. A part does this for its own rooms, in its
    # own frame, and `bring` lifts the result; what a part brought is therefore left alone here,
    # because its levels are not this document's.
    for feat in world.features:
        if "part" not in feat and feat.get("in") in built.get("rooms", {}):
            lv = next(l for l in doc["level"] if l["id"] == built["rooms"][feat["in"]]["level"])
            feat["floor_z"] = float(lv["z"])

    if not as_part:      # checked here, not while writing: bookmarks, then budgets, then water bodies
        done.marks = bookmarks(world, instances)
        done.budget = budgets(world, instances)
        done.water = water_bodies(world)
    if out is not None:
        for name in dict.fromkeys(lints):      # each lint once, in the order asked for
            lint, picture = LINTS[name]
            out.mkdir(parents=True, exist_ok=True)
            lint(world, instances, out / picture)
        emit(world, instances, out, done.volumes, built, done.marks, done.budget, done.water)
        if with_map:
            emit_map(world, instances, out / "map")
        if own_steps and any(s["at"] == "lint" for s in steps_of(doc, Diagnostics())):
            run_lints(path, doc, diag, out)      # they read the files just written, and add to what was said
            (out / "diagnostics.json").write_text(json.dumps(diag.items, indent=1))
    return done


def _check_ground(world: World) -> None:
    """The ground materials in use, against what the compiled form and the pack can carry."""
    diag, pack = world.diag, world.pack
    if len(world.materials) > 8:      # the compiled form carries eight ground materials
        diag.error("ground.too_many", "world", f"{len(world.materials)} ground materials after merging parts; the output holds 8: "
                   + ", ".join(world.materials), hint="share materials between the master and its parts, or drop one")
    for name in world.materials:
        if name not in pack["ground"]:
            diag.error("ground.unknown", name, f"the pack has no ground material '{name}'", hint=f"pack: {pack['path']}")
