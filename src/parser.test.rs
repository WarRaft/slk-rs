use super::*;
use crate::parse;

fn errors(src: &str) -> Vec<(usize, usize, String)> {
    parse(src)
        .errors
        .into_iter()
        .map(|e| (e.line, e.col, e.message))
        .collect()
}

#[test]
fn a_whole_file() {
    let doc = parse("ID;PWXL;N;E\r\nB;X2;Y1;D0\r\nC;Y1;X1;K\"a\"\r\nC;X2;K5;ESUM(R1C1)\r\nE\r\n");
    assert_eq!(doc.errors, vec![]);
    assert_eq!(doc.newline, "\r\n");
    let kinds: Vec<&str> = doc.records.iter().map(|r| r.kind.as_str()).collect();
    assert_eq!(kinds, ["ID", "B", "C", "C", "E"]);
    let c = &doc.records[3];
    assert_eq!(c.field('K').unwrap().value, Value::Number("5".into()));
    assert_eq!(
        c.field('E').unwrap().value,
        Value::Formula("SUM(R1C1)".into())
    );
    let k = c.field('K').unwrap();
    assert_eq!(
        (
            k.span.start_col,
            k.span.end_col,
            k.value_span.start_col,
            k.value_span.end_col
        ),
        (6, 8, 7, 8)
    );
    assert_eq!(doc.records[0].field('N').unwrap().value, Value::Empty);
    assert_eq!(c.full_span(), Span::new(4, 1, 4, 19));
}

#[test]
fn every_problem_is_reported() {
    let src = "C;Y1;X1;K1\n;X1\nC;Xa;K1\nC;Y0;K1\nC;Kabc\nC;K\"open\nC;K\nE\nmore\n";
    assert_eq!(
        errors(src),
        vec![
            (1, 1, "a SYLK file starts with an ID record, like ID;PWXL;N;E".into()),
            (2, 1, "missing the record type before ';'".into()),
            (3, 3, "expected the column number after X".into()),
            (4, 4, "expected the row number, 1 or more: 0".into()),
            (5, 4, "expected a cell value: a number, a \"string\", TRUE, FALSE or an error like #VALUE!, found 'abc'".into()),
            (6, 4, "string isn't closed: '\"' missing".into()),
            (7, 3, "expected the cell value after K".into()),
            (9, 1, "text after the E record (line 8), which ends the file".into()),
        ]
    );
}

#[test]
fn missing_end() {
    assert_eq!(
        errors("ID;P\nC;K1\n"),
        vec![(2, 1, "missing the E record that ends the file".into())]
    );
    assert_eq!(
        errors(""),
        vec![(1, 1, "missing the E record that ends the file".into())]
    );
}

#[test]
fn unknown_records_and_fields_are_kept() {
    let doc = parse("ID;P\nW;N1;A1 1\nC;K1;Z9\nE\n");
    assert_eq!(doc.errors, vec![]);
    assert_eq!(doc.records[1].kind, "W");
    assert_eq!(
        doc.records[2].field('Z').unwrap().value,
        Value::Text("9".into())
    );
}
