# 11 — Query files (`*.cty.yaml`): saved gitignore-style selectors

## Context

Builds on the `09` resolver. A query file is a saved, reusable selector that
unfolds to a source-file set instead of being emitted as content. A project may
have several. Recognized by the `*.cty.{yaml,yml}` sub-extension (`09` reserves
and skips it; this task makes it unfold). YAML reuses `serde_yaml` (already a
dependency — no new dep).

Selection is expressed in `.gitignore` syntax, not bespoke include/exclude lists:
it is battle-tested, matches the tool's existing model, and maps exactly onto
`ignore::overrides::Override` (bare line = include, `!` = exclude). This also
sidesteps the naming clash between query keys and the CLI `--include` /
`--exclude` category flags.

## Goal

A `*.cty.{yaml,yml}` argument — named on the CLI or found inside a walked folder —
unfolds to its selected source files, deduped into the `09` union.

## Format (decided)

Recognition by sub-extension BEFORE language detection (CRITICAL: YAML is a
built-in source language, `src/lang/rules/yaml.yml`, so it would otherwise be
stripped as YAML). `09` already reserves the sub-extension.

Schema is strict (`deny_unknown_fields`, like the rest of config); a broken file
is an error, not a silent fallback. This task ships `rules` + `import` only; the
query later gains `ignore:` (task `12`) and `strip:` (task `13`), reusing the same
names as their CLI flags.

```yaml
# api.cty.yaml — gitignore-syntax selection
rules: |
  src/Domain
  !src/Domain/Cart

# or a list of patterns
rules:
  - "src/api/**/*.rs"
  - "!**/*_tests.rs"

# or load patterns from an external gitignore-format file
rules:
  path: ./special.ignore

# pull in other query files (union)
import:
  - shared.cty.yaml
  - path: optional.cty.yaml
    required: false
```

- `rules`: `.gitignore` syntax. Bare line = include; `!` = exclude (an allow-list
  with negation — exactly `ignore::overrides::Override`). Accepts an inline string
  (multiline `|`), a list of strings, or `{ path }` to an external file.
- Path relativity: patterns are relative to the file that declares them — the
  query file for inline `rules`, the external file for the `{ path }` form (just
  like a real `.gitignore`). `../` allowed; a resolved path must stay within the
  CWD (lexical check) — escaping it is an error. CLI args (from `09`) stay
  CWD-relative and may leave the CWD; only query-derived paths are sandboxed.
- `exclude` (`!`) is local to its own query's `rules` (does not affect the union).
- `import`: list of other query files; each a bare string or `{ path, required }`.
  `required` defaults `true` (missing → error, per `09`'s policy); `required:
  false` → skip silently. Import paths are relative to the importing query's dir,
  capped at the CWD. Total selection = own `rules` ∪ each import; deduped at the
  end.

## Design notes

- Recognition: reuse `is_query_file(path)` from `09` to gate unfolding (CLI arg
  and inside a folder walk — auto-unfold confirmed).
- Parse with `serde_yaml` into a strict struct. `rules` is an untagged enum
  `Inline(String) | List(Vec<String>) | File { path }`.
- Resolve a query: build an `Override` from its `rules` (rooted at the declaring
  file's dir), walk candidates `.gitignore`-aware, then filter candidates through
  the `Override` matcher (intersect: `rules` select within candidates; they do
  not re-include `.gitignore`d files — mode controls candidacy, added in `12`).
  Recurse imports, union, lexical-dedup.
- Recursion + cycle guard: `resolve` is recursive (query → import → query; a
  query's `rules` can also match another `*.cty.yaml`). The `09` visited-set
  (lexical keys) prevents cycles and double work.
- The CWD-escape check is a single chokepoint (one function), so it can be
  hardened later (symlink-aware / `canonicalize`-based) without reshaping the
  resolver.
- Errors via `thiserror` / `anyhow`: broken YAML, unknown field, missing required
  import, path escaping the CWD — all actionable messages.

## Acceptance

- [x] `*.cty.{yaml,yml}` recognized before language detection; unfolds instead of
      emitting — both as a CLI arg and when found inside a walked folder.
- [x] `serde_yaml` parser; `rules` (inline / list / `{ path }`) + `import`
      (string / `{ path, required }`); strict (`deny_unknown_fields`); broken file
      errors.
- [x] gitignore-syntax selection via `ignore::overrides::Override`; `!` exclude is
      local to the query.
- [x] Patterns relative to the declaring file; `../` allowed; escaping the CWD is
      an error.
- [x] `import` union; `required` defaults `true` (missing → error), `required:
      false` → skip; cycle guard holds.
- [x] Tests: query unfolds to expected files; `!` exclusion; list form; external
      `{ path }` form; folder containing a query auto-unfolds; import union;
      missing required import errors; cycle guard; path-escape errors.
- [x] Docs: `docs/queries.md` (≤200 lines) — schema, relativity rules, examples.
- [x] `just fix-check` green.

## Out of scope

- gitignore modes / query `ignore` field — task `12`.
- ast-grep node-level selection beyond file selection (the "query" name leaves
  room; v1 is file selection only).
- URL / remote-repo inputs.
