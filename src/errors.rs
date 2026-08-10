//! xshape's error type. Reshape errors are structural (a bad column address, a header the
//! verb needs but the file lacks, a pivot collision), so the messages name the fix.

use std::fmt;

pub type Result<T> = std::result::Result<T, XshapeError>;

#[derive(Debug)]
pub enum XshapeError {
    /// A malformed or unresolvable column address (bad letter, unknown `[name]`, empty spec).
    Address(String),
    /// The verb needs something the input does not provide (a header row, a wider grid).
    Input(String),
    /// A pivot collision: two source rows would land in the same output cell. xshape never
    /// aggregates to resolve one — the message hands the job to xql/DuckDB.
    Collision(String),
    /// A verb that is scaffolded but not yet implemented (Phase 3).
    Unimplemented(String),
    /// Filesystem / CSV I/O.
    Io(String),
}

impl fmt::Display for XshapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XshapeError::Address(m) => write!(f, "address error: {m}"),
            XshapeError::Input(m) => write!(f, "{m}"),
            XshapeError::Collision(m) => write!(f, "collision: {m}"),
            XshapeError::Unimplemented(m) => write!(f, "not yet implemented: {m}"),
            XshapeError::Io(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for XshapeError {}

impl From<std::io::Error> for XshapeError {
    fn from(e: std::io::Error) -> Self {
        XshapeError::Io(e.to_string())
    }
}

/// An address that will not parse and one that names nothing in this table are both `Address`
/// here. `xaddr` keeps them apart for callers that show errors as you type; a CLI reports the
/// message either way, so the distinction earns nothing on this side.
impl From<xaddr::Error> for XshapeError {
    fn from(e: xaddr::Error) -> Self {
        XshapeError::Address(e.message)
    }
}
