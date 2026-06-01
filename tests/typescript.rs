//! TypeScript stripping behaviour, driven by `src/lang/rules/typescript.yml`.
//! Mirrors the PHP harness: golden fixture plus per-category coverage through
//! the public `Registry` API.

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
    let lang = reg.detect(Path::new("x.ts")).expect("ts registered");
    lang.strip(
        src,
        Path::new("x.ts"),
        drop_tests,
        drop_comments,
        drop_imports,
        compact,
    )
    .expect("strip")
}

#[test]
fn typescript_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.ts")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/typescript/sample.ts");
    let expected = include_str!("fixtures/typescript/sample.stripped.ts");
    let out = strip(src, true, true, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn elides_function_method_and_arrow_bodies() {
    let src = "function add(a: number, b: number): number { return a + b; }\n\
               class C {\n  m(): number { return 1; }\n}\n\
               const f = (x: number): number => x + 1;\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(
        out.contains("function add(a: number, b: number): number {}"),
        "{out}"
    );
    assert!(out.contains("m(): number {}"), "{out}");
    assert!(
        out.contains("const f = (x: number): number => {};"),
        "{out}"
    );
    assert!(!out.contains("return a + b"), "fn body kept: {out}");
    assert!(!out.contains("return 1"), "method body kept: {out}");
}

#[test]
fn drops_imports_keeps_exports_under_flag() {
    let src = "import { A } from \"./a\";\nexport function keep(): void {}\n";
    let out = strip(src, false, false, true, &CompactConfig::default());
    assert!(!out.contains("import"), "import kept: {out}");
    assert!(
        out.contains("export function keep"),
        "export dropped: {out}"
    );
}

#[test]
fn keeps_imports_when_flag_off() {
    let src = "import { A } from \"./a\";\nfunction keep(): void {}\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(
        out.contains("import { A } from \"./a\";"),
        "import dropped: {out}"
    );
}

#[test]
fn drops_comments_under_flag() {
    let src = "// line comment\n/* block */\nfunction keep(): void {}\n";
    let out = strip(src, false, true, false, &CompactConfig::default());
    assert!(out.contains("function keep"), "{out}");
    assert!(!out.contains("line comment"), "// kept: {out}");
    assert!(!out.contains("block"), "/* */ kept: {out}");
}

#[test]
fn keeps_comments_when_flag_off() {
    let src = "// keep me\nfunction keep(): void {}\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("// keep me"), "comment dropped: {out}");
}

#[test]
fn drops_describe_it_blocks_under_flag() {
    let src = "function prod(): number { return 1; }\n\
               describe(\"s\", () => { it(\"w\", () => {}); });\n\
               test(\"t\", () => {});\n";
    let out = strip(src, true, false, false, &CompactConfig::default());
    assert!(out.contains("function prod"), "prod dropped: {out}");
    assert!(!out.contains("describe"), "describe kept: {out}");
    assert!(!out.contains("test("), "test kept: {out}");
}

#[test]
fn keeps_tests_when_flag_off() {
    let src = "describe(\"s\", () => {});\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("describe"), "describe dropped: {out}");
}

#[test]
fn truncates_long_strings_above_threshold() {
    let src = "const s = \"this string is long enough to be truncated\";\n\
               const t = `this template is long enough to be truncated`;\n";
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
fn elides_large_object_initializer() {
    let src = "const small = { a: 1 };\nconst big = { a: 1, b: 2, c: 3, d: 4 };\n";
    let compact = CompactConfig {
        elide_min_bytes: 12,
        max_string_bytes: 256,
    };
    let out = strip(src, false, false, false, &compact);
    assert!(
        out.contains("const small = { a: 1 };"),
        "small elided: {out}"
    );
    assert!(out.contains("const big = {};"), "big not elided: {out}");
}
