//! Column addressing — xshape's slice of xled's dialect.
//!
//! xled's full address algebra is set algebra over *cells* (regex row-select, intersect,
//! negate). xshape's verbs are column-oriented, so it vendors only the column half of the
//! dialect: a spec resolves to an ordered list of column indices. The atoms match xled
//! exactly — a bijective base-26 letter (`C`, `AF`), or a bracketed header name
//! (`[first name]`, `[price (USD)]`, `]]` an escaped literal `]`). On top of the atoms:
//!
//!   - a range `A:D` / `[first]:[last]` — inclusive, either direction, expanded left→right
//!   - a comma list `C,E,[foo]` — atoms and ranges in any mix, order preserved, dups kept
//!
//! Preserving order matters: `unpivot --cols K,E` gathers K before E because the user said so.
//! When `xaddr` is extracted into a shared crate this module and `model`'s letter helpers go
//! with it, and xled depends on the same code instead of its private copy.

use crate::errors::{Result, XshapeError};
use crate::model::{letter_to_col, Table};

/// Resolve a column spec to an ordered list of 0-based column indices.
///
/// The spec is a comma-separated list of atoms and ranges. Order is preserved and duplicates
/// are kept — the caller decides whether repetition is meaningful (it is for `merge`, an error
/// for `unpivot`). Names require a header; letters do not.
pub fn cols(spec: &str, table: &Table) -> Result<Vec<usize>> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err(XshapeError::Address("empty column spec".into()));
    }
    let mut out = Vec::new();
    for part in split_top_level(spec)? {
        let part = part.trim();
        if part.is_empty() {
            return Err(XshapeError::Address("empty item in column list".into()));
        }
        match split_range(part)? {
            Some((lo, hi)) => {
                let a = atom(lo.trim(), table)?;
                let b = atom(hi.trim(), table)?;
                let (a, b) = if a <= b { (a, b) } else { (b, a) };
                out.extend(a..=b);
            }
            None => out.push(atom(part, table)?),
        }
    }
    Ok(out)
}

/// Resolve exactly one column from a spec, rejecting lists and ranges. For verbs that name a
/// single column (`split --col`, `pivot --names-from`).
pub fn one_col(spec: &str, table: &Table) -> Result<usize> {
    let cs = cols(spec, table)?;
    if cs.len() != 1 {
        return Err(XshapeError::Address(format!(
            "expected a single column, got {} from {spec:?}",
            cs.len()
        )));
    }
    Ok(cs[0])
}

/// Resolve a single atom — a bracketed `[name]` or a run of letters — to a column index.
fn atom(s: &str, table: &Table) -> Result<usize> {
    if let Some(rest) = s.strip_prefix('[') {
        let name = parse_name(rest)?;
        return table.name_to_col(&name).ok_or_else(|| {
            if table.header.is_none() {
                XshapeError::Address(format!(
                    "column name [{name}] needs a header row (this file has none — address by letter, or drop --no-header)"
                ))
            } else {
                XshapeError::Address(format!("no column named [{name}]"))
            }
        });
    }
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphabetic()) {
        return Ok(letter_to_col(s));
    }
    Err(XshapeError::Address(format!(
        "unrecognized column {s:?} — use a letter (C, AF) or a bracketed name ([first name])"
    )))
}

/// Parse a bracketed name body (everything after the opening `[`). `]]` is an escaped literal
/// `]`; a lone `]` closes the name and must be the last character. Matches xled's `parse_name`.
fn parse_name(body: &str) -> Result<String> {
    let bytes: Vec<char> = body.chars().collect();
    let mut name = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == ']' {
            if bytes.get(i + 1) == Some(&']') {
                name.push(']');
                i += 2;
            } else if i + 1 == bytes.len() {
                return Ok(name);
            } else {
                return Err(XshapeError::Address(format!(
                    "trailing text after [name]: {:?}",
                    bytes[i + 1..].iter().collect::<String>()
                )));
            }
        } else {
            name.push(bytes[i]);
            i += 1;
        }
    }
    Err(XshapeError::Address("unterminated [name]".into()))
}

/// Split a spec on top-level commas — commas inside `[...]` are part of a name, not separators.
fn split_top_level(spec: &str) -> Result<Vec<String>> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let chars: Vec<char> = spec.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '[' => {
                depth += 1;
                cur.push(c);
            }
            ']' => {
                // `]]` inside a name stays inside; otherwise it closes one level.
                if depth > 0 && chars.get(i + 1) == Some(&']') {
                    cur.push_str("]]");
                    i += 2;
                    continue;
                }
                depth -= 1;
                cur.push(c);
            }
            ',' if depth <= 0 => {
                parts.push(std::mem::take(&mut cur));
            }
            _ => cur.push(c),
        }
        i += 1;
    }
    if depth > 0 {
        return Err(XshapeError::Address("unterminated [name]".into()));
    }
    parts.push(cur);
    Ok(parts)
}

/// Split one item on a top-level `:` into (lo, hi), or `None` if it is not a range. A `:`
/// inside `[...]` is part of a name.
fn split_range(item: &str) -> Result<Option<(String, String)>> {
    let chars: Vec<char> = item.chars().collect();
    let mut depth = 0i32;
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '[' => depth += 1,
            ']' => {
                if depth > 0 && chars.get(i + 1) == Some(&']') {
                    i += 2;
                    continue;
                }
                depth -= 1;
            }
            ':' if depth <= 0 => {
                let lo: String = chars[..i].iter().collect();
                let hi: String = chars[i + 1..].iter().collect();
                return Ok(Some((lo, hi)));
            }
            _ => {}
        }
        i += 1;
    }
    Ok(None)
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
        Table { header: None, rows: vec![vec!["a".into(), "b".into()]], delim: b',' }
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
}
