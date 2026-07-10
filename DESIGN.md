# xshape — design

**Status:** Design seed, 2026-07-10. **Parked** — captured while the idea is fresh; the active session is carrying [xray](https://github.com/excelano/xray) forward instead. Name: `xshape` (x-family + reshape). crates.io availability **not yet verified**.

**One line:** header-aware structural reshaping for a single table — pivot, unpivot, transpose, split, merge, explode — the missing verb between xled (which edits values but never moves them) and xql (which queries the row set but never restructures the axes).

## The problem

xled and xql share a boundary that neither will cross, and the tools that live on the far side of it are all general-purpose reshapers borrowed from other ecosystems — Miller, tidyr, pandas `melt`/`pivot`. The recurring need is small and specific: a wide export has to go long before it can be queried; a "long" event log has to spread into a matrix before a human can read it; a single column holds three values glued by a delimiter and has to become three columns; the whole sheet arrived transposed. None of this changes what the data *says*. It only changes which axis holds what. That is a distinct job, and right now it has no home in the owned stack, so the reach is always for a general tool with a hundred other features attached.

Built for one user's real need, not a market. The need is the shape-change that shows up mid-session when a table is correct but oriented wrong for the next step.

## The user

The primary user is Claude Code, working alongside David inside a live data session — the same organizing principle as xled and xql. The success metric is not elegance or adoption; it is whether this collapses the friction enough that Claude reaches for `xshape` instead of pulling in `mlr` or a pandas `melt`/`pivot` script the moment a reshape is needed. Two requirements follow, both inherited from xled and non-negotiable: an example-dense one-page reference that maps every subcommand to its tidyr/pandas/Miller analog so an LLM has working fluency on first contact, and a cheap preview of the reshaped grid before anything is written, because a structure change you cannot see before committing will be distrusted and routed around.

## The boundary — geometry, not content

This is the line that keeps xshape focused and keeps it from becoming a second, worse xql. State it precisely:

**xshape changes the *geometry* of the grid — which axis holds which cells — and never changes, filters, computes, or aggregates a value.** Every cell that goes in comes out somewhere else, unchanged. Nothing is invented, summarised, or dropped by predicate.

That single rule settles the tool's hardest temptation. A pivot in SQL or pandas usually carries an aggregation (`pivot … SUM(spend)`) because two source rows can collide into one output cell. **xshape's pivot does not aggregate.** It assumes the key/value pairs are unique and *errors loudly on a collision* rather than silently summing — because the moment a reshape needs an aggregation function, it has stopped being a reshape and become a query, and queries belong to xql or DuckDB. This is the same discipline as xled's "never coerce a value you didn't ask it to," applied one level up: never *combine* two values you didn't ask it to.

The clean three-way split across the family:

| Tool  | Changes | Leaves alone |
|-------|---------|--------------|
| xled   | cell *values* (in place) | the grid's shape, the row set |
| **xshape** | the grid's *shape* (axes, splits, merges) | the values, the row set's membership |
| xql    | the row *set* (filter, aggregate, group) | — hands cell edits to xled, reshape to xshape |

## What it is

A small set of reshape verbs, each a subcommand, each named after the canonical operation an LLM already knows so the vocabulary transfers wholesale (the same unoriginality-as-adoption bet xled makes). Candidate set:

- **pivot** / spread — long → wide. A key column becomes new column headers; a value column fills the matrix. No aggregation; collisions are an error, not a sum.
- **unpivot** / gather / melt — wide → long. A named set of columns collapses into two columns, a key and a value. The inverse of pivot and the more common direction in practice (wide exports are everywhere).
- **transpose** — swap the row and column axes wholesale. The one-liner nobody remembers how to do safely with a header row.
- **split** — one column into several, by delimiter, regex, or fixed width. xled explicitly punts this ("split one column into several → that's reshape"); this is where it lands.
- **merge** / unite — several columns into one, joined by a separator. The inverse of split.
- **explode** / unnest — a delimited cell into multiple rows, repeating the other fields. This *does* change the row count, which looks like it crosses into xql territory — but it adds no new information and applies no predicate, so it is pure reshape and belongs here. (Flag for review; see open questions.)

## Shared grammar

xshape must speak the **same addressing dialect as xled** — column letters (`C`, `AF`), bracketed header names (`[first name]`, `[price (USD)]`), and A1 ranges (`B2:C3`) — so a user (human or LLM) who has learned to point at a column in xled points at it identically here. Cross-family consistency is worth more than any per-tool cleverness; the family should feel like one dialect with different verbs, not three tools that each reinvented addressing. This is a hard requirement, not a nicety, and it is the strongest reason to share code or at least a spec with xled from day one.

## Non-goals — who gets each near-miss

- Editing values inside cells (strip, recase, derive) → **xled**.
- Filter, aggregate, group, dedup, join → **xql** (single-table) / **DuckDB** (relational).
- Aggregating pivot (`pivot … SUM/COUNT`) → **DuckDB**. xshape's pivot errors on the collision that would require it.
- Format conversion (`.xlsx`/JSON ↔ CSV) → office-convert / ditto. (This was a third candidate tool, deliberately *not* built — the conversion surface is already half-owned.)
- Profiling / "what is this file" → **xray**.

## Open questions — settle against the real corpus next session

1. **Vocabulary lineage.** Borrow tidyr (`pivot_longer`/`pivot_wider`), pandas (`melt`/`pivot`), or Miller (`reshape`/`nest`) verbatim for the subcommand names? Pick the one lineage with the highest instant-recognition for an LLM and stay faithful to it; do not blend. Leaning tidyr's gather/spread mental model with plain-English subcommand names (`pivot`/`unpivot`).
2. **Collision policy on pivot.** Hard error is the default per the boundary rule. Offer an explicit `--on-collision first|last|error` escape hatch, or refuse entirely and make the user dedup upstream with xql? Refusing is more honest and keeps the boundary bright.
3. **Does `explode` belong?** It changes row count. It is pure reshape by the geometry test, but it is the one verb that could be argued into xql. Decide deliberately.
4. **Preview model.** Inherit xled's preview-before-commit wholesale. Reshape is more structurally destructive than a cell edit, so the preview matters *more*, not less — show the new header row and a few rows of the new grid.
5. **REPL or runner-only?** xled earns its REPL from iterative cell-scrubbing. Reshape is more often a single decisive move; runner-only may be the honest scope. Park until there's usage.
6. **Header-row handling in split/merge.** When a column splits, what do the new headers become (`col_1`/`col_2`, a supplied list, a regex capture-group name)? When columns merge, which header survives?

## First move next session

Settle the vocabulary lineage (open question 1) against the real corpus in `~/xled-corpus`, because every subcommand name and its flags depend on it, and lock the pivot collision policy (2) since it is the decision that defines the tool's boundary. Everything else is downstream of those two.
