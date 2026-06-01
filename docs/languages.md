# Language rule files

contasty strips a language by matching AST nodes with [ast-grep] rules. Each
supported language ships one rule file, `src/lang/rules/<lang>.yml`, embedded at
build time. Adding a language is a rule file plus a one-line registration; no
per-language matching logic in Rust.

[ast-grep]: https://ast-grep.github.io

## Supported languages

| Language   | Rule file                        | Value elision | Test detection                                          |
| ---------- | -------------------------------- | ------------- | ------------------------------------------------------- |
| Rust       | `src/lang/rules/rust.yml`        | const/static/type | `#[test]` / `#[cfg(test)]` items.                   |
| PHP        | `src/lang/rules/php.yml`         | —             | `class_declaration` whose name ends in `Test`.          |
| TypeScript | `src/lang/rules/typescript.yml`  | object/array  | `describe` / `it` / `test` / `suite` call statements.   |
| TSX        | `src/lang/rules/tsx.yml`         | object/array  | `describe` / `it` / `test` / `suite` call statements.   |
| JavaScript | `src/lang/rules/javascript.yml`  | object/array  | `describe` / `it` / `test` / `suite` call statements.   |
| Python     | `src/lang/rules/python.yml`      | dict/list/set | `test_*` functions and `Test*` classes.                 |
| Go         | `src/lang/rules/go.yml`          | —             | `Test*` / `Benchmark*` / `Example*` / `Fuzz*` functions. |
| Java       | `src/lang/rules/java.yml`        | —             | `@Test`-family methods; `*Test` / `*Tests` / `*IT` classes. |
| C#         | `src/lang/rules/csharp.yml`      | —             | `[Fact]` / `[Theory]` / `[Test]` / `[TestMethod]` methods; `*Test` / `*Tests` classes. |
| Ruby       | `src/lang/rules/ruby.yml`        | hash          | `test_*` methods; `describe` / `it` / `context` / ... blocks. |
| C++        | `src/lang/rules/cpp.yml`         | aggregate-init | `TEST` / `TEST_F` / `TEST_P` / `TYPED_TEST` macros.    |
| C          | `src/lang/rules/c.yml`           | aggregate-init | `test_*` functions (Unity / CMocka naming).            |
| Kotlin     | `src/lang/rules/kotlin.yml`      | —             | `@Test`-family functions.                               |
| Swift      | `src/lang/rules/swift.yml`       | —             | `test*` functions (XCTest naming).                      |
| Scala      | `src/lang/rules/scala.yml`       | —             | `*Test` / `*Spec` / `*Suite` classes/objects.           |

Each is an embedded rule file plus a one-line `Registry::new` registration,
riding the 28 grammars ast-grep 0.43 bundles — no `.so`, no config. Remaining
bundled languages are tracked in `tasks/07-builtin-languages.md`; the rule files
carry per-rule comments. Notes:

- The generic `{}` marker means value-init elision fires only where `{}` is
  valid in that slot: C/C++ aggregate inits (`= { ... }` → `= {}`), Ruby hashes,
  TS/TSX/JS object/array, Python dict/list/set. The "—" rows (Go, Java, C#,
  Kotlin, Swift, Scala) have no brace value literal and keep initializers. Ruby
  also has no brace body, but `{}` is a valid empty-hash statement so an elided
  method body still parses; C# `=> expr` members and Swift computed properties
  stay verbatim (neither is a brace body).
- Test detection is by AST shape and name, not filename — a heuristic (PHP misses
  `#[Test]`; TS/JS miss `it.skip`; C/Swift go by name). TS/TSX/JS share node
  kinds, so their files are deliberate self-contained near-duplicates, not shared
  via `extend` (a user-rule mechanism, not wired between built-ins).

## Custom grammars (dynamic `.so`)

The 28 grammars ast-grep bundles cover the common case with zero `.so`. For a
language it does not ship, build a native tree-sitter grammar (one per OS/arch —
native libraries are not portable), supply a rule file, and register both in
`contasty.toml` — no rebuild of contasty:

```sh
tree-sitter build --output mylang.so   # run in the grammar repo
```

Register it under `[languages.<name>]` with a `libraryPath` (which marks a custom
grammar). The `<name>` key must match the rule file's `language:` and (unless
overridden) fixes the dylib symbol to `tree_sitter_<name>`:

```toml
[languages.mylang]
# One library for the current host...
libraryPath = "grammars/mylang.so"
extensions = ["ml", "mli"]
rules = "rules/mylang.yml"

# ...or a per-target-triple map when you ship more than one platform:
# [languages.mylang.libraryPath]
# "x86_64-unknown-linux-gnu" = "grammars/mylang-linux.so"
# "aarch64-apple-darwin"     = "grammars/mylang-mac.dylib"

# Optional overrides:
# languageSymbol = "tree_sitter_mylang"  # default: tree_sitter_<name>
# metaVarChar = "$"                       # pattern sigil; default $
# expandoChar = "_"                       # identifier-safe stand-in for $
```

Relative `libraryPath` / `rules` paths resolve against the config file's
directory. The grammar registers once at startup and is never unloaded
(libloading leaks the library on purpose), so every custom grammar must be
declared in a single config. The rule file is identical to a built-in's; confirm
`kind`/`field` names against the grammar's `node-types.json`. A missing library,
wrong symbol, incompatible tree-sitter version, or a target absent from a
`libraryPath` map fails with an actionable error, not a panic (only native
libraries are supported — ast-grep has no wasm path).

## Overriding a language's rules

A built-in or dynamic language can be pointed at a user rule file that extends or
replaces its standard rules, with no rebuild — see
[custom-rules.md](custom-rules.md).

## Schema

The rule file format is a generated JSON Schema (Draft 2020-12) at
`schemas/contasty-rules.schema.json`, derived from the Rust config types so it
never drifts: the `rule:` subtree is composed from `ast-grep-config`'s own
`SerializableRule` schema, giving the full ast-grep rule grammar with completion.

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

The path is relative to the rule file; new files under `src/lang/rules/` reuse it.

[yaml-language-server]: https://github.com/redhat-developer/yaml-language-server

### Zed

Zed's YAML support is the same language server, configured in `settings.json`.
Map a file glob to the schema (see the [Zed YAML docs]):

```json
{ "lsp": { "yaml-language-server": { "settings": {
  "yaml": { "schemas": { "./schemas/contasty-rules.schema.json": ["src/lang/rules/*.yml"] } }
} } } }
```

The inline modeline already covers shipped files; the glob helps when authoring
files that do not (yet) carry one.

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

The `tests`, `comments`, and `imports` gates are language-agnostic: any rule
file (built-in or custom) can use them. Whether a category is active is
controlled by the CLI and config, not by which language is being stripped.

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
`CompactConfig`: `elide-min` (`elide_min_bytes`, skip small value elisions) or
`max-string` (`max_string_bytes`, only truncate long strings). A rule without
`min-bytes` always fires regardless of captured size.

### Verifying node kinds

Rule `kind`/`field` names are tree-sitter grammar names, not guesses — confirm
them against the grammar's `node-types.json` or the `ast-grep` playground,
especially for a new language. (`ast-grep run --lang L -p "$(cat f)"
--debug-query=ast f` dumps a file's tree.)
