//! Scala stripping behaviour, driven by `src/lang/rules/scala.yml`.

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
    let lang = reg.detect(Path::new("x.scala")).expect("scala registered");
    lang.strip(
        src,
        Path::new("x.scala"),
        drop_tests,
        drop_comments,
        drop_imports,
        true,
        compact,
    )
    .expect("strip")
}

#[test]
fn scala_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.scala")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/scala/sample.scala");
    let expected = include_str!("fixtures/scala/sample.stripped.scala");
    let out = strip(src, true, true, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn elides_block_and_expression_bodies_keeps_object() {
    let src = "object O {\n  \
                   def add(a: Int, b: Int): Int = { a + b }\n  \
                   def square(x: Int): Int = x * x\n\
               }\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("object O {"), "object dropped: {out}");
    assert!(out.contains("def add(a: Int, b: Int): Int = {}"), "{out}");
    assert!(
        out.contains("def square(x: Int): Int = {}"),
        "expr body kept: {out}"
    );
    assert!(!out.contains("a + b"), "block body kept: {out}");
    assert!(!out.contains("x * x"), "expr body kept: {out}");
}

#[test]
fn drops_imports_keeps_package_under_flag() {
    let src = "package app\nimport scala.collection.mutable\nobject O {}\n";
    let out = strip(src, false, false, true, &CompactConfig::default());
    assert!(out.contains("package app"), "package dropped: {out}");
    assert!(
        !out.contains("import scala.collection.mutable"),
        "import kept: {out}"
    );
}

#[test]
fn drops_comments_under_flag() {
    let src = "// gone\n/* also gone */\nobject O {}\n";
    let out = strip(src, false, true, false, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
}

#[test]
fn drops_test_classes_under_flag() {
    let src = "class Calc {}\nclass CalcSpec {}\nclass CalcTest {}\nobject CalcSuite {}\n";
    let out = strip(src, true, false, false, &CompactConfig::default());
    assert!(out.contains("class Calc {"), "prod class dropped: {out}");
    assert!(!out.contains("CalcSpec"), "spec kept: {out}");
    assert!(!out.contains("CalcTest"), "test kept: {out}");
    assert!(!out.contains("CalcSuite"), "suite kept: {out}");
}

#[test]
fn keeps_tests_when_flag_off() {
    let src = "class CalcSpec {}\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("CalcSpec"), "spec dropped: {out}");
}

#[test]
fn truncates_long_strings_above_threshold() {
    let src = "object O { val s = \"this string is long enough to truncate\" }\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, false, false, false, &compact);
    assert!(!out.contains("long enough"), "string kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "string truncated: {out}");
}
