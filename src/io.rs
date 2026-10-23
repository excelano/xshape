//! Read and write CSV/DSV. Tolerant on input (ragged rows, flexible widths); the `csv`
//! crate handles quoting and embedded newlines. Values are kept as strings so leading
//! zeros and long IDs survive a reshape untouched.
//!
//! Author: David M. Anderson
//! Built with AI assistance (Claude, Anthropic)

use crate::errors::{Result, XshapeError};
use crate::model::{Shape, Table};
use crate::verbs::Input;
use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use std::fs::File;
use std::io::Read;
use std::path::Path;

fn io_err(e: impl std::fmt::Display) -> XshapeError {
    XshapeError::Io(e.to_string())
}

/// Parse CSV/DSV text into a table. When `has_header`, the first record becomes the
/// name overlay; otherwise every record is data (columns reachable only by letter).
pub fn read_str(data: &str, delim: u8, has_header: bool) -> Result<Table> {
    Source::from_bytes(data.as_bytes().to_vec(), delim, has_header)?.table()
}

/// An input read in passes rather than held as a table. Opening it parses every record once to
/// learn the shape, so a malformed file fails before any output is written; each later pass
/// parses again from the start. A file is re-read from disk, and piped input is kept as its raw
/// bytes, a fraction of the size of the parsed table.
pub struct Source {
    data: Data,
    has_header: bool,
    shape: Shape,
}

enum Data {
    File(String),
    Bytes(Vec<u8>),
}

impl Source {
    /// Open a file, choosing the delimiter from its extension unless one is given.
    pub fn open(path: &str, delim: Option<u8>, has_header: bool) -> Result<Source> {
        sniff_and_warn(path);
        let delim = delim.unwrap_or_else(|| default_delim(path));
        Source::scan(Data::File(path.to_string()), delim, has_header)
    }

    pub fn from_bytes(data: Vec<u8>, delim: u8, has_header: bool) -> Result<Source> {
        Source::scan(Data::Bytes(data), delim, has_header)
    }

    fn scan(data: Data, delim: u8, has_header: bool) -> Result<Source> {
        let mut source = Source {
            data,
            has_header,
            shape: Shape {
                header: None,
                nrows: 0,
                ncols: 0,
                delim,
            },
        };
        let (mut header, mut nrows, mut ncols) = (None, 0, 0);
        source.records(&mut |first, rec| {
            ncols = ncols.max(rec.len());
            if first && has_header {
                header = Some(rec.iter().map(str::to_string).collect());
            } else {
                nrows += 1;
            }
            Ok(())
        })?;
        source.shape.header = header;
        source.shape.nrows = nrows;
        source.shape.ncols = ncols;
        Ok(source)
    }

    /// One pass over every record, header included. The csv reader strips a UTF-8 BOM (Excel's
    /// "CSV UTF-8"), so the first column name does not carry a U+FEFF.
    fn records(&self, f: &mut dyn FnMut(bool, &StringRecord) -> Result<()>) -> Result<()> {
        let input: Box<dyn Read + '_> = match &self.data {
            Data::File(path) => {
                // Name the file in the error. A bare "No such file or directory" is useless
                // in a script that reads several, and the OS error alone does not carry it.
                Box::new(File::open(path).map_err(|e| XshapeError::Io(format!("{path}: {e}")))?)
            }
            Data::Bytes(bytes) => Box::new(bytes.as_slice()),
        };
        let mut rdr = ReaderBuilder::new()
            .delimiter(self.shape.delim)
            .has_headers(false)
            .flexible(true)
            .from_reader(input);
        let mut rec = StringRecord::new();
        let mut first = true;
        while rdr.read_record(&mut rec).map_err(io_err)? {
            f(first, &rec)?;
            first = false;
        }
        Ok(())
    }

    /// The whole input as a table, for the verbs that need every row at once.
    pub fn table(&self) -> Result<Table> {
        let mut rows = Vec::with_capacity(self.shape.nrows);
        self.each_row(&mut |row| {
            rows.push(row.iter().map(|s| s.to_string()).collect());
            Ok(())
        })?;
        Ok(Table {
            header: self.shape.header.clone(),
            rows,
            delim: self.shape.delim,
        })
    }
}

