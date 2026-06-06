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

Patterns in an external file are relative to that file's directory (like
`.gitignore`); inline and list patterns are relative to the query file's own
directory.

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

- **Inline / list patterns**: relative to the query file's directory.
- **External `{ path }` rules**: relative to the external file's directory.
- **Import paths**: relative to the importing query file's directory.
- **`../` is allowed** in all of the above, but the resolved path must stay
  within the working directory (the directory you run `contasty` from).
  Escaping the working directory is an error.

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

## Cycle guard

Imports and `rules` that match other query files may form cycles. The resolver
tracks visited query files and skips duplicates, so cycles terminate without error.

## Error conditions

| Condition                      | Result  |
|--------------------------------|---------|
| Broken YAML syntax             | Error   |
| Unknown top-level key          | Error   |
| Missing required import        | Error   |
| Path escapes working directory | Error   |
| Malformed pattern              | Error   |
| Missing optional import        | Skipped |
| Cyclic import                  | Skipped |
