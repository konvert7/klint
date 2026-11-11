use super::fixtures_cs::*;
use super::fixtures_swift_rs::*;
use super::fixtures_ts_py::*;

pub struct Lang {
    pub file: &'static str,
    pub before_report: &'static str,
    pub before_lines: &'static [u64],
    pub before_clean: &'static str,
    pub after_report: &'static str,
    pub after_lines: &'static [u64],
    pub after_clean: &'static str,
}

pub const LANGS: [Lang; 5] = [
    Lang {
        file: "a.ts",
        before_report: TS_BEFORE_REPORT,
        before_lines: &[2, 4, 7, 10, 13, 16, 19, 21, 28],
        before_clean: TS_BEFORE_CLEAN,
        after_report: TS_AFTER_REPORT,
        after_lines: &[3, 8, 10, 13, 16, 20, 25],
        after_clean: TS_AFTER_CLEAN,
    },
    Lang {
        file: "a.py",
        before_report: PY_BEFORE_REPORT,
        before_lines: &[2, 4, 6, 8, 11, 12, 18, 20],
        before_clean: PY_BEFORE_CLEAN,
        after_report: PY_AFTER_REPORT,
        after_lines: &[2, 6, 7, 9, 12, 16, 18],
        after_clean: PY_AFTER_CLEAN,
    },
    Lang {
        file: "a.swift",
        before_report: SWIFT_BEFORE_REPORT,
        before_lines: &[2, 4, 7, 10, 13, 16, 20, 25, 28],
        before_clean: SWIFT_BEFORE_CLEAN,
        after_report: SWIFT_AFTER_REPORT,
        after_lines: &[3, 8, 10, 13, 19, 24, 27],
        after_clean: SWIFT_AFTER_CLEAN,
    },
    Lang {
        file: "a.rs",
        before_report: RS_BEFORE_REPORT,
        before_lines: &[3, 5, 8, 11, 15, 18, 22, 25, 28],
        before_clean: RS_BEFORE_CLEAN,
        after_report: RS_AFTER_REPORT,
        after_lines: &[4, 9, 11, 14, 18, 21],
        after_clean: RS_AFTER_CLEAN,
    },
    Lang {
        file: "a.cs",
        before_report: CS_BEFORE_REPORT,
        before_lines: &[2, 4, 7, 10, 13, 16, 19, 21, 28],
        before_clean: CS_BEFORE_CLEAN,
        after_report: CS_AFTER_REPORT,
        after_lines: &[3, 8, 10, 13, 16, 21, 26],
        after_clean: CS_AFTER_CLEAN,
    },
];

pub const JS_BEFORE_FILE: &str = "a.js";
pub const TS_BEFORE: &str = TS_BEFORE_REPORT;
pub const TS_AFTER: &str = TS_AFTER_REPORT;
