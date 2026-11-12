#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LineTotals {
    pub lines: u32,
    pub non_blank_lines: u32,
}

#[derive(Default)]
pub(crate) struct LineTally {
    totals: LineTotals,
    previous_token_end: usize,
    pending_breaks: u32,
}

pub(crate) fn line_breaks_between(source: &[u8], from: usize, to: usize) -> u32 {
    let count = source
        .get(from..to)
        .map_or(0, |gap| gap.iter().filter(|byte| **byte == b'\n').count());
    u32::try_from(count).unwrap_or(u32::MAX)
}

impl LineTotals {
    pub(crate) fn since(self, before: LineTotals) -> LineTotals {
        LineTotals {
            lines: self.lines.saturating_sub(before.lines),
            non_blank_lines: self.non_blank_lines.saturating_sub(before.non_blank_lines),
        }
    }
}

impl LineTally {
    pub(crate) fn add_token(&mut self, source: &[u8], kind: &str, start: usize, end: usize) {
        let breaks = line_breaks_between(source, self.previous_token_end, start)
            .saturating_add(std::mem::take(&mut self.pending_breaks));
        self.previous_token_end = end;

        if is_brace_kind(kind) {
            return;
        }

        self.totals.lines = self.totals.lines.saturating_add(breaks);

        if breaks > 0 {
            self.totals.non_blank_lines = self.totals.non_blank_lines.saturating_add(1);
        }
    }

    pub(crate) fn absorb_comment(&mut self, source: &[u8], start: usize, end: usize) {
        let breaks = line_breaks_between(source, self.previous_token_end, start);
        self.pending_breaks = self.pending_breaks.saturating_add(breaks);
        self.previous_token_end = self.previous_token_end.max(end);
    }

    pub(crate) fn absorb_gap_before(&mut self, start: usize) {
        self.previous_token_end = self.previous_token_end.max(start);
    }

    pub(crate) fn totals(&self) -> LineTotals {
        self.totals
    }
}

fn is_brace_kind(kind: &str) -> bool {
    kind == "{" || kind == "}"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_line_breaks_between_offsets_count_both() {
        assert_eq!(line_breaks_between(b"a\n\nb", 1, 3), 2);
    }

    #[test]
    fn a_crlf_pair_counts_as_one_line_break() {
        assert_eq!(line_breaks_between(b"a\r\nb", 1, 3), 1);
    }

    #[test]
    fn a_gap_without_a_line_break_counts_zero() {
        assert_eq!(line_breaks_between(b"a b", 1, 2), 0);
    }

    #[test]
    fn a_reversed_range_counts_zero() {
        assert_eq!(line_breaks_between(b"ab", 2, 1), 0);
    }

    #[test]
    fn a_range_past_the_end_of_the_source_counts_zero() {
        assert_eq!(line_breaks_between(b"ab", 1, 9), 0);
    }

    #[test]
    fn since_subtracts_each_field() {
        let after = LineTotals {
            lines: 5,
            non_blank_lines: 3,
        };
        let before = LineTotals {
            lines: 2,
            non_blank_lines: 1,
        };
        assert_eq!(
            after.since(before),
            LineTotals {
                lines: 3,
                non_blank_lines: 2
            }
        );
    }

    #[test]
    fn since_is_zero_when_nothing_was_added() {
        let totals = LineTotals {
            lines: 2,
            non_blank_lines: 1,
        };
        assert_eq!(totals.since(totals), LineTotals::default());
    }

    #[test]
    fn since_never_goes_below_zero() {
        let after = LineTotals {
            lines: 1,
            non_blank_lines: 0,
        };
        let before = LineTotals {
            lines: 4,
            non_blank_lines: 4,
        };
        assert_eq!(after.since(before), LineTotals::default());
    }

    #[test]
    fn open_brace_is_a_brace_kind() {
        assert!(is_brace_kind("{"));
    }

    #[test]
    fn close_brace_is_a_brace_kind() {
        assert!(is_brace_kind("}"));
    }

    #[test]
    fn template_substitution_open_is_not_a_brace_kind() {
        assert!(!is_brace_kind("${"));
    }

    #[test]
    fn statement_block_is_not_a_brace_kind() {
        assert!(!is_brace_kind("statement_block"));
    }
}
