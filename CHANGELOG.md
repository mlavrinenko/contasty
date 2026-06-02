# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

First public release (not yet tagged).

### Added

- `contasty [PATH]` CLI: a `.gitignore`-aware walker that parses each supported
  source file and renders one context bundle. Defaults to the current
  directory; accepts a single file or a directory. Files are parsed and
  stripped in parallel with rayon.
- ast-grep matching engine: each language is driven by a YAML rule file
  (`src/lang/rules/<lang>.yml`, embedded at build time) whose rules select AST
  nodes and map to `elide` / `delete` / `truncate` actions over a byte-range
  splicer. Adding a built-in is a rule file plus a `Registry::new` entry — no
  per-language matching logic in Rust. The full ast-grep rule grammar
  (`kind`, `pattern`, `regex`, `inside`, `has`, `all`/`any`/`not`, ...) is
  available.
- Rust stripping: elide `fn` bodies, `const` / `static` value expressions, and
  `type` alias right-hand sides.
- PHP support as a rules-only language: elide function/method/closure bodies,
  truncate long string literals, keep namespaces.
- Tier 1 built-in languages (TypeScript, TSX, JavaScript, Python, Go): each an
  embedded rule file plus a `Registry::new` entry, riding ast-grep's bundled
  grammars — no `.so`, no config. Elide function/method/closure bodies, drop
  imports / comments / tests by category, truncate long strings, and elide large
  object/array/dict initializers (Go keeps initializers — the `{}` marker is no
  valid Go expression). TS/TSX/JS ship self-contained near-duplicate rule files.
- Tier 2 built-in languages (Java, C#, Ruby, C++, C, Kotlin, Swift, Scala): same
  rules-only recipe. Elide function/method/constructor/lambda bodies, drop
  imports / comments / tests by category, truncate long strings. Value-init
  elision where `{}` is valid in position — C/C++ aggregate initializers and Ruby
  hash literals; Java/C#/Kotlin/Swift/Scala keep initializers, like Go. Test
  detection spans JUnit/NUnit/xUnit attributes, GoogleTest macros, `test_*` names,
  and ScalaTest `*Spec` classes.
- Tier 3 built-in languages, completing every grammar ast-grep bundles except
  Markdown: Bash, Lua, Dart, Elixir, Haskell, Nix, Solidity, plus the data/markup
  grammars JSON, YAML, HTML, CSS, HCL. Body elision where the generic `{}` marker
  is a valid empty body — Dart and Solidity (brace bodies) and HTML's
  `<script>`/`<style>` payloads; the non-brace languages (Bash, Lua, Elixir,
  Haskell, Nix) keep bodies and strip comments, imports, and long strings. The
  data/markup grammars truncate long string/scalar values, drop comments, and
  keep keys, structure, selectors, and block labels. Markdown is intentionally
  structural-only (prose context) and ships no rule file.
- `delete` now removes a deleted node's indentation when it stands alone on its
  line, so an indented import/comment/test leaves no blank stub.
- Category model with ordered, repeatable flags: `--include=<SEL>` /
  `--exclude=<SEL>` over `comments`, `imports`, `tests`, and `all` (alias
  `everything`). Both flags are processed left to right and the last mention of
  a category wins, so `--exclude=all --include=comments` excludes everything
  then re-includes comments. Defaults: comments and tests excluded, imports
  included.
- Language-agnostic category gating: any rule (built-in or custom) declares a
  `when: comments|imports|tests` gate; the same flags and config activate it
  for every language.
- Configuration via `contasty.toml`: cross-language category defaults under
  `[include]`, per-language overrides under `[languages.<lang>.include]`, and
  compaction thresholds under `[compact]` (`elide_min_bytes`, default 80;
  `max_string_bytes`, default 256). Config loads first; CLI overrides it.
- Per-language config consolidated under `[languages.<lang>]`: a `libraryPath`
  registers a dynamic native tree-sitter grammar (with `extensions`, `rules`,
  and optional `languageSymbol` / `metaVarChar` / `expandoChar`) for a language
  ast-grep does not bundle — no rebuild; `extend` / `override` point a language
  at a user rule file to append to (`extend`) or replace (`override`) its
  embedded rules. Missing or incompatible libraries fail with an actionable
  error.
- Optional per-language post-strip reformatter via the `reformat` key of a
  `[languages.<lang>]` entry: `"none"` (default), `"topiary"` (embedded Topiary,
  behind the `topiary` build feature), or `{ command = [...] }` (shell out to an
  external formatter over stdin/stdout). No language is reformatted by default —
  the engine ships no per-language formatting dependency. A reformat failure
  (missing tool, non-zero exit, timeout, unsupported language) degrades to the
  raw splice with a warning; an unavailable Topiary selection is a clear config
  error. The `--no-reformat` flag disables all reformatting for a run. See
  `docs/reformatting.md`.
- String truncation: literals longer than `max_string_bytes` are replaced with
  a truncation marker. Blank-line runs are collapsed.
- `--format=<markdown|json>`: Markdown document (default) or a pretty-printed
  JSON bundle `{ base, files: [{ path, lang, content }] }`.
- `--stats`: print original vs compacted line counts (code, comments, blanks)
  via tokei instead of the stripped output.
- `--config <path>`: select a `contasty.toml`; otherwise the one in the current
  directory is used.
- JSON Schema (Draft 2020-12) for rule files at
  `schemas/contasty-rules.schema.json`, derived from the Rust config types and
  composing ast-grep's `SerializableRule` schema. Generated via
  `just gen-schema` and drift-guarded by a test. Shipped rule files carry a
  `yaml-language-server` modeline; editor wiring is documented for Zed.
- Markdown renderer with relative paths in per-file headers.

[Unreleased]: https://github.com/mlavrinenko/contasty/commits/main
