# 05 — User-extensible & overridable rule sets

## Context

Every language ships one embedded rule file (`src/lang/rules/<lang>.yml`).
Today a user who wants different stripping for a built-in language has no lever:
the rules are baked in at compile time. The two real needs:

- Add one extra rule on top of the standard set (e.g. also elide a project's
  custom macro body in Rust) without restating the whole file.
- Replace the standard set entirely when the project wants full control.

Custom dynamic languages (task 04) already load rules from a user path, but
built-in languages cannot be touched. This task closes that gap for every
language, built-in or dynamic.

## Goal

From `contasty.toml`, a user points a built-in (or dynamic) language at a rule
file that either extends or overrides its standard rules, with no rebuild.

## Design

Config under the existing per-language story. Sketch (`contasty.toml`):

```toml
[rules.rust]
# Append these to the embedded rust.yml rules (default mode).
extend = "rules/rust-extra.yml"

[rules.php]
# Ignore the embedded php.yml; use only this file.
override = "rules/php-custom.yml"
```

- One mode key per language entry, `extend` xor `override` (reject both — a
  config error, not a silent precedence rule).
- `extend`: parse the user file, compile its rules against the language, append
  to the embedded set. Order matters for splice precedence (wider-range-wins is
  already handled in `splice`), so document that user rules run after built-ins.
- `override`: skip the embedded rules entirely; the user file is the whole set.
  The user file still needs a `language:` (or infer from the table key).
- Reuse the task-04 rule-file loader and the existing `RuleFile` schema — no new
  rule grammar. An extend file is just a `RuleFile` whose rules are appended.
- Resolve paths relative to the config file, like `customLanguages`.
- Registry build (`Registry::with_config`) grows a post-pass that, per language,
  applies the configured extend/override before the registry is frozen.

Open questions to settle during implementation:

- Whether `language:` in an extend file is required or redundant with the table
  key (prefer: optional, must match if present).
- Whether override on a language with no built-in rules (a dynamic grammar) is
  just the normal task-04 path (likely yes — unify them).
- Dedup: should an extend rule that duplicates a built-in be detected, or left
  to the idempotent splice? Start permissive, document.

## Steps

1. Add `[rules.<lang>]` parsing to `config.rs` (`extend` xor `override`).
2. Thread it into `Registry::with_config`; apply per language after built-ins.
3. Errors: both keys set, unknown language, unreadable/malformed file, rule that
   fails to compile against the grammar — all actionable.
4. Tests: extend adds a rule (built-ins still fire); override replaces (a
   built-in behavior is gone, the custom one fires); both-keys is an error.
5. Document in `docs/languages.md`: the two modes, precedence, path resolution.

## Acceptance

- A one-rule `extend` file adds behavior while every standard rule still applies.
- An `override` file fully replaces the standard set for that language.
- `extend` + `override` on one language is a clear config error.
- Works for built-in and dynamic languages alike.
- `just fix-check` green.

## Risks

- Precedence/overlap between user and built-in rules: lean on the existing
  range-sort/dedup in `splice`; document the resolution rather than inventing a
  priority system.
- Scope creep into a full rule-merging DSL. Keep it to append-or-replace.

Refs: tasks/05-custom-rules.md
