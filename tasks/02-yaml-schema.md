# 02 — Strict typing: JSON Schema + editor integration

## Context

Task 01 lands the rule file format. Rule files are the public surface for adding
languages, so they must be strictly typed: unknown keys rejected, IDE completion
and validation available (per https://zed.dev/docs/languages/yaml).

## Goal

A generated JSON Schema for contasty rule files, wired so editors (Zed, the
yaml-language-server) validate and autocomplete them, plus a CI check the schema
stays in sync with the Rust types.

## Design

- Derive `schemars::JsonSchema` on the rule config types from task 01.
  `ast-grep-config` already derives `schemars` 1.0 for `SerializableRule` —
  compose its schema in, do not redefine the rule grammar.
- `#[serde(deny_unknown_fields)]` on every config struct (typo = hard error).
- Generate `schemas/contasty-rules.schema.json` from the types. Prefer an `xtask`
  subcommand (`cargo xtask gen-schema`) over a build script so generation is
  explicit and reviewable. (No xtask crate exists yet — add a minimal one, or a
  `#[test]` that writes + diffs the file. Pick the lighter option.)
- CI/`just` check: regenerate into a temp file and diff against the committed
  schema; fail if drift. Add `just gen-schema` and wire a check into `just check`.
- Editor wiring:
  - Inline modeline at the top of each shipped rule file:
    `# yaml-language-server: $schema=../schemas/contasty-rules.schema.json`.
  - Document Zed `settings.json` mapping in `docs/` (file-glob -> schema URL),
    citing the zed yaml docs.
- Loader: on parse failure, surface the offending file + key path (serde_yaml
  gives line/col). Strict load is the runtime backstop; the schema is the
  authoring aid.

## Steps

1. Add `schemars` dep (match ast-grep's `1.0`); derive on config types.
2. Add schema generation (xtask or test) -> `schemas/contasty-rules.schema.json`.
3. Add `just gen-schema` + drift check in `just check`.
4. Add `# yaml-language-server` modeline to `rules/*.yml`.
5. Write `docs/languages.md`: schema location, Zed setup, authoring a rule file.
6. Add `#[serde(deny_unknown_fields)]`; test that an unknown key is rejected.

## Acceptance

- `schemas/contasty-rules.schema.json` exists and validates the shipped
  `rules/*.yml`.
- Drift check fails when a config struct changes without regenerating.
- Unknown-key rule file is rejected with a path-bearing error (regression test).
- `just fix-check` green; docs under the 200-line Markdown limit.

## Risks

- `schemars` 1.0 schema composition with ast-grep's derived `SerializableRule`
  schema may need a `$ref`/`definitions` merge; verify the rule subtree resolves
  in the yaml-language-server, not just at generation time.
- Keep `docs/languages.md` < 200 lines (linecop). Split if needed.

Refs: tasks/02-yaml-schema.md
