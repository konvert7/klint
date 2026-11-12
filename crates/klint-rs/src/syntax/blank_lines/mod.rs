mod after;
mod before;
mod csharp;
mod grammar;
mod items;
mod javascript;
mod keyword;
mod lines;
mod python;
mod rust;
mod setup;
mod swift;

use crate::syntax::SourceLanguage;
use after::after_findings;
use before::before_findings;
use grammar::grammar_for;
use items::block_items;
use lines::SourceLines;
use tree_sitter::Node;

pub(crate) use grammar::is_statement_container;
pub(crate) use keyword::{Keyword, KeywordSet};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct BlankLineFinding {
    pub line: usize,
    pub insertion_line: usize,
    pub keyword: &'static str,
}

pub(crate) fn missing_blank_before(
    blocks: &[Node<'_>],
    content: &str,
    language: SourceLanguage,
    targets: KeywordSet,
    allow_setup: bool,
) -> Vec<BlankLineFinding> {
    if targets == KeywordSet::default() {
        return Vec::new();
    }

    let lines = SourceLines::new(content);
    let grammar = grammar_for(language);
    let mut findings: Vec<BlankLineFinding> = blocks
        .iter()
        .flat_map(|block| {
            let items = block_items(*block, grammar, &lines);
            before_findings(
                &items,
                &lines,
                grammar,
                content.as_bytes(),
                targets,
                allow_setup,
            )
        })
        .collect();
    findings.sort_by_key(|finding| finding.line);
    findings
}

pub(crate) fn missing_blank_after(
    blocks: &[Node<'_>],
    content: &str,
    language: SourceLanguage,
    targets: KeywordSet,
) -> Vec<BlankLineFinding> {
    if targets == KeywordSet::default() {
        return Vec::new();
    }

    let lines = SourceLines::new(content);
    let grammar = grammar_for(language);
    let mut findings: Vec<BlankLineFinding> = blocks
        .iter()
        .flat_map(|block| {
            after_findings(
                &block_items(*block, grammar, &lines),
                &lines,
                grammar,
                targets,
            )
        })
        .collect();
    findings.sort_by_key(|finding| finding.line);
    findings
}
