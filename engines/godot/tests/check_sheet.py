"""Checks what the sheet tool wrote (spec/build.md 15.1), Python's standard library only.

    python3 engines/godot/tests/check_sheet.py OUT ROWS [--one]

Every PNG it names is 1280 by 720 and not one colour; sheet.png is 1280 (640 with --one) by
48 + 384 * ROWS; sheet.json is canonical and has the keys 15.1 lists; diagnostics.json is
diagnostics.md 7's document. Images are never compared byte for byte: renderers differ.
Prints one line per check; exit 0 when all pass.
"""

import json
import struct
import sys
import zlib
from pathlib import Path


def png(path):
    """(width, height, rows of RGB(A) bytes) of an 8-bit truecolour PNG."""
    data = path.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", "not a PNG"
    pos, idat, w = 8, b"", 0
    while pos < len(data):
        n, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + n]
        if kind == b"IHDR":
            w, h, depth, ctype = struct.unpack(">IIBB", body[:10])
            assert depth == 8 and ctype in (2, 6), f"colour type {ctype}, depth {depth}"
            bpp = 3 if ctype == 2 else 4
        elif kind == b"IDAT":
            idat += body
        pos += 12 + n
    raw = zlib.decompress(idat)
    stride = w * bpp
    rows, prev = [], bytearray(stride)
    for y in range(h):
        f = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - bpp] if i >= bpp else 0
            b = prev[i]
            c = prev[i - bpp] if i >= bpp else 0
            if f == 1:
                line[i] = (line[i] + a) & 255
            elif f == 2:
                line[i] = (line[i] + b) & 255
            elif f == 3:
                line[i] = (line[i] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(bytes(line))
        prev = line
    return w, h, rows, bpp


def _norm(v):
    """Integral doubles as integers, as canonical JSON writes them (documents.md 8)."""
    if isinstance(v, float) and v == int(v):
        return int(v)
    if isinstance(v, dict):
        return {k: _norm(x) for k, x in v.items()}
    if isinstance(v, list):
        return [_norm(x) for x in v]
    return v


def one_colour(rows, bpp):
    first = rows[0][:bpp]
    return all(r == first * (len(r) // bpp) for r in rows)


def main():
    out, n_rows, one = Path(sys.argv[1]), int(sys.argv[2]), "--one" in sys.argv[3:]
    failed = 0

    def check(what, ok, detail=""):
        nonlocal failed
        failed += not ok
        print(("ok   " if ok else "FAIL ") + what + (f": {detail}" if detail else ""))

    text = (out / "sheet.json").read_text()
    doc = json.loads(text)
    canon = json.dumps(_norm(doc), sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n"
    check("sheet.json is canonical", text == canon)
    check("sheet.json keys", sorted(doc) == ["after", "before", "bookmarks", "camera", "godot", "light", "renderer", "size"])
    check("one entry per row, in name order", [b["name"] for b in doc["bookmarks"]] == sorted(b["name"] for b in doc["bookmarks"])
          and len(doc["bookmarks"]) == n_rows)
    check("after is null without --after", (doc["after"] is None) == one)
    for b in doc["bookmarks"]:
        for side in ("before", "after"):
            if b[side] is None:
                continue
            w, h, rows, bpp = png(out / b[side])
            check(f"{b[side]} is 1280 by 720 and not one colour", (w, h) == (1280, 720) and not one_colour(rows, bpp), f"{w}x{h}")
    w, h, rows, bpp = png(out / "sheet.png")
    want = (640 if one else 1280, 48 + 384 * n_rows)
    check(f"sheet.png is {want[0]} by {want[1]}", (w, h) == want, f"{w}x{h}")
    diag = json.loads((out / "diagnostics.json").read_text())
    check("diagnostics.json is diagnostics.md 7's document", diag.get("format") == "via-diagnostics-1"
          and set(diag) == {"counts", "diagnostics", "format", "lints", "status"})
    print("PASS" if not failed else f"FAIL: {failed} check(s) failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
