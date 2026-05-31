# Overriding a language's rules

Every language — built-in or dynamic — ships one embedded rule set
(`src/lang/rules/<lang>.yml`). To change how a language is stripped without
rebuilding contasty, point it at a user rule file from `contasty.toml` under
`[rules.<lang>]`. Two mutually exclusive modes:

```toml
[rules.rust]
# Append these rules to the embedded rust.yml set.
extend = "rules/rust-extra.yml"

[rules.php]
# Ignore the embedded php.yml; use only this file.
override = "rules/php-custom.yml"
```

- `extend` — compile the file's rules against the language and append them to the
  embedded set. User rules run after the built-ins.
- `override` — skip the embedded rules entirely; the user file is the whole set
  for that language.
- Exactly one of `extend` / `override` per entry. Setting both (or neither) is a
  config error, not a silent precedence rule.

The `<lang>` table key is the language name (`rust`, `php`, or a custom grammar's
name). The user file is an ordinary rule file (same format and schema as a
built-in's, see [languages.md](languages.md)); its `language:` is required and
must name the same language as the table key. Paths resolve against the config
file's directory, like `libraryPath` / `rules` under `[customLanguages]`.

## Precedence

contasty does not rank user rules against built-ins. Every rule contributes byte
ranges; `splice` sorts them (start ascending, then end descending) and a wider
range sharing a start wins, with exact duplicates deduped. An `extend` rule that
overlaps a built-in is resolved by range, not by a priority system — order only
breaks ties between identical ranges. When a built-in's behavior must be gone,
use `override`, not `extend`.

## Dynamic grammars

The modes apply equally to a dynamic grammar (`[customLanguages]`, see
[languages.md](languages.md)): `override` swaps the rule file declared there for
the one named under `[rules.<lang>]`, and `extend` appends to it.
