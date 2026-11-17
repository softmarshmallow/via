# Charter

What `via` is for, what it owns, what it leaves to others, and what it does not promise. One page,
derived from the goal and principles (`docs/doctrine.md` sections 1 and 2), the set-aside list
(section 7), the ledger's verdicts (`docs/ledger.md`, the "Whose" column and section 26) and the
people it is for. A ruling that contradicts this page amends it and says so. Written 2026-11-17
(M7); the two lines marked "ours" are choices with no ruling behind them, open to the owner.

## The intended use

An agent writes a world as text: terrain, water, ways, buildings and dressing, said by intent ("on
the lane, a third of the way along"), never as a painted result. The compiler builds a folder an
engine loads: fields as rasters, every placed instance, generated meshes, chunks, and a list of
what does not fit, each with where and what to change. A person reviews named views at eye height
and the change report. To change the world, the agent edits a decision; what depended on it
settles again. Nobody repairs a map by hand.

Sized for a few kilometres (`atlas`: thirteen worlds joined on 3.2 by 2.56 km), on Godot first.

## Who it is for

- **New games:** a world that starts here, rather than one brought from elsewhere. The developer may
  be experienced; it is the game that is new.
- **Independent developers first:** they take up a new tool more readily and have fewer people
  managing assets.
- **Godot or Unreal**, with **an agent writing** the documents and a person reviewing them.
- **Studios later, not ignored in engineering:** reproducible builds (R26), headless runs with an
  exit status, budgets per chunk (R44), layers with owners, and files a reviewer can diff.

Not for: someone who wants an editor window, 2D or tile games, or worlds generated while the game
runs.

## What we own

- **The text and its meaning:** documents and merge, the vocabulary, the formula language (R6),
  who gives way when two statements claim one place (authority, R21, R29), and the rule that a
  clash between two firm decisions is an error, never a silent winner.
- **Placement:** instances, rules that lower to them (set, run, scatter), placement by cause as the
  default taught (R5), and hand edits kept as intent at a place (R3), locks (R4).
- **Where things meet:** one surfaces model for ground, decks, floors and caves (R25, R37), the
  meetings between them (R38) and who owns each (R18).
- **Composition:** worlds built apart and joined, with their exits answered (R8 to R11, R49 to R51).
- **The asset contract, not the assets:** ids, packs, measured facts and a taxonomy; a world never
  names a file (principle 6, R30).
- **Checks:** diagnostics that name the place and the fix (principle 8), budgets (R44), what a walk
  may assume (R41), and measures reported beside the eye (R15).
- **The build:** its format, the change report, and same text, same bytes (principle 7, R26).
- **Our libraries** (land, terrain, water, vegetation, ways, town, building, review, map). Eight
  are built only on what a user's step can reach (R19b): the Python SDK and the step protocol.
  `terrain`'s step is a Rust program, `via-terrain`, admitted on stated terms (R34,
  `spec/protocol.md` 8): it reaches the build only through the step protocol, though it links
  `via-num`, `via-proto` and `via-step`, which are not a door users have.
- **One loader done well:** Godot, with the sheet tool that renders and walks the views.

## What we do not own

Each is the engine's, the game's or the user's. The most we do is carry the data that says where
and what, in a form their systems read (ledger section 26).

- Making, converting, optimising, storing or versioning assets.
- Rendering, light, sky, weather, sound and the frame's look (principle 12; doctrine section 7).
- Physics, navigation meshes, streaming in the shipped game, gameplay, crowds, traffic and animal
  behaviour, saves and co-op.
- An editor window, a generator from a prompt, or generation while the game runs.
- The agent: which model, where it runs and what it costs.
- Styling the player map; we hand over the map as data (doctrine section 7).
- Whether content is fit to ship: whoever reviews and ships it is accountable.

## What we do not promise

- **That the result looks good.** We promise that what you decided holds and the rest settles round
  it. Looks are judged by eye; a measure informs and never passes or fails a world (R15). Our own
  worlds are stand-in shapes.
- **A frame rate.** We group instances so an engine can batch them, and count per chunk against
  limits you set (R44). The number is the engine's.
- **Unreal, or a maintained plugin beyond the Godot loader, in the first cut** (doctrine section 7).
- **Worlds past a few kilometres in the first cut** (doctrine section 7).
- **Linux and Windows, tested.** The code stays portable; same bytes is checked on macOS only, until
  the owner asks (doctrine section 7).
- **The same bytes across engine versions,** or from your own steps: same text gives same bytes for
  one engine version, and a user's step is recorded, not promised (R26).
- **Anything kept outside the text.** An edit made in the engine's editor is lost on the next build
  (principle 1).
- **An exact port of an existing map** (doctrine section 7).
- **Stable words and keys before the first cut** (ours): names still move (`spec/vocabulary.md`,
  ledger section 25).
- **Support, a service or a warranty** (ours, beside the licence): the code is MIT and provided as
  is (`LICENSE`). Nothing has shipped with it.

The honest list of what does not work yet is `docs/challenges.md`, each example's README, and every
Open in `docs/faq.md`.
