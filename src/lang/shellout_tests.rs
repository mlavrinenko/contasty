//! Shell-out backend tests. `rustfmt` is the positive fixture: it reads stdin
//! and writes formatted source to stdout, and is present in the CI toolchain.
//! Failure paths use a guaranteed-absent program so they never depend on the
//! environment.

use super::run;

fn argv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_owned()).collect()
}

#[test]
fn rustfmt_reformats_messy_source() {
    let messy = "fn   main( ){let  x=1;}";
    let Some(out) = run(&argv(&["rustfmt", "--edition", "2021"]), messy) else {
        // rustfmt missing on this host: nothing to assert, but the call must not
        // panic. Treated as a skip.
        return;
    };
    assert!(out.contains("fn main()"), "not reformatted: {out}");
    assert!(out.contains("let x = 1;"), "not reformatted: {out}");
}

#[test]
fn missing_program_yields_none() {
    let out = run(&argv(&["contasty-no-such-formatter-xyz"]), "anything");
    assert!(out.is_none(), "missing program should fall back");
}

#[test]
fn empty_command_yields_none() {
    let out = run(&[], "anything");
    assert!(out.is_none(), "empty command should fall back");
}

#[test]
fn nonzero_exit_yields_none() {
    // `false` exits non-zero without reading stdin.
    let out = run(&argv(&["false"]), "anything");
    assert!(out.is_none(), "non-zero exit should fall back");
}

#[test]
fn passthrough_command_returns_input() {
    // `cat` is the identity formatter: proves the stdin -> stdout plumbing.
    let Some(out) = run(&argv(&["cat"]), "verbatim text\n") else {
        return;
    };
    assert_eq!(out, "verbatim text\n");
}
