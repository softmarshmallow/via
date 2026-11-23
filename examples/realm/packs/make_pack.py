"""Writes packs/realm.toml: one pack that holds the kits of three other examples.

    sdk/python/.venv/bin/python examples/realm/packs/make_pack.py     # Python 3.11+, for tomllib
    via pack accept examples/realm/packs/realm.toml                   # after a kit changes

A world has one pack (spec/assets.md 2), and the three kits were each written as a pack of their
own: `examples/village/packs/village.toml`, `examples/court/packs/timber.toml` (the timber kit
and the yard kit) and `examples/temple/packs/order.toml`. Read together they do not make a pack:
`door`, `tile` and `base` are an asset of all three, `post`, `roof_slope` and others of two, and
`material.ground` has two colours. So this script reads the three and writes one, changing two
things and nothing else:

- every asset's id gains its kit's prefix (`v_`, `t_`, `o_`), so no two are one id. Nothing in a
  kit names a piece by its id (lib/building reads a kit by its taxon and each piece by its
  `role:` tag, docs/design/kit-contract.md), so no other line changes;
- every `file` is written from this pack's `root`, the `examples` folder, so the models are the
  other examples' own files and none is copied.

`[taxon."structure"]` is in all three with one value and is written once. The three kits' taxa
(`structure/village`, `structure/timber`, `structure/yard`, `structure/order`) differ already.
The land's paints are this world's own and are added below.
"""

import sys
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
EXAMPLES = HERE.parents[1]

# (the pack, its ids' prefix)
KITS = [("village/packs/village.toml", "v_"),
        ("court/packs/timber.toml", "t_"),
        ("temple/packs/order.toml", "o_")]

# The land's paints (lib/land/ground.md names them; the colours are lib/packs/starter.toml's), and
# `ground`, which the kits' own worlds paint: here it is the trodden earth of a yard.
MATERIALS = {
    "grass": [0.2, 0.4, 0.1],
    "rock": [0.45, 0.43, 0.4],
    "gravel": [0.36, 0.33, 0.28],
    "earth": [0.3, 0.22, 0.14],
    "sand": [0.76, 0.7, 0.5],
    "ground": [0.5, 0.5, 0.45],
}


def key(k):
    return k if k.replace("_", "").replace("-", "").isalnum() else '"' + k + '"'


def value(v):
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, str):
        return '"' + v.replace("\\", "\\\\").replace('"', '\\"') + '"'
    if isinstance(v, (int, float)):
        return repr(v)
    if isinstance(v, list):
        return "[" + ", ".join(value(x) for x in v) + "]"
    if isinstance(v, dict):
        return "{ " + ", ".join(f"{key(k)} = {value(x)}" for k, x in v.items()) + " }"
    raise TypeError(type(v))


def table(path, body, out):
    """One table and the tables under it. A table inside an asset is written inline."""
    plain = {k: v for k, v in body.items() if not isinstance(v, dict) or path[0] == "asset"}
    under = {k: v for k, v in body.items() if k not in plain}
    if plain or not under:
        out.append("[" + ".".join(key(p) for p in path) + "]")
        out.extend(f"{key(k)} = {value(v)}" for k, v in plain.items())
        out.append("")
    for k, v in under.items():
        table(path + [k], v, out)


def merged():
    taxa, assets, counted = {}, {}, []
    for rel, prefix in KITS:
        src = EXAMPLES / rel
        pack = tomllib.loads(src.read_text())
        folder = Path(rel).parent / pack.get("root", ".")
        for name, body in pack.get("taxon", {}).items():
            if name in taxa and taxa[name] != body:
                sys.exit(f"{rel}: taxon `{name}` differs from another kit's")
            taxa[name] = body
        for name, body in pack.get("asset", {}).items():
            body = dict(body)
            if "file" in body:
                body["file"] = (folder / body["file"]).as_posix()
            assets[prefix + name] = body
        counted.append((rel, prefix, len(pack.get("asset", {}))))
    return taxa, assets, counted


def main():
    taxa, assets, counted = merged()
    out = ["# The realm's pack: the kits of three other examples as one pack. WRITTEN BY",
           "# packs/make_pack.py: edit the kits' own packs, or that script, and run it again.",
           "#"]
    out += [f"#   {rel}: {n} assets, ids under `{prefix}`" for rel, prefix, n in counted]
    out += ["", 'root = "../.."', ""]
    for name, body in taxa.items():
        table(["taxon", name], body, out)
    for name, body in assets.items():
        table(["asset", name], body, out)
    for name, colour in MATERIALS.items():
        out += [f"[material.{name}]", f"colour = {value(colour)}", ""]
    (HERE / "realm.toml").write_text("\n".join(out))
    print(f"realm.toml: {len(assets)} assets, {len(taxa)} taxa, {len(MATERIALS)} materials")


if __name__ == "__main__":
    main()
