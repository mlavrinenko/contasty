# Language rule files

contasty strips a language by matching AST nodes with [ast-grep] rules. Each
supported language ships one rule file, `src/lang/rules/<lang>.yml`, embedded at
build time. Adding a language is a rule file plus a one-line registration; no
per-language matching logic in Rust.

[ast-grep]: https://ast-grep.github.io

## Supported languages

| Language | Rule file                  | Test detection                                   |
| -------- | -------------------------- | ------------------------------------------------ |
| Rust     | `src/lang/rules/rust.yml`  | `#[test]` / `#[cfg(test)]` items.                |
| PHP      | `src/lang/rules/php.yml`   | `class_declaration` whose name ends in `Test`.   |

PHP binds tree-sitter-php's `php_only` grammar (the variant ast-grep's
`SupportLang::Php` uses), so kinds/fields come from `php_only/src/node-types.json`.
It has no post-strip formatter wired (`format` is `None`).

PHPUnit test detection is a deliberately small heuristic: it deletes classes
whose name carries the conventional `*Test` suffix. It does not catch `#[Test]`-
annotated methods inside non-suffixed classes, data providers, or test traits;
tighten later with relational rules (`has` on the `attributes` field) if needed.

## Custom grammars (dynamic `.so`)

The 28 grammars ast-grep bundles cover the common case with zero `.so`. For a
language it does not ship, supply a compiled native tree-sitter grammar plus a
rule file and register it in `contasty.toml` — no rebuild of contasty.

Build the grammar to a native shared library (one per OS/arch — native libraries
are not portable):

```sh
tree-sitter build --output mylang.so   # run in the grammar repo
```

Register it under `[customLanguages.<name>]`. The `<name>` key is the language
identifier; it must match the rule file's `language:` and (unless overridden)
fixes the dylib symbol to `tree_sitter_<name>`:

```toml
[customLanguages.mylang]
# One library for the current host...
libraryPath = "grammars/mylang.so"
extensions = ["ml", "mli"]
rules = "rules/mylang.yml"

# ...or a per-target-triple map when you ship more than one platform:
# [customLanguages.mylang.libraryPath]
# "x86_64-unknown-linux-gnu" = "grammars/mylang-linux.so"
# "aarch64-apple-darwin"     = "grammars/mylang-mac.dylib"

# Optional overrides:
# languageSymbol = "tree_sitter_mylang"  # default: tree_sitter_<name>
# metaVarChar = "$"                       # pattern sigil; default $
# expandoChar = "_"                       # identifier-safe stand-in for $
```

Relative `libraryPath` and `rules` paths resolve against the config file's
directory. The grammar registers once at startup and is never unloaded
(libloading keeps the library leaked on purpose — dropping it nulls its
symbols), so every custom grammar must be declared in a single config.

The rule file is identical in format and schema to a built-in's; only its
`language:` names the custom grammar. Confirm `kind`/`field` names against the
grammar's `node-types.json` as for any language.

A missing library, wrong symbol, incompatible tree-sitter version, or a target
absent from a `libraryPath` map fails with an actionable error, not a panic.
Only native libraries are supported: ast-grep loads grammars through
`libloading` and has no wasm path.

## Overriding a language's rules

A built-in or dynamic language can be pointed at a user rule file that extends or
replaces its standard rules, with no rebuild — see
[custom-rules.md](custom-rules.md).

## Schema

The rule file format is described by a generated JSON Schema (Draft 2020-12) at
`schemas/contasty-rules.schema.json`. It is derived from the Rust config types,
so it never drifts from the loader: the rule subtree (`rule:`) is composed
directly from `ast-grep-config`'s own `SerializableRule` schema, giving you the
full ast-grep rule grammar with completion and validation.

Regenerate after changing a config struct:

```sh
just gen-schema
```

The `schema_in_sync` test (run by `just check`) fails if the committed schema
diverges from the types, so CI catches a forgotten regeneration.

## Editor integration

### Inline modeline (any yaml-language-server editor)

Every shipped rule file starts with a modeline so editors backed by
[yaml-language-server] pick up the schema with no per-project config:

```yaml
# yaml-language-server: $schema=../../../schemas/contasty-rules.schema.json
```

The path is relative to the rule file. New rule files under `src/lang/rules/`
use the same relative path.

[yaml-language-server]: https://github.com/redhat-developer/yaml-language-server

### Zed

Zed's YAML support is the same language server, configured in `settings.json`.
Map a file glob to the schema (see the [Zed YAML docs]):

```json
{
  "lsp": {
    "yaml-language-server": {
      "settings": {
        "yaml": {
          "schemas": {
            "./schemas/contasty-rules.schema.json": [
              "src/lang/rules/*.yml"
            ]
          }
        }
      }
    }
  }
}
```

The inline modeline already covers the shipped files; the glob mapping helps
when authoring rule files that do not (yet) carry a modeline.

[Zed YAML docs]: https://zed.dev/docs/languages/yaml

## Authoring a rule file

A rule file is a target `language` and an ordered list of `rules`:

```yaml
# yaml-language-server: $schema=../../../schemas/contasty-rules.schema.json
language: rust
rules:
  - action: elide
    field: body
    rule:
      kind: function_item
```

Unknown keys are rejected at load time (`deny_unknown_fields`), so a typo is a
hard error, not a silently ignored rule.

### Rule fields

| Field                | Required | Meaning                                                              |
| -------------------- | -------- | -------------------------------------------------------------------- |
| `action`             | yes      | `elide`, `delete`, or `truncate` the captured range.                 |
| `rule`               | yes      | An ast-grep `SerializableRule` selecting the anchor node.            |
| `field`              | no       | Named field to descend into before acting; absent field skips.      |
| `when`               | no       | Gate: `always` (default), `tests`, `comments`, or `imports`.         |
| `min-bytes`          | no       | Threshold the match must clear: `elide-min` or `max-string`.         |
| `expand-attributes`  | no       | Absorb adjacent attribute siblings plus the decorated item.          |

`action` semantics:

- `elide` — replace the range with `{}`.
- `delete` — remove the range plus one trailing newline.
- `truncate` — replace a string literal with a truncation marker.

The `rule` value is the full ast-grep rule grammar (`kind`, `pattern`, `regex`,
`any`, `all`, `inside`, `has`, ...). The schema autocompletes every option; see
the [ast-grep rule reference] for semantics.

[ast-grep rule reference]: https://ast-grep.github.io/reference/rule.html

### Thresholds

`min-bytes` names a threshold resolved at strip time from the active
`CompactConfig`:

- `elide-min` — `elide_min_bytes` (skip eliding small const/static/type values).
- `max-string` — `max_string_bytes` (only truncate long string literals).

A rule without `min-bytes` always fires regardless of captured size.

### Verifying node kinds

Rule `kind`/`field` names are tree-sitter grammar names, not guesses. Confirm
them against the grammar's `node-types.json` (or `ast-grep` playground) before
writing rules, especially for a new language.
