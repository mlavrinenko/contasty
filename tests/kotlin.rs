//! Kotlin stripping behaviour, driven by `src/lang/rules/kotlin.yml`.

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
    let lang = reg.detect(Path::new("x.kt")).expect("kotlin registered");
    lang.strip(
        src,
        Path::new("x.kt"),
        drop_tests,
        drop_comments,
        drop_imports,
        compact,
    )
    .expect("strip")
}

#[test]
fn kotlin_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.kt")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/kotlin/sample.kt");
    let expected = include_str!("fixtures/kotlin/sample.stripped.kt");
    let out = strip(src, true, true, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn elides_block_and_expression_bodies_keeps_class() {
    let src = "class C {\n    \
                   fun add(a: Int, b: Int): Int { return a + b }\n    \
                   fun square(x: Int) = x * x\n\
               }\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("class C {"), "class dropped: {out}");
    assert!(out.contains("fun add(a: Int, b: Int): Int {}"), "{out}");
    assert!(
        out.contains("fun square(x: Int) {}"),
        "expr body kept: {out}"
    );
    assert!(!out.contains("return a + b"), "block body kept: {out}");
    assert!(!out.contains("x * x"), "expr body kept: {out}");
}

#[test]
fn drops_imports_keeps_package_under_flag() {
    let src = "package app\nimport kotlin.math.abs\nfun keep() {}\n";
    let out = strip(src, false, false, true, &CompactConfig::default());
    assert!(out.contains("package app"), "package dropped: {out}");
    assert!(
        !out.contains("import kotlin.math.abs"),
        "import kept: {out}"
    );
}

#[test]
fn drops_comments_under_flag() {
    let src = "// gone\n/* also gone */\nfun keep() {}\n";
    let out = strip(src, false, true, false, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
}

#[test]
fn drops_test_function_under_flag() {
    let src = "fun prod() {}\n@Test\nfun adds() { assertEquals(3, 3) }\n";
    let out = strip(src, true, false, false, &CompactConfig::default());
    assert!(out.contains("fun prod"), "prod dropped: {out}");
    assert!(!out.contains("@Test"), "annotation kept: {out}");
    assert!(!out.contains("fun adds"), "test fn kept: {out}");
}

#[test]
fn keeps_tests_when_flag_off() {
    let src = "@Test\nfun adds() {}\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("fun adds"), "test dropped: {out}");
}

#[test]
fn truncates_long_strings_above_threshold() {
    let src = "val s = \"this string is long enough to truncate\"\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, false, false, false, &compact);
    assert!(!out.contains("long enough"), "string kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "string truncated: {out}");
}
