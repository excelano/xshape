//! CLI-level tests: run the built binary so the argument plumbing in main.rs —
//! which the library tests cannot reach — is covered too.

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
    child
        .stdin
        .take()
        .expect("no stdin")
        .write_all(input.as_bytes())
        .expect("failed to write to xshape's stdin");
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
