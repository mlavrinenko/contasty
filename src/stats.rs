//! Compactization statistics using tokei for line counting.

use std::fmt;

use tokei::{CodeStats, Config, LanguageType};

use crate::walk::Stripped;

/// Aggregated statistics for original vs compacted code.
pub struct StatsReport {
    pub files: usize,
    pub original: CodeStats,
    pub compacted: CodeStats,
}

fn pct_reduction(orig: usize, comp: usize) -> String {
    if orig > 0 {
        format!("{:.1}", (1.0 - comp as f64 / orig as f64) * 100.0)
    } else {
        "N/A".to_owned()
    }
}

impl fmt::Display for StatsReport {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let orig_lines = self.original.lines();
        let comp_lines = self.compacted.lines();

        writeln!(
            f,
            "{:<12} {:>10} {:>10} {:>10}",
            format!("files: {}", self.files),
            "original",
            "compacted",
            "reduction",
        )?;
        writeln!(f, "{}", "-".repeat(46))?;
        writeln!(
            f,
            "{:<12} {:>10} {:>10} {:>9}%",
            "lines",
            orig_lines,
            comp_lines,
            pct_reduction(orig_lines, comp_lines),
        )?;
        writeln!(
            f,
            "{:<12} {:>10} {:>10} {:>9}%",
            "code",
            self.original.code,
            self.compacted.code,
            pct_reduction(self.original.code, self.compacted.code),
        )?;
        writeln!(
            f,
            "{:<12} {:>10} {:>10} {:>9}%",
            "comments",
            self.original.comments,
            self.compacted.comments,
            pct_reduction(self.original.comments, self.compacted.comments),
        )?;
        writeln!(
            f,
            "{:<12} {:>10} {:>10} {:>9}%",
            "blanks",
            self.original.blanks,
            self.compacted.blanks,
            pct_reduction(self.original.blanks, self.compacted.blanks),
        )
    }
}

/// Compute compactization statistics for a list of stripped files.
#[must_use]
pub fn compute(items: &[Stripped]) -> StatsReport {
    let config = Config::default();
    let mut original = CodeStats::new();
    let mut compacted = CodeStats::new();
    let mut files = 0;

    for item in items {
        let Some(lang) = lang_from_name(item.lang_name) else {
            continue;
        };
        files += 1;
        original += lang.parse_from_str(&item.original, &config);
        compacted += lang.parse_from_str(&item.content, &config);
    }

    StatsReport {
        files,
        original,
        compacted,
    }
}

fn lang_from_name(name: &str) -> Option<LanguageType> {
    let mut chars = name.chars();
    let capitalized = match chars.next() {
        None => return None,
        Some(first) => {
            let mut s = String::with_capacity(name.len());
            s.push(first.to_ascii_uppercase());
            s.push_str(chars.as_str());
            s
        }
    };
    LanguageType::from_name(&capitalized)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn compute_reports_reduction() {
        let src = "/// doc comment\npub fn add(lhs: i32, rhs: i32) -> i32 {\n    lhs + rhs\n}\n";
        let compacted = "/// doc comment\npub fn add(lhs: i32, rhs: i32) -> i32 { /* ... */ }\n";

        let items = vec![Stripped {
            path: PathBuf::from("a.rs"),
            lang_name: "rust",
            original: src.to_owned(),
            content: compacted.to_owned(),
        }];

        let report = compute(&items);
        assert_eq!(report.files, 1);
        assert!(report.original.lines() > report.compacted.lines());
        assert_eq!(report.original.comments, report.compacted.comments);
    }

    #[test]
    fn compute_with_empty_input() {
        let report = compute(&[]);
        assert_eq!(report.files, 0);
        assert_eq!(report.original.lines(), 0);
        assert_eq!(report.compacted.lines(), 0);
    }

    #[test]
    fn display_formats_stats() {
        let src = "fn a() { 1 }\nfn b() { 2 }\n";
        let compacted = "fn a() { /* ... */ }\nfn b() { /* ... */ }\n";

        let items = vec![Stripped {
            path: PathBuf::from("x.rs"),
            lang_name: "rust",
            original: src.to_owned(),
            content: compacted.to_owned(),
        }];

        let report = compute(&items);
        let output = format!("{report}");
        assert!(output.contains("original"));
        assert!(output.contains("compacted"));
        assert!(output.contains("reduction"));
        assert!(output.contains("files:"));
        assert!(output.contains("code"));
    }
}
