//! Column addressing — xshape's slice of the shared dialect.
//!
//! The grammar itself lives in the `xaddr` crate: letters (`C`, `AF`), bracketed header names
//! (`[first name]`, `]]` an escaped literal `]`), inclusive ranges in either direction
//! (`A:D`, `[first]:[last]`), open-ended ranges that run to the table's edge (`B:`, `:C`), and
//! comma lists mixing all of them. xled resolves the same strings through the same code, so
//! an address learned in one tool means the same thing in the other.
//!
//! What this module contributes is xshape's two policies on top of it. Every verb here is
//! column-oriented, so an address naming a row or a single cell is rejected rather than
//! quietly projected onto its column. And bounds are `Strict`: a reshape that ran past the
//! edge of the table would silently do less than it was asked, which for a destructive verb
//! is worse than stopping. (xled clamps instead, deliberately — see `xaddr::Bounds`.)

use crate::errors::Result;
use crate::model::Table;
use xaddr::Bounds;

/// Resolve a column spec to an ordered list of 0-based column indices.
///
/// Order is preserved and duplicates are kept — the caller decides whether repetition is
/// meaningful (it is for `merge`, an error for `unpivot`). Names require a header; letters do
/// not. Every returned index is inside the table, so callers can use them directly.
pub fn cols(spec: &str, table: &Table) -> Result<Vec<usize>> {
    Ok(xaddr::parse(spec)?.columns(table, Bounds::Strict)?)
}

/// Resolve exactly one column from a spec, rejecting lists and ranges. For verbs that name a
/// single column (`split --col`, `pivot --names-from`).
pub fn one_col(spec: &str, table: &Table) -> Result<usize> {
    let cs = cols(spec, table)?;
    if cs.len() != 1 {
        return Err(crate::errors::XshapeError::Address(format!(
            "expected a single column, got {} from {spec:?}",
            cs.len()
        )));
    }
    Ok(cs[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Table;

    fn table() -> Table {
        Table {
            header: Some(vec![
                "id".into(),
                "first name".into(),
                "last name".into(),
                "fy2024".into(),
                "fy2025".into(),
            ]),
            rows: vec![],
            delim: b',',
        }
    }

    fn headerless() -> Table {
        Table {
            header: None,
            rows: vec![vec!["a".into(), "b".into()]],
            delim: b',',
        }
    }

    #[test]
    fn single_letter_and_name() {
        let t = table();
        assert_eq!(cols("A", &t).unwrap(), vec![0]);
        assert_eq!(cols("[first name]", &t).unwrap(), vec![1]);
    }

    #[test]
    fn letter_range_expands_inclusive() {
        let t = table();
        assert_eq!(cols("A:C", &t).unwrap(), vec![0, 1, 2]);
    }

    #[test]
    fn reversed_range_normalizes() {
        let t = table();
        assert_eq!(cols("C:A", &t).unwrap(), vec![0, 1, 2]);
    }

    #[test]
    fn name_range() {
        let t = table();
        assert_eq!(cols("[fy2024]:[fy2025]", &t).unwrap(), vec![3, 4]);
    }

    #[test]
    fn list_preserves_order_and_dups() {
        let t = table();
        assert_eq!(cols("D,B,B", &t).unwrap(), vec![3, 1, 1]);
    }

    #[test]
    fn mixed_list_and_range() {
        let t = table();
        assert_eq!(cols("[id],D:E", &t).unwrap(), vec![0, 3, 4]);
    }

    #[test]
    fn one_col_rejects_multiple() {
        let t = table();
        assert!(one_col("A:C", &t).is_err());
        assert_eq!(one_col("B", &t).unwrap(), 1);
    }

    #[test]
    fn unknown_name_errors() {
        let t = table();
        assert!(cols("[nope]", &t).is_err());
    }

    #[test]
    fn name_without_header_errors_clearly() {
        let t = headerless();
        let e = cols("[x]", &t).unwrap_err().to_string();
        assert!(e.contains("needs a header row"), "got: {e}");
    }

    #[test]
    fn escaped_bracket_in_name() {
        let mut t = table();
        t.header = Some(vec!["weird]name".into()]);
        assert_eq!(cols("[weird]]name]", &t).unwrap(), vec![0]);
    }

    #[test]
    fn empty_spec_errors() {
        let t = table();
        assert!(cols("", &t).is_err());
        assert!(cols("A,,B", &t).is_err());
    }

    /// The five forms xshape used to reject with an error about a column named `""`, while
    /// xled resolved them. They come from the shared crate now, so the two cannot disagree.
    #[test]
    fn open_ended_ranges_now_resolve() {
        let t = table();
        assert_eq!(cols("B:", &t).unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(cols(":C", &t).unwrap(), vec![0, 1, 2]);
        assert_eq!(cols("[last name]:", &t).unwrap(), vec![2, 3, 4]);
        assert_eq!(cols(":[last name]", &t).unwrap(), vec![0, 1, 2]);
    }

    /// Bounds are Strict here: a reshape stops rather than quietly doing less.
    #[test]
    fn past_the_last_column_is_refused_not_clamped() {
        let t = table();
        let e = cols("A:Z", &t).unwrap_err().to_string();
        assert!(e.contains("beyond the table's 5 columns"), "got: {e}");
    }

    /// Rows and cells belong to xled's half of the dialect; a reshape verb takes columns.
    #[test]
    fn row_and_cell_addresses_are_rejected() {
        let t = table();
        assert!(cols("3", &t).is_err());
        assert!(cols("$", &t).is_err());
        assert!(cols("B2", &t).is_err());
    }
}
