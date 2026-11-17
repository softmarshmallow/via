"""Check the ledger (docs/ledger/) and count it.

    python3 tools/ledger_check.py            # check; exit 1 on a finding
    python3 tools/ledger_check.py --write    # also rewrite the counts block in docs/ledger/README.md
    python3 tools/ledger_check.py --rows     # print every term row as tab-separated text

What is checked (docs/ledger/README.md, "The row"):

- every term table has the row shape: term, meaning, Archetype, Whose (Tool in sections 21 to
  24), Now;
- the archetype is one of the list in docs/ledger/archetypes.md, with its class where it has one;
- the owner marks and the Tool marks come from their closed lists;
- `Now` is `none`, or `shown:` / `partly:` followed by pointers, and every pointer names
  something that exists in this repository (a path, a spec chapter's section, a formula
  function, a library kind, a vocabulary key);
- no term is written twice;
- the counts block in the README equals the count.

Standard library only, Python 3.9 or later.
"""

from __future__ import annotations

import re
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LEDGER = ROOT / "docs" / "ledger"

# The closed lists. docs/ledger/archetypes.md defines each; a change there is a change here.
ARCHETYPES = {
    "Field": (),
    "Shape": (),
    "Asset": (),
    "Derivation": ("local", "focal", "zonal", "global"),
    "Simulation": (),
    "Profile": (),
    "Network": ("directed", "undirected"),
    "Area partition": (),
    "Placement": (),
    "Linear placement": (),
    "Scatter": (),
    "Recipe": (),
    "Grammar": (),
    "Representation switch": (),
    "Meeting": (),
    "Overlay": (),
    "Guidance": (),
    "Measure": (),
    "Markup": (),
    "Delivery": (),
}
OUTSIDE = ("Runtime", "Practice", "Principle")  # what the content model leaves out, on purpose
MISFIT = "-"
OWNERS = ("core", "lib", "pack", "loader", "engine", "game", "carries", "?")
OURS = ("core", "lib", "pack", "loader")
TOOLS = ("nothing", "carry", "rule", "measure")
STATES = ("shown", "partly", "none")

BEGIN, END = "<!-- counts: begin -->", "<!-- counts: end -->"


def split_row(line: str) -> list[str]:
    body = line.strip().strip("|")
    return [c.strip() for c in re.split(r"(?<!\\)\|", body)]


def term_rows() -> tuple[list[dict], list[str]]:
    rows, problems = [], []
    for path in sorted(LEDGER.glob("*.md")):
        if path.name in ("README.md", "archetypes.md"):
            continue
        section, header = "", None
        for n, line in enumerate(path.read_text().splitlines(), start=1):
            if line.startswith("## "):
                section, header = line[3:].strip(), None
                continue
            if not line.startswith("|"):
                header = None
                continue
            cells = split_row(line)
            if header is None:
                header = cells
                continue
            if all(set(c) <= set("-: ") for c in cells):
                continue
            if "Archetype" not in header:
                continue  # a table that is not term rows: names, sources, a tool's stance
            where = f"docs/ledger/{path.name}:{n}"
            if len(header) != 5 or header[2] != "Archetype" or header[4] != "Now":
                problems.append(f"{where}: a term table's columns are term, meaning, Archetype, Whose or Tool, Now")
                continue
            if header[3] not in ("Whose", "Tool"):
                problems.append(f"{where}: the fourth column is Whose or Tool, not {header[3]!r}")
                continue
            if len(cells) != 5:
                problems.append(f"{where}: {len(cells)} cells in a row of five")
                continue
            rows.append(
                {
                    "where": where,
                    "file": path.name,
                    "section": section,
                    "term": cells[0],
                    "meaning": cells[1],
                    "archetype": cells[2],
                    "by": header[3],
                    "who": cells[3],
                    "now": cells[4],
                }
            )
    return rows, problems


