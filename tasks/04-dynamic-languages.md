# 04 — Custom `.so` grammars via ast-grep-dynamic

## Context

Optional, last. Built-in languages (the 28 ast-grep bundles, including Rust and
PHP) cover the common case with zero `.so`. This task adds languages ast-grep
does not bundle, via user-supplied native tree-sitter grammars.

## Goal

A user drops a compiled grammar plus a rule file, registers it in contasty
config, and `.ext` files strip with no rebuild of contasty.

## Why native `.so` (and not wasm)

- tree-sitter grammars are generated C; the ecosystem ships them as native shared
  libraries.
- `ast-grep-dynamic` loads them with `libloading`, resolving symbol
  `tree_sitter_<name>`. It is the only loader ast-grep provides — no wasm path.
- wasm grammars exist at the tree-sitter layer but would bypass ast-grep entirely
  (custom loader + a wasm runtime dep). Out of scope; revisit only if portability
  of third-party grammars becomes a real need.

## Design

- Add `ast-grep-dynamic`. Introduce/extend the `Lang` enum from task 01:
  `Builtin(SupportLang)` | `Dynamic(DynamicLang)`.
- Config (`contasty.toml`) `customLanguages` section, mapping to
  `ast_grep_dynamic::CustomLang`:
  - `library_path`: single path or per-target-triple map (the `Platform`
    variant — native libs are not cross-platform).
  - `extensions`, optional `language_symbol`, `meta_var_char`, `expando_char`.
- Register once at startup: `CustomLang::register(base, langs)` (process-global
  `DynamicLang` registry). Resolve `library_path` relative to the config file.
- Rule files for dynamic languages use the same format and schema as built-ins;
  dispatch by extension as usual.

## Steps

1. Add `ast-grep-dynamic`; extend `Lang` dispatch to dynamic registrations.
2. Parse `customLanguages` from config into `CustomLang`; register at startup.
3. Surface load failures clearly (missing lib, bad symbol, wrong target triple).
4. Test with a fixture grammar `.so` (mirror ast-grep's `fixtures/json-linux.so`
   pattern; gate the test to supported targets like ast-grep does).
5. Document in `docs/languages.md`: `tree-sitter build --output name.so`, config
   stanza, per-platform paths.

## Acceptance

- A registered custom `.so` language strips its files via a rule file, no rebuild.
- Missing/incompatible library yields an actionable error, not a panic.
- Test gated to supported target(s) passes; others skip cleanly.
- `just fix-check` green.

## Risks

- Per-platform artifacts: document that users ship one `.so` per OS/arch, or use
  the `Platform` map.
- Global registry + `libloading` lifetime: ast-grep keeps the `Library` leaked on
  purpose (dropping nulls symbols). Do not fight it; register once, never unload.

Refs: tasks/04-dynamic-languages.md
