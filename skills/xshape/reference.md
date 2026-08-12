# xshape reference

Complete reference for the reshape verbs. The binary (`xshape <verb> --help`) and the
[README](https://github.com/excelano/xshape/blob/main/README.md) are authoritative; this
expands the edge cases and gives the tidyr/pandas/Miller translations. Any verb or flag
here that `xshape --help` doesn't list means the installed copy predates it.

## Invocation and flags

```sh
xshape <verb> [options] [FILE]      # FILE omitted, or `-`, → read stdin
```

Common flags, accepted by every verb:

| Flag | Effect |
|---|---|
| `-d, --delim <CHAR>` | field delimiter; default `,`, or tab for a `.tsv` file |
| `--no-header` | treat row 1 as data, not a header (names then unavailable; use letters) |
| `-i, --in-place[=SUFFIX]` | write the result back to FILE; `=.bak` keeps the original as `FILE.bak` |
| `-h, --help` / `-V, --version` | standard |

Output: the reshaped table to **stdout** (the preview), advisory notices to **stderr**.
`-i` requires a real FILE, however stdin was spelled. A UTF-8 BOM is stripped on read;
non-UTF-8 input draws an encoding warning with an `iconv` hint (via encsniff).

Omitting FILE and writing `-` mean the same thing, so a caller that would rather be
explicit can be. They differ in one place: a bare `xshape <verb>` at a terminal, with
nothing piped in, is a usage error rather than a silent wait, while an explicit `-` blocks
the way `cat -` does — it was asked for.

## Exit codes

`0` is success, including a reshape with nothing to do: no columns matched the address, one
row to transpose. The emitted table is the answer, so read it rather than the exit status.
`1` is bad input — an unreadable file, a malformed table, an address that does not resolve.
`2` is a bad invocation — an unknown flag, a missing argument, contradictory options.

## Column addressing

xshape's verbs are column-oriented. The grammar is the `xaddr` crate, which xled resolves
through as well, so an address means the same thing in both tools.

| Form | Resolves to | Notes |
|---|---|---|
| `C`, `AF` | one column by letter | bijective base-26: A=0, Z=25, AA=26; letters case-insensitive |
| `[first name]` | one column by header name | exact, **case-sensitive**; `]]` = a literal `]`, and a bare `[` inside a name is ordinary |
| `E:K`, `[a]:[d]` | an inclusive column range | either direction; expanded left→right |
| `B:`, `:C` | an open-ended range | runs to the first or last column of this table |
| `[id],D:E` | a comma list of atoms and ranges | **order preserved, duplicates kept** |

Single-column flags (`--col`, `--names-from`, `--values-from`) take exactly one atom and
reject a range or list. Set flags (`--cols`) take the full grammar. A `[name]` against a
file with no header errors and says so.

Two limits are xshape's own rather than the grammar's. An address naming a row (`3`, `$`) or
a single cell (`B2`) is rejected: those are xled's half of the dialect, and a reshape verb
takes columns. And a column past the table's width **errors** rather than clamping to the
last one — xled clamps, because reading less than you asked for is recoverable and a reshape
that quietly did less is not.

## The verbs

### `unpivot` — wide → long

```sh
xshape unpivot --cols <SPEC> [--into KEY,VALUE] [FILE]
```

Gathers the columns in `--cols` into two new columns: a **key** column holding each
gathered column's *name*, and a **value** column holding its cell. Every other column is an
identifier, repeated down the block. Output rows = input rows × gathered columns.

- `--into` names the two new columns; default `name,value`. Both parts required if given.
- Gathered columns come out in **spec order** (`--cols K,E` gathers K before E).
- **Requires a header** — the key values *are* the gathered column names. Under
  `--no-header` there are no names to gather into, so it errors (promote a header first
  with xled).
- A column named twice in `--cols` errors (it would duplicate the block).

### `pivot` — long → wide

```sh
xshape pivot --names-from <COL> --values-from <COL> [FILE]
```

The inverse of unpivot. Distinct values of `--names-from` become new column headers;
`--values-from` fills the grid; every other column identifies a row. New headers and rows
appear in **first-seen order**. A combination that never occurs is left **empty**, not
invented.

- **Never aggregates.** If two source rows share an identifier tuple *and* a name — they
  would land in one cell — pivot **errors** (`collision:`) and names xql/DuckDB. There is
  no `--on-collision` escape hatch: picking `first`/`last` would drop a cell, and summing
  is a query. Dedupe or aggregate upstream, then pivot the clean result.
- `--names-from` and `--values-from` must differ.

### `split` — one column → several

```sh
xshape split --col <COL> --sep <SEP> [--into A,B,C] [FILE]
```

Splits `--col` on the **literal** `--sep`; the pieces become new columns spliced in where
`--col` was, and the original column is removed.

- Output width = `--into`'s length if given, else the **widest split** across all rows.
  Rows with fewer pieces pad with `""`.
- New column names = `--into`, else `<col>_1`, `<col>_2`, … (`<col>` is the header name, or
  the letter if headerless).
- With a fixed `--into`, a row that splits into **more** pieces than there are names
  **errors** (naming the row) rather than discard the overflow — no value is dropped.
- Does not trim: `--sep ";"` on `"a; b"` yields `"a"` and `" b"`. Use `--sep "; "`, or
  clean afterward with xled.

### `explode` — a delimited cell → multiple rows

```sh
xshape explode --col <COL> --sep <SEP> [FILE]
```

Splits `--col` on the literal `--sep` and emits the row **once per piece**, every other
field repeated. The one verb that changes row count — but it invents nothing and applies no
predicate, so it is pure reshape (a cell holding `"a; b; c"` was always three values glued
together).

- A cell with no separator yields **one** row (itself); an empty cell yields **one** row
  with an empty value. Rows are never dropped, so output rows = Σ (pieces per row).
- Same literal-`--sep`, no-trim rule as `split`.

### `merge` — several columns → one

```sh
xshape merge --cols <SPEC> --sep <SEP> --into <NAME> [FILE]
```

Joins the values of `--cols` with the literal `--sep`, in **spec order**, into a single
column named `--into`, spliced in at the **leftmost** source column; the other source
columns are removed. The inverse of `split`.

- Needs at least two columns.
- Empty values join too: `["a", ""]` with `--sep ";"` → `"a;"`. (Skip-empty is a value
  decision — clean with xled if you need it.)
- `--cols [last],[first]` joins last then first; order follows the spec.

### `transpose` — swap the axes

```sh
xshape transpose [FILE]
```

Transposes the grid as a matrix, header row included. The **old first column becomes the
new header**, and the old header becomes the new first column. Ragged rows are padded to
the table width first. Under `--no-header` the whole grid transposes with no header overlay.

## Translation table — tidyr / pandas / Miller → xshape

The subcommand names are plain English, but the semantics are tidyr's. If you know one of
these, this is the mapping:

| Operation | tidyr | pandas | Miller (`mlr`) | xshape |
|---|---|---|---|---|
| wide → long | `pivot_longer(cols, names_to, values_to)` | `melt(id_vars, value_vars, var_name, value_name)` | `reshape --long -i … -o k,v` | `unpivot --cols … --into k,v` |
| long → wide | `pivot_wider(names_from, values_from)` | `pivot(index, columns, values)` | `reshape --wide -s k,v` | `pivot --names-from … --values-from …` |
| one col → many | `separate(col, into, sep)` | `str.split(sep, expand=True)` | `nest --explode --values --across-fields` | `split --col … --sep … [--into …]` |
| cell → rows | `separate_rows(col, sep)` | `assign(...).explode(col)` | `nest --explode --values --across-records` | `explode --col … --sep …` |
| many cols → one | `unite(into, cols, sep)` | `df[cols].agg(sep.join)` | `merge-fields` | `merge --cols … --sep … --into …` |
| swap axes | `t()` (base R) | `df.T` | `mlr … reshape`/`--transpose` | `transpose` |

Two differences from all of them: xshape **requires an explicit separator** (no default, no
inference), and its **pivot refuses to aggregate** — where pandas silently takes a mean on
duplicate index/column pairs, xshape errors.

## The three boundary rules (why xshape is trustworthy)

1. **Geometry only.** Every input cell reappears in the output unchanged. xshape never
   edits, casts, trims, or recases a value — that is xled. It only changes which axis holds
   the cell.
2. **Explicit separators.** `--sep` is required for `split`/`explode`/`merge` and is a
   literal string (spaces significant). Comma appears inside quoted values and slash inside
   dates, so there is no safe default to guess.
3. **No silent drop or combine.** `pivot` errors on a collision instead of aggregating;
   `split` errors rather than discard an overflow piece. Both errors mean the same thing:
   the next step is a query, and it belongs to xql/DuckDB.

## What xshape does not do

Filter rows by predicate, aggregate, group, join, dedupe, sort → **xql** / DuckDB. Edit
cell values (currency, case, padding, computed columns, blank-fill) → **xled**. Profile an
unfamiliar file → **xray**. Convert formats (`.xlsx`/JSON ↔ CSV) → office-convert / ditto.
An aggregating pivot (`pivot … SUM`) is a query, not a reshape — xshape errors on the
collision that would require it.
