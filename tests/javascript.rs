//! JavaScript stripping behaviour, driven by `src/lang/rules/javascript.yml`.

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
    let lang = reg.detect(Path::new("x.js")).expect("js registered");
    lang.strip(
        src,
        Path::new("x.js"),
        drop_tests,
        drop_comments,
        drop_imports,
        compact,
    )
    .expect("strip")
}

#[test]
fn javascript_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.js")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/javascript/sample.js");
    let expected = include_str!("fixtures/javascript/sample.stripped.js");
    let out = strip(src, true, true, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn elides_function_method_and_arrow_bodies() {
    let src = "function add(a, b) { return a + b; }\n\
               class C {\n  m() { return 1; }\n}\n\
               const f = (x) => x + 1;\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("function add(a, b) {}"), "{out}");
    assert!(out.contains("m() {}"), "{out}");
    assert!(out.contains("const f = (x) => {};"), "{out}");
    assert!(!out.contains("return a + b"), "fn body kept: {out}");
}

#[test]
fn drops_imports_under_flag() {
    let src = "import { A } from \"./a.js\";\nfunction keep() {}\n";
    let out = strip(src, false, false, true, &CompactConfig::default());
    assert!(!out.contains("import"), "import kept: {out}");
    assert!(out.contains("function keep"), "{out}");
}

#[test]
fn drops_comments_under_flag() {
    let src = "// gone\nfunction keep() {}\n";
    let out = strip(src, false, true, false, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
}

#[test]
fn drops_describe_blocks_under_flag() {
    let src = "function prod() { return 1; }\ndescribe(\"s\", () => {});\n";
    let out = strip(src, true, false, false, &CompactConfig::default());
    assert!(out.contains("function prod"), "prod dropped: {out}");
    assert!(!out.contains("describe"), "describe kept: {out}");
}

#[test]
fn truncates_long_strings_above_threshold() {
    let src = "const s = \"this string is long enough to be truncated\";\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, false, false, false, &compact);
    assert!(!out.contains("long enough"), "string kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "string truncated: {out}");
}
