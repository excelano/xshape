//! The table: the in-memory grid xshape reshapes.
//!
//! Stringly-typed (`Vec<Vec<String>>`) so leading zeros and long IDs survive a reshape
//! untouched — xshape moves cells between axes, it never reinterprets them. The header is
//! an overlay kept separate from the data rows; `None` when the file has no header row.
//! Ragged rows are tolerated: a missing cell reads as "".
//!
//! Column-letter ↔ index is bijective base-26: A=0, Z=25, AA=26, … — the same dialect as
//! xled, vendored here verbatim (see DESIGN.md, "Addressing strategy"). When `xaddr` is
//! extracted into a shared crate, these move there and both tools depend on it.

#[derive(Clone, Debug)]
pub struct Table {
    /// Column-name overlay. `None` when the file has no header row.
    pub header: Option<Vec<String>>,
    /// Data rows only (the header, if any, lives in `header`).
    pub rows: Vec<Vec<String>>,
    /// Field delimiter (`,` for CSV, `\t` for TSV).
    pub delim: u8,
}

impl Table {
    /// Number of data rows.
    pub fn nrows(&self) -> usize {
        self.rows.len()
    }

    /// Logical width: the widest of the header and any data row.
    pub fn ncols(&self) -> usize {
        let h = self.header.as_ref().map(|h| h.len()).unwrap_or(0);
        let r = self.rows.iter().map(|r| r.len()).max().unwrap_or(0);
        h.max(r)
    }

    /// Cell value at 0-based (row, col); "" if the row is short or out of range (ragged).
    pub fn cell(&self, r: usize, c: usize) -> &str {
        self.rows
            .get(r)
            .and_then(|row| row.get(c))
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// The header label for a column, if a header overlay exists.
    pub fn col_name(&self, c: usize) -> Option<&str> {
        self.header.as_ref().and_then(|h| h.get(c)).map(|s| s.as_str())
    }

    /// The label to show for a column: its header name, or its letter when there is no header.
    pub fn col_label(&self, c: usize) -> String {
        self.col_name(c).map(str::to_string).unwrap_or_else(|| col_to_letter(c))
    }

    /// Resolve a bracketed column name to its index. Case-sensitive, exact (`[userId]` ≠ `userid`).
    pub fn name_to_col(&self, name: &str) -> Option<usize> {
        self.header.as_ref()?.iter().position(|h| h == name)
    }
}

/// Column letters → 0-based index. "A"→0, "Z"→25, "AA"→26. Letters are uppercased first.
pub fn letter_to_col(s: &str) -> usize {
    let mut n: usize = 0;
    for ch in s.chars() {
        n = n * 26 + (ch.to_ascii_uppercase() as usize - 'A' as usize + 1);
    }
    n - 1
}

/// 0-based index → column letters. Inverse of [`letter_to_col`].
pub fn col_to_letter(mut c: usize) -> String {
    let mut s = Vec::new();
    loop {
        s.push(b'A' + (c % 26) as u8);
        if c < 26 {
            break;
        }
        c = c / 26 - 1;
    }
    s.reverse();
    String::from_utf8(s).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_round_trip() {
        for (i, s) in [(0, "A"), (25, "Z"), (26, "AA"), (27, "AB"), (51, "AZ"), (52, "BA")] {
            assert_eq!(letter_to_col(s), i);
            assert_eq!(col_to_letter(i), s);
        }
    }

    #[test]
    fn lowercase_letters_accepted() {
        assert_eq!(letter_to_col("c"), letter_to_col("C"));
    }
}
