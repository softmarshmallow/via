# Formulas

A formula is a short expression in our own closed language that gives a value at a point, or once
per feature, from values the engine can see (R6, R7). This chapter fixes its text, its types and
rates, the names it may use in each place, its function table and how it fails.

Words are those of `vocabulary.md`. Numbers, frames, posts, windows, halos and seed derivation are
defined in `numbers.md`; this chapter uses them and does not restate them.

## 1. What a formula is

- **In a formula key.** A key whose **Takes** column in `vocabulary.md` says `formula` holds a TOML
  number or a TOML string. The key decides that a string is a formula (`vocabulary.md` 2.4). A
  number is a constant formula. A string must start with `=`, then the expression:
  `base = "= 20 + 0.06 * y"`. A string that does not is `formula.unmarked`.
- **In an attribute.** A feature attribute, or a kind's `defaults`, has no Takes column, so the mark
  decides: a TOML string that starts with `=` is a formula, and any other string is text. Text that
  starts with `=` cannot be written as an attribute; no world has needed it. Section 6.7 gives the
  types.
- In either place, a string whose first character other than space or tab is `=`, but which does
  not start with it, is `formula.unmarked`, with the hint to remove what comes before `=`.
- This settles doctrine section 6's "one convention for both": every formula, everywhere, is
  written `"= expr"`.
- A formula has no statements, no assignment, no loops, no user functions and no imports (R6).
  Named sub-expressions are `let` names, declared in TOML (section 5). Logic beyond this is a step
  (`protocol.md`).
- A formula is checked in four phases. Nothing that can be checked earlier waits for a later one.

| Phase | When | Checks |
|---|---|---|
| text | After merge, before any node runs. | Marking, characters, size, syntax, names, types, rates, arguments, selector form and class, ids written in text, shadowing, seed form, missing cap, ranges of constant parameters. |
| selection | When the reading node's inputs are known, before it evaluates (`graph.md`). | What a selector resolves to, including things producers made: `formula.selector`, `formula.selector_count`, `formula.empty_selection`, `formula.shape`. |
| feature | When a feature's attributes are worked out, before its formulas evaluate. | Ranges of parameters given per feature (`cap`, `wavelength`, `octaves`, ...). |
| value | While computing. | `formula.not_finite`, `formula.range` on results and per-post parameters, `formula.outside_field` (section 9). |

## 2. Lexical grammar

The formula text is everything after the leading `=`. It is at most 4096 bytes
(`formula.too_long`). Outside a selector, only the characters below may appear; any other
character (upper case, non-ASCII, `$`, `#`, `[`, `;`, ...) is `formula.character`, naming it. An
upper-case letter gets the hint "names are lower case".

| Token | Form | Notes |
|---|---|---|
| whitespace | space, tab, line feed, carriage return | Separates tokens; otherwise ignored. A TOML multi-line string may hold a long formula. |
| number | `digits [ "." digits ] [ ("e" / "E") ["+" / "-"] digits ]` | `0.5`, `12`, `1e-3`. Read to a 64-bit float by the correctly rounded conversion of `numbers.md`. No sign (a sign is an operator). |
| identifier | `[a-z][a-z0-9_]*` | Same form as an id (`identity.md`). |
| selector | `'` any characters but `'` and control characters `'` | Single quotes only. Its contents are checked by section 7. |
| operator | `+ - * / < <= > >= == != ( ) , . =` | `=` appears only after a parameter name in a call. |
| keyword | `and`, `or`, `not` | Reserved: never a name. |

Each of these is `formula.syntax`, with the hint shown:

| Written | Hint |
|---|---|
| A number followed at once by a letter, digit, `.` or `_` (`1e`, `2e5x`, `1.e3`, `5.`, `1_0`) | write the number in full, `1e3`, `5.0` |
| `.5` | write `0.5` |
| A literal that rounds to infinity (`1e400`), or a non-zero literal that rounds to 0 (`1e-400`) | the largest and smallest magnitudes a double holds |
| An empty formula, `"="` | write an expression after `=` |

Operators other languages use are not tokens. Each is `formula.operator`, with the hint shown:

| Written | Hint |
|---|---|
| `&`, `&&` | write `and` |
| `\|`, `\|\|` | write `or` |
| `~`, `!` (alone) | write `not` |
| `**`, `^` | write `pow(a, b)` |
| `%` | write `a - b * floor(a / b)` |
| `"` | selectors take single quotes |
| `?`, `:` outside a selector | write `if(c, a, b)` |
| `=` anywhere but after a parameter name in a call (`x = 1`) | write `==` |

## 3. Syntax

EBNF (ISO 14977). Whitespace may appear between any two tokens.

```ebnf
formula    = "=" , expr ;
expr       = or_expr ;
or_expr    = and_expr , { "or" , and_expr } ;
and_expr   = not_expr , { "and" , not_expr } ;
not_expr   = "not" , not_expr | compare ;
compare    = sum , [ cmp_op , sum ] ;
cmp_op     = "<" | "<=" | ">" | ">=" | "==" | "!=" ;
sum        = product , { ( "+" | "-" ) , product } ;
product    = negation , { ( "*" | "/" ) , negation } ;
negation   = "-" , negation | primary ;
primary    = number | selector | call | name | "(" , expr , ")" ;
call       = identifier , "(" , [ arguments ] , ")" ;
arguments  = argument , { "," , argument } ;
argument   = [ identifier , "=" ] , expr ;
name       = identifier , [ "." , identifier ] ;
```

Precedence, loosest first:

| Level | Operators | Associativity | Operands | Result |
|---|---|---|---|---|
| 1 | `or` | left | yes/no | yes/no |
| 2 | `and` | left | yes/no | yes/no |
| 3 | `not` | prefix | yes/no | yes/no |
| 4 | `< <= > >= == !=` | none: `a < b < c` is `formula.syntax`, hint `a < b and b < c` | number | yes/no |
| 5 | `+ -` | left | number | number |
| 6 | `* /` | left | number | number |
| 7 | `-` (negation) | prefix | number | number |
| 8 | call, name, literal, `( )` | | | |

- There is no unary `+`; it is `formula.syntax`.
- Nesting is at most 64 deep, counting parentheses, calls and prefix operators; deeper is
  `formula.too_deep`. With the size limit of section 2, this keeps a parser's stack bounded for
  text an agent wrote.
- `and` and `or` evaluate both sides. There is no short circuit (section 9).
- In a call, positional arguments come before named ones, each parameter is given at most once, and
  every name is a parameter of the function. A breach is `formula.arguments`.
- `a.b` reads attribute `b` of feature `a` (section 6.2). It is not a place name; a place name is
  always a selector in quotes.

## 4. Types and rates

### 4.1 Types

| Type | Values | Literals |
|---|---|---|
| number | Finite 64-bit floats (`numbers.md`). Units are those of the key or name; the language has none. | `12`, `0.5`, `pi` |
| yes/no | yes or no. | None. Write a comparison. |
| selector | A name of a thing (section 7). Legal only as an argument whose parameter takes one. | `'beck'`, `'kind:lane'` |

There is no conversion between types. `1 and x > 0` is `formula.type`; so is `if(d, 1, 0)` where
`d` is a number. A bool attribute reads as yes/no. Reading a text, array or table attribute is
`formula.type`, naming the attribute.

### 4.2 Rates

Every expression has a rate: how often its value can change.

| Rate | Changes | Example names |
|---|---|---|
| constant | Never, within one build. | literals, `pi`, a constant `let`, `<id>.<attribute>` |
| per feature | Once per feature. | the bare attributes a kind's formula reads, `under(...)` |
| per post | At each post of the grid (`numbers.md`). | `x`, `y`, `d`, `f('height')` |
| per voxel | At each voxel. (open): volume formulas arrive in M5, which decides their names. | `z` |

