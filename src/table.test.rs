use super::*;
use crate::parse;

#[test]
fn the_cursor_carries_over() {
    // From the SheetJS notes: F records move the cursor too.
    let doc = parse("ID;P\nF;M4;Y1;X1\nC;K\"A1\"\nF;M5;X2\nC;K\"C1\";X3\nC;K\"C2\";Y2\nE\n");
    let table = Table::new(&doc);
    assert_eq!(table.text(1, 1), "A1");
    assert_eq!(table.text(1, 3), "C1");
    assert_eq!(table.text(2, 3), "C2");
    assert_eq!(table.text(1, 2), "");
    assert_eq!((table.rows(), table.cols()), (2, 3));
    assert_eq!(
        cursors(&doc),
        vec![(1, 1), (1, 1), (1, 1), (1, 2), (1, 3), (2, 3), (2, 3)]
    );
}

#[test]
fn bounds_formulas_comments_duplicates() {
    let doc = parse("ID;P\nB;Y5;X4\nC;Y1;X1;AA note\nC;K2;ESUM(R2C2)\nC;K3\nC;X2;K\"x\"\nE\n");
    let table = Table::new(&doc);
    assert_eq!(table.bounds, Some((5, 4)));
    assert_eq!((table.rows(), table.cols()), (5, 4));
    let a1 = table.get(1, 1).unwrap();
    // The comment's record and the value's are one cell; the second value
    // is a duplicate.
    assert_eq!(a1.value, Value::Number("3".into()));
    assert_eq!(a1.formula, None);
    assert_eq!(a1.comment.as_deref(), Some("A note"));
    assert_eq!(a1.record, 4);
    assert_eq!(table.duplicates, vec![(4, 3)]);
    assert_eq!(table.cells().count(), 2);
}
