"""Write the change packs of examples/high_street from kenney.toml, so each differs from it in one
thing only (README.md, "Changes"). Ours. Standard library only.

    python3 examples/high_street/packs/make_cases.py

- remodel.toml: kenney.toml with the `windows` block swapped for another of Kenney's models of
  the same box (`building-window-large.glb`): a piece remodelled, its placing facts the same.
- module.toml: kenney.toml read at `scale = 5`, and the lengths it states at that size: the kit's
  module 5 m, where it was 4 m. Its facts file is kenney.facts.toml as accepted at `scale = 4`,
  copied, so a build says every block revised (`asset.revised`, spec/assets.md 4.2), as a pack
  re-exported and not yet accepted does.
"""
import os

HERE = os.path.dirname(os.path.abspath(__file__))
HEAD = "# Written by make_cases.py from kenney.toml: {why}. Do not edit; edit kenney.toml.\n\n"


def swap(text, old, new):
    if text.count(old) != 1:
        raise SystemExit(f"kenney.toml: `{old}` is not where make_cases.py looks")
    return text.replace(old, new)


def main():
    base = open(os.path.join(HERE, "kenney.toml")).read()
    remodel = HEAD.format(why="the windows block swapped for Kenney's large window") + swap(
        base, 'file  = "building-windows.glb"', 'file  = "building-window-large.glb"')
    module = HEAD.format(why="the kit read at scale 5, its stated lengths at that size") + base
    for old, new in (("scale = 4 }", "scale = 5 }"),
                     ("module = 4.0 ", "module = 5.0 "), ("wall   = 4.0 ", "wall   = 5.0 "),
                     ("storey = 2.5 ", "storey = 3.125"), ("door   = 1.0 ", "door   = 1.25"),
                     ("eaves  = 0.556", "eaves  = 0.695")):
        module = swap(module, old, new)
    facts = open(os.path.join(HERE, "kenney.facts.toml")).read()
    for name, text in (("remodel.toml", remodel), ("module.toml", module),
                       ("remodel.facts.toml", facts), ("module.facts.toml", facts)):
        open(os.path.join(HERE, name), "w").write(text)
        print(f"wrote {name}")


if __name__ == "__main__":
    main()
