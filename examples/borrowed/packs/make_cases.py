"""Write the case packs of examples/borrowed from kenney.toml, so each differs from it in one
thing only (README.md, "Cases"). Ours. Standard library only.

    python3 examples/borrowed/packs/make_cases.py

- sill.toml: kenney.toml and one more variant, Kenney's window with a sill 0.05 proud of the
  block's face (`building-window-sill.glb`).
- revised.toml: kenney.toml with the window block re-exported 1.1 times as long
  (`building-window-revised.glb`, written by make_revised.py).
"""
import os

HERE = os.path.dirname(os.path.abspath(__file__))
HEAD = "# Written by make_cases.py from kenney.toml: {why}. Do not edit; edit kenney.toml.\n\n"


def main():
    base = open(os.path.join(HERE, "kenney.toml")).read()
    sill = (HEAD.format(why="the window with a sill, as one more variant") + base
            + '\n[asset.sill]\nfile   = "building-window-sill.glb"\ntaxon  = "structure/kenney"\n'
              'tags   = ["role:wall", "variant:sill"]\nparams = { pitch = 1.0 }\n')
    old = 'file   = "building-window.glb"'
    if base.count(old) != 1:
        raise SystemExit("kenney.toml: the window's file line is not where make_cases.py looks")
    revised = HEAD.format(why="the window block re-exported 1.1 times as long") + base.replace(
        old, 'file   = "building-window-revised.glb"')
    for name, text in (("sill.toml", sill), ("revised.toml", revised)):
        open(os.path.join(HERE, name), "w").write(text)
        print(f"wrote {name}")


if __name__ == "__main__":
    main()
