"""Diagnostics: what the compiler has to say about a world, and the registry of everything it can say.

Owns the `Diagnostics` collector and `CODES`, the one list of every diagnostic code in the package
with a plain description of when it fires. A code that is not in `CODES` cannot be reported, so the
registry cannot drift from the code.
"""
from __future__ import annotations

CODES: dict[str, str] = {
    # the document and its layers
    "layer.direction": "An item in one layer refers to something defined in a higher layer.",
    # the asset pack
    "pack.missing_file": "The pack names a texture, model or recipe file that does not exist.",
    "pack.undeclared": "A recipe asset does not declare the `size` and `tris` the compiler cannot measure.",
    "asset.unknown": "An instance, kit run or scatter names an asset the pack does not have.",
    # features and frames
    "feature.no_id": "A feature has no id.",
    "feature.duplicate": "Two features share one id.",
    "feature.shape": "A feature has no path, region, ellipse or point to take its shape from.",
    "feature.outside": "A feature lies wholly outside the world extent.",
    "kind.passthrough": "A feature's kind is not declared, so the feature is passed through untouched.",
    "frame.unknown": "An `on`, `look` or `joins` names a feature that is not the path or region it needs.",
    # fields, expressions and effects
    "override.orphan": "An override names a produced thing that is no longer produced.",
    "pin.conflict": "A pinned thing now stands where a field its asset avoids says to keep clear.",
    "lock.taken": "A lock recorded what a rule placed inside it.",
    "lock.holds": "A rule would now place differently inside a lock; the record is kept.",
    "mount.unknown": "A path starts or ends at a mount that cannot be found.",
    "mount.unmet": "An asset's mount expects a path and none ends there.",
    "attr.invalid": "An attribute written as a formula could not be worked out.",
    "attr.cycle": "Attributes written as formulas depend on each other in a circle; the chain is named.",
    "expr.invalid": "An expression in a field, kind, scatter or volume could not be evaluated.",
    "field.unresolved": "A field the compiler needs could not be resolved, in a world or in one of its parts.",
    "field.unknown": "An expression or a flood reads a field that is not declared.",
    "field.cycle": "Fields read each other in a circle; reported once, with the chain of names.",
    "feature.unknown": "An expression measures from a feature that does not exist.",
    "field.required": "The world lacks a `height` or a `ground` field, so nothing can be emitted, or a seam part lacks a `ground` field.",
    "ground.too_many": "More than eight ground materials are in use after merging parts.",
    "ground.unknown": "The ground field paints a material the pack does not have.",
    "effect.unbounded": "An effect's mask has not fallen to zero at its reach, which leaves a visible edge.",
    "seam.change_exceeds": "An effect would move a field by more than the limit its kind declares.",
    "seam.grade": "A path is steeper after grading than the limit its kind declares.",
    # authority between features that claim the same field
    "authority.unknown": "A feature's authority is not one of derived, shaped or anchored.",
    "authority.conforms": "A derived feature's effect on the field it is derived from is not applied.",
    "authority.conflict": "Two anchored features claim different values of one field on the same ground.",
    "authority.squeezed": "The aprons of two anchored features meet with no free ground to take up their difference.",
    "authority.cycle": "An instance stands on a derived feature and its kind reshapes the field that feature is derived from.",
    # derived water
    "water.no_basin": "The ground at a flood point holds no water inside the fence.",
    "water.leaks": "At its level a flood runs past its fence and is cut off there.",
    "water.dry": "A flood or water body ends up with no water, or with no closed shore.",
    # placing instances
    "instance.no_position": "An instance has neither `at` nor `on`.",
    "instance.outside": "An instance stands outside the world extent.",
    "set.unknown": "An instance names a set that is not declared.",
    "kit.misfit": "A kit run's length is not a whole number of bays within its tolerance.",
    "kit.module": "The pieces named for one kit run do not share a module.",
    "kit.remainder": "A room's walls do not divide into its kind's wall module.",
    "dress.no_host": "A dress row names a host that is not a placed instance.",
    "flank.no_host": "A flank row names a host that is not a placed instance.",
    "place.no_host": "`atop` or `beside` names something that is not an instance placed before it.",
    "place.off_surface": "An instance placed `atop` another is past the edge of that surface.",
    "place.overlap": "The footprints of two solid instances overlap.",
    "place.avoid": "An instance stands where a field its asset avoids says to keep clear.",
    "seam.relief": "The ground under an instance's footprint varies by more than its asset tolerates.",
    "seam.junction": "A path declares it joins another path but neither of its ends is on it.",
    "scatter.count": "How many instances a scatter produced.",
    # interiors
    "room.no_level": "A room is not on a declared level.",
    "room.sealed": "No chain of doors and stairs joins a room to the outside.",
    "opening.no_wall": "A door or window is not `on` a room's outline.",
    "door.dangling": "A door leads to something that is not a room, or to a room it does not touch.",
    "seam.threshold": "An outside door's threshold is not at the height of the ground outside it.",
    "stair.dangling": "A stair does not name two rooms as its `from` and `to`.",
    "stair.flat": "A stair joins two rooms on the same level.",
    "stair.steep": "A stair's steps are too high or too short to walk.",
    "stair.through_wall": "Part of a stair lies outside both rooms it joins.",
    "place.no_room": "An instance says it is `in` something that is not a room.",
    "place.in_wall": "Part of an instance's footprint is outside the room it is in.",
    "place.no_ceiling": "An instance is hung from the ceiling but is in no room whose level has a height.",
    "place.over_opening": "A wall-mounted instance hangs across a door or window.",
    "place.on_stair": "An instance stands on a stair or over its well.",
    "place.blocks_door": "A solid instance stands in the swing of a door.",
    # volumes
    "volume.outside": "A 3D path leaves the volume it carves; nothing is carved outside a volume.",
    "volume.floor": "Air reaches the bottom of a volume's z range.",
    "volume.ceiling": "Solid reaches the top of a volume's z range.",
    # parts
    "part.no_door": "A portal part names a door the part does not have.",
    "part.outside": "A part's footprint does not fit inside the master's extent.",
    "part.ground": "The ground under a portal part varies by more than the part allows.",
    "part.seam": "At its edge a seam part stands further from the land it joins than its limit.",
    "part.volume_apron": "A volume of a seam part reaches into the part's apron, where the blended ground does not meet its mesh.",
    "part.blend": "A part's `blend` expression could not be worked out; the default falloff was used.",
    "part.named_in_apron": "A place the master names in a part lies in that part's apron, where the ground is blended.",
    "part.overruled": "A master feature laid after the parts moves ground that a part had anchored.",
    "part.unknown": "An expression reads `inside()` or `outside()` of a part that is not joined, or reads it too early.",
    "exit.unanswered": "A part declares an exit, and no path of the master starts or ends there.",
    "feature.where_empty": "A region given by `where` holds nowhere in the world.",
    "part.context": "A part that expects land all round has part of its edge under water.",
    # what leaves the compiler
    "bookmark.blocked": "Terrain hides a bookmark's target from its eye.",
    "bookmark.crowded": "Instances stand in the first metres of a bookmark's view.",
    "bookmark.inside": "A bookmark's eye is inside the footprint of an instance.",
    "budget.exceeded": "A chunk holds more instances or source triangles than the declared budget.",
    # ways that leave the ground
    "deck.clearance": "A deck leaves less room over something it says it passes over than it needs.",
    "deck.detached": "A deck's end is said to rest on another way but lies off it.",
    "deck.grade": "A deck climbs more steeply than the limit its kind declares.",
    "deck.misses": "A deck says it passes over a feature that it never crosses.",
    # user code joining the build
    "field.file": "A field's file is missing, is not a picture that can be read, or does not hold one value per post.",
    "field.image_coarse": "An 8-bit picture is stretched over a range wide enough that its 256 levels show as steps.",
    "step.point": "A step names a point of the build that user code cannot join.",
    "step.after": "A generate step runs after a layer the world does not have.",
    "step.failed": "A step's command ended with an error; its last lines are quoted.",
    "step.no_output": "A generate step ended without writing a layer.",
    "step.ran": "A step ran, or its recorded result was reused because nothing it reads has changed.",
    # the reach lint
    "reach.nothing_expected": "The reach lint was asked for but the world has no [[expect]] rows.",
    "reach.no_profile": "An expectation names a profile that does not declare max_slope_deg.",
    "reach.no_start": "An expectation's `from` is not a place its profile can stand on.",
    "reach.unknown": "An expectation's `to` names neither a point feature nor an instance.",
    "reach.cut_off": "A target expected to be reachable has no route over the ground from the start.",
    "reach.not_sealed": "A target expected to be out of reach has a route over the ground.",
}


