# 16 — `.contasty/` project dir + XDG global config & queries

Introduce a namespaced `.contasty/` home for config, saved queries, and custom
rule files — at the project level and at an XDG global level — so a language
definition or a reusable query is written once and shared across projects.

Breaking change: config moves out of `./contasty.toml`; query pattern roots
change. Bump to 0.3.0.

## Directory layout (identical at both levels)

```
<project>/.contasty/            # project-local
  config.toml                   # was ./contasty.toml
  queries/<name>.cty.yaml       # named, reusable selectors
  rules/<lang>.yml              # custom rule files referenced by config.toml

$XDG_CONFIG_HOME/contasty/      # global (fallback: $HOME/.config/contasty)
  config.toml
  queries/<name>.cty.yaml
  rules/<lang>.yml
```

## Config loading & layering

Merge two layers into one `Config`, project over global (later wins):

1. Global: `<global>/config.toml`
2. Project: `--config <path>` when given, else `<project>/.contasty/config.toml`

Merge rules (project overrides global):

- `compact`: wholesale — project `[compact]` replaces global's if present, else
  inherit, else defaults. Deserialize as `Option<CompactConfig>` to tell
  "set" from "default".
- `strip`: `project.strip.or(global.strip)`.
- `languages`: union by key; a key defined in both, project entry wins wholesale.

No `./contasty.toml` fallback — clean break. Migrate the repo's own file with
`git mv contasty.toml .contasty/config.toml`.

### Per-language rule/grammar paths → resolve to absolute at load

`Config.base` (single dir) can no longer resolve rule paths once two layers
merge: a global-sourced language's `rules`/`libraryPath` is relative to the
global dir, a project-sourced one to the project dir. Resolve every
`LangConfig` path (`rules`, `extend`, `override`, `libraryPath` incl. the
`Platform` map) to an absolute path against its own config file's dir, right
after deserializing each layer and before merging. Then downstream joins are
no-ops. Remove `Config.base`; `with_config`, `dynamic::register`, and
`apply_overrides` stop depending on a shared base (pass a neutral base or drop
the param — the paths are already absolute).

### XDG resolution (env read in `main`, dir injected into lib)

Keep the library pure and testable. `main` resolves the global dir from the
environment and passes it in; tests pass a tempdir.

```rust
// main.rs
fn global_contasty_dir() -> Option<PathBuf> {
    use std::env;
    if let Some(x) = env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(x).join("contasty"));
    }
    env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .map(|h| PathBuf::from(h).join(".config").join("contasty"))
}
```

`Config::load(cli_config: Option<&Path>, project_dir: &Path, global_dir: Option<&Path>) -> Config`.
No new crate — env only.

## Named queries: `@name`

- A path argument of the form `@name` names a saved query.
- Resolve `@name` to a file: `<project>/.contasty/queries/<name>.cty.yaml`
  (then `.cty.yml`), else `<global>/queries/<name>.cty.yaml` (then `.yml`).
  First hit wins.
- Not found → `AppError::Input` listing the searched paths.
- The resolved file unfolds exactly like a query file passed by path.
- Plumb `global_dir` through `inputs::resolve` → `Resolver` so `@name` can reach
  the global queries dir.

## Query pattern rebasing (makes queries reusable)

A query's `rules` currently root at the query file's own directory, so a query
in `.contasty/queries/` or the XDG dir would look for `src/…` beside itself.
Rebase so a saved query describes the scanned project:

- Inline / list `rules` patterns root at the working directory (the scanned
  project root, `cwd`), not the query file's dir. The rules-walker walks `cwd`.
- External `{ path }` rules: locate the file relative to the query file's dir,
  but root its patterns at `cwd` too.
- Selected source files are therefore always under `cwd` (walker root = `cwd`).
- Relax the `check_within_cwd` sandbox: machinery paths (external rule files,
  `import` targets) may live outside `cwd` (under `.contasty/` or the XDG dir).
  The "selected files stay under `cwd`" guarantee is preserved by the walker
  root, so this only lifts the restriction on trusted config-referenced files.
- `import` targets still resolve relative to the importing query's dir.

For a query at the project root run from the project root, `cwd == query_dir`,
so existing root-level queries behave as before.

## Files

Rust: `src/config.rs` (split if it nears the 500-line linecop limit — e.g.
`src/config/mod.rs` + a merge/load submodule; update `outdatty.yaml` sources and
the `#[path]` test include if so), `src/lang/mod.rs`, `src/lang/dynamic.rs`,
`src/lang/overrides.rs`, `src/inputs.rs`, `src/query.rs`, `src/main.rs`,
`src/lib.rs`. Add tests for: two-layer merge, per-lang absolute paths, XDG
fallback, `@name` resolution (project + global + not-found), rebased query
patterns, relaxed sandbox.

Physical: `git mv contasty.toml .contasty/config.toml` + update its header
comment. `git add` all new `.contasty/**` and source files (nix flake and
outdatty only see tracked files).

Docs (outdatty dependents — edit, then `just outdatty-update`):

- `docs/custom-rules.md` — `.contasty/config.toml`, `.contasty/rules/`, XDG
  global config, per-language absolute path resolution.
- `docs/queries.md` — `@name`, `.contasty/queries/`, XDG queries, pattern
  rebasing, relaxed sandbox.
- `README.md`, `www/index.html`, `skills/contasty/SKILL.md`,
  `skills/contasty/references/cli.md` — config path, `@name`, XDG.
- `docs/languages.md` — if `with_config` doc text changes.
- `CHANGELOG.md` + `Cargo.toml` version → 0.3.0 (Cargo.toml trips release-notes).
- `--config` flag help text: `.contasty/config.toml`, not `contasty.toml`.

## Done when

- `contasty` reads `<project>/.contasty/config.toml`, layered under
  `<global>/config.toml`; `--config` overrides the project layer.
- A language/grammar defined only in global config applies in any project.
- `contasty @name` resolves project-then-global; saved queries select the
  project regardless of where the query file lives.
- `just fix-check` green (fmt, clippy, tests, machete, linecop, outdatty).
- Regression tests for every bullet under Files.
</content>
</invoke>
