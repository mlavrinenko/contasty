# 08 — README comparison vs repomix --compress

## Context

contasty ships into a crowded category (repo packers for LLM context). The core
move — tree-sitter parse, elide function bodies, keep signatures — is no longer
novel: repomix `--compress` does the same thing, and is the established
incumbent. Without a visible, honest positioning, contasty reads as
yet-another-stripper and stays invisible.

Research (2026-06) on the field:

- Same-shape peers (batch → one stripped document): repomix `--compress`
  (16 languages, large ecosystem) and ctx / context-hub generator
  (signature extraction, PHP only).
- Different shape (on-demand symbol index the agent queries live): aider repo
  map, jCodeMunch-MCP, RepoMapper, CodeRLM, AiDex, LLMap. Not direct peers.

repomix compress language count is authoritative: 16 query files under
`src/core/treeSitter/queries/` (C, C#, C++, CSS, Dart, Go, Java, JavaScript,
PHP, Python, Ruby, Rust, Solidity, Swift, TypeScript, Vue). Mechanism: layout
preserved, removed regions replaced with a `⋮----` non-code marker — output is
not valid source. contasty elides in place leaving valid empty bodies and can
reformat the result.

## Goal

A `## How it compares` section in README.md: a balanced table against repomix
`--compress` where each side wins several rows, plus a pointer to ctx (PHP-only)
and the query-on-demand cluster. Every positive claim in either column links to
a live artifact (contasty doc/source or repomix doc/source).

## Done

- [x] Table added, repomix and contasty each ahead on ~6 rows (no all-wins
      marketing grid).
- [x] Ecosystem / star-count row dropped (not a capability claim).
- [x] Every "has it" cell links to a verified-live artifact; "no" cells unlinked.
- [x] Language counts pinned to source: contasty 26, repomix 16.
- [x] `just fix-check` green (README under the 200-line markdown limit).

## Follow-up (not this task)

- Set the git remote and tag 0.1.0 (badges already point at the repo).
- Optional approximate token line under `--stats`, clearly labeled — or skip;
  a precise count is per-model and dishonest to fake locally.
