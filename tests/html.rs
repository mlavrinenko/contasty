//! HTML stripping behaviour, driven by `src/lang/rules/html.yml`.

use std::path::Path;

use contasty::Registry;
use contasty::config::CompactConfig;

fn strip(src: &str, drop_comments: bool, compact: &CompactConfig) -> String {
    let reg = Registry::new().expect("registry init");
    let lang = reg.detect(Path::new("x.html")).expect("html registered");
    lang.strip(
        src,
        Path::new("x.html"),
        false,
        drop_comments,
        false,
        compact,
    )
    .expect("strip")
}

#[test]
fn html_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.html")).is_some());
    assert!(reg.detect(Path::new("foo.htm")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/html/sample.html");
    let expected = include_str!("fixtures/html/sample.stripped.html");
    let out = strip(src, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn elides_script_and_style_keeps_markup() {
    let src = "<p>Hi</p>\n<script>console.log(1);</script>\n<style>a { color: red; }</style>\n";
    let out = strip(src, false, &CompactConfig::default());
    assert!(out.contains("<p>Hi</p>"), "markup dropped: {out}");
    assert!(out.contains("<script>{}</script>"), "{out}");
    assert!(out.contains("<style>{}</style>"), "{out}");
    assert!(!out.contains("console.log"), "script kept: {out}");
    assert!(!out.contains("color: red"), "style kept: {out}");
}

#[test]
fn drops_comments_under_flag() {
    let src = "<!-- gone -->\n<p>Hi</p>\n";
    let out = strip(src, true, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
    assert!(out.contains("<p>Hi</p>"), "{out}");
}
