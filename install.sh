#!/bin/sh
# install.sh: build via from this checkout and install it into one folder (docs/install.md).
#
#   ./install.sh                                  # into ~/.via
#   ./install.sh --prefix DIR                     # into DIR (empty, new, or an earlier install)
#   ./install.sh --prefix .via-tools --godot game # a project's own copy, and the Godot loader
#   ./install.sh --uninstall [--prefix DIR]       # remove an install
#
# What lands in the prefix:
#   bin/via            runs libexec/via with this install's settings folder (config/)
#   libexec/           via and via-terrain, built in release from this checkout
#   lib/               the libraries, found by via as its shipped root (documents.md 4)
#   python/            a Python with NumPy for the library steps (or --python, yours)
#   config/            programs and trust (protocol.md 7), this install's own
#   env.sh             `. env.sh` puts bin/ on PATH
# Running it again rebuilds and replaces all but python/ and config/. macOS and Linux; on Windows,
# docs/install.md gives the steps by hand.
set -eu

SRC=$(cd "$(dirname "$0")" && pwd)
PREFIX="$HOME/.via"
GODOT=""
PYTHON=""
UNINSTALL=""
MARK=".via-install"

say() { printf '%s\n' "$*"; }
die() { printf 'install.sh: %s\n' "$*" >&2; exit 1; }

usage() {
    sed -n '2,9p' "$0" | sed 's/^# \{0,1\}//'
    exit "${1:-0}"
}

while [ $# -gt 0 ]; do
    case "$1" in
        --prefix) [ $# -ge 2 ] || die "--prefix takes a folder"; PREFIX=$2; shift 2 ;;
        --prefix=*) PREFIX=${1#--prefix=}; shift ;;
        --godot) [ $# -ge 2 ] || die "--godot takes a Godot project folder"; GODOT=$2; shift 2 ;;
        --godot=*) GODOT=${1#--godot=}; shift ;;
        --python) [ $# -ge 2 ] || die "--python takes an interpreter"; PYTHON=$2; shift 2 ;;
        --python=*) PYTHON=${1#--python=}; shift ;;
        --uninstall) UNINSTALL=1; shift ;;
        -h|--help) usage 0 ;;
        *) say "install.sh: unknown argument \`$1\`" >&2; usage 2 >&2 ;;
    esac
done

