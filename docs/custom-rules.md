# Overriding a language's rules

Every language — built-in or dynamic — ships one embedded rule set
(`src/lang/rules/<lang>.yml`). To change how a language is stripped without
rebuilding contasty, point it at a user rule file with the `extend` / `override`
key of its `[languages.<lang>]` entry, in `.contasty/config.toml` (project) or
the XDG global `config.toml` (see [Config locations](#config-locations)
below). The two keys are mutually exclusive:

```toml
[languages.rust]
# Append these rules to the embedded rust.yml set.
extend = "rules/rust-extra.yml"

[languages.php]
# Ignore the embedded php.yml; use only this file.
override = "rules/php-custom.yml"
```

- `extend` — compile the file's rules against the language and append them to the
  embedded set. User rules run after the built-ins.
- `override` — skip the embedded rules entirely; the user file is the whole set
  for that language.
- At most one of `extend` / `override` per entry; neither is fine (the entry then
  only tunes `include` or registers a grammar). Setting both is a config error,
  not a silent precedence rule.

The `<lang>` table key is the language name (`rust`, `php`, or a custom grammar's
name). The user file is an ordinary rule file (same format and schema as a
built-in's, see [languages.md](languages.md)); its `language:` is required and
must name the same language as the table key. `extend` / `override` (and
`libraryPath` / `rules` in the same `[languages.<lang>]` entry) resolve to an
absolute path against their own config file's directory as soon as that layer
loads — before the two layers merge — so a global-defined rule file keeps
working no matter which project it applies to.

## Config locations

`.contasty/config.toml` in the project root is the project layer;
`$XDG_CONFIG_HOME/contasty/config.toml` (or `$HOME/.config/contasty/config.toml`
when `XDG_CONFIG_HOME` is unset) is the global layer. Both are optional and
layer project-over-global: a `[languages.<lang>]` entry in one, no matching key
in the other, applies as-is; the same key in both is replaced wholesale by the
project's entry — the two are not merged field-by-field. Anything registered
once in the global config (a dynamic grammar, an `extend` / `override`) is
available to every project on the machine. `--config <path>` overrides the
project layer's path only; the global layer still applies.

Custom rule files, like a project's own rule sets, conventionally live under
`.contasty/rules/<lang>.yml` at either level, keeping everything the config
references under one namespaced directory — but any path the config points at
works, since it is resolved relative to that config file's own directory.

## Precedence

contasty does not rank user rules against built-ins. Every rule contributes byte
ranges; `splice` sorts them (start ascending, then end descending) and a wider
range sharing a start wins, with exact duplicates deduped. An `extend` rule that
overlaps a built-in is resolved by range, not by a priority system — order only
breaks ties between identical ranges. When a built-in's behavior must be gone,
use `override`, not `extend`.

## Dynamic grammars

The modes apply equally to a dynamic grammar (a `[languages.<lang>]` entry with a
`libraryPath`, see [languages.md](languages.md)): `override` swaps the grammar's
declared `rules` file for the `override` file, and `extend` appends to it.
