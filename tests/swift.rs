//! Swift stripping behaviour, driven by `src/lang/rules/swift.yml`.

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
    let lang = reg.detect(Path::new("x.swift")).expect("swift registered");
    lang.strip(
        src,
        Path::new("x.swift"),
        drop_tests,
        drop_comments,
        drop_imports,
        true,
        compact,
    )
    .expect("strip")
}

#[test]
fn swift_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.swift")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/swift/sample.swift");
    let expected = include_str!("fixtures/swift/sample.stripped.swift");
    let out = strip(src, true, true, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn elides_function_and_init_bodies_keeps_class() {
    let src = "class C {\n    \
                   init(s: Int) { total = s }\n    \
                   func add(_ a: Int, _ b: Int) -> Int { return a + b }\n\
               }\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("class C {"), "class dropped: {out}");
    assert!(out.contains("init(s: Int) {}"), "init body kept: {out}");
    assert!(
        out.contains("func add(_ a: Int, _ b: Int) -> Int {}"),
        "{out}"
    );
    assert!(!out.contains("total = s"), "init body kept: {out}");
    assert!(!out.contains("return a + b"), "body kept: {out}");
}

#[test]
fn drops_imports_under_flag() {
    let src = "import Foundation\nfunc keep() {}\n";
    let out = strip(src, false, false, true, &CompactConfig::default());
    assert!(!out.contains("import Foundation"), "import kept: {out}");
    assert!(out.contains("func keep"), "{out}");
}

#[test]
fn drops_comments_under_flag() {
    let src = "// gone\n/* also gone */\nfunc keep() {}\n";
    let out = strip(src, false, true, false, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
}

#[test]
fn drops_test_functions_under_flag() {
    let src = "func prod() {}\nfunc testAdds() { XCTAssertEqual(3, 3) }\n";
    let out = strip(src, true, false, false, &CompactConfig::default());
    assert!(out.contains("func prod"), "prod dropped: {out}");
    assert!(!out.contains("testAdds"), "test fn kept: {out}");
}

#[test]
fn keeps_tests_when_flag_off() {
    let src = "func testAdds() {}\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("testAdds"), "test dropped: {out}");
}

#[test]
fn truncates_long_strings_above_threshold() {
    let src = "let s = \"this string is long enough to truncate\"\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, false, false, false, &compact);
    assert!(!out.contains("long enough"), "string kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "string truncated: {out}");
}
