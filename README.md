# xshape — reshape for tabular data

> **Status: parked design seed.** Not yet implemented. See [DESIGN.md](DESIGN.md).

xshape is the reshaping verb in a three-tool family for messy tabular data. It changes the *geometry* of a table — pivot, unpivot, transpose, split a column into several, merge several into one — without ever changing, filtering, or aggregating a value. It is the piece that neither of its siblings will grow into:

- **[xled](https://github.com/excelano/xled)** edits cell *values* in place, and never moves them.
- **xshape** changes the *shape* of the grid, and never touches the values.
- **[xql](https://github.com/excelano/xql)** queries the row *set* — filter, aggregate, group — and hands reshaping here.

The line that keeps it focused: xshape moves cells between the row and column axes, and errors rather than aggregate when two would collide. The moment a reshape needs a `SUM`, it has become a query — that belongs to xql or DuckDB.

xshape shares xled's column-addressing dialect (letters, `[bracketed names]`, A1 ranges) so the family reads as one language with different verbs.
