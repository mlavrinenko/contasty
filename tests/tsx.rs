//! TSX stripping behaviour, driven by `src/lang/rules/tsx.yml`. Shares its rule
//! shape with TypeScript; this harness checks the `.tsx` extension binds and JSX
//! bodies elide.

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
    let lang = reg.detect(Path::new("x.tsx")).expect("tsx registered");
    lang.strip(
        src,
        Path::new("x.tsx"),
        drop_tests,
        drop_comments,
        drop_imports,
        compact,
    )
    .expect("strip")
}

#[test]
fn tsx_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.tsx")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/tsx/sample.tsx");
    let expected = include_str!("fixtures/tsx/sample.stripped.tsx");
    let out = strip(src, true, true, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn elides_jsx_returning_bodies() {
    let src = "function C(): JSX.Element { return <div/>; }\n\
               const B = (): JSX.Element => <span/>;\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("function C(): JSX.Element {}"), "{out}");
    assert!(out.contains("const B = (): JSX.Element => {};"), "{out}");
    assert!(!out.contains("<div"), "jsx body kept: {out}");
    assert!(!out.contains("<span"), "jsx arrow body kept: {out}");
}

#[test]
fn drops_describe_blocks_under_flag() {
    let src = "describe(\"s\", () => { it(\"w\", () => {}); });\n";
    let out = strip(src, true, false, false, &CompactConfig::default());
    assert!(!out.contains("describe"), "describe kept: {out}");
}
