use std::env;
use std::fs;
use std::process::ExitCode;

use slk_rs::{Level, Table, lint, render_dot, render_html, render_tree};

enum Mode {
    Lint,
    Ast,
    Dot,
    Html,
    Table,
}

fn usage() {
    eprintln!("usage: slk-rs [--ast|--dot|--html|--table] <file.slk>");
    eprintln!("  (no flag)  check the file and print its problems");
    eprintln!("  --ast      print the records as an indented tree");
    eprintln!("  --dot      print the records as a Graphviz DOT graph");
    eprintln!("  --html     print a self-contained viewer page (source + tree)");
    eprintln!("  --table    print the sheet, tab-separated, a row per line");
}

fn main() -> ExitCode {
    let mut mode = Mode::Lint;
    let mut path = None;
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--ast" => mode = Mode::Ast,
            "--dot" => mode = Mode::Dot,
            "--html" => mode = Mode::Html,
            "--table" => mode = Mode::Table,
            "-h" | "--help" => {
                usage();
                return ExitCode::SUCCESS;
            }
            other => path = Some(other.to_string()),
        }
    }
    let Some(path) = path else {
        usage();
        return ExitCode::FAILURE;
    };
    let src = match fs::read(&path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(err) => {
            eprintln!("{path}: {err}");
            return ExitCode::FAILURE;
        }
    };

    // Parsing never fails as a whole.
    let doc = slk_rs::parse(&src);
    match mode {
        Mode::Ast => print!("{}", render_tree(&doc)),
        Mode::Dot => print!("{}", render_dot(&doc)),
        Mode::Html => print!("{}", render_html(&doc, &src, &path)),
        Mode::Table => {
            use std::io::Write;
            let table = Table::new(&doc);
            let mut out = std::io::stdout().lock();
            for row in 1..=table.rows() {
                let cells: Vec<String> = (1..=table.cols())
                    .map(|col| table.text(row, col).replace(['\t', '\n'], " "))
                    .collect();
                // A closed pipe (`| head`) just ends the output.
                if writeln!(out, "{}", cells.join("\t")).is_err() {
                    break;
                }
            }
        }
        Mode::Lint => {
            let diagnostics = lint(&doc);
            for err in &doc.errors {
                println!("{path}:{}:{}: error: {}", err.line, err.col, err.message);
            }
            for diag in &diagnostics {
                println!(
                    "{path}:{}: {} [{}]: {}",
                    diag.line,
                    diag.level.as_str(),
                    diag.rule,
                    diag.message
                );
            }
            if doc.errors.is_empty() && diagnostics.is_empty() {
                println!("{path}: no issues found");
            }
            if !doc.errors.is_empty() || diagnostics.iter().any(|d| d.level == Level::Error) {
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}
