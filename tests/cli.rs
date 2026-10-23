//! CLI-level tests: run the built binary so the argument plumbing in main.rs —
//! which the library tests cannot reach — is covered too.
//!
//! Author: David M. Anderson
//! Built with AI assistance (Claude, Anthropic)

use std::io::Write;
use std::process::{Command, Stdio};

const TABLE: &str = "id,name,qty\n007,Ann,5\n008,Bob,3\n";

fn run(args: &[&str]) -> (String, String, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_xshape"))
        .args(args)
        .output()
        .expect("failed to run xshape");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

fn run_piped(args: &[&str], input: &str) -> (String, String, i32) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_xshape"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run xshape");
    // A command that fails argument validation exits without reading stdin.
    if let Err(e) = child
        .stdin
        .take()
        .expect("no stdin")
        .write_all(input.as_bytes())
    {
        if e.kind() != std::io::ErrorKind::BrokenPipe {
            panic!("failed to write to xshape's stdin: {e}");
        }
    }
    let out = child.wait_with_output().expect("failed to wait for xshape");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn a_dash_reads_the_same_table_as_an_omitted_file() {
    let (dash, _, dash_code) = run_piped(&["transpose", "-"], TABLE);
    let (bare, _, bare_code) = run_piped(&["transpose"], TABLE);
    assert_eq!(dash_code, 0, "explicit `-` should succeed");
    assert_eq!(bare_code, 0);
    assert_eq!(dash, bare, "`-` and an omitted file must read alike");
    assert!(dash.starts_with("id,007,008"), "unexpected output: {dash}");
}

#[test]
fn in_place_over_stdin_is_refused_however_stdin_was_spelled() {
    for args in [vec!["transpose", "-", "-i"], vec!["transpose", "-i"]] {
        let (_, stderr, code) = run_piped(&args, TABLE);
        assert_eq!(code, 1, "{args:?} should fail");
        assert!(
            stderr.contains("needs a file argument"),
            "{args:?} gave: {stderr}"
        );
    }
}

#[test]
fn a_missing_file_is_bad_input_not_a_bad_invocation() {
    let (_, stderr, code) = run(&["transpose", "no/such/file.csv"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("no/such/file.csv"), "{stderr}");
}

#[test]
fn an_unknown_flag_is_a_bad_invocation() {
    let (_, _, code) = run(&["transpose", "--nope"]);
    assert_eq!(code, 2);
}

#[test]
fn help_states_the_exit_code_contract() {
    let (stdout, _, code) = run(&["--help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Exit codes:"), "{stdout}");
}

/// The destination group is required and exclusive, and both failures are invocation errors
/// (exit 2) rather than input errors — clap rejects them before a file is ever opened.
#[test]
fn reorder_demands_exactly_one_destination() {
    let (_, stderr, code) = run_piped(&["reorder", "--cols", "[qty]"], TABLE);
    assert_eq!(code, 2);
    assert!(stderr.contains("required arguments"), "{stderr}");

    let (_, stderr, code) = run_piped(&["reorder", "--cols", "[qty]", "--first", "--last"], TABLE);
    assert_eq!(code, 2);
    assert!(stderr.contains("cannot be used with"), "{stderr}");
}

#[test]
fn reorder_moves_a_column_relative_to_another() {
    let (stdout, _, code) = run_piped(&["reorder", "--cols", "[qty]", "--before", "[name]"], TABLE);
    assert_eq!(code, 0);
    assert_eq!(stdout, "id,qty,name\n007,5,Ann\n008,3,Bob\n");
}

const TSV: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/tags.tsv");

#[test]
fn a_tsv_file_is_read_and_written_with_tabs() {
    let (stdout, stderr, code) = run(&["explode", "--col", "[tags]", "--sep", ";", TSV]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        stdout,
        "id\tname\ttags\n007\tAnn\ta\n007\tAnn\tb\n008\tBob\t\n"
    );
}

/// Piped input has no extension to detect a tab from, so it takes `-d '\t'`.
#[test]
fn piped_tsv_needs_the_tab_delimiter_named() {
    let tsv = std::fs::read_to_string(TSV).unwrap();
    let (_, stderr, code) = run_piped(&["explode", "--col", "[tags]", "--sep", ";"], &tsv);
    assert_eq!(code, 1, "{stderr}");

    let (stdout, _, code) = run_piped(
        &["explode", "-d", "\\t", "--col", "[tags]", "--sep", ";"],
        &tsv,
    );
    assert_eq!(code, 0);
    assert!(
        stdout.starts_with("id\tname\ttags\n007\tAnn\ta\n"),
        "{stdout}"
    );
}

#[test]
fn no_header_addresses_by_letter_and_emits_no_header() {
    let input = "007,Ann,a;b\n008,Bob,c\n";
    let (stdout, _, code) = run_piped(
        &["explode", "--no-header", "--col", "C", "--sep", ";"],
        input,
    );
    assert_eq!(code, 0);
    assert_eq!(stdout, "007,Ann,a\n007,Ann,b\n008,Bob,c\n");

    let (_, stderr, code) = run_piped(
        &["explode", "--no-header", "--col", "[tags]", "--sep", ";"],
        input,
    );
    assert_eq!(code, 1);
    assert!(stderr.contains("needs a header row"), "{stderr}");
}

#[test]
fn unpivot_without_a_header_is_refused() {
    let (_, stderr, code) = run_piped(
        &["unpivot", "--no-header", "--cols", "B:C", "--into", "k,v"],
        "007,Ann,a\n",
    );
    assert_eq!(code, 1);
    assert!(stderr.contains("unpivot needs a header row"), "{stderr}");
}
