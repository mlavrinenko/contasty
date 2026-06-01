//! PHP stripping behaviour, driven by `src/lang/rules/php.yml`. Mirrors the
//! per-flag coverage of the Rust rules in `src/lang/mod_tests.rs`, exercised
//! through the public `Registry` API plus fixtures under `tests/fixtures/php`.

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
    let lang = reg.detect(Path::new("x.php")).expect("php registered");
    lang.strip(
        src,
        Path::new("x.php"),
        drop_tests,
        drop_comments,
        drop_imports,
        compact,
    )
    .expect("strip")
}

#[test]
fn php_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.php")).is_some());
    assert!(reg.detect(Path::new("foo.rs")).is_some());
    assert!(reg.detect(Path::new("foo.unknownext")).is_none());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/php/sample.php");
    let expected = include_str!("fixtures/php/sample.stripped.php");
    let out = strip(src, true, true, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn elides_function_method_and_closure_bodies() {
    let src = "<?php\n\
               function add(int $a, int $b): int { return $a + $b; }\n\
               class C {\n    \
                   public function m(): int { return 1; }\n\
               }\n\
               $f = function (): int { return 2; };\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(
        out.contains("function add(int $a, int $b): int {}"),
        "{out}"
    );
    assert!(out.contains("public function m(): int {}"), "{out}");
    assert!(out.contains("$f = function (): int {};"), "{out}");
    assert!(!out.contains("return $a + $b"), "fn body kept: {out}");
    assert!(!out.contains("return 1"), "method body kept: {out}");
    assert!(!out.contains("return 2"), "closure body kept: {out}");
}

#[test]
fn keeps_namespace_drops_use_imports() {
    let src = "<?php\n\
               namespace App\\Calc;\n\
               use App\\Contracts\\Adder;\n\
               use App\\Util\\Logger;\n\
               function keep(): void {}\n";
    let out = strip(src, false, false, true, &CompactConfig::default());
    assert!(
        out.contains("namespace App\\Calc;"),
        "namespace dropped: {out}"
    );
    assert!(out.contains("function keep"), "{out}");
    assert!(
        !out.contains("use App\\Contracts\\Adder"),
        "use kept: {out}"
    );
    assert!(!out.contains("Logger"), "use kept: {out}");
}

#[test]
fn keeps_use_imports_when_flag_off() {
    let src = "<?php\nuse App\\Util\\Logger;\nfunction keep(): void {}\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("use App\\Util\\Logger;"), "use dropped: {out}");
}

#[test]
fn drops_comments_under_flag() {
    let src = "<?php\n\
               // line comment\n\
               # hash comment\n\
               /* block comment */\n\
               /** doc block */\n\
               function keep(): void {}\n";
    let out = strip(src, false, true, false, &CompactConfig::default());
    assert!(out.contains("function keep"), "{out}");
    assert!(!out.contains("line comment"), "// kept: {out}");
    assert!(!out.contains("hash comment"), "# kept: {out}");
    assert!(!out.contains("block comment"), "/* */ kept: {out}");
    assert!(!out.contains("doc block"), "/** */ kept: {out}");
}

#[test]
fn keeps_comments_when_flag_off() {
    let src = "<?php\n// keep me\nfunction keep(): void {}\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("// keep me"), "comment dropped: {out}");
}

#[test]
fn drops_phpunit_test_class_under_flag() {
    let src = "<?php\n\
               class Calculator { public function add(): int { return 1; } }\n\
               class CalculatorTest extends TestCase {\n    \
                   public function testAdds(): void { $this->assertSame(1, 1); }\n\
               }\n";
    let out = strip(src, true, false, false, &CompactConfig::default());
    assert!(
        out.contains("class Calculator "),
        "prod class dropped: {out}"
    );
    assert!(!out.contains("CalculatorTest"), "test class kept: {out}");
    assert!(!out.contains("testAdds"), "test method kept: {out}");
}

#[test]
fn keeps_test_class_when_flag_off() {
    let src =
        "<?php\nclass CalculatorTest extends TestCase { public function testAdds(): void {} }\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("CalculatorTest"), "test class dropped: {out}");
}

#[test]
fn truncates_long_strings_above_threshold() {
    let src = "<?php\n\
               $single = 'this single-quoted string is long enough to be truncated';\n\
               $double = \"this double-quoted string is long enough to be truncated\";\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, false, false, false, &compact);
    assert!(
        !out.contains("long enough to be truncated"),
        "string kept: {out}"
    );
    assert_eq!(
        out.matches("[…CTY]").count(),
        2,
        "two strings truncated: {out}"
    );
}

#[test]
fn keeps_short_strings_below_threshold() {
    let src = "<?php\n$msg = 'hi';\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 256,
    };
    let out = strip(src, false, false, false, &compact);
    assert!(out.contains("'hi'"), "short string truncated: {out}");
    assert!(!out.contains("[…CTY]"), "marker emitted: {out}");
}