Rates are ordered as listed. An expression's rate is the highest rate among its operands and its
function's own rate (section 8, column **Rate**). A `let` name has the rate of its formula.

A bare attribute in a kind's formula is per feature, since one kind text serves many features.
`<id>.<attribute>` names one feature, so it is constant wherever it is read. In a feature's own
`where` and attributes, there is one feature, and per feature and constant coincide.

**Sharing.** The evaluator may compute a sub-expression once per unit of its rate and share the
result among every place that reads it: a constant once per build, a per-feature part once per
feature, a per-post part once per post, a `let` once per unit of its rate however often it is read.
Every operation is pure, so sharing changes no bit (R26). The evaluator never reorders, fuses or
rewrites an operation. A per-feature part that reads `v`, one-argument `under` or `smoothed_along`
is shared per (feature, effect, written field), not per feature (6.5).

**Rate errors.** Each formula key has a highest rate (section 6). A formula above it is
`formula.rate`, naming the sub-expression that raised the rate, for example: "`f('height')` is per
post; an attribute is worked out once per feature". Some parameters also have a highest rate
(section 8, column **Parameters**), because they set a halo, an octave count or a seed that must be
known before a window runs.

## 5. Names

A name is a built-in name, a `let` name, an attribute, or `<id>.<attribute>`. Functions are called,
never named bare: `fbm` alone is `formula.unknown_name`, unless a `let` or attribute is called
`fbm`. Calls and names never meet in the grammar.

- **Built-in names**: `pi`, `x`, `y`, `d`, `sd`, `t`, `v`, and later `z`. Each is visible only in
  the contexts of section 6.
