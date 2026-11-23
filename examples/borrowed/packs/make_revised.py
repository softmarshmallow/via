"""Write packs/kenney/building-window-revised.glb: Kenney's window block re-exported 1.1 times as
long (x), as an artist's revision might be. Ours, for examples/borrowed's `revised` case; it reads
the fetched model and writes beside it, so neither is committed. Standard library only.

    python3 examples/borrowed/packs/make_revised.py
"""
import json
import os
import struct

HERE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "kenney")
SRC, DST = os.path.join(HERE, "building-window.glb"), os.path.join(HERE, "building-window-revised.glb")


def main():
    b = open(SRC, "rb").read()
    magic, version, _ = struct.unpack_from("<III", b, 0)
    if magic != 0x46546C67:
        raise SystemExit(f"{SRC} is not a GLB file")
    clen, ctype = struct.unpack_from("<II", b, 12)
    doc = json.loads(b[20:20 + clen])
    rest = b[20 + clen:]                          # the binary chunk, unchanged
    for n in doc["nodes"]:
        if "mesh" in n:
            if "matrix" in n:
                raise SystemExit("a node with a matrix: not handled")
            s = n.get("scale", [1, 1, 1])
            n["scale"] = [s[0] * 1.1, s[1], s[2]]
    j = json.dumps(doc, separators=(",", ":")).encode()
    j += b" " * (-len(j) % 4)
    out = struct.pack("<III", magic, version, 12 + 8 + len(j) + len(rest)) + struct.pack("<II", len(j), ctype) + j + rest
    open(DST, "wb").write(out)
    print(f"wrote {DST}")


if __name__ == "__main__":
    main()
