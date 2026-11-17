# The erosion model

What `crates/via-terrain` computes and why. The step around it (`settle.rs`, `facts.rs`,
`wire.rs`) turns stated facts into the model's starting conditions and checks them afterwards
(`docs/doctrine.md` R14, `spec/protocol.md`); this page is about the model itself: `stage.rs` and
what it calls. Code comments cite it by section, as `model.md 4.2`.

## 1. The model

### 1.1 The equation

    ∂h/∂t = U − K·√Q·S + κ∇²h + D

- U is uplift (m/yr), a field given from outside (section 3). Run as a step, it is zero.
- K·√Q·S is fluvial incision by stream power, with m = ½ and n = 1 fixed. Q is discharge in
  equivalent cells (section 2.3); the code takes √Q as `dx·sqrt(Q)`, so K keeps its units and its
  calibration whatever the rainfall.
- κ∇²h is hillslope diffusion: an explicit 4-neighbour Laplacian, subcycled so that
  κ·dt/dx² ≤ 0.24 in each subcycle. Base cells are fixed (Dirichlet); the grid's edges pass no
  flux (Neumann).
- D is deposition (section 4.1).

h is the true surface, bedrock plus sediment (section 4.3). Flow is routed on a second, filled
copy of it (section 4.2) along a weighted multiple-flow-direction graph (section 5).

### 1.2 One step

In order, each step:

1. Uplift: h += U·dt. Subsidence (U < 0) stops at `base_depth_m`. Without that floor, open-sea
   cells with negative uplift deepen without end, and diffusion drags every cell of a sinking
   coast down with them each step: a band that never converges, at any resolution.
2. The border ring is pinned at `base_depth_m` and held cells at their level; sediment on either
   has left the system.
3. Base level is found: the sea (cells below sea level connected to the border through such
   cells) plus held cells. A pocket below sea level that does not reach the border is not base
   level; it floods as a lake (section 4.2).
4. The routing copy is filled (priority-flood + ε, Barnes et al. 2014) and shallow ponds merge
   into the true surface (section 4.2). The flooded mask is frozen here, before erosion.
5. Precipitation is computed on the routing copy (section 2), then discharge on the routing graph
   (section 5).
6. The implicit fluvial solve (section 5.4) lowers h and reports the depth detached per cell;
   detachment takes sediment first, then bedrock.
7. The sediment sweep routes what was detached and deposits part of it (section 4.1).
8. Diffusion runs; a cell it lowers loses sediment first, a cell it raises gains sediment.
9. max |Δh| and mean |Δh| over the step are recorded: the convergence trace.

After the last step the routing, precipitation, temperature and discharge are computed once more
on the final surface, which is what the outputs describe. One step's detachment and sediment
sweep is then replayed on scratch copies, only to measure the deposition rate the residual gate
needs (section 6); the real state is not touched.

### 1.3 Limits on the time step

The fluvial solve is implicit and stable at any dt. The step as a whole is not, because its parts
are applied one after another within it:

- **Diffusion against uplift.** Splitting erosion from diffusion is faithful only while
  κ·∇²h·dt ≲ U·dt. Fine cells bring large curvatures, so they need short steps.
- **Deposition against detachment.** Deposition is explicit: at steady state it moves G·U·dt
  per channel cell against (1+G)·U·dt of implicit detachment. When G·U·dt is no longer small
  against valley relief, the erode-then-deposit split oscillates, and the oscillation feeds back
  through the sediment flux: max |Δh| grows and peaks run away. With G = 1 and the default uplift,
  dt = 1e5 yr diverges (G·U·dt = 50 m a step); the default dt = 1e4 yr (5 m a step) is stable.

The convergence trace and the residual gate (section 6) catch a configuration that breaks either
limit. The residual gate is core for this reason: a bad dt must fail, not warn.

## 2. Climate as forcing

Orographic rain depends on the relief it helps carve. The loop is closed inside the model:
precipitation is recomputed against the evolving surface every step, so the step hands back one
consistent pair of ground and climate, and nothing outside the step feeds back into it.

### 2.1 Wind

One of the 8 grid directions, drawn from the seed (`sub_seed(seed, "climate", 0) % 8`), like any
other boundary condition.

