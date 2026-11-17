"""Check docs/layers.md and the project's own words (docs/ledger/names.md, section 25).

    python3 tools/layers_check.py            # check; exit 1 on a finding
    python3 tools/layers_check.py --write    # also rewrite the generated cells and blocks

What is checked:

- docs/layers.md, "Where everything sits": every path exists; every crate, every spec chapter,
  every library and every tracked top-level folder has a row; the layer and the ring come from
  their lists;
- the rings: a crate depends only on crates of its own ring or one nearer the centre, the model
  and the door do not depend on each other, and a library's program links neither the model
  nor the command (read from each crate's Cargo.toml);
- docs/layers.md, "The engine's form of each archetype": each archetype has one row and every
  pointer names something that exists;
- docs/ledger/names.md, section 25: every kind of every library has one row, with what it is
  made of as read from its `kinds.toml`; every step of every library has one row; each row's
  archetypes come from the list.

Standard library only, Python 3.9 or later.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ledger_check import (  # noqa: E402
    ARCHETYPES,
    OUTSIDE,
    ROOT,
    Pointers,
    family,
    ours,
    split_row,
    state_of,
    term_rows,
)

LAYERS = ROOT / "docs" / "layers.md"
NAMES = ROOT / "docs" / "ledger" / "names.md"

LAYER_NAMES = ("data types", "archetypes", "mechanisms", "concepts", "worlds", "-")
# From the centre outward. `model` and `door` share a rank and may not depend on each other.
RINGS = {"substrate": 0, "model": 1, "door": 1, "command": 2, "library": 3, "world": 4}
BESIDE = ("contract", "proof", "loader", "record")
BEGIN, END = "<!-- shows: begin -->", "<!-- shows: end -->"


def tables(path: Path) -> list[tuple[str, list[str], list[tuple[int, list[str]]]]]:
    """Every table of a Markdown file: its heading, its header and its rows with line numbers."""
    out, heading, header, rows = [], "", None, []
    for n, line in enumerate(path.read_text().splitlines(), start=1):
        if line.startswith("#"):
            heading = line.lstrip("# ").strip()
        if not line.startswith("|"):
            if header is not None:
                out.append((heading, header, rows))
            header, rows = None, []
            continue
        cells = split_row(line)
        if header is None:
            header = cells
        elif not all(set(c) <= set("-: ") for c in cells):
            rows.append((n, cells))
    if header is not None:
        out.append((heading, header, rows))
    return out


def crate_deps() -> dict[str, list[str]]:
    deps = {}
    for manifest in sorted((ROOT / "crates").glob("*/Cargo.toml")):
        body = re.search(r"(?ms)^\[dependencies\]\n(.*?)(?=^\[|\Z)", manifest.read_text())
        names = re.findall(r"(?m)^(via-[a-z]+)\s*=", body.group(1)) if body else []
        deps[manifest.parent.name] = names
    return deps


def check_layers() -> list[str]:
    problems = []
    found = {}
    for t in tables(LAYERS):
        key = "Archetype" if t[1][0] == "Archetype" and len(t[1]) == 3 else t[1][0] if t[1][0] != "Archetype" else "Counted"
        found[key] = t
    where = found.get("Path")
    if not where or where[1] != ["Path", "Layer", "Ring", "Holds"]:
        return ["docs/layers.md: no table with the columns Path, Layer, Ring, Holds"]
    ring_of: dict[str, str] = {}
    for n, cells in where[2]:
        at = f"docs/layers.md:{n}"
        path = cells[0].strip("`")
        if path in ring_of:
            problems.append(f"{at}: {path} has two rows")
        ring_of[path] = cells[2]
        if not (ROOT / path).exists():
            problems.append(f"{at}: no such path {path}")
        if cells[1] not in LAYER_NAMES:
            problems.append(f"{at}: the layer is one of {', '.join(LAYER_NAMES)}; found {cells[1]!r}")
        if cells[2] not in RINGS and cells[2] not in BESIDE:
            problems.append(f"{at}: the ring is one of {', '.join(list(RINGS) + list(BESIDE))}; found {cells[2]!r}")

    owed = [f"crates/{p.parent.name}" for p in sorted((ROOT / "crates").glob("*/Cargo.toml"))]
    owed += [f"spec/{p.name}" for p in sorted((ROOT / "spec").glob("*.md"))]
    owed += [f"lib/{p.name}" for p in sorted((ROOT / "lib").iterdir()) if p.is_dir()]
    for path in owed:
        if path not in ring_of:
            problems.append(f"docs/layers.md: {path} has no row")
    tracked = subprocess.run(
        ["git", "-C", str(ROOT), "ls-files"], check=True, capture_output=True, text=True
    ).stdout.splitlines()
    tops = sorted({t.split("/")[0] for t in tracked if "/" in t and not t.startswith(".")})
    for top in tops:
        if not any(p == top or p.startswith(top + "/") for p in ring_of):
            problems.append(f"docs/layers.md: the folder {top}/ has no row")

    for crate, deps in crate_deps().items():
        mine = ring_of.get(f"crates/{crate}")
        if mine not in RINGS:
            continue
        for dep in deps:
            theirs = ring_of.get(f"crates/{dep}")
            if theirs not in RINGS:
                problems.append(f"crates/{crate}: depends on {dep}, which has no ring")
            elif RINGS[theirs] > RINGS[mine] or {mine, theirs} == {"model", "door"}:
                problems.append(f"crates/{crate} ({mine}) depends on {dep} ({theirs}): outward or sideways")
            elif mine == "library" and theirs in ("model", "command"):
                problems.append(f"crates/{crate} (library) links {dep} ({theirs}): a library has no private door")

    order = [p.split("/")[1] for p in ring_of if p.startswith("lib/") and ring_of[p] == "library"]
    for lib, other, kind in library_refs():
        if lib in order and other in order and order.index(other) > order.index(lib):
            problems.append(
                f"lib/{lib} names {other}.{kind}, but lib/{other} is listed below it in docs/layers.md: "
                "a library names only the kinds of those above it"
            )

    form = found.get("Archetype")
    if not form or len(form[1]) != 3:
        problems.append("docs/layers.md: no table of the engine's form of each archetype")
    else:
        pointers = Pointers()
        seen = []
        for n, cells in form[2]:
            at = f"docs/layers.md:{n}"
            seen.append(cells[0])
            if cells[0] not in ARCHETYPES:
                problems.append(f"{at}: {cells[0]!r} is not an archetype")
            for p in re.findall(r"`([^`]+)`", cells[1] + " " + cells[2]):
                why = pointers.missing(p)
                if why:
                    problems.append(f"{at}: pointer `{p}`: {why}")
        for a in ARCHETYPES:
            if seen.count(a) != 1:
                problems.append(f"docs/layers.md: {a} has {seen.count(a)} rows in the table of forms")
    return problems


def kind_facts() -> dict[tuple[str, str], str]:
    """What each library kind is made of, read from its kinds.toml: (library, kind) -> text."""
    out = {}
    for f in sorted((ROOT / "lib").glob("*/kinds.toml")):
        lib = f.parent.name
        facts: dict[str, dict[str, list[str]]] = {}
        for tab in re.split(r"(?m)^(?=\[)", f.read_text()):
            m = re.match(r"\[kind\.([a-z_0-9]+)((?:\.[a-z_0-9]+)*)\]", tab)
            if not m:
                continue
            d = facts.setdefault(m.group(1), {"authority": [], "effect": [], "surface": [], "mount": [], "flag": []})
            rest = m.group(2).strip(".").split(".") if m.group(2) else []
            body = "\n".join(line.split("#")[0] for line in tab.splitlines()[1:])
            if not rest:
                a = re.search(r'(?m)^authority\s*=\s*"(\w+)"', body)
                if a:
                    d["authority"].append(a.group(1))
                for flag in ("exit", "output"):
                    if re.search(rf"(?m)^{flag}\s*=", body):
                        d["flag"].append(flag)
            elif rest[0] == "effect":
                field = re.search(r'(?m)^field\s*=\s*"([^"]+)"', body)
                d["effect"].append(field.group(1) if field else "?")
            elif rest[0] == "surface":
                d["surface"].append(rest[1] + (" with a cut" if re.search(r"(?m)^cut\s*=", body) else ""))
            elif rest[0] == "mount":
                d["mount"].append(rest[1])
        for kind, d in facts.items():
            parts = []
            if d["authority"]:
                parts.append(d["authority"][0])
            if d["effect"]:
                fields = sorted(set(d["effect"]))
                parts.append(f"{len(d['effect'])} effect{'s' if len(d['effect']) > 1 else ''} on " + ", ".join(fields))
            if d["surface"]:
                parts.append("surface " + ", ".join(d["surface"]))
            if d["mount"]:
                parts.append(f"{len(d['mount'])} mount{'s' if len(d['mount']) > 1 else ''}")
            parts += d["flag"]
            out[(lib, kind)] = "; ".join(parts) if parts else "attributes only"
    return out


def library_refs() -> list[tuple[str, str, str]]:
    """(library, other library, kind): where one library's kinds file or step names a kind
    that another library ships."""
    owner = {kind: lib for (lib, kind) in kind_facts()}
    found = set()
    for f in sorted((ROOT / "lib").glob("*/kinds.toml")):
        lib = f.parent.name
        text = "\n".join(line.split("#")[0] for line in f.read_text().splitlines())
        for name in re.findall(r"kind:([a-z_0-9]+)", text):
            if name in owner and owner[name] != lib:
                found.add((lib, owner[name], name))
    for path in step_files():
        lib = path.split("/")[1]
        text = "\n".join(line.split("#")[0] for line in (ROOT / path).read_text().splitlines())
        for name in re.findall(r"""["']([a-z_0-9]+)["']""", text):
            # a quoted word of five letters or more that is another library's kind; "building"
            # is also an ordinary attribute value
            if name in owner and owner[name] != lib and len(name) > 4 and name != "building":
                found.add((lib, owner[name], name))
    return sorted(found)


def step_files() -> list[str]:
    return sorted(
        str(p.relative_to(ROOT)) for p in (ROOT / "lib").glob("*/*.py") if not p.name.startswith("test_")
    )


def archetypes_ok(cell: str) -> bool:
    for name in [c.strip() for c in cell.split(",")]:
        fam, cls = family(name)
        if fam in ARCHETYPES:
            if bool(ARCHETYPES[fam]) != bool(cls) or (cls and cls not in ARCHETYPES[fam]):
                return False
        elif name not in OUTSIDE:
            return False
    return bool(cell.strip())


def check_names(write: bool) -> list[str]:
    problems = []
    text = NAMES.read_text()
    lines = text.splitlines()
    facts = kind_facts()
    seen_kinds: dict[tuple[str, str], int] = {}
    seen_steps: list[str] = []
    header = None
    for i, line in enumerate(lines):
        if not line.startswith("|"):
            header = None
            continue
        cells = split_row(line)
        if header is None:
            header = cells
            continue
        if all(set(c) <= set("-: ") for c in cells):
            continue
        at = f"docs/ledger/names.md:{i + 1}"
        if header == ["Kind", "Library", "Archetypes", "Made of"]:
            key = (cells[1].strip("`"), cells[0].strip("`"))
            seen_kinds[key] = seen_kinds.get(key, 0) + 1
            if key not in facts:
                problems.append(f"{at}: lib/{key[0]} has no kind {key[1]}")
                continue
            if not archetypes_ok(cells[2]):
                problems.append(f"{at}: {cells[2]!r} is not a list of archetypes")
            if cells[3] != facts[key]:
                if write:
                    lines[i] = f"| {cells[0]} | {cells[1]} | {cells[2]} | {facts[key]} |"
                else:
                    problems.append(f"{at}: {key[1]} is made of {facts[key]!r}; run with --write")
        elif header == ["Step", "When", "Archetypes", "What it does"]:
            path = cells[0].strip("`")
            seen_steps.append(path)
            if not (ROOT / path).exists():
                problems.append(f"{at}: no such step {path}")
            if not archetypes_ok(cells[2]):
                problems.append(f"{at}: {cells[2]!r} is not a list of archetypes")
    for key in facts:
        if seen_kinds.get(key, 0) != 1:
            problems.append(f"docs/ledger/names.md: {key[0]}.{key[1]} has {seen_kinds.get(key, 0)} rows in section 25")
    for path in step_files():
        if seen_steps.count(path) != 1:
            problems.append(f"docs/ledger/names.md: {path} has {seen_steps.count(path)} rows in section 25")
    if write:
        new = "\n".join(lines) + "\n"
        if new != text:
            NAMES.write_text(new)
            print("docs/ledger/names.md: generated cells rewritten")
    return problems


def world_texts() -> dict[str, str]:
    """Every world, library and pack file, comments removed, by path."""
    out = {}
    for base in ("examples", "lib"):
        for f in sorted((ROOT / base).rglob("*.toml")):
            rel = str(f.relative_to(ROOT))
            if "/build/" in rel or "/.via/" in rel or "/gen/" in rel:
                continue
            out[rel] = "\n".join(line.split("#")[0] for line in f.read_text(errors="replace").splitlines())
    return out


def names_tables() -> tuple[dict[str, list[str]], dict[str, list[str]]]:
    """Section 25's kinds and steps, each with its archetype families."""
    kinds: dict[str, list[str]] = {}
    steps: dict[str, list[str]] = {}
    for _heading, header, rows in tables(NAMES):
        for _n, cells in rows:
            fams = [family(c.strip())[0] for c in cells[2].split(",")] if len(cells) > 2 else []
            if header == ["Kind", "Library", "Archetypes", "Made of"]:
                kinds[cells[1].strip("`") + "." + cells[0].strip("`")] = fams
            elif header == ["Step", "When", "Archetypes", "What it does"]:
                steps[cells[0].strip("`")] = fams
    return kinds, steps


def shows_block() -> str:
    """What the layout shows, counted: each archetype through the layers, and the holes."""
    texts = world_texts()
    examples = sorted(p.name for p in (ROOT / "examples").iterdir() if p.is_dir())
    kinds, steps = names_tables()
    uses: dict[str, set[str]] = {k: set() for k in kinds}
    for rel, text in texts.items():
        if not rel.startswith("examples/"):
            continue
        ex = rel.split("/")[1]
        for name in re.findall(r'(?m)^\s*kind\s*=\s*"([a-z_0-9]+)"', text):
            for k in kinds:
                if k.split(".")[1] == name:
                    uses[k].add(ex)
    rows, _ = term_rows()
    cases = {p.name: sum(1 for c in p.iterdir() if c.is_dir()) for p in sorted((ROOT / "spec" / "cases").iterdir()) if p.is_dir()}
    forms = {}
    for _heading, header, trows in tables(LAYERS):
        if header and header[0] == "Archetype" and len(header) == 3:
            for _n, cells in trows:
                forms[cells[0]] = cells
    out = [BEGIN, ""]
    out.append(f"Counted over {len(rows)} ledger rows, {len(kinds)} kinds, {len(steps)} steps, {len(examples)} example worlds and {sum(cases.values())} conformance cases.")
    out.append("")
    out.append("| Archetype | Ledger rows that are ours: shown, partly, none | Chapters, with their cases | Kinds | Steps | Examples that use it |")
    out.append("|---|---|---|---|---|---|")
    for a in ARCHETYPES:
        mine = [r for r in rows if family(r["archetype"])[0] == a and ours(r)]
        st = [sum(state_of(r) == s for r in mine) for s in ("shown", "partly", "none")]
        chapters = []
        for ch in re.findall(r"`([a-z]+)\.md \d", forms.get(a, ["", "", ""])[2]):
            if ch not in chapters:
                chapters.append(ch)
        ks = [k for k, fams in kinds.items() if a in fams]
        ss = [s for s, fams in steps.items() if a in fams]
        exs = set()
        for k in ks:
            exs |= uses[k]
        for token in re.findall(r"`([^`]+)`", forms.get(a, ["", "", ""])[1]):
            if token.startswith("["):
                pat = r"(?m)^\s*\[" + re.escape(token[1:].split(".")[0]) + r"\."
            elif token.endswith("()"):
                pat = r"\b" + re.escape(token[:-2]) + r"\("
            elif "." in token or "/" in token:
                continue
            else:
                pat = r"(?m)(^|[{,])\s*" + re.escape(token) + r"\s*="
            for rel, text in texts.items():
                if rel.startswith("examples/") and re.search(pat, text):
                    exs.add(rel.split("/")[1])
        out.append(
            f"| {a} | {st[0]}, {st[1]}, {st[2]} | "
            + (", ".join(f"{c} {cases.get(c, 0)}" for c in chapters) or "none")
            + f" | {len(ks)} | {len(ss)} | {len(exs)} |"
        )
    out.append("")
    none_form = [a for a, cells in forms.items() if cells[1].startswith("None in the engine")]
    out.append("**Archetypes with no form in the engine:** " + (", ".join(none_form) or "none") + ".")
    out.append("")
    unused = sorted(k for k, v in uses.items() if not v)
    out.append(f"**Kinds no example writes ({len(unused)}):** " + ", ".join(f"`{k}`" for k in unused) + ".")
    out.append("")
    by_step = []
    engine_has = [a for a, cells in forms.items() if not cells[1].startswith("None in the engine")]
    for s_path, fams in steps.items():
        path = ROOT / s_path
        if path.is_file() and any(f in engine_has and f not in ("Measure", "Simulation", "Delivery") for f in fams):
            by_step.append(f"`{s_path}` ({', '.join(fams)}, {len(path.read_text().splitlines())} lines)")
    out.append("**Steps that do in code what the engine has a form for:** " + "; ".join(by_step) + ".")
    out.append("")
    data = sum(len(p.read_text().splitlines()) for p in (ROOT / "lib").glob("*/kinds.toml"))
    code = sum(len((ROOT / p).read_text().splitlines()) for p in step_files())
    out.append(f"**Concepts as data and as code:** {data} lines of kinds, {code} lines of steps (tests left out).")
    out.append("")
    cross = [f"`{lib}` names `{other}.{kind}`" for lib, other, kind in library_refs()]
    out.append("**One library naming another's kinds:** " + ("; ".join(cross) or "none") + ".")
    out.append("")
    keys = re.findall(r'(?m)^\[key\."([^"]+)"\]', (ROOT / "spec/vocabulary.toml").read_text())
    blob = "\n".join(texts.values())
    unwritten = []
    for k in keys:
        if k.startswith("["):
            pat = re.sub(r"<[a-z]+>", "[A-Za-z0-9_]+", re.escape(k[1:-1]).replace("\\<", "<").replace("\\>", ">"))
            hit = re.search(r"(?m)^\s*\[" + pat + r"[\].]", blob)
        else:
            hit = re.search(r"(?m)(^|[{,])\s*" + re.escape(k) + r"\s*=", blob)
        if not hit:
            unwritten.append(k)
    out.append(f"**Keys of the format that no example, library or pack writes ({len(unwritten)}):** " + ", ".join(f"`{k}`" for k in unwritten) + ".")
    out.append("")
    out.append(END)
    return "\n".join(out)


def main() -> int:
    write = "--write" in sys.argv
    problems = check_layers() + check_names(write)
    text = LAYERS.read_text()
    if BEGIN in text and END in text:
        current = text[text.index(BEGIN) : text.index(END) + len(END)]
        block = shows_block()
        if current != block:
            if write:
                LAYERS.write_text(text.replace(current, block))
                print("docs/layers.md: what the layout shows, rewritten")
            else:
                problems.append("docs/layers.md: the block of what the layout shows is stale; run with --write")
    for p in problems:
        print(p)
    print(f"{len(problems)} finding(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
