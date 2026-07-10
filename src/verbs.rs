//! The reshape verbs. Each takes a `&Table` and returns a new `Table` — geometry changes only,
//! every input cell reappearing unchanged (see DESIGN.md, "Corpus-ranked build order":
//! unpivot → explode → split → pivot → merge/transpose).
//!
//! Two rules run through all of them. Values are never altered: `explode`/`split`/`merge` split
//! and join on the *literal* `--sep` the caller gives (spaces included) and never trim, because
//! trimming would change a value — that is xled's job. And nothing is ever combined or dropped by
//! the tool's own choice: `pivot` errors on a collision rather than aggregate, and `split` errors
//! rather than discard a piece that overflows a fixed `--into`.

use crate::addr;
use crate::errors::{Result, XshapeError};
use crate::model::{col_to_letter, Table};

/// The dense row `r`: every column 0..ncols as an owned string, ragged cells padded to "".
/// Reshape output is rectangular, so each verb materializes its rows through this.
fn dense_row(t: &Table, r: usize) -> Vec<String> {
    (0..t.ncols()).map(|c| t.cell(r, c).to_string()).collect()
}

/// The header labels for every column (name, or letter where there is no name).
fn labels(t: &Table) -> Vec<String> {
    (0..t.ncols()).map(|c| t.col_label(c)).collect()
}

/// wide → long. The columns in `cols_spec` are gathered into two new columns: a key column
/// holding each gathered column's name, and a value column holding its cell. Every other column
/// is an identifier, repeated down the block. No value is changed, summarized, or dropped.
///
/// Requires a header — the key values *are* the gathered column names. Under `--no-header` there
/// are no names to gather into, so it errors rather than invent letter keys.
pub fn unpivot(table: &Table, cols_spec: &str, key_name: &str, value_name: &str) -> Result<Table> {
    if table.header.is_none() {
        return Err(XshapeError::Input(
            "unpivot needs a header row: the gathered column names become the key values \
             (drop --no-header, or promote the header first with xled)"
                .into(),
        ));
    }
    let ncols = table.ncols();
    let gather = addr::cols(cols_spec, table)?;

    // A column gathered twice is almost certainly a mistake, and it would duplicate the block.
    let mut seen = std::collections::HashSet::new();
    for &c in &gather {
        if c >= ncols {
            return Err(XshapeError::Address(format!(
                "column {} is beyond the table's {ncols} columns",
                crate::model::col_to_letter(c)
            )));
        }
        if !seen.insert(c) {
            return Err(XshapeError::Address(format!(
                "column {} appears twice in --cols; each gathered column must be distinct",
                crate::model::col_to_letter(c)
            )));
        }
    }

    // Identifier columns: everything not gathered, in original left-to-right order.
    let id_cols: Vec<usize> = (0..ncols).filter(|c| !seen.contains(c)).collect();

    // New header: id labels, then the two synthesized names.
    let mut header: Vec<String> = id_cols.iter().map(|&c| table.col_label(c)).collect();
    header.push(key_name.to_string());
    header.push(value_name.to_string());

    // One output row per (input row × gathered column).
    let mut rows = Vec::with_capacity(table.nrows() * gather.len().max(1));
    for r in 0..table.nrows() {
        for &g in &gather {
            let mut row: Vec<String> = id_cols.iter().map(|&c| table.cell(r, c).to_string()).collect();
            row.push(table.col_label(g));
            row.push(table.cell(r, g).to_string());
            rows.push(row);
        }
    }

    Ok(Table { header: Some(header), rows, delim: table.delim })
}

/// A delimited cell → multiple rows. Each input row's `col` value is split on the literal `sep`;
/// the row is emitted once per piece, every other column repeated. This is the one verb that
/// changes the row count, but it invents nothing and applies no predicate — a cell holding
/// `"a; b; c"` was always three values glued together, so it is pure reshape (DESIGN.md).
///
/// A cell with no separator yields one row (itself, unchanged); an empty cell yields one row with
/// an empty value — the row is never dropped.
pub fn explode(table: &Table, col_spec: &str, sep: &str) -> Result<Table> {
    require_sep(sep)?;
    let col = addr::one_col(col_spec, table)?;
    in_range(col, table.ncols())?;

    let mut rows = Vec::with_capacity(table.nrows());
    for r in 0..table.nrows() {
        let base = dense_row(table, r);
        for piece in base[col].split(sep) {
            let mut row = base.clone();
            row[col] = piece.to_string();
            rows.push(row);
        }
    }
    Ok(Table { header: table.header.clone(), rows, delim: table.delim })
}