def family(archetype: str) -> tuple[str, str]:
    m = re.fullmatch(r"(.+?) \((.+)\)", archetype)
    return (m.group(1), m.group(2)) if m else (archetype, "")


class Pointers:
    """What a `Now` pointer may name, read once from the repository."""

    def __init__(self) -> None:
        self.keys = set(re.findall(r'^\[key\."([^"]+)"\]', (ROOT / "spec/vocabulary.toml").read_text(), re.M))
        self.functions = set(
            re.findall(r"^\[function\.([a-z_0-9]+)\]", (ROOT / "spec/functions.toml").read_text(), re.M)
        )
        self.kinds = {}
        for f in sorted((ROOT / "lib").glob("*/kinds.toml")):
            self.kinds[f.parent.name] = set(re.findall(r"^\[kind\.([a-z_0-9]+)\]", f.read_text(), re.M))
        self.heads: dict[str, set[str]] = {}

    def chapter(self, name: str) -> set[str] | None:
        if name not in self.heads:
            path = ROOT / "spec" / f"{name}.md"
            if not path.exists():
                return None
            self.heads[name] = set(re.findall(r"^#{2,4} (\d+(?:\.\d+)*)\.? ", path.read_text(), re.M))
        return self.heads[name]

    def missing(self, p: str) -> str | None:
        """None when `p` names something that exists, else what was looked for."""
        if "/" in p:
            return None if (ROOT / p).exists() else "no such path"
        m = re.fullmatch(r"([a-z]+)\.md (\d+(?:\.\d+)*)", p)
        if m:
            heads = self.chapter(m.group(1))
            if heads is None:
                return f"no chapter spec/{m.group(1)}.md"
            return None if m.group(2) in heads else f"no section {m.group(2)} in spec/{m.group(1)}.md"
        if p.endswith("()"):
            return None if p[:-2] in self.functions else "no such function in spec/functions.toml"
        m = re.fullmatch(r"([a-z]+)\.([a-z_0-9]+)", p)
        if m and m.group(1) in self.kinds:
            return None if m.group(2) in self.kinds[m.group(1)] else f"no kind {m.group(2)} in lib/{m.group(1)}"
        return None if p in self.keys else "not a key of spec/vocabulary.toml"


def check(rows: list[dict]) -> list[str]:
    problems = []
    pointers = Pointers()
    seen: dict[str, str] = {}
    for r in rows:
        at = f"{r['where']}: {r['term']!r}"
        fam, cls = family(r["archetype"])
        if fam in ARCHETYPES:
            classes = ARCHETYPES[fam]
            if classes and cls not in classes:
                problems.append(f"{at}: {fam} takes a class, one of {', '.join(classes)}")
            if not classes and cls:
                problems.append(f"{at}: {fam} takes no class")
        elif r["archetype"] not in OUTSIDE and r["archetype"] != MISFIT:
            problems.append(f"{at}: {r['archetype']!r} is not an archetype (docs/ledger/archetypes.md)")
        if r["by"] == "Tool":
            if r["who"] not in TOOLS:
                problems.append(f"{at}: Tool is one of {', '.join(TOOLS)}, not {r['who']!r}")
        else:
            marks = [m.strip() for m in r["who"].split(",")]
            bad = [m for m in marks if m not in OWNERS]
            if bad or not marks:
                problems.append(f"{at}: owner marks are {', '.join(OWNERS)}; found {r['who']!r}")
        state, _, rest = r["now"].partition(":")
        if state not in STATES:
            problems.append(f"{at}: Now is none, or shown: or partly: with pointers; found {r['now']!r}")
        elif state == "none":
            if rest:
                problems.append(f"{at}: a none takes no pointer")
        else:
            ptrs = re.findall(r"`([^`]+)`", rest)
            if not ptrs or re.sub(r"`[^`]+`", "", rest).strip(" ,"):
                problems.append(f"{at}: {state} needs pointers in backticks, separated by commas")
            for p in ptrs:
                why = pointers.missing(p)
                if why:
                    problems.append(f"{at}: pointer `{p}`: {why}")
        key = r["term"].lower()
        if key in seen:
            problems.append(f"{at}: the term is also at {seen[key]}")
        seen[key] = r["where"]
    return problems


