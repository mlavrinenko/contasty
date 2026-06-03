# contasty

<p align="center">
  <img src="https://raw.githubusercontent.com/mlavrinenko/contasty/main/www/logo.svg" alt="contasty logo" width="96" height="96">
</p>

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
hardcoded per-language logic. Built-in support: Rust, PHP, TypeScript, TSX,
JavaScript, Python, Go, Java, C#, Ruby, C++, C, Kotlin, Swift, Scala, Bash, Lua,
Dart, Elixir, Haskell, Nix, Solidity, JSON, YAML, HTML, CSS, and HCL — every
grammar ast-grep bundles except Markdown (prose, nothing to strip). You can
add a language with a dynamic tree-sitter grammar, and extend or override any
language's rules from `contasty.toml` — both
without rebuilding contasty.

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
contasty src/ > context.md             # strip a directory
contasty src/lib.rs                    # strip a single file
contasty src/ tests/ > context.md      # multiple paths (deduped union)
contasty src/lib.rs src/main.rs        # several files at once
contasty 'src/**/*.rs'                 # glob (quote it; expanded internally)
contasty 'crates/*/src'                # glob to dirs; each subtree is walked
contasty                               # default path is "."
contasty --include=comments src/       # keep comments (doc comments included)
contasty --include=tests src/          # keep test functions and test modules
contasty --exclude=imports src/        # drop every import / `use` declaration
contasty --include=all src/            # keep everything (alias: --include=everything)
contasty --exclude=all --include=comments src/   # only comments kept
contasty --include=everything --exclude=imports  # all except imports
contasty --format=json src/            # emit a JSON bundle instead of Markdown
contasty --stats src/                  # print compactization statistics
contasty --config path.toml src/       # use a specific contasty.toml
contasty --no-reformat src/            # skip all post-strip reformatting
contasty --ignore=disable src/         # include .gitignored files too
contasty --ignore=reverse src/         # only .gitignored files
contasty A --ignore=disable B --ignore=enable C  # per-path mode switching
```

Multiple arguments resolve to a deduped, sorted union of source files. A folder is
walked `.gitignore`-aware; a glob is expanded internally (quote it so the shell
does not expand or fail on no match), and a glob that matches directories walks
each matched subtree. A glob matching nothing warns and is skipped; a named path
that does not exist is an error.

Output defaults to Markdown. Pass `--format=json` for a pretty-printed JSON
bundle shaped as `{ "base": <dir>, "files": [{ "path", "lang", "content" }] }`,
mirroring the Markdown layout.

Three categories control what is kept or dropped:

| Category   | Default  | Selectors              |
| ---------- | -------- | ---------------------- |
| `comments` | excluded | `--include=comments`   |
| `tests`    | excluded | `--include=tests`      |
| `imports`  | included | `--exclude=imports`    |

`--include` and `--exclude` are repeatable and processed left to right, so the
last mention of a category wins. `all` (alias `everything`) applies to all
three at once.

`--ignore=<mode>` controls `.gitignore` filtering and is repeatable, interleaved
with paths (find-style). Each occurrence sets the mode for the paths that follow:

| Mode      | Effect                                           |
| --------- | ------------------------------------------------ |
| `enable`  | Respect `.gitignore` — only non-ignored (default)|
| `disable` | Include ignored files too (everything)           |
| `reverse` | Only `.gitignore`d files                         |

The default (before any `--ignore`) is `enable`. Query files can set their own
mode with the `ignore:` field (see [docs/queries.md](docs/queries.md)).

Category gating applies to every supported language — test and import rules in
each built-in rule file (and any custom rule file) declare which category gates
them, so the same flags work uniformly.

## How it compares

Two architectures. contasty is a one-shot stripper: walk the tree, elide bodies
in place, print one document. The same-shape peer is repomix `--compress`. Each
is stronger at different things.

|                                        | contasty                                                                | repomix --compress                                                                          |
| -------------------------------------- | ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| Languages with body elision            | [26, incl. JSON/YAML/HTML/CSS](docs/languages.md)                       | [16, incl. Vue](https://github.com/yamadashy/repomix/tree/main/src/core/treeSitter/queries) |
| Add a language without a rebuild       | [yes — dynamic grammar + rules](docs/languages.md)                      | no                                                                                          |
| Extend / override strip rules          | [yes — contasty.toml](docs/custom-rules.md)                             | no (fixed queries)                                                                          |
| Gate comments (keep / drop)            | [yes — per-language toggle](src/lang/rules/python.yml)                  | [yes — removeComments](https://repomix.com/guide/comment-removal)                           |
| Gate imports (keep / drop)             | [yes — --exclude=imports](src/lang/rules/python.yml)                    | no (imports kept)                                                                           |
| Gate tests (keep / drop)               | [yes — --include/--exclude=tests](src/lang/rules/python.yml)            | no                                                                                          |
| Stripped-region output                 | [valid empty bodies, reparseable](tests/fixtures/go/sample.stripped.go) | [⋮---- placeholder markers](https://repomix.com/guide/code-compress)                        |
| Optional reformat of result            | [yes — Topiary / shell-out](docs/reformatting.md)                       | no                                                                                          |
| Runtime                                | [single static binary](Cargo.toml)                                      | [Node.js](https://github.com/yamadashy/repomix)                                             |
| Output formats                         | [Markdown, JSON](src/render.rs)                                         | [XML, Markdown, JSON, plain](https://repomix.com/guide/output)                              |
| Token counting                         | no (by design)                                                          | [yes, multi-tokenizer](https://repomix.com/guide/command-line-options)                      |
| Secret scanning                        | no                                                                      | [yes](https://repomix.com/guide/security)                                                   |
| Git integration (diffs, history)       | no                                                                      | [yes](https://repomix.com/guide/command-line-options)                                       |
| Remote repos (clone by URL)            | no (local only)                                                         | [yes](https://repomix.com/guide/remote-repository-processing)                               |
| MCP server                             | no (CLI; agents shell out)                                              | [yes](https://repomix.com/guide/mcp-server)                                                 |

[ctx](https://docs.ctxllm.com) also extracts signatures, but only for PHP. For
interactive, query-on-demand context, see [aider's repo map](https://aider.chat/docs/repomap.html)
or [jCodeMunch-MCP](https://github.com/jgravelle/jcodemunch-mcp) — a different
approach: an index the agent queries live, not a static document.

## Configuration

Drop a `contasty.toml` in your project root to tune compaction thresholds,
set default category inclusion, register dynamic grammars, and extend or
override per-language rules. All fields are optional. See
[docs/languages.md](docs/languages.md) and [docs/custom-rules.md](docs/custom-rules.md).

Category inclusion can be set cross-language under `[include]` and refined
per language under `[languages.<lang>.include]`:

```toml
[include]
comments = false   # exclude comments by default (built-in default)
imports  = true    # include imports by default (built-in default)
tests    = false   # exclude tests by default (built-in default)

[languages.rust.include]
comments = true    # keep doc comments for Rust only
```

CLI flags override config for all languages. Config loads first; CLI wins.

Optional per-language post-strip reformatting (cosmetic, off by default) is
configured with the `reformat` key — embedded Topiary or a shell-out command.
See [docs/reformatting.md](docs/reformatting.md).

## Adding a language

contasty matches AST nodes with [ast-grep](https://ast-grep.github.io/) rules,
so a language is data, not code.

- Built-in: drop a rule file at `src/lang/rules/<lang>.yml` (embedded at build
  time) and register the language in `Registry::new`. No per-language matching
  logic in Rust.
- Dynamic grammar: for a language ast-grep does not bundle, supply a compiled
  native tree-sitter grammar (`.so`) plus a rule file and register it under
  `[languages.<lang>]` with a `libraryPath` in `contasty.toml` — no rebuild.
- Extend / override: point an existing language at a user rule file with the
  `extend` / `override` key of its `[languages.<lang>]` entry to append to
  (`extend`) or replace (`override`) its embedded rules.

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
