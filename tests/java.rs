//! Java stripping behaviour, driven by `src/lang/rules/java.yml`.

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
    let lang = reg.detect(Path::new("x.java")).expect("java registered");
    lang.strip(
        src,
        Path::new("x.java"),
        drop_tests,
        drop_comments,
        drop_imports,
        true,
        compact,
    )
    .expect("strip")
}

#[test]
fn java_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.java")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/java/sample.java");
    let expected = include_str!("fixtures/java/sample.stripped.java");
    let out = strip(src, true, true, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn elides_method_constructor_and_lambda_bodies_keeps_class() {
    let src = "class C {\n    \
                   C(int t) { this.total = t; }\n    \
                   int add(int a, int b) { return a + b; }\n    \
                   Runnable r = () -> { run(); };\n\
               }\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("class C {"), "class dropped: {out}");
    assert!(out.contains("C(int t) {}"), "ctor body kept: {out}");
    assert!(out.contains("int add(int a, int b) {}"), "{out}");
    assert!(
        out.contains("Runnable r = () -> {};"),
        "lambda body kept: {out}"
    );
    assert!(!out.contains("this.total = t"), "ctor body kept: {out}");
    assert!(!out.contains("return a + b"), "method body kept: {out}");
}

#[test]
fn drops_imports_keeps_package_under_flag() {
    let src = "package app;\nimport java.util.List;\nclass C {}\n";
    let out = strip(src, false, false, true, &CompactConfig::default());
    assert!(out.contains("package app;"), "package dropped: {out}");
    assert!(!out.contains("import java.util.List"), "import kept: {out}");
}

#[test]
fn drops_comments_under_flag() {
    let src = "// gone\n/* also gone */\nclass C {}\n";
    let out = strip(src, false, true, false, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
}

#[test]
fn drops_test_method_and_class_under_flag() {
    let src = "class C {\n    @Test\n    void adds() { assertEquals(3, 3); }\n}\n\
               class CalcTest {}\n";
    let out = strip(src, true, false, false, &CompactConfig::default());
    assert!(!out.contains("@Test"), "test method kept: {out}");
    assert!(!out.contains("void adds"), "test method kept: {out}");
    assert!(!out.contains("CalcTest"), "test class kept: {out}");
}

#[test]
fn keeps_tests_when_flag_off() {
    let src = "class C {\n    @Test\n    void adds() {}\n}\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("@Test"), "test dropped: {out}");
    assert!(out.contains("void adds"), "test dropped: {out}");
}

#[test]
fn truncates_long_strings_above_threshold() {
    let src = "class C { String s = \"this string is long enough to truncate\"; }\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, false, false, false, &compact);
    assert!(!out.contains("long enough"), "string kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "string truncated: {out}");
}
