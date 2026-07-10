# xshape — design

**Status:** In build, 2026-07-10. Design seed settled against the real corpus this session; decisions below are locked and Phase 1 scaffolding is underway. Name: `xshape` (x-family + reshape). crates.io name **`xshape` confirmed free** (404 on the sparse index for both `xshape` and `x-shape` — no `x-ray`-style workaround needed).

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

## Settled decisions — 2026-07-10, against the `~/xray-corpus` client tables

All six open questions are now closed. Where the corpus drove the call, the evidence is cited.

1. **Vocabulary lineage — SETTLED: plain-English verbs, tidyr semantics.** Subcommands are `unpivot`, `pivot`, `split`, `explode`, `merge`, `transpose`, carrying tidyr's mental model but not its names. Rejected tidyr's own `gather`/`spread` (deprecated) and pandas `melt`/`cast` (direction is ambiguous). The reference doc maps each verb to its tidyr/pandas/Miller analog; the CLI itself stays plain English because `unpivot` has the highest instant-recognition for an LLM.
2. **Collision policy on pivot — SETTLED: error-only, no escape hatch.** A `--on-collision first|last` would *drop* a cell, which breaks the core rule that every input cell reappears unchanged. On a collision `pivot` errors and the message names xql/DuckDB for the dedup or aggregation. Refusing keeps the boundary bright.
3. **Does `explode` belong? — SETTLED: yes, and it is core, not deferred.** The geometry test already passed it (no predicate, no new information, no aggregation); the corpus settles the priority. Your AI-analysis contract tables are full of `; `-delimited list cells (`vendor_aliases`, `related_contracts`, `application_names`, `integration_points`, `secondary_categories`) across ~20 files — one contract row wants to become one row per application. That pull is as strong as unpivot's, so `explode` ships in the core wave.
4. **Preview model — SETTLED: inherit xled exactly.** Reshaped table goes to **stdout by default** (stdout *is* the preview — you see the whole new grid), and `-i` / `--in-place[=.bak]` commits back to the file, sed-style, matching xled's `main.rs`. No new concepts; the family stays one dialect.
5. **REPL or runner-only? — SETTLED: runner-only for v1.** Reshape is a single decisive move, not the iterative cell-scrubbing that earns xled its REPL. Park the REPL until real usage asks for it.
6. **Header handling & the separator — SETTLED, and the separator is a "never guess" call.** `split` names new columns `<name>_1`, `<name>_2`, overridable with `--into a,b,c` (tidyr's `into=`); `merge` takes a required `--into newname`. Crucially, `split`/`explode`/`merge` require an **explicit `--sep` with no inference**, because the corpus proves the two obvious defaults are unsafe: comma appears *inside* values (`Sirius Computer Solutions, LLC`) and slash appears in dates and free text (`06/05/23`, `S010222/S011253/S060120`). The dominant real delimiter is semicolon-space, but the tool never assumes it — same "never coerce a value you didn't ask it to" discipline as xled, one level up.

## Corpus-ranked build order

pivot is the theoretical inverse and defines the collision boundary, but the corpus does not pull for it (the `metric`/`category` header hits were false positives — classification columns, not long key/value tables). The data wants unpivot and explode first.

| Rank | Verb | Corpus evidence |
|------|------|-----------------|
| 1 | **unpivot** | `fy_spending_summary`, `fy_spending_detail`, `coverage_detail` — `contract_id` + `fy2020…fy2026` spread; the xql query-blocker |
| 2 | **explode** | ~20 files with `; `-delimited list cells (the AI-analysis output shape) |
| 3 | **split** | same list cells when the target is columns, not rows; feeds the others (merged `S010.../S011...` keys) |
| 4 | **pivot** | no corpus pull; built for the boundary rule and inverse symmetry |
| 5 | **merge / transpose** | speculative — `full_contract_string` vs its parts hints merge round-trips exist; transpose is the rare "arrived sideways" case |

## Addressing strategy — SETTLED: copy-now, extract-later (option c)

xshape must speak xled's addressing dialect (column letters, `[bracketed names]`, A1 ranges), which today lives baked into xled's `resolver.rs`/`parser.rs` (~1,180 lines), not a shared crate. Rather than refactor a shipped, published tool to bootstrap an unbuilt one, vendor xled's column/range parsing into xshape verbatim now (with its tests), get xshape proven, then extract a shared `xaddr` crate in a dedicated low-risk refactor once both tools are stable. This mirrors how `encsniff` became a shared lib dep — extracted after the fact, not up front — and still honors "share the spec from day one," because xled's parser *is* the spec, copied faithfully.

## First move

Scaffold Phase 1 from xray's template (Cargo layout, clap-derive CLI shell, `-V`/`-h` standard, csv + encsniff + anstream, the `-i` commit model, and the `install.sh`/`uninstall.sh`/`RELEASING.md`/`SECURITY.md`/`dist-workspace.toml` release plumbing), then vendor xled's addressing, then implement verbs in the corpus-ranked order above.
