# contasty

[![CI](https://github.com/speconaut/contasty/actions/workflows/ci.yml/badge.svg)](https://github.com/speconaut/contasty/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/contasty.svg)](https://crates.io/crates/contasty)
[![License: MIT](https://img.shields.io/crates/l/contasty.svg)](LICENSE-MIT)

Strips executable lines from your code to prepare tasty context for your agent.

Walks a directory (respecting `.gitignore`), parses each supported source file
with [tree-sitter](https://tree-sitter.github.io/), elides function bodies, and
prints the result as a single Markdown document — declarations, signatures, and
types intact, ready to paste into an LLM context window.

Supported languages: Rust.

## Install

### From crates.io

```bash
cargo install contasty
```

### From binary releases

Download a pre-built binary from the
[latest release](https://github.com/speconaut/contasty/releases/latest).

## Usage

```bash
contasty src/ > context.md     # strip a directory
contasty src/lib.rs            # strip a single file
contasty                       # default path is "."
```

### Adding a language

1. Add a `tree-sitter-<lang>` dependency to `Cargo.toml`.
2. Drop a sibling module under `src/lang/` returning a `Language` (grammar,
   extensions, elide query).
3. Register it inside `Registry::new`.

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
