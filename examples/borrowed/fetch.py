"""Fetch Kenney's Modular Buildings 2.1 (CC0) into packs/kenney/, beside the pack that lays it.
The kit is never committed (docs/plan-kits.md, K10; packs/kenney/ is in .gitignore): this script is
how a checkout gets it. Standard library only.

    python3 examples/borrowed/fetch.py            # download, check, extract
    python3 examples/borrowed/fetch.py --zip FILE # extract a zip already downloaded (checked first)

What it does, in order:
1. Asks kenney.nl for a download link the way the asset page's own button does: the page first
   (for its session cookie), then `download-link/<id>`, which answers `{"url": ...}`, then the url.
2. Refuses a download or `--zip` over 20 MB, by its size, then the zip unless its SHA-256 is the
   one below: the kit this example was measured with. Redirects are followed only to kenney.nl
   and its subdomains; the hash is what makes the bytes the kit wherever they came from.
3. Extracts only `Models/GLB format/*.glb`, the texture they name, and `License.txt`, each by
   its own file name. Any entry whose name would land outside the kit's folder is refused, as is
   an entry over 2 MB or a selection over 20 MB once unpacked, and a zip without `License.txt`.
   Nothing in the zip is run. Everything is extracted into a new folder beside packs/kenney/
   first and moved into packs/kenney/ only when every check has passed, so a refused zip writes
   nothing there.
4. Prints the kit's licence.

Then `python3 examples/borrowed/packs/make_revised.py` writes the one model the `revised` case
needs, and `via pack accept examples/borrowed/packs/kenney.toml` checks the files against the
digests in kenney.facts.toml.
"""

import hashlib
import http.cookiejar
import io
import json
import os
import posixpath
import shutil
import sys
import tempfile
import urllib.parse
import urllib.request
import zipfile

PAGE = "https://kenney.nl/assets/modular-buildings"
LINK = "https://kenney.nl/download-link/6173736574732f6d6f64756c61722d6275696c64696e6773"
SHA256 = "47e1614686b4c0fe55190e88b22ff2b8b10935edf61f37f478a9b6a99477edc6"
SIZE_MAX = 20_000_000          # the zip, as downloaded (the kit is 1,825,490 bytes)
ENTRY_MAX = 2_000_000          # one file unpacked (the largest the kit holds is 44,856 bytes)
UNPACKED_MAX = 20_000_000      # every file unpacked (the kit's 110 are 777,037 bytes)
MODELS = "Models/GLB format/"
TEXTURE = "Models/GLB format/Textures/colormap.png"
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "packs", "kenney")


class KenneyOnly(urllib.request.HTTPRedirectHandler):
    """Follows a redirect only to kenney.nl or a subdomain of it."""
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        host = urllib.parse.urlsplit(newurl).hostname or ""
        if not (host == "kenney.nl" or host.endswith(".kenney.nl")):
            raise SystemExit(f"kenney.nl redirected to {host}: refused")
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def download():
    jar = http.cookiejar.CookieJar()
    web = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar), KenneyOnly())
    web.addheaders = [("User-Agent", "Mozilla/5.0 (via examples/borrowed/fetch.py)")]
    web.open(PAGE, timeout=60).read()
    req = urllib.request.Request(LINK, headers={"Referer": PAGE})
    answer = json.loads(web.open(req, timeout=60).read())
    url = answer.get("url")
    if not url or not url.startswith("https://kenney.nl/"):
        raise SystemExit(f"kenney.nl gave no download link: {answer}")
    with web.open(url, timeout=120) as r:
        data = r.read(SIZE_MAX + 1)
    return data


def safe(name, root):
    """The path under `root` an entry is written to, or None where it would land outside it."""
    rel = posixpath.normpath(name)
    if rel.startswith("/") or rel.startswith("..") or "\\" in name:
        return None
    path = os.path.normpath(os.path.join(root, *rel.split("/")))
    return path if path.startswith(root + os.sep) else None


def extract(data, out=OUT):
    if len(data) > SIZE_MAX:
        raise SystemExit(f"the zip is over {SIZE_MAX:,} bytes: refused (the kit is 1,825,490)")
    digest = hashlib.sha256(data).hexdigest()
    if digest != SHA256:
        raise SystemExit(f"the zip's SHA-256 is {digest}, not {SHA256}: refused (the kit has changed "
                         "since this example was measured, or the download is not the kit)")
    parent = os.path.dirname(out)
    os.makedirs(parent, exist_ok=True)
    stage = tempfile.mkdtemp(prefix=".kenney-", dir=parent)
    try:
        os.makedirs(os.path.join(stage, "Textures"))
        licence, written, total = None, [], 0
        with zipfile.ZipFile(io.BytesIO(data)) as z:
            for info in z.infolist():
                name = info.filename
                if name == "License.txt":
                    target = "License.txt"
                elif name == TEXTURE:
                    target = "Textures/colormap.png"
                elif name.startswith(MODELS) and name.endswith(".glb") and "/" not in name[len(MODELS):]:
                    target = name[len(MODELS):]
                else:
                    continue
                path = safe(target, stage)
                if path is None:
                    raise SystemExit(f"refused: `{name}` would be written outside {out}")
                if info.file_size > ENTRY_MAX:
                    raise SystemExit(f"refused: `{name}` unpacks to {info.file_size:,} bytes, over {ENTRY_MAX:,}")
                with z.open(info) as src:
                    body = src.read(ENTRY_MAX + 1)
                if len(body) > ENTRY_MAX:
                    raise SystemExit(f"refused: `{name}` unpacks to over {ENTRY_MAX:,} bytes")
                total += len(body)
                if total > UNPACKED_MAX:
                    raise SystemExit(f"refused: the kit's files unpack to over {UNPACKED_MAX:,} bytes")
                with open(path, "wb") as f:
                    f.write(body)
                written.append(target)
                if target == "License.txt":
                    licence = body.decode("utf-8", "replace")
        if licence is None:
            raise SystemExit("refused: the zip holds no `License.txt`, so its licence is not stated")
        # Every check passed: move the files into place. Other files there (the `revised` case's
        # model, written by packs/make_revised.py) are left as they are.
        os.makedirs(os.path.join(out, "Textures"), exist_ok=True)
        for target in written:
            os.replace(os.path.join(stage, *target.split("/")), os.path.join(out, *target.split("/")))
    finally:
        shutil.rmtree(stage, ignore_errors=True)
    print(f"extracted {len(written)} files into {out}")
    print(licence)


def main(argv):
    if argv[1:2] == ["--zip"]:
        with open(argv[2], "rb") as f:
            data = f.read(SIZE_MAX + 1)
    else:
        data = download()
    extract(data)


if __name__ == "__main__":
    main(sys.argv)
