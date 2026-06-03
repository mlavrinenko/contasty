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

A query file has two optional top-level keys:

| Key      | Purpose                                      |
|----------|----------------------------------------------|
| `rules`  | Gitignore-syntax selection patterns           |
| `import` | Other query files to union into the result    |

Unknown keys are rejected (`deny_unknown_fields`).

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

When `path` points to an external file, patterns are relative to that file's
directory (just like a real `.gitignore`). Inline and list patterns are
relative to the query file's own directory.

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

### Select a subtree, exclude tests

```yaml
# domain.cty.yaml
rules: |
  src/Domain
  !**/*Test.php
```

### Combine multiple queries

```yaml
# full.cty.yaml
import:
  - domain.cty.yaml
  - api.cty.yaml
  - path: experimental.cty.yaml
    required: false
```

### Use an external ignore file

```yaml
# review.cty.yaml
rules:
  path: ./review-scope.ignore
```

Where `review-scope.ignore` contains:

```
src/Service
src/Controller
!src/Controller/Admin
```

### Query inside a walked folder

When `contasty` walks a directory, any `*.cty.yaml` found inside is
automatically unfolded. Its selected files are added to the union alongside
other files found by the walk.

### Nested query files

A `rules` pattern that matches another `*.cty.yaml` unfolds it recursively,
exactly like an `import` — its selection joins the union instead of being
emitted as content.

## Cycle guard

Imports (and `rules` that match other query files) may form cycles (A pulls in
B, B pulls in A). The resolver tracks visited query files and skips duplicates,
so cycles terminate without error.

## Error conditions

| Condition                          | Result  |
|------------------------------------|---------|
| Broken YAML syntax                 | Error   |
| Unknown top-level key              | Error   |
| Missing required import            | Error   |
| Path escapes working directory     | Error   |
| Malformed pattern                  | Error   |
| Missing optional import            | Skipped |
| Cyclic import                      | Skipped |
