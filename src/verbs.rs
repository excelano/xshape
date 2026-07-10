//! The reshape verbs. Each takes a `&Table` and returns a new `Table` — geometry changes only,
//! every input cell reappearing unchanged. `unpivot` is implemented (corpus rank 1); the rest
//! are scaffolded and return `Unimplemented` until Phase 3 (see DESIGN.md, "Corpus-ranked build
//! order": unpivot → explode → split → pivot → merge/transpose).

use crate::addr;
use crate::errors::{Result, XshapeError};
use crate::model::Table;

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

pub fn pivot(_table: &Table) -> Result<Table> {
    Err(XshapeError::Unimplemented(
        "pivot (long → wide) — Phase 3, corpus rank 4. Errors on collision by design.".into(),
    ))
}

pub fn split(_table: &Table) -> Result<Table> {
    Err(XshapeError::Unimplemented(
        "split (one column → several) — Phase 3, corpus rank 3. Requires an explicit --sep.".into(),
    ))
}

pub fn explode(_table: &Table) -> Result<Table> {
    Err(XshapeError::Unimplemented(
        "explode (delimited cell → rows) — Phase 3, corpus rank 2. Requires an explicit --sep.".into(),
    ))
}

pub fn merge(_table: &Table) -> Result<Table> {
    Err(XshapeError::Unimplemented(
        "merge (several columns → one) — Phase 3, corpus rank 5. Requires an explicit --sep.".into(),
    ))
}

pub fn transpose(_table: &Table) -> Result<Table> {
    Err(XshapeError::Unimplemented(
        "transpose (swap axes) — Phase 3, corpus rank 5.".into(),
    ))
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
}
