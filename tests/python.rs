//! Python stripping behaviour, driven by `src/lang/rules/python.yml`.

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
    let lang = reg.detect(Path::new("x.py")).expect("py registered");
    lang.strip(
        src,
        Path::new("x.py"),
        drop_tests,
        drop_comments,
        drop_imports,
        compact,
    )
    .expect("strip")
}

#[test]
fn python_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.py")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/python/sample.py");
    let expected = include_str!("fixtures/python/sample.stripped.py");
    let out = strip(src, true, true, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn elides_function_and_method_bodies_keeps_class() {
    let src = "class C:\n    def m(self) -> int:\n        return 1\n\
               def f(a: int) -> int:\n    return a\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("class C:"), "class dropped: {out}");
    assert!(
        out.contains("def m(self) -> int:"),
        "method sig dropped: {out}"
    );
    assert!(
        out.contains("def f(a: int) -> int:"),
        "fn sig dropped: {out}"
    );
    assert!(!out.contains("return 1"), "method body kept: {out}");
    assert!(!out.contains("return a"), "fn body kept: {out}");
}

#[test]
fn drops_imports_under_flag() {
    let src = "import os\nfrom typing import List\ndef keep() -> None:\n    pass\n";
    let out = strip(src, false, false, true, &CompactConfig::default());
    assert!(!out.contains("import os"), "import kept: {out}");
    assert!(!out.contains("from typing"), "from-import kept: {out}");
    assert!(out.contains("def keep"), "{out}");
}

#[test]
fn drops_comments_keeps_docstrings() {
    let src = "# gone\ndef keep() -> None:\n    \"keepdoc\"\n    pass\n";
    let out = strip(src, false, true, false, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
}

#[test]
fn drops_test_functions_and_classes_under_flag() {
    let src = "def prod() -> int:\n    return 1\n\
               def test_adds():\n    assert True\n\
               class TestThing:\n    def test_m(self):\n        assert True\n";
    let out = strip(src, true, false, false, &CompactConfig::default());
    assert!(out.contains("def prod"), "prod dropped: {out}");
    assert!(!out.contains("test_adds"), "test fn kept: {out}");
    assert!(!out.contains("TestThing"), "test class kept: {out}");
}

#[test]
fn drops_decorated_test_function_with_decorator() {
    let src = "@deco\ndef test_x():\n    assert True\n";
    let out = strip(src, true, false, false, &CompactConfig::default());
    assert!(!out.contains("@deco"), "orphan decorator kept: {out}");
    assert!(!out.contains("test_x"), "test fn kept: {out}");
}

#[test]
fn keeps_tests_when_flag_off() {
    let src = "def test_x():\n    assert True\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("test_x"), "test dropped: {out}");
}

#[test]
fn truncates_long_strings_above_threshold() {
    let src = "s = \"this string is long enough to be truncated\"\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, false, false, false, &compact);
    assert!(!out.contains("long enough"), "string kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "string truncated: {out}");
}

#[test]
fn elides_large_dict_initializer() {
    let src = "small = {\"a\": 1}\nbig = {\"a\": 1, \"b\": 2, \"c\": 3, \"d\": 4}\n";
    let compact = CompactConfig {
        elide_min_bytes: 12,
        max_string_bytes: 256,
    };
    let out = strip(src, false, false, false, &compact);
    assert!(out.contains("small = {\"a\": 1}"), "small elided: {out}");
    assert!(out.contains("big = {}"), "big not elided: {out}");
}
