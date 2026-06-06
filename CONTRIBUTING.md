# Contributing to contasty

## Code Style

All clippy lints are set to `deny` level — the project will not compile with violations.

Key restrictions:
- No `unwrap()` — use `?` operator or `anyhow`/`thiserror` error handling
- No `todo!()`, `unimplemented!()`, `unreachable!()` — handle all cases
- No `unsafe` code
- No wildcard imports (`use foo::*`)
- No single-character variable names (minimum 2 characters)
- Functions: max 70 lines, max 5 arguments, max cognitive complexity 20

## Error Handling

- Use `anyhow::Result` for application-level code (binaries, CLI)
- Use `thiserror::Error` for library error types that callers will match on
- Propagate errors with `?` — never `unwrap()` or `expect()`

## Project Structure

Keep `main.rs` as a thin entry point — argument parsing, logger init, and a call into
library code. All logic belongs in `lib.rs` (and its modules). `main.rs` is excluded from
coverage, so anything there is untested by default.

## Code Coverage

Minimum 70% coverage enforced via `cargo-tarpaulin`. Run `just cover` to check.
`main.rs` is excluded — keep it thin and move testable logic to `lib.rs`.

## Testing

Any config-layered or CLI-driven behavior needs at least one end-to-end
assertion in `tests/cli.rs` (or another integration test) that exercises the
whole pipeline, not only a unit test of the resolver function in isolation.
A function can be written, unit-tested, and still never wired into `collect` /
`main`; only an end-to-end test catches that gap.

## File Size Limits

- Rust files: 500 lines max
- Markdown files: 200 lines max

When a file exceeds the limit, split it into modules or separate documents.

## Dependency Drift

[outdatty.yaml](outdatty.yaml) declares groups that couple `source` files to the
`dependents` that must stay in sync with them — for example, CLI code to the docs
that describe it. `just check` runs `outdatty check`, which fails when a source
changed but its dependents were not re-confirmed.

After editing a source, review the listed dependents, update them as needed, then
run `just outdatty-update` to record the new state into `outdatty.lock` and commit
it. Add or adjust groups whenever you introduce files that must move together.

## Submitting Changes

1. Run `just check` before submitting — it runs clippy, tests, and file size checks
2. Run `just fmt` to format code
3. Ensure `just cover` meets the 70% threshold

## Task References

Every commit footer carries a `Refs:` pointing at the task file it advances:

```
Refs: tasks/01-engine-swap.md
```

Active tasks live in `tasks/`, tracked by the checklist in `tasks/index.md`.
`tasks/index.md` is permanent; individual `tasks/NN-slug.md` files are cleaned
up from time to time once their work has shipped. A `Refs:` may therefore point
at a file no longer in the working tree — it resolves against git history, read
it with `git show <commit>:tasks/NN-slug.md` or `git log -- tasks/NN-slug.md`
(e.g. the foundational `01`–`05` tasks removed at the `0.1.0` release).
A commit that does not advance a planned task (release prep, maintenance, docs)
uses `Refs: CHANGELOG.md`.
