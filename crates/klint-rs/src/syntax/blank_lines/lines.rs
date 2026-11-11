use tree_sitter::Node;

pub(super) struct SourceLines<'s> {
    lines: Vec<&'s str>,
}

impl<'s> SourceLines<'s> {
    pub(super) fn new(content: &'s str) -> Self {
        Self {
            lines: content.split('\n').collect(),
        }
    }

    pub(super) fn is_blank(&self, row: usize) -> bool {
        self.lines
            .get(row)
            .is_some_and(|line| line.trim().is_empty())
    }

    pub(super) fn starts_line(&self, row: usize, column: usize) -> bool {
        self.lines.get(row).is_some_and(|line| {
            column <= line.len()
                && line.as_bytes()[..column]
                    .iter()
                    .all(|&byte| byte == b' ' || byte == b'\t')
        })
    }
}

pub(super) fn last_row(node: Node<'_>) -> usize {
    let end = node.end_position();
    if end.column == 0 && end.row > node.start_position().row {
        end.row - 1
    } else {
        end.row
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTENT: &str = "a();\n  \n\tif (x) {\r\n}";

    #[test]
    fn whitespace_only_line_is_blank() {
        assert!(SourceLines::new(CONTENT).is_blank(1));
    }

    #[test]
    fn code_line_is_not_blank() {
        assert!(!SourceLines::new(CONTENT).is_blank(0));
    }

    #[test]
    fn row_past_the_end_is_not_blank() {
        assert!(!SourceLines::new(CONTENT).is_blank(9));
    }

    #[test]
    fn tab_before_the_column_still_starts_the_line() {
        assert!(SourceLines::new(CONTENT).starts_line(2, 1));
    }

    #[test]
    fn code_before_the_column_does_not_start_the_line() {
        assert!(!SourceLines::new(CONTENT).starts_line(0, 4));
    }

    #[test]
    fn column_zero_starts_the_line() {
        assert!(SourceLines::new(CONTENT).starts_line(0, 0));
    }
}
