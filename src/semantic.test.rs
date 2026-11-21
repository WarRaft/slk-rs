use super::*;

#[test]
fn every_part_has_a_color() {
    let src = "C;X1;K\"a\";ESUM(R1C1);AHi\nC;KTRUE\nC;K#N/A\n";
    let got: Vec<(usize, usize, usize, T)> = semantic_tokens(src)
        .into_iter()
        .map(|t| (t.line, t.start_col, t.end_col, t.ty))
        .collect();
    assert_eq!(
        got,
        vec![
            (1, 1, 2, T::Keyword),
            (1, 2, 3, T::Operator),
            (1, 3, 4, T::Property),
            (1, 4, 5, T::Number),
            (1, 5, 6, T::Operator),
            (1, 6, 7, T::Property),
            (1, 7, 10, T::String),
            (1, 10, 11, T::Operator),
            (1, 11, 12, T::Property),
            (1, 12, 21, T::Function),
            (1, 21, 22, T::Operator),
            (1, 22, 23, T::Property),
            (1, 23, 25, T::Comment),
            (2, 1, 2, T::Keyword),
            (2, 2, 3, T::Operator),
            (2, 3, 4, T::Property),
            (2, 4, 8, T::Keyword),
            (3, 1, 2, T::Keyword),
            (3, 2, 3, T::Operator),
            (3, 3, 4, T::Property),
            (3, 4, 8, T::EnumMember),
        ]
    );
}
