# 14 — End-user agent skill

Ship an installable agent skill so coding agents (Claude Code and compatible
tools) reach for contasty when they need a codebase's shape, instead of reading
every file in full.

## Goal

A self-contained skill under `skills/contasty/` that an end user can copy into
their agent's skills folder. It must trigger on overview / API-surface /
"where is X" / onboarding requests and drive the agent to:

- run contasty scoped to the question (whole tree for broad, a subtree/files for
  narrow),
- treat the stripped map as the working source,
- open a real file only when it needs a body.

## Deliverables

- `skills/contasty/SKILL.md` — frontmatter trigger description + workflow,
  strip categories, pitfalls. Stays under the Markdown line limit.
- `skills/contasty/references/cli.md` — full flag / query-file / config
  reference, loaded lazily.
- `skills/contasty/evals/evals.json` — reproducible test prompts + assertions,
  runnable against contasty's own repo.
- README "Agent skill" section pointing at `skills/contasty/`.
- outdatty: skill files coupled to the CLI surface (`cli-docs` group) so flag
  changes flag the skill for review.

## Validation

Eval the skill with Sonnet sub-agents (with-skill vs no-skill baseline) on
contasty's own repo and on a large well-known codebase (Django). Confirm it
triggers reliably and that the stripped-map approach pays off in tokens on a
large tree.

## Done when

- Skill triggers on the target intents and uses contasty correctly.
- `just check` (incl. linecop + outdatty) green.
