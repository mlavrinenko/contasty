//! HCL / Terraform stripping behaviour, driven by `src/lang/rules/hcl.yml`.

use std::path::Path;

use contasty::Registry;
use contasty::config::CompactConfig;

fn strip(src: &str, drop_comments: bool, compact: &CompactConfig) -> String {
    let reg = Registry::new().expect("registry init");
    let lang = reg.detect(Path::new("x.hcl")).expect("hcl registered");
    lang.strip(
        src,
        Path::new("x.hcl"),
        false,
        drop_comments,
        false,
        compact,
    )
    .expect("strip")
}

#[test]
fn hcl_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.hcl")).is_some());
    assert!(reg.detect(Path::new("foo.tf")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/hcl/sample.hcl");
    let expected = include_str!("fixtures/hcl/sample.stripped.hcl");
    let out = strip(src, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn truncates_long_strings_keeps_block_and_labels() {
    let src = "resource \"aws_instance\" \"web\" {\n  user_data = \"this value is long enough to truncate\"\n}\n";
    // Threshold above the short block labels but below the long value, so labels
    // stay and only the value truncates.
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 20,
    };
    let out = strip(src, false, &compact);
    assert!(out.contains("\"aws_instance\""), "label truncated: {out}");
    assert!(out.contains("\"web\""), "label truncated: {out}");
    assert!(!out.contains("long enough"), "value kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "value truncated: {out}");
}

#[test]
fn drops_comments_under_flag() {
    let src = "# gone\nresource \"x\" \"y\" {}\n";
    let out = strip(src, true, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
}
