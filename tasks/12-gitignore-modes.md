# 12 — gitignore modes: `--ignore=<mode>` (find-style, per-path)

## Context

Builds on `09` (resolver candidate walk) and `11` (query schema). `.gitignore` is
respected by default; this task lets a user change that — per path on the CLI, and
per query in YAML. A valued flag is mirrored by a query field so the CLI and
`*.cty.yaml` share one vocabulary (the `--strip` / `strip:` pair in task `13`
follows the same pattern).

This replaces the earlier `--all` / `--ignored` / `--non-ignored` value-less flags:
a single valued `--ignore=<mode>` flag avoids the `--all` vs `--include=all` clash,
mirrors the query field, and — being valued — needs no clap spike (valued flags
already carry per-occurrence indices, exactly what `ordered_selectors` uses today).

## Goal

A valued `--ignore=<mode>` flag, repeatable and interleaved with the path list
(find-style):

```
contasty A B --ignore=disable C D --ignore=enable E F
```

Each `--ignore=` sets the mode for the paths that follow, until the next one.
Default (before any switch) is `enable` — respect `.gitignore`, the current
behaviour. Applies uniformly to every arg class: named file, folder, glob, and
query candidates.

Modes (the flag enables / disables / inverts the gitignore filter):

- `enable` (default): gitignore on — only non-ignored files.
- `disable`: gitignore off — include ignored files too (everything).
- `reverse`: invert — only the `.gitignore`d files.

Queries set their own mode with the same vocabulary:

```yaml
ignore: enable   # enable (default) | disable | reverse
rules: |
  generated/**
```

A query's `ignore` wins for its own selection; absent → inherits the ambient mode
at the point the query was referenced.

## Design notes

- CLI parse reuses the proven mechanism: `--ignore` is a valued repeatable flag
  like `--include` / `--exclude`. Order it against the positional `paths` via
  `indices_of` — extend `ordered_selectors`-style logic (`src/main.rs`) to merge
  positional indices with `--ignore` value indices, assigning each path the
  most-recent mode. No value-less-flag spike.
- Build this as a general parser that partitions positionals into groups (maximal
  runs under one flag state; one left-to-right pass over positional + flag indices,
  last-of-kind wins) and attaches options to each group. Coalesce groups that
  resolve to identical options. Task `13` extends the same parser with `--strip`;
  keep it generic so both flags ride it.
- Thread a `Mode` per arg into the `09` resolver (`09` left `resolve` ready;
  refactor its input to carry a per-arg mode, e.g. `&[(PathBuf, Mode)]`).
- Walk per mode:
  - `enable` (default): standard `ignore` walk.
  - `disable`: `WalkBuilder::standard_filters(false)`, but keep skipping `.git/`
    (and likely hidden files) to avoid garbage.
  - `reverse`: walk with filters off, keep only paths an
    `ignore::gitignore::Gitignore` matcher marks ignored.
- Named file: test it against the gitignore matcher; keep / drop per mode.
- Selection vs mode (intersect): mode picks the candidate set; query / glob rules
  select within it. A rule cannot re-include an ignored file unless the mode
  admits it — which is how a query reaches generated / ignored files: set its
  `ignore`.

## Acceptance

- [ ] CLI: valued `--ignore=<mode>` (`enable` / `disable` / `reverse`),
      repeatable, interleaved with paths; default `enable`.
- [ ] Modes applied uniformly to file / folder / glob / query candidates.
- [ ] `reverse` via a `Gitignore` matcher; `disable` via disabled standard filters
      (still skip `.git/`).
- [ ] Query `ignore:` field (same vocabulary); query value wins, else inherits the
      ambient mode.
- [ ] `--help` documents the modes with examples.
- [ ] Tests: default respects gitignore; `disable` includes ignored; `reverse`
      only ignored; interleave (mode switch mid-list); query `ignore` field;
      intersect semantics.
- [ ] Docs: README usage (modes) + `docs/queries.md` `ignore` field.
- [ ] `just fix-check` green.

## Out of scope

- `--strip` / `strip:` category vocabulary — task `13`.
- URL / remote-repo inputs.
