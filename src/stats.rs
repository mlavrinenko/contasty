//! Compactization statistics using tokei for line counting.

use std::fmt;

use tokei::{CodeStats, Config, LanguageType};

use crate::walk::Stripped;

/// Aggregated statistics for original vs compacted code.
pub struct StatsReport {
    pub files: usize,
    pub original: CodeStats,
    pub compacted: CodeStats,
    pub original_tokens: usize,
    pub compacted_tokens: usize,
}

/// Approximate token count for `text`, dependency-free.
///
/// Heuristic: `ceil(byte_length / 4)`, the common "~4 bytes per token" rule of
/// thumb for English and code under cl100k-style tokenizers. This is an estimate
/// only — it is not a model tokenizer and makes no per-model accuracy claim.
/// Properties relied on elsewhere: non-zero for non-empty input, deterministic,
/// and monotonic under concatenation.
#[must_use]
pub fn approx_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
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
        )?;
        writeln!(
            f,
            "{:<12} {:>10} {:>10} {:>9}%",
            "~tokens",
            self.original_tokens,
            self.compacted_tokens,
            pct_reduction(self.original_tokens, self.compacted_tokens),
        )?;
        writeln!(
            f,
            "~tokens: estimate (~bytes/4), not a model tokenizer count"
        )
    }
}

/// Compute compactization statistics for a list of stripped files.
#[must_use]
pub fn compute(items: &[Stripped]) -> StatsReport {
    let config = Config::default();
    let mut original = CodeStats::new();
    let mut compacted = CodeStats::new();
    let mut original_tokens = 0;
    let mut compacted_tokens = 0;
    let mut files = 0;

    for item in items {
        let Some(lang) = lang_from_name(item.lang_name) else {
            continue;
        };
        files += 1;
        original += lang.parse_from_str(&item.original, &config);
        compacted += lang.parse_from_str(&item.content, &config);
        original_tokens += approx_tokens(&item.original);
        compacted_tokens += approx_tokens(&item.content);
    }

    StatsReport {
        files,
        original,
        compacted,
        original_tokens,
        compacted_tokens,
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
        assert!(output.contains("~tokens"));
        assert!(output.contains("estimate"));
        assert!(output.contains("not a model tokenizer"));
    }

    #[test]
    fn approx_tokens_non_zero_for_non_empty() {
        assert_eq!(approx_tokens(""), 0);
        assert!(approx_tokens("a") > 0);
        assert!(approx_tokens("hello world") > 0);
    }

    #[test]
    fn approx_tokens_is_deterministic() {
        let text = "fn main() { println!(\"hi\"); }";
        assert_eq!(approx_tokens(text), approx_tokens(text));
    }

    #[test]
    fn approx_tokens_monotonic_under_concatenation() {
        let a = "first chunk of text";
        let b = "second chunk that differs";
        let joined = format!("{a}{b}");
        let ta = approx_tokens(a);
        let tb = approx_tokens(b);
        let tj = approx_tokens(&joined);
        // Concatenation never drops below either part.
        assert!(tj >= ta);
        assert!(tj >= tb);
        // And never exceeds the sum of the parts (ceil sub-additivity).
        assert!(tj <= ta + tb);
    }

    #[test]
    fn compute_reports_token_reduction() {
        let src = "/// doc comment\npub fn add(lhs: i32, rhs: i32) -> i32 {\n    lhs + rhs\n}\n";
        let compacted = "pub fn add(lhs: i32, rhs: i32) -> i32 { /* ... */ }\n";

        let items = vec![Stripped {
            path: PathBuf::from("a.rs"),
            lang_name: "rust",
            original: src.to_owned(),
            content: compacted.to_owned(),
        }];

        let report = compute(&items);
        assert_eq!(report.original_tokens, approx_tokens(src));
        assert_eq!(report.compacted_tokens, approx_tokens(compacted));
        assert!(report.original_tokens > report.compacted_tokens);
    }
}
