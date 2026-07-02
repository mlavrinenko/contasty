# 15 — Agent-native `lines` output + drop reformatting

Make contasty's default output agent-native and honest, and shed the
formatting machinery that fights it.

## New default format: `lines`

```
relative/path.ext
12: pub fn checkout(cart: &Cart, user: &User) -> Result<Receipt> …
42: pub fn refund(order: &Order) -> Result<()> …

another/path.ext
1: ...
```

- Bare relative path header per file (leading `./` trimmed), blank line between files.
- Every kept line prefixed with its original 1-based line number: `N: <verbatim line>`.
- Elided ranges collapse: a single `…` sentinel where the cut starts, the
  interior lines simply drop out. The gap in numbering is the body span, so the
  agent can `Read path offset=13 limit=…` instead of the whole file.
- Blank lines dropped.
- Line numbers reference the original file, so they stay valid for editing.

`--format=lines` is the default. `markdown` and `json` stay as opt-in alternates
(json gains the per-file elided ranges).

## Remove reformatting

Reformatting rewrites kept code and desyncs line numbers from the file, breaking
the coordinate contract. Delete it wholesale:

- `src/lang/reformat.rs`, `src/lang/topiary.rs`, `src/lang/shellout.rs` (+ tests)
- `--no-reformat` flag, `reformat` config key, `Config::no_reformat`
- `docs/reformatting.md`, the `reformatting-docs` outdatty group
- the `topiary` optional dep + feature in `Cargo.toml`

Bonus: smaller crate, one fewer heavy dep.

## Consequences

- Default output is no longer reparseable (by design, for agents). Update the
  README comparison honestly: line-anchored default, `--format=markdown` for the
  reparseable/paste-into-chat case.
- Breaking change: 0.2.0.
- Rewrite the skill workflow around "read the gap's lines, never the whole file."

## Done when

- Default `contasty <path>` emits the `lines` format; markdown/json behind `--format`.
- Reformatting fully gone; build has no topiary feature.
- `just check` green (incl. linecop, outdatty drift), regression tests for the
  `lines` renderer.
