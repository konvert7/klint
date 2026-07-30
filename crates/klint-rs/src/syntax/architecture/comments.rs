use std::path::Path;
use tree_sitter::Node;

use crate::syntax::{SourceLanguage, source_language_for_path};

#[derive(Debug, PartialEq, Eq)]
pub struct CommentRecord {
    /// 1-based first physical line the comment occupies.
    pub start_line: usize,
    /// 1-based last physical line the comment occupies.
    pub end_line: usize,
    pub is_doc: bool,
}

/// Collects every comment node, classified as doc vs ordinary. Each grammar
/// names comments differently: `comment` in TypeScript and Python,
/// `comment`/`multiline_comment` in Swift, `line_comment`/`block_comment` in
/// Rust. Docstrings are string expressions, not comment nodes, so they never
/// appear here.
pub(crate) fn scan_comments_from_tree(
    path: &Path,
    root: Node<'_>,
    source: &[u8],
) -> Vec<CommentRecord> {
    let mut comments = Vec::new();
    walk_comments(source_language_for_path(path), root, source, &mut comments);
    comments
}

/// Rows the comment text itself covers, counted from its first row. Rust
/// doc-comment nodes extend past their last character to swallow the newline,
/// so the node's own end row would credit them with a line they do not occupy.
fn trailing_line_span(text: &str) -> usize {
    text.trim_end().lines().count().saturating_sub(1)
}

fn walk_comments(
    language: SourceLanguage,
    node: Node<'_>,
    source: &[u8],
    comments: &mut Vec<CommentRecord>,
) {
    if matches!(
        node.kind(),
        "comment" | "multiline_comment" | "line_comment" | "block_comment"
    ) {
        let text = node.utf8_text(source).unwrap_or("");
        let start_line = node.start_position().row + 1;
        comments.push(CommentRecord {
            start_line,
            end_line: start_line + trailing_line_span(text),
            is_doc: is_doc_comment(language, node, text),
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_comments(language, child, source, comments);
    }
}

/// A `/** */` JSDoc block, but not the empty `/**/` comment. Mirrors the TS
/// engine. Swift also documents with `///`, where TypeScript reserves triple
/// slashes for `/// <reference>` directives rather than documentation. Rust
/// marks `///` and `//!` in the tree itself, so its doc comments are read off
/// the node rather than guessed from the text.
fn is_doc_comment(language: SourceLanguage, node: Node<'_>, text: &str) -> bool {
    match language {
        SourceLanguage::Rust => has_rust_doc_marker(node),
        SourceLanguage::Swift if text.starts_with("///") => true,
        _ => text.starts_with("/**") && text != "/**/",
    }
}

fn has_rust_doc_marker(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).any(|child| {
        matches!(
            child.kind(),
            "outer_doc_comment_marker" | "inner_doc_comment_marker"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::language_for_path;
    use std::path::Path;
    use tree_sitter::Parser;

    fn comments(path: &str, content: &str) -> Vec<CommentRecord> {
        let mut parser = Parser::new();
        parser
            .set_language(&language_for_path(Path::new(path)))
            .expect("parser loads");
        let tree = parser.parse(content, None).expect("source parses");
        scan_comments_from_tree(Path::new(path), tree.root_node(), content.as_bytes())
    }

    #[test]
    fn classifies_doc_and_ordinary_comments_with_line_spans() {
        assert_eq!(
            comments(
                "index.ts",
                "// line\nconst x = 1; /* inline */\n/**\n * doc\n */\n",
            ),
            vec![
                CommentRecord {
                    start_line: 1,
                    end_line: 1,
                    is_doc: false,
                },
                CommentRecord {
                    start_line: 2,
                    end_line: 2,
                    is_doc: false,
                },
                CommentRecord {
                    start_line: 3,
                    end_line: 5,
                    is_doc: true,
                },
            ]
        );
    }

    #[test]
    fn treats_python_hash_comments_as_ordinary() {
        assert_eq!(
            comments("mod.py", "# a\nx = 1\n"),
            vec![CommentRecord {
                start_line: 1,
                end_line: 1,
                is_doc: false,
            }]
        );
    }

    #[test]
    fn ignores_the_empty_block_comment() {
        assert_eq!(
            comments("index.ts", "/**/\n"),
            vec![CommentRecord {
                start_line: 1,
                end_line: 1,
                is_doc: false,
            }]
        );
    }

    #[test]
    fn classifies_swift_block_and_triple_slash_comments() {
        assert_eq!(
            comments("View.swift", "/*\n * block\n */\n/// doc\n// plain\n"),
            vec![
                CommentRecord {
                    start_line: 1,
                    end_line: 3,
                    is_doc: false,
                },
                CommentRecord {
                    start_line: 4,
                    end_line: 4,
                    is_doc: true,
                },
                CommentRecord {
                    start_line: 5,
                    end_line: 5,
                    is_doc: false,
                },
            ]
        );
    }

    #[test]
    fn classifies_rust_line_block_and_doc_comments() {
        assert_eq!(
            comments(
                "lib.rs",
                "//! inner\n/// doc\n// plain\n/*\n * block\n */\n"
            ),
            vec![
                CommentRecord {
                    start_line: 1,
                    end_line: 1,
                    is_doc: true,
                },
                CommentRecord {
                    start_line: 2,
                    end_line: 2,
                    is_doc: true,
                },
                CommentRecord {
                    start_line: 3,
                    end_line: 3,
                    is_doc: false,
                },
                CommentRecord {
                    start_line: 4,
                    end_line: 6,
                    is_doc: false,
                },
            ]
        );
    }
}
