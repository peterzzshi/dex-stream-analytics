# GitHub Copilot Instructions

> **Active Skills (Synced from `~/.codex/skills`):** `effective-go`, `fp-patterns`, `golang-pro`, `java-concurrency`, `java-design-principles`, `java-generics`, `react-dev`, `react-patterns`, `react-state-management`, `rust-pro`, `tauri-v2`, `typescript-dev`, `typescript-pro`
> **Last Synced:** March 2, 2026

## Skill Sources

- `/Users/thaochinguyen/.codex/skills/effective-go/SKILL.md`
- `/Users/thaochinguyen/.codex/skills/fp-patterns/SKILL.md`
- `/Users/thaochinguyen/.codex/skills/golang-pro/SKILL.md`
- `/Users/thaochinguyen/.codex/skills/java-concurrency/SKILL.md`
- `/Users/thaochinguyen/.codex/skills/java-design-principles/SKILL.md`
- `/Users/thaochinguyen/.codex/skills/java-generics/SKILL.md`
- `/Users/thaochinguyen/.codex/skills/react-dev/SKILL.md`
- `/Users/thaochinguyen/.codex/skills/react-patterns/SKILL.md`
- `/Users/thaochinguyen/.codex/skills/react-state-management/SKILL.md`
- `/Users/thaochinguyen/.codex/skills/rust-pro/SKILL.md`
- `/Users/thaochinguyen/.codex/skills/tauri-v2/SKILL.md`
- `/Users/thaochinguyen/.codex/skills/typescript-dev/SKILL.md`
- `/Users/thaochinguyen/.codex/skills/typescript-pro/SKILL.md`

## Skill Application Rules

- Apply the minimal set of relevant skills per task.
- Go tasks: apply `effective-go` + `golang-pro`; include `fp-patterns` for structural refactoring.
- Java tasks: apply `java-design-principles`; add `java-concurrency` and/or `java-generics` as needed.
- React tasks: apply `react-dev`; add `react-patterns` and `react-state-management` for architecture/state-heavy work.
- TypeScript tasks: apply `typescript-dev` by default for strict typing, runtime validation, and day-to-day implementation.
- Advanced TypeScript tasks: add `typescript-pro` for advanced generics/conditional types, tRPC-style contracts, and TS build architecture.
- Functional TypeScript refactors: add `fp-patterns` when eliminating structural duplication or improving algebraic modeling.
- Rust tasks: apply `rust-pro`; add `tauri-v2` for Tauri applications.
- Tauri tasks: apply `tauri-v2` + `rust-pro` and relevant frontend skill (`typescript-dev` or React skills).

## Overlap Handling

- If skills overlap, prefer the more domain-specific skill first.
- Use cross-cutting skills (`fp-patterns`, `java-design-principles`, `effective-go`) as secondary guidance.
- Keep `typescript-dev` as the TypeScript default and activate `typescript-pro` only when advanced type-system or build concerns are central.
- Avoid conflicting guidance by applying no more than 2-3 skills at once.

## Quality Constraints

- Keep behavior identical during refactoring.
- Refactor one pattern at a time.
- Avoid premature abstraction.
- Profile before performance refactors.
- Run tests after structural changes.