- **`let` names** come from `[let]` (the world's), `[field.<n>.let]` and `[kind.<k>.let]`. Each
  value is a formula. They resolve by dependency, never by order in the file; a cycle among them is
  reported by the graph's one cycle report (`graph.md`). A `let` is checked against the context of
  each place that reads it. A `let` nothing reads is still checked, against its own table's widest
  context.
- **Scopes.** There are three: the world (world `let` names), a field (its `let` names) and a kind
  (its attributes and its `let` names). A field's formulas see the field's scope, then the world's.
  A kind's formulas see the kind's scope, then the world's. A name found in the inner scope hides
  the same name in the world's, without a message: a library's `width` means its own attribute
  whatever lets a world declares, so a library that works in one world works in all (principle 5).
- **Shadowing is an error only where it is ambiguous.** `formula.shadow` is raised when a `let`
  name or a declared attribute equals a built-in name or a keyword, or when one name is declared
  twice in one scope (a kind's attribute and a kind `let`). Example: a kind that declares an
  attribute `d`. A name equal to a function's (`floor`, `max`) is legal. The spike's silent override
  of built-ins (SPEC 11) does not carry over.
- **A `let` named like a field.** A `let` name, in any scope, that equals the name of a field is
  `formula.shadow` (`vocabulary.md` 2.1), raised once, by this chapter (`documents.md` 3). Otherwise
  `height` and `f('height')` would name two things (principle 9).
- An unknown name is `formula.unknown_name`, with the nearest visible name (`diagnostics.md` 1.4)
  and, if the name exists in another context, which one ("`d` exists in a kind effect, not in a
  field base").
- A formula reading a `let`, field or feature of a higher layer is `doc.upward_ref`
  (`documents.md` 5.1); a formula reads downward like every other reference.

## 6. Names by context

### 6.1 Formula keys

| Key | Result | Highest rate | Range | Context |
|---|---|---|---|---|
| `[let]` value | number or yes/no | per post | | 6.2 |
| `[field.<n>.let]` value | number or yes/no | per post | | 6.3 |
| field `base` | number | per post | | 6.3 |
| `then` row `value` | number | per post | | 6.4 |
| `then` row or effect `mask` | number | per post | 0 to 1 | 6.4, 6.5 |
| effect `value` | number | per post | | 6.5 |
| effect `max_change` | number | per feature | 0 or more | 6.5, without `v`, `d`, `sd`, `t` |
| `[kind.<k>.let]` value | number or yes/no | per post | | 6.5 |
| feature `where` | yes/no, or number (the region is where it is above 0) | per post | | 6.6 |
| scatter `density` | number | per post | 0 to 1 | 6.8 |
| kind surface `base` | number, metres | per post, read at any point asked (6.9) | | M3; names in `surfaces.md` 1.1 |
| kind surface `bound` | number, metres | per feature | | M3; names in `surfaces.md` 1.1; not above 0 or not finite is `surface.bound` |
| attribute, kind `defaults` | number | per feature | | 6.7 |
| import `blend` | number | per post | 0 to 1 | M4; (open) names decided in `documents.md` |
| volume `base`, effect on a volume | number | per voxel | | M5 (open) |

A value outside a key's range is `formula.range`, with the hint `clamp(..., 0, 1)`, reported as
section 9 says. It is never clamped silently (principle 3). The range belongs to the key, not to
the language.

`max_change` takes a per-feature formula, so a kind can tie it to an attribute (the spike's pad read
`max_cut`). This answers the question `authority.md` 2.2 leaves here.

`where` is the only key that takes two result types: `vocabulary.md` defines it as "above 0", and a
yes/no is accepted because a condition is the usual way to write a region. A yes/no `where` means
"where it is yes".

### 6.2 Everywhere a per-post formula is allowed

- `pi`;
- `x`, `y`: the coordinates of the point being computed, in the frame the formula is evaluated in
  (the world frame, or an import's own world frame, `numbers.md`);
- the world's `let` names;
- `<id>.<attribute>`: the attribute of feature `<id>` after its own formula, constant (4.2). `<id>`
  is a feature in this world; a place inside an import is M4. A feature may name itself this way;
  the value is its own attribute;
- every function of section 8 whose **Where** column allows the context.

### 6.3 A field's base and its `let`

Section 6.2, plus the field's own `let` names. No `v`. Reading the field itself with `f` is a cycle
(`graph.md`).

### 6.4 A `then` row

Section 6.3, plus `v`: the field's value at the post before this row (the base and every earlier
row, in the order of `authority.md` 4). On a `weights` field there is no `v`; a row there has
`mask` only.

### 6.5 A kind effect and a kind's `let`

Section 6.2, plus:

| Name | Rate | Meaning |
|---|---|---|
| each attribute | per feature | The kind's `defaults` overlaid with the feature's own values (`authority.md`). |
| the kind's `let` names | own | As section 5. |
| `d`, `sd`, `t` | per post | The feature's local frame at the point (`numbers.md` 5.5). On a point feature `t` is 0. |
| `v` | per post | The written field at the post before this entry, in the order of `authority.md` 4. Scalar fields only; on a `weights` field `v` is `formula.unknown_name`. |

- **One entry per written field.** An effect whose `field` matches several fields makes one entry
  on each (`authority.md` 2). Its formulas are evaluated once per entry, and `v`, one-argument
  `under` and `smoothed_along` read that entry's field.
- A kind `let` that reads `v`, one-argument `under` or `smoothed_along` is evaluated once per
  (feature, effect, written field) that reads it. Such a `let` may be read only by an effect's
  `value` or `mask` on a scalar field; read elsewhere it is `formula.unknown_name` for `v`.
- An effect is evaluated only within `radius` of its feature (`authority.md`).

### 6.6 A region condition, `where`

Section 6.2, plus the feature's own attributes. A `where` may read fields and other features,
including another `where` region. Reading its own feature, directly or through others, is a cycle
(`graph.md`). What becomes of the condition's pieces is `geometry.md`.

In a `where`, `f` and `slope` read a field's **head view** (`authority.md` 2.3): its base and its
head (`authority.md` 4.1), every `then` row of every layer, rows a step returned included, never an
effect. This is not a step's head view (`authority.md` 4.4). This follows the doctrine's build order, "fields from their
bases, then features". It also keeps regions from being cycles: the order of effects depends on
region shapes (nesting, `authority.md` 4.3), so a region that read the finished field would depend
on itself whenever any region of its tier wrote that field. A feature whose `where` reads a field
then conforms to it (`authority.md` 2.3).

Everywhere else, `f` reads the build's own view, the finished field. A formula a step evaluates
through `expr.eval` reads the step's view (`fields.md` 6.2).

### 6.7 Attribute formulas

An attribute's type is that of its kind's default: a number or a formula gives a number, a bool a
yes/no, a string text. An attribute named only in `requires` takes its type from what each feature
writes (`authority.md` 1.1). An attribute formula must give a number; a yes/no is `formula.type`. A
formula written where the attribute is a bool or text is `formula.type`, naming the attribute and
its kind's default.

An attribute formula is worked out once per feature, before any shape or field, in dependency
order (`graph.md`). Afterwards it is a plain number. Names:

| Name | Meaning |
|---|---|
| the feature's own attributes | As 6.5. |
| `<id>.<attribute>` | As 6.2. |
| the world's constant `let` names | A per-post `let` is `formula.rate`. |
| `pi` and the arithmetic functions of 8.1 | Functions with rate per post are `formula.rate`. |

A kind's `defaults` may be formulas, so every feature of the kind has derived attributes others
can read (the `pistol` case, challenge 47). Whether formulas may also give positions (`at`, `path`)
or a facing is (open): decided in M5 by challenge 47's owed rotation.

### 6.8 A scatter's density

Section 6.2. `keep_out` and `clump` are applied by the scatter itself (`rules.md` 7.1); the density
repeats neither. Growth beside another producer is `clump.around`, not a formula: it reads
neighbours (R7, `rules.md` 7.2).

### 6.9 A formula at a point that is not a post

A scatter's density is evaluated at a candidate's point (`rules.md` 7.1), a kind surface's `base` at
the point asked (`surfaces.md` 1.1), and `expr.eval` at any points a step sends. A post is one such point;
nothing below changes a value at a post.

| Name or function | At a point `p` |
|---|---|
| `x`, `y` | `p`'s coordinates, millimetres as a double divided by 1000 (`numbers.md` 4). |
| `d`, `sd`, `t`, `dist`, `sdist`, `near`, `inside`, `along` | At `p`, a lattice point, by `numbers.md` 5.5, as at a post (8.4). |
| noise | At `p`'s coordinates. |
| `f` | The field's value at `p` by `fields.md` 1.2: its cell rule and its three lines `s`, `n` and the value, in that order. At a post the value is the post's, bit for bit. |
| `slope` | By the same rule, from the slope at the four posts around `p` (`fields.md` 1.2). |
| `v`, `under`, `smoothed_along` | As at a post: `v` exists only in rows, which are evaluated at posts; the others are per feature or read at `t`. |

A point beyond the extent gives no field value: `formula.outside_field`, handled as section 9
handles a non-finite value. A kind surface is not a field: no selector matches it and `f` never
reads it (`surfaces.md` 1, rule 5); reading a field from a surface formula is `surfaces.md` Open 9.

## 7. Selectors

A selector is its own literal type: a name of a thing, in single quotes, passed to a parameter that
says what class of thing it accepts. The forms are those of `vocabulary.md` section 3, Selectors;
`identity.md` resolves them.

| Class | Accepted forms | Used by |
|---|---|---|
| field | `'<field name>'`; `'tag:<tag>'` matching exactly one field | `f`, `slope`, `under` |
| shape | `'<id>'` of a feature; `'kind:<name>'`; `'<id>.start'`, `'<id>.end'`, `'<id>.<mount>'` | `dist`, `sdist`, `near`, `along`, `inside` |
| seed | `'<name>'`, an id-form name | `fbm`, `ridged` |
| statistic | `'mean'`, `'min'`, `'max'`, `'edge_min'` | `under` |

- **Resolved at a stratum.** A selector resolves to the things visible where the formula is
  evaluated: features written in text, and features and instances made by producers the graph
  orders before the reading node (`graph.md`). A layer sees its own rank and below (R21). A selector
  whose resolution would need the reading node's own output is a cycle.
- **No produced names.** Produced names are never written in text (R3, `identity.md` 3), and a
  formula is text. A selector containing `#` is `identity.key_in_text`. A formula reaches produced
  things by what they are: features a producer made by `'kind:<name>'`, and a field a step returns
  by a tag it gives that field (`'tag:<tag>'`) (open: needs ruling, R21; `fields.md` 7). The
  field class takes no produced name. (`identity.md` 3's list of keys that raise
  `identity.key_in_text` is owed "formula selectors".)
- `'role:<role>'` in a formula is `formula.selector`, with the hint to tag the field and use
  `'tag:<tag>'` (formulas never read by role, `vocabulary.md` 2.4).
- A selector that names nothing, or names a thing of another class, is `formula.selector`, with the
  nearest match.
- **One code for too many.** A selection that holds more things than the function takes (a field
  selector matching several fields, `sdist` over several paths, `along` over several features), or
  none where the function needs one (a field selector, `along`), is `formula.selector_count`,
  naming what it found.
- **Wrong shape.** A selection holding a shape the function cannot take is `formula.shape`.
- **Empty.** A `kind:` selection with no features at its stratum, read by `dist`, `sdist`, `near` or
  `inside`, is `formula.empty_selection` (warning), once per formula and stratum. These functions
  then return their "nothing near" value: `dist` its cap, `sdist` minus its cap, `near` 0, `inside`
  no.
- A place inside an import (`'a.quay_road.end'`) is M4.

## 8. The function table

This table is normative and is the single source for the evaluator, the reference page and the
SDK's function list (section 11). Angles are degrees, matching bearings (`vocabulary.md`).

**Rate**: the function's own rate; its result is the higher of this and its arguments' rates.
**Halo**: what it reads beyond the point being computed (`numbers.md` 6). A window's values equal
the same cut of the whole world; every function below honours that by reading its halo, never by
changing its rule at a window's edge. **Where**: the contexts of section 6 it may appear in; "any"
is every context its rate allows; a call outside them is `formula.unknown_function`, saying where
the function exists. **M**: the milestone that needs it; earlier milestones reject it with
`doc.not_yet`.

### 8.1 Arithmetic and shaping

| Function | Parameters | Result | Rate | Halo | Meaning | M | Source |
|---|---|---|---|---|---|---|---|
| `abs(a)` | number | number | none | none | Absolute value. | 1 | |
| `sqrt(a)` | number | number | none | none | Square root. `a < 0` is non-finite (section 9). | 1 | |
| `pow(a, b)` | number, number | number | none | none | `a` to the power `b`. | 1 | |
| `exp(a)` | number | number | none | none | e to the power `a`. | 1 | |
| `floor(a)` | number | number | none | none | Largest whole number not above `a`. | 1 | |
| `sin(a)`, `cos(a)`, `tan(a)` | number, degrees | number | none | none | Trigonometry in degrees, by the reduction below. | 1 | Ours, after `numbers.md` 2 |
| `min(a, b, ...)`, `max(a, b, ...)` | two or more numbers | number | none | none | Folded left to right by the binary rule of `numbers.md` 8, rule 4: `min(a, b, c) = min(min(a, b), c)`. | 1 | |
| `clamp(a, lo, hi)` | number ×3 | number | none | none | `min(max(a, lo), hi)`. `lo > hi` at a post is `formula.range`. | 1 | |
| `mix(a, b, s)` | number ×3 | number | none | none | `(1 - s) * a + s * b`, in that order. Exact at `s = 0` and `s = 1`; the same form as `fields.md` 1.2 and 5.1. | 1 | GLSL `mix` (Khronos GLSL 4.60, section 8.3) |
| `smoothstep(e0, e1, a)` | number ×3 | number | none | none | With `s = clamp((a - e0) / (e1 - e0), 0, 1)`: `s * s * (3 - 2 * s)`. `e0 >= e1` at a post is `formula.range`. | 1 | GLSL `smoothstep` (Khronos GLSL 4.60, section 8.3) |
| `if(c, a, b)` | yes/no, then two of one type | that type | none | none | `a` where `c` is yes, else `b`. Both are computed; the branch not taken is not checked (section 9). | 1 | |

**Degrees.** For `a` in degrees: `r = fmod(a, 360)` (exact, from `via-num`), plus 360 if `r < 0`.
The quadrant `q` is 0, 1, 2 or 3 as `r` is below 90, 180, 270 or 360, and `s = r - 90 * q`, which
is exact (Sterbenz). With `h = s * (pi / 180)` (the double division of `pi` by `180.0`),
`sin` of quadrants 0 to 3 is `sin(h)`, `cos(h)`, `-sin(h)`, `-cos(h)`; `cos` is `cos(h)`,
`-sin(h)`, `-cos(h)`, `sin(h)`. A zero result is `+0`. `tan(a)` is `sin(a) / cos(a)` from these.
So `sin(180)`, `cos(90)` and `sin(360)` are exactly 0, `sin(90)` is 1, `cos(180)` is -1, and
`tan(90)` is non-finite, never a large finite number.

### 8.2 Noise

| Function | Parameters | Result | Rate | Halo | Meaning | M | Source |
|---|---|---|---|---|---|---|---|
| `fbm(wavelength, octaves = 4, seed = <owner>, gain = 0.5, lacunarity = 2)` | number (m, > 0); a whole number 1 to 12; seed selector; number, 0 < gain <= 1; number > 1; all at most per feature | number in -1 to 1 | per post | none | Below. | 1 | Fractional Brownian motion (Musgrave, in Ebert et al., *Texturing and Modeling*, 1994) over gradient noise (Perlin, 2002) |
| `ridged(...)` | as `fbm` | number in 0 to 1 | per post | none | Below. | 2 | Ridged multifractal (Musgrave, 1994), without its weight feedback |

With `o` octaves, for `i` from 0 to `o - 1`: `w_i = wavelength / pow(lacunarity, i)`,
`g_i = pow(gain, i)`, and `n_i` the gradient noise of octave `i` at `(x / w_i, y / w_i)`. Then

```
fbm    = (g_0 * n_0 + g_1 * n_1 + ...) / (g_0 + g_1 + ...)
ridged = (g_0 * r_0 + g_1 * r_1 + ...) / (g_0 + g_1 + ...),   r_i = (1 - abs(n_i)) * (1 - abs(n_i))
```

Each sum runs from `i = 0` upward. A parameter outside its range, including an `octaves` that is not
a whole number, is `formula.range`.

- **Seed.** `seed` is a name, never a number (`formula.seed`). An explicit seed's stream is
  `stream(world seed, ["noise", "seed", <name>])` (`numbers.md` 7.2). One name gives one noise
  wherever it is used, so two fields share a pattern by sharing a name.
- **Default seed.** Without `seed`, the stream's parts name the formula's owner and its place in
  the owner, so no two places share a default pattern:

| Formula | Parts |
|---|---|
| A world `let` | `["noise", "let", <name>]` |
| A field's `base`, `let`, `then` row key | `["noise", "field", <field>, "base"]`, `[..., "let", <name>]`, `[..., "then", <row id>, <key>]` |
| A feature's `where` | `["noise", "feature", <feature id>, "where"]` |
| A kind effect key, a kind `let`, for one feature | `["noise", "feature", <feature id>, "effect", <effect id>, <key>]`, `[..., "let", <name>]` |
| A scatter's density | `["noise", "scatter", <id>, "density"]` |
| A kind surface's `base`, for one feature | `["noise", "feature", <feature id>, "surface", <name>, "base"]` |

  The second part is a class or `"seed"`, so a default stream never meets an explicit one, and a
  field and a scatter of one name differ. A noise call inside a `let` takes the `let`'s parts, so a
  `let` read twice is one noise.
- **`formula.seed_shared`** (warning): after `let`s are resolved, two distinct noise calls among all
  the formulas of one owner (a field, a feature, a scatter, a world `let`) that use one stream. They
  are scaled copies of one pattern.
- **Octaves** are independent: octave `i` hashes its lattice with `i` among the keys.

**The gradient noise.** `n_i` is Perlin's improved noise (Perlin, "Improving Noise", SIGGRAPH 2002)
in two dimensions: his quintic fade and his twelve edge gradients, evaluated in the plane `z = 0`.
His permutation table is replaced by our lattice hash, `draw` of `numbers.md` 7.2; the replacement
is ours. For a stream `s`, octave `i` and a point `(px, py) = (x / w_i, y / w_i)`, each a double
division:

```
ix = floor(px), iy = floor(py)          as signed 64-bit integers
fx = px - ix,   fy = py - iy            exact, in [0, 1)
corner(a, b) = let (gx, gy) = G[draw(s, [ix + a, iy + b, i]) >> 60]
               in gx * (fx - a) + gy * (fy - b)
fade(t) = t * t * t * (t * (t * 6 - 15) + 10)
u = fade(fx), v = fade(fy)
n_i = clamp(mix(mix(corner(0, 0), corner(1, 0), u), mix(corner(0, 1), corner(1, 1), u), v), -1, 1)
```

`>> 60` keeps the top four bits, an index from 0 to 15 into `G`, Perlin's sixteen-entry gradient
list (the twelve edges of a cube, four of them twice) at `z = 0`:

| Index | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `(gx, gy)` | (1, 1) | (-1, 1) | (1, -1) | (-1, -1) | (1, 0) | (-1, 0) | (1, 0) | (-1, 0) | (0, 1) | (0, -1) | (0, 1) | (0, -1) | (1, 1) | (1, -1) | (-1, 1) | (-1, -1) |

- **Operations.** Only `+`, `-`, `*`, `floor` and comparisons, each correctly rounded, in the
  order written (`numbers.md` 8, rules 1 and 3); `mix` and `clamp` are those of 8.1. So `n_i` is the
  same bits on every system. Only `w_i` uses `pow`, from `via-num`.
- **Keys.** `[ix + a, iy + b, i]`: the lattice corner, then the octave. A corner whose integer
  would leave the signed 64-bit range, or whose `px` or `py` has a magnitude of 2^52 or more, is
  `formula.range` on `wavelength`, never a wrap. No world at M1 comes near it.
- **Range.** With gradients no longer than `sqrt(2)`, two-dimensional Perlin noise lies in -1 to 1
  (its known bound, `sqrt(N / 4)` for unit gradients in `N` dimensions, scaled by `sqrt(2)`). The
  final `clamp` makes that hold to the last bit, so the ranges in the table hold.
- **Zeros.** `n_i` is 0 at every lattice corner of its octave. With `lacunarity = 2`, the octaves'
  lattices share the corners on multiples of `wavelength`, where `fbm` is exactly 0. These are
  isolated points, not lines; accepted for M1.
- Only the vector bytes in `spec/vectors/`, written with the code at M1's shared base, remain to
  confirm this on macOS, Linux and Windows (R26).

Noise vectors, `s = stream(0, ["noise", "seed", "hills"])`. Every value is exact in binary64.

| Input | Value |
|---|---|
| `draw(s, [0, 0, 0])` | `0x4B53AD8575D91FE2`, index 4 |
| `draw(s, [1, 0, 0])` | `0x8A012828A633CA3F`, index 8 |
| `draw(s, [0, 1, 0])` | `0xF1B145B81005BEF1`, index 15 |
| `draw(s, [-1, 0, 0])` | `0x3A665A82041D210B`, index 3 |
| `draw(s, [0, 0, 1])` | `0x286CF8A0063C878F`, index 2 |
| `n_0` at `(px, py) = (0.5, 0.5)` | 0.375 |
| `n_0` at `(0.25, 0.75)` | 0.054436683654785156 (`0x3FABDF2000000000`) |
| `n_0` at `(-0.5, 0.5)` | -0.25 |
| `n_1` at `(0.5, 0.5)` | 0 |
| `n_0` at `(3, 2)`, a corner | 0 |

Stream vectors, `numbers.md` 7.2 with world seed 0:

| Parts | Stream |
|---|---|
| `["noise", "seed", "hills"]` | `0xB2257365AD2F65DB` |
| `["noise", "seed", "a"]` | `0x28499B517E36090E` |
| `["noise", "field", "height", "base"]` | `0x2B7A1CEA060EDEB6` |
| `["noise", "field", "trees", "base"]` | `0xD7218EDC7B009158` |
| `["noise", "scatter", "trees", "density"]` | `0x4A7DFA81BAFF877F` |

### 8.3 Field reads

| Function | Parameters | Result | Rate | Halo | Meaning | M |
|---|---|---|---|---|---|---|
| `f(field)` | field selector | number | per post | none at a post; 1 post at a point (6.9) | The field's value at the point, in the view of 6.6. A `weights` field is `formula.type`. Reading the field being built is a cycle; inside it, use `v`. | 1 |
| `slope(field)` | field selector | number, degrees, 0 to 90 | per post | 1 post | Below. | 1 |

**Slope.** At post `(i, j)`, with `r` the spacing in metres and `h` the field's values:
`gx = (h[i+1, j] - h[i-1, j]) / (2 * r)` and `gy` likewise along `j`. Where a neighbour lies beyond
the extent, the difference is one-sided: `(h[i+1, j] - h[i, j]) / r`
or `(h[i, j] - h[i-1, j]) / r`. With no neighbour on either side, there is no value
(`formula.outside_field`). Then `slope = atan(sqrt(gx * gx + gy * gy)) * (180 / pi)`, with `atan`
from `via-num` and `180 / pi` the double division. Central differences are the usual stencil; the
edge rule is ours.

- A `volume` field is rejected before M5 (`doc.not_yet`, `fields.md`); M5 decides `f` on one.
- How a formula reads one material's weight is (open): a function of a `weights` field and a
  `paint`, added when the first world needs one (M1 `ford` or M2 `glade`).

### 8.4 Distance and membership

`cap` bounds the work (the spike's ten-minute distance to an 8 km shape, FINDINGS). A distance
beyond `cap` is returned as `cap`.

| Function | Parameters | Result | Rate | Halo | Meaning | M |
|---|---|---|---|---|---|---|
| `dist(shape, cap)` | shape selector; number (m, > 0) at most per feature | number | per post | geometry within `cap` | The least `d` (`numbers.md` 5.5) over the selection, at most `cap`. | 1 |
| `sdist(shape, cap)` | as `dist` | number | per post | geometry within `cap` | `sd` of `numbers.md` 5.5, in `-cap` to `cap`. Regions: the greatest `sd` among them, so positive inside their union. One path: its `sd`. Points: minus `dist`. A selection mixing regions, paths and points is `formula.shape`; several paths is `formula.selector_count`. | 1 |
| `near(shape, radius)` | shape selector; number (m, > 0) at most per feature | number | per post | geometry within `radius` | `1 - smoothstep(0, radius, dist(shape, radius))`. | 1 |
| `inside(shape)` | shape selector of regions | yes/no | per post | geometry within the window | Yes where the point is in a region of the selection, its outline included (exact winding, `numbers.md` 5.5). A path or point is `formula.shape`. | 1 |
| `along(shape)` | shape selector naming one feature | number | per post | the whole feature | `t` of that feature at the point (`numbers.md` 5.5): 0 on a point. | 1 |

- **Arithmetic.** `dist` and `sdist` add no arithmetic of their own. For each feature of the
  selection, `d` and `sd` are those of `numbers.md` 5.5: its exact choice of the nearest segment
  or vertex, its expressions for `d` and its exact side test. Over several features, `dist` is the
  least `d` and `sdist` the greatest `sd`, folded in id order by `numbers.md` 8, rule 4, then
  bounded by the cap. So `sdist('pond', cap)` equals the pond's own `sd` at every post within the
  cap, bit for bit.
- **The cap.** `cap` may be left out only in a kind's formulas. In an effect it defaults to the
  effect's `radius`, else the kind's; in a kind `let`, to the kind's `radius`. With neither, or
  anywhere else, a missing `cap` is `formula.cap`, whose hint gives a cap from the world's extent
  and says what it costs.
- A shape selector may name the formula's own feature only in a kind effect; there `d`, `sd` and
  `t` are the short form, with the same value and sign.

### 8.5 Measures under a feature

| Function | Parameters | Result | Rate | Halo | Where | Meaning | M |
|---|---|---|---|---|---|---|---|
| `under(stat)`, `under(field, stat)` | statistic selector; or field and statistic | number | per feature | the feature's footprint | kind effect, kind `let` | A measure of a scalar field over the feature's samples: the entry's field before this entry (one argument), or a named field in the view of 6.6. | 1; `'edge_min'` 2 |
| `smoothed_along(length)` | number (m, > 0), at most per feature | number | per post | the whole feature | kind effect on a scalar field, kind `let` | Paths only: the entry's field before this entry, smoothed along the path, read at the point's `t`. A region or point is `formula.shape`. | 1 |

**Samples of a feature** are those of `numbers.md` 6, in its order: a path's points at arc length
`0, r, 2r, ...` while below `L`, then `L`, each rounded to the lattice; a region's posts inside it,
outline included, in post order; a point's one point. So a path shorter than `r` has two samples,
its ends. Each sample's value is the field's at that lattice point, by `fields.md` 1.2.

- A sample beyond the extent is `formula.outside_field`.
- A feature with no samples (a region with no post inside) is `num.empty_reduction`, naming the
  feature (`numbers.md` 6). There is no fallback to its written points: the engine says it and
  invents no value (principle 8). The author makes the region larger or the spacing finer.

**`under`.** `'mean'` is the sum of the samples by `numbers.md` 8, rule 5 (the chunk rule), in sample
order, divided by their count. `'min'` and `'max'` fold the samples in order by `numbers.md` 8,
rule 4.
`'edge_min'` (regions only, else `formula.shape`) is the least value at the posts within 1.5
spacings of the region's outline, inside or outside it, in post order. The test is exact on the
lattice: a post `p` is in the band when, for some outline segment, `4 * dist² <= 9 * r²` in integer
square millimetres (for a projection inside the segment, `dist² = cross(e, w)² / |e|²`, compared
without dividing, in 128 bits). The 1.5-post band is the spike's; the exact test is ours.

**`smoothed_along`.**

1. Take the path's samples `v_0 ... v_(n-1)` at arc lengths `s_0 ... s_(n-1)`, as above.
2. Half-width `k = floor(length / (2 * r))`, in doubles. With `k = 0` the passes change nothing.
3. One pass: `u_m` is the mean of `v_j` for `j` from `max(0, m - k)` to `min(n - 1, m + k)`, summed
   left to right and divided by the count. The window shrinks at the ends; a length longer than
   the path makes every value the mean of all.
4. Two passes, the second over the first's result.
5. At a point with arc position `s = t * L`: the largest `m` with `s_m <= s`; the value is
   `mix(u_m, u_(m+1), (s - s_m) / (s_(m+1) - s_m))`, or `u_(n-1)` at `s = L`.

Two passes of a moving mean are the spike's (`profile`); the shrinking ends and the reading at `t`
are ours. The name is a mechanism's: `profile` is a library word (`vocabulary.md` 7).

### 8.6 Reading beyond the point (R7, R28)

R7 as amended by R28: a formula is a value at a point from other values at that point, and may also
call a function of this table that reads around the point when the function declares its halo and
cap, so the engine still sees every input and evaluates by window. An open-ended neighbourhood or an
algorithm stays a step.

| Function | What it reads | Halo and cap |
|---|---|---|
| `d`, `sd`, `t`, `dist`, `sdist`, `near`, `inside`, `along` | A feature's geometry. No field value but the point's own. | Geometry within the cap (`dist`, `sdist`) or the feature's own extent. |
| `slope` | A field at the posts around the point. | One post. |
| `under` | A field over the feature. | The feature's footprint. |
| `smoothed_along` | A field along the path. | The whole path, bounded by its length. |

A function that cannot state its halo and cap does not enter this table.

That is twenty-five functions, against R6's "about twenty"; the count is open to review. Nothing
else is callable.

## 9. Evaluation and failure

- **Arithmetic.** Every operation is IEEE 754 binary64 through the one maths module (`numbers.md`
  8). The evaluator never fuses `a * b + c` into one rounding, never reassociates, and evaluates
  operands left to right, then the operation. `sin`, `cos`, `tan`, `exp`, `pow`, `sqrt`, `atan`,
  `fmod` and the noise come from that module, so the same text gives the same bytes on every system
  (R26).
- **Every failure is located.** A diagnostic from this chapter has a span location
  (`diagnostics.md` 1.2): the file, line and column of the faulty sub-expression, and its bytes
  `[start, end)` in the file, mapped from the formula through the string's span map, escapes and
  line breaks included (`documents.md` 2.1). A fault inside a `let` has the `let`'s span as its
  location and the reading formula's span in `related`. The subject is the formula's key address.
  Nothing in this language becomes a silent zero, and nothing is skipped (the spike's failure
  table, SPEC 4.4, does not carry over).
- **No non-finite value enters a build** (R26). The rule is per operation, not per result:
  - At a post, an operation is **live** unless it lies, at any depth, in a branch of an `if` not
    taken at that post. Every other operand is live when its operation is: both sides of `and` and
    `or`, a comparison's operands, a condition of a live `if`.
  - A live operation whose result at a post is not finite is `formula.not_finite`. So
    `if(sqrt(x - 100) > 1, 1, 0)` fails at `sqrt`, and `1 / exp(1000)` at `exp`, though both
    results are finite. Inputs (fields, attributes, literals) are finite, so the first non-finite
    value always has an operation that made it.
  - A shared per-feature part that is not finite is an error only if it is live at some post that
    reads it. In a per-feature formula, liveness is judged once.
  - The same liveness governs `formula.range` on a parameter (`clamp`, `smoothstep`) and
    `formula.outside_field`. A key's result is always live.
- **Which failure is reported.** One record per formula and owner (the feature, for a kind's
  formula), whatever the windows and threads: the first post in post order over the formula's
  whole domain at which some live operation fails, and at that post the first failing operation in
  evaluation order. Windows may run in any order; the record is the least over all of them. Records
  of several features are sorted by `diagnostics.md` 5. The record names the operation and its
  span, its operand values, the post's position in world coordinates and, where there is one, the
  feature. The hint names a guard (`max(a, 0)` under `sqrt`, `if(b != 0, a / b, 0)`). The node
  fails; what happens to nodes that read it is `graph.md`.
- **Explain mode** is how the engine may find that record: compute columns with a per-operation
  flag, and when a window's flag is set, re-run it post by post. It changes no value. It is an
  implementation choice, not a rule.

## 10. Worked example

A field, a kind in a library, two lanes and a scatter.

```toml
# world.toml
[let]
sea = 12

[field.height]
role = "surface"
tags = ["ground"]
base = "= 20 + 0.06 * y + 5 * fbm(110, 3, seed = 'hills')"

[feature.lane_west]
kind  = "lane"
path  = [[0, 120], [140, 128]]
width = 3.5

[feature.lane_east]
kind  = "lane"
path  = [[140, 128], [256, 140]]
width = "= lane_west.width + 0.5"

[scatter.trees]
assets  = ["@plant/tree"]
spacing = 7
density = "= smoothstep(0.1, 0.5, fbm(60, 3)) * (1 - near('kind:lane', 10)) * if(f('height') > sea, 1, 0)"
```

```toml
# lib/ways/kinds.toml
[kind.lane]
defaults = { width = 4, camber = 0.02 }
radius   = 12

[kind.lane.effect.bed]
field = "role:surface"
value = "= smoothed_along(24) - camber * d"
mask  = "= 1 - smoothstep(width / 2, width / 2 + 6, d)"
```

What the text phase finds:

| Formula | Rate | Notes |
|---|---|---|
| `lane_east.width` | constant | 4.0, worked out once; an edge from `lane_east` to `lane_west`. |
| `height.base` | per post | Noise stream `["noise", "seed", "hills"]`; `20` and `0.06` are constants. |
| `bed.value` | per post | `smoothed_along(24)` reads `height` before this entry along each lane; `camber` per feature. |
| `bed.mask` | per post | `width / 2` and `width / 2 + 6` shared once per lane; `d` per post, only within `radius` 12. |
| `trees.density` | per post | `fbm(60, 3)` on stream `["noise", "scatter", "trees", "density"]`. `near` reads lane geometry within 10 m; `f('height')` is read at each candidate by `fields.md` 1.2 (6.9). |

Change `"= lane_west.width + 0.5"` to `"= lane_west.widht + 0.5"` and the build stops with
`formula.unknown_name`, subject `feature.lane_east.width`, location `world.toml` line 18, column 22,
bytes `[307, 312)`: "`widht` is not an attribute of `lane_west` (kind `lane`)", with
`data.nearest = "width"`.

## 11. One table, three outputs

The function table of section 8 is held once, as data: name, parameters (name, type, selector
class, default, highest rate, range), result type, rate, halo rule, contexts, milestone, meaning and
source. From it are generated:

1. the evaluator's dispatch and its name, type, rate and halo checks (`crates/via-expr`);
2. the reference page users read, one entry per function, in the order of section 8;
3. the SDK's function list (M2). The SDK computes nothing itself: a step evaluates formulas and
   calls these functions through the protocol method `expr.eval` (`protocol.md`), always as a batch
   of points in and a column out (R23). A formula and a worker step therefore give the same bytes,
   and one can be moved to the other (R7). A plain-command step cannot call the engine.

A function added without a row, or a row without a vector in `spec/vectors/`, fails the code checks
(plan, "How every milestone runs"). The data file is `spec/functions.toml` (R33): one
`[function.<name>]` table per function, in the order of section 8, holding its columns; the code
checks hold it equal to section 8. The reference page is generated into `docs/reference/` (R33).

## Diagnostics

| Code | Severity | Says |
|---|---|---|
| `formula.unmarked` | error | A formula key holds a string that does not start with `=`, or any string starts with spaces before `=`. |
| `formula.character` | error | A character outside the language; names it; for upper case, "names are lower case". |
| `formula.too_long` | error | The formula text is over 4096 bytes. |
| `formula.too_deep` | error | Nesting over 64 deep. |
| `formula.syntax` | error | The text does not parse; names the span and what was expected (section 2's table). |
| `formula.operator` | error | An operator of another language, or `=` for `==`; names ours. |
| `formula.unknown_name` | error | A name is not visible here (`data.name`); nearest name, and where it does exist. |
| `formula.unknown_function` | error | A call names no function in the table; the alias table below, else the nearest function. |
| `formula.arguments` | error | Wrong count, an unknown or repeated named argument, or a positional one after a named one. |
| `formula.type` | error | A number where yes/no is needed, or the reverse; a selector out of place; a text, array or table attribute read; a formula for a bool or text attribute; a `weights` field read by `f`. |
| `formula.rate` | error | A sub-expression's rate is above what the key or parameter allows; names it. |
| `formula.shadow` | error | A `let` or attribute name equals a built-in or keyword, a `let` name equals a field's, or one name is declared twice in one scope. |
| `formula.selector` | error | A selector names nothing, is of the wrong class, or uses `role:`. |
| `formula.selector_count` | error | A selection holds more things than the function takes, or none where it needs one. |
| `formula.empty_selection` | warning | A `kind:` selection is empty at its stratum; says the value used. Once per formula and stratum. |
| `formula.shape` | error | A function given a feature of a shape it cannot take. |
| `formula.cap` | error | `dist` or `sdist` without a cap and without a `radius` to default to. |
| `formula.seed` | error | A seed that is not a name. |
| `formula.seed_shared` | warning | Two noise calls of one owner use one stream. |
| `formula.range` | error | A value outside its key's or parameter's range, at a named post or feature. |
| `formula.not_finite` | error | A live operation gave a non-finite value; names the operation, operands, post and feature. |
| `formula.outside_field` | error | A field read at a point beyond the extent. |

`formula.unknown_function` hints from a fixed table first, since `diagnostics.md` 1.4's distance
would not find them:

| Written | Hint |
|---|---|
| `clip` | `clamp` |
| `maximum`, `minimum` | `max`, `min` |
| `where` | `if`; `where` is a feature key |
| `lerp` | `mix` |
| `radians`, `degrees` | angles are degrees; no converter |
| `ln`, `log`, `log10` | none in the table; use `exp` and `pow`, or a step |
| `atan2`, `atan`, `asin`, `acos` | none in the table; a step |
| `mod`, `fmod` | `a - b * floor(a / b)` |
| `round`, `ceil` | `floor(a + 0.5)`, `-floor(-a)` |
| `noise`, `perlin`, `simplex` | `fbm(wavelength, 1)` |

Cycles through formulas use the graph's cycle code (`graph.md`). A formula reading upward is
`doc.upward_ref` (`documents.md` 5.1). A formula string in a key that takes none is `doc.type`, whose
hint says the key takes no formula (`documents.md` 7). Volume names before M5 use `doc.not_yet`. A produced name in a selector is
`identity.key_in_text` (`identity.md` 3). A measure under a feature with no samples is
`num.empty_reduction` (`numbers.md` 6).

## Conformance

Each case: input, command, what must hold. Every case runs through the command line (R20):
`via build WORLD`, reading the build's field files and `diagnostics.json`. A "tiny world" has an
extent of 2 by 2 posts. Windowed evaluation is tested by `via eval WORLD --field F --tile N --out
FILE` (`build.md` 14); it changes no byte. Field files hold 32-bit floats, so a
formula's double is compared after narrowing (`numbers.md` 8, rule 9).

Text phase:

1. `base = "20 + y"` → `formula.unmarked`, span of the value, exit 1. `base = " = 20"` → the same,
   hint to remove the space.
2. `base = 20` → 20 at every post.
3. `base = "="` → `formula.syntax`, "expected an expression".
4. `"= (x > 1) & (y > 1)"` in `where` → `formula.operator`, hint `and`. `"= x = 1"` → hint `==`.
   `"= 2 ** 3"`, `"= 7 % 2"`, `"= x > 1 ? 1 : 0"` → `formula.operator`. A selector in double
   quotes, `"= f(\"height\")"` → `formula.operator`, hint single quotes.
5. `"= Sin(x)"` → `formula.character`, hint lower case. `"= x $ 1"` → `formula.character`.
6. `"= 0 < x < 5"` → `formula.syntax`, hint `0 < x and x < 5`.
7. `.5`, `5.`, `1e`, `2e5x`, `1.e3`, `1_0`, `1e400`, `1e-400` as a base → `formula.syntax` each.
   `"= 1e-3"` → the bits of 0.001, narrowed.
8. A formula of 4097 bytes → `formula.too_long`. 65 nested parentheses → `formula.too_deep`; 64
   build.
9. A tiny world, `base = "= -2 * 3 + 4 / 2 - 1"` → -5 at every post.
10. A kind attribute `d` → `formula.shadow` naming the kind and attribute. A kind declaring `width`
    in `defaults` and in `let` → `formula.shadow`. A kind attribute `floor` read as
    `floor(floor)` → builds. `[let] height` beside `[field.height]` → `formula.shadow`, once, and no
    `doc` or `identity` code (`documents.md` case `dup_let_field`).
11. A world `let` `width = 1` and the library `lane` reading `width` → builds; the kind reads its
    own attribute, no message.
12. `d` in a field base → `formula.unknown_name`, saying `d` exists in a kind effect.
13. `clip(x, 0, 1)` → `formula.unknown_function`, hint `clamp`. `lerp(0, 1, 0.5)` → hint `mix`.
14. `fbm(60, 3, octaves = 4)`, `fbm(wavelength = 60, 3)`, `fbm(60, foo = 1)` →
    `formula.arguments` each.
15. `"= 1 and x > 0"` → `formula.type`. `if(x > 1, 1, x > 2)` → `formula.type`. An attribute
    `"= 1 > 0"` → `formula.type`. A formula reading a text attribute → `formula.type`.
16. An attribute `"= f('height')"` → `formula.rate` naming `f('height')`. `fbm(x, 3)` →
    `formula.rate` on `wavelength`. `max_change = "= f('height')"` → `formula.rate`.
17. `fbm(60, 3.5)`, `fbm(60, 13)`, `fbm(60, 3, gain = 0)`, `fbm(60, 3, lacunarity = 1)` →
    `formula.range` each.
18. `fbm(60, 3, seed = 2)` → `formula.seed`.
19. `dist('beck')` in a field base → `formula.cap`. In a kind effect with no `radius` on the effect
    or the kind → `formula.cap`.
20. `f('role:surface')` → `formula.selector`, hint `tag:`. `near('height', 5)`, naming a field →
    `formula.selector`. `dist('streets#e3', 10)` → `identity.key_in_text`, hint `'kind:<name>'`.
21. A rank 0 formula reading a rank 1 `let` → `doc.upward_ref`.

Selection phase:

22. `f('tag:ground')` with two ground fields → `formula.selector_count` naming both.
    `sdist('kind:lane', 20)` and `along('kind:lane')` with two lanes → `formula.selector_count`.
23. `dist('kind:pond', 50)` with no pond → one `formula.empty_selection` warning; value 50.
    `sdist` → -50; `near` → 0; `inside` → no.
24. `inside('lane_west')` → `formula.shape`. `sdist` over a pond and a lane → `formula.shape`.
    `smoothed_along(24)` in a kind of regions → `formula.shape`. `under('edge_min')` on a path →
    `formula.shape`.

Values:

25. `sd` in a pond's effect → positive inside. `sdist('pond', 20)` in a base equals that `sd` at
    every post within 20 m. `t` in an effect on a point feature → 0.
26. `inside('pond')` at a post on the outline → yes.
27. Two fields with `fbm(80, 3, seed = 'a')` → equal at every post. A field `trees` with
    `base = "= fbm(80, 3)"` and a scatter `trees` with the same density term → different values.
28. A base `"= fbm(80, 3) + fbm(40, 2)"` → `formula.seed_shared`. A base and a `then` row each
    with `fbm(80, 3)` → no warning, different patterns.
29. The stream vectors of 8.2 → `via vectors` gives them bit for bit; the SDK's own tests read the
    same file (not conformance).
30. `sin(180)`, `cos(90)`, `sin(360)` → 0; `sin(90)`, `sin(450)` → 1; `cos(180)`, `sin(-90)` → -1;
    `tan(90)` → `formula.not_finite` at `tan`.
31. `min(3, 1, 2)` → 1; `1 / min(0, -0)` → `formula.not_finite` at `/`, operand `-0`.
32. At resolution 1, a base `"= 0.5 * x"` and `slope('height')` → `atan(0.5) * (180 / pi)` at every post, edges
    included.
33. An effect with `field = "tag:ground"` matching two fields of bases 5 and 9, `value = "= v + 1"`
    → 6 and 10.
34. A kind that lowers `height`, with a feature `where = "= f('height') < 10"` → no cycle; the
    region follows the base. Adding a `then` row that raises the ground moves it.
35. `under('mean')` on a region with no post inside → `num.empty_reduction`, naming the feature
    (`numbers.md` 6, its case `empty_reduction`); no value. On a 2.3 m path at spacing 1 → the mean of samples at 0, 1, 2 and 2.3 m.
36. `smoothed_along(4)` on a 10 m path at spacing 1 over `base = "= x"` → the values of the
    vector table written with the code (M1).
37. (M2) A step calls `expr.eval` with `"= f('height')"` at a point 1 m beyond the extent →
    `formula.outside_field`; the call fails (`protocol.md` 3.4). In the branch of an `if` not taken
    there → no diagnostic.
38. A chain of 40 `let`s, each reading the one before twice → builds; the field equals the vector
    table written with the code, bit for bit (M1). That each `let` is evaluated once per post is a
    repository test of `via-expr`, not conformance (R20).
39. `lane_east.width = "= lane_west.width + 0.5"` → 4.0. Two attribute formulas reading each other
    → the graph's cycle report with the chain.
40. A density reading `f('height')` → at each candidate, the value of `fields.md` 1.2 (6.9).

Failure:

41. `"= sqrt(x - 100)"` in a base → `formula.not_finite` at `sqrt`, at the first post in post order
    with `x < 100`, its world position; no field written.
42. `"= if(x > 100, sqrt(x - 100), 0)"` → no diagnostic. `"= if(x > 50, if(x > 100, sqrt(x - 100), 0), 0)"`
    → no diagnostic.
43. `"= if(sqrt(x - 100) > 1, 1, 0)"` → `formula.not_finite` at `sqrt`. `"= 1 / exp(1000)"` → at
    `exp`. `"= x > 1 or sqrt(x - 100) > 0"` → at `sqrt`.
44. A density `"= 1.2"` → `formula.range`, hint `clamp`. A mask `"= 1.5"` → `formula.range`.
    `clamp(x, 1, 0)` and `smoothstep(1, 1, x)` → `formula.range`.
45. Case 41's field by `via eval` with `--tile 1` and `--tile 16`, and case 41's world by `via
    build` with `--threads 1` and `--threads 8` → each pair's `--diagnostics` documents equal, byte
    for byte.
46. `slope('height')` and `dist('beck', 30)` by `via eval` with `--tile 1` and `--tile 16` → equal
    `--out` files and `--diagnostics` documents, byte for byte.
47. A `let` holding `sqrt(-1)` read by two formulas → one location, the `let`'s, and the readers in
    `related`.

Whole and generated:

48. `ford` at `--threads 1` and `--threads 8`, on macOS and Linux → one digest. On Windows: pending
    until hosted runs are allowed (plan, "Comes back to you later").
49. (M2) A step calls `expr.eval` with `"= fbm(60, 3, seed = 'a')"` at a field's posts → the column,
    narrowed by `numbers.md` 8, rule 9, equals that field's bytes.

Owed by the limitation ledger (`docs/design/limitations.md`), M1:

50. (S11.K.4) A kind of paths with `[let] w2 = "= sqrt(width - 10)"`, a feature of that kind with
    `width = 3`, and an effect on the 2D field `height` reading `w2` → `formula.not_finite` at `sqrt`,
    located at the `let`'s span, naming the feature; exit 1; no `height` written. The effect is not
    skipped and no other value stands in.
51. (S11.K.11) `along('pond')` in a field base, the pond a square region → at a post on the outline,
    the outline fraction `t` of `numbers.md` 5.5 at that post; at a post off the outline, `t` of the
    nearest outline point. `along('well')`, a point feature → 0 at every post.
52. The noise vectors of 8.2 → the engine computes them, bit for bit.

**Workspace check** (repository test, not conformance): the reference page and the SDK function
list, generated from the function table, hold every row; the check fails when a row is removed from
the evaluator alone.

Change propagation for a formula attribute (`lane_west.width` changed, only its readers re-run) is
`graph.md`'s case.

## Changed from the spike

| Spike (SPEC 4, 12.6b, 12.6d, 12.6j) | v1 |
|---|---|
| Python `eval` over arrays; no grammar | Our grammar (section 3), checked before running (R6). |
| Bare strings, and `=` only for attributes | `"= expr"` everywhere. |
| `&`, `\|`, `~` for conditions | `and`, `or`, `not`; the old forms are errors with a hint. |
| `where(c, a, b)` | `if(c, a, b)`; `where` is a feature key only. |
| `[params]` names, which could shadow functions | `let` names in scopes; a name equal to a built-in is an error. |
| `min`, `max` of exactly two | Two or more. |
| `sin`, `cos` in radians; `radians`, `degrees` | `sin`, `cos`, `tan` in degrees, exact on the axes; the converters dropped. |
| `fbm(..., seed = 0)` by number | `seed = '<name>'`; by default a stream per owner and place. |
| `dist`, `sdist` unbounded | A cap, required or defaulted to a `radius`. |
| `sdist` negative inside a region | Positive inside, as `sd` and `offset` (`numbers.md` 5.5). |
| `along` on a region raised `expr.invalid` | Defined on regions; 0 on a point. |
| `base`, `base_min`, `base_max`, `under(f, how)` | `under(stat)` and `under(field, stat)`. |
| `profile` name with the `grade` attribute | `smoothed_along(length)`; `profile` is a library word. |
| `around()` in a density | `clump.around` only (`rules.md` 7.3): one mechanism. |
| `t = 0` for a region or point in an effect | Outline fraction for a region; 0 for a point. |
| `id.attribute` in attribute formulas only | In every formula, constant. |
| Fields read by name; tags as `#tag` | By name or `'tag:<tag>'`; never by role. |
| `check_where` | Dropped (`authority.md` 2.2). |
| `max_change` as a string naming an attribute | A per-feature formula. |
| A failure became 0 (2D) or `ground` (volume); a kind `let` failure skipped | Every failure is a located error; nothing is skipped. |
| `fbm3`, `fbm2`, `ground`, `steep`, `crest`, `tube`, `pd`, `pz` | M5 (open). |
| New | Types, rates and sharing; `inside`; `floor`; liveness for non-finite values; a formula at a point; size and depth limits; one table for evaluator, reference page and SDK. |

## Open

| Question | Decided by |
|---|---|
| A function reading one material's weight. | The first world that needs one: M1 `ford` or M2 `glade`. |
| Volume formulas: `z`, per-voxel rate, 3D noise, names for a column. | M5, with `type = "volume"`. |
| Formulas for positions (`at`, `path`) and facing. | M5, challenge 47's owed rotation. |
| `blend`'s names on an import. | `documents.md`, for M4. |
| Whether a per-post world `let` read by several fields is stored once as an unnamed field. Results are equal either way. | M1 build timings on `ford`. |
| Whether `smoothed_along` and `'edge_min'` stay in the table or move to a library step. | M1: the approach-grade lint on `ford`; M2's lake shores. |
| The function table's data file. | M0 step 5, with the code checks. |
