# engines/godot

The Godot loader and the sheet tool of `spec/build.md` 15.1. Godot release pinned in
`GODOT_VERSION` (4.7.2); the Compatibility renderer.

| Path | What |
|---|---|
| `addons/via_loader/` | The loader: `loader.gd` turns a build directory into one node tree; `plugin.gd` (Project > Tools > "Load via build...") saves it as a scene. `linear.gdshaderinc` hands a declared linear colour to ALBEDO on every renderer. |
| `addons/via_loader/stream.gd` | Streaming (build.md 15.1, M6): `loader.prepare` makes the root without chunks; the streamer makes each chunk (on a worker thread) and each generated mesh as a walker nears it and frees it past the unload radius, a guard making the chunk under the walker at once; past the load radius a coarse far ground (one mesh, coloured by the weights) and far instances (boxes of their bounds, one mesh) stand in for each chunk not loaded. `reload` takes a rebuild, keeping the loaded chunks whose `digest` is unchanged. A walk streams in lockstep, so it is the same on every run. |
| `sheet.tscn`, `sheet.gd`, `compose.gd` | The sheet tool: every bookmark at eye height, before and after, on one sheet. |
| `walk.gd`, `marks.gd` | The sheet tool's walker (a capsule walked from each bookmark toward its target: arrived, blocked, fell) and its marks for the hand edits of the after build's change report (pins magenta, clears cyan). They write `walks.png` and `<side>/walk/`. |
| `rig.gd` | The sheet's light and environment; exposure lives here, checked by `tests/test_exposure.gd`. |
| `sheet.sh` | Runs the sheet tool with a watchdog. `--stream RADIUS` streams each build; `--frames SECONDS` adds the frame tour (frame times in `sheet.json` `frames`). |
| `tests/` | The loader suite and its hand-made fixture builds. `make_floor_fixture.py DIR` writes a synthetic build with a cut terrain and four kind surface meshes (a floor at 3 m, a tread, a 0.25 m step, a 0.6 m plinth); `test_floor.gd` (headless) loads it and walks on them. `test_stream.gd` (headless) streams a build: content once, a long walk round the extent loading and unloading chunks, then load, unload, load. `test_stream_walks.gd` (headless) walks every bookmark whole and streamed: reproducible, same outcome and blocker, stop within a frame's pace. `test_reload.gd` (headless) reloads a rebuild by chunk digest. `compare_views.py WHOLE STREAMED` compares two sheets of one build (at most 1% of pixels differ). |

```
engines/godot/sheet.sh --before build/ford --after build/ford+downstream --out review/ford \
    --option water=tag:water_level --option paths=kind:lane
engines/godot/tests/run.sh
engines/godot/sheet.sh --before build/city --out review/city_frames --option water=tag:water_level --stream 96 --frames 30
godot --headless --fixed-fps 60 --path engines/godot --script res://tests/test_stream.gd -- --build build/vale --log stream.json
```

Both need a display: Godot's `--headless` draws nothing and keeps no `MultiMesh` data. The fixture
builds are written by `tests/make_fixture.py` (Python's standard library only); `--check` says
whether the files on disk still match it. The floor test is the one part that runs headless: it reads data and physics only, never a render.
