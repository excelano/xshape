//! The table: the in-memory grid xshape reshapes.
//!
//! Stringly-typed (`Vec<Vec<String>>`) so leading zeros and long IDs survive a reshape
//! untouched — xshape moves cells between axes, it never reinterprets them. The header is
//! an overlay kept separate from the data rows; `None` when the file has no header row.
//! Ragged rows are tolerated: a missing cell reads as "".
//!
//! Addressing itself is not here. Column letters, bracketed names, and ranges live in the
//! `xaddr` crate, which xled uses too, so the dialect is one implementation rather than two
//! that agree in the middle and drift at the edges. This module only teaches `xaddr` what
//! shape this table is, via the `Grid` impl on `Shape` below.
//!
//! Author: David M. Anderson
//! Built with AI assistance (Claude, Anthropic)

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
        self.header
            .as_ref()
            .and_then(|h| h.get(c))
            .map(|s| s.as_str())
    }

    /// The label to show for a column: its header name, or its letter when there is no header.
    pub fn col_label(&self, c: usize) -> String {
        self.col_name(c)
            .map(str::to_string)
            .unwrap_or_else(|| col_to_letter(c))
    }

    /// Resolve a bracketed column name to its index. Case-sensitive, exact (`[userId]` ≠ `userid`).
    pub fn name_to_col(&self, name: &str) -> Option<usize> {
        self.header.as_ref()?.iter().position(|h| h == name)
    }

    pub fn shape(&self) -> Shape {
        Shape {
            header: self.header.clone(),
            nrows: self.nrows(),
            ncols: self.ncols(),
            delim: self.delim,
        }
    }
}

/// What a verb needs to know about its input before the first row: the header, the row count,
/// the logical width (the widest of the header and any data row), and the delimiter. A streamed
/// input learns it in a first pass; a `Table` computes it from its rows.
#[derive(Clone, Debug)]
pub struct Shape {
    pub header: Option<Vec<String>>,
    pub nrows: usize,
    pub ncols: usize,
    pub delim: u8,
}

impl Shape {
    /// The label to show for a column: its header name, or its letter when there is no header.
    pub fn col_label(&self, c: usize) -> String {
        self.header
            .as_ref()
            .and_then(|h| h.get(c))
            .cloned()
            .unwrap_or_else(|| col_to_letter(c))
    }
}

/// Column letters ↔ index lives in `xaddr` now, so xshape and xled cannot drift apart on it.
pub use xaddr::col_to_letter;

/// Everything `xaddr` needs to resolve an address against an input of this shape.
///
/// `name_to_col` is left to the trait's default: an exact, case-sensitive position match.
impl xaddr::Grid for Shape {
    fn nrows(&self) -> usize {
        self.nrows
    }

    fn ncols(&self) -> usize {
        self.ncols
    }

    fn header(&self) -> Option<&[String]> {
        self.header.as_deref()
    }
}
