//! xshape — reshape the geometry of a single table without touching a value.
//!
//! The library half: the table model, the CSV/DSV I/O, the vendored column-addressing dialect
//! (`addr`), and the reshape verbs. The CLI lives in `main.rs`.

pub mod addr;
pub mod errors;
pub mod io;
pub mod model;
pub mod verbs;

pub use errors::{Result, XshapeError};
