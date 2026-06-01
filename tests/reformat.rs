//! Post-strip reformatter behaviour, exercised through the public `Registry`
//! API: shell-out and (feature-gated) embedded Topiary backends, config-error
//! surfacing, and graceful degradation when a formatter is absent or fails.
//!
//! Shell-out cases that need a real formatter use `pretty-php` (PHP) and are
//! skipped when it is absent, so the suite never depends on a formatter being
//! installed. Failure paths use a guaranteed-absent program.

use std::path::Path;
use std::process::Command;

use contasty::Registry;
use contasty::config::{Config, LangConfig, Reformat, ReformatMode};

fn cmd(parts: &[&str]) -> Reformat {
    Reformat::Command {
        command: parts.iter().map(|part| (*part).to_owned()).collect(),
    }
}

fn config_with(lang: &str, reformat: Option<Reformat>, no_reformat: bool) -> Config {
    let mut config = Config {
        no_reformat,
        ..Config::default()
    };
    config.languages.insert(
        lang.to_owned(),
        LangConfig {
            reformat,
            ..LangConfig::default()
        },
    );
    config
}

fn strip(config: &Config, ext: &str, src: &str) -> Result<String, contasty::AppError> {
    let registry = Registry::with_config(config)?;
    let name = format!("x.{ext}");
    let path = Path::new(&name);
    let lang = registry.detect(path).expect("language registered");
    lang.strip(src, path, true, true, true, &config.compact)
}

fn expect_config_error(config: &Config) -> contasty::AppError {
    match Registry::with_config(config) {
        Err(err) => err,
        Ok(_) => panic!("expected a config error, got a registry"),
    }
}

fn tool_present(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success())
}

const PHP_SRC: &str = include_str!("fixtures/php/sample.php");

#[test]
fn unknown_language_reformat_is_config_error() {
    let config = config_with("javascript", Some(cmd(&["cat"])), false);
    let err = expect_config_error(&config);
    assert!(
        matches!(err, contasty::AppError::Config(_)),
        "expected config error, got {err:?}"
    );
}

#[test]
fn empty_reformat_command_is_config_error() {
    let config = config_with("php", Some(cmd(&[])), false);
    let err = expect_config_error(&config);
    assert!(matches!(err, contasty::AppError::Config(_)), "{err:?}");
}

#[test]
fn missing_formatter_falls_back_to_raw_splice() {
    let raw = strip(&config_with("php", None, false), "php", PHP_SRC).expect("raw");
    let bogus = strip(
        &config_with("php", Some(cmd(&["contasty-no-such-formatter-xyz"])), false),
        "php",
        PHP_SRC,
    )
    .expect("degrades, not errors");
    assert_eq!(
        bogus, raw,
        "missing formatter must keep the unformatted splice"
    );
}

#[test]
fn no_reformat_skips_reformatter_resolution() {
    // An empty command is rejected when reformatting is active...
    let active = config_with("php", Some(cmd(&[])), false);
    assert!(matches!(
        expect_config_error(&active),
        contasty::AppError::Config(_)
    ));
    // ...but --no-reformat forces every language to `none`, so resolution is
    // skipped and the otherwise-invalid entry is accepted.
    let disabled = config_with("php", Some(cmd(&[])), true);
    Registry::with_config(&disabled).expect("--no-reformat must skip reformat resolution");
}

#[test]
fn no_reformat_keeps_raw_splice() {
    if !tool_present("pretty-php") {
        eprintln!("skipping: pretty-php not installed");
        return;
    }
    let raw = strip(&config_with("php", None, false), "php", PHP_SRC).expect("raw");
    let off = strip(
        &config_with("php", Some(cmd(&["pretty-php", "-"])), true),
        "php",
        PHP_SRC,
    )
    .expect("no-reformat");
    assert_eq!(
        off, raw,
        "--no-reformat must bypass the configured formatter"
    );
}

#[test]
fn shellout_reformats_php_fixture() {
    if !tool_present("pretty-php") {
        eprintln!("skipping: pretty-php not installed");
        return;
    }
    let expected = include_str!("fixtures/php/sample.reformatted.php");
    let config = config_with("php", Some(cmd(&["pretty-php", "-"])), false);
    let out = strip(&config, "php", PHP_SRC).expect("reformat");
    assert_eq!(
        out.trim_end(),
        expected.trim_end(),
        "shell-out reformat drifted from the tidy snapshot"
    );
}

#[cfg(not(feature = "topiary"))]
#[test]
fn topiary_without_feature_is_config_error() {
    let config = config_with("rust", Some(Reformat::Mode(ReformatMode::Topiary)), false);
    let err = expect_config_error(&config);
    match err {
        contasty::AppError::Config(msg) => {
            assert!(
                msg.contains("--features topiary"),
                "unhelpful message: {msg}"
            );
        }
        other => panic!("expected config error, got {other:?}"),
    }
}

#[cfg(feature = "topiary")]
#[test]
fn topiary_reformats_rust() {
    let src = "pub   fn   add(lhs:i32,rhs:i32)->i32{lhs+rhs}\n";
    let config = config_with("rust", Some(Reformat::Mode(ReformatMode::Topiary)), false);
    let out = strip(&config, "rs", src).expect("topiary reformat");
    assert!(
        out.contains("pub fn add(lhs: i32, rhs: i32) -> i32 {"),
        "topiary did not normalize rust: {out}"
    );
    assert!(
        !out.contains("pub   fn"),
        "topiary left the raw spacing: {out}"
    );
}

#[cfg(feature = "topiary")]
#[test]
fn topiary_unsupported_language_is_config_error() {
    let config = config_with("php", Some(Reformat::Mode(ReformatMode::Topiary)), false);
    let err = expect_config_error(&config);
    match err {
        contasty::AppError::Config(msg) => {
            assert!(msg.contains("Topiary"), "unhelpful message: {msg}");
        }
        other => panic!("expected config error, got {other:?}"),
    }
}
