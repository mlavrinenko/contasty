# Reformatting stripped output

Stripping splices kept declarations around elided bodies by raw byte ranges, so
the kept lines keep their original indentation while their spliced neighbours
shift — output is correct but can look ragged. A post-strip reformatter cleans
that up. Rust is reflowed through prettyplease out of the box; every other
language keeps the raw splice unless you opt into a reformatter.

Reformatting is purely cosmetic. It runs after splice, before render, and a
failure is never fatal: contasty logs a `warn` and falls back to the
unformatted text. Stripping correctness never depends on a formatter being
installed.

## Selecting a backend

Set `reformat` under a language's `[languages.<lang>]` entry. Three forms:

```toml
[languages.rust]
# Disable the built-in prettyplease pass for Rust.
reformat = "none"

[languages.php]
# Embedded Topiary backend (needs the `topiary` build feature + a query).
reformat = "topiary"

[languages.typescript]
# Shell out to an external formatter. Source arrives on stdin; formatted
# source is read from stdout.
reformat = { command = ["prettier", "--parser", "typescript"] }
```

- absent — keep the language's default (Rust: prettyplease; everything else:
  raw splice).
- `"none"` — force the raw splice, even for a language with a built-in pass.
- `"topiary"` — embedded Topiary backend (see below).
- `{ command = [...] }` — shell-out backend (see below).

Both Markdown and JSON output render from the same stripped string, so a
language is reformatted once, upstream of either renderer.

The `--no-reformat` CLI flag disables every reformatter for the run — built-in,
Topiary, and shell-out alike — without editing config. Use it to skip a slow or
untrusted formatter.

## Embedded mode (Topiary)

Topiary is a tree-sitter-based formatter. It is gated behind a cargo feature so
the default binary stays lean (Topiary pulls a grammar stack separate from
ast-grep's):

```sh
cargo build --features topiary
```

Topiary needs both a tree-sitter grammar and a formatting query per language.
contasty bundles the queries from `topiary-queries` and a small set of grammars;
where Topiary has no query/grammar for a language (PHP, today), `reformat =
"topiary"` is a hard config error, not a silent no-op. Build with the feature
off and the same key reports that the feature is missing — again, never silent.

Today the only registered language with a Topiary query is Rust (which can swap
its prettyplease default for Topiary). The supported set grows as more bundled
grammars gain queries.

## Shell-out mode

`reformat = { command = [...] }` runs an external formatter: the argv vector is
executed directly (no shell, no interpolation), stripped source is written to
its stdin, and formatted source is read from its stdout. A non-zero exit, a spawn
failure, a timeout, or non-UTF-8 output all degrade to the unformatted splice
with a `warn`.

Examples: `["gofmt"]`, `["black", "-", "-q"]`, `["prettier", "--parser", "php"]`,
`["pretty-php", "-"]`.

### Security note

A shell-out command in a checked-in `contasty.toml` runs an arbitrary local
program every time contasty processes a matching file. Treat a repository's
`contasty.toml` as you would any other executable content it ships: review the
`reformat` commands before running contasty against an untrusted checkout, and
use `--no-reformat` to neutralize them for a run. The argv vector is passed
straight to the OS with no shell, so there is no metacharacter-injection surface,
but the named program itself is fully trusted.

## Regenerating the PHP fixture

`tests/fixtures/php/sample.reformatted.php` is the tidy shell-out snapshot,
produced by piping the stripped fixture through `pretty-php` (in the dev shell):

```sh
pretty-php - < tests/fixtures/php/sample.stripped.php > tests/fixtures/php/sample.reformatted.php
```

The `shellout_reformats_php_fixture` test verifies it when `pretty-php` is
present and skips otherwise, so the suite never depends on the formatter.
