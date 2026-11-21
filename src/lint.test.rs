use super::*;
use crate::parse;

#[test]
fn duplicates_and_bounds() {
    let doc = parse("ID;P\nB;Y1;X1\nC;Y1;X1;K1\nC;K2\nC;X2;K3\nC;X1;AA comment\nE\n");
    let found: Vec<(usize, &str, String)> = lint(&doc)
        .into_iter()
        .map(|d| (d.line, d.rule, d.message))
        .collect();
    assert_eq!(
        found,
        vec![
            (
                4,
                parser_rs::rule::DUPLICATE_CELL,
                "this cell is already set at line 3; this record wins".into()
            ),
            (
                5,
                parser_rs::rule::CELL_OUTSIDE_BOUNDS,
                "row 1, column 2 is outside the 1 rows and 1 columns the B record gives".into()
            ),
        ]
    );
    assert!(lint(&parse("ID;P\nC;Y1;X1;K1\nE\n")).is_empty());
}
