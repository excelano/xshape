//! The xshape CLI.
//!
//! One verb per subcommand, each reshaping a single table:
//!   xshape unpivot --cols E:K --into fy,amount data.csv
//!   cat data.csv | xshape unpivot --cols E:K
//!
//! The reshaped table goes to stdout by default — stdout *is* the preview, the whole new grid
//! in view before anything is committed. `-i` / `--in-place[=.bak]` writes it back to the file,
//! sed-style. A reshape always rewrites the whole table, so there is no inspect-vs-mutate split
//! to guard (unlike xled): `-i` simply needs a file, not piped stdin.

mod skill;

use clap::{Args, Parser, Subcommand};
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::process::exit;
use xshape::{io as xio, verbs, Result, XshapeError};

/// What each exit status means, so a caller can branch on the number instead of on the
/// message. A reshape that has nothing to do — no columns matched, one row to transpose —
/// still succeeds and still emits a table; the table is the answer.
const EXIT_CODES: &str = "\
Exit codes:
  0  success
  1  bad input \u{2014} unreadable file, malformed table, an address that does not resolve
  2  bad invocation \u{2014} unknown flag, missing argument, contradictory options";

#[derive(Parser)]
#[command(
    name = "xshape",
    version,
    about = "reshape tabular data — pivot, unpivot, split, merge, explode, transpose",
    long_about = "Change the geometry of a single table — which axis holds which cells — without \
                  ever changing, filtering, or aggregating a value. Cell edits are xled's job; \
                  querying the row set is xql's.",
    // The verb is optional only so `--install-skill` can stand alone; a bare
    // `xshape` still prints help rather than falling through to a None verb.
    arg_required_else_help = true,
    after_help = EXIT_CODES
)]
struct Cli {
    #[command(subcommand)]
    verb: Option<Verb>,

    /// Install xshape's Claude Code skill into ~/.claude/skills/xshape and exit.
    #[arg(long)]
    install_skill: bool,

    /// Remove the installed Claude Code skill and exit.
    #[arg(long)]
    uninstall_skill: bool,
}

#[derive(Subcommand)]
enum Verb {
    /// wide → long: gather a set of columns into two, a key and a value
    Unpivot(UnpivotArgs),
    /// long → wide: spread a key column into headers, a value column fills the grid
    Pivot(PivotArgs),
    /// one column into several, by an explicit separator
    Split(SplitArgs),
    /// a delimited cell into multiple rows, repeating the other fields
    Explode(ExplodeArgs),
    /// several columns into one, joined by an explicit separator
    Merge(MergeArgs),
    /// swap the row and column axes wholesale
    Transpose(TransposeArgs),
    /// move columns to a new position, changing order and nothing else
    Reorder(ReorderArgs),
}

/// Input + output plumbing shared by every verb.
#[derive(Args)]
struct Common {
    /// input file (CSV/TSV); omit, or give `-`, to read stdin
    file: Option<String>,
    /// field delimiter, `\t` for tab (defaults to ',', or tab for a .tsv file)
    #[arg(short, long, value_name = "CHAR", value_parser = xio::parse_delim)]
    delim: Option<u8>,
    /// treat the first row as data, not a header
    #[arg(long)]
    no_header: bool,
    /// write the result back to the file instead of stdout (like sed -i). Attach an optional
    /// backup suffix to keep the original: `-i.bak` / `--in-place=.bak`
    #[arg(
        short = 'i',
        long = "in-place",
        value_name = "SUFFIX",
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = ""
    )]
    in_place: Option<String>,
}

#[derive(Args)]
struct UnpivotArgs {
    /// columns to gather (the wide value columns), e.g. `E:K` or `[fy2024]:[fy2026]`
    #[arg(long)]
    cols: String,
    /// names for the two new columns, key,value (default: name,value)
    #[arg(long, value_name = "KEY,VALUE", default_value = "name,value")]
    into: String,
    #[command(flatten)]
    common: Common,
}

#[derive(Args)]
struct PivotArgs {
    /// column whose values become the new column headers
    #[arg(long)]
    names_from: String,
    /// column whose values fill the new grid
    #[arg(long)]
    values_from: String,
    #[command(flatten)]
    common: Common,
}

