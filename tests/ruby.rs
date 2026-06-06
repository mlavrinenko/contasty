//! Ruby stripping behaviour, driven by `src/lang/rules/ruby.yml`.

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
    let lang = reg.detect(Path::new("x.rb")).expect("ruby registered");
    lang.strip(
        src,
        Path::new("x.rb"),
        drop_tests,
        drop_comments,
        drop_imports,
        true,
        compact,
    )
    .expect("strip")
}

#[test]
fn ruby_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.rb")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/ruby/sample.rb");
    let expected = include_str!("fixtures/ruby/sample.stripped.rb");
    let out = strip(src, true, true, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn elides_method_bodies_keeps_class_and_signature() {
    let src =
        "class C\n  def add(a, b)\n    a + b\n  end\n  def self.zero\n    new(0)\n  end\nend\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("class C"), "class dropped: {out}");
    assert!(out.contains("def add(a, b)"), "method sig dropped: {out}");
    assert!(
        out.contains("def self.zero"),
        "singleton sig dropped: {out}"
    );
    assert!(!out.contains("a + b"), "method body kept: {out}");
    assert!(!out.contains("new(0)"), "singleton body kept: {out}");
}

#[test]
fn elides_large_hash_initializer() {
    let src = "small = { a: 1 }\nbig = { a: 1, b: 2, c: 3, d: 4 }\n";
    let compact = CompactConfig {
        elide_min_bytes: 12,
        max_string_bytes: 256,
    };
    let out = strip(src, false, false, false, &compact);
    assert!(out.contains("small = { a: 1 }"), "small elided: {out}");
    assert!(out.contains("big = {}"), "big not elided: {out}");
}

#[test]
fn drops_requires_keeps_method_calls_under_flag() {
    let src = "require \"set\"\nrequire_relative \"x\"\nobj.require \"keep\"\n";
    let out = strip(src, false, false, true, &CompactConfig::default());
    assert!(!out.contains("require \"set\""), "require kept: {out}");
    assert!(
        !out.contains("require_relative"),
        "require_relative kept: {out}"
    );
    assert!(
        out.contains("obj.require \"keep\""),
        "method call dropped: {out}"
    );
}

#[test]
fn drops_comments_under_flag() {
    let src = "# gone\ndef keep\nend\n";
    let out = strip(src, false, true, false, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
}

#[test]
fn drops_minitest_and_rspec_tests_under_flag() {
    let src = "def test_adds\n  assert true\nend\n\
               describe \"thing\" do\n  it \"works\" do\n  end\nend\n";
    let out = strip(src, true, false, false, &CompactConfig::default());
    assert!(!out.contains("test_adds"), "minitest method kept: {out}");
    assert!(!out.contains("describe"), "rspec block kept: {out}");
}

#[test]
fn keeps_tests_when_flag_off() {
    let src = "def test_adds\n  assert true\nend\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("test_adds"), "test dropped: {out}");
}

#[test]
fn truncates_long_strings_above_threshold() {
    let src = "s = \"this string is long enough to truncate\"\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, false, false, false, &compact);
    assert!(!out.contains("long enough"), "string kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "string truncated: {out}");
}
