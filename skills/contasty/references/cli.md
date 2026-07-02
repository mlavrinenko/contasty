# contasty CLI reference

Full surface for the `contasty` command. SKILL.md covers the common path; read
this when a task needs interleaved per-path settings, query files, JSON output,
or custom language rules.

## Synopsis

```
contasty [OPTIONS] [PATH]...
```

`PATH` is repeatable: files, directories, or globs. Each unfolds to a set of
source files; the deduped, sorted union is stripped. Directories are walked
`.gitignore`-aware. Globs are expanded internally — quote them (`'src/**/*.rs'`)
so the shell does not expand them first. A glob matching nothing warns; a missing
literal path errors. Default path is `.`.

## Options

| Flag            | Effect                                                            |
| --------------- | ---------------------------------------------------------------- |
| `--strip=<CATS>`| Categories to strip. Comma-separated, repeatable, interleaved with paths. |
| `--ignore=<MODE>`| `.gitignore` filtering: `enable` (default), `disable`, `reverse`. Repeatable, interleaved. |
| `--stats`       | Print compaction statistics instead of the stripped code.         |
| `--format=<FMT>`| `lines` (default), `markdown`, or `json`.                         |
| `--config=<PATH>`| Use a specific `contasty.toml` (default: `./contasty.toml`).     |
| `-h, --help`    | Print help.                                                       |
| `-V, --version` | Print version.                                                    |

## Strip categories

| Category   | Default  | Meaning                                        |
| ---------- | -------- | ---------------------------------------------- |
| `comments` | stripped | Comment lines and blocks, including doc comments. |
| `imports`  | stripped | Import / use declarations.                     |
| `tests`    | kept     | Test functions and test modules.               |
| `body`     | stripped | Function/method bodies, constant values, long strings. |

`--strip` vocabulary: list any of `comments, imports, tests, body`; `all`
(alias `everything`) means all four; `none` means strip nothing. Prefix a
category with `!` to remove it from the set (e.g. `all,!body` strips everything
except bodies).

`--strip` is repeatable and find-style: each occurrence sets the strip set for
the paths that follow it, until the next `--strip`.

```bash
contasty --strip=comments,imports src/      # only comments + imports
contasty --strip=all src/                    # everything, including tests
contasty --strip=all,!body src/              # all except bodies
contasty --strip=none tests/                 # keep tests/ verbatim
contasty src/ --strip=none tests/            # default for src/, keep-all for tests/
```

Config precedence: `contasty.toml` loads first; any CLI `--strip` overrides it
for all languages.

## Ignore modes

`--ignore` controls `.gitignore` filtering, repeatable and interleaved with
paths like `--strip`.

| Mode      | Effect                                            |
| --------- | ------------------------------------------------- |
| `enable`  | Respect `.gitignore` — only non-ignored (default). |
| `disable` | Include ignored files too (everything).            |
| `reverse` | Only `.gitignore`d files.                          |

```bash
contasty --ignore=disable src/                       # include ignored files
contasty A --ignore=disable B --ignore=enable C      # per-path switching
```

## Output formats

Lines (default): per file, a bare relative-path header, then each surviving line
verbatim as `N: <line>` at its original number. A multi-line body keeps its
opening line and drops out below, so the gap in the numbering is its span — a
signature at line 42 whose next symbol is line 60 has its body at lines 43–59,
which you read straight back from the file. Only a cut confined to one line (a
one-line body, a mid-line value or truncated string) is marked in place with `…`.
Blank lines are omitted; files are separated by a blank line.

```
src/checkout.rs
12: pub fn checkout(cart: &Cart, user: &User) -> Result<Receipt> {
42: pub fn refund(order: &Order) -> Result<()> {
```

Markdown (`--format=markdown`): one document, a fenced code block per file under
a heading, elided bodies shown as `{}` — reparseable, for pasting to a human or
another chat.

JSON (`--format=json`): pretty-printed bundle shaped as

```json
{ "base": "<dir>", "files": [ { "path": "...", "lang": "...", "content": "..." } ] }
```

Use JSON when another tool consumes the bundle.

## Stats

`--stats` prints original-vs-compacted line counts (code / comments / blanks)
and an approximate token figure (`~tokens`). The token figure is a
dependency-free estimate (`~bytes / 4`), not a model tokenizer count — use it
for relative comparison, not exact budgeting.

## Query files (`*.cty.yaml`)

A query file is a saved, reusable selector that unfolds to a set of files using
`.gitignore` syntax. Pass it like any path: `contasty api.cty.yaml`. When
`contasty` walks a directory it auto-unfolds any `*.cty.yaml` it finds.

Top-level keys (all optional; unknown keys rejected):

| Key      | Purpose                                          |
| -------- | ------------------------------------------------ |
| `rules`  | `.gitignore`-syntax selection (bare = include, `!` = exclude). |
| `ignore` | Ignore mode for this query (`enable`/`disable`/`reverse`). |
| `import` | Other query files to union in.                   |
| `strip`  | Strip categories for this query's files (unioned with CLI). |

```yaml
# api.cty.yaml — a subtree minus its tests, keep comments
ignore: enable
rules: |
  src/api
  !**/*_test.rs
strip: [body, imports]
```

`rules` accepts an inline multiline string, a list of strings, or an external
gitignore-format file (`rules: { path: ./scope.ignore }`). `import` entries are
bare strings (required) or `{ path, required }`. Inline/list patterns are
relative to the query file's directory; external rule paths to the external
file's directory; imports to the importing file's directory. `../` is allowed but
the resolved path must stay within the working directory. Cyclic imports are
skipped; last matching pattern wins.

## Configuration (`contasty.toml`)

Optional file in the project root (or via `--config`). All fields optional.

```toml
strip = ["comments", "imports", "body"]   # cross-language default strip set

[languages.rust]
strip = ["comments", "body"]              # keep imports for Rust only
```

It also tunes compaction thresholds (e.g. how long a string must be to elide),
registers dynamic tree-sitter grammars for languages ast-grep does not bundle,
and extends or overrides per-language strip rules. CLI `--strip` overrides the
config strip set for all languages.

## Built-in languages

Body elision is rule-driven per language. Built-ins cover Rust, PHP, TypeScript,
TSX, JavaScript, Python, Go, Java, C#, Ruby, C++, C, Kotlin, Swift, Scala, Bash,
Lua, Dart, Elixir, Haskell, Nix, Solidity, JSON, YAML, HTML, CSS, and HCL. A
language without a body-elision rule still appears in the output, just less
compacted. New languages can be added via a dynamic grammar plus a rule file in
`contasty.toml`, no rebuild required.
