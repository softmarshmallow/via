# ADR 0008 — Validation doctrine: gates, benchmarks, and reference populations

Status: accepted (2026-08-20)
Scope: revises project doctrine (README research commitments, ADR 0003
epistemic tiers) in light of a documented failure, and applies to
every stage, including the ones already built.

## Context: how the existing doctrine failed

ADR 0003 fixed three epistemic tiers and made *process* the only tier
that carries gates. It did not say what a gate must be measured
against. In practice the project has used **bands quoted from
literature** — slope–area θ ∈ [0.40, 0.60], Hack h ∈ [0.45, 0.70],
Horton R_b ∈ [3, 5] on the nature side; street meshedness
"organic 0.15–0.26, planned grids 0.26–0.35" on the human side.

A spike on the human side (`spikes/townfabric`, commit 07a95b8) tested
that practice against real towns and it broke in five distinct ways.
Each is recorded here because each generalises:

1. **Literature bands are protocol-dependent, and real cases fail
   them.** Alnwick — the town of Conzen's (1960) study — measures
   meshedness 0.057 counting mapped streets and 0.138 counting
   footpaths and alleys, against a quoted "organic" band of 0.15–0.26.
   A gate on that band would have failed the reference itself. The band
   is not wrong; it is stated for a protocol that was never recorded
   with it.
2. **Statistics computed on the wrong graph.** Every published street
   statistic refers to the *simplified* graph, where degree-2 geometry
   vertices are dissolved and edges are whole streets. Measured on an
   unsimplified graph, real Alnwick returns 0.018 — an impossible
   value — and so did all of this project's earlier numbers.
3. **Boundary effects were unhandled.** Clipping an extract at the
   study radius turns every street leaving it into a false dead end
   (Alnwick read 41.7% dead ends, mostly artifacts).
4. **A single seed is not a measurement.** One run reported meshedness
   0.072 against a reference 0.057 — an apparent match. Across five
   seeds the same configuration spans 0.028–0.113.
5. **The scoring function was invented.** Parameters were fitted by
   minimising a weighted average of relative errors whose weights,
   transform and composition had no source. A single composite score
   also hides *which* pattern failed, which is precisely how a fabric
   that was visibly wrong could be reported as matching "6 of 7"
   statistics.

The through-line: **a gate is only as good as the standard behind it,
and the project had no standard — only quotations.**

## Decision

### D1. Two distinct instruments, named separately

- A **gate** is an internal consistency check: it asks whether a stage
  obeys the equations it claims to solve. Mass closure, the SPL
  residual, pit count, completeness, bitwise determinism. Gates are
  cheap, binary, and belong in CI. They say nothing about realism.
- A **benchmark** is an external comparison: it asks whether the
  stage's output is distinguishable from measurements of the real
  world. Benchmarks are distributional, expensive, and belong in a
  validation report, not in CI.

The project has been conflating the two. θ, Hack and R_b are
benchmarks wearing a gate's clothing: they compare our landscape
against empirical regularities, but they are scored against quoted
bands rather than against a measured population.

### D2. Characters must be defined by a citable source or an open
implementation

A **character** is one measurable property of an output. A character
is admissible only if its definition comes from a published source or
from an open implementation whose definitions are published (for the
human side: OSMnx/`osmnx` indicators, Fleischmann et al.'s momepy
character set). Inventing a statistic is permitted only when no
published character covers the property, and then it must be written
down as an invention, with its definition, its motivation, and the
failure mode it was created to detect.

Retroactive consequence: three characters used in the spike —
"street-wall share" at a 6 m threshold, a Lorenz-style "backbone
concentration", and block minor-axis — were inventions. Two have
published equivalents that must replace them (`bc_gini` in Boeing's
indicator set; morphometric characters in momepy).

### D3. The reference is a population, never a band and never a case

Benchmarks compare against a *distribution* measured over many real
cases under our own protocol, or over a published dataset whose
protocol is documented. A single reference town is an anecdote: it
carries its own idiosyncrasy and gives no dispersion against which to
judge a difference.

### D4. One frozen protocol, restated with every number

Every benchmark number is meaningless without: the spatial extent
measured, the buffer beyond it, the element set included (which street
classes, which building types), the graph simplification applied, and
the software version. A number reported without its protocol is not a
result.

### D5. Ensembles, not runs

Generative stages are seeded. A benchmark statistic is reported as a
distribution over **at least five seeds** (mean and standard
deviation, or the full set), never as a single value. Where the
project's determinism contract makes a single run reproducible, that
is a statement about reproducibility, not about the model.

### D6. Calibration and validation are separated, and both are reported

Parameters fitted against reference cases are *calibration*. The same
cases cannot then evidence the model. At least one reference case per
class is held out of fitting and used only for evaluation, and every
report states which cases were fitted on and which were held out
(Grimm et al. 2005's pattern-oriented modelling requires this
separation, and requires several patterns to be satisfied at once).

### D7. No composite score

Characters are compared and reported **individually**. Comparison uses
a standard statistic — a two-sample Kolmogorov–Smirnov distance for
distributional characters, the percentile position within the
reference population for scalar characters — never a bespoke weighted
sum. Aggregation across characters, if ever wanted for ranking during
a parameter sweep, must be reported alongside the full per-character
table and is never the basis of an acceptance claim.