### 2.2 Precipitation

A single-layer moisture sweep along the wind, the standard first-order orographic model. Each line
of cells parallel to the wind starts at the upwind border with saturated air (humidity 1) and is
walked cell by cell:

- Over water (h below sea level): the air recovers a fraction `evaporation_per_cell` of its
  humidity deficit, and rain is `convective_rainout_per_cell` × humidity.
- Over land: rain is min(r·humidity, humidity) with
  r = `convective_rainout_per_cell` + `orographic_rainout_per_100m` × lift / 100 m,
  and the air loses what fell. Ridges therefore dry the air, and the lee lies in a rain shadow.
- Lift is the rise of an exponential moving average of the ground along the line, with
  α = 1 − exp(−step / L), L = `airflow_smooth_m`. The air rides over gullies; without the
  smoothing, rain traces every valley wall on fine cells.

The sweep runs on the routing copy, so air passes over a lake's surface, not its bed. Where two
lines of a diagonal wind visit a cell, the larger value is kept. Two passes of a 3×3 box blur
then remove the stripes the lines leave. Last, the field is scaled so its mean over land equals
`precip_mean_m_per_yr`. Because of that scaling, only the ratios of the sweep coefficients matter.

Defaults: evaporation 0.06, convective rainout 0.0015, orographic rainout 0.055 per 100 m,
airflow length 600 m (three cells in the cell-size presets of `scale.rs`), land mean 1.2 m/yr.

### 2.3 Erosion runs on discharge

Each cell contributes its precipitation relative to the land mean, floored at 0.05 so that full
rain shadow still drains a trickle rather than leaving a channel with no water. Accumulated over
the routing graph this is the discharge Q, in equivalent cells, which drives incision. So wet
windward flanks erode faster than dry lee flanks. With rainfall that varies in space, the steady
channel slope is S = U/(K√Q): the slope–area gate regresses on discharge, not area, which would
mix climates.

### 2.4 Temperature

Mean annual, at the end of the run:

    T = T_sea − lapse · max(h − sea, 0) / 1000 + ΔT · (v − ½) + 0.6 · fbm

with v the row position across the grid (0 to 1) and the noise drawn from
`sub_seed(seed, "climate", 1)`. Defaults: T_sea 15 °C, lapse 6.5 °C/km, ΔT 0.6 °C. It is computed
on the routing copy and is an output only; erosion does not read it. There is no seasonality.

## 3. Modelled science and stated preference

The model keeps three kinds of quantity apart.

- **Process**: stream power, diffusion, flow routing, sediment transport. These are falsifiable
  mechanisms. The gates (section 6) measure this kind only; a failing gate means the code or its
  calibration is wrong.
- **Forcing**: what a bounded domain cannot derive for itself: the starting surface, uplift,
  held cells, sea level, wind, and the climate's coefficients. Forcing is given from outside, in
  `TerrainConfig` or `Given`, never hard-coded. It is carried through the model by mechanism (a
  rain shadow comes from the moisture sweep; it is never painted) and is never claimed as
  emergent. Forcing chosen to suit a world (a compressed climate, a steep lapse rate) is
  legitimate because it is stated.
- **Interpretation**: classes drawn over the fields. The model ships spectra (water depth,
  discharge, sediment thickness) and leaves the thresholds to whoever reads them, each with its
  own stated constant: `settle.rs` treats water over 0.05 m as standing, for example. The model's
  own cut-offs are numerical guards, not classes: a cell is flooded for the physics above 1 mm
  (`FLOOD_EPS_M`), because the ε-fill raises micro-depressions by about 1e-6 m and those must
  behave as dry land. The 0.3 m lake count in the gates report is a reference statistic, not a
  class.

What an author states (R14) reaches the model only as forcing: shape in the starting surface and
held cells, decided by `settle.rs`. The model runs once on them; whether the facts held is checked
afterwards on the finished build, by `facts.rs`.

## 4. Sediment and standing water

### 4.1 Erosion and deposition

Erosion–deposition follows Davy & Lague (2009) in the G-coefficient form of Yuan et al. (2019):

    ∂h/∂t = U − K·√Q·S + G · q_s / Q

