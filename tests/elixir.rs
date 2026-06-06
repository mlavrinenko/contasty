//! Elixir stripping behaviour, driven by `src/lang/rules/elixir.yml`.

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
    let lang = reg.detect(Path::new("x.ex")).expect("elixir registered");
    lang.strip(
        src,
        Path::new("x.ex"),
        drop_tests,
        drop_comments,
        drop_imports,
        true,
        compact,
    )
    .expect("strip")
}

#[test]
fn elixir_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.ex")).is_some());
    assert!(reg.detect(Path::new("foo.exs")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/elixir/sample.ex");
    let expected = include_str!("fixtures/elixir/sample.stripped.ex");
    let out = strip(src, true, true, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn keeps_def_bodies() {
    // `{}` is an empty tuple, semantically wrong as a body, so bodies are kept.
    let src = "defmodule M do\n  def add(a, b) do\n    a + b\n  end\nend\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(out.contains("a + b"), "body dropped: {out}");
}

#[test]
fn drops_module_directives_under_flag() {
    let src = "defmodule M do\n  alias Foo.Bar\n  import Integer\n  use GenServer\nend\n";
    let out = strip(src, false, false, true, &CompactConfig::default());
    assert!(!out.contains("alias"), "alias kept: {out}");
    assert!(!out.contains("import"), "import kept: {out}");
    assert!(!out.contains("use GenServer"), "use kept: {out}");
}

#[test]
fn drops_test_blocks_under_flag() {
    let src = "test \"adds\" do\n  assert true\nend\n";
    let out = strip(src, true, false, false, &CompactConfig::default());
    assert!(!out.contains("adds"), "test kept: {out}");
}

#[test]
fn drops_comments_under_flag() {
    let src = "# gone\ndefmodule M do\nend\n";
    let out = strip(src, false, true, false, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
}

#[test]
fn truncates_long_strings_above_threshold() {
    let src = "defmodule M do\n  @x \"this string is long enough to truncate\"\nend\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, false, false, false, &compact);
    assert!(!out.contains("long enough"), "string kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "string truncated: {out}");
}
