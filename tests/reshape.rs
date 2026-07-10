//! End-to-end reshape tests over the read → verb → serialize pipeline on a real fixture,
//! including the two hazards xshape must never trip: a leading-zero id and a quoted comma.

use xshape::{io, verbs};

fn fixture() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/wide.csv")).unwrap()
}

#[test]
fn unpivot_fixture_round_trips_values() {
    let table = io::read_str(&fixture(), b',', true).unwrap();
    let long = verbs::unpivot(&table, "[fy2024]:[fy2026]", "fiscal_year", "spend").unwrap();

    // 2 data rows × 3 gathered columns = 6 long rows.
    assert_eq!(long.nrows(), 6);
    assert_eq!(long.header.as_ref().unwrap(), &["contract_id", "vendor", "fiscal_year", "spend"]);

    let out = io::serialize(&long).unwrap();

    // Leading-zero id survives untouched (stringly-typed).
    assert!(out.contains("0007,Acme LLC,fy2024,100"));
    assert!(out.contains("0042,"));
    // The value with an embedded comma is re-quoted correctly by the CSV writer.
    assert!(out.contains("\"Beta, Inc.\""));
    // Every value is preserved, nothing invented or aggregated.
    assert!(out.contains("0042,\"Beta, Inc.\",fy2025,50"));
}
