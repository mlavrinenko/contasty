//! Dart stripping behaviour, driven by `src/lang/rules/dart.yml`.

use std::path::Path;

use contasty::Registry;
use contasty::config::CompactConfig;

fn strip(
    src: &str,
    drop_tests: bool,
    drop_comments: bool,
    drop_imports: bool,
    compact: &CompactConfig,
) -> String {
    let reg = Registry::new().expect("registry init");
    let lang = reg.detect(Path::new("x.dart")).expect("dart registered");
    lang.strip(
        src,
        Path::new("x.dart"),
        drop_tests,
        drop_comments,
        drop_imports,
        true,
        compact,
    )
    .expect("strip")
}

#[test]
fn dart_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.dart")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/dart/sample.dart");
    let expected = include_str!("fixtures/dart/sample.stripped.dart");
    let out = strip(src, true, true, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn elides_brace_bodies_keeps_arrow_and_signature() {
    let src = "class C {\n  int add(int a, int b) {\n    return a + b;\n  }\n  String g() => \"hi\";\n}\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("int add(int a, int b) {}"), "{out}");
    assert!(!out.contains("return a + b"), "body kept: {out}");
    assert!(
        out.contains("String g() => \"hi\";"),
        "arrow dropped: {out}"
    );
}

#[test]
fn drops_imports_under_flag() {
    let src = "import 'dart:math';\nexport 'src/x.dart';\nvoid main() {}\n";
    let out = strip(src, false, false, true, &CompactConfig::default());
    assert!(!out.contains("dart:math"), "import kept: {out}");
    assert!(!out.contains("src/x.dart"), "export kept: {out}");
    assert!(out.contains("void main"), "{out}");
}

#[test]
fn drops_test_blocks_under_flag() {
    let src = "void main() {\n  test('adds', () {});\n  prod();\n}\n";
    let out = strip(src, true, false, false, &CompactConfig::default());
    assert!(!out.contains("adds"), "test kept: {out}");
}

#[test]
fn drops_comments_under_flag() {
    let src = "// gone\n/// doc gone\nvoid main() {}\n";
    let out = strip(src, false, true, false, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
}

#[test]
fn truncates_long_strings_above_threshold() {
    let src = "var s = \"this string is long enough to truncate\";\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, false, false, false, &compact);
    assert!(!out.contains("long enough"), "string kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "string truncated: {out}");
}
