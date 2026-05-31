# Tasks

Migration from hardcoded per-language tree-sitter queries to ast-grep as the
matching engine, with language support expressed as strictly-typed external YAML
rule files. Goal: adding a language needs no Rust code — a YAML rule file (and,
for grammars ast-grep does not bundle, a tree-sitter `.so`).

## Conventions

- One file per task, named `NN-slug.md`, sized for a single dense session.
- Commit footers reference the task: `Refs: tasks/NN-slug.md`.
- Each task is independently shippable: tests green, `just fix-check` clean.
- Mark a task done by checking its box below; leave the file in place as record.

## Pipeline (target)

```
YAML rule -> RuleCore (get_matcher) -> root.find_all(&rule) ->
NodeMatch.range() -> (start, end, Action) -> splice()
```

contasty's splice engine (`src/lang/mod.rs`) is kept. ast-grep replaces only the
"select node ranges" half — the hand-rolled `tree_sitter::Query` plumbing.

## Order

- [x] 01 — Engine swap + rule-driven strip core (`01-engine-swap.md`)
- [x] 02 — Strict typing: JSON Schema + editor integration (`02-yaml-schema.md`)
- [ ] 03 — PHP language, zero Rust (`03-php-language.md`)
- [ ] 04 — Custom `.so` grammars via ast-grep-dynamic (`04-dynamic-languages.md`)

01 is foundational and ships Rust support on the new engine. 02 locks the rule
file format down with a generated schema. 03 proves the zero-code claim by adding
PHP as YAML only. 04 is the extensibility path for unbundled grammars; optional,
last.

## Key facts (verified against var/ast-grep @ 0.43.0)

- ast-grep 0.43.0: edition 2024, rust-version 1.85 — matches contasty exactly.
- tree-sitter 0.26.3 (transitive via ast-grep) replaces contasty's direct 0.20.
- Rust and PHP are both built-in `ast_grep_language::SupportLang` variants
  (feature `builtin-parser`). Neither needs a `.so`.
- `ast-grep-dynamic` loads native `.so`/`.dylib`/`.dll` via `libloading` only.
  No wasm path exists in ast-grep; wasm grammars would bypass its machinery.
- `ast-grep-config` already derives `schemars` 1.0 schemas for its rule types —
  compose, do not reinvent, in task 02.
