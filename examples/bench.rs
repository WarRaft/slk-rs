//! Times each stage on a file: `cargo run --release --example bench -- <file.slk>`.
use std::time::Instant;

fn time<T>(name: &str, runs: u32, mut f: impl FnMut() -> T) -> T {
    let mut out = f();
    let t = Instant::now();
    for _ in 0..runs {
        out = f();
    }
    println!(
        "{name:<12} {:>8.2} ms",
        t.elapsed().as_secs_f64() * 1000.0 / runs as f64
    );
    out
}

fn main() {
    let src = std::fs::read_to_string(std::env::args().nth(1).expect("a .slk file")).unwrap();
    let toks = time("tokenize", 10, || slk_rs::lexer::tokenize(&src));
    let doc = time("parse", 10, || slk_rs::parse(&src));
    let table = time("table", 10, || slk_rs::Table::new(&doc));
    time("lint", 10, || slk_rs::lint(&doc));
    let sem = time("semantic", 10, || slk_rs::semantic_tokens(&src));
    let enc = time("encoded", 10, || slk_rs::encoded_semantic_tokens(&src));
    println!(
        "{} tokens, {} cells, {} semantic, {} encoded",
        toks.len(),
        table.cells().count(),
        sem.len(),
        enc.len()
    );
}