### D8. Acceptance thresholds are declared conventions

No published source states how close a generated town must be to a
real one. Any threshold we adopt is ours. It must be written as a
convention, with its rationale, and must never be attributed to the
literature.

### D9. Equifinality guard

Matching a statistic is weak evidence: correlated percolation and
diffusion-limited-aggregation models reproduce urban statistics with
no human mechanism in them at all, and Janssen's (2009) replication of
Artificial Anasazi showed a celebrated fit carried by two calibration
parameters rather than by the modelled behaviour. Therefore a
benchmark suite must include at least one character that a null model
demonstrably fails, and the null models must be run and reported
alongside the model.

### D10. Visual inspection is a first-class instrument

Twice in the spike, direct visual comparison detected a failure that
the entire numeric battery missed: blocks tiled edge-to-edge with
buildings where real blocks are mostly unbuilt, and long triangular
blocks where real blocks are compact. Both were later confirmed
numerically — but only after a character was added to detect them.

The doctrine is therefore explicit: **the eye is the discovery channel
for unmodelled failure modes; the battery is the instrument that
prevents regression.** When visual inspection finds a defect the
battery does not measure, a character is added *before* the mechanism
is changed. A defect found by eye and not measurable is recorded as an
open character gap.

### D11. Provenance

Every reported number carries: protocol identifier, code revision,
reference-data snapshot date, seed set. Reference extracts from
third-party sources are stored outside version control with their
licence recorded, and are never redistributed from this repository.
The store this rule presupposes — its layout, its manifest, and the
rulings on data measured before the rule existed — is fixed in
ADR 0010.

## Consequences

- The human-side benchmark is specified separately, as an instrument,
  in docs/research/humanity/0012.
- The nature side keeps its gates (mass closure, residual, pits,
  determinism) unchanged: those are gates under D1 and are sound.
  Its **band-scored characters** (θ, Hack, R_b) are re-labelled as
  benchmarks scored against quoted bands, which is the weakest form
  under this doctrine. Upgrading them to reference-population
  benchmarks — against measured DEM populations rather than quoted
  ranges — is recorded as an open item, not scheduled here. No
  terrain result is withdrawn: the bands are broad, well established,
  and were measured under a stated protocol, but their epistemic
  status is now correctly named. Until that upgrade, θ, Hack and R_b
  stay in `gates.json` and CI where they run today — a recorded
  exception to D1's benchmarks-belong-in-a-report rule, kept because
  dropping them would trade a mislabelled check for no check.
- Any future claim of the form "the model reproduces X" requires: a
  population, a protocol, an ensemble, a held-out case, and a
  per-character table.

## Rejected

- **A single realism score.** No such standard exists. The published
  work addressing this exact problem — Shaw et al. (2023) on
  determining the realism of procedurally generated city road networks
  — proposes entropy and orientation-order measured against real
  cities as *"a reasonable baseline for future projects"*, and the
  surveying literature states plainly that quantifying the realism of
  a simulated city with standardised metrics remains an open
  challenge. Manufacturing a score and presenting it as a standard
  would be the exact failure this ADR exists to prevent.
- **Visual-only acceptance.** It cannot detect regression, cannot be
  automated, and cannot be argued with.
- **Numeric-only acceptance.** D10 exists because it demonstrably
  failed twice in this project.

## References

- Conzen, M. R. G. (1960). *Alnwick, Northumberland: A Study in
  Town-Plan Analysis*. Institute of British Geographers Publication 27.
- Grimm, V., Revilla, E., Berger, U., Jeltsch, F., Mooij, W. M.,
  Railsback, S. F., Thulke, H.-H., Weiner, J., Wiegand, T., DeAngelis,
  D. L. (2005). Pattern-oriented modeling of agent-based complex
  systems: lessons from ecology. *Science* 310(5750), 987–991.
- Janssen, M. A. (2009). Understanding Artificial Anasazi. *Journal of
  Artificial Societies and Social Simulation* 12(4), 13.
- Boeing, G. (2021). Street Network Models and Indicators for Every
  Urban Area in the World. *Geographical Analysis* 54(3), 519–535.
  doi:10.1111/gean.12281.
- Fleischmann, M., Feliciotti, A., Romice, O., Porta, S. (2022).
  Methodological foundation of a numerical taxonomy of urban form.
  *Environment and Planning B* 49(4), 1283–1299.
  doi:10.1177/23998083211059835.
- Shaw, A., Wünsche, B. C., Yde, J., Vergerakis, P., Chaney, L., Jin,
  Y., Sarwate, N. (2023). Determining Realism of Procedurally
  Generated City Road Networks. In *Image and Vision Computing*
  (IVCNZ 2022), Lecture Notes in Computer Science 13836, 272–287.
  Springer. doi:10.1007/978-3-031-25825-1_20.
- ADR 0003 (epistemic tiers); docs/research/humanity/0009 (candidate
  gates, superseded as a scoring proposal by this ADR); 0011 (open
  questions, of which single-seed-vs-ensemble and delineation are
  resolved here as D5 and D4).
