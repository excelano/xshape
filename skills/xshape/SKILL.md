---
name: xshape
description: >-
  Reshape a single CSV/TSV table with `xshape` without changing any value: unpivot wide to
  long, pivot long to wide, split or merge columns, explode a delimited cell into rows,
  reorder, transpose. Often needed without being named: "one column per year", "months
  across the top", "can't group by year", "three tags in one cell", "one row per value".
  Not for editing values (xled) or filtering, aggregating, joining or deduping rows (xql).
---

# xshape

`xshape` changes which axis holds which cells and never changes, filters, computes or
aggregates a value. Every input cell reappears in the output unchanged. Unlike pandas or
DuckDB it needs no flag to keep values as strings, so leading zeros, long IDs and blank
cells survive. `xshape <verb> --help` is authoritative; `reference.md` in this directory
has every verb's flags and edge cases and a tidyr/pandas/Miller translation table, and is
read only when needed.

## Running it

```sh
xshape <verb> [options] file.csv      # result to stdout: this is the preview
xshape <verb> -i.bak [options] f.csv  # write back in place, keeping f.csv.bak
```

Check the stdout preview before using `-i`. Exit 0 includes a reshape with nothing to do,
so read the output, not the status; 1 is bad input or a refusal, 2 a bad invocation.

## Addressing

Columns are a letter (`C`, `AF`) or a bracketed header name (`[first name]`, exact and
case-sensitive). Set flags (`--cols`) also take ranges (`E:K`, `[fy2020]:[fy2026]`),
open ranges (`B:`) and comma lists (`[id],D:E`). A header that is a number must be
bracketed: `--cols 2023:2025` is read as rows and rejected, so write
`--cols '[2023]:[2025]'`.

## Verbs

| Verb | Required flags |
|---|---|
| `unpivot` | `--cols`, `--into KEY,VALUE` |
| `pivot` | `--names-from`, `--values-from` |
| `split` | `--col`, `--sep` (optional `--into a,b,c`) |
| `explode` | `--col`, `--sep` |
| `merge` | `--cols`, `--sep`, `--into` |
| `reorder` | `--cols`, one of `--before`/`--after`/`--first`/`--last` |
| `transpose` | none |

`--sep` is required, literal and never trimmed: `"; "` and `";"` are different
separators, so match the data.

## Refusals are the boundary

`pivot` exits 1 when two rows would land in one cell, and `split` with a fixed `--into`
exits 1 rather than drop an overflow piece. Neither has an override. The fix is upstream:
dedupe or aggregate with xql, then reshape the clean result. An aggregating pivot
(`pivot … SUM`) is a query, not a reshape.

## Handoffs

Editing values (strip `$`, restore leading zeros, trim, recase, compute or create a
column) goes to xled; reshape first, then clean. A new column is created with xled, which
appends it, and placed with `xshape reorder`. Filtering, grouping, aggregating and
deduping one table go to xql. Joins across files need a SQL engine: use whichever of
`duckdb`, `sqlite3` or `csvsql` is installed. Profile an unfamiliar file with xray first.
