# 07 — Built-in support for every ast-grep bundled language

## Context

contasty's engine is rules-only: a language is a YAML rule file plus one
`Registry::new` line
(`Language::from_rules("php", include_str!("rules/php.yml"), Reformatter::None)`).
Built-ins today are just Rust and PHP, but ast-grep 0.43 bundles 28 grammars
out of the box (no `.so` needed). Every one is reachable as a built-in with
zero new Rust matching logic — only rule data. For a tool pitched as "tasty
context for your agent," shipping the languages agents actually live in
(TypeScript/TSX, Python, Go, JavaScript, ...) is the single biggest adoption
lever.

No language ships a built-in, always-on formatter (task 06 removed Rust's
prettyplease): the engine carries no per-language formatting dependency. A new
built-in registers with `Reformatter::None`. Output is the raw byte-splice;
tidy it only via task 06's opt-in backends (embedded Topiary where a query
exists, else a shell-out command in `contasty.toml`). Adding a Rust formatter
crate per language is explicitly out of scope — it would bloat the default
binary, the exact cost the reformatter design avoids.

## Goal

Ship an embedded rule file for each ast-grep bundled language, registered in
`Registry::new`, so `contasty` strips them with no config and no rebuild step
for the user.

## Bundled languages (ast-grep-language 0.43.0, `SupportLang`)

Bash, C, Cpp, CSharp, Css, Dart, Elixir, Go, Haskell, Hcl, Html, Java,
JavaScript, Json, Kotlin, Lua, Markdown, Nix, Php, Python, Ruby, Rust, Scala,
Solidity, Swift, Tsx, TypeScript, Yaml.

(Rust, Php already done.)

## Design

Each new language is `src/lang/rules/<lang>.yml` + a `from_rules` entry. Per
the established pattern, every rule file should, where the construct exists:

- Elide function / method / closure / constructor bodies.
- Elide large `const` / value initializers above `elide_min_bytes`.
- Truncate string literals above `max_string_bytes`.
- Gate categories with `when: imports | tests | comments` so the existing
  `--include`/`--exclude` flags work uniformly (per-language test conventions
  differ — Go `*_test.go` funcs, Python `test_*` / `pytest`, JS/TS `describe`/
  `it`, JUnit `@Test`, etc.).
- Keep declarations, signatures, types, namespaces/packages intact.

Markup/data languages (Json, Yaml, Html, Markdown, Css, Hcl) have no
"executable body" to elide; decide per language whether they get a meaningful
rule set (e.g. truncate long string/scalar values) or are intentionally
skipped. Document which are stripping-capable vs structural-only.

### Phasing

Do not land 28 at once. Tier by demand:

1. Tier 1 (highest agent-codebase value): TypeScript, Tsx, JavaScript, Python,
   Go.
2. Tier 2: Java, CSharp, Ruby, Cpp, C, Kotlin, Swift, Scala.
3. Tier 3 / niche: the rest (Bash, Lua, Dart, Elixir, Haskell, Nix, Solidity,
   ...) and the data/markup languages.

Each tier is independently shippable. Track sub-progress in this file's
checklist below.

## Per-language acceptance

For every added language:

- A fixture pair under `tests/fixtures/<lang>/` (`sample.<ext>` +
  `sample.stripped.<ext>`) and a golden test, mirroring the PHP harness.
- Categories (`imports`, `tests`, `comments`) verified by test.
- Rule file carries the `yaml-language-server` schema modeline; the
  `schema_in_sync` test stays green.
- README "Built-in support" line and `docs/languages.md` table updated.

## Checklist

- [ ] Tier 1: TypeScript / Tsx / JavaScript / Python / Go
- [ ] Tier 2: Java / CSharp / Ruby / Cpp / C / Kotlin / Swift / Scala
- [ ] Tier 3: remaining + data/markup languages (or documented skips)

## Open questions

- Share rules across close cousins (TypeScript / Tsx / JavaScript) via the
  `extend` mechanism, or keep one self-contained file each?
- A bundled grammar with ragged raw-splice output never blocks this task:
  reformatting is opt-in via task 06 (Topiary query if one exists, else a
  shell-out command), never a bundled per-language formatter. Note in
  `docs/reformatting.md` which new languages have a Topiary query available.

## Done when

- Tier 1 lands with fixtures, tests, and docs; `just fix-check` green.
- Remaining tiers tracked above and shipped incrementally.
