//! C# stripping behaviour, driven by `src/lang/rules/csharp.yml`.

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
    let lang = reg.detect(Path::new("x.cs")).expect("csharp registered");
    lang.strip(
        src,
        Path::new("x.cs"),
        drop_tests,
        drop_comments,
        drop_imports,
        compact,
    )
    .expect("strip")
}

#[test]
fn csharp_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.cs")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/csharp/sample.cs");
    let expected = include_str!("fixtures/csharp/sample.stripped.cs");
    let out = strip(src, true, true, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn elides_block_bodies_keeps_expression_bodied_member() {
    let src = "class C {\n    \
                   public C(int t) { total = t; }\n    \
                   public int Add(int a, int b) { return a + b; }\n    \
                   public int Sq(int x) => x * x;\n\
               }\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("public C(int t) {}"), "ctor body kept: {out}");
    assert!(out.contains("public int Add(int a, int b) {}"), "{out}");
    assert!(
        out.contains("public int Sq(int x) => x * x;"),
        "expression body elided: {out}"
    );
}

#[test]
fn drops_usings_keeps_namespace_under_flag() {
    let src = "using System;\nnamespace App;\nclass C {}\n";
    let out = strip(src, false, false, true, &CompactConfig::default());
    assert!(out.contains("namespace App;"), "namespace dropped: {out}");
    assert!(!out.contains("using System"), "using kept: {out}");
}

#[test]
fn drops_comments_under_flag() {
    let src = "// gone\n/* also gone */\nclass C {}\n";
    let out = strip(src, false, true, false, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
}

#[test]
fn drops_test_method_and_class_under_flag() {
    let src = "class C {\n    [Fact]\n    public void Adds() { Assert.Equal(3, 3); }\n}\n\
               public class CalcTests {}\n";
    let out = strip(src, true, false, false, &CompactConfig::default());
    assert!(!out.contains("[Fact]"), "test attr kept: {out}");
    assert!(!out.contains("Adds"), "test method kept: {out}");
    assert!(!out.contains("CalcTests"), "test class kept: {out}");
}

#[test]
fn keeps_tests_when_flag_off() {
    let src = "class C {\n    [Fact]\n    public void Adds() {}\n}\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("[Fact]"), "test dropped: {out}");
}

#[test]
fn truncates_long_strings_above_threshold() {
    let src = "class C { string s = \"this string is long enough to truncate\"; }\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, false, false, false, &compact);
    assert!(!out.contains("long enough"), "string kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "string truncated: {out}");
}
