//! Every `.slk` file under `$SLK_CORPUS` parses without problems, and
//! writing or clearing any cell changes that cell and nothing else:
//! `SLK_CORPUS=dir cargo test --test corpus -- --ignored`.

use std::path::{Path, PathBuf};

use slk_rs::{Table, Value, apply, lint, parse, set_cell};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            files(&path, out);
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("slk"))
        {
            out.push(path);
        }
    }
}

/// Every cell of `table` as (row, col, value, comment).
fn cells(table: &Table) -> Vec<((u32, u32), Value, Option<String>)> {
    table
        .cells()
        .map(|(at, c)| (at, c.value.clone(), c.comment.clone()))
        .collect()
}

/// Sets the cell and checks the result: no errors, and the table is the
/// old one with only that cell changed.
fn check(path: &Path, src: &str, row: u32, col: u32, value: Option<Value>) {
    let doc = parse(src);
    let before = Table::new(&doc);
    let out = apply(src, &set_cell(&doc, row, col, value.as_ref()));
    let after_doc = parse(&out);
    assert_eq!(
        after_doc.errors,
        vec![],
        "{}: setting ({row}, {col})",
        path.display()
    );
    let after = Table::new(&after_doc);
    let mut want = cells(&before);
    let old = before.get(row, col);
    want.retain(|(at, _, _)| *at != (row, col));
    let comment = old.and_then(|c| c.comment.clone());
    match &value {
        Some(v) => want.push(((row, col), v.clone(), comment)),
        None if comment.is_some() || old.is_some_and(|c| c.formula.is_some()) => {
            want.push(((row, col), Value::Empty, comment))
        }
        None => {}
    }
    want.sort_by_key(|(at, _, _)| *at);
    assert!(
        cells(&after) == want,
        "{}: setting ({row}, {col}) to {value:?} changed other cells",
        path.display()
    );
}

#[test]
#[ignore]
fn corpus() {
    let dir = std::env::var("SLK_CORPUS").expect("set SLK_CORPUS to a folder of .slk files");
    let mut paths = Vec::new();
    files(Path::new(&dir), &mut paths);
    assert!(!paths.is_empty(), "no .slk files in {dir}");
    let mut edits = 0;
    for path in &paths {
        let src = String::from_utf8_lossy(&std::fs::read(path).unwrap()).into_owned();
        let doc = parse(&src);
        assert_eq!(doc.errors, vec![], "{}", path.display());
        assert_eq!(lint(&doc), vec![], "{}", path.display());
        let table = Table::new(&doc);
        let (rows, cols) = (table.rows().max(1), table.cols().max(1));
        // A spread of cells, used and empty, and just past the edges.
        let used: Vec<(u32, u32)> = table.cells().map(|(at, _)| at).collect();
        let mut spots: Vec<(u32, u32)> =
            used.iter().copied().step_by(used.len() / 20 + 1).collect();
        spots.extend([
            (1, 1),
            (rows / 2 + 1, cols / 2 + 1),
            (rows, cols),
            (rows + 1, 1),
            (1, cols + 1),
            (rows + 2, cols + 3),
        ]);
        for (row, col) in spots {
            check(path, &src, row, col, Some(Value::Str("x;y\nz".into())));
            check(path, &src, row, col, Some(Value::Number("42".into())));
            check(path, &src, row, col, None);
            edits += 3;
        }
    }
    eprintln!("{} files, {edits} edits", paths.len());
}