/// One column → several. Each `col` value is split on the literal `sep`; the pieces become new
/// columns spliced in where `col` was, and `col` itself is removed. Widths vary row to row, so
/// the output column count is `--into`'s length when given, else the widest split seen. Rows with
/// fewer pieces pad with "". If a fixed `--into` is too narrow for some row, xshape errors rather
/// than silently drop the overflow — a discarded piece would break the no-value-lost rule.
pub fn split(table: &Table, col_spec: &str, sep: &str, into: Option<&[String]>) -> Result<Table> {
    require_sep(sep)?;
    let col = addr::one_col(col_spec, table)?;
    in_range(col, table.ncols())?;

    // Split every row once; keep the pieces so width and overflow are decided from real data.
    let pieces: Vec<Vec<String>> = (0..table.nrows())
        .map(|r| table.cell(r, col).split(sep).map(str::to_string).collect())
        .collect();

    let width = match into {
        Some(names) => {
            if let Some((r, got)) = pieces.iter().enumerate().find(|(_, p)| p.len() > names.len()) {
                return Err(XshapeError::Input(format!(
                    "row {} splits into {} pieces but --into names only {} columns — add names, \
                     or the extra pieces would be dropped (xshape never discards a value)",
                    r + 2, // +1 for 0-based, +1 for the header row
                    got.len(),
                    names.len()
                )));
            }
            names.len()
        }
        None => pieces.iter().map(Vec::len).max().unwrap_or(1),
    };

    let new_names: Vec<String> = match into {
        Some(names) => names.to_vec(),
        None => {
            let base = table.col_label(col);
            (1..=width).map(|i| format!("{base}_{i}")).collect()
        }
    };

    let header = table.header.as_ref().map(|_| splice(&labels(table), col, &new_names));
    let mut rows = Vec::with_capacity(table.nrows());
    for (r, row_pieces) in pieces.iter().enumerate() {
        let mut cells = row_pieces.clone();
        cells.resize(width, String::new()); // pad short rows; never truncate (checked above)
        rows.push(splice(&dense_row(table, r), col, &cells));
    }
    Ok(Table { header, rows, delim: table.delim })
}

/// long → wide. The distinct values of `names_from` become new column headers; `values_from`
/// fills the grid; every other column identifies a row. xshape's pivot **never aggregates**: if
/// two source rows share an identifier tuple and a name — they would land in one cell — it errors
/// and names xql/DuckDB, because resolving that collision is a query, not a reshape (DESIGN.md).
pub fn pivot(table: &Table, names_from: &str, values_from: &str) -> Result<Table> {
    let name_col = addr::one_col(names_from, table)?;
    let val_col = addr::one_col(values_from, table)?;
    in_range(name_col, table.ncols())?;
    in_range(val_col, table.ncols())?;
    if name_col == val_col {
        return Err(XshapeError::Input(
            "--names-from and --values-from must be different columns".into(),
        ));
    }

    let id_cols: Vec<usize> = (0..table.ncols()).filter(|&c| c != name_col && c != val_col).collect();

    use std::collections::HashMap;
    let mut names: Vec<String> = Vec::new(); // spread headers, first-seen order
    let mut name_idx: HashMap<String, usize> = HashMap::new();
    let mut id_tuples: Vec<Vec<String>> = Vec::new(); // output rows' id part, first-seen order
    let mut id_idx: HashMap<Vec<String>, usize> = HashMap::new();
    let mut grid: HashMap<(usize, usize), String> = HashMap::new();

    for r in 0..table.nrows() {
        let name = table.cell(r, name_col).to_string();
        let ni = *name_idx.entry(name.clone()).or_insert_with(|| {
            names.push(name.clone());
            names.len() - 1
        });
        let ids: Vec<String> = id_cols.iter().map(|&c| table.cell(r, c).to_string()).collect();
        let ii = *id_idx.entry(ids.clone()).or_insert_with(|| {
            id_tuples.push(ids.clone());
            id_tuples.len() - 1
        });
        if grid.insert((ii, ni), table.cell(r, val_col).to_string()).is_some() {
            let label = table.col_label(name_col);
            let where_ = if ids.is_empty() { String::new() } else { format!(" for row [{}]", ids.join(", ")) };
            return Err(XshapeError::Collision(format!(
                "two source rows map to the same cell — {label}={name:?}{where_} appears twice. \
                 xshape does not aggregate; dedup or aggregate upstream with xql (GROUP BY) or DuckDB"
            )));
        }
    }

    let mut header: Vec<String> = id_cols.iter().map(|&c| table.col_label(c)).collect();
    header.extend(names.iter().cloned());

    let mut rows = Vec::with_capacity(id_tuples.len());
    for (ii, ids) in id_tuples.iter().enumerate() {
        let mut row = ids.clone();
        for ni in 0..names.len() {
            row.push(grid.get(&(ii, ni)).cloned().unwrap_or_default());
        }
        rows.push(row);
    }
    Ok(Table { header: Some(header), rows, delim: table.delim })
}