with q_s the incoming sediment flux. At steady state under uniform U a channel's slope is
S = U(1+G)/(K√Q): channel slopes scale by (1+G) and the concavity θ is untouched, so the
slope–area band still holds.

After the implicit solve, one sweep in descending topological order (donors before receivers,
section 5.3) routes the detached volume:

- A dry cell in the **fluvial domain** (discharge ≥ `fluvial_min_area_km2`) keeps
  Φ · min(G/Q, 1) of the incoming flux Φ, capped so it stays at least `DONOR_MARGIN_M` (1e-6 m)
  below its lowest donor. The cap keeps deposition from inverting the routing graph within a
  step; dams form only by a lake filling, never by a single step's spike. Local detachment joins
  the outflow, so a headwater cell cannot re-bury itself.
- A dry hillslope cell (below the fluvial threshold) passes its flux through.
- A flooded cell traps flux up to its water level, whatever its discharge (section 4.2).
- A base cell exports all it receives.
- What leaves a cell is Φ − deposited + detached, split along its routing weights.

Deposition acts on the fluvial domain only because the G-form is channel physics. On a hillslope
cell Q ≈ 1, so G/Q saturates at 1: every hillslope cell becomes a perfect trap, sediment creeps
one cell per step, and the landscape smears into a dome, its valleys buried. Moving sediment on
hillslopes is diffusion's job. For the same reason K keeps its detachment-only calibration:
hillslopes, outside the deposition domain, set most of the relief and keep their slopes, while
channels carry the (1+G) steepening. Raising K by (1+G) everywhere instead halves hillslope slopes
and cuts relief to about a third. `fluvial_min_area_km2` must be positive: at zero every dry cell
would join the domain and the smearing would return.

The sweep is closed bookkeeping: detached = deposited + exported, to numerical precision
(section 6).

### 4.2 Standing water

The routing copy h_route is filled; the true surface h keeps its depressions.

- **Routing on the filled copy.** The routing graph and discharge are built on h_route, so water
  crosses a lake's surface to its spillway (the grid form of depression routing).
- **Shallow ponds merge.** After each fill, any connected ponded component whose deepest point is
  shallower than `MIN_LAKE_DEPTH_M` (0.5 m) is sub-grid noise: a pit that erosion and diffusion
  recut every step, and that within a step of centuries would silt up at once. h takes h_route
  there. Left standing, every centimetre pit stops incision and traps flux, and the landscape
  smears. Only deeper depressions stand as lakes. The merge adds mass, so its volume is metered
  (`pond_merge_m3`) and reported beside the fluvial budget.
- **Flooded cells do not incise.** A cell is flooded where h_route − h > 1 mm. The implicit solve
  would otherwise raise a lake-bed cell toward its receiver across the lake, which is deposition
  by the wrong mechanism. The bed under a lake is kept. Diffusion still acts under water.
- **The flooded mask is frozen** before erosion, for the whole step. Deciding "submerged" by
  comparing the evolving surface with h_route would class every freshly cut cell as flooded, and
  the sediment sweep would refill what detachment cut (deposition near 89 % of detachment, and a
  smeared landscape).
- **Rivers grade to water surfaces.** A receiver's effective height in the solve is its water
  level when flooded, max(h, sea level) at base level, and h otherwise. A stream entering a lake
  grades to the lake's level, and at the coast to sea level, never to a submerged bed.
- **Lakes trap sediment.** Flux entering a flooded cell deposits up to the water level. The first
  flooded cell along the path fills to the level and is land the next step: the delta builds out
  into the lake, and a lake in time becomes a plain.
- **The sea exports.** Flux reaching base level leaves the system. At steady state the land
  exports all its uplift; parking that at the river mouth, with nothing to spread it under water,
  would build a one-cell-wide pier of land without end. The model has no marine deposition.

Water ships as a spectrum, `water_depth_m` = h_route − h. A lake cell routes to its spillway and
is not its own receiver, so the receivers alone do not mark all water: only the sea receives
itself.

### 4.3 Two surfaces

