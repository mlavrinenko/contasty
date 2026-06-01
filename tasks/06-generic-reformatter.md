# 06 — Generic reformatter: embedded Topiary + shell-out

## Context

After stripping, each language's output is only as clean as its splice. Rust
gets reflowed through prettyplease (`rust::format`), so elided bodies collapse
to tidy `{}` and indentation is normalized. Every other language keeps the raw
byte-splice result: PHP, for example, leaves the original indentation of kept
lines while their spliced neighbours shift, producing ragged output (captured
in `tests/fixtures/php/sample.stripped.php`). It is cosmetic — an LLM reads it
fine — but it hurts the "tasty" promise and gets worse as more languages land
(task 07).

`Language::from_rules` already takes an optional formatter hook
(`Some(rust::format)` vs `None`). This task generalizes that hook into a
configurable, per-language reformatter with two backends.

## Goal

Any language can be reformatted after stripping, via one of two modes the user
selects in `contasty.toml`:

- Embedded: Topiary (`topiary-core`), tree-sitter based like ast-grep. No
  external process.
- Shell-out: run a user-configured formatter command (e.g. `prettier`,
  `gofmt`, `black`), feeding stripped source on stdin and reading stdout.

Rust keeps prettyplease as its built-in default; the new modes are opt-in for
languages that have no built-in formatter.

## Design

Config under the existing per-language story (`[languages.<lang>]`). Sketch:

```toml
[languages.php]
# Embedded Topiary backend (requires a Topiary query for the language).
reformat = "topiary"

[languages.typescript]
# Shell-out backend. {} is replaced by nothing; source arrives on stdin,
# formatted source is read from stdout. Non-zero exit => leave unformatted + warn.
reformat = { command = ["prettier", "--parser", "typescript"] }
```

- `reformat` is `none` (default) | `"topiary"` | `{ command = [...] }`.
- The reformat step runs after splice, before render, replacing/extending the
  current `Option<fn(&str) -> ...>` formatter slot on a registered language.
- A reformat failure is never fatal: emit a `log::warn!` and fall back to the
  unformatted stripped text. Stripping correctness must not depend on a
  formatter being installed.

### Embedded mode (Topiary)

- Depend on `topiary-core` (+ the relevant `topiary-queries` / grammar set).
- Topiary needs a tree-sitter grammar plus a query file per language. Confirm
  the overlap with ast-grep's bundled grammars; where Topiary lacks a language
  (e.g. PHP today), embedded mode is simply unavailable for it — surface a
  clear config error, do not silently no-op.
- Decide whether Topiary grammars are vendored, fetched, or gated behind a
  cargo feature so the default build stays lean (Topiary pulls a grammar stack
  separate from ast-grep's).

### Shell-out mode

- `std::process::Command`, stdin = stripped bytes, capture stdout, timeout
  guard. No shell interpolation — argv vector, not a string.
- Document the security note: a shell-out command in a checked-in
  `contasty.toml` runs arbitrary local programs. Consider a `--no-reformat`
  CLI override and/or refusing shell-out unless explicitly allowed.

## Open questions

- Feature-gate embedded Topiary (`--features topiary`) to keep the default
  binary small, or always include it?
- One `reformat` per language, or a fallback chain (try embedded, then
  shell-out)?
- Should `--format=json` content also be reformatted, or only Markdown? (Yes —
  both render from the same stripped string; reformat once upstream.)

## Done when

- A non-Rust language can be reformatted via both modes, selected from config.
- Formatter absence/failure degrades gracefully (warn + raw output), proven by
  a test.
- PHP fixture regenerated through a reformatter shows tidy indentation.
- Docs (`docs/languages.md`) cover both modes and the shell-out security note.
- `just fix-check` green.
