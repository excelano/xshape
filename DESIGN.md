# xshape — design

**Name:** `xshape` (x-family + reshape), which is also the crate name — unlike `xray`, no `x-`prefixed workaround was needed.

**One line:** header-aware structural reshaping for a single table — pivot, unpivot, transpose, split, merge, explode, reorder — the missing verb between xled (which edits values but never moves them) and xql (which queries the row set but never restructures the axes).

## The problem

xled and xql share a boundary that neither will cross, and the tools that live on the far side of it are all general-purpose reshapers borrowed from other ecosystems — Miller, tidyr, pandas `melt`/`pivot`. The recurring need is small and specific: a wide export has to go long before it can be queried; a "long" event log has to spread into a matrix before a human can read it; a single column holds three values glued by a delimiter and has to become three columns; the whole sheet arrived transposed. None of this changes what the data *says*. It only changes which axis holds what. That is a distinct job, and without xshape it has no home in the owned stack, so the reach is for a general tool with a hundred other features attached.

Built for one user's real need, not a market. The need is the shape-change that shows up mid-session when a table is correct but oriented wrong for the next step.

## The user

The primary user is Claude Code, working alongside David inside a live data session — the same organizing principle as xled and xql. The success metric is not elegance or adoption; it is whether this collapses the friction enough that Claude reaches for `xshape` instead of pulling in `mlr` or a pandas `melt`/`pivot` script the moment a reshape is needed. Two requirements follow, both inherited from xled and non-negotiable: an example-dense one-page reference that maps every subcommand to its tidyr/pandas/Miller analog so an LLM has working fluency on first contact, and a cheap preview of the reshaped grid before anything is written, because a structure change you cannot see before committing will be distrusted and routed around.

## The boundary — geometry, not content

This is the line that keeps xshape focused and keeps it from becoming a second, worse xql:

**xshape changes the *geometry* of the grid — which axis holds which cells — and never changes, filters, computes, or aggregates a value.** Every cell that goes in comes out somewhere else, unchanged. Nothing is invented, summarized, or dropped by predicate.

That single rule settles the tool's hardest temptation. A pivot in SQL or pandas usually carries an aggregation (`pivot … SUM(spend)`) because two source rows can collide into one output cell. **xshape's pivot does not aggregate.** It assumes the key/value pairs are unique and *errors loudly on a collision* rather than silently summing — because the moment a reshape needs an aggregation function, it has stopped being a reshape and become a query, and queries belong to xql or DuckDB. This is the same discipline as xled's "never coerce a value you didn't ask it to," applied one level up: never *combine* two values you didn't ask it to.

The split across the family:

| Tool  | Changes | Leaves alone |
|-------|---------|--------------|
| xled   | cell *values* (in place) | the grid's shape, the row set |
| **xshape** | the grid's *shape* (axes, splits, merges) | the values, the row set's membership |
| xql    | the row *set* (filter, aggregate, group) | — hands cell edits to xled, reshape to xshape |

## What it is

A small set of reshape verbs, each a subcommand, each named after the canonical operation an LLM already knows so the vocabulary transfers wholesale (the same unoriginality-as-adoption bet xled makes):

- **pivot** / spread — long → wide. A key column becomes new column headers; a value column fills the matrix. No aggregation; collisions are an error, not a sum.
- **unpivot** / gather / melt — wide → long. A named set of columns collapses into two columns, a key and a value. The inverse of pivot and the more common direction in practice (wide exports are everywhere).
- **transpose** — swap the row and column axes wholesale. The one-liner nobody remembers how to do safely with a header row.
- **split** — one column into several, on an explicit separator. xled explicitly punts this ("split one column into several → that's reshape"); this is where it lands.
- **merge** / unite — several columns into one, joined by a separator. The inverse of split.
- **explode** / unnest — a delimited cell into multiple rows, repeating the other fields. This *does* change the row count, which looks like it crosses into xql territory — but it adds no new information and applies no predicate, so it is pure reshape and belongs here.
- **reorder** — move columns to a position named relative to another column or an edge. A permutation: no column is added or dropped.

## Shared grammar

xshape speaks the **same addressing dialect as xled** — column letters (`C`, `AF`), bracketed header names (`[first name]`, `[price (USD)]`), and column ranges (`E:K`, `[fy2020]:[fy2026]`) — so a user (human or LLM) who has learned to point at a column in xled points at it identically here. Cross-family consistency is worth more than any per-tool cleverness; the family should feel like one dialect with different verbs, not tools that each reinvented addressing. Every verb acts on whole columns, so an address that names a row or a cell range (`B2:C3`) is rejected rather than projected onto its columns.

## Non-goals — who gets each near-miss

- Editing values inside cells (strip, recase, derive) → **xled**.
- Filter, aggregate, group, dedup → **xql**.
- Join across files → a SQL engine such as DuckDB (`duckdb`, `sqlite3`, `csvsql`, whichever is installed).
- Aggregating pivot (`pivot … SUM/COUNT`) → **xql**. xshape's pivot errors on the collision that would require it.
- Format conversion (`.xlsx`/JSON ↔ CSV) → **this2that**.
- Profiling / "what is this file" → **xray**.

## Addressing

Addressing is the shared `xaddr` crate, which xled uses too, so the two tools cannot drift apart on the dialect; `xaddr`'s conformance suite holds both to it.

## Separators are never guessed

`split`, `explode` and `merge` require an explicit `--sep`, with no inference: comma appears *inside* values and slash appears in dates and free text, so the two obvious defaults are unsafe. `split` names new columns `<name>_1`, `<name>_2`, overridable with `--into a,b,c`; `merge` takes a required `--into newname`.

## Collisions on pivot

`pivot` errors on a collision and names xql or a SQL engine for the dedup or aggregation. There is no `--on-collision first|last`, because dropping a cell breaks the rule that every input cell reappears unchanged.

## Preview and commit

The reshaped table goes to stdout by default (stdout *is* the preview) and `-i` / `--in-place[=.bak]` commits back to the file, sed-style, as in xled. There is no REPL: a reshape is a single decisive move, not the iterative cell-scrubbing that xled's REPL is for.