impl Input for Source {
    fn shape(&self) -> Shape {
        self.shape.clone()
    }

    fn each_row(&self, f: &mut dyn FnMut(&[&str]) -> Result<()>) -> Result<()> {
        self.records(&mut |first, rec| {
            if first && self.has_header {
                return Ok(());
            }
            let cells: Vec<&str> = rec.iter().collect();
            f(&cells)
        })
    }
}

/// Sniff the file head for non-UTF-8 encodings and emit a one-line iconv hint when
/// warranted. Sniff failures are silently ignored — the downstream read surfaces a
/// clearer error if the file is really unreadable.
fn sniff_and_warn(path: &str) {
    let Ok(s) = encsniff::sniff_file(path) else {
        return;
    };
    if !s.is_warning() {
        return;
    }
    // Two shapes of warning. A signature match can name the encoding; a window
    // that is merely not valid UTF-8 cannot, and saying so plainly beats the
    // old behaviour, which was to call the file usable and let the read fail
    // with "stream did not contain valid UTF-8" — a message naming neither the
    // problem nor the fix.
    match s.encoding {
        Some(enc) => eprintln!("xshape: warning: {path} appears to be {enc} encoded."),
        None => eprintln!(
            "xshape: warning: {path} is not valid UTF-8, and its encoding could not be identified."
        ),
    }
    if let Some(hint) = &s.hint {
        eprintln!("hint: {hint}");
    }
}

/// Parse a `--delim` value: one ASCII character, or the escape `\t` for tab.
/// The escape earns its keep because a literal tab is awkward to type and most
/// shells swallow it; the rest of the family accepts it for the same reason.
pub fn parse_delim(s: &str) -> std::result::Result<u8, String> {
    let c = if s == "\\t" || s == "\t" {
        '\t'
    } else {
        let mut chars = s.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => c,
            _ => {
                return Err(format!(
                    "expected one character (or \\t for tab), got {s:?}"
                ))
            }
        }
    };
    if !c.is_ascii() {
        return Err(format!("expected an ASCII character, got {c:?}"));
    }
    Ok(c as u8)
}

/// `\t` for `.tsv`, otherwise `,`.
pub fn default_delim(path: &str) -> u8 {
    match Path::new(path).extension().and_then(|e| e.to_str()) {
        Some("tsv") => b'\t',
        _ => b',',
    }
}

/// Serialize the whole table back to CSV/DSV text (header overlay first, then data rows).
pub fn serialize(table: &Table) -> Result<String> {
    let mut wtr = WriterBuilder::new()
        .delimiter(table.delim)
        .flexible(true)
        .from_writer(Vec::new());

    if let Some(h) = &table.header {
        wtr.write_record(h).map_err(io_err)?;
    }
    for row in &table.rows {
        wtr.write_record(row).map_err(io_err)?;
    }

    let bytes = wtr.into_inner().map_err(io_err)?;
    String::from_utf8(bytes).map_err(io_err)
}

#[cfg(test)]
mod tests {
    use super::parse_delim;

    #[test]
    fn tab_is_reachable_by_escape_and_literally() {
        assert_eq!(parse_delim("\\t"), Ok(b'\t'));
        assert_eq!(parse_delim("\t"), Ok(b'\t'));
    }

    #[test]
    fn ordinary_single_characters_pass_through() {
        assert_eq!(parse_delim(","), Ok(b','));
        assert_eq!(parse_delim("|"), Ok(b'|'));
        assert_eq!(parse_delim(";"), Ok(b';'));
    }

    #[test]
    fn multi_character_and_non_ascii_are_refused() {
        // The csv reader takes one byte, so a multi-byte char can't be a delimiter.
        assert!(parse_delim("ab").is_err());
        assert!(parse_delim("").is_err());
        assert!(parse_delim("§").is_err());
    }
}
