# engines/godot

The Godot loader and the sheet tool of `spec/build.md` 15.1. Godot release pinned in
`GODOT_VERSION` (4.7.2); the Compatibility renderer.

| Path | What |
|---|---|
| `addons/via_loader/` | The loader: `loader.gd` turns a build directory into one node tree; `plugin.gd` (Project > Tools > "Load via build...") saves it as a scene. `linear.gdshaderinc` hands a declared linear colour to ALBEDO on every renderer. |
| `addons/via_loader/stream.gd` | Streaming (build.md 15.1, M6): `loader.prepare` makes the root without chunks; the streamer makes each chunk (on a worker thread) and each generated mesh as a walker nears it and frees it past the unload radius, a guard making the chunk under the walker at once; past the load radius a coarse far ground (posts 8 m apart, one mesh, coloured by the weights, with skirts under its edges and the far water), far instances (each asset's outline in three height bands, each a frustum between octagonal rings, one mesh) and a bodiless copy of each generated mesh stand in for what is not loaded. `reload` takes a rebuild, keeping the loaded chunks whose `digest` is unchanged. A walk streams in lockstep, so it is the same on every run. |
| `sheet.tscn`, `sheet.gd`, `compose.gd` | The sheet tool: every bookmark at eye height, before and after, on one sheet. |
| `walk.gd`, `marks.gd` | The sheet tool's walker (a capsule walked from each bookmark toward its target: arrived, blocked, fell; `blocked` always names what stopped it, `ground` with its slope when it pressed against nothing steeper than it walks) and its marks for the hand edits of the after build's change report (pins magenta, clears cyan). They write `walks.png` and `<side>/walk/`. |
| `rig.gd` | The sheet's light and environment; exposure lives here, checked by `tests/test_exposure.gd`. |
| `sheet.sh` | Runs the sheet tool with a watchdog. `--stream RADIUS` streams each build; `--frames SECONDS` adds the frame tour (frame times in `sheet.json` `frames`); `--page N` lays the sheet and walks out in pages of N bookmarks (20 by default: `sheet.png`, `sheet_2.png`, ...). `--pack DIR` names the pack's root for a build whose `locations.pack_root` is null (`build.pack_root_far`, a pack on another drive); a written `pack_root` wins. What lands in `--out`: `before/<bookmark>.png` and `after/<bookmark>.png` (each view at eye height), `before/walk/` and `after/walk/` (each walk's plan), `<side>/map.png` (the player map, when the build exports one), `sheet.png` (and `sheet_2.png`, ...), `walks.png`, and `sheet.json` (per view: the walk's outcome, `on.stop`, what blocked it). An import's own bookmarks are on the sheet too, under their scoped names. |
| `addons/via_loader/minimap.gd` | The player map (build.md 9.4, 15.1) as a corner map turning with the walker and a whole map at a toggle (M), per floor, read from the build's `player_map` export. |
| `tests/` | The loader suite and its hand-made fixture builds. `make_floor_fixture.py DIR` writes a synthetic build with a cut terrain and four kind surface meshes (a floor at 3 m, a tread, a 0.25 m step, a 0.6 m plinth); `test_floor.gd` (headless) loads it and walks on them. `test_stream.gd` (headless) streams a build: content once, a long walk round the extent loading and unloading chunks, then load, unload, load. `test_stream_walks.gd` (headless) walks every bookmark whole and streamed: reproducible, same outcome and blocker, stop within two frames' pace (`--water` to wade); an import's bookmarks are walked too, under their scoped names. Run on a build: `godot --headless --path engines/godot --script res://tests/test_stream_walks.gd -- --build DIR [--radius 96] [--water tag:water_level] [--log FILE]`. `test_far.gd` (headless) checks the far layer; `test_minimap.gd` (headless) the minimap on `fixture_map` (station's export) or any build with a player map (`--build`). The sheet draws the player map to `<side>/map.png` and the corner map on each walk's eye view. `test_reload.gd` (headless) reloads a rebuild by chunk digest. `compare_views.py WHOLE STREAMED [--jobs N]` compares two sheets of one build (at most 1% of pixels differ) and, when STREAMED was drawn with `sheet.sh --stream R --far-mask`, says per view what the differing pixels are on: far instances, the far ground, or loaded content. |

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
