# strata512 — layered stratigraphy at research scale

Declared forcing only (ADR 0003 / 0007): the core knows nothing about
this experiment. Everything below is a statement about the *column*, not
about landforms.

## The column

Ten units, alternating soft (k_mult 1.0) and resistant (k_mult 0.35,
kappa_mult 0.5), 1500 m each, spanning 15 km of section — the full
exhumation range of the research preset (uplift up to 0.5 mm/yr ×
30 Myr). A thinner column would be swept entirely past the surface in
the uplift core, which is the material frame being honest: mountain
cores really do erase their cover. One resistant unit (index 3) is
soluble (0.7) to exercise the karst-potential spectrum. The bottom unit
is moderately resistant basement.

Structure: regional dip [0.02, 0.008] (≈1.2°; up to ~2.0 km of
displacement along x plus ~0.8 km along y ≈ 2.9 km corner-to-corner),
one 600 m / 30 km fold train at azimuth 40°, one 900 m fault at azimuth
70° through (−8, +4) km.

## What is expected to emerge (never drawn)

Steady-state channel slopes scale as 1/K: resistant bands should carry
≈2.9× steeper channels at matched discharge, read as escarpments and
knickzones where rivers cross contacts; exposure bands follow the
dome's exhumation rings, tilted by the dip and offset across the fault.
The unit_spl_consistency advisory reports whether each unit obeys the
incision law it was evolved under.

## Runs

```
target/release/via terrain --config experiments/strata512/terrain.json --seed 42 --out runs/m4-strata512-s42
```

Gates must pass as on the homogeneous research preset; the θ regression
restricts itself to the modal exposed unit (ADR 0007).

sediment_k_mult stays at its neutral default here: one physics variable
at a time — the alluvial-K contrast is its own experiment when a
consumer needs it.
