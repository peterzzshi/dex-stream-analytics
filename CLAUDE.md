## Session Handoff

- SESSION-HANDOFF.md must stay at or under 60 lines. Trim by moving durable content to ARCHITECTURE.md, not by
  deleting meaning.
- AGENT.md contains conventions and commands ONLY. Never add plans, progress trackers, milestone status, or
  auto-commit behavior.
- Update memory/handoff as the LAST step of a session, in a single pass, after code changes are verified.

## Git Workflow

Never commit unless explicitly asked. Always ask before committing.

## Documentation

This file holds cross-cutting rules; `AGENT.md` holds conventions and commands.

## Doc Accuracy

- Verify every claim against source before writing it: file paths, test names, class names, commands, env vars.
- Never invent a test file or build target. If you cannot find it in the repo, say so instead of naming one.
- When docs and code disagree, the code wins — correct the doc and note the conflict.

## Scope Discipline

- Do not edit project docs, docker-compose.yml, CI configs, or other files outside the requested change — propose the
  edit and wait for a yes.
- Do not "improve" configuration that already works (e.g. do not add explicit RUST_LOG overrides; rely on the code's
  EnvFilter defaults).
- When the user says a task is simple, give a 3-5 line plan and then act; no long investigation chains before the first
  edit.

## Testing

Tests assert behavior and outcomes, not implementation. Every test must contain a real assertion; flag tests that query
empty/unpopulated state.

Unit tests are for pure function behavior and do no IO: no file, schema, classpath resource, env var, or network reads.
Use embedded constants or generated classes instead. Do not record test counts in docs.
