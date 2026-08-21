# docs/research/humanity — the human side, research corpus

Status: **research corpus — pre-decision.** Adopting any mechanism,
gate, or parameter recorded here requires its own ADR with its own
review. One document has been adopted this way:
[0012](0012-benchmark-specification.md), the benchmark specification,
adopted as the project's validation instrument by
[ADR 0010](../../adr/0010-benchmark-instrument-and-reference-corpus.md).
Everything else remains pre-decision. This directory exists so that
the eventual decisions are made against surveyed literature rather
than intuition.

## Scope

The README's causal chain reserves six human layers downstream of the
nature side: **suitability → corridors → settlements → flows →
network → morphology**. This corpus surveys the literatures that bear
on them, with one standing requirement: a single mechanism set must
span eras — a medieval town, a 19th-century frontier town, and a
modern metropolis should be the *same model* under different declared
forcing, or the framework fails via's doctrine before it starts.

Two terms of art used throughout, because the vernacular "city
generation" conflates them:

- **Settlement system** — the regional scale: where settlements
  appear, how they space, size, and rank themselves. Spatial
  economics and quantitative geography own this scale.
- **Urban morphogenesis** — the within-settlement scale: how street
  fabric, plots, and buildings grow. Urban morphology (the Conzenian
  school's term) owns this scale. The street graph is the *street
  network*; "city networks" in the literature means networks *of*
  cities and is avoided here.

## The corpus

Field dossiers — one per literature, faithful to the survey, no
tiering or adoption judgments:

| Doc | Literature |
| --- | --- |
| [0001](0001-settlement-systems.md) | Settlement systems: central place theory, spatial interaction, rank-size, urban scaling |
| [0002](0002-urban-morphology.md) | Urban morphology: Conzenian plan analysis, typological process, space syntax |
| [0003](0003-street-networks.md) | Street/road network growth models and their empirical statistics |
| [0004](0004-land-use-economics.md) | Land use & land value: bid-rent, monocentric model, polycentricity, land-use CA |
| [0005](0005-transport-eras.md) | Transport technology & city form across eras: speeds, costs, caps |
| [0006](0006-founding-and-planning.md) | Founding & planning regimes: planted towns, survey lattices, plats, zoning |
| [0007](0007-simulation-precedents.md) | Validated simulation precedents: archaeology ABMs, SIMPOP, validation doctrine |

Commissioned dossiers — targeted sweeps answering a question the
corpus was found not to answer, compiled for a specific adopting
decision (they span literatures and note doctrine fit in place):

| Doc | Content |
| --- | --- |
| [0013](0013-affordance-detection.md) | Affordance detection & movement cost: citable schemes over via's terrain artifacts (commissioned for ADR 0011) |
| [0014](0014-ml-generation-path.md) | The ML path: learned generation of settlement fabric — I/O and modality definition, training data and ODbL, architectures, prior work, schedule and budget (commissioned for the ML spike decision) |

Cross-cutting screens — synthesis over the dossiers, still
pre-decision:

| Doc | Content |
| --- | --- |
| [0008](0008-doctrine-screen.md) | Candidate tier assignments (ADR 0003) and the honest-refusal list |
| [0009](0009-gate-candidates.md) | Falsifiable gate candidates with empirical ranges and sources |
| [0010](0010-era-parameters.md) | The era-parameter table: numbers that would make era a config |
| [0011](0011-open-questions.md) | Load-bearing questions the survey did not answer |
| [0012](0012-benchmark-specification.md) | The benchmark instrument: characters, reference populations, protocol, statistics |

## Validation

Two spikes (`spikes/settlements`, `spikes/townfabric`, commits 5c770b5
and 07a95b8) tested parts of this corpus against real towns and failed
in ways that changed the project's method, not just its parameters.
The outcome is recorded in **[ADR 0008](../../adr/0008-validation-doctrine.md)**
— gates versus benchmarks, reference populations rather
than quoted bands, ensembles rather than single runs, no invented
composite scores — and the instrument that follows from it is
specified in **[0012](0012-benchmark-specification.md)**. Where this
corpus's [0009](0009-gate-candidates.md) proposed scoring against
literature bands, ADR 0008 supersedes it: real towns measured under a
fixed protocol fall outside several of those bands, so the bands are
context, not targets.

## The corpus's leading hypothesis (not a decision)

Three independent lines of evidence suggest **era is forcing, not
model**: the invariant daily travel-time budget (Marchetti 1994 —
extent = era speed × fixed budget), the palimpsest evidence (Conzen's
streets > plots > buildings persistence; ~90% backbone-route
persistence over ~170 yr in Strano et al. 2012), and the era-invariant
settled-area scaling A ~ N^(2/3..5/6) (Ortman et al. 2014, spanning
prehispanic to modern). If adopted, era parameters would act on *new
accretion only*, with existing fabric carried as slow state — a
per-era generator, or an era switch that restyles standing fabric,
is rejected by the palimpsest evidence itself (0008). This is the
hypothesis future ADRs must test, not a settled conclusion.

## Method

Compiled August 2026 by a structured multi-track literature survey
(seven parallel tracks, one per field dossier) followed by an
adversarial doctrine screen against ADR 0003. Commissioned dossiers
are added by separately run sweeps under the same verification rules
(0013: five parallel tracks, 2026-08, for ADR 0011). Citations were verified against
sources at survey time; items the survey could not verify are flagged
as such in place. Errors that surface later should be corrected in
place with a note, as the FAQ ledger does.
