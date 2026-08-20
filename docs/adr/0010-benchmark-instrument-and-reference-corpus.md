# ADR 0010 — Benchmark instrument adoption, and the reference corpus

Status: accepted (2026-08-20)
Scope: adopts the benchmark specification
(docs/research/humanity/0012) as the project's validation instrument,
rules on the five towns that were measured before pre-registration
existed, bounds how the pilot may change the protocol, and fixes the
reference data store that ADR 0008 D11 requires but never defined.
Prompted by the discovery that the spike's entire reference corpus is
unreproducible.

## Context

docs/research/humanity declares that nothing in the research corpus is
normative until an ADR adopts it. ADR 0008 fixed the validation
doctrine and research 0012 specified the instrument that satisfies it,
but no document adopted the instrument — so "build the benchmark
against the spec" had, formally, no adopted spec to build against.

The second problem is worse, and it is recorded here so it cannot
happen again. Every reference number in
spikes/townfabric/VALIDATION.md was measured on OSM extracts that
lived only in the gitignored `runs/` tree. The extracts are gone. No
fetch date was ever recorded. Four of the five study centres survived
only in a pull-request description, outside version control —
Alnwick's alone was committed, embedded in VALIDATION.md's example
fetch command — and one variant (the Abilene residential disc) was
never recorded anywhere. OpenStreetMap has been edited since.
Consequence: not one reference number in VALIDATION.md can be exactly
re-measured (the synthetic rows are seed-reproducible, but with
nothing valid left to compare them against). The provenance rule
(ADR 0008 D11) was never satisfied even once, and the corpus the
whole validation argument rests on is unrecoverable. The rule
existed; the store it presupposes did not.

The third problem is the five measured towns themselves.
Pre-registration (0012 §5.4) exists so that reference cases cannot be
selected after seeing which ones flatter the model. For Alnwick,
Lavenham, Monpazier, Abilene and Levittown that protection is already
spent: their measured values are published in VALIDATION.md and known
to the model's authors. ADR 0008 D6 defines calibration narrowly —
"parameters fitted against reference cases" — which covers only
Alnwick unambiguously. It is silent on mechanism-selection and
diagnostic use, and all five towns were used one of those ways.

## Decision 1 — Research 0012 is adopted as the validation instrument

The benchmark specification is adopted as written, with three named
deferrals. Adoption makes the specification binding on the instrument
build: deviating from it now requires amending it with a note, as the
FAQ ledger does, or a further ADR.

The deferrals, each of which 0012 itself leaves open:

- **The §9.4 scale mismatch** (Boeing's indicators are whole-urban-area
  quantities; our study disc is not that object) is resolved in the
  frozen protocol document (Decision 3), in the same document that
  fixes the radius — they are one question — and before Tier B uses
  the Boeing population, 0012's own precondition.
- **Acceptance thresholds**: none are declared. Under ADR 0008 D8 any
  threshold is our own convention, and there is nothing yet to judge —
  the first benchmark report describes reference populations and
  ships threshold-free, reporting positions and distances only.
- **The disc radius and the adopted character list** are fixed at the
  protocol freeze (Decision 3), before the pre-registered sample is
  measured — not in this ADR. This is 0012 P1 read correctly: the
  specification treats the radius as a protocol parameter; a freeze
  binds one value under one protocol identifier, and that identifier
  is what P1's quoting requirement refers to.

## Decision 2 — The five measured towns are calibration, permanently

Alnwick, Lavenham, Monpazier, Abilene and Levittown are assigned to
the fitted/calibration partition of their classes, permanently. None
of them may ever appear in a held-out split.

The ruling is deliberately broader than D6's narrow definition:

- **Alnwick** is calibration by the narrow definition itself — two
  parameter sweeps (36 and 54 combinations) were fitted on it, and the
  street-class shares were calibrated from it.
- **Lavenham** was never fitted on, but it was the evaluation case in
  two successive rounds of mechanism selection — the first led to the
  step-3a mechanism change, the second condemned the growth rule and
  named its replacement before the generator was retired. Under the
  pattern-oriented modelling frame D6 cites, that is model selection —
  a form of fitting.
- **Abilene and Levittown** were used diagnostically to motivate
  mechanism changes; **Monpazier** was measured and its values
  published. And for all five, the authors know the published values,
  which is precisely the knowledge pre-registration exists to
  exclude.

Where the narrow and broad readings disagree, the broad one costs
nothing: the sample design requires at least twenty towns per class
regardless, so conservatism here forfeits no capacity.

Their VALIDATION.md numbers are historical record, not reference
values — the extracts they were measured on no longer exist, so the
numbers carry no provenance under D11 and cannot be reproduced.

