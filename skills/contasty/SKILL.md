---
name: contasty
description: >-
  Build a compact, token-cheap map of a codebase with the `contasty` CLI, which
  strips function bodies, constant values, and long strings while keeping every
  declaration, signature, and type. Reach for this BEFORE reading a pile of
  source files just to learn a project's shape — use it whenever you need to
  understand an unfamiliar repo, map a module's public API or architecture,
  locate where something is declared, onboard to a new codebase, or fit a large
  tree into limited context. Triggers include "give me an overview of this
  repo", "what's the architecture", "what's the public API of X", "I'm new to
  this project", "map out src/", and any moment you are about to open many files
  only to skim their structure.
---

# contasty: compact codebase context

`contasty` walks a directory (respecting `.gitignore`), parses each source file,
and elides the parts you rarely need for orientation — function bodies, constant
values, long string literals, comments, imports — while keeping declarations,
signatures, and types intact. By default it prints a line-numbered per-file dump:
each kept line as `N: <line>` at its original number, elided bodies left as gaps
in the numbering. Those are the file's own line numbers, so the map doubles as an
index — when you do need a body, read exactly the gap's lines, not the whole file.

Why this matters: understanding a codebase's shape — its modules, types, public
API, layering — does not require its implementation bodies. The payoff is
completeness without the cost of reading everything. One contasty pass over a
large, unfamiliar tree gives you every declaration at roughly a fifth of the
tokens, so you can map the whole thing at once instead of sampling a handful of
files and guessing (or hallucinating) the rest. It is strongest on big or
unknown codebases, and when you need to hand a compact, faithful snapshot of the
code to another context window or model.

## When to use it

Use `contasty` whenever the goal is structural rather than line-level:

- Orienting in an unfamiliar repo, onboarding, "explain this project".
- Mapping a module or crate's public API / type surface.
- Finding where a symbol is declared across a large tree.
- Architecture or design questions that span many files.
- Anytime you would otherwise read many files just to skim them.

It earns its cost when the scope is many files. For a narrow, pinpoint question —
where one symbol is defined, the methods on a single class, one file's surface —
a `rg`/`grep` search is cheaper and just as accurate; use that instead. A capable
agent already explores small scopes efficiently with grep, so reach for contasty
when you would otherwise open many files, not when grep already answers it.

Do NOT use it when you need the actual logic: debugging a specific function,
reviewing a diff, editing code, or reasoning about an algorithm. Bodies are gone
in the output — read the real file for those. A good pattern is contasty first
for the map, then open the two or three files that matter in full.

## First: confirm it is installed

Run `contasty --version`. If it is missing, install it (`cargo install contasty`)
or download a release binary from
https://github.com/mlavrinenko/contasty/releases/latest. If you cannot install
it, fall back to reading files directly — do not block the task on it.

## Workflow

1. Scope the run to the question. The payoff scales with how much you would
   otherwise read, so aim contasty at exactly what the task needs — no wider:
   - Broad question (whole-project overview, "what is this", architecture) →
     run it over the whole tree or top package: `contasty src/`.
   - Narrow question (one subsystem, one module's API, where X lives) → point it
     at just that subtree or those files: `contasty src/db/models/` or
     `contasty src/forms/fields.py src/forms/forms.py`. A whole-tree map for a
     one-package question spends the budget you came to save.
2. For anything past a handful of files, write the map to a file and read that
   rather than emitting one enormous tool output:
   `contasty src/ > /tmp/contasty-context.md`, then read the file. A quick
   `contasty --stats <paths>` first confirms you aimed at the right scope and
   shows the reduction you are buying.
3. Answer from the map, and navigate by its line numbers. The map is your working
   source — read it for the types, signatures, and module layout. When you do need
   an elided body, the gap tells you where: a signature at line 42 whose next
   symbol is at line 60 means its body is lines 43–59, so read just that slice
   (`offset`/`limit`) instead of the whole file. Do not run contasty and then
   re-read whole files; that pays the token cost twice and throws away the saving.

## Selecting what to process

Paths are positional and repeatable; the result is a deduped union.

```bash
contasty src/                 # walk a directory (.gitignore-aware)
contasty src/lib.rs           # a single file
contasty src/ tests/          # several paths
contasty 'src/**/*.rs'        # a glob — quote it so contasty expands it, not the shell
contasty                      # default path is "."
```

contasty walks every file under a directory — templates, fixtures, and assets
included. If a package mixes those with code, aim it at the source files or a
source glob (`'pkg/**/*.py'`) so the map stays about the code and does not balloon.

For a scope you will reuse, save a query file (`*.cty.yaml`) instead of retyping
path lists; see `references/cli.md`.

## Controlling what gets stripped

Four categories. Defaults strip everything except tests.

| Category   | Default  | Keep it with        |
| ---------- | -------- | ------------------- |
| `comments` | stripped | `--strip=!comments` |
| `imports`  | stripped | `--strip=!imports`  |
| `tests`    | kept     | strip: `--strip=tests` |
| `body`     | stripped | `--strip=!body`     |

`--strip` is comma-separated and repeatable; prefix a category with `!` to keep
it. `all` (alias `everything`) strips all four; `none` strips nothing.

```bash
contasty src/                       # default: bodies, comments, imports gone; tests kept
contasty --strip=!comments src/     # keep comments (often worth it — doc comments
                                    #   carry the intent the stripped body no longer shows)
contasty --strip=tests src/         # also drop test functions
contasty --strip=none docs/         # keep everything (verbatim)
contasty --strip=all src/           # strip every category, including tests
```

Tip: doc comments explain what a signature does once its body is gone, so
`--strip=!comments` can be worth it on a modest scope. On a large tree it
inflates the map sharply — keep the default (comments stripped) for the wide
pass, then re-run with comments only on the narrower part where you need them.

## Other flags worth knowing

- `--format=markdown` — a reparseable Markdown document with fenced code blocks
  (elided bodies shown as `{}`), for pasting to a human or another chat;
  `--format=json` — a `{ base, files: [{ path, lang, content }] }` bundle for
  programmatic use. Default is the line-numbered `lines` format above.
- `--ignore=disable` — include `.gitignore`d files too; `--ignore=reverse` —
  only ignored files. Default respects `.gitignore`.
- `--config path.toml` — use a specific config file for the project layer
  (default: `.contasty/config.toml`; thresholds, custom rules, extra
  languages). Always layered under the XDG global config.
- `contasty @name` — run a saved query from `.contasty/queries/<name>.cty.yaml`
  or the XDG global queries dir instead of retyping a path list.

Full flag, query-file, and config reference: `references/cli.md`. Read it when a
task needs interleaved per-path settings, query files, JSON output details, or
custom language rules.

## Pitfalls

- The map has no bodies — only signatures and surrounding declarations are real.
  Don't quote it as evidence of what a function does internally; when you need the
  logic, use the line numbers to read that body's lines from the real file.
- Languages without a body-elision rule still appear, just less compacted; the
  `--stats` reduction tells you how much you actually saved.
- A glob must be quoted (`'src/**/*.rs'`) so contasty expands it; an unquoted
  glob is expanded by the shell and may behave differently or match nothing.