#[derive(Args)]
struct SplitArgs {
    /// column to split
    #[arg(long)]
    col: String,
    /// separator to split on (required — xshape never guesses a delimiter)
    #[arg(long)]
    sep: String,
    /// names for the new columns (default: `<col>_1`, `<col>_2`, …)
    #[arg(long, value_name = "A,B,C")]
    into: Option<String>,
    #[command(flatten)]
    common: Common,
}

#[derive(Args)]
struct ExplodeArgs {
    /// column whose delimited cell becomes multiple rows
    #[arg(long)]
    col: String,
    /// separator to split the cell on (required — xshape never guesses a delimiter)
    #[arg(long)]
    sep: String,
    #[command(flatten)]
    common: Common,
}

#[derive(Args)]
struct MergeArgs {
    /// columns to merge, e.g. `A:C` or `[first],[last]`
    #[arg(long)]
    cols: String,
    /// separator to join the values with (required)
    #[arg(long)]
    sep: String,
    /// name for the merged column
    #[arg(long)]
    into: String,
    #[command(flatten)]
    common: Common,
}

#[derive(Args)]
struct TransposeArgs {
    #[command(flatten)]
    common: Common,
}

#[derive(Args)]
struct ReorderArgs {
    /// columns to move, e.g. `[note]` or `D:F` or `[last],[first]`
    #[arg(long)]
    cols: String,
    #[command(flatten)]
    place: PlaceArgs,
    #[command(flatten)]
    common: Common,
}

/// Where the moved columns land. Exactly one is required — a reorder with no destination has
/// nothing to do, and defaulting to an edge would move columns the caller never asked to move.
#[derive(Args)]
#[group(required = true, multiple = false)]
struct PlaceArgs {
    /// place them immediately before this column
    #[arg(long, value_name = "COL")]
    before: Option<String>,
    /// place them immediately after this column
    #[arg(long, value_name = "COL")]
    after: Option<String>,
    /// place them at the front of the table
    #[arg(long)]
    first: bool,
    /// place them at the end of the table
    #[arg(long)]
    last: bool,
}

fn main() {
    let cli = Cli::parse_from(normalize_in_place(std::env::args()));
    // Terminal actions: they touch the user's skills directory and nothing
    // else, so they run before any input is read or any file is opened.
    if cli.install_skill {
        exit(skill::install());
    }
    if cli.uninstall_skill {
        exit(skill::uninstall());
    }
    // `arg_required_else_help` means a bare `xshape` prints help and never
    // reaches this, so the only way here without a verb is global flags alone.
    let Some(verb) = cli.verb else {
        eprintln!("xshape: no verb given — try `xshape --help`");
        exit(2);
    };
    if let Err(e) = run(verb) {
        // Every diagnostic names the tool that raised it, so a message in a
        // pipeline of family tools points at the one that objected.
        eprintln!("xshape: {e}");
        exit(1);
    }
}

/// sed attaches the in-place backup suffix to the flag (`-i.bak`); clap models an optional
/// value with `=`, so rewrite the short attached form `-i<suffix>` to `-i=<suffix>` before
/// parsing. Bare `-i` and the long `--in-place[=suffix]` form pass through untouched.
fn normalize_in_place<I: IntoIterator<Item = String>>(args: I) -> Vec<String> {
    args.into_iter()
        .map(|a| {
            if a.len() > 2 && a.starts_with("-i") && !a.starts_with("-i=") {
                format!("-i={}", &a[2..])
            } else {
                a
            }
        })
        .collect()
}

