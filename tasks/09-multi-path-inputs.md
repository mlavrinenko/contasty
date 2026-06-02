# 09 — Multi-path inputs: files, folders, query files, wildcards

## Context

contasty is an in-repo dev tool: you add it to a project and use it to hand an
agent a tasty, compressed view of specific parts of the app. That is a recurring,
targeted query — "the auth layer", "the API surface" — not a one-shot whole-repo
dump.

Today the CLI takes exactly one path (`src/main.rs`, `path: PathBuf`, default
`"."`) and the walker (`src/walk.rs:53`) filters only by `.gitignore`. There is
no way to combine parts of the tree, scope by glob, or save a reusable selection.
The nearest compression peer (repomix `--compress`) scopes with `--include` /
`--ignore` globs; contasty needs richer, project-native selection.

## Goal

`contasty` accepts multiple path arguments. Each argument unfolds to a set of
source files; the deduped union is stripped and rendered by the existing
pipeline. An argument is one of:

- a source file (e.g. `src/main.rs`) — contributes itself.
- a folder (e.g. `src/`) — walked `.gitignore`-aware; may contain both source
  files and query files.
- a query file — a saved selector that unfolds to other files.
- a glob / wildcard (e.g. `src/**/*.rs`, `crates/*/src`) — expands to matching
  files/folders.

Two explicit phases:

1. Resolve every argument to a deduped, sorted set of source files.
2. Strip + render that set (current pipeline, unchanged).

## Design notes

- Query file: a project may have several. It is a selector that resolves to a
  file set — when one is encountered (named directly, or found inside a walked
  folder) it unfolds to its target files instead of being emitted as content.
- Wildcards: expand globs ourselves — do not rely on the shell (Windows shells
  do not expand; quoted globs do not either). Reuse `globset` (already pulled in
  transitively via `ignore`) for matching, or the small `glob` crate for
  filesystem expansion. No heavy dependency.
- Dedupe: canonicalize and dedupe before phase 2 so a file reached twice (direct
  + via folder/glob/query) appears once.
- Determinism: keep the final list sorted (walk.rs already sorts) for stable
  output.

## Format (decided)

- Query files are YAML (reuses `serde_yaml`, already a dependency — no new dep).
  Sub-extension `*.cty.yaml`; also accept `*.cty.yml`. Both `.yaml` and `.yml`
  spellings supported.
- v1 schema: include/exclude globs and paths that resolve to a file set.
  ("query" leaves room to later carry ast-grep node-level selection — v1 is file
  selection only.) Proposed shape:

      # api.cty.yaml
      include:
        - "src/api/**/*.rs"
        - "src/routes.rs"
      exclude:
        - "**/*_tests.rs"

- Recognition is by the `.cty.yaml` / `.cty.yml` sub-extension. CRITICAL: YAML is
  itself a built-in source language (`src/lang/rules/yaml.yml`), so the resolver
  must test the query sub-extension BEFORE language detection — otherwise a
  `*.cty.yaml` file is taken for YAML source and stripped instead of unfolded.

## Open questions (settle before coding)

- Folder + query interaction: confirm a query file found inside a walked folder
  auto-unfolds (vs only when named explicitly on the command line).
- Nested queries: may a query file reference other query files in v1, or
  globs/paths only.

## Acceptance

- [ ] CLI: `path: PathBuf` → `paths: Vec<PathBuf>` (variadic, default `["."]`)
      in `src/main.rs`; category flags keep working.
- [ ] Resolver module (e.g. `src/inputs.rs`):
      `resolve(args, config) -> Result<Vec<PathBuf>>` classifies each arg
      (file / folder / query / glob), unfolds folders via the existing `ignore`
      walk, expands globs, parses query files, returns a deduped sorted set.
- [ ] Query-file YAML parser (`serde_yaml`) for `*.cty.{yaml,yml}`; recognition
      by sub-extension BEFORE language detection; errors via `thiserror`/`anyhow`.
- [ ] `collect` (`src/walk.rs`) consumes the resolved file set (phase 2) instead
      of a single root; `.gitignore` semantics preserved for folder args.
- [ ] Tests: multi-path union + dedupe; glob expansion; query unfolds to the
      expected files; folder containing a query file; missing/empty args.
- [ ] Docs: README usage + `docs/queries.md` (≤200 lines) for the query format.
- [ ] `just fix-check` green.

## Out of scope

- URL / remote-repo inputs — separate later task (wildcards do not apply to URLs).
- ast-grep node-level query semantics beyond file selection.
