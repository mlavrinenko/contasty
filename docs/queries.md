# Query Files

Query files (`*.cty.yaml` / `*.cty.yml`) are saved, reusable selectors that
unfold to a set of source files. Instead of listing paths on the CLI, you
describe which files to include in a YAML file using `.gitignore` syntax.

## Quick start

```yaml
# api.cty.yaml
rules: |
  src/Domain
  !src/Domain/Cart
```

```sh
contasty api.cty.yaml
```

This selects every file under `src/Domain/` except those under
`src/Domain/Cart/`.

## Saved queries: `@name`

Drop a query file under `.contasty/queries/<name>.cty.yaml` (project) or
`$XDG_CONFIG_HOME/contasty/queries/<name>.cty.yaml` (global, fallback
`$HOME/.config/contasty/queries/`) and invoke it by name instead of by path:

```sh
contasty @api
```

Resolution order: project `.cty.yaml`, then `.cty.yml`, then the same two
extensions under the global queries dir; first hit wins. A name matching
neither is an error listing every path searched. The resolved file unfolds
like a query file passed by path. Because `rules` root at the project you run
against (see [Path relativity](#path-relativity)), `@name` works in any
project regardless of where the query file itself lives.

## Schema

A query file has three optional top-level keys:

| Key      | Purpose                                      |
|----------|----------------------------------------------|
| `ignore` | Gitignore filtering mode for this query       |
| `rules`  | Gitignore-syntax selection patterns           |
| `import` | Other query files to union into the result    |
| `strip`  | Categories to strip for this query's files    |

Unknown keys are rejected (`deny_unknown_fields`).

### `ignore`

Controls `.gitignore` filtering for this query's selection. Accepts the same
values as the CLI `--ignore` flag:

| Value     | Effect                                           |
|-----------|--------------------------------------------------|
| `enable`  | Respect `.gitignore` — only non-ignored (default)|
| `disable` | Include ignored files too (everything)           |
| `reverse` | Only `.gitignore`d files                         |

When set, the query's own `ignore` wins its selection; when absent, inherits
the ambient mode from the CLI (the most recent `--ignore` or `enable` by default).

```yaml
# Reach generated files that .gitignore excludes.
ignore: disable
rules: |
  generated/**
```

### `rules`

Selection patterns in `.gitignore` syntax. A bare line includes; a `!` prefix
excludes. Three forms are accepted:

**Inline string** (multiline YAML `|`):

```yaml
rules: |
  src/api
  !**/*_test.rs
```

**List of strings**:

```yaml
rules:
  - "src/api/**/*.rs"
  - "!**/*_test.rs"
```

**External file** (gitignore-format):

```yaml
rules:
  path: ./special.ignore
```

The external file itself is located relative to the query file's own
directory; the *patterns it contains* — like inline and list patterns — root
at the scanned project's working directory. See
[Path relativity](#path-relativity).

### `import`

Pull in other query files. The total selection is the union of the current
file's `rules` and every import's result.

```yaml
import:
  - shared.cty.yaml
  - path: optional.cty.yaml
    required: false
```

Each entry is either a bare string (required) or an object with `path` and
`required` (defaults to `true`). A missing required import is an error; a
missing optional import is silently skipped.

Import paths are relative to the importing query file's directory.

### `strip`

Categories to strip for files selected by this query. Same vocabulary as the
CLI `--strip` flag: a list of `comments`, `imports`, `tests`, `body`. The
query's strip set is unioned with the CLI's active strip set (CLI adds to
query).

```yaml
strip: [comments, imports]
```

## Pattern semantics

Patterns follow `.gitignore` conventions:

| Pattern             | Meaning                                       |
|---------------------|-----------------------------------------------|
| `src`               | Everything under `src/` (and `src` itself)    |
| `src/api`           | Everything under `src/api/`                   |
| `**/*.rs`           | All `.rs` files at any depth                  |
| `!src/secret`       | Exclude everything under `src/secret/`        |
| `src/**/*.php`      | All `.php` files under `src/`                 |

A directory pattern like `src` matches the directory and every file inside it
(including nested subdirectories). A `!` negation is local to its own query's
`rules` — it does not affect files selected by other queries in an `import`
union.

Last matching pattern wins (standard gitignore precedence).

## Path relativity

`rules` patterns — inline, list, or read from an external file — always root
at `cwd` (the directory you run `contasty` from), never at the query file's
own directory. A saved query under `.contasty/queries/` or the XDG global
queries dir thus describes *the project*, not its own location, so
`contasty @api` selects correctly in any project. At the project root run
from the project root, the query file's directory and `cwd` coincide, so
nothing changes from pre-rebasing behavior.

- **Inline / list patterns**, and **patterns read from an external file**: root at `cwd`.
- **The external `{ path }` file itself** and **import paths**: relative to
  the query file's own directory; may live outside `cwd` (e.g. under
  `.contasty/rules/` or the XDG global dir) — trusted, config-referenced
  machinery, not sandboxed.
- Files a query ultimately *selects* still always live under `cwd`, since the
  rules-matching walk roots there regardless of where the query lives.

CLI path arguments (non-query) are not sandboxed and may point outside the
working directory as before.

## Examples

```yaml
# domain.cty.yaml — select a subtree, exclude tests
rules: |
  src/Domain
  !**/*Test.php
```

```yaml
# full.cty.yaml — combine multiple queries
import:
  - domain.cty.yaml
  - api.cty.yaml
  - path: experimental.cty.yaml
    required: false
```

```yaml
# review.cty.yaml — use an external ignore file
rules:
  path: ./review-scope.ignore
```

### Query inside a walked folder / Nested query files

When `contasty` walks a directory, any `*.cty.yaml` found inside is
automatically unfolded. If its `rules` match another `*.cty.yaml`, that file
unfolds recursively — its selection joins the union instead of being emitted.
Imports and `rules` matches may form cycles this way; the resolver tracks
visited query files and skips duplicates, so cycles terminate without error.

## Error conditions

| Condition                      | Result  |
|--------------------------------|---------|
| Broken YAML syntax             | Error   |
| Unknown top-level key          | Error   |
| Missing required import        | Error   |
| Saved `@name` query not found  | Error   |
| Malformed pattern              | Error   |
| Missing optional import        | Skipped |
| Cyclic import                  | Skipped |
