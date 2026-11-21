use crate::ast::*;
use crate::table::cursors;
pub use parser_rs::Node;

/// Builds a visualizable tree out of a parsed document: its records, each
/// with the cell it's at when it moves the cursor, and their fields.
pub fn build_tree(doc: &Document) -> Node {
    let at = cursors(doc);
    let children = doc
        .records
        .iter()
        .zip(at)
        .map(|(r, (row, col))| {
            let label = if crate::table::moves_cursor(r) {
                format!("{} (row {row}, column {col})", r.kind)
            } else {
                r.kind.clone()
            };
            let fields = r
                .fields
                .iter()
                .map(|f| {
                    Node::leaf(format!("{} = {}", f.name, field_text(&f.value)))
                        .tag("lit", Some(f.span))
                })
                .collect();
            Node::with_children(label, fields).tag("decl", Some(r.full_span()))
        })
        .collect();
    Node::with_children("Document", children).tag("root", None)
}

fn field_text(v: &Value) -> String {
    match v {
        Value::Empty => "(flag)".into(),
        Value::Str(s) => format!("{s:?}"),
        Value::Formula(s) => format!("formula {s}"),
        Value::Comment(s) => format!("comment {s:?}"),
        other => other.display(),
    }
}

/// Renders the document as an indented tree.
pub fn render_tree(doc: &Document) -> String {
    parser_rs::render_tree(&build_tree(doc))
}

/// Renders the document as a Graphviz DOT graph.
pub fn render_dot(doc: &Document) -> String {
    parser_rs::render_dot(&build_tree(doc))
}
