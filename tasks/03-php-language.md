# 03 — PHP language, zero Rust

## Context

Proves the migration's headline claim: a new language needs no Rust code. PHP is
a built-in `SupportLang` variant (`tree-sitter-php` via `builtin-parser`), so it
needs only a rule file, fixtures, and tests.

## Goal

`contasty` strips `.php` files using `rules/php.yml` alone. No Rust changes
beyond registering the rule file (data, not logic) if task 01 did not already
make registration fully data-driven.

## Design

Author `rules/php.yml` mirroring the Rust behaviors, using PHP tree-sitter kinds.
Confirm exact kind names against the bundled grammar (inspect with
`ast-grep --lang php --debug-query` or the grammar's `node-types.json`); likely:

- elide function/method bodies: `function_definition` / `method_declaration`
  body `compound_statement`.
- elide closures/arrow fns bodies where present.
- truncate strings: `string` / `encapsed_string` / `heredoc` literals
  (`from_config: max_string_bytes`).
- delete comments: `comment` (covers `//`, `#`, `/* */`, `/** */`).
- delete imports: `namespace_use_declaration` (and `namespace_definition`? keep
  namespaces — they are structure, not imports; mirror Rust `use`-only drop).
- tests: drop PHPUnit cases — methods/classes with `#[Test]` attribute or
  `Test`-suffixed classes extending `TestCase`. Start minimal (attribute +
  class-name heuristic) and note limits; relational rules (`inside`, `has`) can
  tighten later.

PHP has no format hook (no prettyplease equivalent wired) — leave `format` None.

## Fixtures

Add `tests/fixtures/php/` with a representative file: namespace, `use`, class
with methods + bodies, a heredoc, comments, and a PHPUnit test class. Add an
expected stripped output (snapshot) for assertion.

## Steps

1. Author `rules/php.yml` (+ schema modeline from task 02).
2. Register the rule file / extension `php` (data-driven if task 01 allows).
3. Add PHP fixtures + a stripped-output snapshot.
4. Tests: bodies elided, comments/imports/tests dropped per flags, strings
   truncated, namespaces kept.
5. Update `docs/languages.md` and README's supported-languages list.

## Acceptance

- `.php` files dispatch to PHP and strip correctly under each `drop_*` flag.
- Zero Rust logic added for PHP (only rule file + optional registration data).
- `just fix-check` green; coverage >= 70%.

## Risks

- PHP kind names differ from intuition; verify against the actual grammar before
  writing rules, not from memory.
- PHPUnit detection is heuristic; scope it and document what it catches/misses.

Refs: tasks/03-php-language.md