`sediment_m ≥ 0` is kept per cell; bedrock = h − sediment. Uplift moves both together. Fluvial
detachment removes sediment first, then bedrock. Deposition adds sediment. Diffusion that lowers a
cell takes sediment first, and diffusion that raises one adds sediment (hillslope flux is
colluvium). This is bookkeeping of mass and material: sediment and bedrock share one
erodibility K and behave alike.

## 5. Multiple-flow-direction routing

Single-receiver (D8) routing locks channels to 0°, 45° and 90°, fans parallel lines down smooth
flanks and builds single-thread comb deltas. Weighted multiple flow directions remove or soften
these. It is a change of discretisation, not of physics. It does not make rivers meander or braid;
that takes other physics (bank erosion, secondary flow, sorting of bedload), which the model does
not have.

### 5.1 Freeman weights

Each non-base cell sends flow to every strictly lower 8-neighbour on h_route, with weight
∝ S^p (Freeman 1991), normalised to sum 1. p = `mfd_exponent`, default 1.1, Freeman's value; it
is a discretisation parameter, not forcing. Base cells have no out-edges. Priority-flood + ε gives
every non-base cell at least one strictly lower neighbour, so the graph reaches base level from
everywhere. That is why `epsilon_fill_m` must be at least 1e-9: below it, h + ε rounds back to h
on high ground and a cell is left with nowhere to drain. A non-base cell with no out-edge stops
the run, since it would silently swallow water and sediment while the mass balance still closed.
Edges, weights, distances, donors, the order and the channel tree live in one structure,
`flow::MfdGraph`.

### 5.2 Channels converge

Spreading everywhere braids valley floors into flat sheets of many threads: flat channel cells
multiply, the bifurcation ratio climbs out of its band, and lakes never deepen past the merge
depth. Water that has gathered into a channel converges, the principle behind Holmgren (1994)'s
area-dependent exponent, taken here to its limit at the fluvial threshold. So routing is built
twice each step:

1. A pure-spreading pass finds where water gathers (its discharge).
2. The routing pass converges each dry cell whose first-pass discharge is at least the fluvial
   threshold. Hillslopes and flooded cells keep spreading.

