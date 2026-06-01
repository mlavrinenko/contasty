# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `--include=<SEL>` / `--exclude=<SEL>`: ordered, repeatable flags replacing
  the old three booleans. Selectors: `comments`, `imports`, `tests`, `all`
  (alias `everything`). Flags are processed left to right across both, so
  `--exclude=all --include=comments` excludes all categories then re-includes
  comments.
- `[include]` config section: cross-language category defaults
  (`comments`, `imports`, `tests` as booleans). Config loads first; CLI overrides
  config globally.
- `[languages.<lang>.include]` config sub-section: per-language category
  overrides, applied between cross-language defaults and the CLI.
- Category gating is language-agnostic: any rule file (built-in or custom) can
  declare a `when: tests|comments|imports` gate; the same CLI/config flags
  activate it regardless of language.

### Changed

- All per-language config is consolidated under `[languages.<lang>]`. The
  separate `[customLanguages.<lang>]` (dynamic grammars) and `[rules.<lang>]`
  (rule extend/override) tables are gone; their fields (`libraryPath`,
  `languageSymbol`, `metaVarChar`, `expandoChar`, `extensions`, `rules`,
  `extend`, `override`) now live directly in `[languages.<lang>]` alongside
  `include`. Breaking config change: a `libraryPath` marks the entry as a
  dynamic grammar; `extend` / `override` point any language at a user rule file.

### Removed

- `--include-tests`, `--include-comments`, `--no-imports` (replaced by the new
  ordered `--include`/`--exclude` interface).
- `[customLanguages]` and `[rules]` top-level config tables (merged into
  `[languages.<lang>]`).

## [0.1.0] - 2026-06-01

First public release.

### Added

- `contasty <PATH>` CLI: a `.gitignore`-aware directory walker that parses each
  supported source file and renders a single context bundle. Defaults to the
  current directory; accepts a single file or a directory. Files are parsed and
  stripped in parallel with rayon.
- ast-grep matching engine: each language is driven by a YAML rule file
  (`src/lang/rules/<lang>.yml`, embedded at build time) whose rules select AST
  nodes and map to `elide` / `delete` / `truncate` actions over a byte-range
  splicer. Adding a built-in is a rule file plus a `Registry::new` entry — no
  per-language matching logic in Rust. The full ast-grep rule grammar
  (`kind`, `pattern`, `regex`, `inside`, `has`, `all`/`any`/`not`, ...) is
  available.
- Rust stripping: elide `fn` bodies, `const` / `static` value expressions, and
  `type` alias right-hand sides; output is reformatted via prettyplease.
- PHP support as a rules-only language: elide function/method/closure bodies,
  truncate long string literals, drop comments / imports / PHPUnit `*Test`
  classes per flag, keep namespaces.
- String truncation: literals longer than `max_string_bytes` (default 256) are
  replaced with a truncation marker. Blank-line runs are collapsed.
- `min-bytes` thresholds: `elide-min` (`elide_min_bytes`, default 80) keeps
  small const/static/type values intact; `max-string` gates truncation. Both
  are configurable from `contasty.toml`.
- Test elision: `#[test]` functions and `#[cfg(test)]` modules (with adjacent
  attributes) are dropped by default. `--include=tests` keeps them.
- Comment elision: every comment is dropped by default. `--include=comments`
  keeps them (doc and non-doc share the selector).
- `--exclude=imports`: drop every `use` declaration (kept by default).
- `--format=json`: emit a pretty-printed JSON bundle
  (`{ base, files: [{ path, lang, content }] }`) instead of Markdown.
- `--stats`: print original vs compacted line counts (code, comments, blanks)
  via tokei instead of the stripped output.
- `--config <path>`: select a `contasty.toml`; otherwise the one in the current
  directory is used.
- Dynamic `.so` grammars: register a native tree-sitter grammar ast-grep does
  not bundle under `[customLanguages]` in `contasty.toml`, bound to a rule file
  — no rebuild. Missing or incompatible libraries fail with an actionable error.
- User-extensible rules: `[rules.<lang>]` points a built-in or dynamic language
  at a user rule file that either `extend`s (appends to) or `override`s
  (replaces) its embedded rule set, with no rebuild.
- JSON Schema (Draft 2020-12) for rule files at
  `schemas/contasty-rules.schema.json`, derived from the Rust config types and
  composing ast-grep's `SerializableRule` schema. Generated via
  `just gen-schema` and drift-guarded by a test. Shipped rule files carry a
  `yaml-language-server` modeline; editor wiring is documented for Zed.
- Markdown renderer with relative paths in per-file headers.

[Unreleased]: https://github.com/mlavrinenko/contasty/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/mlavrinenko/contasty/releases/tag/v0.1.0
