# contasty

[![CI](https://github.com/mlavrinenko/contasty/actions/workflows/ci.yml/badge.svg)](https://github.com/mlavrinenko/contasty/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/contasty.svg)](https://crates.io/crates/contasty)
[![License: MIT](https://img.shields.io/crates/l/contasty.svg)](LICENSE-MIT)

Strips executable lines from your code to prepare tasty context for your agent.

Walks a directory (respecting `.gitignore`), parses each supported source file
with [ast-grep](https://ast-grep.github.io/), and elides function bodies,
constant values, and long string literals — keeping declarations, signatures,
and types intact. The result prints as a single Markdown document, ready to
paste into an LLM context window.

Each language is driven by a YAML rule file matched against the AST, not by
hardcoded per-language logic. Built-in support: Rust and PHP. You can add a
language with a dynamic tree-sitter grammar, and extend or override any
language's rules from `contasty.toml` — both without rebuilding contasty.

## Install

### From crates.io

```bash
cargo install contasty
```

### From binary releases

Download a pre-built binary from the
[latest release](https://github.com/mlavrinenko/contasty/releases/latest).

## Usage

```bash
contasty src/ > context.md        # strip a directory
contasty src/lib.rs               # strip a single file
contasty                          # default path is "."
contasty --include-tests src/     # keep #[test] / #[cfg(test)] items
contasty --include-comments src/  # keep every comment (doc comments included)
contasty --no-imports src/        # drop every `use` declaration
contasty --format=json src/       # emit a JSON bundle instead of Markdown
contasty --stats src/             # print compactization statistics
contasty --config path.toml src/  # use a specific contasty.toml
```

Output defaults to Markdown. Pass `--format=json` for a pretty-printed JSON
bundle shaped as `{ "base": <dir>, "files": [{ "path", "lang", "content" }] }`,
mirroring the Markdown layout.

Tests (`#[test]` functions and `#[cfg(test)]` modules) and comments (every
`//`, `///`, `//!`, `/* */`, `/** */`, `/*! */`) are dropped from the output
by default — both are noise for most context-bundle use cases. Pass
`--include-tests` and/or `--include-comments` to keep them. Import lists are
kept by default; pass `--no-imports` to shed every `use` declaration.

## Configuration

Drop a `contasty.toml` in your project root to tune compaction thresholds,
register dynamic grammars, and extend or override per-language rules. All
fields are optional. See [docs/languages.md](docs/languages.md) and
[docs/custom-rules.md](docs/custom-rules.md).

## Adding a language

contasty matches AST nodes with [ast-grep](https://ast-grep.github.io/) rules,
so a language is data, not code.

- Built-in: drop a rule file at `src/lang/rules/<lang>.yml` (embedded at build
  time) and register the language in `Registry::new`. No per-language matching
  logic in Rust.
- Dynamic grammar: for a language ast-grep does not bundle, supply a compiled
  native tree-sitter grammar (`.so`) plus a rule file and register it under
  `[customLanguages]` in `contasty.toml` — no rebuild.
- Extend / override: point an existing language at a user rule file from
  `[rules.<lang>]` to append to (`extend`) or replace (`override`) its embedded
  rules.

The rule file format, dynamic `.so` grammars, JSON Schema, and editor
integration are documented in [docs/languages.md](docs/languages.md); rule
extend/override in [docs/custom-rules.md](docs/custom-rules.md).

## Development

Prerequisites: [Nix](https://nixos.org/) with flakes enabled.

```bash
direnv allow   # or: nix develop

just check     # fmt + clippy + tests + file-size check
just build
just test
just cover     # code coverage (70% minimum)
just fmt       # format code
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for coding conventions.

## License

MIT