A converging cell takes the surface's own downhill direction (central differences) and splits it
between the two lower neighbours 45° apart whose cone holds it, in proportion to its components
along each (Tarboton 1997's D-infinity, in vector form). A channel on a smooth slope then runs the
slope's way rather than along a row, column or diagonal. Where the surface is flat or no such pair
holds the direction, the cell sends everything down its single steepest edge, ties to the lower
cell index.

### 5.3 Order by routed height

Every edge goes to a strictly lower routed height, so sorting cells by (routed height, index)
gives a topological order, receivers before donors, with ties broken the same way on every run.
Ascending, it drives the implicit solve (receivers are already updated); descending, it drives
discharge accumulation and the sediment sweep (donors are complete before their receivers).

### 5.4 The implicit solve on the graph

The Braun & Willett (2013) implicit scheme needs every receiver solved before its donors. That
holds on any acyclic graph walked in the order of 5.3, so each cell is solved in closed form
against the weighted blend of its already-updated receivers:

    h⁺ = (h + Σᵢ cᵢ·hrᵢ⁺) / (1 + Σᵢ cᵢ),   cᵢ = K·√Q·dt·wᵢ / distᵢ

summed over the out-edges whose effective receiver height (section 4.2) lies below the cell.
With one edge this is exactly the single-receiver update. Base and flooded cells are skipped. The
sediment sweep splits each cell's outflux by the same weights, and the donor cap of 4.1 is taken
over all donors in the graph.

### 5.5 The channel tree

Statistics and extraction need a tree. Each cell's channel-tree receiver is its largest-weight
edge (the steepest, since weight grows with slope), ties to the lower index; a base cell receives
itself. Strahler orders, basins, flow distance, mainstream length, the Hack and Horton gates and
the `receivers` output all use the tree, so the sea is still exactly the cells that receive
themselves. `discharge` is the routed flux of the whole graph; `area_cells` is the tree's whole-cell
count, so Hack's law stays planimetric.

River membership is thresholded on discharge accumulated along the tree, not on the graph's
flux. Graph flux is not monotone along the tree: a spreading cell passes only its largest share
down it. Thresholding it breaks streams mid-channel and starts them again downstream as false
first-order heads, which skews the Horton count. Tree flux grows downstream by construction, so
a river cell's receiver is a river cell.

The river threshold (`river_min_area_km2`) is set for this flux field: the smoother routed
discharge crosses a given threshold at more marginal heads than D8 did, so the 200 m default is
1.5 km², which selects a network like the D8 one at 1.0 km².

## 6. The gates

The gates measure process (section 3) against empirical norms for fluvial landscapes
(`docs/references.md`). Advisory gates are reported only; core gates decide `overall_pass`, and a
core gate that cannot be measured (too little data) fails. The steady-state gates assume uplift.
Run as a step there is none, so the step reads only `axis_runs` (`wire.rs` reports it); the rest
are recorded for a run under uplift (`tests/gates_snapshot.rs`).

| Gate | Measures | Band | Kind |
|---|---|---|---|
| `slope_area_theta` | θ from S ∝ Q^−θ over fluvial cells | 0.40 – 0.60 | core |
| `spl_residual_median` | steady-state balance (below) | ≤ 0.35 | core |
| `hack_exponent` | h from L ∝ A^h over all river subbasins | 0.45 – 0.70 | core |
| `horton_bifurcation` | R_b from Strahler counts in the largest basin | 3 – 5 | core |
| `pit_cells` | land cells that receive themselves | 0 | core |
| `drainage_completeness` | share of land draining to open water | 1 | core |
| `sediment_mass_closure` | \|Σdetached − Σdeposited − Σexported\| / Σdetached | ≤ 1e-9 | core |
| `lake_level_spread_m` | widest water-surface spread in one water body | ≤ 0.05 m | core |
| `hypsometric_integral` | mean normalised land height | 0.35 – 0.60 | advisory |
| `rain_shadow_ratio` | land rain, upwind half over downwind half | ≥ 1.05 | advisory |
| `floodplain_slope_ratio` | median floodplain slope over median channel slope | ≤ 1 | advisory |
| `axis_runs` | share of channel cells in runs of more than 8 steps one grid way while the land falls over 5° another | ≤ 0.25 | advisory |

- **Slope–area.** Regressed on discharge (section 2.3), toward the tree receiver graded to the
  surface it meets (section 4.2), over fluvial cells whose uplift lies within 0.75 to 1.25 of the
  median, since θ ≈ m/n assumes uniform uplift. Flats with S < 1e-5 (ε-fill) are left out, as on
  real filled elevation models. Flooded cells are left out: a lake bed's slope says nothing about
  stream power. Channels covered by sediment stay in: with one K, every steady channel obeys
  S = U(1+G)/(K√Q) whatever its cover, and channel cells carry a layer in transit of order G·U·dt
  every step, so leaving them out would measure the model wrong.
- **Residual.** The median over fluvial cells of

      | (K·√Q·Σᵢ wᵢ·Sᵢ − κ∇²h − D) / U − 1 |

  with the fluvial term as erosion applied it, over all out-edges, and D from the replayed step
  (section 1.2). On fine cells the diffusion flux into valleys rivals uplift, so it must be in the
  balance or it reads as disequilibrium. This gate is also the trip-wire for the time-step limits
  of section 1.3.
- **Hack.** Every river cell defines a subbasin, which spans three and more decades of area
  (Rigon et al. 1996); fitting outlets only, on an island, is dominated by short coastal basins
  and reads low. Membership uses tree discharge (section 5.5).
- **Horton.** Horton's laws hold per basin, so the counts are taken in the largest basin only;
  pooling the whole land lets small coastal basins inflate the first order.
- **Mass closure.** Pure consistency, but it is the trip-wire for every bookkeeping error in the
  sediment sweep.
- **Lake level.** Every lake stands level at its spill by construction, so a spread means the
  routing is wrong.
- **Floodplain slope.** Floodplain cells are dry fluvial cells whose sediment is well above the
  layer in transit: thicker than max(2·G·U_max·dt, 0.5 m). The fluvial restriction matters:
  colluvium piled by diffusion on hillslopes is as thick and would decide the median.
- **Pits.** The pit gate applies to the routing surface's tree.
