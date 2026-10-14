# ingester conventions

Extends root `AGENT.md`.

- Idiom: follow *Effective Go*; `gofmt`/`goimports` already enforced via lint.
- Apply `/functional-programming-patterns` where it fits Go — pure functions, explicit data flow, side effects confined to `internal/*` boundary types (RPC, Kafka, cache).
- Package layout: flat, one level under `internal/` (`internal/cache`, `internal/oracle`, ...). No nested subpackages — matches current layout.
- Mocking: generate interface mocks with `mockery` for new tests; pair with the existing `testify` assertions.
- No SQL database today. If one is introduced (e.g. persisting the finality buffer per the deferred fix in memory), use `ent` for schema and queries rather than hand-written SQL.
