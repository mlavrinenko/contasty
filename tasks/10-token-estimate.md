# 10 — Approximate token estimate in `--stats`

## Context

Every compression / packer peer surfaces a token count (repomix
`--token-count-tree`, code2prompt, gitingest) — it answers the one question that
matters for context: does this fit the window? contasty's `--stats`
(`src/stats.rs`) reports tokei line counts (code / comments / blanks) only.

Constraint (decided this session): contasty ships a lean static binary (release
profile: `strip`, `lto`, `panic = "abort"`) and avoids heavy runtime and
dependencies. A real tokenizer (tiktoken-rs plus its data tables) is too heavy,
and an exact count is per-model anyway — task 08's follow-up already flagged that
faking a precise count locally is dishonest. So: a cheap, dependency-free
heuristic, clearly labelled as an estimate.

## Goal

`--stats` additionally prints an approximate token figure for original vs
compacted output, explicitly labelled approximate, with the heuristic documented
so nobody mistakes it for a model tokenizer.

## Design notes

- Dependency-free heuristic only. Options: `ceil(bytes / 4)` (the common rough
  rule), or a char-class split (alphanumeric runs + individual punctuation as
  separate tokens). Pick one, document the formula.
- Label clearly in the report (e.g. `~tokens (estimate)`); never imply per-model
  accuracy.
- Lives in `src/stats.rs`. No new entry in `Cargo.toml`.

## Acceptance

- [ ] `approx_tokens(text: &str) -> usize` helper in `src/stats.rs`, formula
      documented in a doc comment.
- [ ] Stats report shows approximate tokens for original vs compacted alongside
      the existing line counts, labelled approximate.
- [ ] Unit tests: non-zero for non-empty input, deterministic, monotonic under
      concatenation; report-format test.
- [ ] README `--stats` note states the figure is an estimate, not a model
      tokenizer count.
- [ ] No new dependency; `just fix-check` green.

## Out of scope

- Exact per-model tokenization (tiktoken / HF). If ever wanted, it belongs behind
  an off-by-default build feature — not here.
