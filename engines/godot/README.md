# engines/godot

The Godot loader and the sheet tool of `spec/build.md` 15.1. Godot release pinned in
`GODOT_VERSION` (4.7.2); the Compatibility renderer.

| Path | What |
|---|---|
| `addons/via_loader/` | The loader: `loader.gd` turns a build directory into one node tree; `plugin.gd` (Project > Tools > "Load via build...") saves it as a scene. |
| `sheet.tscn`, `sheet.gd`, `compose.gd` | The sheet tool: every bookmark at eye height, before and after, on one sheet. |
| `sheet.sh` | Runs the sheet tool with a watchdog. |
| `tests/` | The loader suite and its hand-made fixture builds. |

```
engines/godot/sheet.sh --before build/ford --after build/ford+downstream --out review/ford \
    --option water=tag:water_level
engines/godot/tests/run.sh
```

Both need a display: Godot's `--headless` draws nothing and keeps no `MultiMesh` data. The fixture
builds are written by `tests/make_fixture.py` (Python's standard library only); `--check` says
whether the files on disk still match it.
