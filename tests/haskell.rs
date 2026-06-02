//! Haskell stripping behaviour, driven by `src/lang/rules/haskell.yml`.

use std::path::Path;

use contasty::Registry;
use contasty::config::CompactConfig;

fn strip(src: &str, drop_comments: bool, drop_imports: bool, compact: &CompactConfig) -> String {
    let reg = Registry::new().expect("registry init");
    let lang = reg.detect(Path::new("x.hs")).expect("haskell registered");
    lang.strip(
        src,
        Path::new("x.hs"),
        false,
        drop_comments,
        drop_imports,
        compact,
    )
    .expect("strip")
}

#[test]
fn haskell_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.hs")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/haskell/sample.hs");
    let expected = include_str!("fixtures/haskell/sample.stripped.hs");
    let out = strip(src, true, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn keeps_signatures_and_bodies() {
    // No brace body to elide; signatures and equations stay.
    let src = "add :: Int -> Int -> Int\nadd a b = a + b\n";
    let out = strip(src, false, false, &CompactConfig::default());
    assert!(
        out.contains("add :: Int -> Int -> Int"),
        "sig dropped: {out}"
    );
    assert!(out.contains("add a b = a + b"), "body dropped: {out}");
}

#[test]
fn drops_imports_under_flag() {
    let src = "import Data.List\nadd a b = a + b\n";
    let out = strip(src, false, true, &CompactConfig::default());
    assert!(!out.contains("Data.List"), "import kept: {out}");
    assert!(out.contains("add a b"), "{out}");
}

#[test]
fn drops_comments_under_flag() {
    let src = "-- gone\nadd a b = a + b\n";
    let out = strip(src, true, false, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
}

#[test]
fn truncates_long_strings_above_threshold() {
    let src = "banner = \"this string is long enough to truncate\"\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, false, false, &compact);
    assert!(!out.contains("long enough"), "string kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "string truncated: {out}");
}
