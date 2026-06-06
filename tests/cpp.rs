//! C++ stripping behaviour, driven by `src/lang/rules/cpp.yml`.

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
    let lang = reg.detect(Path::new("x.cpp")).expect("cpp registered");
    lang.strip(
        src,
        Path::new("x.cpp"),
        drop_tests,
        drop_comments,
        drop_imports,
        true,
        compact,
    )
    .expect("strip")
}

#[test]
fn cpp_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.cpp")).is_some());
    assert!(reg.detect(Path::new("foo.cc")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/cpp/sample.cpp");
    let expected = include_str!("fixtures/cpp/sample.stripped.cpp");
    let out = strip(src, true, true, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn elides_function_method_and_lambda_bodies_keeps_namespace() {
    let src = "namespace ns {\n\
               int add(int a, int b) { return a + b; }\n\
               auto sq = [](int x) { return x * x; };\n\
               }\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("namespace ns {"), "namespace dropped: {out}");
    assert!(out.contains("int add(int a, int b) {}"), "{out}");
    assert!(
        out.contains("auto sq = [](int x) {};"),
        "lambda body kept: {out}"
    );
    assert!(!out.contains("return a + b"), "body kept: {out}");
}

#[test]
fn keeps_constructor_member_init_list() {
    let src = "class C { int t; public: C(int s) : t(s) { log(); } };\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(
        out.contains("C(int s) : t(s) {}"),
        "init list dropped: {out}"
    );
    assert!(!out.contains("log()"), "body kept: {out}");
}

#[test]
fn drops_includes_under_flag() {
    let src = "#include <vector>\nint keep() { return 0; }\n";
    let out = strip(src, false, false, true, &CompactConfig::default());
    assert!(!out.contains("#include"), "include kept: {out}");
}

#[test]
fn drops_comments_under_flag() {
    let src = "// gone\n/* also gone */\nint keep() { return 0; }\n";
    let out = strip(src, false, true, false, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
}

#[test]
fn drops_googletest_cases_under_flag() {
    let src = "int prod() { return 1; }\nTEST(Suite, Adds) { EXPECT_EQ(3, 3); }\n\
               TEST_F(Fix, Subs) { EXPECT_EQ(1, 1); }\n";
    let out = strip(src, true, false, false, &CompactConfig::default());
    assert!(out.contains("int prod"), "prod dropped: {out}");
    assert!(!out.contains("TEST(Suite, Adds)"), "TEST kept: {out}");
    assert!(!out.contains("TEST_F"), "TEST_F kept: {out}");
}

#[test]
fn keeps_tests_when_flag_off() {
    let src = "TEST(Suite, Adds) {}\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("TEST(Suite, Adds)"), "test dropped: {out}");
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
