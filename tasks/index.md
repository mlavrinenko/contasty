# Tasks

Durable checklist of contasty's planned work. One file per task, named
`NN-slug.md`, sized for a single dense session.

## How to use

- Add a task: create `tasks/NN-slug.md` (next free number) and append a
  checklist line below linking to it.
- Work a task: keep it independently shippable — tests green, `just fix-check`
  clean.
- Finish a task: check its box here. The line stays as a permanent record, even
  after the file is cleaned up.

Commit footers reference the task (`Refs: tasks/NN-slug.md`); see the Task
References section in [CONTRIBUTING.md](../CONTRIBUTING.md) for the footer
convention and how a path resolves once its file is removed.

## Checklist

Each line links to its task file — present in the tree, or recoverable from git
history once cleaned up.

- [x] [01 — Engine swap + rule-driven strip core](01-engine-swap.md)
- [x] [02 — Strict typing: JSON Schema + editor integration](02-yaml-schema.md)
- [x] [03 — PHP language, zero Rust](03-php-language.md)
- [x] [04 — Custom `.so` grammars via ast-grep-dynamic](04-dynamic-languages.md)
- [x] [05 — User-extensible & overridable rule sets](05-custom-rules.md)
- [x] [06 — Generic reformatter: embedded Topiary + shell-out](06-generic-reformatter.md)
- [x] [07 — Built-in support for every ast-grep bundled language](07-builtin-languages.md)
- [x] [08 — README comparison vs repomix --compress](08-readme-comparison.md)
- [x] [09 — Multi-path inputs: files, folders, wildcards](09-multi-path-inputs.md)
- [x] [10 — Approximate token estimate in `--stats`](10-token-estimate.md)
- [x] [11 — Query files (`*.cty.yaml`): saved gitignore-style selectors](11-query-files.md)
- [x] [12 — gitignore modes: `--ignore=<mode>` (find-style, per-path)](12-gitignore-modes.md)
- [x] [13 — Selection vocabulary: `--strip=<categories>` + query `strip:`](13-strip-vocabulary.md)
- [x] [14 — End-user agent skill](14-end-user-skill.md)
- [x] [15 — Agent-native `lines` output + drop reformatting](15-lines-format.md)
