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
- [ ] 09 — Multi-path inputs: files, folders, query files, wildcards (`09-multi-path-inputs.md`)
- [ ] 10 — Approximate token estimate in `--stats` (`10-token-estimate.md`)

## Order

07 is the larger reach lever for adoption (TS/JS, Python, Go, ... at launch) and
is pure rule-file data — pursue first. 06 is cross-cutting polish (cosmetic
output quality) and can land independently, after or alongside 07.

09 is the next adoption lever: targeted, reusable selections turn contasty into a
routine in-repo query tool instead of a whole-tree dump. 10 is small, dependency-
free polish (token estimate in `--stats`) and lands independently.
