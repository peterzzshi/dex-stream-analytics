## Git Workflow

Never commit changes unless explicitly asked to. Always ask before committing, even if the work is complete.

## Documentation Conventions

When writing or updating AGENT.md/SESSION-HANDOFF.md files, verify every referenced file, test name, command, and
dependency actually exists in the codebase before including it. Keep these files concise (prefer <60 lines); do not
include plans, progress logs, or auto-commit behavior.

## Scope Discipline

Before refactoring or editing beyond the scope of a review/analysis request, present the proposed change and get
confirmation first. Do not edit docs or code proactively during a 'review' or 'assess' task.

## Testing

Tests should assert on behavior and outcomes, not implementation details or internal structure. Every test must contain
a real assertion; flag tests that query empty/unpopulated state.
