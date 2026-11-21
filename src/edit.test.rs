use super::*;
use crate::parse;

/// `src` with the cell set, and its table after.
fn set(src: &str, row: u32, col: u32, value: Option<Value>) -> (String, Table) {
    let doc = parse(src);
    let out = apply(src, &set_cell(&doc, row, col, value.as_ref()));
    let after = parse(&out);
    assert_eq!(after.errors, vec![], "{out}");
    (out.clone(), Table::new(&after))
}

const SHEET: &str =
    "ID;PWXL;N;E\nB;Y3;X3;D0\nC;Y1;X1;K\"name\"\nC;X2;K\"hp\"\nC;Y2;X1;K\"hfoo\"\nC;X2;K420\nE\n";

#[test]
fn replace_a_value() {
    let (out, table) = set(SHEET, 2, 2, Some(Value::Number("500".into())));
    assert_eq!(out, SHEET.replace("K420", "K500"));
    assert_eq!(table.text(2, 2), "500");
    // The same value: nothing to do.
    assert!(set_cell(&parse(SHEET), 2, 2, Some(&Value::Number("420".into()))).is_empty());
    let (out, _) = set(SHEET, 1, 1, Some(Value::Str("a;b\nc".into())));
    assert!(out.contains("C;Y1;X1;K\"a;;b\x1B :c\"\n"), "{out}");
}

#[test]
fn a_new_cell_in_reading_order() {
    // In row 1, after its last cell: the next record (row 2) names its row
    // already.
    let (out, table) = set(SHEET, 1, 3, Some(Value::Str("mana".into())));
    assert_eq!(out, SHEET.replace("C;Y2;X1", "C;X3;K\"mana\"\nC;Y2;X1"));
    assert_eq!(table.text(1, 3), "mana");
    // A new row at the end, before E.
    let (out, table) = set(SHEET, 3, 1, Some(Value::Str("hkni".into())));
    assert!(out.ends_with("C;X2;K420\nC;Y3;X1;K\"hkni\"\nE\n"), "{out}");
    assert_eq!(table.text(3, 1), "hkni");
    assert_eq!(table.text(2, 2), "420");
}

#[test]
fn the_next_record_keeps_its_place() {
    // C;X2;K"hp" relies on the row of the record before it; a cell put
    // between them on another row makes it say its row.
    let src = "ID;P\nC;Y1;X1;K1\nC;X3;K3\nC;Y3;X1;K5\nE\n";
    let (out, table) = set(src, 1, 2, Some(Value::Number("2".into())));
    assert_eq!(out, "ID;P\nC;Y1;X1;K1\nC;X2;K2\nC;X3;K3\nC;Y3;X1;K5\nE\n");
    assert_eq!(
        (table.text(1, 2), table.text(1, 3)),
        ("2".into(), "3".into())
    );
    // Clearing a cell drops its record; the next one relied on row 1,
    // where the cursor starts anyway.
    let (out, table) = set(src, 1, 1, None);
    assert_eq!(out, "ID;P\nC;X3;K3\nC;Y3;X1;K5\nE\n");
    assert_eq!(table.text(1, 3), "3");
    assert_eq!(table.get(1, 1), None);
}

#[test]
fn dropping_a_record_the_next_relies_on() {
    let src = "ID;P\nC;Y2;X1;K1\nC;X2;K2\nE\n";
    let (out, table) = set(src, 2, 1, None);
    assert_eq!(out, "ID;P\nC;Y2;X2;K2\nE\n");
    assert_eq!(table.text(2, 2), "2");
    // A record with more than its place keeps it, without K.
    let src = "ID;P\nC;Y1;X1;K1;AA note\nE\n";
    let (out, table) = set(src, 1, 1, None);
    assert_eq!(out, "ID;P\nC;Y1;X1;AA note\nE\n");
    assert_eq!(table.get(1, 1).unwrap().value, Value::Empty);
}

#[test]
fn bounds_grow_and_k_is_added() {
    let (out, table) = set(SHEET, 5, 4, Some(Value::Bool(true)));
    assert!(out.starts_with("ID;PWXL;N;E\nB;Y5;X4;D0\n"), "{out}");
    assert_eq!(table.text(5, 4), "TRUE");
    // A cell with only a comment gets a K.
    let (out, table) = set(
        "ID;P\nC;Y1;X1;AA note\nE\n",
        1,
        1,
        Some(Value::Number("7".into())),
    );
    assert_eq!(out, "ID;P\nC;Y1;X1;AA note;K7\nE\n");
    assert_eq!(table.text(1, 1), "7");
}

#[test]
fn no_trailing_line_break_and_no_e() {
    let (out, table) = set("ID;P\r\nE", 1, 1, Some(Value::Number("1".into())));
    assert_eq!(out, "ID;P\r\nC;X1;K1\r\nE");
    assert_eq!(table.text(1, 1), "1");
    // Without E, the record goes at the end.
    let doc = parse("ID;P\nC;Y1;X1;K1");
    let out = apply(
        "ID;P\nC;Y1;X1;K1",
        &set_cell(&doc, 2, 1, Some(&Value::Number("2".into()))),
    );
    assert_eq!(out, "ID;P\nC;Y1;X1;K1\nC;Y2;X1;K2");
}

#[test]
fn input() {
    let n = |s: &str| Some(Value::Number(s.into()));
    assert_eq!(parse_input("12", None), n("12"));
    assert_eq!(parse_input("TRUE", None), Some(Value::Bool(true)));
    assert_eq!(parse_input("#N/A", None), Some(Value::Error("#N/A".into())));
    assert_eq!(
        parse_input("hello world", None),
        Some(Value::Str("hello world".into()))
    );
    assert_eq!(parse_input("\"12\"", None), Some(Value::Str("12".into())));
    // Text stays text.
    assert_eq!(
        parse_input("12", Some(&Value::Str("x".into()))),
        Some(Value::Str("12".into()))
    );
    assert_eq!(parse_input("", None), None);
}
