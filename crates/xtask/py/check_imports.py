"""Library code imports only the SDK and the standard library (R19b, CONTRIBUTING.md).

Run by `cargo run -p xtask -- check` through uv, with an isolated interpreter (`python -I`), so
nothing from the repository or a user's site-packages is importable while it runs. It parses each
file with `ast` rather than matching import lines, so an import inside a function, a
parenthesised `from x import (a, b)` or a line continuation is read the same as any other.

Usage: check_imports.py LIB_ROOT FILE...
Prints one JSON object per finding: {"path", "line", "message"}. Exit 0 when it could read every
file (findings or not), 2 on a usage error.
"""

import ast
import json
import os
import sys

SDK = "viasdk"
# `__future__` is a compiler directive, not a module a library reaches the engine through.
ALLOWED = {SDK, "__future__"} | set(sys.stdlib_module_names)


def library_of(lib_root, path):
    """The library folder (`lib/<name>`) a file belongs to, or None for a file at lib's top."""
    rel = os.path.relpath(path, lib_root)
    parts = rel.split(os.sep)
    return os.path.join(lib_root, parts[0]) if len(parts) > 1 else None


def check_file(lib_root, path):
    out = []

    def add(node, message):
        out.append({"path": path, "line": getattr(node, "lineno", 0), "message": message})

    try:
        with open(path, "rb") as f:
            tree = ast.parse(f.read(), filename=path)
    except SyntaxError as e:
        out.append({"path": path, "line": e.lineno or 0, "message": f"does not parse: {e.msg}"})
        return out

    library = library_of(lib_root, path)
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                top = alias.name.split(".")[0]
                if top not in ALLOWED:
                    add(node, f"imports `{alias.name}`: library code imports only {SDK} and the standard library")
        elif isinstance(node, ast.ImportFrom):
            if node.level == 0:
                top = (node.module or "").split(".")[0]
                if top not in ALLOWED:
                    add(node, f"imports from `{node.module}`: library code imports only {SDK} and the standard library")
                continue
            # A relative import stays inside its own library folder, or it is a door into another.
            base = os.path.dirname(os.path.abspath(path))
            for _ in range(node.level - 1):
                base = os.path.dirname(base)
            if library is None or os.path.commonpath([base, os.path.abspath(library)]) != os.path.abspath(library):
                add(node, f"relative import of level {node.level} leaves its library folder")
        elif isinstance(node, ast.Call):
            f = node.func
            name = f.id if isinstance(f, ast.Name) else f.attr if isinstance(f, ast.Attribute) else None
            if name in ("__import__", "import_module"):
                add(node, f"calls `{name}`: a dynamic import cannot be checked; import {SDK} by name")
    return out


def main(argv):
    if len(argv) < 2:
        print(__doc__, file=sys.stderr)
        return 2
    lib_root = os.path.abspath(argv[1])
    for path in sorted(argv[2:]):
        for finding in check_file(lib_root, path):
            print(json.dumps(finding, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
