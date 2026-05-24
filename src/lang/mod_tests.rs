
use crate::config::CompactConfig;

use super::*;

#[test]
fn splice_replaces_two_function_bodies() {
    let src = "fn a() { foo(); }\nfn b() { bar(); }\n";
    let ranges = vec![(7, 17, Action::Elide), (25, 35, Action::Elide)];
    let out = splice(src, &ranges);
    assert_eq!(out, "fn a() { /* ... */ }\nfn b() { /* ... */ }\n");
}

#[test]
fn splice_with_no_ranges_returns_source() {
    let src = "hello world";
    assert_eq!(splice(src, &[]), src);
}

#[test]
fn splice_drops_overlapping_ranges() {
    let src = "abcdef";
    let out = splice(src, &[(1, 4, Action::Elide), (2, 5, Action::Elide)]);
    assert_eq!(out, "a{ /* ... */ }ef");
}

#[test]
fn splice_handles_unsorted_input() {
    let src = "fn a() { foo(); }\nfn b() { bar(); }\n";
    let ranges = vec![(25, 35, Action::Elide), (7, 17, Action::Elide)];
    let out = splice(src, &ranges);
    assert_eq!(out, "fn a() { /* ... */ }\nfn b() { /* ... */ }\n");
}

#[test]
fn splice_delete_action_removes_range_and_trailing_newline() {
    let src = "keep\n#[cfg(test)]\nmod t {}\nkeep\n";
    // Delete the attr+mod range — bytes 5..26 covers "#[cfg(test)]\nmod t {}".
    let out = splice(src, &[(5, 26, Action::Delete)]);
    assert_eq!(out, "keep\nkeep\n");
}

#[test]
fn detect_matches_known_extension() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.rs")).is_some());
    assert!(reg.detect(Path::new("foo.py")).is_none());
    assert!(reg.detect(Path::new("noext")).is_none());
}

#[test]
fn registry_strips_a_rust_file() {
    let reg = Registry::new().expect("registry init");
    let lang = reg.detect(Path::new("x.rs")).expect("rust");
    let stripped = lang
        .strip(
            "fn add(lhs: i32, rhs: i32) -> i32 { lhs + rhs }\n",
            Path::new("x.rs"),
            false,
            false,
            &CompactConfig::default(),
        )
        .expect("strip");
    assert!(stripped.contains("fn add(lhs: i32, rhs: i32) -> i32"));
    assert!(stripped.contains("/* ... */"));
    assert!(!stripped.contains("lhs + rhs"));
}

#[test]
fn drop_tests_removes_cfg_test_module() {
    let reg = Registry::new().expect("registry init");
    let lang = reg.detect(Path::new("x.rs")).expect("rust");
    let src = "pub fn add(lhs: i32, rhs: i32) -> i32 { lhs + rhs }\n\n\
               #[cfg(test)]\n\
               mod tests {\n    \
                   use super::*;\n    \
                   #[test]\n    \
                   fn it_adds() { assert_eq!(add(1, 2), 3); }\n\
               }\n";
    let stripped = lang
        .strip(
            src,
            Path::new("x.rs"),
            true,
            false,
            &CompactConfig::default(),
        )
        .expect("strip");
    assert!(stripped.contains("pub fn add"));
    assert!(
        !stripped.contains("cfg(test)"),
        "cfg(test) attribute remained: {stripped}"
    );
    assert!(
        !stripped.contains("mod tests"),
        "test module remained: {stripped}"
    );
    assert!(
        !stripped.contains("it_adds"),
        "test fn remained: {stripped}"
    );
}

#[test]
fn keep_tests_keeps_cfg_test_module() {
    let reg = Registry::new().expect("registry init");
    let lang = reg.detect(Path::new("x.rs")).expect("rust");
    let src = "pub fn add(lhs: i32, rhs: i32) -> i32 { lhs + rhs }\n\n\
               #[cfg(test)]\n\
               mod tests {\n    \
                   #[test]\n    \
                   fn it_adds() { assert_eq!(add(1, 2), 3); }\n\
               }\n";
    let stripped = lang
        .strip(
            src,
            Path::new("x.rs"),
            false,
            false,
            &CompactConfig::default(),
        )
        .expect("strip");
    assert!(stripped.contains("mod tests"));
    assert!(stripped.contains("fn it_adds"));
    assert!(stripped.contains("/* ... */"));
}

#[test]
fn drop_tests_removes_top_level_test_function() {
    let reg = Registry::new().expect("registry init");
    let lang = reg.detect(Path::new("x.rs")).expect("rust");
    let src = "pub fn keep() {}\n\n#[test]\nfn freestanding() { assert!(true); }\n";
    let stripped = lang
        .strip(
            src,
            Path::new("x.rs"),
            true,
            false,
            &CompactConfig::default(),
        )
        .expect("strip");
    assert!(stripped.contains("pub fn keep"));
    assert!(!stripped.contains("freestanding"));
    assert!(!stripped.contains("#[test]"));
}

#[test]
fn drop_tests_absorbs_other_attributes_on_the_test_module() {
    let reg = Registry::new().expect("registry init");
    let lang = reg.detect(Path::new("x.rs")).expect("rust");
    let src = "pub fn keep() {}\n\n\
               #[cfg(test)]\n\
               #[allow(clippy::unwrap_used)]\n\
               mod tests {\n    \
                   fn helper() {}\n\
               }\n";
    let stripped = lang
        .strip(
            src,
            Path::new("x.rs"),
            true,
            false,
            &CompactConfig::default(),
        )
        .expect("strip");
    assert!(stripped.contains("pub fn keep"));
    assert!(
        !stripped.contains("mod tests"),
        "test mod remained: {stripped}"
    );
    assert!(
        !stripped.contains("allow(clippy::unwrap_used)"),
        "orphan attribute remained: {stripped}",
    );
}

#[test]
fn drop_comments_removes_line_block_and_doc_comments() {
    let reg = Registry::new().expect("registry init");
    let lang = reg.detect(Path::new("x.rs")).expect("rust");
    let src = "// regular line comment\n\
               /// outer doc\n\
               //! inner doc\n\
               /* block */\n\
               /** outer block doc */\n\
               /*! inner block doc */\n\
               pub fn keep() {}\n";
    let stripped = lang
        .strip(
            src,
            Path::new("x.rs"),
            false,
            true,
            &CompactConfig::default(),
        )
        .expect("strip");
    assert!(stripped.contains("pub fn keep"));
    assert!(
        !stripped.contains("regular line comment"),
        "line comment remained: {stripped}"
    );
    assert!(
        !stripped.contains("outer doc"),
        "/// doc comment remained: {stripped}"
    );
    assert!(
        !stripped.contains("inner doc"),
        "//! doc comment remained: {stripped}"
    );
    assert!(
        !stripped.contains("block"),
        "/* */ block comment remained: {stripped}"
    );
    assert!(
        !stripped.contains("outer block doc"),
        "/** */ block doc remained: {stripped}"
    );
    assert!(
        !stripped.contains("inner block doc"),
        "/*! */ block doc remained: {stripped}"
    );
}

#[test]
fn keep_comments_keeps_everything() {
    let reg = Registry::new().expect("registry init");
    let lang = reg.detect(Path::new("x.rs")).expect("rust");
    let src = "/// doc\npub fn keep() {}\n// trailing\n";
    let stripped = lang
        .strip(
            src,
            Path::new("x.rs"),
            false,
            false,
            &CompactConfig::default(),
        )
        .expect("strip");
    assert!(stripped.contains("/// doc"));
    assert!(stripped.contains("// trailing"));
}