Their new role: **instrument-development pilot.** The five towns are
re-fetched with full provenance and used to shake down the ported
measurement kernel and its cross-validation against `osmnx`/`momepy`
(ADR 0009 Decision 4) before the fresh sample is pre-registered.
This is why the pilot precedes pre-registration: measuring
already-burned towns burns nothing, and a protocol defect found in
the pilot can still change the protocol without staling a
pre-registration that names it.

This is a recorded exception to the letter of 0012 §5.4 — the
pre-registration list will name pilot towns that were measured before
the list was written — and it is confined to the pilot set. The
rule's purpose is to exclude selection after seeing values; for these
towns the values were already seen, this ADR pins them to the fitted
partition where that knowledge cannot leak into evaluation, and every
town added to the list after them is covered by the rule's letter.

The pilot's numbers are new measurements on new extracts. They
supersede the historical record; they are never presented or tested
as reproductions of it — with the old extracts gone, a difference
between a new number and an old one cannot distinguish a port bug
from OSM drift. Port correctness is judged solely by the
cross-validation against `osmnx`/`momepy` on the identical new
extracts (ADR 0009 Decision 4).

One more town joins the pilot. The five burned towns cover four of
0012 §5.4's five classes; none is contemporary informal — the class
where OSM coverage, the top-ranked threat to validity, is worst, and
the one most likely to break an import kernel. So the pilot set is
six: one informal-class town, designated in a committed addendum to
this ADR *before* it is fetched, assigned to the fitted partition by
the same designation-before-measurement logic. A protocol defect
that only informal fabric can surface must be found before the
freeze, not after pre-registration.

The study centres, recorded in version control at last:

| Town | Class (0012 §5.4) | Centre |
| --- | --- | --- |
| Alnwick | organic pre-modern core | 55.4147, -1.7061 |
| Lavenham | organic pre-modern core | 52.1080, 0.7960 |
| Monpazier | planted pre-modern grid | 44.6797, 0.8967 |
| Abilene, KS | 19th-century survey plat | 38.9172, -97.2137 |
| Levittown, NY | mid-20th-century suburb | 40.7259, -73.5143 |

The Abilene residential-disc variant's centre was never recorded and
is lost; if a residential Abilene disc is wanted again, it is a new
measurement at a newly chosen, recorded centre.

## Decision 3 — The protocol freeze

The pilot exists to change the protocol; this decision bounds how.

- **Protocol changes during the pilot are justified on instrument
  grounds only** — cross-validation disagreement, a measurement
  defect, a data-coverage limit — never by reference to the generated
  model's performance. The authors freezing the protocol know the
  model's published per-character failures; a protocol chosen to
  flatter them would contaminate every fresh town it governs. This is
  the protocol-level analogue of ADR 0008 D6, and it binds the choice
  of radius, element sets and characters alike.
- **The character list defaults to the full published sets** 0012
  adopts by name (Boeing's indicators, momepy's taxonomy); any
  exclusion is named in the freeze document and justified on
  instrument grounds.
- **The frozen protocol is a file**: `reference/protocol-<id>.md`,
  one immutable document per identifier. Any amendment is a new
  identifier. Manifest records, character tables and every reported
  number cite the identifier — this is the "protocol identifier"
  ADR 0008 D11 requires, defined at last. 0012 §3 specifies the
  protocol's shape and its measured sensitivities; the binding
  instance is the identified freeze document.
- **Cross-validation is a check, not a mirror.** Pilot disagreement
  between `via-bench` and `osmnx`/`momepy` beyond a small
  per-character envelope — each envelope justified on numerical
  grounds — is a defect to fix before the freeze, not a tolerance to
  declare. The tolerances that survive are recorded per character in
  the freeze document, and later drift beyond them fails the sidecar
  check.

## Decision 4 — The reference data store

Data stays out of version control; provenance goes in. The split is
what this ADR's context shows the project cannot do without.

**In the repository, a top-level `reference/` directory** holding:

- the frozen protocol documents (Decision 3);
- the pre-registration document, when it is written (0012 §5.4):
  class definitions, the town list with study centres, and the
  per-class held-out designation — committed before any town on the
  list, other than the pilot towns Decision 2 pins to the fitted
  partition, is fetched or measured;
- `reference/extracts.jsonl` — an append-only manifest in JSON Lines,
  one record per line, so independent appends from different machines
  merge cleanly (the same reasoning as `.vgeo` in ADR 0009
  Decision 3). One record per fetched extract: town, class, centre,
  radius, the query that fetched it, the endpoint and its reported
  generator version, the response's `osm_base` timestamp, fetch
  date, blake3 of the extract file, and licence. The record's
  partition is not authored here: it derives from the
  pre-registration document (for the pilot towns, from Decision 2),
  and a mismatch is a defect. Committed records are never edited —
  append-only is checked, not asserted: CI verifies previously
  committed lines are byte-unchanged.

