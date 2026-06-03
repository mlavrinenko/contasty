# 09 — Multi-path inputs: files, folders, wildcards

## Context

contasty is an in-repo dev tool: you add it to a project and use it to hand an
agent a tasty, compressed view of specific parts of the app. That is a recurring,
targeted query — "the auth layer", "the API surface" — not a one-shot whole-repo
dump.

Today the CLI takes exactly one path (`src/main.rs`, `path: PathBuf`, default
`"."`) and the walker (`src/walk.rs:53`) filters only by `.gitignore`. There is
no way to combine parts of the tree or scope by glob. The nearest compression
peer (repomix `--compress`) scopes with `--include` / `--ignore` globs; contasty
needs richer, project-native selection.

This task is the foundation: multiple path args, folders, and wildcards. Two
follow-ups extend the same resolver — `11` (query files `*.cty.yaml`) and `12`
(gitignore modes `--all` / `--ignored` / `--non-ignored`).

## Goal

`contasty` accepts multiple path arguments. Each argument unfolds to a set of
source files; the deduped, sorted union is stripped and rendered by the existing
pipeline. An argument is one of:

- a source file (e.g. `src/main.rs`) — contributes itself.
- a folder (e.g. `src/`) — walked `.gitignore`-aware.
- a glob / wildcard (e.g. `src/**/*.rs`, `crates/*/src`) — expands to matching
  files and folders; a folder match is walked (its whole subtree, confirmed).

Two explicit phases:

1. Resolve every argument to a deduped, sorted set of source files.
2. Strip + render that set (current pipeline, unchanged).

## Design notes

- Resolver module `src/inputs.rs`: `resolve(args) -> Result<Vec<PathBuf>>`
  classifies each arg (file / folder / glob), unfolds folders via the existing
  `ignore` walk, expands globs, returns a deduped sorted set. A `BTreeSet` doubles
  as dedup + visited set (a glob may match overlapping dirs); query unfolding (task
  `11`) hooks the same recursion. `pub`, re-exported. (Config is not needed yet —
  task `11` adds a `config` arg when query parsing needs the base dir.)
- Wildcards: expand globs ourselves — do not rely on the shell (Windows shells do
  not expand; quoted globs do not either). Match with `globset` (already in the
  tree via `ignore`; promote to a direct dep — no new compiled crate). `globset`
  matches a full path against one glob cleanly; `ignore::overrides` was rejected
  because its root/anchoring makes absolute paths (e.g. tempdir test paths)
  awkward. Build with `literal_separator(true)` so `*` stops at `/` and `**`
  crosses it. Expand a glob by walking its literal prefix (longest leading path
  with no glob metachar) `.gitignore`-aware and matching each entry's normalized
  path: a file match contributes itself; a directory match is walked as a folder
  arg (subtree), via the same recursion. The `glob` crate stays rejected (bypasses
  gitignore).
- Dedupe: lexical-normalize each path (strip leading `./`, resolve `..`, unify
  separators) into a `BTreeSet` — no `fs::canonicalize` (keeps relative display,
  no symlink surprises, no fs hit). Preserve the first-seen display form so
  `render.rs` (`common_base`, relative headings) is unaffected.
- Determinism: `BTreeSet` is sorted; `collect` already sorts. Argument order does
  not affect output order (matches current behaviour).
- `collect` (`src/walk.rs`) consumes the resolved file set (phase 2) instead of a
  single root. Breaking change to the re-exported `collect` signature
  (`&Path` → file slice) — fine at 0.1.0. Registry build stays in `collect`.
- Reserve the `*.cty.{yaml,yml}` sub-extension now: recognize (`is_query_file`)
  and SKIP such files during walks (do not strip them as YAML). Unfolding lands
  in `11`; reserving here avoids a behaviour flip when `11` ships.
- Errors (`thiserror` / `anyhow`): a named path that does not exist → error; a
  glob matching zero files → warn to stderr, continue; no args → default `["."]`.

## Open questions (resolved)

- Glob `crates/*/src` (matches directories): walks each matched dir's subtree.
- Dedupe identity: lexical normalization, not `fs::canonicalize`.

## Acceptance

- [x] CLI: `path: PathBuf` → `paths: Vec<PathBuf>` (variadic, default `["."]`) in
      `src/main.rs`; category flags keep working.
- [x] `src/inputs.rs`: `resolve(...)` classifies file / folder / glob, unfolds
      folders via the `ignore` walk, expands globs (subtree for dir matches),
      lexical-dedup, sorted; `pub` + re-exported. Designed to grow per-group
      options (gitignore mode in `12`, strip set in `13`); `collect` grows a
      per-file strip in `13`, so avoid baking a bare `Vec<PathBuf>` assumption
      everywhere.
- [x] Glob expansion via `globset` (promoted to a direct dep — already shipped
      transitively via `ignore`, no new compiled crate) + literal-prefix
      `.gitignore`-aware walk; no shell reliance.
- [x] `collect` consumes the resolved set; `.gitignore` semantics preserved for
      folder / glob walks; render base + relative paths unchanged.
- [x] `*.cty.{yaml,yml}` recognized and skipped during walks (reserved for `11`).
- [x] Errors: missing named path errors; zero-match glob warns; no args → `"."`.
- [x] Tests: multi-path union + dedupe; glob → files; glob → dir subtree; folder
      union; missing path errors; zero-match glob warns.
- [x] Docs: README usage (multi-path + globs).
- [x] `just fix-check` green.

## Out of scope

- Query files (`*.cty.yaml`) — task `11`.
- gitignore modes (`--all` / `--ignored` / `--non-ignored`, query `ignore` field)
  — task `12`.
- URL / remote-repo inputs — separate later task (wildcards do not apply to URLs).
- ast-grep node-level query semantics beyond file selection.
