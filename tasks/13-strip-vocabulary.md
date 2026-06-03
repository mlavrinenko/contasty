# 13 — Selection vocabulary: `--strip=<categories>` (per-path) + query `strip:`

## Context

Today category selection is `--include` / `--exclude` over comments / imports /
tests, with cross-language `[include]` and per-language `[languages.<lang>.include]`
config layers and ordered left-to-right CLI precedence (`src/main.rs`
`ordered_selectors` / `cli_override`; `src/config.rs` `resolve_selection`).

The unified vision wants one vocabulary shared by CLI and query files, and one
per-path mental model shared with `--ignore` (task `12`). This task renames
category selection to a per-path `--strip=<categories>` flag plus a query `strip:`
field — both find-style, both reusing the task-`12` positional-option parser.

Breaking change to shipped CLI behaviour. No backward-compat shim — pre-release.
Independent of the multi-path chain; needs `11` for the query struct (adds `strip:`)
and `12` for the shared parser.

## Goal

- CLI `--strip=<categories>` — per-path, find-style, interleaved with paths exactly
  like `--ignore`; the two share one parser. Each `--strip=` sets the strip set for
  the paths that follow; among several `--strip` before one path the last wins
  (replace, not add). Replaces `--include` / `--exclude` outright (no aliases).
  - Categories, comma-separated: `comments`, `imports`, `tests`, plus `all` /
    `everything` and `none`.
  - `none` = strip no categories (comments / imports / tests all kept); `all` =
    strip all three. Body elision is unconditional (always-on rules), so even
    `--strip=none` still elides bodies — `none` keeps the three category groups,
    not the bodies.
  - Example: `contasty --strip=comments,imports src/ --strip=none tests/`.
- Default (before any `--strip`) = `[comments, imports]`: imports stripped, test
  signatures kept (polarity flip from today; comments still stripped).
- Query `strip:` field, same vocabulary: `strip: [comments, imports]`.

## Combination rules (decided)

- Paths partition into groups: each a maximal run of consecutive path args under
  the same flag state. Options (`strip`, `ignore`) attach to the group, not the
  individual path. Distinct groups may resolve to identical options — coalesce
  them (intern the option-set once; an optimization, not a semantic).
- Plain group: strip = the CLI strip set active for it (last `--strip` before it,
  else the default).
- Query (its own one-path group): strip = query.strip ∪ the group's CLI strip —
  "CLI adds to query".
- Same file reached by several groups (after dedup): the last (highest-index)
  group that selected it wins its strip — extends find-style "last wins" to dedup.
  Put a specific exception after the general selection
  (`--strip=all src/ --strip=none src/keep.rs`).
- `ignore` (task `12`) needs no per-file rule: it governs candidacy, not stripping
  — a file is included if any group's mode admits it.

## Design notes

- Strip is a per-group attribute, realized per file: it travels resolve → collect.
  `resolve` (task `09`) returns files paired with their resolved strip; `collect` /
  `strip_one` apply per-file strip instead of one global `CategorySelection`. Dedup
  keeps the last-group-wins entry.
- Config layering reworked: `[strip]` cross-language + `[languages.<lang>.strip]`
  per-language, replacing `[include]`. Keep the layering shape (built-in <
  cross-language < per-language < CLI-per-path, then query-union); invert polarity
  (strip-list vs include-flags). `src/config.rs` `CategorySelection` /
  `resolve_selection` rework.
- Ripple is contained (verified): the 26 language goldens are NOT affected — their
  snapshot tests and the `strip` example drop all three categories explicitly via
  the `Registry` API, not via defaults. Only `src/config.rs`, `tests/cli.rs`,
  `src/main.rs`, and README change.

## Acceptance

- [ ] CLI `--strip=<categories>` per-path, find-style, sharing the task-`12`
      parser; comma-list; last-of-kind wins; `none` / `all`; `--include` /
      `--exclude` removed.
- [ ] Per-file strip travels resolve → collect; dedup = last-group-wins.
- [ ] Query `strip:` field; query.strip ∪ active CLI strip for the query's files.
- [ ] Config `[strip]` + `[languages.<lang>.strip]`; layering preserved;
      `src/config.rs` reworked.
- [ ] Default strip = `[comments, imports]`; `config.rs` + `tests/cli.rs` updated
      to the new polarity (imports stripped, tests kept).
- [ ] Tests: per-path strip; comma-list; last-of-kind; `none` / `all`; dedup
      last-group-wins; query union; config layering; default behaviour.
- [ ] Docs: README (usage + table) + `docs/queries.md` + CHANGELOG migration note.
- [ ] `just fix-check` green.

## Out of scope

- gitignore modes (`--ignore`) — task `12`.
- Multi-path / query file selection — tasks `09` / `11`.
