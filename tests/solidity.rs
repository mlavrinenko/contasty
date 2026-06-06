//! Solidity stripping behaviour, driven by `src/lang/rules/solidity.yml`.

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
    let lang = reg.detect(Path::new("x.sol")).expect("solidity registered");
    lang.strip(
        src,
        Path::new("x.sol"),
        drop_tests,
        drop_comments,
        drop_imports,
        true,
        compact,
    )
    .expect("strip")
}

#[test]
fn solidity_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.sol")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/solidity/sample.sol");
    let expected = include_str!("fixtures/solidity/sample.stripped.sol");
    let out = strip(src, true, true, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn elides_bodies_keeps_signature_and_state() {
    let src = "contract C {\n    uint256 public total;\n    function add(uint256 a) public pure returns (uint256) {\n        return a;\n    }\n}\n";
    let out = strip(src, false, false, false, &CompactConfig::default());
    assert!(
        out.contains("uint256 public total;"),
        "state dropped: {out}"
    );
    assert!(
        out.contains("function add(uint256 a) public pure returns (uint256) {}"),
        "{out}"
    );
    assert!(!out.contains("return a;"), "body kept: {out}");
}

#[test]
fn drops_imports_under_flag() {
    let src = "import \"./Other.sol\";\ncontract C {}\n";
    let out = strip(src, false, false, true, &CompactConfig::default());
    assert!(!out.contains("Other.sol"), "import kept: {out}");
    assert!(out.contains("contract C"), "{out}");
}

#[test]
fn drops_test_functions_under_flag() {
    let src = "contract C {\n    function testAdd() public {}\n    function prod() public {}\n}\n";
    let out = strip(src, true, false, false, &CompactConfig::default());
    assert!(!out.contains("testAdd"), "test kept: {out}");
    assert!(out.contains("prod"), "prod dropped: {out}");
}

#[test]
fn drops_comments_under_flag() {
    let src = "// gone\ncontract C {}\n";
    let out = strip(src, false, true, false, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
}

#[test]
fn truncates_long_strings_above_threshold() {
    let src = "contract C {\n    string s = \"this string is long enough to truncate\";\n}\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, false, false, false, &compact);
    assert!(!out.contains("long enough"), "string kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "string truncated: {out}");
}
