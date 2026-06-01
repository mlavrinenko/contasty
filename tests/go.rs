//! Go stripping behaviour, driven by `src/lang/rules/go.yml`.

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
    let lang = reg.detect(Path::new("x.go")).expect("go registered");
    lang.strip(
        src,
        Path::new("x.go"),
        drop_tests,
        drop_comments,
        drop_imports,
        compact,
    )
    .expect("strip")
}

#[test]
fn go_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.go")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/go/sample.go");
    let expected = include_str!("fixtures/go/sample.stripped.go");
    let out = strip(src, true, true, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn elides_function_and_method_bodies_keeps_type() {
    let src = "package p\ntype T struct { x int }\n\
               func Add(a, b int) int { return a + b }\n\
               func (t *T) M() int { return t.x }\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(
        out.contains("type T struct { x int }"),
        "type dropped: {out}"
    );
    assert!(out.contains("func Add(a, b int) int {}"), "{out}");
    assert!(out.contains("func (t *T) M() int {}"), "{out}");
    assert!(!out.contains("return a + b"), "fn body kept: {out}");
    assert!(!out.contains("return t.x"), "method body kept: {out}");
}

#[test]
fn drops_imports_keeps_package_under_flag() {
    let src = "package p\nimport (\n\t\"fmt\"\n\t\"os\"\n)\nfunc Keep() {}\n";
    let out = strip(src, false, false, true, &CompactConfig::default());
    assert!(out.contains("package p"), "package dropped: {out}");
    assert!(!out.contains("\"fmt\""), "import kept: {out}");
    assert!(!out.contains("\"os\""), "import kept: {out}");
    assert!(out.contains("func Keep"), "{out}");
}

#[test]
fn drops_comments_under_flag() {
    let src = "package p\n// gone\nfunc Keep() {}\n";
    let out = strip(src, false, true, false, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
}

#[test]
fn drops_test_functions_under_flag() {
    let src = "package p\nfunc Prod() int { return 1 }\n\
               func TestAdd(t *testing.T) {}\n\
               func BenchmarkAdd(b *testing.B) {}\n";
    let out = strip(src, true, false, false, &CompactConfig::default());
    assert!(out.contains("func Prod"), "prod dropped: {out}");
    assert!(!out.contains("TestAdd"), "test fn kept: {out}");
    assert!(!out.contains("BenchmarkAdd"), "benchmark kept: {out}");
}

#[test]
fn keeps_tests_when_flag_off() {
    let src = "package p\nfunc TestAdd(t *testing.T) {}\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("TestAdd"), "test dropped: {out}");
}

#[test]
fn truncates_long_strings_above_threshold() {
    let src = "package p\nvar s = \"this string is long enough to be truncated\"\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, false, false, false, &compact);
    assert!(!out.contains("long enough"), "string kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "string truncated: {out}");
}
