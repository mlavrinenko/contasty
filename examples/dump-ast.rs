//! Throwaway AST dumper: `cargo run --example dump-ast -- <lang> <file>`.
//!
//! Prints the named-node tree (kind + byte range) using contasty's pinned
//! ast-grep grammar version, so rule `kind`/`field` names match what the engine
//! actually parses. Used while authoring `src/lang/rules/<lang>.yml`.

use std::str::FromStr;

use ast_grep_core::AstGrep;
use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_language::SupportLang;

fn main() {
    let mut args = std::env::args().skip(1);
    let lang = args.next().expect("usage: dump-ast <lang> <file>");
    let file = args.next().expect("usage: dump-ast <lang> <file>");
    let lang = SupportLang::from_str(&lang).expect("unknown language");
    let src = std::fs::read_to_string(&file).expect("read file");
    let grep: AstGrep<StrDoc<SupportLang>> = AstGrep::new(&src, lang);
    print_node(&grep.root(), 0, &src);
}

fn print_node(node: &ast_grep_core::Node<'_, StrDoc<SupportLang>>, depth: usize, src: &str) {
    let range = node.range();
    let text = &src[range.clone()];
    let snippet: String = text.chars().take(30).collect();
    let snippet = snippet.replace('\n', "\\n");
    // Probe the field names rule files commonly descend into and report which
    // resolve, so a rule's `field:` can be picked without guessing.
    let fields: Vec<String> = [
        "body",
        "name",
        "value",
        "consequence",
        "parameters",
        "declarator",
        "left",
        "right",
        "type",
        "condition",
        "target",
        "function",
        "operator",
        "object",
    ]
    .iter()
    .filter_map(|f| node.field(f).map(|c| format!("{f}={}", c.kind())))
    .collect();
    println!(
        "{:indent$}{} [{}..{}] {:?} {}",
        "",
        node.kind(),
        range.start,
        range.end,
        snippet,
        if fields.is_empty() {
            String::new()
        } else {
            format!("<{}>", fields.join(","))
        },
        indent = depth * 2
    );
    for child in node.children() {
        if child.is_named() {
            print_node(&child, depth + 1, src);
        }
    }
}
