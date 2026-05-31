# 01 — Engine swap + rule-driven strip core

## Context

`src/lang/rust.rs` hardcodes tree-sitter query strings, and `src/lang/mod.rs`
wires each one to a fixed struct field + `Action`. Adding behaviors or languages
means Rust edits. Replace the matching half with ast-grep; keep the splice half.

## Goal

Strip pipeline driven by external YAML rule files:

```
YAML rule -> RuleCore -> root.find_all(&rule) -> NodeMatch.range() ->
(start, end, Action) -> splice()
```

Rust support reaches parity with current behavior on the new engine. No
hardcoded queries remain in Rust.

## Dependencies

- Remove: `tree-sitter`, `tree-sitter-rust`.
- Add: `ast-grep-core`, `ast-grep-config`, `ast-grep-language`
  (feature `builtin-parser`).
- Keep: `syn`, `prettyplease` (Rust format hook), `serde`, `serde_yaml` (new).
- Run `cargo machete` — drop anything orphaned.

## Design

- Language dispatch: `SupportLang::from_path(path)` maps extension -> language.
  Registry maps extension -> loaded ruleset.
- Rule file (one per language, e.g. `rules/rust.yml`):
  - `language: rust`
  - `extensions: [rs]` (optional; default from SupportLang)
  - `rules: [ { action, rule, min_bytes? } ]`
  - `action`: `elide` | `delete` | `truncate` | `test-splice`.
  - `rule`: an `ast_grep_config::SerializableRule` (kind/pattern/has/inside/...).
  - `min_bytes`: `0` | integer | `{ from_config: elide_min_bytes }` |
    `{ from_config: max_string_bytes }`. Models today's thresholds: function
    bodies = 0; const/static/type = `elide_min_bytes`; strings = `max_string_bytes`.
- Build matcher: `SerializableRuleCore { rule, .. }.get_matcher(DeserializeEnv::new(lang))`
  -> `RuleCore`. Cache per (language, rule) — building is not free.
- Run: `lang.ast_grep(source)` (`LanguageExt`) -> root; `root.root().find_all(&rule)`
  -> `NodeMatch`; `.range()` -> `Range<usize>` byte offsets.
- Actions feed existing `splice`/`apply`/`sort_ranges` unchanged:
  - `elide` -> `ELISION`; `delete` -> remove + trailing newline;
  - `truncate` -> `STR_TRUNCATION`;
  - `test-splice` -> port `expand_attribute_to_item` onto ast-grep `Node`
    sibling navigation (absorb adjacent `#[..]` siblings + the item into one
    delete range).
- Format hook: keep `fn(&str) -> Option<String>` keyed by language name; Rust ->
  prettyplease. Same "only run under --include-comments" guard as today.

## Rust ruleset to port (rules/rust.yml)

- elide function bodies: `kind: function_item`, body block.
- elide const/static/type values: with `from_config: elide_min_bytes`.
- truncate `string_literal` / `raw_string_literal`: `from_config: max_string_bytes`.
- delete comments: `line_comment`, `block_comment` (under `--include-comments`
  default keep — wire to existing `drop_comments` flag).
- delete imports: `use_declaration` (under `drop_imports`).
- test-splice: attribute matching `^#\[(test|cfg\(test\))\]$` (under `drop_tests`).

The flag-gated rulesets (tests/comments/imports) stay grouped so the existing
`drop_*` booleans select which rule groups run, matching `collect_all` today.

## Error handling

Replace `AppError::LangLoad` / `AppError::Query(tree_sitter::QueryError)` with
ast-grep error variants (`RuleCoreError`, rule parse errors). Keep `ParseFailed`.
No `unwrap`/`expect` (clippy denies). Rule-file load errors surface with the
file path.

## Where rule files live

Embed defaults in the binary via `include_str!` so the tool works with zero
config, and allow a `rules/` dir override later (task 02 formalizes external
files + schema). Decide and note the chosen default-load strategy in the PR.

## Steps

1. Swap deps; `cargo build`.
2. Add rule config types + loader (serde structs, `deny_unknown_fields`).
3. Build the `RuleCore` cache + `find_all` -> ranges path.
4. Port `expand_attribute_to_item` and the format hook.
5. Author `rules/rust.yml`; delete `src/lang/rust.rs` query consts.
6. Green `src/lang/mod_tests.rs` and `src/walk.rs` tests unchanged.

## Acceptance

- `just fix-check` green.
- Existing tests pass without behavioral changes (elision marker, test/comment/
  import drop, string truncation, thresholds).
- No `tree_sitter::Query` or per-language query const remains in Rust.
- Coverage stays >= 70%.

## Risks

- `SupportLang` vs `DynamicLang` are distinct types; introduce a `Lang` enum now
  even though only `SupportLang` is used, so task 04 slots in cleanly.
- `builtin-parser` compiles all 28 grammars (binary size). Acceptable for the
  zero-code-per-language goal; note size delta, leave trimming as follow-up.

Refs: tasks/01-engine-swap.md
