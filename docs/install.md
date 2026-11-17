# Installing via

`via` is built from source: there is no package yet (section 6). `install.sh` at the repository's
root builds it and installs it into one folder, its prefix. The same script serves the two ways of
using it: installed for yourself from a clone or a fork (section 2), or as a copy inside a game's
own repository (section 3). It runs on macOS and Linux; on Windows the steps are done by hand
(section 7).

## 1. What you need

| What | Why | Where from |
|---|---|---|
| Rust (rustup) | Builds `via` and `via-terrain`. The toolchain is pinned in `rust-toolchain.toml` (1.92); rustup fetches it on the first build. | https://rustup.rs |
| uv, or Python 3.11+ | The library steps run on Python with NumPy. With uv the script makes a Python 3.12; without it, a venv from the newest `python3.11` to `python3.13` found. `--python PATH` names one of yours that has NumPy instead. | https://docs.astral.sh/uv/ |
| Godot 4.7 (optional) | To load a build in a game. The loader is tested on 4.7.2 (`engines/godot/GODOT_VERSION`), Compatibility renderer. | https://godotengine.org |

The SDK the steps import (`viasdk`) is carried inside the `via` binary and put on each step's path
(`spec/protocol.md` 8.1): nothing is installed into Python but NumPy.

## 2. For yourself, from a clone or a fork

```sh
git clone https://github.com/softmarshmallow/via      # or your fork
cd via
./install.sh                                          # builds, then installs into ~/.via
. ~/.via/env.sh                                       # puts ~/.via/bin on PATH; add this line to ~/.zshrc or ~/.bashrc
```

The first run compiles the whole engine; a later run compiles only what changed. Then build a world (a copy of
one under `examples/`, its files and never its `.via/` store):

```sh
cp -R examples/ford ~/worlds/ford && rm -rf ~/worlds/ford/.via
via trust ~/worlds/ford/world.toml --yes             # once per world, again after its code changes
via build ~/worlds/ford/world.toml --out ~/worlds/ford-build
```

`--prefix DIR` installs elsewhere: a new folder, an empty one, or an earlier install. The script
refuses any other folder, so it never writes over files it did not make.

**A fork.** Clone your fork and keep the original as a second remote:

```sh
git remote add upstream https://github.com/softmarshmallow/via
git pull upstream main                               # take upstream's changes
./install.sh                                          # and install them, with your own edits
```

Your edits to `lib/` are installed as they are. Trust is by digest (`spec/protocol.md` 7), so a
library you change asks to be trusted again in each world that uses it.

## 3. Inside a game's repository

A game that pins its own `via` keeps it as a git submodule and installs it into a folder of the
game's:

```sh
git submodule add https://github.com/softmarshmallow/via third_party/via
third_party/via/install.sh --prefix .via-tools --godot godot
. .via-tools/env.sh
```

- **`.via-tools/`** holds the binaries, the libraries, a Python and this install's settings. It
  ignores itself (it writes its own `.gitignore`), so nothing in it is committed. Each person runs
  the same line once after `git submodule update --init`.
- **`--godot godot`** copies the loader into `godot/addons/via_loader/` (the folder holding
  `project.godot`). That copy is committed with the game: Godot needs it to open and to export the
  project. Enable "via loader" once under Project > Project Settings > Plugins.
- **The version** is the submodule's commit. To update: `git -C third_party/via pull` (or
  `git submodule update --remote`), run the install line again, and commit the submodule and the
  loader together.
- **Settings are per install.** `.via-tools/bin/via` keeps its programs and trust in
  `.via-tools/config/`, so two games, or a game and your own `~/.via`, never share or overwrite
  them. A `VIA_CONFIG_DIR` you set wins over it.

A fork works the same way: add your fork as the submodule.

## 4. Into the engine

| Engine | How a build gets in | What you get |
|---|---|---|
| Godot | The loader, `addons/via_loader/` (`--godot DIR`, or copy `engines/godot/addons/via_loader/` yourself). At run time it loads a build directory, whole or streamed; in the editor, Project > Tools > "Load via build..." saves one as a scene. | Everything a build holds: streaming by chunk, the far stand-ins, reload of changed chunks after a rebuild, the player map, collision (`spec/build.md` 15.1). |
| Any other | `python3 engines/gltf/export.py BUILD --out world.glb` (standard library only), from the clone. | One glTF scene of the whole build: no streaming, no reload, no player map (`spec/build.md` 15.3). |

## 5. What goes where

| In the prefix | What |
|---|---|
| `bin/via` | Runs `libexec/via` with this install's settings folder. |
| `libexec/via`, `libexec/via-terrain` | The engine and the erosion program, built in release. |
| `lib/` | The libraries. `via` finds them beside its binary (`spec/documents.md` 4, the shipped root), so no `VIA_LIB` is needed. A `VIA_LIB` you set is searched first; the script warns when one is set. |
| `python/` | A Python with NumPy, named as the program `python`. Kept when the script runs again. |
| `config/` | `config.toml` (the programs `python` and `via-terrain`) and `trust.toml`. Kept when the script runs again. |
| `env.sh` | `. env.sh` puts `bin/` first on PATH. |

Running the script again rebuilds and replaces `bin/`, `libexec/` and `lib/`. `./install.sh
--uninstall [--prefix DIR]` removes a prefix the script made, and only such a folder; the loader
copied into a Godot project and the line in your shell's profile stay.

## 6. Not yet

- **crates.io and PyPI.** Deferred. The crates name each other by path and carry no publishing
  metadata. `viasdk` has a `pyproject.toml` but is not published, and is not needed to run steps
  (it is carried by the binary).
- **Released binaries.** None. A released binary would carry its libraries' digests and trust them
  without `via trust` (`spec/protocol.md` 7); an install from source trusts by `via trust`, as here.
- **An Unreal loader.** None; Unreal reads the glTF export.

## 7. By hand, on Windows, or while working on via itself

The steps `install.sh` takes, in a clone, without installing anything: the binaries stay in
`target/release`, and `VIA_LIB` names the repository's `lib/` (`spec/documents.md` 4.1). This is
also the setup for changing `via` itself (`CONTRIBUTING.md`).

```sh
cargo build --release -p via-cli -p via-terrain
export VIA_LIB=$PWD/lib                             # a cargo-built via has no shipped root
export PATH=$PWD/target/release:$PATH                # or your --target-dir's release folder
uv sync --project sdk/python --python 3.12          # a Python 3.12 with NumPy (sdk/python/.venv)
via config program python $PWD/sdk/python/.venv/bin/python
via config program via-terrain $PWD/target/release/via-terrain
```

On Windows the same, with `set`/`$env:` for the variables, `target\release\via.exe`, and
`sdk\python\.venv\Scripts\python.exe`. These settings go to the user's settings folder
(`~/Library/Application Support/via` on macOS, `~/.config/via` on Linux, `%APPDATA%\via` on
Windows), shared by every `via` that does not set `VIA_CONFIG_DIR`.