def state_of(r: dict) -> str:
    return r["now"].partition(":")[0]


def ours(r: dict) -> bool:
    """A row some part of which is ours to build: a first owner mark of ours, or a Tool mark
    other than nothing."""
    if r["by"] == "Tool":
        return r["who"] != "nothing"
    return r["who"].split(",")[0].strip() in OURS


def counts_block(rows: list[dict]) -> str:
    out = [BEGIN, ""]
    world = [r for r in rows if r["by"] == "Whose"]
    looks = [r for r in rows if r["by"] == "Tool"]
    out.append(f"{len(rows)} term rows: {len(world)} in sections 1 to 19, {len(looks)} in sections 21 to 24.")
    out.append("")
    out.append("| Archetype | Rows | Ours | Ours, shown | Ours, partly | Ours, none |")
    out.append("|---|---|---|---|---|---|")
    fams = Counter(family(r["archetype"])[0] for r in rows)
    order = [a for a in ARCHETYPES] + list(OUTSIDE) + [MISFIT]
    for a in order:
        mine = [r for r in rows if family(r["archetype"])[0] == a and ours(r)]
        st = Counter(state_of(r) for r in mine)
        out.append(f"| {a} | {fams[a]} | {len(mine)} | {st['shown']} | {st['partly']} | {st['none']} |")
    mine = [r for r in rows if ours(r)]
    st = Counter(state_of(r) for r in mine)
    out.append(f"| **All** | **{len(rows)}** | **{len(mine)}** | **{st['shown']}** | **{st['partly']}** | **{st['none']}** |")
    out.append("")
    classes = Counter(r["archetype"] for r in rows if family(r["archetype"])[1])
    out.append("By class: " + ", ".join(f"{k} {v}" for k, v in sorted(classes.items())) + ".")
    out.append("")
    out.append("| Section | Rows | Ours | Ours, none |")
    out.append("|---|---|---|---|")
    secs: dict[str, list[dict]] = {}
    for r in rows:
        secs.setdefault(r["section"], []).append(r)
    for s in sorted(secs, key=lambda t: int(t.split(".")[0])):
        mine = [r for r in secs[s] if ours(r)]
        out.append(f"| {s} | {len(secs[s])} | {len(mine)} | {sum(state_of(r) == 'none' for r in mine)} |")
    out.append("")
    first = Counter(r["who"].split(",")[0].strip() for r in world)
    out.append("First owner mark, sections 1 to 19: " + ", ".join(f"{k} {first[k]}" for k in OWNERS if first[k]) + ".")
    tool = Counter(r["who"] for r in looks)
    out.append("Tool mark, sections 21 to 24: " + ", ".join(f"{k} {tool[k]}" for k in TOOLS) + ".")
    out.append("")
    out.append(END)
    return "\n".join(out)


def main() -> int:
    rows, problems = term_rows()
    if "--rows" in sys.argv:
        for r in rows:
            print("\t".join([r["section"], r["term"], r["meaning"], r["archetype"], r["who"], r["now"]]))
        return 0
    problems += check(rows)
    block = counts_block(rows)
    readme = LEDGER / "README.md"
    text = readme.read_text() if readme.exists() else ""
    if BEGIN in text and END in text:
        current = text[text.index(BEGIN) : text.index(END) + len(END)]
        if current != block:
            if "--write" in sys.argv:
                readme.write_text(text.replace(current, block))
                print("docs/ledger/README.md: counts rewritten")
            else:
                problems.append("docs/ledger/README.md: the counts block is stale; run with --write")
    else:
        problems.append("docs/ledger/README.md: no counts block")
    for p in problems:
        print(p)
    print(f"{len(rows)} term rows, {len(problems)} finding(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
