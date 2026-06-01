//! C stripping behaviour, driven by `src/lang/rules/c.yml`.

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
    let lang = reg.detect(Path::new("x.c")).expect("c registered");
    lang.strip(
        src,
        Path::new("x.c"),
        drop_tests,
        drop_comments,
        drop_imports,
        compact,
    )
    .expect("strip")
}

#[test]
fn c_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.c")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/c/sample.c");
    let expected = include_str!("fixtures/c/sample.stripped.c");
    let out = strip(src, true, true, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn elides_function_bodies_keeps_signature_and_struct() {
    let src =
        "struct P { int x; };\nint add(int a, int b) { return a + b; }\nint sub(int a, int b);\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("struct P { int x; }"), "struct dropped: {out}");
    assert!(out.contains("int add(int a, int b) {}"), "{out}");
    assert!(
        out.contains("int sub(int a, int b);"),
        "prototype dropped: {out}"
    );
    assert!(!out.contains("return a + b"), "body kept: {out}");
}

#[test]
fn elides_large_aggregate_initializer() {
    let src = "int small[1] = { 0 };\nint big[4] = { 1, 2, 3, 4 };\n";
    let compact = CompactConfig {
        elide_min_bytes: 10,
        max_string_bytes: 256,
    };
    let out = strip(src, false, false, false, &compact);
    assert!(out.contains("int small[1] = { 0 };"), "small elided: {out}");
    assert!(out.contains("int big[4] = {};"), "big not elided: {out}");
}

#[test]
fn drops_includes_under_flag() {
    let src = "#include <stdio.h>\nint keep(void) { return 0; }\n";
    let out = strip(src, false, false, true, &CompactConfig::default());
    assert!(!out.contains("#include"), "include kept: {out}");
    assert!(out.contains("int keep(void)"), "{out}");
}

#[test]
fn drops_comments_under_flag() {
    let src = "// gone\n/* also gone */\nint keep(void) { return 0; }\n";
    let out = strip(src, false, true, false, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
}

#[test]
fn drops_test_functions_under_flag() {
    let src = "int prod(void) { return 1; }\nvoid test_add(void) { assert(1); }\n";
    let out = strip(src, true, false, false, &CompactConfig::default());
    assert!(out.contains("int prod"), "prod dropped: {out}");
    assert!(!out.contains("test_add"), "test fn kept: {out}");
}

#[test]
fn keeps_tests_when_flag_off() {
    let src = "void test_add(void) {}\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("test_add"), "test dropped: {out}");
}

#[test]
fn truncates_long_strings_above_threshold() {
    let src = "const char *s = \"this string is long enough to truncate\";\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, false, false, false, &compact);
    assert!(!out.contains("long enough"), "string kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "string truncated: {out}");
}
