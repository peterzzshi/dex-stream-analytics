# aggregator conventions

Extends root `AGENT.md`.

- Idiom: follow *Effective Java* (Bloch).
- Apply `/functional-programming-patterns` where it fits Java — immutable records, pure transforms in `functions/`, side effects isolated to `serde/` and I/O boundaries.
- Package layout: flat, one level under `com.web3analytics.*` (`types`, `config`, `models`, `functions`, `errors`, `serde`). Matches current layout.
- Mocking: Mockito (already a dependency) — no change.