case "$PREFIX" in
    /*) ;;
    *) PREFIX="$(pwd)/$PREFIX" ;;
esac
PREFIX=${PREFIX%/}
[ -n "$PREFIX" ] || die "the prefix cannot be /"

# Only a folder this script made is ever replaced or removed: it holds the mark.
if [ -n "$UNINSTALL" ]; then
    [ -d "$PREFIX" ] || die "nothing installed at $PREFIX"
    [ -f "$PREFIX/$MARK" ] || die "$PREFIX was not made by install.sh (no $MARK); remove it yourself"
    rm -rf "$PREFIX"
    say "Removed $PREFIX."
    say "A Godot project's addons/via_loader and the PATH line in your shell's profile stay; remove them yourself."
    exit 0
fi

if [ -e "$PREFIX" ] && [ ! -f "$PREFIX/$MARK" ]; then
    [ -d "$PREFIX" ] || die "$PREFIX exists and is not a folder"
    [ -z "$(ls -A "$PREFIX")" ] || die "$PREFIX holds other files; name an empty or new folder"
fi

if [ -n "$GODOT" ]; then
    [ -f "$GODOT/project.godot" ] || die "$GODOT holds no project.godot"
    GODOT=$(cd "$GODOT" && pwd)
    old="$GODOT/addons/via_loader"
    if [ -e "$old" ] && ! grep -q '^name="via loader"' "$old/plugin.cfg" 2>/dev/null; then
        die "$old exists and is not via's loader"
    fi
fi

command -v cargo >/dev/null 2>&1 || die "cargo not found: install Rust from https://rustup.rs (the toolchain is pinned in rust-toolchain.toml)"

# ---------------------------------------------------------------------------------------- build
say "Building via and via-terrain (release) in $SRC ..."
(cd "$SRC" && cargo build --release --locked -p via-cli -p via-terrain)
TARGET=${CARGO_TARGET_DIR:-$SRC/target}
case "$TARGET" in /*) ;; *) TARGET="$SRC/$TARGET" ;; esac
for b in via via-terrain; do
    [ -x "$TARGET/release/$b" ] || die "the build left no $TARGET/release/$b"
done

# -------------------------------------------------------------------------------------- install
mkdir -p "$PREFIX"
# The prefix ignores itself, so it may sit inside a game's repository.
printf '*\n' > "$PREFIX/.gitignore"
printf 'via install; made by %s/install.sh\n' "$SRC" > "$PREFIX/$MARK"

rm -rf "${PREFIX:?}/bin" "${PREFIX:?}/libexec" "${PREFIX:?}/lib"
mkdir -p "$PREFIX/bin" "$PREFIX/libexec" "$PREFIX/config"
cp "$TARGET/release/via" "$TARGET/release/via-terrain" "$PREFIX/libexec/"

# The libraries: the tracked files only, so no cache or scratch file changes a trust digest.
if git -C "$SRC" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    (cd "$SRC" && git ls-files -z -- lib | tar -cf - --null -T -) | (cd "$PREFIX" && tar -xf -)
else
    cp -R "$SRC/lib" "$PREFIX/lib"
    find "$PREFIX/lib" -name __pycache__ -type d -prune -exec rm -rf {} +
fi

cat > "$PREFIX/bin/via" <<EOF
#!/bin/sh
# Written by install.sh: this install's via, with its own settings folder unless VIA_CONFIG_DIR is set.
VIA_CONFIG_DIR="\${VIA_CONFIG_DIR:-$PREFIX/config}"
export VIA_CONFIG_DIR
exec "$PREFIX/libexec/via" "\$@"
EOF
chmod +x "$PREFIX/bin/via"

cat > "$PREFIX/env.sh" <<EOF
# Written by install.sh: \`. $PREFIX/env.sh\` puts this install's via first on PATH.
case ":\$PATH:" in
    *":$PREFIX/bin:"*) ;;
    *) PATH="$PREFIX/bin:\$PATH"; export PATH ;;
esac
EOF

# --------------------------------------------------------------------------------------- python
# The library steps run on Python 3.11+ with NumPy; the SDK itself is carried by via (protocol.md 8).
has_numpy() { "$1" -c 'import sys, numpy; sys.exit(sys.version_info < (3, 11))' >/dev/null 2>&1; }

if [ -n "$PYTHON" ]; then
    has_numpy "$PYTHON" || die "$PYTHON is not Python 3.11+ with NumPy"
    case "$PYTHON" in /*) ;; *) PYTHON=$(command -v "$PYTHON") ;; esac
else
    PYTHON="$PREFIX/python/bin/python"
    if ! has_numpy "$PYTHON"; then
        rm -rf "$PREFIX/python"
        if command -v uv >/dev/null 2>&1; then
            say "Making $PREFIX/python (uv, Python 3.12, NumPy) ..."
            uv venv --quiet --python 3.12 "$PREFIX/python"
            uv pip install --quiet --python "$PYTHON" "numpy>=1.26"
        else
            sys_py=""
            for c in python3.13 python3.12 python3.11 python3; do
                if command -v "$c" >/dev/null 2>&1 \
                    && "$c" -c 'import sys; sys.exit(sys.version_info < (3, 11))' 2>/dev/null; then
                    sys_py=$c; break
                fi
            done
            [ -n "$sys_py" ] || die "no Python 3.11+ found: install uv (https://docs.astral.sh/uv/) or Python 3.11+, or pass --python"
            say "Making $PREFIX/python ($sys_py, NumPy) ..."
            "$sys_py" -m venv "$PREFIX/python"
            "$PYTHON" -m pip install --quiet "numpy>=1.26"
        fi
        has_numpy "$PYTHON" || die "$PREFIX/python has no NumPy"
    fi
fi

"$PREFIX/bin/via" config program python "$PYTHON" >/dev/null
"$PREFIX/bin/via" config program via-terrain "$PREFIX/libexec/via-terrain" >/dev/null

# ---------------------------------------------------------------------------------------- godot
if [ -n "$GODOT" ]; then
    rm -rf "$GODOT/addons/via_loader"
    mkdir -p "$GODOT/addons"
    cp -R "$SRC/engines/godot/addons/via_loader" "$GODOT/addons/via_loader"
fi

# --------------------------------------------------------------------------------------- report
version=$("$PREFIX/bin/via" --version)
say ""
say "Installed $version at $PREFIX"
say "  python:      $PYTHON"
say "  via-terrain: $PREFIX/libexec/via-terrain"
[ -z "$GODOT" ] || say "  Godot loader: $GODOT/addons/via_loader (enable \"via loader\" in Project > Project Settings > Plugins)"
if [ -n "${VIA_LIB:-}" ]; then
    say ""
    say "VIA_LIB is set ($VIA_LIB): it is searched before this install's lib/. Unset it to use the installed libraries."
fi
say ""
case ":$PATH:" in
    *":$PREFIX/bin:"*) say "$PREFIX/bin is on your PATH." ;;
    *)
        say "Put it on your PATH, now and in your shell's profile (~/.zshrc, ~/.bashrc):"
        say "  . \"$PREFIX/env.sh\""
        ;;
esac
say "Then: via trust <world.toml> --yes && via build <world.toml> --out build/w   (docs/install.md)"