class Diagnostics:
    def __init__(self) -> None:
        self.items: list[dict] = []

    def add(self, severity: str, code: str, subject: str, message: str, where=None, hint: str = "") -> None:
        if code not in CODES:
            raise KeyError(f"diagnostic code '{code}' is not registered in worldc.diagnostics.CODES; "
                           "add it there with a one-line description of when it fires")
        item = {"severity": severity, "code": code, "subject": subject, "message": message}
        if where is not None:
            item["where"] = [round(float(where[0]), 1), round(float(where[1]), 1)]
        if hint:
            item["hint"] = hint
        self.items.append(item)

    def external(self, step: str, item: dict) -> None:
        """What a step's own code reported. Its codes are its own, so they carry the step's name
        and are not in the registry; everything else about the row is held to the same form."""
        severity = item.get("severity") if item.get("severity") in ("error", "warning", "info") else "warning"
        row = {"severity": severity, "code": f"{step}:{item.get('code', 'note')}", "subject": str(item.get("subject", step)),
               "message": str(item.get("message", ""))}
        if isinstance(item.get("where"), (list, tuple)) and len(item["where"]) >= 2:
            row["where"] = [round(float(item["where"][0]), 1), round(float(item["where"][1]), 1)]
        if item.get("hint"):
            row["hint"] = str(item["hint"])
        self.items.append(row)

    def error(self, *a, **k) -> None:
        self.add("error", *a, **k)

    def warn(self, *a, **k) -> None:
        self.add("warning", *a, **k)

    def info(self, *a, **k) -> None:
        self.add("info", *a, **k)

    def count(self, severity: str) -> int:
        return sum(1 for i in self.items if i["severity"] == severity)

    def adopt(self, other: "Diagnostics", prefix: str) -> None:
        """Take over another compile's errors and warnings, with its name on their subjects. Their
        positions are in that compile's own frame, so they are dropped."""
        for item in other.items:
            if item["severity"] != "info":
                self.items.append({k: v for k, v in {**item, "subject": f"{prefix}.{item['subject']}"}.items() if k != "where"})
