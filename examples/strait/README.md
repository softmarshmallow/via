# strait

Two islands built apart, joined across a strait (docs/plan.md, M4; R8 to R11, spec/composition.md).
Rebuilt from the spike's `strait` in the v1 vocabulary: the spike placed `[[part]]`s with an
`apron` falloff and a bridge between `a.quay_road.end` and `b.landing.start`; here each island is
an `[import.<id>]`, placed by `at` and `face`, sharing `datum = "sea"`, its ground blended in by a
formula that reads the island's own height, and the crossing is a ferry between two slipways that
each island declares as an exit.

| File | What |
|---|---|
| `world.toml` | Layer `world`: the strait's sea bed, water and paint, `[import.a]`, `[import.b]` (B turned, `face = 90`), the light. |
| `layers/crossing.toml` | Layer `crossing` (rank 1, above the imports, composition.md 6.5): the ferry `a.slip` to `b.slip`, and twelve views (two brought from the islands). |
| `a/`, `b/` | The islands: each its own world with `world.toml` and `changes/`, built alone. |
| `changes/` | `move_b` (B moved and turned to 80 degrees), `harbour` (asks for A's own change `changes/harbour.toml` by path), `forgotten_exit` (asks for A's `second_slip`, an exit nobody answers). |
| `expect.toml`, `measure.py` | The measures, with protocol. |

## Build

```
export VIA_LIB=$PWD/lib
via build examples/strait/a/world.toml --out build/strait_a          # each island alone
via build examples/strait/world.toml --out build/strait
via build examples/strait/world.toml --change examples/strait/changes/move_b.toml --against build/strait --out build/strait+move_b
python3 examples/strait/measure.py build/strait+move_b --run build/strait+move_b.run.json --alone b=build/strait_b
engines/godot/sheet.sh --before build/strait --after build/strait+move_b --out review/strait --option water=tag:water_level
```

## Known faults (2026-11-16, walked on the move_b sheet)

- The islands' paint is not brought: a `weights` field is matched but the parent's `ground` holds
  only its own `sand` (manifest `materials`), so both islands read as sand dunes, the lanes unseen.
  No diagnostic says so (composition.md 6.2 wants each material's weight read).
- Each slipway reads as a square sand platform with a 2:1 face into the water: the kind is a
  point with no direction, so it cannot lay a ramp down into the sea (`lib/ways` `slipway`).
- A change inside an island re-runs all 180 field chunks of the strait, and a move of B does too:
  no `brought` node per chunk yet (composition.md 12).