**Outside the repository**, the extract files themselves, under
`runs/reference/` on each machine. Machines share extracts out of
band — never through the repository — and the manifest's blake3 is
what lets a receiving machine verify it holds the same bytes the
numbers were measured on.

**Loss is designed against, not just detected.** Before a number
measured on an extract is published, the extract exists in at least
two locations — a second machine, or a private off-repository
archive — verified by hash. Fetch queries pin the data timestamp
with an attic `[date:...]` clause where the endpoint supports it, so
a lost extract is content-reproducible from its manifest record. An
extract that survives nowhere is **lost**: its manifest record
remains, and any report quoting numbers measured on it must flag
them as unreproducible — the VALIDATION.md failure mode, named so it
can be seen.

**Re-fetching is a new extract**, not a refresh: OpenStreetMap
drifts, so a re-fetch appends a new manifest record with its own date
and hash, and every reported number cites the extract it was measured
on by hash. Two numbers measured on different extracts of the same
town are different measurements.

## Decision 5 — Licence rulings

Recorded as project conventions, deliberately cautious; none of this
is legal advice.

- **Extracts** are ODbL. They are never redistributed from this
  repository (ADR 0008 D11, unchanged). The manifest records the
  licence per extract.
- **The line is drawn at systematic scale.** Complete character
  tables spanning the corpus (all pre-registered towns) are treated
  as ODbL-derived and live in the reference store beside the
  extracts, not in the repository. Complete tables for a report's
  *named case-study towns* — the pilot six, a held-out town under
  discussion — may be published in-repo with attribution, which is
  what VALIDATION.md already does for five towns; it is grandfathered
  under this line as historical record.
- **Validation reports** publish per-class aggregates freely and
  carry attribution wherever OSM-derived numbers appear: "Contains
  information from OpenStreetMap, © OpenStreetMap contributors,
  ODbL".
- **GHS-UCDB and the Boeing dataset**: the release used is pinned in
  the pre-registration document, and each dataset's terms are
  recorded in the store manifest when first fetched.

## Consequences

- The research corpus README's "no ADR references these documents as
  normative" is revised: 0012 is adopted by this ADR; everything else
  in the corpus stays pre-decision.
- CONTRIBUTING's Python rule is amended in place: the standing,
  version-pinned analysis sidecar under `analysis/` joins throwaway
  scripts as a permitted form, under ADR 0009 Decision 4's quarantine.
- Build order is fixed by Decisions 1–4: port and test the
  measurement kernel, pilot it on the six pilot towns with full
  provenance, freeze and identify the protocol — resolving §9.4 and
  recording the surviving cross-validation tolerances — then
  pre-register the fresh sample, then fetch and measure it. Stages
  come after the instrument works (ADR 0009).

## Rejected

- **Readmitting any burned town to a held-out split by exception.**
  The exception would exist because the towns are convenient, which is
  the selection-after-seeing-values failure pre-registration exists to
  prevent.
- **Treating the old VALIDATION.md numbers as reference values.** No
  extract, no provenance, no number — D11 admits no grandfathering.
- **Committing extracts to pin reference data.** It would solve
  reproducibility by redistribution, which ODbL-cautious practice and
  D11 both rule out.
- **Publishing corpus-scale per-town character tables in the
  repository.** Under a cautious ODbL reading a systematic
  hundred-town character database is a derivative database;
  aggregates with attribution, plus complete tables for named
  case-study towns only (Decision 5), carry the report's argument
  without the exposure.

## Addendum (2026-08-20) — the sixth pilot town

Per Decision 2, the informal-class pilot case, designated here before
any fetch or measurement of it:

| Town | Class (0012 §5.4) | Centre |
| --- | --- | --- |
| Kibera (Nairobi, Kenya) | contemporary informal | -1.3113, 36.7890 |

The centre is OSM's Kibera place node (Lindi ward). Origin
documentation: an informal settlement by documented origin — begun on
an early-20th-century colonial land allocation to Nubian soldiers and
densified without formal planning thereafter; among the most
extensively documented informal settlements in the urban literature.
Chosen on instrument grounds: Kibera is the best-mapped informal
settlement in OSM (the Map Kibera project, 2009 onward), so the pilot
exercises informal-fabric geometry — the import kernel's hardest case
— without confounding it with missing data. The poor-coverage failure
mode is a separate concern, exercised by the coverage-verification
procedure the freeze must define (0012 §9.1). Fitted partition, like
the other five.

## References

- ADR 0008 (validation doctrine: D6 calibration/validation, D8
  thresholds, D11 provenance); ADR 0009 (stage architecture: Decision
  4, where the benchmark lives).
- docs/research/humanity/0012 (the instrument: §5.4 sampling and
  pre-registration, §9.4 scale mismatch).
- spikes/townfabric/VALIDATION.md (the measurements this ADR rules
  on); PR #1, the spike handoff, where four of the five study centres
  survived.
