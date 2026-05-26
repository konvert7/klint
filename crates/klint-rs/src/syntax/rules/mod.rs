mod consecutive_array_push;
mod nested_template_literals;
mod prefer_at;
mod prefer_string_raw_regexp;
mod prefer_string_replaceall;
mod single_char_class;
mod string_match;
mod sync_in_async;
mod unguarded_json_parse;

pub use consecutive_array_push::scan_consecutive_array_push;
pub use nested_template_literals::scan_nested_template_literals;
pub use prefer_at::scan_prefer_at;
pub use prefer_string_raw_regexp::scan_prefer_string_raw_regexp;
pub use prefer_string_replaceall::scan_prefer_string_replaceall;
pub use single_char_class::scan_single_char_classes;
pub use string_match::scan_string_match;
pub use sync_in_async::scan_sync_in_async;
pub use unguarded_json_parse::scan_unguarded_json_parse;

use super::{language_for_path, raw_node_text};

#[derive(Debug, PartialEq, Eq)]
pub struct StringMatchRecord {
    pub line: usize,
    pub receiver: String,
    pub regex: String,
    pub start_row: usize,
    pub end_row: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct NestedTemplateLiteralRecord {
    pub line: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ConsecutiveArrayPushRecord {
    pub line: usize,
    pub count: usize,
    pub receiver: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct UnguardedJsonParseRecord {
    pub line: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SyncInAsyncRecord {
    pub line: usize,
    pub name: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SingleCharClassRecord {
    pub line: usize,
    pub class: String,
    pub fixed_regex: String,
    pub start_row: usize,
    pub end_row: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PreferAtRecord {
    pub line: usize,
    pub base: String,
    pub offset: i64,
    pub start_row: usize,
    pub end_row: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PreferStringReplaceAllRecord {
    pub line: usize,
    pub receiver: String,
    pub pattern: String,
    pub pattern_lit: String,
    pub replacement: String,
    pub start_row: usize,
    pub end_row: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PreferStringRawRegexpRecord {
    pub line: usize,
    pub fixed_arg: String,
    pub start_row: usize,
    pub end_row: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}
