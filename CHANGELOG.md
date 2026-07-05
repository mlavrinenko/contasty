# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-07-05

### Changed

- Configuration moves to a namespaced `.contasty/` project directory:
  `contasty.toml` is now `.contasty/config.toml` (`git mv contasty.toml
  .contasty/config.toml` for existing projects — no fallback to the old
  path). Layered under a matching XDG global config
  (`$XDG_CONFIG_HOME/contasty/config.toml`, or
  `$HOME/.config/contasty/config.toml`), project winning on a shared key:
  `[compact]` replaces wholesale, `[strip]` is `project.or(global)`, and
  `[languages.<lang>]` entries union by key with the project entry winning
  wholesale on a shared key. A language or dynamic grammar registered once in
  the global config is available in every project. `--config` overrides only
  the project layer's path; the global layer is unaffected.
- Every `[languages.<lang>]` path (`rules`, `extend`, `override`,
  `libraryPath`, including its per-target-triple map) resolves to an absolute
  path against its own defining config file's directory at load time, so a
  grammar or rule file declared in the global config resolves correctly
  regardless of which project is being scanned.
- Query file (`*.cty.yaml`) `rules` patterns now root at the scanned
  project's working directory, not the query file's own directory, so a
  saved query under `.contasty/queries/` or the XDG global queries dir
  selects the project it is run against rather than files beside itself. An
  external `{ path }` rules file is still located relative to the query
  file's directory, but the patterns it contains root at the working
  directory too. The path-escape sandbox on external rule files and `import`
  targets is lifted — those are trusted, config-referenced machinery, and the
  walker root already guarantees every *selected* file stays under the
  working directory. `import` targets still resolve relative to the
  importing query's directory.
- `--config` flag help text now points at `.contasty/config.toml`.

### Added

- Saved queries: an argument of the form `@name` resolves to
  `<project>/.contasty/queries/<name>.cty.yaml` (then `.cty.yml`), else
  `<global>/queries/<name>.cty.yaml` (then `.yml`); first hit wins. Not found
  is an error listing every path searched. The resolved file unfolds exactly
  like a query file passed by path.

## [0.2.0] - 2026-07-02

### Changed

- Default output is now the line-numbered `lines` format: a bare relative-path
  header per file, then each kept line as `N: <line>` at its original number,
  elided bodies left as gaps in the numbering. Built for agents — the numbers are
  the file's own, so a tool can read an elided body straight back from the gap
  instead of the whole file. The former Markdown output moves behind
  `--format=markdown`; `--format=json` is unchanged.

### Removed

- Post-strip reformatting, wholesale: the `--no-reformat` flag, the `reformat`
  config key, the embedded Topiary backend and shell-out formatter, the `topiary`
  build feature and its dependencies, and `docs/reformatting.md`. Reformatting
  rewrote kept lines and would desync the new line numbers from the file.

### Added

- Installable agent skill at `skills/contasty/` (`SKILL.md` + `references/cli.md`
  + reproducible `evals/`). Teaches coding agents to reach for contasty when they
  need a codebase's shape — overview, public API, where something is declared —
  scoped to the question, and to open real files only when a body is needed.
  Documented in the README "Agent skill" section.

## [0.1.0] - 2026-06-06

First public release.

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
- Category model replaced with `--strip=<categories>`: a per-path, find-style
  flag over `comments`, `imports`, `tests`, `body`, `all` (alias `everything`),
  and `none`. Comma-separated; prefix a category with `!` to remove it (e.g.
  `--strip=all,!body`). Replaces the old `--include` / `--exclude` flags.
  Default: `[comments, imports, body]` — comments and imports stripped, test
  signatures kept, bodies elided. `body` is now a first-class strip category
  gated by `when: body` on elide rules.
- Query files gain a `strip:` field: a list of categories unioned with the
  CLI's active strip set (CLI adds to query).
- Configuration: `[strip]` and `[languages.<lang>]` `strip` replace the old
  `[include]` / `[languages.<lang>.include]` sections. Same layering shape
  (built-in < cross-language < per-language < CLI-per-path).
- Language-agnostic category gating: any rule (built-in or custom) declares a
  `when: comments|imports|tests` gate; the same flags and config activate it
  for every language.
- Configuration via `contasty.toml`: cross-language strip defaults under
  `[strip]`, per-language overrides under `[languages.<lang>]` `strip`, and
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

[Unreleased]: https://github.com/mlavrinenko/contasty/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/mlavrinenko/contasty/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/mlavrinenko/contasty/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/mlavrinenko/contasty/releases/tag/v0.1.0
