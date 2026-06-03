# Tasks

Active task checklist for contasty. One file per task, named `NN-slug.md`,
sized for a single dense session.

## Conventions

- Commit footers reference the task: `Refs: tasks/NN-slug.md`.
- Each task is independently shippable: tests green, `just fix-check` clean.
- Mark a task done by checking its box below; leave the file in place as record.
- Release prep / maintenance / docs that advance no planned task use
  `Refs: CHANGELOG.md`.

## Checklist

Foundational engine work (01–05) shipped before the first release; those files
live in git history (removed from the working tree at release). Read one with
`git show <commit>:tasks/NN-slug.md`.

- [x] 01 — Engine swap + rule-driven strip core
- [x] 02 — Strict typing: JSON Schema + editor integration
- [x] 03 — PHP language, zero Rust
- [x] 04 — Custom `.so` grammars via ast-grep-dynamic
- [x] 05 — User-extensible & overridable rule sets
- [x] 06 — Generic reformatter: embedded Topiary + shell-out (`06-generic-reformatter.md`)
- [x] 07 — Built-in support for every ast-grep bundled language (`07-builtin-languages.md`) — all tiers shipped (26 langs; Markdown intentionally excluded)
- [x] 08 — README comparison vs repomix --compress (`08-readme-comparison.md`)
- [x] 09 — Multi-path inputs: files, folders, wildcards (`09-multi-path-inputs.md`)
- [ ] 10 — Approximate token estimate in `--stats` (`10-token-estimate.md`)
- [x] 11 — Query files (`*.cty.yaml`): saved gitignore-style selectors (`11-query-files.md`)
- [ ] 12 — gitignore modes: `--ignore=<mode>` (find-style, per-path) (`12-gitignore-modes.md`)
- [ ] 13 — Selection vocabulary: `--strip=<categories>` + query `strip:` (`13-strip-vocabulary.md`)

## Order

07 is the larger reach lever for adoption (TS/JS, Python, Go, ... at launch) and
is pure rule-file data — pursue first. 06 is cross-cutting polish (cosmetic
output quality) and can land independently, after or alongside 07.

09 / 11 / 12 are the next adoption lever: targeted, reusable selections turn
contasty into a routine in-repo query tool instead of a whole-tree dump. They were
split from one oversized task into three single-session pieces that chain on a
shared resolver (`src/inputs.rs`): 09 is the foundation (multi-path files, folders,
wildcards); 11 adds saved `*.cty.yaml` query files (gitignore-syntax selection,
imports); 12 adds find-style gitignore modes (`--all` / `--ignored` /
`--non-ignored`, plus a query `ignore` field). Order is strict: 09 → 11 → 12. 10 is
small, dependency-free polish (token estimate in `--stats`) and lands
independently of the 09 chain. 13 unifies selection vocabulary (`--strip` +
query `strip:`, retiring `--include` / `--exclude`); it is independent of
multi-path but needs 11 for the query struct, so sequence it after 11.
