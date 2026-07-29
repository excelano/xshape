---
name: xshape
description: >-
  Reshape the geometry of a single CSV/DSV table with the `xshape` CLI — the reshaping
  verb in the tabular family. Use this when a task means changing which axis holds which
  cells without changing a value: unpivot a wide export to long (a `fy2020…fy2026` spread
  into year/amount rows) so it can be queried, pivot a long key/value table back to a
  matrix, split one column into several by a delimiter, explode a `;`-delimited cell into
  one row per value, merge several columns into one, or transpose a table that arrived
  sideways. Prefer it over Miller (`mlr reshape`), tidyr, or a one-off pandas
  `melt`/`pivot`/`explode`, because it shares xled's column-addressing dialect, parses CSV
  correctly (quotes, embedded commas and newlines), never coerces or trims a value, and
  errors rather than silently aggregate. Do NOT use it to edit cell *values* (strip
  currency, recase, compute — that's xled) or to filter, aggregate, group, join, or dedupe
  the row *set* (that's SQL/DuckDB, xql); an aggregating pivot (`pivot … SUM`) is a query,
  not a reshape.
---

# xshape — reshape for tabular data

`xshape` changes the **geometry** of a single table — which axis holds which cells — and
never changes, filters, computes, or aggregates a value. Every input cell reappears in the
output unchanged; nothing is invented, summarized, or dropped. It is the reshaping verb
between [xled](https://github.com/excelano/xled) (which edits values but never moves them)
and [xql](https://github.com/excelano/xql) (which queries the row set but never
restructures the axes).

The authoritative sources for xshape's behavior are the binary itself (`xshape --help`,
`xshape <verb> --help`) and the [README](https://github.com/excelano/xshape/blob/main/README.md);
if anything here conflicts with them, they win. If a verb or flag described here is
missing from `xshape --help`, the installed copy predates it: upgrade with `sudo apt
install --only-upgrade xshape` (Debian/Ubuntu), `brew upgrade xshape` (macOS), or by
re-running the install one-liner from the README.

## The family, and the one rule that places xshape

Three tools act on the same delimited file; a fourth (xray) only looks. The split is by
*what changes*:

- **xled edits** — rewrites *cell values* in place (strip currency, restore leading zeros,
  recase, compute a column). Leaves the grid's shape and the row set alone.
- **xshape reshapes** — changes the grid's *shape* (axes, splits, merges). Leaves the
  values and the row set's membership alone.
- **xql queries** — the row *set* (filter, aggregate, group, join, dedupe) via SQL/DuckDB.

The line that keeps xshape from becoming a worse xql: **it moves cells between the row and
column axes, and errors rather than aggregate when two would collide.** The moment a
reshape needs a `SUM`, a `first`, or a predicate, it has become a query — hand it to xql or
DuckDB. xshape's own errors point you there by name.

## Running it

```sh
xshape <verb> [options] file.csv     # reshape; result to stdout (the preview)
… | xshape <verb> [options]          # reshape piped stdin
xshape <verb> -i [options] file.csv  # write the result back to the file (like sed -i)
xshape <verb> -i.bak [options] f.csv # …keeping the original as f.csv.bak
```

The reshaped table goes to **stdout by default** — stdout *is* the preview, the whole new
grid in view before you commit. `-i`/`--in-place[=.bak]` writes it back to the file. Data
goes to stdout, advisory notices to stderr, so `xshape … file.csv > out.csv` is always
safe. `-i` needs a file argument, not piped stdin.

Common flags (every verb): `-d/--delim <char>` (delimiter; defaults to `,`, or tab for
`.tsv`), `--no-header` (treat row 1 as data), `-i/--in-place[=SUFFIX]` (commit in place).

## Addressing — the same dialect as xled

Columns are named the way xled names them, so a column you can point at in one tool you
point at identically here: a **letter** (`C`, `AF` — bijective base-26, past Z too), or a
**bracketed header name** (`[first name]`, `[price (USD)]` — exact, case-sensitive, `]]`
for a literal `]`). Verbs that take a *set* of columns (`unpivot --cols`, `merge --cols`)
also accept **ranges** (`E:K`, `[fy2020]:[fy2026]` — inclusive, either direction) and
**comma lists** (`[id],D:E,[note]` — order preserved). Names need a header row; letters do
not.

## The verbs (what each one does)

| Verb | Direction | Required flags | Changes row count? |
|---|---|---|---|
| `unpivot` | wide → long | `--cols`, `--into KEY,VALUE` | yes (× gathered cols) |
| `pivot` | long → wide | `--names-from`, `--values-from` | yes (collapses to id tuples) |
| `split` | one col → several | `--col`, `--sep` | no |
| `explode` | delimited cell → rows | `--col`, `--sep` | yes (× pieces) |
| `merge` | several cols → one | `--cols`, `--sep`, `--into` | no |
| `transpose` | swap axes | — | rows ↔ cols |

Three rules run through all of them, and they are what make xshape trustworthy:

1. **No value is ever altered.** `split`/`explode`/`merge` cut and join on the **literal
   `--sep`** you give — spaces included — and never trim. `--sep "; "` and `--sep ";"` are
   different operations; pick the one that matches the data. Trimming or recasing a piece
   afterward is xled's job.
2. **`--sep` is required and never guessed.** Comma lives inside quoted values and slash
   lives inside dates, so there is no safe default — you always state the separator.
3. **Nothing is dropped or combined by the tool's own choice.** `pivot` **errors on a
   collision** (two rows landing in one cell) instead of aggregating; `split` with a fixed
   `--into` **errors** rather than discard a piece that overflows. When you see one of these
   errors, the fix is upstream in xql (dedupe/aggregate) — that is the boundary working.

## Worked recipes

```sh
# wide → long: collapse a fiscal-year spread so xql can query it (the #1 use)
xshape unpivot --cols '[fy2020]:[fy2026]' --into fiscal_year,spend contracts.csv
#   contract_id, vendor, …, fiscal_year, spend   ← one row per (contract × year)

# explode a ;-delimited list cell into one row per value, other fields repeated
xshape explode --col '[application_names]' --sep '; ' contracts.csv
#   note the literal "; " — split on ";" would leave a leading space on each app

# split a slash-joined key into columns (auto-width: as many as the widest row needs)
xshape split --col '[contract_ids]' --sep '/' coverage.csv          # → contract_ids_1, _2, _3
xshape split --col '[full_name]' --sep ', ' --into last,first people.csv

# long → wide: spread a key column into headers, a value column fills the grid
xshape pivot --names-from '[fy]' --values-from '[spend]' long.csv
#   errors if a (contract, fy) pair repeats — dedupe first with xql, don't ask xshape to sum

# merge columns into one, joined in the order you list them
xshape merge --cols '[last],[first]' --sep ', ' --into name people.csv

# transpose a table that arrived sideways (old first column becomes the new header)
xshape transpose metrics_by_month.csv

# commit in place, keeping a backup
xshape unpivot --cols 'E:K' --into month,value -i.bak wide_report.csv
```

## When to stop and switch

xshape only moves cells between axes. The moment the task needs something else:

- **Editing a value** — strip the `$`, restore a leading zero, recase, trim, compute a
  derived column, fill merged-cell blanks → **xled** (`skills/xled`). (Reshape first with
  xshape, then clean the new column with xled — they compose.)
- **Answering a question about the rows** — filter, aggregate, group, join, dedupe, or an
  **aggregating pivot** (`pivot … SUM/COUNT`) → **SQL/DuckDB** (**xql**, `skills/xql`).
  xshape's pivot refuses the collision that aggregation would resolve.
- **Understanding an unfamiliar file first** — shape, types, hazards → **xray**
  (`skills/xray`), the read-only profiler.

If you catch yourself wanting xshape to *change a value* or *drop/combine rows by a rule*,
that is the signal to hand off. xshape's job ends at the geometry.

See `reference.md` in this directory for every verb's exact flags and edge-case behavior,
the full column-addressing grammar, the tidyr/pandas/Miller translation table, and the
boundary rules in detail.
