# slk-rs

A parser, table model, editor and semantic highlighter for **SYLK**
(`.slk`), the spreadsheet format Warcraft III keeps its data tables in:
`Units\UnitBalance.slk`, `Units\AbilityData.slk`, the `*MetaData.slk`
tables World Editor reads its fields from, and the rest. Structured like
the other language crates; everything not SYLK-specific comes from
[`parser-rs`](../parser-rs/README.md) (see its README for the shared
design). The format reference is
[SheetJS's notes](https://oss.sheetjs.com/notes/sylk/), checked against
the game's 63 tables.

```rust
let doc = slk_rs::parse("ID;PWXL;N;E\nC;Y1;X1;K\"name\"\nC;X2;K5\nE\n");
let table = slk_rs::Table::new(&doc);
assert_eq!(table.text(1, 2), "5");

// Write a cell: the text edits that do it, applied to the source.
let edits = slk_rs::set_cell(&doc, 2, 1, Some(&slk_rs::Value::Str("hfoo".into())));
```

## The format

### Records

A file is lines, each a **record**: its type, then `;`-separated fields,
each a letter and a value.

```text
ID;PWXL;N;E                 header: the program that wrote it
B;X61;Y837;D0               the size: 61 columns, 837 rows
C;X1;Y1;K"unitBalanceID"    a cell
C;X2;K"sortBalance"
F;P0;FG0R;X3                a format, which moves the cursor too
C;K425;ER[-1]C+0            a value and the formula it came from
C;X4;AA comment\x1B :line 2 a comment
E                           the end
```

- **`;;`** is a `;` inside a value: `K"a;;b"` is `a;b`.
- **Line breaks** are `\r\n` or `\n`; written files keep the file's own.
- **The first record is `ID`**, the last **`E`**. Nothing may follow `E`.
- **Record types**: `ID` header, `B` bounds, `C` cell, `F` format, `P`
  style or number format, `O` options, `NN` name, `W`, `NE`, `NU`, `NL`,
  `E` end. Readers skip records and fields they don't know, so any other
  type is kept and not checked.

### The cursor

Records are read in order with a cursor, starting at row 1, column 1.
**`Y` (row) and `X` (column) of `C` and `F` records move it**, and a field
left out keeps the row or column it had: `C;X2;K5` is in the row of the
record before it. A `C` record sets the cell under the cursor. So adding
or dropping a record can move every cell after it; `set_cell` takes care
of that.

`B;Y..;X..` gives the sheet's size. Excel ignores it; Warcraft III may
not, so a cell outside it is a lint warning.

### Cell values

`C;K` holds the value:

| written                  | value                         |
|--------------------------|-------------------------------|
| `K"text"`                | a string                      |
| `K12`, `K-0.5`, `K1E+3`  | a number, kept as written     |
| `KTRUE`, `KFALSE`        | a boolean                     |
| `K#VALUE!`, `K#N/A`      | an error value                |

`C;E` is the formula that computed it (R1C1 references:
`CONCATENATE(RC[-1],".mp3")`), kept as text and not evaluated: readers use
`K`. `C;A` is a comment. Excel writes a comment as a record of its own, at
the same cell as the value's; the two are one cell.

### Escapes

In strings and comments, `ESC` and two characters stand for one byte:
`ESC [\x20-\x2F][\x30-\x3F]`, high bits from the first (`- 0x20`), low
bits from the second (`- 0x30`). `ESC :` is a line break. They are decoded
in values and written for control characters. Other `ESC` sequences
(`ESC N` and a letter, for code page bytes) are kept as written: the
game's files are UTF-8 and don't use them.

## Parser status

Parsing never fails as a whole: a line with a problem is read as far as it
goes and the problem recorded in `Document::errors`. Checked:

- the file starts with `ID` and ends with `E`, with nothing after it;
- a record has a type;
- `X` and `Y` of `C`, `F` and `B` are whole numbers, of `C` and `F` at
  least 1;
- a cell value is a number, a closed string, `TRUE`, `FALSE` or an error.

Every one of the game's 63 tables parses without a problem and without a
lint warning.

### Lint

| rule                  | finds |
|-----------------------|-------|
| `duplicate-cell`      | a cell whose value or formula two records set; the later wins |
| `cell-outside-bounds` | a cell past the size `B` gives |

### The table

`Table::new(&doc)` reads the cells with the cursor: `get(row, col)`,
`text(row, col)`, `cells()`, `rows()`, `cols()`, `bounds`. Rows and
columns count from 1.

### Editing

`set_cell(&doc, row, col, value)` gives the text edits that write a cell
(`Some`) or clear it (`None`); `apply(src, &edits)` applies them. An editor
keeps the file as text and changes only what it must:

- a cell with a record gets its `K` replaced, or added;
- a new cell gets a `C` record in reading order, before the first cell
  after it (or `E`), naming its row only when the cursor is on another;
- clearing drops `K`, or the whole record when that's all it had;
- the next record that moves the cursor gets the `X` or `Y` it relied on
  when a record before it is added or dropped;
- a cell past `B` grows it.

`parse_input(text, previous)` reads what a person typed into a cell:
`TRUE` / `FALSE`, `#ERROR`, a number, or text; a cell that holds text
keeps holding text, and `"quoted"` input is always text.

Checked over the game's tables: 4,890 cells written and cleared (used
ones, empty ones, past the edges), each changing that cell and nothing
else, with no parse errors after.

### Not supported

- Evaluating formulas: `E` is kept as text.
- Formats and styles (`F`, `P`) are records like any other; nothing reads
  them.
- `ESC N` code page escapes.

### Speed

`Units\UnitBalance.slk` (565 KB, 51,000 cells): scanning 43 ms, parsing
72 ms, the table 16 ms, lint 20 ms, highlighting 95 ms (release build). The
scanner allocates a string per token; that is the first thing to make
faster.

## Highlighting

Straight from the scanner, so it works on any file, broken or not: record
types and `TRUE` / `FALSE` `keyword`, `;` `operator`, field letters
`property`, numbers `number`, strings and other values `string`, error
values `enumMember`, formulas `function`, comments `comment`.

## CLI

```text
slk-rs [--ast|--dot|--html|--table] <file.slk>
  (no flag)  check the file and print its problems
  --ast      print the records as an indented tree
  --dot      print the records as a Graphviz DOT graph
  --html     print a self-contained viewer page (source + tree)
  --table    print the sheet, tab-separated, a row per line
```

## Tests

```sh
cargo test
SLK_CORPUS=path/to/game/files cargo test --release --test corpus -- --ignored
UPDATE_SNAPSHOTS=1 cargo test --test html_snapshot   # after an intended change
```

The corpus test reads every `.slk` under the folder: each must parse with
no problems and no lint, and writing and clearing a spread of cells must
change those cells and nothing else.