/// Several columns → one, joined by the literal `sep` in spec order. The merged column takes the
/// place of the leftmost source column; the others are removed. Empty values join too (nothing is
/// skipped), so `["a", ""]` with sep `;` becomes `"a;"` — a value change would be xled's job.
pub fn merge(table: &Table, cols_spec: &str, sep: &str, into: &str) -> Result<Table> {
    let cols = addr::cols(cols_spec, table)?;
    for &c in &cols {
        in_range(c, table.ncols())?;
    }
    if cols.len() < 2 {
        return Err(XshapeError::Input("merge needs at least two columns".into()));
    }
    let remove: std::collections::HashSet<usize> = cols.iter().copied().collect();
    let anchor = *cols.iter().min().unwrap();

    let build = |cells: &[String]| -> Vec<String> {
        let joined = cols.iter().map(|&c| cells[c].clone()).collect::<Vec<_>>().join(sep);
        let mut out = Vec::with_capacity(cells.len() - remove.len() + 1);
        for (c, cell) in cells.iter().enumerate() {
            if c == anchor {
                out.push(joined.clone());
            } else if !remove.contains(&c) {
                out.push(cell.clone());
            }
        }
        out
    };

    // The merged column's header is just `into` — not a join of the old labels (those describe
    // the parts, not the result). Splice `into` in at the anchor, drop the other source columns.
    let header = table.header.as_ref().map(|_| {
        labels(table)
            .into_iter()
            .enumerate()
            .filter_map(|(c, name)| {
                if c == anchor {
                    Some(into.to_string())
                } else if remove.contains(&c) {
                    None
                } else {
                    Some(name)
                }
            })
            .collect()
    });
    let rows: Vec<Vec<String>> = (0..table.nrows()).map(|r| build(&dense_row(table, r))).collect();
    Ok(Table { header, rows, delim: table.delim })
}

/// Swap the row and column axes wholesale. The grid — header row included, when present — is
/// transposed as a matrix, so the old first column becomes the new header and the old header
/// becomes the new first column. The one-liner nobody remembers to do safely with a header.
pub fn transpose(table: &Table) -> Result<Table> {
    let ncols = table.ncols();
    let has_header = table.header.is_some();

    // The full logical grid: header (if any) as row 0, then data rows, each padded to ncols.
    let mut grid: Vec<Vec<String>> = Vec::new();
    if let Some(h) = &table.header {
        let mut row = h.clone();
        row.resize(ncols, String::new());
        grid.push(row);
    }
    for r in 0..table.nrows() {
        grid.push(dense_row(table, r));
    }

    // Transpose: new[c][r] = grid[r][c].
    let mut t: Vec<Vec<String>> = vec![Vec::with_capacity(grid.len()); ncols];
    for row in &grid {
        for (c, cell) in row.iter().enumerate() {
            t[c].push(cell.clone());
        }
    }

    let (header, rows) = if has_header && !t.is_empty() {
        (Some(t.remove(0)), t)
    } else {
        (None, t)
    };
    Ok(Table { header, rows, delim: table.delim })
}

/// `--sep` must be a non-empty literal; xshape never guesses a delimiter (DESIGN.md).
fn require_sep(sep: &str) -> Result<()> {
    if sep.is_empty() {
        Err(XshapeError::Input("--sep must be a non-empty separator (xshape never guesses one)".into()))
    } else {
        Ok(())
    }
}