fn run(verb: Verb) -> Result<()> {
    match verb {
        Verb::Unpivot(a) => {
            let (key, value) = parse_pair(&a.into, "--into")?;
            let (table, source) = load(&a.common)?;
            let out = verbs::unpivot(&table, &a.cols, &key, &value)?;
            emit(&out, &a.common, source)
        }
        Verb::Pivot(a) => {
            let (table, source) = load(&a.common)?;
            let out = verbs::pivot(&table, &a.names_from, &a.values_from)?;
            emit(&out, &a.common, source)
        }
        Verb::Split(a) => {
            let into = a.into.as_deref().map(parse_list);
            let (table, source) = load(&a.common)?;
            let out = verbs::split(&table, &a.col, &a.sep, into.as_deref())?;
            emit(&out, &a.common, source)
        }
        Verb::Explode(a) => {
            let (table, source) = load(&a.common)?;
            let out = verbs::explode(&table, &a.col, &a.sep)?;
            emit(&out, &a.common, source)
        }
        Verb::Merge(a) => {
            let (table, source) = load(&a.common)?;
            let out = verbs::merge(&table, &a.cols, &a.sep, &a.into)?;
            emit(&out, &a.common, source)
        }
        Verb::Transpose(a) => {
            let (table, source) = load(&a.common)?;
            let out = verbs::transpose(&table)?;
            emit(&out, &a.common, source)
        }
        Verb::Reorder(a) => {
            // clap's group guarantees exactly one of the four is set.
            let place = if let Some(spec) = a.place.before.as_deref() {
                verbs::Place::Before(spec)
            } else if let Some(spec) = a.place.after.as_deref() {
                verbs::Place::After(spec)
            } else if a.place.first {
                verbs::Place::First
            } else {
                verbs::Place::Last
            };
            let (table, source) = load(&a.common)?;
            let out = verbs::reorder(&table, &a.cols, place)?;
            emit(&out, &a.common, source)
        }
    }
}

/// Split a `KEY,VALUE` pair argument, requiring exactly two comma-separated parts.
fn parse_pair(s: &str, flag: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = s.splitn(2, ',').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(XshapeError::Input(format!(
            "{flag} takes two comma-separated names, KEY,VALUE (got {s:?})"
        )));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Split a comma-separated `--into` list into owned names (for `split`). Empties are kept as-is;
/// the verb decides what a blank name means.
fn parse_list(s: &str) -> Vec<String> {
    s.split(',').map(str::to_string).collect()
}

/// Load the input table from the file or piped stdin. Returns the table and the source path
/// (`None` for stdin), which `emit` needs to honor `-i`.
///
/// Omitting the file and giving `-` mean the same thing; `-` is the spelling for a caller
/// that would rather say what it means than rely on the omission. A bare `xshape VERB` at a
/// terminal has no input coming, so it says so instead of sitting on a stdin nobody is
/// writing to. An explicit `-` still blocks, the way `cat -` does: that was asked for.
fn load(c: &Common) -> Result<(xshape::model::Table, Option<String>)> {
    let has_header = !c.no_header;
    let delim = c.delim;
    let path = c.file.as_deref().filter(|p| *p != "-");
    match path {
        Some(path) => Ok((
            xio::read_file(path, delim, has_header)?,
            Some(path.to_string()),
        )),
        None => {
            if c.file.is_none() && io::stdin().is_terminal() {
                eprintln!("xshape: no input — give a file, or pipe data in");
                exit(2);
            }
            let mut data = String::new();
            io::stdin().read_to_string(&mut data)?;
            Ok((
                xio::read_str(&data, delim.unwrap_or(b','), has_header)?,
                None,
            ))
        }
    }
}

/// Send the reshaped table to its destination: stdout by default, or — when `-i` is set and a
/// source file exists — back to that file, copying it to `<file><suffix>` first if a backup
/// suffix was given. `-i` over piped stdin has no file to edit and is an error.
fn emit(out: &xshape::model::Table, c: &Common, source: Option<String>) -> Result<()> {
    let text = xio::serialize(out)?;
    match (c.in_place.as_deref(), source) {
        (Some(suffix), Some(path)) => {
            if !suffix.is_empty() {
                fs::copy(&path, format!("{path}{suffix}"))
                    .map_err(|e| XshapeError::Io(e.to_string()))?;
            }
            fs::write(&path, &text).map_err(|e| XshapeError::Io(e.to_string()))
        }
        (Some(_), None) => Err(XshapeError::Input(
            "-i edits a file in place — it needs a file argument, not stdin".into(),
        )),
        (None, _) => write_stdout(&text),
    }
}

/// Write to stdout, exiting cleanly when the reader closes the pipe early (`xshape … | head`).
fn write_stdout(s: &str) -> Result<()> {
    let mut out = io::stdout().lock();
    match out.write_all(s.as_bytes()).and_then(|()| out.flush()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => exit(0),
        Err(e) => Err(XshapeError::Io(e.to_string())),
    }
}
