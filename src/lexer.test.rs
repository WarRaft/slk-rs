use super::*;

fn kinds(src: &str) -> Vec<Tok> {
    tokenize(src).into_iter().map(|t| t.kind).collect()
}

#[test]
fn a_cell_record() {
    let toks = tokenize("C;X2;K\"a;;b\"");
    let got: Vec<(Tok, usize, usize)> = toks
        .iter()
        .map(|t| (t.kind.clone(), t.col, t.end_col))
        .collect();
    assert_eq!(
        got,
        vec![
            (Tok::RecordType("C".into()), 1, 2),
            (Tok::Semi, 2, 3),
            (Tok::FieldName('X'), 3, 4),
            (Tok::Number("2".into()), 4, 5),
            (Tok::Semi, 5, 6),
            (Tok::FieldName('K'), 6, 7),
            (
                Tok::Str {
                    value: "a;b".into(),
                    closed: true
                },
                7,
                13
            ),
            (Tok::Eof, 13, 13),
        ]
    );
}

#[test]
fn cell_values() {
    let value = |k: &str| kinds(&format!("C;K{k}"))[3].clone();
    assert_eq!(value("12"), Tok::Number("12".into()));
    assert_eq!(value("-1.5E+3"), Tok::Number("-1.5E+3".into()));
    assert_eq!(value("TRUE"), Tok::Bool(true));
    assert_eq!(value("FALSE"), Tok::Bool(false));
    assert_eq!(value("#VALUE!"), Tok::ErrorValue("#VALUE!".into()));
    assert_eq!(
        value("\"open"),
        Tok::Str {
            value: "open".into(),
            closed: false
        }
    );
    assert_eq!(value("abc"), Tok::Text("abc".into()));
}

#[test]
fn values_by_field() {
    let toks = kinds("C;Y3;X1;K1;ECONCATENATE(RC[-1],\".mp3\");AHi\x1B :there");
    assert!(toks.contains(&Tok::Formula("CONCATENATE(RC[-1],\".mp3\")".into())));
    assert!(toks.contains(&Tok::Comment("Hi\nthere".into())));
    // X / Y of other records, and other fields, are text.
    assert_eq!(kinds("P;PGeneral")[3], Tok::Text("General".into()));
    assert_eq!(kinds("F;Xa")[3], Tok::Text("a".into()));
    // A flag field has no value.
    assert_eq!(
        kinds("ID;N;E"),
        vec![
            Tok::RecordType("ID".into()),
            Tok::Semi,
            Tok::FieldName('N'),
            Tok::Semi,
            Tok::FieldName('E'),
            Tok::Eof
        ]
    );
}

#[test]
fn lines_and_the_end() {
    let toks = kinds("ID;P\r\n\r\nE\r\nleft over\n");
    assert_eq!(
        toks,
        vec![
            Tok::RecordType("ID".into()),
            Tok::Semi,
            Tok::FieldName('P'),
            Tok::Newline,
            Tok::Newline,
            Tok::RecordType("E".into()),
            Tok::Newline,
            Tok::AfterEnd("left over".into()),
            Tok::Newline,
            Tok::Eof,
        ]
    );
}

#[test]
fn numbers() {
    for n in ["0", "12", "-3", "+3", "1.5", ".5", "5.", "1e9", "1E-3"] {
        assert!(is_number(n), "{n}");
    }
    for n in ["", "-", ".", "e5", "1e", "1.2.3", "0x10", "1,5"] {
        assert!(!is_number(n), "{n}");
    }
}

#[test]
fn escapes() {
    assert_eq!(decode("a\x1B :b\x1B#;c"), "a\nb;c");
    assert_eq!(decode("\x1BNj"), "\x1BNj");
    assert_eq!(encode("a;b\nc\td"), "a;;b\x1B :c\x1B 9d");
    let s = "x;y\nz\r";
    assert_eq!(decode(&encode(s).replace(";;", ";")), s);
}
