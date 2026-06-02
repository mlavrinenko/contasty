# Golden fixtures

Each `<lang>/` holds a `sample.<ext>` input and its `sample.stripped.<ext>`
golden. The `fixture_strips_to_snapshot` test in `tests/<lang>.rs` strips the
sample with every category dropped (tests, comments, imports) at the default
`CompactConfig` and asserts byte-equality with the golden.

## Regenerating a golden

Use the `strip` example so the golden is exactly what the engine emits:

```sh
just strip tests/fixtures/<lang>/sample.<ext> > tests/fixtures/<lang>/sample.stripped.<ext>
```

Redirect with `>`. Do NOT capture through command substitution
(`out=$(just strip ...)`): `$(...)` strips trailing newlines, so the golden ends
up missing the final newline(s) the splice actually produces and the snapshot
test fails (or, worse, drifts) for reasons that have nothing to do with the
rules.

## Adding a language

Pick a sample that exercises every rule the language has: a function/method body
(if it elides), a long string (> `max_string_bytes`, default 256, so truncation
fires), a comment, and an import / test where the language has those categories.
Confirm the stripped output re-parses with zero tree-sitter ERROR/MISSING nodes
(`just dump-ast <lang> tests/fixtures/<lang>/sample.stripped.<ext>` — no `ERROR`
or `MISSING` lines), and with the real parser where one is cheap (`bash -n`,
`ruby -c`, `node --check`), since tree-sitter parses some constructs the language
itself rejects.