/// Reject a column address that lands past the table's width.
fn in_range(col: usize, ncols: usize) -> Result<()> {
    if col >= ncols {
        Err(XshapeError::Address(format!(
            "column {} is beyond the table's {ncols} columns",
            col_to_letter(col)
        )))
    } else {
        Ok(())
    }
}

/// Replace element `at` of `row` with the sequence `repl`, keeping everything else in place.
fn splice(row: &[String], at: usize, repl: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(row.len() - 1 + repl.len());
    out.extend_from_slice(&row[..at]);
    out.extend_from_slice(repl);
    out.extend_from_slice(&row[at + 1..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Table;

    fn wide() -> Table {
        Table {
            header: Some(vec!["contract".into(), "fy2024".into(), "fy2025".into()]),
            rows: vec![
                vec!["A-1".into(), "100".into(), "200".into()],
                vec!["B-2".into(), "0".into(), "50".into()],
            ],
            delim: b',',
        }
    }

    #[test]
    fn unpivot_gathers_to_long() {
        let out = unpivot(&wide(), "[fy2024]:[fy2025]", "fy", "amount").unwrap();
        assert_eq!(out.header.as_ref().unwrap(), &["contract", "fy", "amount"]);
        assert_eq!(out.nrows(), 4);
        // Row order: input row 0 fully spread, then input row 1.
        assert_eq!(out.rows[0], vec!["A-1", "fy2024", "100"]);
        assert_eq!(out.rows[1], vec!["A-1", "fy2025", "200"]);
        assert_eq!(out.rows[2], vec!["B-2", "fy2024", "0"]);
        assert_eq!(out.rows[3], vec!["B-2", "fy2025", "50"]);
    }

    #[test]
    fn unpivot_preserves_values_exactly() {
        let mut t = wide();
        t.rows[0][1] = "007".into(); // leading zero must survive
        // Gathering only B leaves A (contract) and C (fy2025) as id columns, so the header is
        // [contract, fy2025, k, v] and the gathered value lands in the last column.
        let out = unpivot(&t, "B", "k", "v").unwrap();
        assert_eq!(out.header.as_ref().unwrap(), &["contract", "fy2025", "k", "v"]);
        assert_eq!(out.rows[0], vec!["A-1", "200", "fy2024", "007"]);
    }

    #[test]
    fn unpivot_requires_header() {
        let t = Table { header: None, rows: vec![vec!["x".into()]], delim: b',' };
        assert!(unpivot(&t, "A", "k", "v").is_err());
    }

    #[test]
    fn unpivot_rejects_duplicate_gather() {
        assert!(unpivot(&wide(), "B,B", "k", "v").is_err());
    }

    // ---- explode ----

    fn listy() -> Table {
        Table {
            header: Some(vec!["contract".into(), "apps".into()]),
            rows: vec![
                vec!["C-1".into(), "Splunk; RSA; Imperva".into()],
                vec!["C-2".into(), "Solo".into()],
                vec!["C-3".into(), "".into()],
            ],
            delim: b',',
        }
    }

    #[test]
    fn explode_splits_cell_into_rows() {
        let out = explode(&listy(), "[apps]", "; ").unwrap();
        // 3 + 1 + 1 = 5 rows: the list row spreads, the others pass through (empty kept).
        assert_eq!(out.nrows(), 5);
        assert_eq!(out.rows[0], vec!["C-1", "Splunk"]);
        assert_eq!(out.rows[2], vec!["C-1", "Imperva"]);
        assert_eq!(out.rows[3], vec!["C-2", "Solo"]);
        assert_eq!(out.rows[4], vec!["C-3", ""]);
    }

    #[test]
    fn explode_literal_sep_no_trim() {
        // Splitting on ";" (not "; ") leaves the leading space — xshape never trims a value.
        let out = explode(&listy(), "[apps]", ";").unwrap();
        assert_eq!(out.rows[1], vec!["C-1", " RSA"]);
    }

    #[test]
    fn explode_requires_sep() {
        assert!(explode(&listy(), "[apps]", "").is_err());
    }

    // ---- split ----

    #[test]
    fn split_auto_width_pads_short_rows() {
        let out = split(&listy(), "[apps]", "; ", None).unwrap();
        // Widest row has 3 pieces → apps_1..apps_3; the original column is replaced in place.
        assert_eq!(out.header.as_ref().unwrap(), &["contract", "apps_1", "apps_2", "apps_3"]);
        assert_eq!(out.rows[0], vec!["C-1", "Splunk", "RSA", "Imperva"]);
        assert_eq!(out.rows[1], vec!["C-2", "Solo", "", ""]);
    }

    #[test]
    fn split_into_names_fixes_width() {
        let out = split(&listy(), "[apps]", "; ", Some(&["a".into(), "b".into(), "c".into()])).unwrap();
        assert_eq!(out.header.as_ref().unwrap(), &["contract", "a", "b", "c"]);
    }

    #[test]
    fn split_errors_when_into_too_narrow() {
        // Row 0 has 3 pieces but only 2 names — must error, never drop a piece.
        let err = split(&listy(), "[apps]", "; ", Some(&["a".into(), "b".into()])).unwrap_err();
        assert!(err.to_string().contains("never discards"), "got: {err}");
    }

    // ---- pivot ----

    fn long() -> Table {
        Table {
            header: Some(vec!["contract".into(), "fy".into(), "spend".into()]),
            rows: vec![
                vec!["A-1".into(), "fy2024".into(), "100".into()],
                vec!["A-1".into(), "fy2025".into(), "200".into()],
                vec!["B-2".into(), "fy2024".into(), "0".into()],
            ],
            delim: b',',
        }
    }

    #[test]
    fn pivot_spreads_to_wide() {
        let out = pivot(&long(), "[fy]", "[spend]").unwrap();
        assert_eq!(out.header.as_ref().unwrap(), &["contract", "fy2024", "fy2025"]);
        assert_eq!(out.rows[0], vec!["A-1", "100", "200"]);
        // B-2 has no fy2025 row → that cell is empty, not invented.
        assert_eq!(out.rows[1], vec!["B-2", "0", ""]);
    }

    #[test]
    fn pivot_errors_on_collision_not_aggregate() {
        let mut t = long();
        t.rows.push(vec!["A-1".into(), "fy2024".into(), "999".into()]); // duplicate cell
        let err = pivot(&t, "[fy]", "[spend]").unwrap_err();
        assert!(err.to_string().contains("does not aggregate"), "got: {err}");
    }

    #[test]
    fn pivot_is_unpivots_inverse() {
        let round = pivot(&unpivot(&wide(), "[fy2024]:[fy2025]", "fy", "amount").unwrap(), "[fy]", "[amount]").unwrap();
        assert_eq!(round.header.as_ref().unwrap(), &["contract", "fy2024", "fy2025"]);
        assert_eq!(round.rows[0], vec!["A-1", "100", "200"]);
    }

    // ---- merge ----

    #[test]
    fn merge_joins_columns_in_spec_order() {
        let t = Table {
            header: Some(vec!["first".into(), "last".into(), "dept".into()]),
            rows: vec![vec!["Ada".into(), "Lovelace".into(), "Eng".into()]],
            delim: b',',
        };
        let out = merge(&t, "[last],[first]", ", ", "name").unwrap();
        assert_eq!(out.header.as_ref().unwrap(), &["name", "dept"]);
        assert_eq!(out.rows[0], vec!["Lovelace, Ada", "Eng"]);
    }

    #[test]
    fn merge_keeps_empty_values() {
        let t = Table {
            header: Some(vec!["a".into(), "b".into()]),
            rows: vec![vec!["x".into(), "".into()]],
            delim: b',',
        };
        let out = merge(&t, "A:B", ";", "j").unwrap();
        assert_eq!(out.rows[0], vec!["x;"]);
    }

    // ---- transpose ----

    #[test]
    fn transpose_swaps_axes_first_col_becomes_header() {
        let t = Table {
            header: Some(vec!["metric".into(), "jan".into(), "feb".into()]),
            rows: vec![
                vec!["temp".into(), "5".into(), "6".into()],
                vec!["rain".into(), "10".into(), "20".into()],
            ],
            delim: b',',
        };
        let out = transpose(&t).unwrap();
        assert_eq!(out.header.as_ref().unwrap(), &["metric", "temp", "rain"]);
        assert_eq!(out.rows[0], vec!["jan", "5", "10"]);
        assert_eq!(out.rows[1], vec!["feb", "6", "20"]);
    }
}
